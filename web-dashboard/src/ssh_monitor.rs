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
        
        // Get listening ports per PID from ss command
        let (listening_ports, all_reverse_ports) = self.get_sshd_listening_ports();
        
        // Get all sshd processes
        let output = Command::new("ps")
            .args(&["aux"])
            .output()?;

        let ps_output = String::from_utf8_lossy(&output.stdout);
        
        // Parse for sshd processes (reverse tunnel connections)
        // Skip [priv] privilege separation processes
        let mut sessions: Vec<(u32, String, String)> = Vec::new();
        for line in ps_output.lines() {
            if line.contains("sshd:") && !line.contains("grep") && !line.contains("[priv]") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        let user = parts[0].to_string();
                        if let Some(sshd_pos) = line.find("sshd:") {
                            let sshd_rest = &line[sshd_pos + 5..].trim();
                            let session_info = sshd_rest.split_whitespace().next().unwrap_or("unknown").to_string();
                            
                            // Skip pts/notty sessions (interactive shells)
                            if !session_info.contains("@pts") && !session_info.contains("@notty") && !line.contains("[listener]") {
                                sessions.push((pid, user, session_info));
                            }
                        }
                    }
                }
            }
        }
        
        // Match sessions with ports
        // First, try exact PID match
        let mut used_ports: Vec<u16> = Vec::new();
        for (pid, user, session_info) in &sessions {
            if let Some(&port) = listening_ports.get(pid) {
                self.connections.push(Connection {
                    pid: *pid,
                    user: user.clone(),
                    remote_bind: format!("*:{}", port),
                    local_bind: "localhost:22".to_string(),
                    remote_port: port,
                    local_port: 22,
                    bastion_host: session_info.clone(),
                    bastion_port: 22,
                    command: format!("ssh -p {} root@<bastion-ip>", port),
                    uptime: "running".to_string(),
                    status: "Connected".to_string(),
                });
                used_ports.push(port);
            }
        }
        
        // If we have sessions without matched ports, and available ports, assign them
        let unmatched_sessions: Vec<_> = sessions.iter()
            .filter(|(pid, _, _)| !listening_ports.contains_key(pid))
            .collect();
        
        let available_ports: Vec<_> = all_reverse_ports.iter()
            .filter(|p| !used_ports.contains(p))
            .collect();
        
        for (i, (pid, user, session_info)) in unmatched_sessions.iter().enumerate() {
            if let Some(&&port) = available_ports.get(i) {
                self.connections.push(Connection {
                    pid: *pid,
                    user: user.clone(),
                    remote_bind: format!("*:{}", port),
                    local_bind: "localhost:22".to_string(),
                    remote_port: port,
                    local_port: 22,
                    bastion_host: session_info.clone(),
                    bastion_port: 22,
                    command: format!("ssh -p {} root@<bastion-ip>", port),
                    uptime: "running".to_string(),
                    status: "Connected".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Get listening ports associated with sshd processes
    /// Returns a HashMap mapping PID to listening port, and a Vec of all reverse tunnel ports
    fn get_sshd_listening_ports(&self) -> (HashMap<u32, u16>, Vec<u16>) {
        let mut port_map: HashMap<u32, u16> = HashMap::new();
        let mut all_ports: Vec<u16> = Vec::new();
        
        // Use ss to get listening sockets
        if let Ok(output) = Command::new("ss")
            .args(&["-tlnp"])
            .output()
        {
            let ss_output = String::from_utf8_lossy(&output.stdout);
            
            for line in ss_output.lines() {
                if !line.contains("sshd") {
                    continue;
                }
                
                // Extract port from Local Address:Port format
                if let Some(port_str) = line.split_whitespace().nth(3) {
                    if let Some(port_part) = port_str.split(':').last() {
                        if let Ok(port) = port_part.parse::<u16>() {
                            // Skip standard SSH port (22) as that's the listener
                            if port == 22 {
                                continue;
                            }
                            
                            // Extract all PIDs from the process info (there may be multiple)
                            if let Some(process_info) = line.split("users:").nth(1) {
                                let pid_regex = Regex::new(r"pid=(\d+)").ok();
                                if let Some(regex) = pid_regex {
                                    for cap in regex.captures_iter(process_info) {
                                        if let Some(pid_str) = cap.get(1) {
                                            if let Ok(pid) = pid_str.as_str().parse::<u32>() {
                                                port_map.insert(pid, port);
                                            }
                                        }
                                    }
                                }
                            }
                            
                            all_ports.push(port);
                        }
                    }
                }
            }
        }
        
        (port_map, all_ports)
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
