use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Instant;
use crate::models::{ServerEntry, NetworkStatus, NetworkStats};

pub struct AppState {
    pub servers: Arc<Mutex<Vec<ServerEntry>>>,
    pub network_status: Arc<Mutex<NetworkStatus>>,
    pub network_stats: Arc<Mutex<NetworkStats>>,
    pub connection_time: Arc<Mutex<Option<Instant>>>,
    pub nebula_process: Arc<Mutex<Option<std::process::Child>>>,
    pub last_stats: Arc<Mutex<HashMap<String, (u64, u64)>>>,
    pub close_behavior: Arc<Mutex<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(Vec::new())),
            network_status: Arc::new(Mutex::new(NetworkStatus { connected: false, virtual_ip: None, group_name: None, uptime_seconds: 0 })),
            network_stats: Arc::new(Mutex::new(NetworkStats::default())),
            connection_time: Arc::new(Mutex::new(None)),
            nebula_process: Arc::new(Mutex::new(None)),
            last_stats: Arc::new(Mutex::new(HashMap::new())),
            close_behavior: Arc::new(Mutex::new("minimize".to_string())),
        }
    }
}
