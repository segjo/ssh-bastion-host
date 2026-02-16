use std::process::Command;
use regex::Regex;
use crate::models::Connection;
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

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
        
        // Get all sshd processes
        let output = Command::new("ps")
            .args(&["aux"])
            .output()?;

        let ps_output = String::from_utf8_lossy(&output.stdout);
        
        // Parse for sshd processes (reverse tunnel connections)
        for line in ps_output.lines() {
            // Look for sshd processes
            if line.contains("sshd:") && !line.contains("grep") {
                if let Some(conn) = self.parse_sshd_process(line) {
                    self.connections.push(conn);
                }
            }
        }

        Ok(())
    }

    fn parse_sshd_process(&self, line: &str) -> Option<Connection> {
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
            
            // Extract the session info (e.g., "bastion" or "bastion [priv]")
            let session_info = sshd_rest.split_whitespace().next().unwrap_or("unknown");

            let remote_bind = format!("0.0.0.0:22");
            let status = "Connected".to_string();

            // Try to get additional TCP connection info from /proc/[pid]/net/tcp
            let proc_path = format!("/proc/{}/net/tcp", pid);
            let (local_bind, remote_port, local_port, bastion_host, bastion_port) = 
                if let Ok(tcp_data) = std::fs::read_to_string(&proc_path) {
                    // Parse the TCP connection table to find established connections
                    let mut found = false;
                    for tcp_line in tcp_data.lines().skip(1) {
                        let fields: Vec<&str> = tcp_line.split_whitespace().collect();
                        if fields.len() < 4 {
                            continue;
                        }
                        
                        // Look for established connections (state 01)
                        if let Some(state_field) = fields.get(3) {
                            if *state_field == "01" {
                                found = true;
                                break;
                            }
                        }
                    }
                    if found {
                        ("0.0.0.0:22".to_string(), 22u16, 22u16, "localhost".to_string(), 22u16)
                    } else {
                        ("0.0.0.0:22".to_string(), 22u16, 22u16, session_info.to_string(), 22u16)
                    }
                } else {
                    ("0.0.0.0:22".to_string(), 22u16, 22u16, session_info.to_string(), 22u16)
                };

            return Some(Connection {
                pid,
                user,
                remote_bind,
                local_bind,
                remote_port,
                local_port,
                bastion_host,
                bastion_port,
                command: format!("sshd: {}", session_info),
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
    fn test_parse_sshd_bastion_session() {
        let monitor = SSHMonitor::new();
        let line = "bastion   4848  0.0  0.0  14688  6540 ?        S    18:36   0:00 sshd: bastion";
        
        if let Some(conn) = monitor.parse_sshd_process(line) {
            assert_eq!(conn.pid, 4848);
            assert_eq!(conn.user, "bastion");
            assert_eq!(conn.command, "sshd: bastion");
            assert_eq!(conn.status, "Connected");
        } else {
            panic!("Failed to parse valid sshd process");
        }
    }

    #[test]
    fn test_parse_sshd_pts_session() {
        let monitor = SSHMonitor::new();
        let line = "bastion   4860  0.0  0.0  14788  6748 ?        S    18:36   0:00 sshd: bastion@pts/1";
        
        if let Some(conn) = monitor.parse_sshd_process(line) {
            assert_eq!(conn.pid, 4860);
            assert_eq!(conn.user, "bastion");
            assert!(conn.command.contains("bastion@pts"));
            assert_eq!(conn.status, "Connected");
        } else {
            panic!("Failed to parse valid sshd process with pts");
        }
    }

    #[test]
    fn test_parse_sshd_priv_session() {
        let monitor = SSHMonitor::new();
        let line = "root      4837  0.1  0.0  14428 10076 ?        Ss   18:36   0:00 sshd: bastion [priv]";
        
        if let Some(conn) = monitor.parse_sshd_process(line) {
            assert_eq!(conn.pid, 4837);
            assert_eq!(conn.user, "root");
            assert!(!conn.command.contains("[listener]"));
            assert_eq!(conn.status, "Connected");
        } else {
            panic!("Failed to parse sshd [priv] process");
        }
    }

    #[test]
    fn test_skip_sshd_listener() {
        let monitor = SSHMonitor::new();
        let line = "root         1  0.0  0.0  12016  8212 pts/0    Ss+  11:56   0:00 sshd: /usr/sbin/sshd -D -e [listener] 0 of 10-100 startups";
        
        let result = monitor.parse_sshd_process(line);
        assert!(result.is_none(), "Should skip listener process");
    }

    #[test]
    fn test_invalid_pid_line() {
        let monitor = SSHMonitor::new();
        let line = "user invalid_pid 0.0 0.0 1234 5678 ?";
        
        let result = monitor.parse_sshd_process(line);
        assert!(result.is_none(), "Should return None for invalid PID");
    }
}
