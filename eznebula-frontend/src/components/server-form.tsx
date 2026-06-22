import { useState, useEffect } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

interface Props {
  open: boolean
  onClose: () => void
  onSave: (name: string, address: string, port: number, defaultGroup: string, defaultDevice: string) => void
}

/** 从地址字符串中解析主机名和端口。支持 host:port、https://host:port、纯域名/IP */
function parseAddress(input: string): { host: string; port: number } {
  let s = input.trim()
  // 去掉协议前缀
  if (s.includes("://")) {
    s = s.split("://")[1]
  }
  // 去掉尾部路径
  if (s.includes("/")) {
    s = s.split("/")[0]
  }
  // 分离端口：查找最后一个冒号（排除 IPv6）
  const lastColon = s.lastIndexOf(":")
  if (lastColon > 0 && !s.substring(0, lastColon).includes("[")) {
    const port = parseInt(s.substring(lastColon + 1), 10)
    if (!isNaN(port) && port >= 1 && port <= 65535) {
      return { host: s.substring(0, lastColon), port }
    }
  }
  // 默认端口 443 (HTTPS)
  return { host: s, port: 443 }
}

export function ServerForm({ open, onClose, onSave }: Props) {
  const [name, setName] = useState("")
  const [address, setAddress] = useState("")
  const [defaultGroup, setDefaultGroup] = useState("")
  const [defaultDevice, setDefaultDevice] = useState("")
  const [error, setError] = useState("")

  useEffect(() => {
    if (open) {
      setName("")
      setAddress("")
      setDefaultGroup("")
      setDefaultDevice("")
      setError("")
    }
  }, [open])

  if (!open) return null

  const handleSave = () => {
    if (!address.trim()) { setError("地址不能为空"); return }
    const { host, port } = parseAddress(address)
    onSave(name.trim() || host, host, port, defaultGroup.trim(), defaultDevice.trim())
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
            <Label className="text-[10px]">地址 * <span className="text-muted-foreground">(可含端口，如 host:52346)</span></Label>
            <Input value={address} onChange={e => setAddress(e.target.value)} placeholder="nebula.jayczee.cn 或 1.2.3.4:52346" className="h-7 text-xs" />
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
