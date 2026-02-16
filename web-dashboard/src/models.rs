use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub pid: u32,
    pub user: String,
    pub remote_bind: String,
    pub local_bind: String,
    pub remote_port: u16,
    pub local_port: u16,
    pub bastion_host: String,
    pub bastion_port: u16,
    pub command: String,
    pub uptime: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub total_connections: usize,
    pub timestamp: u64,
    pub connections: Vec<Connection>,
}
