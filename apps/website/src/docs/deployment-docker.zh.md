---
title: "全自动部署与容器编排 (bee deploy)"
subtitle: "一键生成生产级多阶段 Dockerfile、独立 SEA 二进制编译及 Kubernetes 生产编排清单"
group: "生态与扩展"
id: "deployment-docker"
---

## 1. 为什么推出 `bee deploy`？

从本地代码到生产环境的部署往往充满摩擦：
- **Dockerfile 编写繁琐**：很多开发者编写的 Dockerfile 过于庞大（几个 GB）、缺少非 root 权限保护、或者未利用构建缓存；
- **环境碎片化**：不同部署目标（Docker 容器、独立免依赖二进制、Kubernetes 集群）需要维护多套不一致的配置脚本；
- **配置与安全合规缺漏**：在 K8s 中常常遗忘就绪探针（readinessProbe）、存活探针（livenessProbe）与 CPU/内存物理配额。

Beejs 内置了 **`bee deploy`** 工具链：自动检测项目类型、自动生成开箱即用的多阶段优化部署配置。

---

## 2. 基础使用与命令选项

```bash
# 默认生成 Dockerfile 与 docker-compose.yml
bee deploy

# 指定端口与项目入口文件
bee deploy --port 3000 --entry server.ts

# 指定生成目标：docker、compile 或 k8s
bee deploy --target docker
bee deploy --target compile
bee deploy --target k8s
```

### 命令选项参考

| 选项 | 描述 | 默认值 |
| :--- | :--- | :--- |
| `-t, --target <target>` | 部署目标：`docker`、`compile`、`k8s` | `docker` |
| `-e, --entry <file>` | 项目入口文件（自动检测 `server.ts`/`app.ts`/`index.ts`） | 自动推断 |
| `-p, --port <port>` | 生产服务监听端口 | `3000` |
| `-o, --output-dir <dir>` | 配置文件产出目标目录 | 当前目录 |

---

## 3. 生成模式详解

### 一、生产级多阶段 Docker 容器 (`--target docker`)

执行 `bee deploy --target docker` 会自动在根目录创建：
1. **`Dockerfile`**：采用轻量级多阶段构建（Multi-Stage），基于非 root 用户运行，镜像体积小至几十兆；
2. **`.dockerignore`**：自动过滤 `node_modules`、`.git`、测试文件与临时产物；
3. **`docker-compose.yml`**：预先配置健康检查、端口映射与自动重启策略。

生成的 `Dockerfile` 范例：

```dockerfile
# Build stage
FROM ghcr.io/zh30/beejs:latest AS builder
WORKDIR /app
COPY . .
RUN bee bundle --minify --outfile dist/server.js server.ts

# Production runner stage
FROM debian:bookworm-slim
RUN useradd -m -u 10001 beejs
WORKDIR /app
COPY --from=builder /usr/local/bin/bee /usr/local/bin/bee
COPY --from=builder /app/dist/server.js ./dist/server.js
USER beejs
EXPOSE 3000
CMD ["bee", "run", "dist/server.js"]
```

---

### 二、独立 SEA 二进制应用 (`--target compile`)

对于嵌入式设备、边缘节点或内网分发，你可以选择直接编译为单文件可执行二进制（Single Executable Application）：

```bash
bee deploy --target compile --entry app.ts
```

该命令调用内置编译流水线，将你的 TypeScript 代码、转译产物与 Beejs V8 运行时打包进单一的可执行文件。目标机器无需安装 Node.js、Rust 或 Beejs，直接双击或通过命令行即可秒级启动！

---

### 三、Kubernetes 原生编排清单 (`--target k8s`)

执行 `bee deploy --target k8s` 会在 `k8s/` 目录下生成完整的云原生生产清单：
- **`deployment.yaml`**：配置了 3 副本高可用、RollingUpdate 滚动更新策略、非 root 安全上下文（securityContext）、以及精确的 CPU/内存资源配额限制；
- **`service.yaml`**：开箱即用的 ClusterIP 服务配置；
- **`hpa.yaml`**：自动水平弹性扩缩容（基于 80% CPU 阈值，支持 2-10 副本）。

一键将服务上线至生产集群：

```bash
kubectl apply -f k8s/
```
