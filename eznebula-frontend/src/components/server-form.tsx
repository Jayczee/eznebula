import { useState, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

interface Props {
  open: boolean
  onClose: () => void
  onSave: (name: string, address: string, port: number, defaultGroup: string, defaultDevice: string) => void
}

export function ServerForm({ open, onClose, onSave }: Props) {
  const [name, setName] = useState("")
  const [address, setAddress] = useState("")
  const [port, setPort] = useState("")
  const [defaultGroup, setDefaultGroup] = useState("")
  const [defaultDevice, setDefaultDevice] = useState("")
  const [error, setError] = useState("")

  useEffect(() => {
    if (open) {
      setName("")
      setAddress("")
      setPort("")
      setDefaultGroup("")
      setDefaultDevice("")
      setError("")
    }
  }, [open])

  if (!open) return null

  const handleSave = () => {
    if (!address.trim()) { setError("地址不能为空"); return }
    if (!port.trim() || isNaN(Number(port))) { setError("端口号必须为数字"); return }
    const portNum = Number(port)
    if (portNum < 1 || portNum > 65535) { setError("端口号范围 1-65535"); return }

    onSave(name.trim() || address.trim(), address.trim(), portNum, defaultGroup.trim(), defaultDevice.trim())
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/40" onClick={onClose} />
      <div className="relative bg-background rounded-lg border shadow-lg w-[360px] p-4">
        <div className="mb-1">
          <h3 className="text-sm font-semibold">添加服务器</h3>
        </div>
        <div className="space-y-3 mt-3">
          <div>
            <Label className="text-[10px]">名称 <span className="text-muted-foreground">(为空时用地址做名称)</span></Label>
            <Input value={name} onChange={e => setName(e.target.value)} placeholder="例如：公司服务器" className="h-7 text-xs" />
          </div>
          <div>
            <Label className="text-[10px]">地址 *</Label>
            <Input value={address} onChange={e => setAddress(e.target.value)} placeholder="116.62.206.205" className="h-7 text-xs" />
          </div>
          <div>
            <Label className="text-[10px]">端口号 *</Label>
            <Input value={port} onChange={e => setPort(e.target.value)} placeholder="52346" className="h-7 text-xs" />
          </div>
          <div>
            <Label className="text-[10px]">默认组名 <span className="text-muted-foreground">(可选)</span></Label>
            <Input value={defaultGroup} onChange={e => setDefaultGroup(e.target.value)} placeholder="my-group" className="h-7 text-xs" />
          </div>
          <div>
            <Label className="text-[10px]">默认设备名 <span className="text-muted-foreground">(可选)</span></Label>
            <Input value={defaultDevice} onChange={e => setDefaultDevice(e.target.value)} placeholder="my-device" className="h-7 text-xs" />
          </div>
          {error && <p className="text-[10px] text-destructive">{error}</p>}
          <div className="flex justify-end gap-2">
            <Button variant="outline" size="xs" onClick={onClose}>取消</Button>
            <Button size="xs" onClick={handleSave}>保存</Button>
          </div>
        </div>
      </div>
    </div>
  )
}
