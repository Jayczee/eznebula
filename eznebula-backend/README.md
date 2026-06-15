# EZNebula Backend

EZNebula 控制服务器和 CA 中心

## 技术栈

- Java 17
- Spring Boot 3.2.5
- SQLite (嵌入式数据库)
- Nebula Certificate Tools

## 快速开始

### 前置要求

1. Java 17 或更高版本
2. Maven 3.6+
3. `nebula-cert` 二进制文件在系统 PATH 中

### 安装 nebula-cert

从 [Nebula releases](https://github.com/slackhq/nebula/releases) 下载对应平台的二进制文件，并确保 `nebula-cert` 可执行文件在系统 PATH 中。

### 配置

编辑 `src/main/resources/application.yml`:

```yaml
eznebula:
  lighthouse:
    public-ip: YOUR_PUBLIC_IP  # 设置为服务器的公网 IP
    port: 4242
```

### 编译运行

```bash
# 编译
mvn clean package

# 运行
java -jar target/eznebula-backend-1.0.0.jar
```

## API 文档

### 1. 健康检查

```
GET /api/v1/health
```

### 2. 创建网络组 (管理接口)

```
POST /api/v1/admin/groups?groupName=dev-team&cidrBlock=10.168.0.0/16
```

响应:
```json
{
  "success": true,
  "data": {
    "groupName": "dev-team",
    "joinToken": "abc123...",
    "cidrBlock": "10.168.0.0/16"
  }
}
```

### 3. 加入网络组 (客户端接口)

```
POST /api/v1/join
Content-Type: application/json

{
  "groupName": "dev-team",
  "joinToken": "abc123...",
  "clientPublicKey": "-----BEGIN NEBULA ED25519 PUBLIC KEY-----...",
  "clientName": "laptop-001"
}
```

响应:
```json
{
  "success": true,
  "data": {
    "virtualIpWithCidr": "10.168.0.1/24",
    "clientCertificate": "-----BEGIN NEBULA CERTIFICATE-----...",
    "caCertificate": "-----BEGIN NEBULA CERTIFICATE-----...",
    "lighthouseIp": "1.2.3.4",
    "lighthousePort": 4242,
    "networkCidr": "10.168.0.0/16"
  }
}
```

## 架构说明

### 分层架构

```
Controller Layer (REST API)
    ↓
Service Layer (业务逻辑)
    ↓
Repository Layer (数据访问)
    ↓
Database (SQLite)
```

### 核心组件

- **NebulaCertService**: 封装 nebula-cert 命令行工具，负责 CA 和证书管理
- **NetworkService**: 处理客户端加入网络的核心业务逻辑
- **IpAllocationService**: 虚拟 IP 地址分配和管理
- **GlobalExceptionHandler**: 统一异常处理

### 安全特性

- 常量时间字符串比较防止时间攻击
- 数据库悲观锁防止 IP 分配竞争
- 输入验证和参数校验
- 私钥仅存储在服务端，永不传输

## 数据库

数据库文件默认位置: `~/.eznebula/eznebula.db`

### 表结构

**network_groups**: 网络组信息
- id, group_name, join_token, cidr_block, next_ip_address, description, active, created_at, updated_at

**client_nodes**: 客户端节点信息
- id, group_id, client_name, virtual_ip, cidr_suffix, public_key, certificate, certificate_expires_at, last_seen_at, active, created_at, updated_at
