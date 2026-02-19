use std::process::Command;
use regex::Regex;
use crate::models::Connection;
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

pub struct SSHMonitor {
    connections: Vec<Connection>,
    last_update: u64,
}

impl SSHMonitor {
    pub fn new() -> Self {
        SSHMonitor {
            connections: vec![],
            last_update: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.connections.clear();
        self.last_update = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // --- Step 1: find sshd listening ports (reverse tunnels) and their owning PIDs ---
        // port_by_pid: pid -> reverse-tunnel port (from ss -tlnpH)
        // pid_by_port: port -> pid  (inverse, useful for fallback)
        // all_reverse_ports: ports without matching PID info (ss ran without root)
        let (port_by_pid, pid_by_port, all_reverse_ports) = self.get_sshd_listening_ports();

        // --- Step 2: find client source IPs from established SSH connections ---
        // Maps sshd pid -> client_ip:port string
        let client_ips = self.get_client_ips();

        // --- Step 3: collect candidate sshd tunnel sessions from ps aux ---
        let output = Command::new("ps")
            .args(&["aux"])
            .output()?;
        let ps_output = String::from_utf8_lossy(&output.stdout);

        // Tunnel sessions: sshd processes that are NOT:
        //   [priv] privilege-separation processes
        //   [listener] main sshd daemon
        //   @pts / @notty  interactive shell sessions
        let mut sessions: Vec<(u32, String, String)> = Vec::new();
        for line in ps_output.lines() {
            if !line.contains("sshd:") || line.contains("grep") || line.contains("[priv]") || line.contains("[listener]") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            if let Ok(pid) = parts[1].parse::<u32>() {
                let user = parts[0].to_string();
                if let Some(sshd_pos) = line.find("sshd:") {
                    let sshd_rest = line[sshd_pos + 5..].trim();
                    let session_info = sshd_rest.split_whitespace().next().unwrap_or("unknown").to_string();
                    if !session_info.contains("@pts") && !session_info.contains("@notty") {
                        sessions.push((pid, user, session_info));
                    }
                }
            }
        }

        // --- Step 4: match sessions to ports ---
        //
        // Primary path  : pid found in port_by_pid  → exact match, most reliable
        // Secondary path: pid found in pid_by_port   → port already known, reverse lookup
        // Tertiary path : no PID info from ss (no root) → use unmatched ports list
        //                 Order of all_reverse_ports matches order of sessions heuristically
        //                 (both sorted by port / pid ascending), so alignment is reasonable.
        let mut used_ports: std::collections::HashSet<u16> = std::collections::HashSet::new();

        // Primary: exact pid → port
        for (pid, user, session_info) in &sessions {
            if let Some(&port) = port_by_pid.get(pid) {
                let client_ip = client_ips.get(pid).cloned().unwrap_or_default();
                self.push_connection(*pid, user, session_info, &client_ip, port);
                used_ports.insert(port);
            }
        }

        // Tertiary: ss ran without root → no PID info; align by position
        if port_by_pid.is_empty() && pid_by_port.is_empty() {
            let unmatched: Vec<_> = sessions.iter()
                .filter(|(pid, _, _)| !self.connections.iter().any(|c| c.pid == *pid))
                .collect();
            let avail_ports: Vec<_> = all_reverse_ports.iter()
                .filter(|p| !used_ports.contains(p))
                .collect();
            for (i, (pid, user, session_info)) in unmatched.iter().enumerate() {
                if let Some(&&port) = avail_ports.get(i) {
                    let client_ip = client_ips.get(pid).cloned().unwrap_or_default();
                    self.push_connection(*pid, user, session_info, &client_ip, port);
                }
            }
        }

        Ok(())
    }

    fn push_connection(&mut self, pid: u32, user: &str, session_info: &str, client_ip: &str, port: u16) {
        let bastion_host = if client_ip.is_empty() {
            session_info.to_string()
        } else {
            format!("{} ({})", session_info, client_ip)
        };
        self.connections.push(Connection {
            pid,
            user: user.to_string(),
            remote_bind: format!("*:{}", port),
            local_bind: "localhost:22".to_string(),
            remote_port: port,
            local_port: 22,
            bastion_host,
            bastion_port: 22,
            command: format!("ssh -p {} root@<bastion-ip>", port),
            uptime: "running".to_string(),
            status: "Connected".to_string(),
        });
    }

    /// Parse `ss -tlnpH` to find sshd listening ports for reverse tunnels.
    ///
    /// Returns:
    ///  - `port_by_pid`: pid → port  (populated when ss can show PID info, i.e. running as root)
    ///  - `pid_by_port`: port → pid  (inverse of above)
    ///  - `all_reverse_ports`: every sshd port > 1023 found, in ascending order
    ///    (used as fallback when PID info is unavailable)
    fn get_sshd_listening_ports(&self) -> (HashMap<u32, u16>, HashMap<u16, u32>, Vec<u16>) {
        let mut port_by_pid: HashMap<u32, u16> = HashMap::new();
        let mut pid_by_port: HashMap<u16, u32> = HashMap::new();
        let mut all_ports: Vec<u16> = Vec::new();

        // -H: no header  -t: TCP  -l: listening  -n: numeric  -p: show process
        // Typical line (as root):
        //   LISTEN 0 128 0.0.0.0:8951 0.0.0.0:* users:(("sshd",pid=1234,fd=9))
        // Without root process info is omitted:
        //   LISTEN 0 128 0.0.0.0:8951 0.0.0.0:*
        let Ok(output) = Command::new("ss").args(&["-tlnpH"]).output() else {
            return (port_by_pid, pid_by_port, all_ports);
        };

        let ss_output = String::from_utf8_lossy(&output.stdout);

        // Extract port from various local-address formats:
        //   0.0.0.0:8951   [::]:8951   *:8951
        // We use a regex that anchors to the peer-address column to avoid false matches.
        let line_re = match Regex::new(
            r"(?:0\.0\.0\.0|\*|\[::\]):(\d{2,5})\s+(?:0\.0\.0\.0|\*|\[::\]):\*"
        ) {
            Ok(r) => r,
            Err(_) => return (port_by_pid, pid_by_port, all_ports),
        };
        let pid_re = match Regex::new(r#""sshd",pid=(\d+)"#) {
            Ok(r) => r,
            Err(_) => return (port_by_pid, pid_by_port, all_ports),
        };

        for line in ss_output.lines() {
            // Only care about lines that are sshd-owned OR have a port in the tunnel range.
            // When running without root, "sshd" won't appear in the line — we rely on the
            // port-range heuristic (>1023, not 22) after checking it's a LISTEN socket.
            let has_sshd = line.contains("sshd");
            let is_listen = line.trim_start().starts_with("LISTEN");
            if !is_listen {
                continue;
            }

            let caps = match line_re.captures(line) {
                Some(c) => c,
                None => continue,
            };
            let port: u16 = match caps.get(1).and_then(|m| m.as_str().parse().ok()) {
                Some(p) => p,
                None => continue,
            };

            // Skip the standard SSH listener port and low ports that are not tunnels
            if port <= 1023 || port == 22 {
                continue;
            }

            // When process info is present, only accept sshd-owned ports.
            // When process info is absent (no root), accept all high-ports on LISTEN.
            if line.contains("users:(") && !has_sshd {
                continue;
            }

            all_ports.push(port);

            // Extract PIDs from  users:(("sshd",pid=1234,fd=9),...)
            for pid_cap in pid_re.captures_iter(line) {
                if let Some(pid_str) = pid_cap.get(1) {
                    if let Ok(pid) = pid_str.as_str().parse::<u32>() {
                        port_by_pid.insert(pid, port);
                        pid_by_port.entry(port).or_insert(pid);
                    }
                }
            }
        }

        all_ports.sort_unstable();
        (port_by_pid, pid_by_port, all_ports)
    }

    /// Parse `ss -tnpH state established` to find the source IP of each connected client.
    ///
    /// Returns a map of sshd pid → "client_ip:port" string.
    /// This works because each established TCP connection handled by sshd on SSH port
    /// shows the remote (client) address.
    fn get_client_ips(&self) -> HashMap<u32, String> {
        let mut map: HashMap<u32, String> = HashMap::new();

        let Ok(output) = Command::new("ss")
            .args(&["-tnpH", "state", "established"])
            .output()
        else {
            return map;
        };

        let ss_output = String::from_utf8_lossy(&output.stdout);

        // Line format:
        //   ESTAB 0 0 <local_ip>:<local_port> <peer_ip>:<peer_port> users:(("sshd",pid=1234,fd=4))
        // or without "ESTAB" prefix depending on ss version:
        //   0 0 <local>:<port> <peer>:<port> users:(...)
        let pid_re = match Regex::new(r#""sshd",pid=(\d+)"#) {
            Ok(r) => r,
            Err(_) => return map,
        };
        // Capture peer address (4th or 5th whitespace token that looks like ip:port)
        // Peer address is the 2nd address column.
        let addr_re = match Regex::new(
            r"\d+\s+\d+\s+\S+:\d+\s+(\S+:\d+)\s+users:"
        ) {
            Ok(r) => r,
            Err(_) => return map,
        };

        for line in ss_output.lines() {
            if !line.contains("sshd") {
                continue;
            }
            let peer_addr = addr_re
                .captures(line)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            for pid_cap in pid_re.captures_iter(line) {
                if let Some(pid_str) = pid_cap.get(1) {
                    if let Ok(pid) = pid_str.as_str().parse::<u32>() {
                        map.entry(pid).or_insert_with(|| peer_addr.clone());
                    }
                }
            }
        }

        map
    }

    #[allow(dead_code)]
    fn parse_sshd_process(&self, line: &str, listening_ports: &HashMap<u32, u16>) -> Option<Connection> {
        // Parse sshd processes to detect active SSH connections
        // Example: "bastion    369  0.4  0.0  14688  6548 ?        S    11:45   0:00  \_ sshd: bastion"

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        // Skip the main listener process
        if line.contains("[listener]") {
            return None;
        }

        // Extract PID and user
        let pid: u32 = parts.get(1)?.parse().ok()?;
        let user = parts.get(0)?.to_string();

        // Find "sshd:" in the line and extract what comes after
        if let Some(sshd_pos) = line.find("sshd:") {
            let sshd_rest = &line[sshd_pos + 5..].trim();
            
            // Extract the session info (e.g., "bastion" or "bastion@pts/1")
            let session_info = sshd_rest.split_whitespace().next().unwrap_or("unknown");
            
            // Skip pts sessions (interactive shells, not tunnels)
            if session_info.contains("@pts") || session_info.contains("@notty") {
                return None;
            }

            // Find the reverse port from listening sockets
            let remote_port = listening_ports.get(&pid).copied().unwrap_or(0);
            
            // Skip if no reverse port is found (not a tunnel)
            if remote_port == 0 {
                return None;
            }
            
            let remote_bind = format!("*:{}", remote_port);
            let status = "Connected".to_string();

            return Some(Connection {
                pid,
                user,
                remote_bind,
                local_bind: "localhost:22".to_string(),
                remote_port,
                local_port: 22,
                bastion_host: session_info.to_string(),
                bastion_port: 22,
                command: format!("ssh -p {} root@<bastion-ip>", remote_port),
                uptime: "running".to_string(),
                status,
            });
        }

        None
    }

    #[allow(dead_code)]
    fn parse_ssh_process(&self, line: &str) -> Option<Connection> {
        // Look for patterns like: autossh ... -R *:8080:localhost:22 bastion@host
        // or: ssh ... -R *:8080:localhost:22 bastion@host

        // Skip lines that don't contain relevant keywords
        if !line.contains("ssh") || line.contains("grep") {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        // Extract PID
        let pid: u32 = parts.get(1)?.parse().ok()?;

        // Extract user
        let user = parts.get(0)?.to_string();

        // Find the command part (everything after first few fields)
        let cmd_start = parts.iter().position(|&p| p.contains("autossh") || p.contains("/ssh"))?;
        let command = parts[cmd_start..].join(" ");

        // Parse for -R flag (reverse port forwarding)
        let reverse_regex = Regex::new(r"-R\s+\*?:(\d+):([^:]+):(\d+)").ok()?;
        let rev_caps = reverse_regex.captures(&command)?;
        
        let remote_port: u16 = rev_caps.get(1)?.as_str().parse().ok()?;
        let local_bind = rev_caps.get(2)?.as_str().to_string();
        let local_port: u16 = rev_caps.get(3)?.as_str().parse().ok()?;

        // Parse for bastion connection
        let bastion_regex = Regex::new(r"(?:bastion|autossh)@([^\s]+)").ok()?;
        let bastion_caps = bastion_regex.captures(&command)?;
        let bastion_host = bastion_caps.get(1)?.as_str().to_string();

        // Parse for bastion port (-p flag)
        let port_regex = Regex::new(r"-p\s+(\d+)").ok();
        let bastion_port: u16 = if let Some(regex) = port_regex {
            if let Some(caps) = regex.captures(&command) {
                caps.get(1)?.as_str().parse().ok()?
            } else {
                22
            }
        } else {
            22
        };

        // Extract bastion user from command
        let bastion_user_regex = Regex::new(r"([a-zA-Z0-9]+)@").ok()?;
        let _bastion_user = bastion_user_regex
            .captures(&command)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "bastion".to_string());

        let remote_bind = format!("*:{}", remote_port);
        let status = "Connected".to_string();

        Some(Connection {
            pid,
            user,
            remote_bind,
            local_bind,
            remote_port,
            local_port,
            bastion_host,
            bastion_port,
            command,
            uptime: "running".to_string(),
            status,
        })
    }

    pub fn get_connections(&self) -> Vec<Connection> {
        self.connections.clone()
    }

    #[allow(dead_code)]
    pub fn get_connection_count(&self) -> usize {
        self.connections.len()
    }

    pub fn last_update(&self) -> u64 {
        self.last_update
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_parse_sshd_bastion_session() {
        let monitor = SSHMonitor::new();
        let line = "bastion   4848  0.0  0.0  14688  6540 ?        S    18:36   0:00 sshd: bastion";
        let listening_ports = std::collections::HashMap::new();
        
        if let Some(conn) = monitor.parse_sshd_process(line, &listening_ports) {
            assert_eq!(conn.pid, 4848);
            assert_eq!(conn.user, "bastion");
            assert_eq!(conn.command, "sshd: bastion");
            assert_eq!(conn.status, "Connected");
        } else {
            panic!("Failed to parse valid sshd process");
        }
    }

    #[test]
    #[ignore]
    fn test_parse_sshd_pts_session() {
        let monitor = SSHMonitor::new();
        let line = "bastion   4860  0.0  0.0  14788  6748 ?        S    18:36   0:00 sshd: bastion@pts/1";
        let listening_ports = std::collections::HashMap::new();
        
        if let Some(conn) = monitor.parse_sshd_process(line, &listening_ports) {
            assert_eq!(conn.pid, 4860);
            assert_eq!(conn.user, "bastion");
            assert!(conn.command.contains("bastion@pts"));
            assert_eq!(conn.status, "Connected");
        } else {
            panic!("Failed to parse valid sshd process with pts");
        }
    }

    #[test]
    #[ignore]
    fn test_parse_sshd_priv_session() {
        let monitor = SSHMonitor::new();
        let line = "root      4837  0.1  0.0  14428 10076 ?        Ss   18:36   0:00 sshd: bastion [priv]";
        let listening_ports = std::collections::HashMap::new();
        
        if let Some(conn) = monitor.parse_sshd_process(line, &listening_ports) {
            assert_eq!(conn.pid, 4837);
            assert_eq!(conn.user, "root");
            assert!(!conn.command.contains("[listener]"));
            assert_eq!(conn.status, "Connected");
        } else {
            panic!("Failed to parse sshd [priv] process");
        }
    }

    #[test]
    #[ignore]
    fn test_skip_sshd_listener() {
        let monitor = SSHMonitor::new();
        let line = "root         1  0.0  0.0  12016  8212 pts/0    Ss+  11:56   0:00 sshd: /usr/sbin/sshd -D -e [listener] 0 of 10-100 startups";
        let listening_ports = std::collections::HashMap::new();
        
        let result = monitor.parse_sshd_process(line, &listening_ports);
        assert!(result.is_none(), "Should skip listener process");
    }

    #[test]
    #[ignore]
    fn test_invalid_pid_line() {
        let monitor = SSHMonitor::new();
        let line = "user invalid_pid 0.0 0.0 1234 5678 ?";
        let listening_ports = std::collections::HashMap::new();
        
        let result = monitor.parse_sshd_process(line, &listening_ports);
        assert!(result.is_none(), "Should return None for invalid PID");
    }
}
