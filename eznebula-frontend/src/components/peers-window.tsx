import { useState, useEffect, useCallback } from "react"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { RefreshCw, Zap, Satellite, ArrowDown, ArrowUp } from "lucide-react"
import { listen } from "@tauri-apps/api/event"
import { eznebulaApi, type NetworkStatus, type PeerInfo } from "@/lib/api"

function fmtBytes(b: number) {
  if (b >= 1_073_741_824) return `${(b / 1_073_741_824).toFixed(1)} GB`
  if (b >= 1_048_576) return `${(b / 1_048_576).toFixed(1)} MB`
  if (b >= 1024) return `${(b / 1024).toFixed(0)} KB`
  return `${b} B`
}

function fmtSpeed(bps: number) {
  if (bps >= 1_048_576) return `${(bps / 1_048_576).toFixed(1)} MB/s`
  if (bps >= 1024) return `${(bps / 1024).toFixed(0)} KB/s`
  return `${bps.toFixed(0)} B/s`
}

function LatencyBadge({ ms }: { ms: number | null }) {
  if (ms === null) return <span className="text-[10px] text-muted-foreground">—</span>
  const color = ms < 50 ? "text-green-600" : ms < 100 ? "text-yellow-600" : "text-red-600"
  return <span className={`text-[10px] font-mono ${color}`}>{ms.toFixed(0)}ms</span>
}

export default function PeersWindow() {
  const [status, setStatus] = useState<NetworkStatus | null>(null)
  const [peers, setPeers] = useState<PeerInfo[]>([])
  const [refreshing, setRefreshing] = useState(false)

  // Poll status every 3s
  const refresh = useCallback(async () => {
    setRefreshing(true)
    try { setStatus(await eznebulaApi.getStatus()) } catch {}
    setRefreshing(false)
  }, [])

  useEffect(() => {
    refresh()
    const t = setInterval(refresh, 3000)
    return () => clearInterval(t)
  }, [refresh])

  // Listen for real-time peer updates from backend
  useEffect(() => {
    const unlisten = listen<PeerInfo[]>("peers-updated", (event) => {
      setPeers(event.payload)
    })
    return () => { unlisten.then(f => f()) }
  }, [])

  // Also poll on mount for initial state
  useEffect(() => {
    eznebulaApi.getPeers().then(p => { if (p.length > 0) setPeers(p) }).catch(() => {})
  }, [])

  return (
    <div className="h-screen bg-background flex flex-col">
      <div className="p-2 border-b flex items-center justify-between">
        <h1 className="text-sm font-bold">组内客户端</h1>
        <Button size="xs" variant="ghost" onClick={refresh} disabled={refreshing}>
          <RefreshCw className={`size-3 ${refreshing ? "animate-spin" : ""}`} />
        </Button>
      </div>
      <div className="flex-1 overflow-auto p-2">
        {!status?.connected ? (
          <p className="text-xs text-muted-foreground text-center mt-4">未连接</p>
        ) : (
          <div className="space-y-2">
            {/* 本机 */}
            <div className="flex items-center justify-between p-2 border rounded-md bg-accent/30">
              <div>
                <div className="text-xs font-medium">本机</div>
                <div className="text-[10px] text-muted-foreground font-mono">{status.virtual_ip}</div>
              </div>
              <Badge className="text-[10px] h-4 px-1.5 bg-green-600">在线</Badge>
            </div>

            {/* 其他节点 */}
            {peers.length === 0 && (
              <p className="text-[10px] text-muted-foreground text-center mt-2">
                等待其他客户端上线...
              </p>
            )}
            {peers.map((peer) => (
              <div key={peer.vpn_ip} className="p-2 border rounded-md space-y-1.5">
                {/* 头部: 名称 + IP + 状态 */}
                <div className="flex items-center justify-between">
                  <div>
                    <div className="text-xs font-medium">
                      {peer.hostname || peer.vpn_ip}
                    </div>
                    <div className="text-[10px] text-muted-foreground font-mono">
                      {peer.vpn_ip}
                    </div>
                  </div>
                  <div className="flex items-center gap-1.5">
                    {/* 连接方式 */}
                    {peer.connection_type === "p2p" ? (
                      <span className="flex items-center gap-0.5 text-[10px] text-green-600" title="P2P 直连">
                        <Zap className="size-3" /> P2P
                      </span>
                    ) : peer.connection_type === "relay" ? (
                      <span className="flex items-center gap-0.5 text-[10px] text-yellow-600" title="灯塔中转">
                        <Satellite className="size-3" /> 中转
                      </span>
                    ) : (
                      <span className="text-[10px] text-muted-foreground">...</span>
                    )}
                    {/* 延迟 */}
                    <LatencyBadge ms={peer.latency_ms} />
                    {/* 在线状态 */}
                    <Badge className={`text-[10px] h-4 px-1.5 ${
                      peer.state === "alive" ? "bg-green-600" : peer.state === "testing" ? "bg-yellow-600" : "bg-red-600"
                    }`}>
                      {peer.state === "alive" ? "在线" : peer.state === "testing" ? "检测中" : "离线"}
                    </Badge>
                  </div>
                </div>
                {/* 流量 */}
                <div className="flex items-center gap-3 text-[10px]">
                  <span className="flex items-center gap-0.5 text-green-600">
                    <ArrowDown className="size-2.5" />
                    {fmtBytes(peer.rx_bytes)}
                  </span>
                  <span className="text-green-600 font-semibold">{fmtSpeed(peer.rx_speed)}</span>
                  <span className="flex items-center gap-0.5 text-blue-600">
                    <ArrowUp className="size-2.5" />
                    {fmtBytes(peer.tx_bytes)}
                  </span>
                  <span className="text-blue-600 font-semibold">{fmtSpeed(peer.tx_speed)}</span>
                </div>
              </div>
            ))}
            {/* 底部信息 */}
            <p className="text-[10px] text-muted-foreground text-center mt-2">
              组: {status.group_name} · {peers.length + 1} 在线 · 运行 {status.uptime_seconds}s
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
