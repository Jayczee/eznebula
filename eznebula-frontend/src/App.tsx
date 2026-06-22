import { useState, useEffect, useCallback, useRef } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Checkbox } from "@/components/ui/checkbox"
import { Badge } from "@/components/ui/badge"
import { ScrollText, Users, Plug, PlugZap, Server, Settings, ArrowUp, ArrowDown, ChevronDown } from "lucide-react"
import { listen } from "@tauri-apps/api/event"
import { eznebulaApi, type NetworkStatus, type NetworkStats, type ServerEntry } from "@/lib/api"

type ConnState = "idle" | "connecting" | "connected" | "stopping"

function fmtBytes(b: number) {
  if (b >= 1_073_741_824) return `${(b / 1_073_741_824).toFixed(1)} GB`
  if (b >= 1_048_576) return `${(b / 1_048_576).toFixed(1)} MB`
  if (b >= 1024) return `${(b / 1024).toFixed(0)} KB`
  return `${b} B`
}

export default function App() {
  const [serverUrl, setServerUrl] = useState("")
  const [groupName, setGroupName] = useState("")
  const [clientName, setClientName] = useState(() => { const t = new Date(); return `pc-${t.getHours().toString().padStart(2,"0")}${t.getMinutes().toString().padStart(2,"0")}` })
  const [forceRelay, setForceRelay] = useState(false)
  const [connState, setConnState] = useState<ConnState>("idle")
  const [status, setStatus] = useState<NetworkStatus | null>(null)
  const [stats, setStats] = useState<NetworkStats | null>(null)
  const [speedRx, setSpeedRx] = useState(0)
  const [speedTx, setSpeedTx] = useState(0)
  const [confirmCloseOpen, setConfirmCloseOpen] = useState(false)
  const [errorOpen, setErrorOpen] = useState(false)
  const [errorMsg, setErrorMsg] = useState("")
  const [savedServers, setSavedServers] = useState<ServerEntry[]>([])
  const [serverRttMap, setServerRttMap] = useState<Record<string, number | null>>({})
  const [serverDropdownOpen, setServerDropdownOpen] = useState(false)
  const serverInputRef = useRef<HTMLInputElement>(null)

  const isConnected = connState === "connected"
  const isConnecting = connState === "connecting"
  const idle = connState === "idle"
  const canConnect = !!(serverUrl.trim() && groupName.trim() && idle)

  useEffect(() => { const u = listen("confirm-close", () => setConfirmCloseOpen(true)); return () => { u.then(f => f()) } }, [])
  useEffect(() => { if (!isConnected) return; const t = setInterval(async () => { try { const [s,st] = await Promise.all([eznebulaApi.getStatus(), eznebulaApi.getNetworkStats()]); setStatus(s); setStats(st); setSpeedRx(st.rx_speed); setSpeedTx(st.tx_speed) } catch {} }, 1000); return () => clearInterval(t) }, [isConnected])

  const handleServerInputFocus = useCallback(async () => {
    if (!idle) return
    try {
      const list = await eznebulaApi.getServers()
      if (list.length === 0) return
      setSavedServers(list)
      setServerDropdownOpen(true)
      // 并行测量 RTT
      const rtts: Record<string, number | null> = {}
      await Promise.all(
        list.map(async (s) => {
          try {
            rtts[s.id] = await eznebulaApi.measureServerRtt(s.address, s.port)
          } catch {
            rtts[s.id] = null
          }
        })
      )
      setServerRttMap(rtts)
    } catch {}
  }, [idle])

  const handleSelectServer = useCallback((s: ServerEntry) => {
    const proto = s.port === 443 ? "https" : "http"
    const portSuffix = (proto === "https" && s.port === 443) || (proto === "http" && s.port === 80) ? "" : `:${s.port}`
    setServerUrl(`${proto}://${s.address}${portSuffix}`)
    if (s.default_group) setGroupName(s.default_group)
    if (s.default_device) setClientName(s.default_device)
    setServerDropdownOpen(false)
  }, [])

  const doConnect = useCallback(async () => { setConnState("connecting"); try { await eznebulaApi.joinNetwork({ server_url: serverUrl.trim(), group_name: groupName.trim(), join_token: "", client_name: clientName.trim() || "eznebula-node" }); setConnState("connected"); setStatus(await eznebulaApi.getStatus()) } catch (e: any) { setErrorMsg(String(e)); setErrorOpen(true); setConnState("idle") } }, [serverUrl, groupName, clientName])
  const doDisconnect = useCallback(async () => { setConnState("stopping"); try { await eznebulaApi.disconnectNetwork() } catch (e: any) { setErrorMsg(String(e)); setErrorOpen(true) } finally { setConnState("idle"); setStatus(null); setStats(null); setSpeedRx(0); setSpeedTx(0) } }, [])

  return (
    <div className="h-screen bg-background p-3 flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {isConnected ? <Badge className="text-[10px] h-4 px-1.5 bg-green-600">已连接</Badge> : isConnecting ? <Badge className="text-[10px] h-4 px-1.5" variant="secondary">启动中</Badge> : <Badge className="text-[10px] h-4 px-1.5" variant="secondary">未连接</Badge>}
        </div>
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="icon-xs" title="服务器列表" onClick={() => eznebulaApi.openWindow("servers","服务器列表",520,480)}><Server className="size-3.5"/></Button>
          <Button variant="ghost" size="icon-xs" title="在线客户端" onClick={() => eznebulaApi.openWindow("peers","在线客户端",380,480)}><Users className="size-3.5"/></Button>
          <Button variant="ghost" size="icon-xs" title="设置" onClick={() => eznebulaApi.openWindow("settings","设置",420,400)}><Settings className="size-3.5"/></Button>
          <Button variant="ghost" size="icon-xs" title="运行日志" onClick={() => eznebulaApi.openWindow("logs","运行日志",520,500)}><ScrollText className="size-3.5"/></Button>
        </div>
      </div>
      <div className="flex flex-col gap-3">
        <div className="w-full max-w-md mx-auto">
          <div className="flex items-center justify-between mb-0.5">
            <Label className="text-[10px]">服务器</Label>
            <div className="flex items-center gap-1"><Checkbox id="relay" checked={forceRelay} onCheckedChange={c => setForceRelay(c === true)} disabled={!idle} className="h-3 w-3"/><Label htmlFor="relay" className="text-[10px] cursor-pointer whitespace-nowrap">强制中转</Label></div>
          </div>
          <div className="relative">
            <Input
              ref={serverInputRef}
              placeholder=""
              value={serverUrl}
              onChange={e => setServerUrl(e.target.value)}
              onFocus={handleServerInputFocus}
              onBlur={() => setTimeout(() => setServerDropdownOpen(false), 200)}
              disabled={!idle}
              className="h-7 text-xs text-center pr-7"
            />
            <button
              type="button"
              className="absolute right-1 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground p-0.5"
              onClick={() => {
                if (!idle) return
                if (serverDropdownOpen) { setServerDropdownOpen(false); return }
                handleServerInputFocus()
              }}
              tabIndex={-1}
            >
              <ChevronDown className={`size-3.5 transition-transform ${serverDropdownOpen ? "rotate-180" : ""}`} />
            </button>

            {/* 服务器下拉列表 */}
            {serverDropdownOpen && savedServers.length > 0 && (
              <div className="absolute top-full left-0 right-0 mt-1 z-50 bg-popover border rounded-md shadow-md max-h-48 overflow-auto">
                {savedServers.map(s => (
                  <button
                    key={s.id}
                    type="button"
                    className="w-full text-left px-3 py-1.5 hover:bg-accent flex items-center justify-between gap-2 text-xs"
                    onMouseDown={e => { e.preventDefault(); handleSelectServer(s) }}
                  >
                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">{s.name || s.address}</div>
                      <div className="text-[10px] text-muted-foreground">{s.address}:{s.port}</div>
                    </div>
                    <div className="text-[10px] text-muted-foreground shrink-0">
                      {serverRttMap[s.id] === undefined ? (
                        <span className="text-blue-500">测试中...</span>
                      ) : serverRttMap[s.id] === null ? (
                        <span className="text-red-500">超时</span>
                      ) : (
                        <span className={serverRttMap[s.id]! < 100 ? "text-green-600" : serverRttMap[s.id]! < 300 ? "text-yellow-600" : "text-orange-600"}>
                          {serverRttMap[s.id]} ms
                        </span>
                      )}
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
        <div className="w-full max-w-md mx-auto flex items-end gap-3">
          <div className="flex-1"><Label className="text-[10px]">组名</Label><Input placeholder="" value={groupName} onChange={e => setGroupName(e.target.value)} disabled={!idle} className="h-7 text-xs text-center"/></div>
          <div className="flex-1"><Label className="text-[10px]">设备名</Label><Input placeholder="" value={clientName} onChange={e => setClientName(e.target.value)} disabled={!idle} className="h-7 text-xs text-center"/></div>
        </div>
        <div className="w-full max-w-md mx-auto flex justify-center">
          {isConnected ? <Button onClick={doDisconnect} variant="destructive" size="xs"><PlugZap className="size-3 mr-1"/>断开</Button> : isConnecting ? <Button onClick={doDisconnect} variant="secondary" size="xs"><Plug className="size-3 mr-1 animate-pulse"/>取消</Button> : <Button onClick={doConnect} disabled={!canConnect} size="xs"><Plug className="size-3 mr-1"/>连接</Button>}
        </div>
      </div>
      {isConnected && <div className="border-t pt-1.5 flex items-center gap-2 text-[10px]"><span className="flex items-center gap-0.5 text-green-600 shrink-0"><ArrowDown className="size-3"/>{fmtBytes(stats?.rx_bytes ?? 0)}</span><span className="flex items-center gap-0.5 text-blue-600 shrink-0"><ArrowUp className="size-3"/>{fmtBytes(stats?.tx_bytes ?? 0)}</span><span className="text-muted-foreground shrink-0">{status?.virtual_ip ?? ""}</span><span className="text-green-600 shrink-0 font-semibold">{fmtBytes(Math.round(speedRx))}/s</span><span className="text-blue-600 shrink-0 font-semibold">{fmtBytes(Math.round(speedTx))}/s</span></div>}
      {confirmCloseOpen && <div className="fixed inset-0 z-50 flex items-center justify-center"><div className="absolute inset-0 bg-black/40" onClick={() => setConfirmCloseOpen(false)}/><div className="relative bg-background rounded-lg border shadow-lg w-72 p-4"><h3 className="text-sm font-semibold mb-1">退出 EZNebula</h3><p className="text-xs text-muted-foreground mb-3">确定要退出程序吗？退出后连接将断开。</p><div className="flex justify-end gap-2"><Button variant="outline" size="xs" onClick={() => setConfirmCloseOpen(false)}>取消</Button><Button variant="destructive" size="xs" onClick={() => eznebulaApi.quitApp()}>退出</Button></div></div></div>}
      {errorOpen && <div className="fixed inset-0 z-50 flex items-center justify-center"><div className="absolute inset-0 bg-black/40" onClick={() => setErrorOpen(false)}/><div className="relative bg-background rounded-lg border shadow-lg w-80 p-4"><h3 className="text-sm font-semibold mb-2">错误</h3><div className="text-xs text-muted-foreground max-h-40 overflow-auto whitespace-pre-wrap break-all mb-3">{errorMsg}</div><div className="flex justify-end"><Button size="xs" onClick={() => setErrorOpen(false)}>确认</Button></div></div></div>}
    </div>
  )
}
