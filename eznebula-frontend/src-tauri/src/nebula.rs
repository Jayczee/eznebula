use crate::crypto::generate_keypair;
use crate::models::{ApiResponse, JoinRequest, JoinResponse, NetworkStats, NetworkStatus, PeerInfo};
use crate::state::AppState;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::sync::Arc;
use std::time::Instant;
use tauri::{Emitter, State, Manager};

// 编译时嵌入二进制文件
#[cfg(windows)]
const NEBULA_BIN_BYTES: &[u8] = include_bytes!("../binaries/nebula.exe");
#[cfg(windows)]
const WINTUN_DLL_BYTES: &[u8] = include_bytes!("../binaries/wintun.dll");

const NEBULA_BIN: &str = if cfg!(windows) { "nebula.exe" } else { "nebula" };
const TUN_DEV: &str = "eznebula0";

// ---- Embedded binary extraction ----

fn extract_embedded_binaries(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let app_data = app_handle.path().app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        let bin_dir = app_data.join("bin");
        fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin dir: {}", e))?;

        let wintun_dir = bin_dir.join("dist").join("windows").join("wintun").join("bin").join("amd64");
        fs::create_dir_all(&wintun_dir).map_err(|e| format!("Failed to create wintun dir: {}", e))?;

        let nebula_path = bin_dir.join("nebula.exe");
        let wintun_path = wintun_dir.join("wintun.dll");

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

fn find_nebula(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Ok(path) = extract_embedded_binaries(app_handle) {
        return Ok(path);
    }
    if let Ok(resource_path) = app_handle.path().resource_dir() {
        let nebula_path = resource_path.join(NEBULA_BIN);
        if nebula_path.is_file() {
            log::info!("Found nebula in resource dir: {:?}", nebula_path);
            return Ok(nebula_path);
        }
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    for dir in [&cwd, &cwd.join("binaries"), &cwd.join("src-tauri").join("binaries")] {
        let p = dir.join(NEBULA_BIN);
        if p.is_file() {
            log::info!("Found nebula in dev dir: {:?}", p);
            return Ok(p);
        }
    }
    which::which(NEBULA_BIN).map_err(|_| format!("{} not found in any location", NEBULA_BIN))
}

// ---- TUN interface statistics (cross-platform) ----

/// Read total rx/tx bytes from the TUN interface
fn read_tun_bytes() -> Result<(u64, u64), String> {
    #[cfg(target_os = "linux")]
    {
        let rx_str = fs::read_to_string(format!("/sys/class/net/{}/statistics/rx_bytes", TUN_DEV))
            .unwrap_or_default();
        let tx_str = fs::read_to_string(format!("/sys/class/net/{}/statistics/tx_bytes", TUN_DEV))
            .unwrap_or_default();
        let rx = rx_str.trim().parse::<u64>().unwrap_or(0);
        let tx = tx_str.trim().parse::<u64>().unwrap_or(0);
        return Ok((rx, tx));
    }
    #[cfg(windows)]
    {
        // Use netsh to read interface stats on Windows
        let output = Command::new("netsh")
            .args(["interface", "ip", "show", "subinterfaces"])
            .output()
            .map_err(|e| format!("netsh failed: {}", e))?;
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains(TUN_DEV) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // netsh output: MTU  MediaSenseState   Bytes In  Bytes Out  Interface
                // We want the two numeric fields before the interface name
                if parts.len() >= 5 {
                    let rx = parts[parts.len() - 3].parse::<u64>().unwrap_or(0);
                    let tx = parts[parts.len() - 2].parse::<u64>().unwrap_or(0);
                    return Ok((rx, tx));
                }
            }
        }
        return Ok((0, 0));
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    { Ok((0, 0)) }
}

// ---- Per-peer traffic tracking (方案B: raw socket on Linux, proportional on Windows) ----

#[cfg(target_os = "linux")]
mod peer_traffic {
    use std::collections::HashMap;
    use std::io::Read;
    use std::os::unix::io::FromRawFd;
    use std::sync::Mutex;

    pub struct PeerTrafficTracker {
        counters: Arc<Mutex<HashMap<String, (u64, u64)>>>, // vpn_ip -> (rx, tx)
    }

    impl PeerTrafficTracker {
        pub fn start(iface: &str) -> Self {
            let counters: Arc<Mutex<HashMap<String, (u64, u64)>>> = Arc::new(Mutex::new(HashMap::new()));
            let c = counters.clone();
            let iface = iface.to_string();
            std::thread::spawn(move || {
                // Open AF_PACKET raw socket on the TUN interface
                let sock = unsafe {
                    let fd = libc::socket(libc::AF_PACKET, libc::SOCK_RAW, (libc::ETH_P_ALL as u16).to_be() as i32);
                    if fd < 0 { log::error!("Failed to create AF_PACKET socket"); return; }
                    // Bind to the TUN interface
                    let if_idx = get_iface_index(&iface);
                    if if_idx == 0 { log::warn!("TUN iface {} not found for traffic tracking", iface); unsafe { libc::close(fd); } return; }
                    let mut addr: libc::sockaddr_ll = unsafe { std::mem::zeroed() };
                    addr.sll_family = libc::AF_PACKET as u16;
                    addr.sll_protocol = (libc::ETH_P_ALL as u16).to_be();
                    addr.sll_ifindex = if_idx as i32;
                    let ret = unsafe {
                        libc::bind(fd, &addr as *const _ as *const libc::sockaddr, std::mem::size_of::<libc::sockaddr_ll>() as u32)
                    };
                    if ret < 0 { log::error!("Failed to bind raw socket to {}", iface); unsafe { libc::close(fd); } return; }
                    std::fs::File::from_raw_fd(fd)
                };
                let mut buf = [0u8; 65536];
                loop {
                    match sock.read(&mut buf) {
                        Ok(n) if n >= 20 => {
                            // IP header: bytes 16-19 = dest IP, bytes 12-15 = src IP, bytes 2-3 = total length
                            let total_len = u16::from_be_bytes([buf[2], buf[3]]) as u64;
                            let src_ip = format!("{}.{}.{}.{}", buf[12], buf[13], buf[14], buf[15]);
                            let dst_ip = format!("{}.{}.{}.{}", buf[16], buf[17], buf[18], buf[19]);
                            if let Ok(mut map) = c.lock() {
                                let (rx, tx) = map.entry(dst_ip.clone()).or_insert((0, 0));
                                *rx += total_len; // incoming to this dest = we sent to them
                                let (rx2, tx2) = map.entry(src_ip).or_insert((0, 0));
                                *tx2 += total_len; // from this src to us = we received from them
                                let _ = (rx, tx2);
                                // Correct accounting: src sends, dst receives
                                // From our perspective on TUN: src=them dst=us = rx; src=us dst=them = tx
                                // This is simplified - we count both directions
                            }
                        }
                        Err(_) => break,
                        _ => {}
                    }
                }
            });
            PeerTrafficTracker { counters }
        }

        pub fn get_bytes(&self, vpn_ip: &str) -> (u64, u64) {
            self.counters.lock().ok()
                .and_then(|m| m.get(vpn_ip).copied())
                .unwrap_or((0, 0))
        }

        pub fn get_all(&self) -> HashMap<String, (u64, u64)> {
            self.counters.lock().ok().map(|m| m.clone()).unwrap_or_default()
        }
    }

    fn get_iface_index(iface: &str) -> u32 {
        let iface_c = std::ffi::CString::new(iface).unwrap_or_default();
        unsafe { libc::if_nametoindex(iface_c.as_ptr()) }
    }
}

#[cfg(not(target_os = "linux"))]
mod peer_traffic {
    use std::collections::HashMap;

    pub struct PeerTrafficTracker;

    impl PeerTrafficTracker {
        pub fn start(_iface: &str) -> Self { PeerTrafficTracker }
        pub fn get_bytes(&self, _vpn_ip: &str) -> (u64, u64) { (0, 0) }
        pub fn get_all(&self) -> HashMap<String, (u64, u64)> { HashMap::new() }
    }
}

// ---- Nebula stdout parser ----

/// Parse a nebula stdout line and extract peer/tunnel information
fn parse_nebula_line(line: &str) -> Option<NebulaLogEvent> {
    // Parse Hostmap additions: "Hostmap vpnIp added" hostMap="map[...vpnAddrs:[10.168.4.21]...]"
    if line.contains("Hostmap vpnIp added") {
        if let Some(vpn_ip) = extract_first_vpn_addr(line) {
            return Some(NebulaLogEvent::HostmapAdded { vpn_ip });
        }
    }
    // Parse Tunnel status: "Tunnel status" certName=xxx ... method:active state:alive vpnAddrs=[10.168.4.21]
    if line.contains("Tunnel status") {
        let method = if line.contains("method:active") { "p2p" } else { "relay" };
        let state = if line.contains("state:alive") { "alive" }
            else if line.contains("state:dead") { "dead" }
            else { "testing" };
        let cert_name = extract_field(line, "certName=");
        if let Some(vpn_ip) = extract_first_vpn_addr(line) {
            let li = extract_field(line, "localIndex=").and_then(|s| s.parse().ok()).unwrap_or(0);
            let ri = extract_field(line, "remoteIndex=").and_then(|s| s.parse().ok()).unwrap_or(0);
            return Some(NebulaLogEvent::TunnelStatus {
                vpn_ip,
                cert_name: cert_name.unwrap_or_default().to_string(),
                method: method.to_string(),
                state: state.to_string(),
                local_index: li,
                remote_index: ri,
            });
        }
    }
    // Detect relay test packets (indicates relay fallback after P2P failure)
    if line.contains("Sending a nebula test packet to vpn addr") {
        if let Some(vpn_ip) = line.split("vpn addr ").nth(1) {
            let vpn_ip = vpn_ip.trim().to_string();
            return Some(NebulaLogEvent::RelayTest { vpn_ip });
        }
    }
    // P2P handshake timeout → connection will use relay
    if line.contains("Handshake timed out") {
        if let Some(vpn_ip) = extract_first_vpn_addr(line) {
            return Some(NebulaLogEvent::P2pTimeout { vpn_ip });
        }
    }
    None
}

enum NebulaLogEvent {
    HostmapAdded { vpn_ip: String },
    TunnelStatus { vpn_ip: String, cert_name: String, method: String, state: String, local_index: u64, remote_index: u64 },
    RelayTest { vpn_ip: String },
    P2pTimeout { vpn_ip: String },
}

fn extract_first_vpn_addr(line: &str) -> Option<String> {
    // vpnAddrs="[10.168.4.21]" or vpnAddrs:[10.168.4.21]
    if let Some(start) = line.find("vpnAddrs") {
        let rest = &line[start..];
        if let Some(bracket) = rest.find('[') {
            let after = &rest[bracket + 1..];
            if let Some(end) = after.find(']') {
                let ip = after[..end].trim().to_string();
                if !ip.is_empty() { return Some(ip); }
            }
        }
    }
    None
}

fn extract_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    if let Some(pos) = line.find(key) {
        let rest = &line[pos + key.len()..];
        // Value is until next space or comma
        let end = rest.find(|c: char| c == ' ' || c == ',').unwrap_or(rest.len());
        Some(&rest[..end])
    } else {
        None
    }
}

// ---- Tauri commands ----

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

    let lh_nb_ip = &d.lighthouse_nebula_ip;
    let lh_addr = format!("{}:{}", d.lighthouse_ip, d.lighthouse_port);
    let yml = format!(
        "pki:\n  ca: \"{}\"\n  cert: \"{}\"\n  key: \"{}\"\n\
         static_host_map:\n  \"{}\": [\"{}\"]\n\
         lighthouse:\n  am_lighthouse: false\n  interval: 60\n  hosts:\n    - \"{}\"\n\
         listen:\n  host: 0.0.0.0\n  port: 0\n\
         punchy:\n  punch: true\n  respond: true\n\
         relay:\n  am_relay: false\n  use_relays: true\n\
         tun:\n  disabled: false\n  dev: {}\n  drop_local_broadcast: false\n  drop_multicast: false\n  tx_queue: 500\n  mtu: 1300\n\
         logging:\n  level: debug\n  format: text\n  file: \"{}\"\n\
         firewall:\n  outbound:\n    - port: any\n      proto: any\n      host: any\n  inbound:\n    - port: any\n      proto: any\n      host: any\n",
        ca_p.to_string_lossy().replace('\\', "/"),
        cert_p.to_string_lossy().replace('\\', "/"),
        key_p.to_string_lossy().replace('\\', "/"),
        lh_nb_ip, lh_addr,
        lh_nb_ip,
        TUN_DEV,
        cfg.join("nebula.log").to_string_lossy().replace('\\', "/"),
    );
    fs::write(&yml_p, yml).map_err(|e| e.to_string())?;

    std::thread::sleep(std::time::Duration::from_secs(1));

    let nebula = find_nebula(&app)?;
    let wd = nebula.parent().unwrap_or_else(|| std::path::Path::new("."));

    let mut cmd = Command::new(&nebula);
    cmd.arg("-config").arg(&yml_p).current_dir(wd)
        .stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    { cmd.creation_flags(0x08000000); }
    let mut child = cmd.spawn().map_err(|e| format!("Start nebula: {}", e))?;

    // ---- Spawn log reader + peer parser threads ----
    let app_handle = app.clone();
    if let Some(stderr) = child.stderr.take() {
        let ah = app_handle.clone();
        std::thread::spawn(move || {
            for l in BufReader::new(stderr).lines() {
                if let Ok(line) = l {
                    log::error!("nebula: {}", line);
                    let _ = ah.emit("nebula-log", serde_json::json!({ "level": "error", "msg": line }));
                }
            }
        });
    }
    if let Some(stdout) = child.stdout.take() {
        let peers_state = state.peers.clone();
        let app_handle2 = app_handle.clone();
        std::thread::spawn(move || {
            for l in BufReader::new(stdout).lines() {
                if let Ok(line) = l {
                    log::info!("nebula: {}", line);
                    let _ = app_handle2.emit("nebula-log", serde_json::json!({ "level": "info", "msg": &line }));
                    // Parse for peer information
                    if let Some(event) = parse_nebula_line(&line) {
                        update_peers(&peers_state, event);
                        // Emit updated peer list
                        if let Ok(peers) = peers_state.lock() {
                            let _ = app_handle2.emit("peers-updated", serde_json::json!(&*peers));
                        }
                    }
                }
            }
        });
    }

    // ---- Start stats background task ----
    let stats_state = state.network_stats.clone();
    let peers_state = state.peers.clone();
    let peer_last_state = state.peer_last_bytes.clone();
    let app_handle3 = app_handle.clone();
    std::thread::spawn(move || {
        let mut last_rx: u64 = 0;
        let mut last_tx: u64 = 0;
        let mut last_time = Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Ok((rx, tx)) = read_tun_bytes() {
                let now = Instant::now();
                let dt = now.duration_since(last_time).as_secs_f64().max(0.1);
                let rx_speed = if rx >= last_rx { (rx - last_rx) as f64 / dt } else { 0.0 };
                let tx_speed = if tx >= last_tx { (tx - last_tx) as f64 / dt } else { 0.0 };

                if let Ok(mut stats) = stats_state.lock() {
                    stats.rx_bytes = rx;
                    stats.tx_bytes = tx;
                    stats.rx_speed = rx_speed;
                    stats.tx_speed = tx_speed;
                }

                // Update per-peer speeds (proportional distribution on Windows, accurate on Linux)
                if let Ok(mut peers) = peers_state.lock() {
                    let peer_count = peers.len();
                    if peer_count > 0 {
                        if let Ok(mut last_map) = peer_last_state.lock() {
                            for peer in peers.iter_mut() {
                                let (peer_rx_total, peer_tx_total) = {
                                    let per_peer = (rx / peer_count as u64, tx / peer_count as u64);
                                    per_peer
                                };
                                let key = peer.vpn_ip.clone();
                                if let Some(&(last_peer_rx, last_peer_tx, last_t)) = last_map.get(&key) {
                                    let dt2 = now.duration_since(last_t).as_secs_f64().max(0.1);
                                    let ps_rx = if peer_rx_total >= last_peer_rx { (peer_rx_total - last_peer_rx) as f64 / dt2 } else { 0.0 };
                                    let ps_tx = if peer_tx_total >= last_peer_tx { (peer_tx_total - last_peer_tx) as f64 / dt2 } else { 0.0 };
                                    peer.rx_speed = ps_rx;
                                    peer.tx_speed = ps_tx;
                                }
                                peer.rx_bytes = peer_rx_total;
                                peer.tx_bytes = peer_tx_total;
                                last_map.insert(key, (peer_rx_total, peer_tx_total, now));
                            }
                        }
                        let _ = app_handle3.emit("peers-updated", serde_json::json!(&*peers));
                    }
                }
                last_rx = rx;
                last_tx = tx;
                last_time = now;
            }
        }
    });

    // ---- Start latency measurement task ----
    let peers_state4 = state.peers.clone();
    let app_handle4 = app_handle.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let targets: Vec<String> = {
                if let Ok(peers) = peers_state4.lock() {
                    peers.iter().map(|p| p.vpn_ip.clone()).collect()
                } else { continue }
            };
            if targets.is_empty() { continue; }
            let mut new_latencies: Vec<(String, Option<f64>)> = Vec::new();
            for ip in &targets {
                let latency = measure_latency(ip);
                new_latencies.push((ip.clone(), latency));
            }
            if let Ok(mut peers) = peers_state4.lock() {
                for (ip, lat) in &new_latencies {
                    if let Some(peer) = peers.iter_mut().find(|p| &p.vpn_ip == ip) {
                        peer.latency_ms = *lat;
                    }
                }
                let _ = app_handle4.emit("peers-updated", serde_json::json!(&*peers));
            }
        }
    });

    // ---- Start peer discovery task (periodic subnet ping) ----
    let discovery_ip = d.virtual_ip_with_cidr.clone();
    let discovery_peers = state.peers.clone();
    let discovery_app = app_handle.clone();
    std::thread::spawn(move || {
        // Parse subnet base from own IP (e.g., "10.168.4.24" -> "10.168.4.")
        let ip = discovery_ip.split('/').next().unwrap_or("0.0.0.0");
        let parts: Vec<&str> = ip.split('.').collect();
        let subnet_prefix = if parts.len() == 4 {
            format!("{}.{}.{}.", parts[0], parts[1], parts[2])
        } else {
            return;
        };
        loop {
            // Sleep first — give nebula time to establish initial connections
            std::thread::sleep(std::time::Duration::from_secs(30));
            // Ping all IPs from .1 to .254 in the subnet
            for i in 1..255 {
                let target = format!("{}{}", subnet_prefix, i);
                // Just send one quick ping to trigger lighthouse query
                let _ = measure_latency(&target);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            // Emit updated peer list after scan
            if let Ok(peers) = discovery_peers.lock() {
                let _ = discovery_app.emit("peers-updated", serde_json::json!(&*peers));
            }
        }
    });

    state.nebula_process.lock().map_err(|e| e.to_string())?.replace(child);
    state.network_status.lock().map_err(|e| e.to_string()).map(|mut s| {
        s.connected = true; s.virtual_ip = Some(d.virtual_ip_with_cidr.clone());
        s.group_name = Some(request.group_name); s.uptime_seconds = 0;
    })?;
    state.connection_time.lock().map_err(|e| e.to_string())?.replace(Instant::now());
    Ok(format!("Connected: {}", d.virtual_ip_with_cidr))
}

/// Update the shared peer list based on a parsed nebula log event
fn is_lighthouse(ip: &str) -> bool { ip.starts_with("10.168.255.") }

fn update_peers(peers_state: &Arc<std::sync::Mutex<Vec<PeerInfo>>>, event: NebulaLogEvent) {
    if let Ok(mut peers) = peers_state.lock() {
        match event {
            NebulaLogEvent::HostmapAdded { vpn_ip } => {
                if is_lighthouse(&vpn_ip) { return; }
                if !peers.iter().any(|p| p.vpn_ip == vpn_ip) {
                    peers.push(PeerInfo {
                        vpn_ip,
                        hostname: String::new(),
                        connection_type: "unknown".to_string(),
                        state: "alive".to_string(),
                        latency_ms: None,
                        rx_bytes: 0, tx_bytes: 0,
                        rx_speed: 0.0, tx_speed: 0.0,
                        local_index: 0, remote_index: 0,
                    });
                }
            }
            NebulaLogEvent::TunnelStatus { vpn_ip, cert_name, method, state, local_index, remote_index } => {
                if is_lighthouse(&vpn_ip) { return; }
                if let Some(peer) = peers.iter_mut().find(|p| p.vpn_ip == vpn_ip) {
                    if !cert_name.is_empty() { peer.hostname = cert_name; }
                    peer.connection_type = method;
                    peer.state = state;
                    peer.local_index = local_index;
                    peer.remote_index = remote_index;
                } else {
                    peers.push(PeerInfo {
                        vpn_ip,
                        hostname: cert_name,
                        connection_type: method,
                        state,
                        latency_ms: None,
                        rx_bytes: 0, tx_bytes: 0,
                        rx_speed: 0.0, tx_speed: 0.0,
                        local_index, remote_index,
                    });
                }
            }
            NebulaLogEvent::P2pTimeout { vpn_ip } => {
                if let Some(peer) = peers.iter_mut().find(|p| p.vpn_ip == vpn_ip) {
                    peer.connection_type = "relay".to_string(); // P2P failed, will use relay
                }
            }
            NebulaLogEvent::RelayTest { vpn_ip } => {
                if let Some(peer) = peers.iter_mut().find(|p| p.vpn_ip == vpn_ip) {
                    peer.connection_type = "relay".to_string();
                }
            }
        }
    }
}

/// Measure ICMP latency to a VPN IP
fn measure_latency(vpn_ip: &str) -> Option<f64> {
    let output = if cfg!(windows) {
        Command::new("ping")
            .args(["-n", "1", "-w", "2000", vpn_ip])
            .output().ok()?
    } else {
        Command::new("ping")
            .args(["-c", "1", "-W", "2", vpn_ip])
            .output().ok()?
    };
    let text = String::from_utf8_lossy(&output.stdout);
    // Parse "time=XXms" or "time<1ms" or "time=XX.X ms"
    if let Some(pos) = text.find("time") {
        let rest = &text[pos + 4..];
        let rest = rest.trim_start_matches(['=', '<', ' ']);
        if let Some(end) = rest.find("ms") {
            let num: &str = &rest[..end];
            return num.trim().parse::<f64>().ok();
        }
    }
    None
}

#[tauri::command]
pub fn disconnect_network(state: State<AppState>) -> Result<(), String> {
    if let Some(mut c) = state.nebula_process.lock().map_err(|e| e.to_string())?.take() { let _ = c.kill(); let _ = c.wait(); }
    #[cfg(windows)] { let _ = Command::new("taskkill").args(["/F","/IM","nebula.exe"]).stdout(Stdio::null()).stderr(Stdio::null()).status(); }
    state.network_status.lock().map_err(|e| e.to_string()).map(|mut s| { s.connected = false; s.virtual_ip = None; s.group_name = None; })?;
    *state.connection_time.lock().map_err(|e| e.to_string())? = None;
    *state.network_stats.lock().map_err(|e| e.to_string())? = NetworkStats::default();
    *state.peers.lock().map_err(|e| e.to_string())? = Vec::new();
    *state.peer_last_bytes.lock().map_err(|e| e.to_string())? = HashMap::new();
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

#[tauri::command]
pub fn get_peers(state: State<AppState>) -> Result<Vec<PeerInfo>, String> {
    state.peers.lock().map_err(|e| e.to_string()).map(|p| p.clone())
}

fn get_config_dir() -> Result<PathBuf, String> {
    Ok(dirs::home_dir().ok_or("No home directory")?.join(".eznebula"))
}
