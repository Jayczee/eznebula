use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Instant;
use crate::models::{NetworkStatus, NetworkStats, PeerInfo};

pub struct AppState {
    pub network_status: Arc<Mutex<NetworkStatus>>,
    pub network_stats: Arc<Mutex<NetworkStats>>,
    pub connection_time: Arc<Mutex<Option<Instant>>>,
    pub nebula_process: Arc<Mutex<Option<std::process::Child>>>,
    /// Per-peer traffic tracking: last (rx_bytes, tx_bytes, Instant) for speed calc
    pub peer_last_bytes: Arc<Mutex<HashMap<String, (u64, u64, Instant)>>>,
    /// Current peer list built from nebula stdout parsing
    pub peers: Arc<Mutex<Vec<PeerInfo>>>,
    pub close_behavior: Arc<Mutex<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            network_status: Arc::new(Mutex::new(NetworkStatus { connected: false, virtual_ip: None, group_name: None, client_name: None, server_url: None, uptime_seconds: 0 })),
            network_stats: Arc::new(Mutex::new(NetworkStats::default())),
            connection_time: Arc::new(Mutex::new(None)),
            nebula_process: Arc::new(Mutex::new(None)),
            peer_last_bytes: Arc::new(Mutex::new(HashMap::new())),
            peers: Arc::new(Mutex::new(Vec::new())),
            close_behavior: Arc::new(Mutex::new("minimize".to_string())),
        }
    }
}
