use crate::models::ServerEntry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ServerStore {
    servers: Vec<ServerEntry>,
}

pub struct ServerManager {
    file_path: PathBuf,
    cache: Arc<Mutex<Vec<ServerEntry>>>,
}

impl ServerManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let file_path = app_data_dir.join("servers.json");
        if let Some(parent) = file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // 加载已保存的服务器
        let cache = match fs::read_to_string(&file_path) {
            Ok(content) => {
                let store: ServerStore = serde_json::from_str(&content).unwrap_or_default();
                Arc::new(Mutex::new(store.servers))
            }
            Err(_) => Arc::new(Mutex::new(Vec::new())),
        };

        Self { file_path, cache }
    }

    fn save_to_file(&self, servers: &[ServerEntry]) -> Result<(), String> {
        let store = ServerStore { servers: servers.to_vec() };
        let content = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
        fs::write(&self.file_path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save(&self, name: String, address: String, port: u16, default_group: String, default_device: String) -> Result<ServerEntry, String> {
        let entry = ServerEntry {
            id: Uuid::new_v4().to_string(),
            name,
            address,
            port,
            default_group,
            default_device,
        };

        let mut servers = self.cache.lock().map_err(|e| e.to_string())?;
        servers.push(entry.clone());
        self.save_to_file(&servers)?;
        Ok(entry)
    }

    pub fn get_all(&self) -> Result<Vec<ServerEntry>, String> {
        self.cache.lock().map_err(|e| e.to_string()).map(|s| s.clone())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut servers = self.cache.lock().map_err(|e| e.to_string())?;
        servers.retain(|s| s.id != id);
        self.save_to_file(&servers)?;
        Ok(())
    }

    pub fn measure_rtt(address: &str, port: u16) -> Option<u64> {
        let addr_str = format!("{}:{}", address, port);
        let addr: SocketAddr = match addr_str.to_socket_addrs().ok()?.next() {
            Some(a) => a,
            None => return None,
        };
        let start = Instant::now();
        match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
            Ok(_) => Some(start.elapsed().as_millis() as u64),
            Err(_) => None,
        }
    }
}

#[tauri::command]
pub fn save_server(
    manager: tauri::State<ServerManager>,
    name: String,
    address: String,
    port: u16,
    default_group: String,
    default_device: String,
) -> Result<ServerEntry, String> {
    manager.save(name, address, port, default_group, default_device)
}

#[tauri::command]
pub fn get_servers(manager: tauri::State<ServerManager>) -> Result<Vec<ServerEntry>, String> {
    manager.get_all()
}

#[tauri::command]
pub fn delete_server(manager: tauri::State<ServerManager>, id: String) -> Result<(), String> {
    manager.delete(&id)
}

#[tauri::command]
pub fn measure_server_rtt(address: String, port: u16) -> Result<Option<u64>, String> {
    Ok(ServerManager::measure_rtt(&address, port))
}
