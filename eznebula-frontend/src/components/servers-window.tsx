import { useState, useEffect, useCallback } from "react"
import { Button } from "@/components/ui/button"
import { Plus, Pencil, Trash2 } from "lucide-react"
import { toast } from "sonner"
import { eznebulaApi, type ServerEntry } from "@/lib/api"
import ServerForm from "./server-form"

export default function ServersWindow() {
  const [servers, setServers] = useState<ServerEntry[]>([])
  const [formOpen, setFormOpen] = useState(false)
  const [editServer, setEditServer] = useState<ServerEntry | null>(null)

  const load = useCallback(async () => {
    try { setServers(await eznebulaApi.getServers()) } catch { /* ignore */ }
  }, [])

  useEffect(() => { load() }, [load])

  const handleSave = async (name: string, address: string, port: number) => {
    try {
      await eznebulaApi.saveServer(name, address, port)
      await load()
      setFormOpen(false)
      setEditServer(null)
      toast.success("服务器已保存")
    } catch (e: any) { toast.error("保存失败", { description: String(e) }) }
  }

  const handleDelete = async (id: string) => {
    try {
      await eznebulaApi.deleteServer(id)
      await load()
      toast.success("已删除")
    } catch (e: any) { toast.error("删除失败", { description: String(e) }) }
  }

  return (
    <div className="h-screen bg-background flex flex-col">
      <div className="p-2 border-b flex items-center justify-between">
        <h1 className="text-sm font-bold">服务器列表</h1>
        <Button size="xs" variant="ghost" onClick={() => { setEditServer(null); setFormOpen(true) }}>
          <Plus className="size-3" />
        </Button>
      </div>
      <div className="flex-1 overflow-auto p-2">
        {servers.length === 0 ? (
          <p className="text-xs text-muted-foreground text-center mt-4">暂无服务器</p>
        ) : (
          <div className="space-y-1">
            {servers.map(s => (
              <div key={s.id} className="flex items-center justify-between p-2 border rounded-md">
                <div>
                  <div className="text-xs font-medium">{s.name || s.address}</div>
                  <div className="text-[10px] text-muted-foreground">{s.address}:{s.port}</div>
                </div>
                <div className="flex items-center gap-1">
                  <Button variant="ghost" size="icon-xs" onClick={() => { setEditServer(s); setFormOpen(true) }}>
                    <Pencil className="size-3" />
                  </Button>
                  <Button variant="ghost" size="icon-xs" onClick={() => handleDelete(s.id)}>
                    <Trash2 className="size-3" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      {formOpen && (
        <ServerForm
          initial={editServer}
          onSave={handleSave}
          onCancel={() => { setFormOpen(false); setEditServer(null) }}
        />
      )}
    </div>
  )
}
