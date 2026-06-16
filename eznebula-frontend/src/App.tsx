import { useState, useEffect, useCallback } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Checkbox } from "@/components/ui/checkbox"
import { Badge } from "@/components/ui/badge"
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog"
import { ScrollText, Users, Plug, PlugZap, Server, Settings, ArrowUp, ArrowDown } from "lucide-react"
import { eznebulaApi, type NetworkStatus, type NetworkStats } from "@/lib/api"

type ConnState = "idle" | "connecting" | "connected" | "stopping"

function fmtBytes(b: number) {
  if (b >= 1_073_741_824) return `${(b / 1_073_741_824).toFixed(1)} GB`
  if (b >= 1_048_576) return `${(b / 1_048_576).toFixed(1)} MB`
  if (b >= 1024) return `${(b / 1024).toFixed(0)} KB`
  return `${b} B`
}

export default function App() {
  const [serverUrl, setServerUrl] = useState("http://116.62.206.205:52346")
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

  const [errorOpen, setErrorOpen] = useState(false)
  const [errorTitle, setErrorTitle] = useState("")
  const [errorMessage, setErrorMessage] = useState("")

  const isConnected = connState === "connected"
  const isConnecting = connState === "connecting"
  const isStopping = connState === "stopping"
  const idle = connState === "idle"
  const canConnect = !serverUrl.trim() || !groupName.trim() ? false : idle

  const showError = useCallback((title: string, msg: string) => {
    setErrorTitle(title); setErrorMessage(msg); setErrorOpen(true)
  }, [])

  useEffect(() => {
    if (!isConnected) return
    const t = setInterval(async () => {
      try {
        const [s, st] = await Promise.all([eznebulaApi.getStatus(), eznebulaApi.getNetworkStats()])
        setStatus(s); setStats(st); setSpeedRx(st.rx_speed); setSpeedTx(st.tx_speed)
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
      const s = await eznebulaApi.getStatus(); setStatus(s)
    } catch (e: any) { showError("连接失败", String(e)); setConnState("idle") }
  }, [serverUrl, groupName, clientName, showError])

  const handleDisconnect = useCallback(async () => {
    setConnState("stopping")
    try { await eznebulaApi.disconnectNetwork() }
    catch (e: any) { showError("断开失败", String(e)) }
    finally { setConnState("idle"); setStatus(null); setStats(null); setSpeedRx(0); setSpeedTx(0) }
  }, [showError])

  return (
    <div className="h-screen bg-background p-3 flex flex-col gap-2">
      {/* ---- 标题栏 ---- */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {isConnected ? (
            <Badge className="text-[10px] h-4 px-1.5 bg-green-600">已连接</Badge>
          ) : isConnecting || isStopping ? (
            <Badge className="text-[10px] h-4 px-1.5" variant="secondary">
              {isConnecting ? "启动中" : "断开中"}
            </Badge>
          ) : (
            <Badge className="text-[10px] h-4 px-1.5" variant="secondary">未连接</Badge>
          )}
        </div>
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="icon-xs" title="服务器列表"
            onClick={() => eznebulaApi.openWindow("servers", "服务器列表", 520, 480)}>
            <Server className="size-3.5" />
          </Button>
          <Button variant="ghost" size="icon-xs" title="在线客户端"
            onClick={() => eznebulaApi.openWindow("peers", "在线客户端", 380, 480)}>
            <Users className="size-3.5" />
          </Button>
          <Button variant="ghost" size="icon-xs" title="设置">
            <Settings className="size-3.5" />
          </Button>
          <Button variant="ghost" size="icon-xs" title="运行日志"
            onClick={() => eznebulaApi.openWindow("logs", "运行日志", 520, 500)}>
            <ScrollText className="size-3.5" />
          </Button>
        </div>
      </div>

      {/* ---- 配置区 ---- */}
      <div className="flex flex-col gap-3">
        {/* 服务器地址 */}
        <div className="w-full max-w-md mx-auto">
          <div className="flex items-center justify-between mb-0.5">
            <Label className="text-[10px]">服务器</Label>
            <div className="flex items-center gap-1">
              <Checkbox id="force-relay" checked={forceRelay} onCheckedChange={c => setForceRelay(c === true)}
                disabled={!idle} className="h-3 w-3" />
              <Label htmlFor="force-relay" className="text-[10px] cursor-pointer whitespace-nowrap">强制中转</Label>
            </div>
          </div>
          <Input
            placeholder="http://server:52346"
            value={serverUrl}
            onChange={e => setServerUrl(e.target.value)}
            disabled={!idle}
            className="h-7 text-xs text-center"
          />
        </div>

        {/* 组名和设备名 - 一行 */}
        <div className="w-full max-w-md mx-auto flex items-end gap-3">
          <div className="flex-1">
            <Label className="text-[10px]">组名</Label>
            <Input placeholder="" value={groupName} onChange={e => setGroupName(e.target.value)}
              disabled={!idle} className="h-7 text-xs text-center" />
          </div>
          <div className="flex-1">
            <Label className="text-[10px]">设备名</Label>
            <Input placeholder="" value={clientName} onChange={e => setClientName(e.target.value)}
              disabled={!idle} className="h-7 text-xs text-center" />
          </div>
        </div>

        {/* 连接按钮 - 居中 */}
        <div className="w-full max-w-md mx-auto flex justify-center">
          {isConnected && !isStopping ? (
            <Button onClick={handleDisconnect} variant="destructive" size="xs">
              <PlugZap className="size-3 mr-1" />断开
            </Button>
          ) : isStopping ? (
            <Button disabled size="xs" variant="outline">断开中...</Button>
          ) : isConnecting ? (
            <Button onClick={handleDisconnect} variant="secondary" size="xs">
              <Plug className="size-3 mr-1 animate-pulse" />取消
            </Button>
          ) : (
            <Button onClick={handleConnect} disabled={!canConnect} size="xs">
              <Plug className="size-3 mr-1" />连接
            </Button>
          )}
        </div>
      </div>

      {/* ---- 流量监控 ---- */}
      {isConnected && (
        <div className="border-t pt-1.5 flex items-center gap-2 text-[10px]">
          <span className="flex items-center gap-0.5 text-green-600 shrink-0" title="总接收字节">
            <ArrowDown className="size-3" />{fmtBytes(stats?.rx_bytes ?? 0)}
          </span>
          <span className="flex items-center gap-0.5 text-blue-600 shrink-0" title="总发送字节">
            <ArrowUp className="size-3" />{fmtBytes(stats?.tx_bytes ?? 0)}
          </span>
          <span className="text-muted-foreground shrink-0">
            {status?.virtual_ip ?? ""}
          </span>
          <span className="text-green-600 shrink-0 font-semibold">
            {fmtBytes(speedRx)}/s
          </span>
          <span className="text-blue-600 shrink-0 font-semibold">
            {fmtBytes(speedTx)}/s
          </span>
        </div>
      )}

      {/* ---- 错误弹窗 ---- */}
      <Dialog open={errorOpen} onOpenChange={setErrorOpen}>
        <DialogContent className="max-w-sm" onInteractOutside={(e: any) => e.preventDefault()}>
          <DialogHeader>
            <DialogTitle className="text-sm">{errorTitle}</DialogTitle>
          </DialogHeader>
          <div className="text-xs text-muted-foreground max-h-40 overflow-auto whitespace-pre-wrap break-all">
            {errorMessage}
          </div>
          <DialogFooter>
            <Button size="xs" onClick={() => setErrorOpen(false)}>确认</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
