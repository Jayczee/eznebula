use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRequest {
    pub server_url: String,
    pub group_name: String,
    pub join_token: String,
    pub client_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinResponse {
    pub virtual_ip_with_cidr: String,
    pub client_certificate: String,
    pub ca_certificate: String,
    pub lighthouse_ip: String,
    pub lighthouse_port: u16,
    pub network_cidr: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub connected: bool,
    pub virtual_ip: Option<String>,
    pub group_name: Option<String>,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkStats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_speed: f64,
    pub tx_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keypair {
    pub public_key: String,
    pub private_key: String,
}
