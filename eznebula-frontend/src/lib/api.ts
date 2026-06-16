import { invoke } from "@tauri-apps/api/core"

export interface JoinRequest { server_url: string; group_name: string; join_token: string; client_name: string }
export interface NetworkStatus { connected: boolean; virtual_ip?: string; group_name?: string; uptime_seconds: number }
export interface NetworkStats { rx_bytes: number; tx_bytes: number; rx_speed: number; tx_speed: number }
export interface ServerEntry { id: string; name: string; address: string; port: number }

export const eznebulaApi = {
  joinNetwork: (req: JoinRequest) => invoke<string>("join_network", { request: req }),
  disconnectNetwork: () => invoke<void>("disconnect_network"),
  getStatus: () => invoke<NetworkStatus>("get_status"),
  getNetworkStats: () => invoke<NetworkStats>("get_network_stats"),
  saveServer: (name: string, address: string, port: number) => invoke<ServerEntry>("save_server", { name, address, port }),
  getServers: () => invoke<ServerEntry[]>("get_servers"),
  deleteServer: (id: string) => invoke<void>("delete_server", { id }),
  openWindow: (view: string, title: string, width: number, height: number) => invoke<void>("open_window", { view, title, width, height }),
  setCloseBehavior: (behavior: string) => invoke<void>("set_close_behavior", { behavior }),
  getCloseBehavior: () => invoke<string>("get_close_behavior"),
  quitApp: () => invoke<void>("quit_app"),
}
