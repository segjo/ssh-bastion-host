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
        
        // Get all processes
        let output = Command::new("ps")
            .args(&["aux"])
            .output()?;

        let ps_output = String::from_utf8_lossy(&output.stdout);
        
        // Parse for autossh and ssh processes with reverse tunnels
        for line in ps_output.lines() {
            if let Some(conn) = self.parse_ssh_process(line) {
                // Found connection
                self.connections.push(conn);
            }
        }

        Ok(())
    }

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
        let bastion_user = bastion_user_regex
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

    pub fn get_connection_count(&self) -> usize {
        self.connections.len()
    }

    pub fn last_update(&self) -> u64 {
        self.last_update
    }
}
