# EZNebula Project

基于 Nebula 的现代化极简跨平台虚拟局域网客户端

## 项目结构

```
eznebula/
├── eznebula-backend/          # Spring Boot 服务端
│   ├── src/
│   │   └── main/
│   │       ├── java/com/eznebula/
│   │       │   ├── config/           # 配置类
│   │       │   ├── controller/       # REST 控制器
│   │       │   ├── domain/           # 实体和仓储
│   │       │   ├── dto/              # 数据传输对象
│   │       │   ├── exception/        # 异常处理
│   │       │   └── service/          # 业务逻辑层
│   │       └── resources/
│   │           └── application.yml   # 配置文件
│   ├── pom.xml
│   └── README.md
└── eznebula-frontend/         # Tauri + React 客户端 (待开发)
```

## 阶段一完成情况 ✅

### 已完成的功能

1. **完整的分层架构**
   - Controller 层：REST API 接口
   - Service 层：业务逻辑
   - Repository 层：数据访问
   - 统一异常处理

2. **核心实体类**
   - `NetworkGroup`：网络组管理
   - `ClientNode`：客户端节点信息

3. **服务组件**
   - `NebulaCertService`：封装 nebula-cert 命令行工具
   - `NetworkService`：处理客户端加入网络的核心逻辑
   - `IpAllocationService`：虚拟 IP 地址分配和管理

4. **API 接口**
   - `POST /api/v1/join`：客户端加入网络 (核心接口)
   - `POST /api/v1/admin/groups`：创建网络组
   - `GET /api/v1/admin/groups`：查询网络组
   - `DELETE /api/v1/admin/groups/{groupName}`：删除网络组
   - `GET /api/v1/health`：健康检查

5. **安全特性**
   - 常量时间字符串比较防止时间攻击
   - 数据库悲观锁防止 IP 分配竞争
   - 参数验证和异常处理
   - CA 私钥仅存储在服务端

### 技术亮点

- **设计模式**：分层架构、工厂模式、策略模式
- **线程安全**：悲观锁保证 IP 分配不冲突
- **安全性**：防时序攻击、输入验证、错误处理
- **可维护性**：清晰的代码结构、完整的日志记录

## 快速开始

### 前置要求

1. Java 17+
2. Maven 3.6+
3. nebula-cert 工具 (下载自 https://github.com/slackhq/nebula/releases)

### 启动服务端

```bash
cd eznebula-backend

# 编译
mvn clean package

# 运行
java -jar target/eznebula-backend-1.0.0.jar
```

### 配置

编辑 `application.yml` 设置 Lighthouse 公网 IP：

```yaml
eznebula:
  lighthouse:
    public-ip: YOUR_PUBLIC_IP  # 替换为你的公网 IP
```

## 下一步：阶段二

准备开发 Tauri (Rust) 客户端：

1. 使用 Rust 生成 Nebula Ed25519 密钥对
2. 调用后端 API 获取签发的证书
3. 动态生成 `config.yaml`
4. 启动和管理 Nebula 进程
5. 监控网络流量和节点状态

## 技术栈

- **后端**：Java 17 + Spring Boot 3.2.5 + SQLite
- **前端**：Rust (Tauri) + React + TailwindCSS + shadcn/ui
- **核心**：Nebula (Slack 开源 SD-WAN)
