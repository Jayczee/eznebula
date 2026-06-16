import { useState, useEffect } from "react"

export default function SettingsWindow() {
  const [closeBehavior, setCloseBehavior] = useState("minimize")

  useEffect(() => {
    import("@/lib/api").then(({ eznebulaApi }) => {
      eznebulaApi.getCloseBehavior().then(setCloseBehavior).catch(() => {})
    })
  }, [])

  const handleSet = async (b: string) => {
    setCloseBehavior(b)
    try {
      const { eznebulaApi } = await import("@/lib/api")
      await eznebulaApi.setCloseBehavior(b)
    } catch { /* ignore */ }
  }

  return (
    <div className="h-screen bg-background flex flex-col">
      <div className="p-3 border-b">
        <h1 className="text-sm font-bold">通用</h1>
      </div>
      <div className="flex-1 overflow-auto p-3">
        <h2 className="text-xs font-semibold mb-2">关闭行为</h2>
        <p className="text-[10px] text-muted-foreground mb-2">点击右上角 × 关闭程序时：</p>
        <div className="space-y-1.5">
          <label
            className={`flex items-center gap-2 p-2 border rounded cursor-pointer text-xs ${closeBehavior === "close" ? "border-primary bg-accent" : ""}`}
            onClick={() => handleSet("close")}
          >
            <div className={`size-3 rounded-full border-2 flex items-center justify-center ${closeBehavior === "close" ? "border-primary" : ""}`}>
              {closeBehavior === "close" && <div className="size-1.5 rounded-full bg-primary" />}
            </div>
            直接关闭
            <span className="text-[10px] text-muted-foreground">（关闭前弹窗确认）</span>
          </label>
          <label
            className={`flex items-center gap-2 p-2 border rounded cursor-pointer text-xs ${closeBehavior === "minimize" ? "border-primary bg-accent" : ""}`}
            onClick={() => handleSet("minimize")}
          >
            <div className={`size-3 rounded-full border-2 flex items-center justify-center ${closeBehavior === "minimize" ? "border-primary" : ""}`}>
              {closeBehavior === "minimize" && <div className="size-1.5 rounded-full bg-primary" />}
            </div>
            最小化到托盘
          </label>
        </div>
      </div>
    </div>
  )
}
