import { invoke } from "@tauri-apps/api/core"
import { WebviewWindow } from "@tauri-apps/api/webviewWindow"

export interface JoinRequest { server_url: string; group_name: string; join_token: string; client_name: string }
export interface NetworkStatus { connected: boolean; virtual_ip?: string; group_name?: string; uptime_seconds: number }
export interface NetworkStats { rx_bytes: number; tx_bytes: number; rx_speed: number; tx_speed: number }
export interface ServerEntry { id: string; name: string; address: string; port: number; default_group: string; default_device: string }

async function openWindowDirect(view: string, title: string, width: number, height: number) {
  const label = `sub_${view}`

  // 直接创建新窗口
  try {
    const webview = new WebviewWindow(label, {
      url: `/?view=${view}`,
      title,
      width,
      height,
      center: true,
      resizable: true,
    })

    webview.once('tauri://created', () => {
      console.log('Window created successfully:', label)
    })

    webview.once('tauri://error', (e) => {
      console.error('Window creation error:', label, e)
    })
  } catch (e) {
    console.error('Failed to create window:', label, e)
  }
}

export const eznebulaApi = {
  joinNetwork: (req: JoinRequest) => invoke<string>("join_network", { request: req }),
  disconnectNetwork: () => invoke<void>("disconnect_network"),
  getStatus: () => invoke<NetworkStatus>("get_status"),
  getNetworkStats: () => invoke<NetworkStats>("get_network_stats"),
  saveServer: (name: string, address: string, port: number, defaultGroup: string, defaultDevice: string) =>
    invoke<ServerEntry>("save_server", { name, address, port, defaultGroup, defaultDevice }),
  getServers: () => invoke<ServerEntry[]>("get_servers"),
  deleteServer: (id: string) => invoke<void>("delete_server", { id }),
  measureServerRtt: (address: string, port: number) => invoke<number | null>("measure_server_rtt", { address, port }),
  openWindow: openWindowDirect,
  setCloseBehavior: (behavior: string) => invoke<void>("set_close_behavior", { behavior }),
  getCloseBehavior: () => invoke<string>("get_close_behavior"),
  quitApp: () => invoke<void>("quit_app"),
}
