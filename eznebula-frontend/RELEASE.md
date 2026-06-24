# EZNebula v1.0.0 发布文件

## 📦 打包完成

构建时间: 2026-06-16

### 文件清单

| 文件类型 | 路径 | 大小 | 说明 |
|---------|------|------|------|
| **单文件便携版** | `target/release/EZNebula-Portable.exe` | 26 MB | ✅ **推荐下载** - 真正的单文件，无需安装 |
| NSIS安装器 | `target/release/bundle/nsis/EZNebula_1.0.0_x64-setup.exe` | 20 MB | Windows 安装器（适合普通用户） |
| MSI安装包 | `target/release/bundle/msi/EZNebula_1.0.0_x64_zh-CN.msi` | 24 MB | 企业部署版本（适合 GPO 分发） |

## 🚀 使用方法

### 单文件便携版（推荐）

1. 下载 `EZNebula-Portable.exe`
2. 双击运行（无需安装）
3. 首次运行会自动提取必要文件到 `%APPDATA%\cn.jayczee.eznebula\bin\`
4. 后续运行无需再次提取

**特点**：
- ✅ 无需安装，下载即用
- ✅ 可放在U盘随身携带
- ✅ nebula.exe 和 wintun.dll 已内嵌
- ✅ 自动管理依赖文件

### NSIS 安装器

1. 下载 `EZNebula_1.0.0_x64-setup.exe`
2. 双击安装
3. 安装后可在开始菜单找到快捷方式

### MSI 安装包

适合企业环境，可通过组策略(GPO)批量部署：
```powershell
msiexec /i EZNebula_1.0.0_x64_zh-CN.msi /qn
```

## 📋 系统要求

- Windows 10/11 (x64)
- WebView2 运行时（Windows 11 已内置，Windows 10 会自动下载）
- 管理员权限（用于创建虚拟网卡）

## 🔧 内嵌资源

以下文件已编译时嵌入到 exe 中：
- nebula.exe (21 MB)
- wintun.dll (418 KB)

## 📝 版本信息

- 版本号: 1.0.0
- 标识符: cn.jayczee.eznebula
- 构建类型: Release (优化编译)

## 🎯 推荐分发

- **个人用户**: `EZNebula-Portable.exe`
- **企业部署**: `EZNebula_1.0.0_x64_zh-CN.msi`
- **技术用户**: 三种格式任选
