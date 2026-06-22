import { useState, useEffect, useRef, useMemo } from "react"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Search } from "lucide-react"
import { listen } from "@tauri-apps/api/event"

interface LogEntry {
  level: string  // "error" | "warning" | "info" | "debug"
  msg: string
  source: string // "nebula" | "frontend"
  time: string
}

function timeNow() {
  const d = new Date()
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`
}

export default function LogsWindow() {
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [search, setSearch] = useState("")
  const bottomRef = useRef<HTMLDivElement>(null)
  const maxLines = 3000

  const addLog = (entry: LogEntry) => {
    setLogs(prev => {
      const next = [...prev, entry]
      if (next.length > maxLines) return next.slice(-maxLines)
      return next
    })
  }

  // Capture frontend console.log/error/warn
  useEffect(() => {
    const orig = { log: console.log, error: console.error, warn: console.warn }
    console.log = (...a: any[]) => {
      orig.log(...a)
      addLog({ level: "info", source: "frontend", time: timeNow(), msg: a.map(x => typeof x === "string" ? x : JSON.stringify(x)).join(" ") })
    }
    console.error = (...a: any[]) => {
      orig.error(...a)
      addLog({ level: "error", source: "frontend", time: timeNow(), msg: a.map(x => typeof x === "string" ? x : JSON.stringify(x)).join(" ") })
    }
    console.warn = (...a: any[]) => {
      orig.warn(...a)
      addLog({ level: "warning", source: "frontend", time: timeNow(), msg: a.map(x => typeof x === "string" ? x : JSON.stringify(x)).join(" ") })
    }
    return () => { console.log = orig.log; console.error = orig.error; console.warn = orig.warn }
  }, [])

  // Listen for Nebula log events from backend
  useEffect(() => {
    const unlisten = listen<{ level: string; msg: string }>("nebula-log", (event) => {
      addLog({ level: event.payload.level, source: "nebula", time: timeNow(), msg: event.payload.msg })
    })
    return () => { unlisten.then(f => f()) }
  }, [])

  useEffect(() => { bottomRef.current?.scrollIntoView({ behavior: "instant" }) }, [logs])

  const filtered = useMemo(() => {
    if (!search.trim()) return logs
    const q = search.toLowerCase()
    return logs.filter(l => l.msg.toLowerCase().includes(q))
  }, [logs, search])

  const levelColor = (level: string) => {
    switch (level) {
      case "error": return "text-red-500"
      case "warning": return "text-yellow-500"
      case "debug": return "text-gray-400"
      default: return ""
    }
  }

  const sourceTag = (s: string) => s === "nebula" ? "[N]" : "[F]"

  return (
    <div className="h-screen bg-background flex flex-col">
      <div className="p-2 border-b space-y-2">
        <div className="flex items-center justify-between">
          <h1 className="text-sm font-bold">运行日志</h1>
          <span className="text-[10px] text-muted-foreground">
            {logs.length} 条{search && filtered.length !== logs.length ? ` / 显示 ${filtered.length}` : ""}
            {" "}<span className="text-muted-foreground/60">[N]=Nebula [F]=前端</span>
          </span>
        </div>
        <div className="relative">
          <Search className="size-3 absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="搜索日志..."
            value={search}
            onChange={e => setSearch(e.target.value)}
            className="h-6 text-[10px] pl-6"
          />
        </div>
      </div>
      <div className="flex-1 overflow-hidden p-2">
        <ScrollArea className="h-full">
          <div className="bg-muted rounded-md p-2 font-mono text-[10px] leading-relaxed">
            {filtered.length === 0 ? (
              <p className="text-muted-foreground">
                {search ? "无匹配结果" : logs.length === 0 ? "等待日志..." : "无日志"}
              </p>
            ) : (
              filtered.map((l, i) => (
                <div key={i} className={`break-all ${levelColor(l.level)}`}>
                  <span className="text-muted-foreground/60">{l.time}</span>
                  {" "}
                  <span className="text-muted-foreground/40">{sourceTag(l.source)}</span>
                  {" "}
                  {l.msg}
                </div>
              ))
            )}
            <div ref={bottomRef} />
          </div>
        </ScrollArea>
      </div>
    </div>
  )
}
