use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static SETTINGS_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    SETTINGS_DIR.set(dir).ok();
}

fn path() -> PathBuf {
    SETTINGS_DIR.get().cloned().unwrap_or_default().join("settings.json")
}

fn load_map() -> HashMap<String, String> {
    fs::read_to_string(path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_map(map: &HashMap<String, String>) {
    if let Ok(json) = serde_json::to_string(map) {
        let _ = fs::write(path(), json);
    }
}

pub fn get_close_behavior() -> String {
    load_map().get("close_behavior").cloned().unwrap_or_else(|| "minimize".into())
}

pub fn set_close_behavior(behavior: String) {
    let mut map = load_map();
    map.insert("close_behavior".into(), behavior);
    save_map(&map);
}
