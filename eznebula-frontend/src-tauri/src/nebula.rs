use crate::crypto::generate_keypair;
use crate::models::{ApiResponse, JoinRequest, JoinResponse, NetworkStats, NetworkStatus};
use crate::state::AppState;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::time::Instant;
use tauri::{State, Manager};

// 编译时嵌入二进制文件
#[cfg(windows)]
const NEBULA_BIN_BYTES: &[u8] = include_bytes!("../binaries/nebula.exe");
#[cfg(windows)]
const WINTUN_DLL_BYTES: &[u8] = include_bytes!("../binaries/wintun.dll");

const NEBULA_BIN: &str = if cfg!(windows) { "nebula.exe" } else { "nebula" };

fn extract_embedded_binaries(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        // 提取到应用数据目录
        let app_data = app_handle.path().app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        let bin_dir = app_data.join("bin");
        fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin dir: {}", e))?;

        // nebula 需要的 wintun 路径结构
        let wintun_dir = bin_dir.join("dist").join("windows").join("wintun").join("bin").join("amd64");
        fs::create_dir_all(&wintun_dir).map_err(|e| format!("Failed to create wintun dir: {}", e))?;

        let nebula_path = bin_dir.join("nebula.exe");
        let wintun_path = wintun_dir.join("wintun.dll");

        // 如果文件不存在或大小不匹配，则重新写入
        let need_write_nebula = !nebula_path.exists() ||
            fs::metadata(&nebula_path).ok().map_or(true, |m| m.len() != NEBULA_BIN_BYTES.len() as u64);
        let need_write_wintun = !wintun_path.exists() ||
            fs::metadata(&wintun_path).ok().map_or(true, |m| m.len() != WINTUN_DLL_BYTES.len() as u64);

        if need_write_nebula {
            let mut f = fs::File::create(&nebula_path).map_err(|e| format!("Failed to create nebula.exe: {}", e))?;
            f.write_all(NEBULA_BIN_BYTES).map_err(|e| format!("Failed to write nebula.exe: {}", e))?;
            log::info!("Extracted nebula.exe to {:?}", nebula_path);
        }

        if need_write_wintun {
            let mut f = fs::File::create(&wintun_path).map_err(|e| format!("Failed to create wintun.dll: {}", e))?;
            f.write_all(WINTUN_DLL_BYTES).map_err(|e| format!("Failed to write wintun.dll: {}", e))?;
            log::info!("Extracted wintun.dll to {:?}", wintun_path);
        }

        return Ok(nebula_path);
    }

    #[cfg(not(windows))]
    Err("Not implemented for non-Windows platforms".to_string())
}

/// 从 virtual_ip_with_cidr (如 "10.168.2.32/24") 计算网络 CIDR ("10.168.2.0/24")
fn compute_network_cidr(cidr: &str) -> String {
    let parts: Vec<&str> = cidr.split('/').collect();
    let ip_str = parts[0];
    let prefix: u8 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(24);

    let ip_parts: Vec<u8> = ip_str.split('.').filter_map(|p| p.parse().ok()).collect();
    if ip_parts.len() != 4 {
        return "0.0.0.0/24".to_string();
    }

    let ip_u32 = ((ip_parts[0] as u32) << 24)
        | ((ip_parts[1] as u32) << 16)
        | ((ip_parts[2] as u32) << 8)
        | (ip_parts[3] as u32);

    let mask = if prefix >= 32 {
        0xFFFFFFFFu32
    } else {
        !((1u32 << (32 - prefix)) - 1)
    };

    let net_u32 = ip_u32 & mask;
    format!(
        "{}.{}.{}.{}/{}",
        (net_u32 >> 24) as u8,
        (net_u32 >> 16) as u8,
        (net_u32 >> 8) as u8,
        net_u32 as u8,
        prefix
    )
}

fn find_nebula(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    // 1. 优先使用嵌入的二进制文件（生产环境）
    if let Ok(path) = extract_embedded_binaries(app_handle) {
        return Ok(path);
    }

    // 2. 从 Tauri 资源目录查找（打包后的备选方案）
    if let Ok(resource_path) = app_handle.path().resource_dir() {
        let nebula_path = resource_path.join(NEBULA_BIN);
        if nebula_path.is_file() {
            log::info!("Found nebula in resource dir: {:?}", nebula_path);
            return Ok(nebula_path);
        }
    }

    // 3. 开发模式：从当前目录或 binaries 目录查找
    let cwd = std::env::current_dir().unwrap_or_default();
    for dir in [&cwd, &cwd.join("binaries"), &cwd.join("src-tauri").join("binaries")] {
        let p = dir.join(NEBULA_BIN);
        if p.is_file() {
            log::info!("Found nebula in dev dir: {:?}", p);
            return Ok(p);
        }
    }

    // 4. 最后尝试 PATH
    which::which(NEBULA_BIN).map_err(|_| format!("{} not found in any location", NEBULA_BIN))
}

#[tauri::command]
pub async fn join_network(app: tauri::AppHandle, state: State<'_, AppState>, request: JoinRequest) -> Result<String, String> {
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

    // 计算路由 CIDR：优先用服务器返回的 network_cidr，否则从 IP 推导
    let route_cidr = if !d.network_cidr.is_empty() {
        d.network_cidr.clone()
    } else {
        // 从 virtual_ip_with_cidr 计算网络地址 (e.g. "10.168.2.32/24" -> "10.168.2.0/24")
        compute_network_cidr(&d.virtual_ip_with_cidr)
    };

    // 提取纯 IP（去掉 CIDR 后缀）作为 via
    let via_ip = d.virtual_ip_with_cidr.split('/').next().unwrap_or("0.0.0.0").to_string();

    let lh_addr = format!("{}:{}", d.lighthouse_ip, d.lighthouse_port);
    let yml = format!(
        "pki:\n  ca: \"{}\"\n  cert: \"{}\"\n  key: \"{}\"\n\
         static_host_map:\n  \"{}\": [\"{}\"]\n\
         lighthouse:\n  am_lighthouse: false\n  interval: 60\n  hosts:\n    - \"{}\"\n\
         listen:\n  host: 0.0.0.0\n  port: 0\n\
         punchy:\n  punch: true\n  respond: true\n\
         relay:\n  am_relay: false\n  use_relays: true\n\
         tun:\n  disabled: false\n  dev: eznebula0\n  drop_local_broadcast: false\n  drop_multicast: false\n  tx_queue: 500\n  mtu: 1300\n  unsafe_routes:\n    - route: {}\n      via: {}\n\
         logging:\n  level: debug\n  format: text\n  file: \"{}\"\n\
         firewall:\n  outbound:\n    - port: any\n      proto: any\n      host: any\n  inbound:\n    - port: any\n      proto: any\n      host: any\n",
        ca_p.to_string_lossy().replace('\\', "/"),
        cert_p.to_string_lossy().replace('\\', "/"),
        key_p.to_string_lossy().replace('\\', "/"),
        d.lighthouse_nebula_ip, lh_addr,
        d.lighthouse_nebula_ip,
        route_cidr, via_ip,
        cfg.join("nebula.log").to_string_lossy().replace('\\', "/"),
    );
    fs::write(&yml_p, yml).map_err(|e| e.to_string())?;

    std::thread::sleep(std::time::Duration::from_secs(1)); // clock skew tolerance

    let nebula = find_nebula(&app)?;
    let wd = nebula.parent().unwrap_or_else(|| std::path::Path::new("."));

    let mut cmd = Command::new(&nebula);
    cmd.arg("-config").arg(&yml_p).current_dir(wd)
        .stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = cmd.spawn().map_err(|e| format!("Start nebula: {}", e))?;

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
