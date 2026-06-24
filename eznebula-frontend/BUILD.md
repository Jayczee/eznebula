# EZNebula 打包指南

## 开发模式运行
```bash
bun run tauri dev
```

## 打包成单文件 Portable exe

### 方案说明
EZNebula 使用 **嵌入式资源** 方案：
- ✅ nebula.exe 和 wintun.dll 在编译时嵌入到最终的 exe 中
- ✅ 首次运行时自动提取到 `%APPDATA%/cn.jayczee.eznebula/bin/` 目录
- ✅ 后续运行直接使用已提取的文件（无需重复解压）
- ✅ 最终产物是真正的单文件 exe（约 15-20 MB）

### 1. 准备二进制文件

确保以下文件存在于正确位置：
```
src-tauri/binaries/nebula.exe     # Nebula 可执行文件
src-tauri/binaries/wintun.dll     # WinTun 驱动 DLL
```

这些文件会在编译时通过 `include_bytes!` 宏嵌入到 Rust 代码中。

### 2. 构建发布版本

```bash
cd eznebula-frontend
bun run tauri build
```

构建过程会：
1. 编译 Rust 代码并嵌入二进制资源
2. 打包前端资源
3. 生成安装器和可执行文件

### 3. 输出文件

构建完成后，输出文件位于：

```
src-tauri/target/release/EZNebula.exe                                # ✅ 单文件 Portable exe
src-tauri/target/release/bundle/nsis/EZNebula_1.0.0_x64-setup.exe    # NSIS 安装器
src-tauri/target/release/bundle/msi/EZNebula_1.0.0_x64_zh-CN.msi     # MSI 安装包
```

**推荐使用**: `EZNebula.exe` - 这是真正的单文件便携版本

### 4. 运行时行为

首次运行时：
```
EZNebula.exe 启动
 ↓
检查 %APPDATA%/cn.jayczee.eznebula/bin/nebula.exe
 ↓
不存在 → 从内嵌资源提取到该目录
 ↓
启动 nebula 进程
```

后续运行：
```
EZNebula.exe 启动
 ↓
检查 %APPDATA%/cn.jayczee.eznebula/bin/nebula.exe
 ↓
已存在 → 直接使用
 ↓
启动 nebula 进程
```

### 5. 资源清理

如需清理提取的文件：
```powershell
Remove-Item -Recurse -Force "$env:APPDATA\cn.jayczee.eznebula\bin"
```

## 优势

✅ **真正的单文件** - 无需安装，无外部依赖  
✅ **便携性强** - 可放在 U 盘随身携带  
✅ **首次运行快** - 提取到 AppData 后永久保留  
✅ **自动更新** - 检测文件大小变化自动重新提取  
✅ **开发友好** - 开发模式仍从 binaries 目录读取  

## 生产环境部署

1. **企业内网**: 使用 MSI 安装包通过 GPO 部署
2. **个人用户**: 提供 Portable exe 下载
3. **自动更新**: 配合 Tauri 的 updater 功能
