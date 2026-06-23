use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    #[serde(default)]
    pub default_group: String,
    #[serde(default)]
    pub default_device: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRequest { pub server_url: String, pub group_name: String, pub join_token: String, pub client_name: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinResponse { pub virtual_ip_with_cidr: String, pub client_certificate: String, pub ca_certificate: String, pub lighthouse_ip: String, pub lighthouse_nebula_ip: String, pub lighthouse_port: u16, pub network_cidr: String, pub message: String }

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> { pub success: bool, pub message: Option<String>, pub data: Option<T> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus { pub connected: bool, pub virtual_ip: Option<String>, pub group_name: Option<String>, pub client_name: Option<String>, pub server_url: Option<String>, pub uptime_seconds: u64 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkStats { pub rx_bytes: u64, pub tx_bytes: u64, pub rx_speed: f64, pub tx_speed: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keypair { pub public_key: String, pub private_key: String }

/// Information about a peer in the same network group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub vpn_ip: String,
    pub hostname: String,
    /// "p2p" or "relay"
    pub connection_type: String,
    /// "alive", "testing", or "dead"
    pub state: String,
    /// Latency in milliseconds (None if not yet measured)
    pub latency_ms: Option<f64>,
    /// Total bytes received from this peer
    pub rx_bytes: u64,
    /// Total bytes sent to this peer
    pub tx_bytes: u64,
    /// Bytes/sec received (instantaneous)
    pub rx_speed: f64,
    /// Bytes/sec sent (instantaneous)
    pub tx_speed: f64,
    /// Local Nebula index ID
    pub local_index: u64,
    /// Remote Nebula index ID
    pub remote_index: u64,
}
