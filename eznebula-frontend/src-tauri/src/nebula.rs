use crate::crypto::generate_keypair;
use crate::models::{ApiResponse, JoinRequest, JoinResponse, NetworkStats, NetworkStatus};
use crate::state::AppState;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;
use tauri::State;

const NEBULA_BIN: &str = if cfg!(windows) { "nebula.exe" } else { "nebula" };

/// Resolve nebula binary path: binaries/ directory first, then fall back to PATH.
fn find_nebula() -> Result<PathBuf, String> {
    // 1. Check binaries/ next to the executable (release mode)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let bundled = exe_dir.join("binaries").join(NEBULA_BIN);
            if bundled.is_file() {
                log::info!("Using bundled nebula: {}", bundled.display());
                return Ok(bundled);
            }
        }
    }
    // 2. Check CWD/binaries (dev mode)
    if let Ok(cwd) = std::env::current_dir() {
        let cwd_bundled = cwd.join("binaries").join(NEBULA_BIN);
        if cwd_bundled.is_file() {
            log::info!("Using bundled nebula: {}", cwd_bundled.display());
            return Ok(cwd_bundled);
        }
    }
    // 3. Fall back to PATH
    which::which(NEBULA_BIN).map_err(|_| format!("{} not found in binaries/ or PATH", NEBULA_BIN))
}

#[tauri::command]
pub async fn join_network(state: State<'_, AppState>, request: JoinRequest) -> Result<String, String> {
    log::info!("Joining network: {} at {}", request.group_name, request.server_url);

    // 1. Generate keypair
    let keypair = generate_keypair()?;

    // 2. Call backend API
    let join_url = format!("{}/api/v1/join", request.server_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let api_req = serde_json::json!({
        "groupName": request.group_name,
        "joinToken": request.join_token,
        "clientPublicKey": keypair.public_key,
        "clientName": request.client_name,
    });

    let resp = client.post(&join_url).json(&api_req).send().await
        .map_err(|e| format!("Cannot reach server: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;

    if !status.is_success() {
        return Err(format!("Server error {}: {}", status, text));
    }

    let api_resp: ApiResponse<JoinResponse> = serde_json::from_str(&text)
        .map_err(|e| format!("Parse error: {} — {}", e, text))?;

    if !api_resp.success {
        return Err(api_resp.message.unwrap_or_else(|| "Unknown error".into()));
    }

    let data = api_resp.data.ok_or_else(|| "No data".to_string())?;
    log::info!("Got certificate for {}", data.virtual_ip_with_cidr);

    // 3. Write config files
    let config_dir = get_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

    let ca_path = config_dir.join("ca.crt");
    let cert_path = config_dir.join("client.crt");
    let key_path = config_dir.join("client.key");
    let config_path = config_dir.join("config.yml");

    fs::write(&ca_path, &data.ca_certificate).map_err(|e| e.to_string())?;
    fs::write(&cert_path, &data.client_certificate).map_err(|e| e.to_string())?;
    fs::write(&key_path, &keypair.private_key).map_err(|e| e.to_string())?;

    // 4. Generate config.yml (use forward slashes to avoid YAML escape issues)
    let log_path = config_dir.join("nebula.log");
    let lighthouse_host = format!("{}:{}", data.lighthouse_ip, data.lighthouse_port);
    let yml = format!(
        "pki:\n  ca: \"{}\"\n  cert: \"{}\"\n  key: \"{}\"\n\
         static_host_map:\n  \"{}\": [\"{}\"]\n\
         lighthouse:\n  am_lighthouse: false\n  interval: 60\n  hosts:\n    - \"{}\"\n\
         listen:\n  host: 0.0.0.0\n  port: 0\n\
         punchy:\n  punch: true\n  respond: true\n\
         relay:\n  am_relay: false\n  use_relays: true\n\
         tun:\n  disabled: false\n  dev: eznebula0\n  drop_local_broadcast: false\n  drop_multicast: false\n  tx_queue: 500\n  mtu: 1300\n\
         logging:\n  level: debug\n  format: text\n  file: \"{}\"\n\
         firewall:\n  outbound:\n    - port: any\n      proto: any\n      host: any\n  inbound:\n    - port: any\n      proto: any\n      host: any\n",
        ca_path.to_string_lossy().replace('\\', "/"),
        cert_path.to_string_lossy().replace('\\', "/"),
        key_path.to_string_lossy().replace('\\', "/"),
        data.lighthouse_ip, lighthouse_host,
        data.lighthouse_ip,
        log_path.to_string_lossy().replace('\\', "/"),
    );

    fs::write(&config_path, yml).map_err(|e| e.to_string())?;
    log::info!("Config written to: {}", config_path.display());

    // Small delay to tolerate client-server clock skew.
    // The cert's notBefore is set by the server's clock — if the server
    // is slightly ahead, the cert may appear "not yet valid" when nebula
    // checks it immediately. A few seconds covers typical NTP drift.
    std::thread::sleep(std::time::Duration::from_secs(3));

    // 5. Start Nebula (parent process is admin → child inherits admin rights)
    let nebula = find_nebula()?;
    let nebula_dir = nebula.parent().unwrap_or_else(|| std::path::Path::new("."));

    let mut child = Command::new(&nebula)
        .arg("-config").arg(&config_path)
        .current_dir(nebula_dir)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start nebula: {}", e))?;

    // Capture nebula's stderr/stdout for debugging
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            for line in BufReader::new(stderr).lines() {
                if let Ok(l) = line { log::error!("nebula: {}", l); }
            }
        });
    }
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            for line in BufReader::new(stdout).lines() {
                if let Ok(l) = line { log::info!("nebula: {}", l); }
            }
        });
    }

    // Small delay to ensure cert notBefore has passed (clock skew tolerance)
    std::thread::sleep(std::time::Duration::from_secs(2));

    log::info!("Nebula started, PID: {}", child.id());

    state.nebula_process.lock().map_err(|e| e.to_string())?.replace(child);

    // 6. Update state
    {
        let mut s = state.network_status.lock().map_err(|e| e.to_string())?;
        s.connected = true;
        s.virtual_ip = Some(data.virtual_ip_with_cidr.clone());
        s.group_name = Some(request.group_name);
        s.uptime_seconds = 0;
    }
    state.connection_time.lock().map_err(|e| e.to_string())?.replace(Instant::now());

    Ok(format!("Connected! IP: {}", data.virtual_ip_with_cidr))
}

#[tauri::command]
pub fn disconnect_network(state: State<AppState>) -> Result<(), String> {
    log::info!("Disconnecting");

    // Kill the PowerShell wrapper process if we still have it
    if let Some(mut child) = state.nebula_process.lock().map_err(|e| e.to_string())?.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    // On Windows, also kill nebula.exe directly (it was launched via RunAs)
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "nebula.exe"])
            .stdout(Stdio::null()).stderr(Stdio::null())
            .status();
    }

    {
        let mut s = state.network_status.lock().map_err(|e| e.to_string())?;
        s.connected = false; s.virtual_ip = None; s.group_name = None; s.uptime_seconds = 0;
    }
    *state.connection_time.lock().map_err(|e| e.to_string())? = None;
    *state.network_stats.lock().map_err(|e| e.to_string())? = NetworkStats::default();

    log::info!("Disconnected");
    Ok(())
}

#[tauri::command]
pub fn get_status(state: State<AppState>) -> Result<NetworkStatus, String> {
    let mut s = state.network_status.lock().map_err(|e| e.to_string())?;
    if s.connected {
        if let Ok(ct) = state.connection_time.lock() {
            if let Some(start) = *ct {
                s.uptime_seconds = start.elapsed().as_secs();
            }
        }
    }
    Ok(s.clone())
}

#[tauri::command]
pub fn get_network_stats(state: State<AppState>) -> Result<NetworkStats, String> {
    state.network_stats.lock().map_err(|e| e.to_string()).map(|s| s.clone())
}

fn get_config_dir() -> Result<PathBuf> {
    Ok(dirs::home_dir().ok_or_else(|| anyhow!("No home dir"))?.join(".eznebula"))
}
