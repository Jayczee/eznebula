# EZNebula

基于 [Nebula](https://github.com/slackhq/nebula) 的现代化极简虚拟局域网客户端，一键组建安全的 P2P 加密网络。

---

## 特性

- **一键组网** — 输入服务器地址和组名即可加入虚拟局域网
- **P2P 直连** — 自动 NAT 穿透，节点间低延迟直连
- **强制中转** — 支持强制走 Lighthouse 中转，应对复杂网络环境
- **实时延迟** — 每个对端节点显示 ICMP 延迟
- **跨平台客户端** — Windows / Linux / macOS（基于 Tauri v2）
- **自动证书管理** — 服务端自动签发 Nebula 证书，客户端零配置

---

## 架构

```
┌─────────────────────────┐      ┌──────────────────────────┐
│   桌面客户端 (Tauri)      │      │   服务端 (Spring Boot)     │
│                          │      │                          │
│  React 19 + Tailwind 4  │◄────►│  REST API                │
│  Rust (Nebula 管理)      │      │  CA 证书签发              │
│  nebula.exe (SD-WAN)    │      │  Lighthouse 中转          │
│                          │      │  SQLite 持久化            │
└─────────────────────────┘      └──────────────────────────┘
```

---

## 快速开始

### 1. 启动服务端

```bash
cd eznebula-backend

# 编辑 lighthouse 公网 IP
# 修改 src/main/resources/application.yml 中 eznebula.lighthouse.public-ip

mvn spring-boot:run
```

### 2. 启动客户端

下载 [Releases](../../releases) 中的 `EZNebula-portable.exe` 直接运行，或从源码构建：

```bash
cd eznebula-frontend
bun install
bun run tauri dev      # 开发模式
bun run tauri build    # 生产构建
```

### 3. 连接网络

1. 打开客户端，输入服务器地址（如 `http://your-server:8080`）
2. 填写组名和设备名
3. 可选：勾选 **强制中转** 让流量全部走 Lighthouse 中转
4. 点击 **连接**

---

## 项目结构

```
eznebula/
├── eznebula-backend/               # Spring Boot 服务端
│   └── src/main/java/com/eznebula/
│       ├── controller/             # REST API
│       ├── service/                # 业务逻辑（CA、网络管理、IP分配）
│       ├── domain/                 # JPA 实体和仓储
│       └── dto/                    # 请求/响应 DTO
├── eznebula-frontend/              # Tauri + React 桌面客户端
│   ├── src/                        # React 前端
│   │   ├── components/             # UI 组件
│   │   └── lib/                    # API 封装、工具函数
│   └── src-tauri/src/              # Rust 后端
│       ├── nebula.rs               # Nebula 进程管理、配置生成、日志解析
│       ├── network.rs              # 服务器测速、RTT 测量
│       ├── crypto.rs               # X25519 密钥生成
│       ├── models.rs               # 数据模型
│       └── state.rs                # 全局状态
├── binaries/                       # Nebula 二进制文件（各平台）
├── Dockerfile                      # Docker 部署
└── docker-compose.yml
```

---

## API 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/join` | 客户端加入网络（签发证书、分配 IP） |
| POST | `/api/v1/heartbeat` | 心跳上报 |
| POST | `/api/v1/leave` | 离开网络 |
| GET | `/api/v1/groups/{name}/clients` | 查询组内在线客户端 |
| GET | `/api/v1/health` | 健康检查 |
| POST | `/api/v1/admin/groups` | 创建网络组 |
| GET | `/api/v1/admin/groups` | 查询所有组 |
| DELETE | `/api/v1/admin/groups/{name}` | 删除网络组 |

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri v2 |
| 前端 UI | React 19 + TypeScript + TailwindCSS 4 + shadcn/ui |
| 客户端后端 | Rust (Nebula 进程管理、密钥生成) |
| 服务端 | Java 17 + Spring Boot 3.2 + SQLite |
| SD-WAN | [Nebula](https://github.com/slackhq/nebula) (Slack) |
| 打包 | NSIS / WiX (Windows), AppImage / deb (Linux), DMG (macOS) |

---

## Docker 部署

```bash
docker-compose up -d
```

服务端将监听 `8080` 端口，Lighthouse 监听 `4242` UDP。

---

## License

MIT
