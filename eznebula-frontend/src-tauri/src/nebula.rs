use crate::crypto::generate_keypair;
use crate::models::{ApiResponse, JoinRequest, JoinResponse, NetworkStats, NetworkStatus};
use crate::state::AppState;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;
use tauri::State;

const NEBULA_BIN: &str = if cfg!(windows) { "nebula.exe" } else { "nebula" };

fn find_nebula() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().unwrap_or_default();
    for dir in [&cwd, &cwd.join("binaries")] {
        let p = dir.join(NEBULA_BIN);
        if p.is_file() { return Ok(p); }
    }
    which::which(NEBULA_BIN).map_err(|_| format!("{} not found", NEBULA_BIN))
}

#[tauri::command]
pub async fn join_network(state: State<'_, AppState>, request: JoinRequest) -> Result<String, String> {
    let kp = generate_keypair()?;
    let api_url = format!("{}/api/v1/join", request.server_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client.post(&api_url).json(&serde_json::json!({
        "groupName": request.group_name, "joinToken": request.join_token,
        "clientPublicKey": kp.public_key, "clientName": request.client_name,
    })).send().await.map_err(|e| format!("Server unreachable: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;
    if !status.is_success() { return Err(format!("Server {}: {}", status, text)); }
    let api: ApiResponse<JoinResponse> = serde_json::from_str(&text).map_err(|e| format!("Parse: {}", e))?;
    if !api.success { return Err(api.message.unwrap_or_default()); }
    let d = api.data.ok_or("No data")?;

    let cfg = get_config_dir()?;
    fs::create_dir_all(&cfg).map_err(|e| e.to_string())?;
    let ca_p = cfg.join("ca.crt"); let cert_p = cfg.join("client.crt");
    let key_p = cfg.join("client.key"); let yml_p = cfg.join("config.yml");
    fs::write(&ca_p, &d.ca_certificate).map_err(|e| e.to_string())?;
    fs::write(&cert_p, &d.client_certificate).map_err(|e| e.to_string())?;
    fs::write(&key_p, &kp.private_key).map_err(|e| e.to_string())?;

    let lh = format!("{}:{}", d.lighthouse_ip, d.lighthouse_port);
    let yml = format!("pki:\n  ca: \"{}\"\n  cert: \"{}\"\n  key: \"{}\"\nstatic_host_map:\n  \"{}\": [\"{}\"]\nlighthouse:\n  am_lighthouse: false\n  interval: 60\n  hosts:\n    - \"{}\"\nlisten:\n  host: 0.0.0.0\n  port: 0\npunchy:\n  punch: true\n  respond: true\nrelay:\n  am_relay: false\n  use_relays: true\ntun:\n  disabled: false\n  dev: eznebula0\n  drop_local_broadcast: false\n  drop_multicast: false\n  tx_queue: 500\n  mtu: 1300\nlogging:\n  level: debug\n  format: text\n  file: \"{}\"\nfirewall:\n  outbound:\n    - port: any\n      proto: any\n      host: any\n  inbound:\n    - port: any\n      proto: any\n      host: any\n",
        ca_p.to_string_lossy().replace('\\', "/"), cert_p.to_string_lossy().replace('\\', "/"),
        key_p.to_string_lossy().replace('\\', "/"), d.lighthouse_ip, lh, d.lighthouse_ip,
        cfg.join("nebula.log").to_string_lossy().replace('\\', "/"));
    fs::write(&yml_p, yml).map_err(|e| e.to_string())?;

    std::thread::sleep(std::time::Duration::from_secs(1)); // clock skew tolerance

    let nebula = find_nebula()?;
    let wd = nebula.parent().unwrap_or_else(|| std::path::Path::new("."));
    let mut child = Command::new(&nebula).arg("-config").arg(&yml_p).current_dir(wd)
        .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Start nebula: {}", e))?;

    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || { for l in BufReader::new(stderr).lines() { if let Ok(line) = l { log::error!("nebula: {}", line); } } });
    }
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || { for l in BufReader::new(stdout).lines() { if let Ok(line) = l { log::info!("nebula: {}", line); } } });
    }

    state.nebula_process.lock().map_err(|e| e.to_string())?.replace(child);
    state.network_status.lock().map_err(|e| e.to_string()).map(|mut s| {
        s.connected = true; s.virtual_ip = Some(d.virtual_ip_with_cidr.clone());
        s.group_name = Some(request.group_name); s.uptime_seconds = 0;
    })?;
    state.connection_time.lock().map_err(|e| e.to_string())?.replace(Instant::now());
    Ok(format!("Connected: {}", d.virtual_ip_with_cidr))
}

#[tauri::command]
pub fn disconnect_network(state: State<AppState>) -> Result<(), String> {
    if let Some(mut c) = state.nebula_process.lock().map_err(|e| e.to_string())?.take() { let _ = c.kill(); let _ = c.wait(); }
    #[cfg(windows)] { let _ = Command::new("taskkill").args(["/F","/IM","nebula.exe"]).stdout(Stdio::null()).stderr(Stdio::null()).status(); }
    state.network_status.lock().map_err(|e| e.to_string()).map(|mut s| { s.connected = false; s.virtual_ip = None; s.group_name = None; })?;
    *state.connection_time.lock().map_err(|e| e.to_string())? = None;
    *state.network_stats.lock().map_err(|e| e.to_string())? = NetworkStats::default();
    Ok(())
}

#[tauri::command]
pub fn get_status(state: State<AppState>) -> Result<NetworkStatus, String> {
    let mut s = state.network_status.lock().map_err(|e| e.to_string())?;
    if s.connected { if let Ok(ct) = state.connection_time.lock() { if let Some(t) = *ct { s.uptime_seconds = t.elapsed().as_secs(); } } }
    Ok(s.clone())
}

#[tauri::command]
pub fn get_network_stats(state: State<AppState>) -> Result<NetworkStats, String> {
    state.network_stats.lock().map_err(|e| e.to_string()).map(|s| s.clone())
}

fn get_config_dir() -> Result<PathBuf, String> {
    Ok(dirs::home_dir().ok_or("No home directory")?.join(".eznebula"))
}
