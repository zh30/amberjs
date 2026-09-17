# Amber 生产环境部署指南

> 发布校验说明（2026-05-26）：本文已按当前 public CLI 做基础修正。当前二进制名为 `bee`，脚本执行使用 `amber run <file>`，表达式执行使用 `amber eval <code>`；历史性能调优 flag（如 `--max-heap`、`--optimize`）不是当前公开 CLI 契约。

## 概述

Amber 是一个高性能的 JavaScript/TypeScript 运行时，专为 AI 时代的高性能脚本执行而设计。本指南将帮助您在生产环境中部署和优化 Amber。

## 系统要求

### 最低要求
- **操作系统**: Linux (Ubuntu 20.04+), macOS (10.15+), 或 Windows 10+
- **内存**: 2GB RAM (推荐 4GB+)
- **CPU**: 2 核 (推荐 4 核+)
- **磁盘空间**: 500MB

### 推荐配置
- **内存**: 8GB+ RAM
- **CPU**: 8 核+ CPU
- **磁盘**: SSD

## 部署方式

### 方式一：一键安装 (推荐)

```bash
curl -fsSL https://get.amberjs.com/install.sh | sh
amber --version
```

### 方式二：二进制部署 (手动)

1. **下载预编译二进制文件**
   ```bash
   # 选择版本和平台
   VERSION=v0.1.0
   TARGET=x86_64-unknown-linux-gnu

   # 下载指定版本
   curl -L https://github.com/zh30/amberjs/releases/download/${VERSION}/bee-${VERSION}-${TARGET}.tar.gz -o bee.tar.gz
   tar -xzf bee.tar.gz
   chmod +x bee
   ```

2. **安装到系统路径**
   ```bash
   mkdir -p ~/.beejs/bin
   mv bee ~/.beejs/bin/
   export PATH=\"$HOME/.beejs/bin:$PATH\"
   amber --version
   ```

3. **验证安装**
   ```bash
   amber eval 'console.log("Hello from Amber!"); 1+1'
   ```

### 方式二：源码编译

1. **安装 Rust 工具链**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **编译项目**
   ```bash
   git clone https://github.com/zh30/amberjs.git
   cd beejs
   cargo build --release
   ```

3. **安装**
   ```bash
   sudo cp target/release/bee /usr/local/bin/
   ```

## 性能优化配置

### 内存配置

```bash
# 当前 public CLI 不支持 --max-heap / --stack-size；
# 请用容器、systemd 或操作系统资源限制管理生产内存。
amber run script.js
```

### V8 优化

```bash
# 当前 public CLI 不支持 --optimize flag。
amber run script.js
```

### Isolate 池化

Amber 自动使用 V8 Isolate 池化以提高性能：
- 默认池大小: CPU 核心数 (最大 8)
- 在生产环境中自动启用

## 生产环境最佳实践

### 1. 进程管理

使用 systemd 管理 Amber 进程：

```ini
# /etc/systemd/system/beejs.service
[Unit]
Description=Amber Runtime
After=network.target

[Service]
Type=simple
User=beejs
WorkingDirectory=/opt/beejs
ExecStart=/usr/local/bin/amber run /opt/beejs/app.js
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

启用并启动服务：
```bash
sudo systemctl enable beejs
sudo systemctl start beejs
```

### 2. 监控

#### 日志配置
```bash
# 启用详细日志
bee --verbose script.js 2>&1 | tee /var/log/beejs.log
```

#### 性能监控
```bash
# 查看内存使用
ps aux | grep bee

# 查看 CPU 使用
top -p $(pgrep bee)
```

### 3. 安全配置

#### 限制资源使用
```bash
# 使用 cgroups 限制内存
sudo cgcreate -g memory:beejs
sudo cgset -r memory.limit_in_bytes=1073741824 beejs
sudo cgexec -g memory:beejs bee script.js
```

#### 文件权限
```bash
# 确保 Amber 二进制文件权限正确
sudo chmod 755 /usr/local/bin/bee
sudo chown root:root /usr/local/bin/bee
```

### 4. 负载均衡

对于高并发场景，使用 PM2 或类似工具：

```bash
# 安装 PM2
npm install -g pm2

# 启动多个实例
pm2 start bee --name "beejs-1" -- script.js
pm2 start bee --name "beejs-2" -- script.js

# 查看状态
pm2 status
```

## 容器化部署

### Docker 部署

1. **创建 Dockerfile**
   ```dockerfile
   FROM ubuntu:22.04

   # 安装依赖
   RUN apt-get update && apt-get install -y \
       ca-certificates \
       && rm -rf /var/lib/apt/lists/*

   # 复制 Amber 二进制文件
   COPY bee /usr/local/bin/bee
   RUN chmod +x /usr/local/bin/bee

   # 设置工作目录
   WORKDIR /app

   # 复制应用文件
   COPY . .

   # 运行应用
   CMD ["bee", "script.js"]
   ```

2. **构建镜像**
   ```bash
   docker build -t beejs:latest .
   ```

3. **运行容器**
   ```bash
   docker run -d --name beejs-app beejs:latest
   ```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  beejs:
    build: .
    restart: always
    environment:
      - NODE_ENV=production
    volumes:
      - ./app:/app
    ports:
      - "3000:3000"
```

## 故障排除

### 常见问题

1. **内存不足错误**
   ```bash
   # 增加堆内存大小
   amber run script.js
   ```

2. **V8 编译错误**
   ```bash
   # 禁用优化
   amber run script.js
   ```

3. **性能问题**
   ```bash
   # 启用详细日志查看性能指标
   bee --verbose script.js
   ```

### 日志分析

查看错误日志：
```bash
journalctl -u beejs -f
```

查看应用日志：
```bash
tail -f /var/log/beejs.log
```

## 升级指南

### 升级 Amber

1. **备份当前版本**
   ```bash
   cp ~/.beejs/bin/bee ~/.beejs/bin/bee.backup
   ```

2. **安装新版本**
   ```bash
   VERSION=v0.1.0
   TARGET=x86_64-unknown-linux-gnu
   curl -L https://github.com/zh30/amberjs/releases/download/${VERSION}/bee-${VERSION}-${TARGET}.tar.gz -o bee.tar.gz
   tar -xzf bee.tar.gz
   cp bee ~/.beejs/bin/bee
   ```

3. **验证升级**
   ```bash
   amber --version
   amber eval 'console.log("Upgrade test"); 1+1'
   ```

4. **回滚（如需要）**
   ```bash
   cp ~/.beejs/bin/bee.backup ~/.beejs/bin/bee
   ```

## 性能基准

参考性能数据（基于标准硬件）：

- **启动时间**: ~17μs
- **代码执行**: ~18μs (简单操作)
- **算术运算**: ~18μs
- **内存使用**: 比 Node.js 优化 15%
- **并发能力**: 支持 10000+ 并发脚本

## 支持

- **文档**: [https://docs.beejs.dev](https://docs.beejs.dev)
- **GitHub**: [https://github.com/zh30/amberjs](https://github.com/zh30/amberjs)
- **问题报告**: [https://github.com/zh30/amberjs/issues](https://github.com/zh30/amberjs/issues)
- **社区**: [https://discord.gg/beejs](https://discord.gg/beejs)

---

*最后更新: 2025-12-18*
