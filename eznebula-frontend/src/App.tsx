import { useState, useEffect, useCallback } from "react"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter,
} from "@/components/ui/dialog"
import { ScrollText, Users, PlugZap, Server, Settings, Loader2 } from "lucide-react"
import { eznebulaApi, type NetworkStatus, type NetworkStats } from "@/lib/api"
import { useTheme } from "@/components/theme-provider"

type ConnState = "idle" | "connecting" | "connected" | "stopping"

function formatBytes(b: number) {
  if (b >= 1_073_741_824) return `${(b / 1_073_741_824).toFixed(1)} GB`
  if (b >= 1_048_576) return `${(b / 1_048_576).toFixed(1)} MB`
  if (b >= 1024) return `${(b / 1024).toFixed(1)} KB`
  return `${b} B`
}

function formatSpeed(bps: number) {
  return `${formatBytes(bps)}/s`
}

export default function App() {
  const [serverUrl, setServerUrl] = useState("http://localhost:8080")
  const [groupName, setGroupName] = useState("")
  const [clientName, setClientName] = useState(() => {
    const t = new Date()
    return `pc-${t.getHours().toString().padStart(2, "0")}${t.getMinutes().toString().padStart(2, "0")}`
  })
  const [forceRelay, setForceRelay] = useState(false)

  const [connState, setConnState] = useState<ConnState>("idle")
  const [status, setStatus] = useState<NetworkStatus | null>(null)
  const [stats, setStats] = useState<NetworkStats | null>(null)
  const [speedRx, setSpeedRx] = useState(0)
  const [speedTx, setSpeedTx] = useState(0)

  // Error dialog
  const [errorOpen, setErrorOpen] = useState(false)
  const [errorTitle, setErrorTitle] = useState("")
  const [errorMessage, setErrorMessage] = useState("")

  const { setTheme, theme: resolvedTheme } = useTheme()

  const isConnected = connState === "connected"
  const isConnecting = connState === "connecting"
  const idle = connState === "idle"
  const canConnect = serverUrl.trim() && groupName.trim() && idle

  const showError = useCallback((title: string, msg: string) => {
    setErrorTitle(title)
    setErrorMessage(msg)
    setErrorOpen(true)
  }, [])

  useEffect(() => {
    if (!isConnected) return
    const t = setInterval(async () => {
      try {
        const [s, st] = await Promise.all([eznebulaApi.getStatus(), eznebulaApi.getNetworkStats()])
        setStatus(s)
        setStats(st)
        setSpeedRx(st.rx_speed)
        setSpeedTx(st.tx_speed)
      } catch { /* ignore */ }
    }, 1000)
    return () => clearInterval(t)
  }, [isConnected])

  const handleConnect = useCallback(async () => {
    setConnState("connecting")
    try {
      await eznebulaApi.joinNetwork({
        server_url: serverUrl.trim(),
        group_name: groupName.trim(),
        join_token: "",
        client_name: clientName.trim() || "eznebula-node",
      })
      setConnState("connected")
      const s = await eznebulaApi.getStatus()
      setStatus(s)
    } catch (e: any) {
      showError("连接失败", String(e))
      setConnState("idle")
    }
  }, [serverUrl, groupName, clientName, showError])

  const handleDisconnect = useCallback(async () => {
    setConnState("stopping")
    try {
      await eznebulaApi.disconnectNetwork()
    } catch (e: any) {
      showError("断开失败", String(e))
    } finally {
      setConnState("idle")
      setStatus(null); setStats(null); setSpeedRx(0); setSpeedTx(0)
    }
  }, [showError])

  return (
    <div className="flex flex-col h-screen gap-2 p-3 select-none">
      {/* ---- Title bar ---- */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold tracking-tight">EZNebula</span>
          {idle && <Badge variant="outline" className="h-5 px-1.5 text-[10px]">未连接</Badge>}
          {isConnecting && (
            <Badge variant="secondary" className="h-5 px-1.5 text-[10px] gap-1">
              <Loader2 className="size-2.5 animate-spin" />连接中
            </Badge>
          )}
          {isConnected && <Badge className="h-5 px-1.5 text-[10px] bg-emerald-500 hover:bg-emerald-500">已连接</Badge>}
        </div>
        <div className="flex items-center gap-0.5">
          <Button variant="ghost" size="icon" className="size-6"><Server className="size-3.5" /></Button>
          <Button variant="ghost" size="icon" className="size-6"><Users className="size-3.5" /></Button>
          <Button variant="ghost" size="icon" className="size-6"
            onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}>
            {resolvedTheme === "dark" ? "☀" : "☾"}
          </Button>
          <Button variant="ghost" size="icon" className="size-6"><ScrollText className="size-3.5" /></Button>
          <Button variant="ghost" size="icon" className="size-6"><Settings className="size-3.5" /></Button>
        </div>
      </div>

      {/* ---- Connection form ---- */}
      <div className="flex flex-col gap-1.5">
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground w-14 shrink-0">服务器</span>
          <Input value={serverUrl} onChange={e => setServerUrl(e.target.value)}
            disabled={!idle} className="flex-1 h-7 text-xs" placeholder="http://server:8080" />
          <div className="flex items-center gap-1 shrink-0">
            <Checkbox id="relay" checked={forceRelay} onCheckedChange={(v: any) => setForceRelay(!!v)} disabled={!idle} />
            <label htmlFor="relay" className="text-[10px] text-muted-foreground cursor-pointer">强制中转</label>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground w-14 shrink-0">组名</span>
          <Input value={groupName} onChange={e => setGroupName(e.target.value)}
            disabled={!idle} className="flex-1 h-7 text-xs" placeholder="dev-team" />
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground w-14 shrink-0">设备名</span>
          <Input value={clientName} onChange={e => setClientName(e.target.value)}
            disabled={!idle} className="w-32 h-7 text-xs" />
          <div className="flex-1" />
          {idle && (
            <Button size="sm" className="h-7 gap-1" disabled={!canConnect} onClick={handleConnect}>
              <PlugZap className="size-3.5" />连接
            </Button>
          )}
          {isConnecting && (
            <Button size="sm" className="h-7 gap-1" disabled>
              <Loader2 className="size-3.5 animate-spin" />连接中
            </Button>
          )}
          {isConnected && (
            <Button size="sm" variant="destructive" className="h-7" onClick={handleDisconnect}>断开</Button>
          )}
        </div>
      </div>

      {/* ---- Status bar ---- */}
      {isConnected && status && (
        <div className="flex items-center gap-3 text-[10px] text-muted-foreground border-t pt-2">
          <span>IP: <span className="font-mono text-foreground">{status.virtual_ip}</span></span>
          <span>组: <span className="text-foreground">{status.group_name}</span></span>
          <span className="text-emerald-500">↓ {formatSpeed(speedRx)}</span>
          <span className="text-blue-500">↑ {formatSpeed(speedTx)}</span>
        </div>
      )}

      {/* ---- Traffic stats ---- */}
      {isConnected && stats && (
        <div className="grid grid-cols-2 gap-2 border-t pt-2">
          <div>
            <div className="text-[10px] text-muted-foreground">接收</div>
            <div className="text-xs font-mono font-medium text-emerald-500">{formatSpeed(stats.rx_speed)}</div>
            <div className="text-[10px] text-muted-foreground">总计 {formatBytes(stats.rx_bytes)}</div>
          </div>
          <div>
            <div className="text-[10px] text-muted-foreground">发送</div>
            <div className="text-xs font-mono font-medium text-blue-500">{formatSpeed(stats.tx_speed)}</div>
            <div className="text-[10px] text-muted-foreground">总计 {formatBytes(stats.tx_bytes)}</div>
          </div>
        </div>
      )}

      {/* ---- Error Dialog ---- */}
      <Dialog open={errorOpen} onOpenChange={setErrorOpen}>
        <DialogContent className="max-w-sm" onInteractOutside={(e: any) => e.preventDefault()}>
          <DialogHeader>
            <DialogTitle className="text-sm">{errorTitle}</DialogTitle>
          </DialogHeader>
          <div className="text-xs text-muted-foreground max-h-40 overflow-auto whitespace-pre-wrap break-all">
            {errorMessage}
          </div>
          <DialogFooter>
            <Button size="sm" onClick={() => setErrorOpen(false)}>确认</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
