use crate::models::ServerEntry;
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn save_server(state: State<AppState>, name: String, address: String, port: u16) -> Result<ServerEntry, String> {
    let entry = ServerEntry { id: Uuid::new_v4().to_string(), name, address, port };
    state.servers.lock().map_err(|e| e.to_string())?.push(entry.clone());
    Ok(entry)
}

#[tauri::command]
pub fn get_servers(state: State<AppState>) -> Result<Vec<ServerEntry>, String> {
    state.servers.lock().map_err(|e| e.to_string()).map(|s| s.clone())
}

#[tauri::command]
pub fn delete_server(state: State<AppState>, id: String) -> Result<(), String> {
    state.servers.lock().map_err(|e| e.to_string())?.retain(|s| s.id != id);
    Ok(())
}
