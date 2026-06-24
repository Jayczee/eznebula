# <img src="eznebula-frontend/src-tauri/icons/128x128.png" width="32" height="32" alt=""> EZNebula

基于 [Nebula](https://github.com/slackhq/nebula) 的现代化极简虚拟局域网客户端，一键组建安全的 P2P 加密网络。

---

## 快速开始

### 客户端

从 [Releases](https://github.com/Jayczee/eznebula/releases) 下载 `EZNebula-setup.exe` 安装，或自行构建：

```bash
cd eznebula-frontend
bun install
bun run tauri dev      # 开发模式
bun run tauri build    # 生产构建
```

### 服务端部署

服务端负责签发证书、运行 Lighthouse 中转。支持 **Docker** 和 **手动部署** 两种方式。

#### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `EZNEBULA_LIGHTHOUSE_IP` | **必填** | 服务器的公网 IP 或域名，客户端通过此地址连接 Lighthouse |
| `EZNEBULA_LIGHTHOUSE_PORT` | `4242` | Lighthouse UDP 端口，需在防火墙放行 |
| `EZNEBULA_PORT` | `8080` | HTTP API 端口 |
| `EZNEBULA_DATA_DIR` | `/data/eznebula-data` | 数据持久化目录（证书、数据库） |
| `JAVA_OPTS` | `-Xms128m -Xmx256m` | JVM 参数 |

#### 方式一：Docker 部署（推荐）

```bash
git clone https://github.com/Jayczee/eznebula.git
cd eznebula

# 设置环境变量
export EZNEBULA_LIGHTHOUSE_IP=你的公网IP
export EZNEBULA_LIGHTHOUSE_PORT=4242
export EZNEBULA_PORT=8080

# 构建并启动
docker compose up -d --build
```

**防火墙要求**：放行 `4242/udp`（Lighthouse）和 `8080/tcp`（API）。

#### 方式二：手动部署

```bash
# 1. 安装依赖：Java 17+、Maven、Nebula 二进制文件
#    将 nebula 和 nebula-cert 放到 PATH 中

# 2. 构建后端
cd eznebula-backend
mvn clean package -DskipTests

# 3. 配置（修改 application.yml 或通过启动参数覆盖）
java -D eznebula.lighthouse.public-ip=你的公网IP \
     -D eznebula.lighthouse.port=4242 \
     -D server.port=8080 \
     -jar target/eznebula-backend-1.0.0.jar
```

#### 配置文件参考（`application.yml`）

```yaml
eznebula:
  lighthouse:
    public-ip: 你的公网IP     # ← 必须修改
    port: 4242                # Lighthouse UDP 端口
  ca:
    storage-path: ${user.home}/.eznebula/ca
    organization: eznebula
    duration: 87600h          # 证书有效期 10 年
server:
  port: 8080                  # HTTP API 端口
```

#### 验证部署

```bash
# 健康检查
curl http://你的服务器IP:8080/api/v1/health
# 预期返回: {"success":true,"message":"EZNebula running"}
```

---

## 项目结构

```
eznebula/
├── eznebula-backend/               # Spring Boot 服务端
│   └── src/main/java/com/eznebula/
│       ├── controller/             # REST API
│       ├── service/                # CA 签发、网络管理、Lighthouse、IP 分配
│       ├── domain/                 # JPA 实体和仓储
│       └── dto/                    # 请求/响应 DTO
├── eznebula-frontend/              # Tauri + React 桌面客户端
│   ├── src/                        # React 前端 (TypeScript + TailwindCSS 4 + shadcn/ui)
│   ├── src-tauri/src/              # Rust 后端
│   │   ├── nebula.rs               # Nebula 进程管理、配置生成、日志解析
│   │   ├── crypto.rs               # X25519 密钥生成
│   │   └── state.rs                # 全局状态
│   └── src-tauri/icons/            # 应用图标
├── binaries/                       # Nebula 二进制（各平台，构建时嵌入客户端）
├── Dockerfile                      # Docker 镜像
├── docker-compose.yml              # Docker 编排
└── deploy.sh                       # 一键部署脚本
```

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri v2 |
| 前端 UI | React 19 + TypeScript + TailwindCSS 4 + shadcn/ui |
| 客户端后端 | Rust（Nebula 进程管理、密钥生成） |
| 服务端 | Java 17 + Spring Boot 3.2 + SQLite |
| SD-WAN | [Nebula](https://github.com/slackhq/nebula) |

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

## License

MIT
