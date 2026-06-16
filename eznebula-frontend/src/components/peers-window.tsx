import { useState, useEffect, useCallback } from "react"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { RefreshCw } from "lucide-react"
import { eznebulaApi, type NetworkStatus } from "@/lib/api"

export default function PeersWindow() {
  const [status, setStatus] = useState<NetworkStatus | null>(null)
  const [refreshing, setRefreshing] = useState(false)

  const refresh = useCallback(async () => {
    setRefreshing(true)
    try { setStatus(await eznebulaApi.getStatus()) } catch { /* ignore */ }
    finally { setRefreshing(false) }
  }, [])

  useEffect(() => {
    refresh()
    const t = setInterval(refresh, 3000)
    return () => clearInterval(t)
  }, [refresh])

  return (
    <div className="h-screen bg-background flex flex-col">
      <div className="p-2 border-b flex items-center justify-between">
        <h1 className="text-sm font-bold">在线客户端</h1>
        <Button size="xs" variant="ghost" onClick={refresh} disabled={refreshing}>
          <RefreshCw className={`size-3 ${refreshing ? "animate-spin" : ""}`} />
        </Button>
      </div>
      <div className="flex-1 overflow-auto p-2">
        {!status?.connected ? (
          <p className="text-xs text-muted-foreground text-center mt-4">未连接</p>
        ) : (
          <div className="space-y-1">
            <div className="flex items-center justify-between p-2 border rounded-md">
              <div>
                <div className="text-xs font-medium">本机</div>
                <div className="text-[10px] text-muted-foreground font-mono">{status.virtual_ip}</div>
              </div>
              <Badge className="text-[10px] h-4 px-1.5 bg-green-600">在线</Badge>
            </div>
            <p className="text-[10px] text-muted-foreground text-center mt-2">
              组: {status.group_name} · 运行 {status.uptime_seconds}s
            </p>
          </div>
        )}
      </div>
    </div>
  )
}
