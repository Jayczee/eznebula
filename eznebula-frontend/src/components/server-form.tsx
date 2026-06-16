import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import type { ServerEntry } from "@/lib/api"

export default function ServerForm({ initial, onSave, onCancel }: { initial: ServerEntry | null; onSave: (n: string, a: string, p: number) => void; onCancel: () => void }) {
  const [name, setName] = useState(initial?.name ?? "")
  const [address, setAddress] = useState(initial?.address ?? "")
  const [port, setPort] = useState(initial?.port?.toString() ?? "")
  const submit = () => { if (!address.trim()) return; onSave(name.trim(), address.trim(), parseInt(port) || 8080) }
  return <div className="absolute inset-0 z-50 flex items-center justify-center bg-background/80"><div className="bg-card border rounded-lg p-4 shadow-lg w-64 space-y-3"><h2 className="text-sm font-semibold">{initial ? "编辑" : "添加"}服务器</h2><div className="space-y-2"><div><Label className="text-[10px]">名称</Label><Input value={name} onChange={e => setName(e.target.value)} className="h-7 text-xs" placeholder="我的服务器"/></div><div><Label className="text-[10px]">IP 地址</Label><Input value={address} onChange={e => setAddress(e.target.value)} className="h-7 text-xs" placeholder="192.168.1.1"/></div><div><Label className="text-[10px]">端口</Label><Input value={port} onChange={e => setPort(e.target.value)} className="h-7 text-xs" placeholder="8080"/></div></div><div className="flex justify-end gap-2"><Button size="xs" variant="outline" onClick={onCancel}>取消</Button><Button size="xs" onClick={submit}>保存</Button></div></div></div>
}
