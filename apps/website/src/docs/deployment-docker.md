---
title: "Automated Deployment & Orchestration (bee deploy)"
subtitle: "One-command generation of production multi-stage Dockerfiles, standalone SEA binaries, and Kubernetes manifests"
group: "Ecosystem"
id: "deployment-docker"
---

## 1. Why `bee deploy`?

Shipping code from local machines to production often introduces friction:
- **Cumbersome Dockerfiles**: Misconfigured image layers, bloated multi-gigabyte images, or missing non-root security principles.
- **Environment Fragmentation**: Different delivery targets (Docker containers, standalone binaries, Kubernetes clusters) require disparate configuration templates.
- **Missing Reliability Best Practices**: Forgetting readiness probes, liveness probes, or CPU/memory limits in Kubernetes manifests.

Beejs provides built-in **`bee deploy`**: automatically inspects your project layout and generates hardened, production-ready configurations.

---

## 2. Basic Usage & Options

```bash
# Default: generate Dockerfile and docker-compose.yml
bee deploy

# Specify listening port and entry file
bee deploy --port 3000 --entry server.ts

# Target specific deployment model: docker, compile, or k8s
bee deploy --target docker
bee deploy --target compile
bee deploy --target k8s
```

### Command Flags

| Flag | Description | Default |
| :--- | :--- | :--- |
| `-t, --target <target>` | Target type: `docker`, `compile`, or `k8s` | `docker` |
| `-e, --entry <file>` | Application entry file (`server.ts`, `app.ts`, `index.ts`) | Auto-inferred |
| `-p, --port <port>` | Service port for container/k8s exposure | `3000` |
| `-o, --output-dir <dir>` | Directory to emit configuration files | Current directory |

---

## 3. Deployment Modes

### 1. Hardened Multi-Stage Docker (`--target docker`)

Running `bee deploy --target docker` creates:
1. **`Dockerfile`**: Multi-stage build running under an unprivileged non-root user (`beejs:10001`), keeping image footprint minimal.
2. **`.dockerignore`**: Excludes `node_modules`, `.git`, temporary test files, and local logs.
3. **`docker-compose.yml`**: Pre-configured health check probes, port bindings, and restart policies.

Generated `Dockerfile` example:

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

### 2. Standalone SEA Application (`--target compile`)

For air-gapped systems, IoT devices, or single-binary CLI distribution, compile directly into a Single Executable Application:

```bash
bee deploy --target compile --entry app.ts
```

This compiles your TypeScript source, bundled assets, and the Beejs V8 runtime into a single, dependency-free binary. The target environment requires zero installations of Node.js, Rust, or Beejs.

---

### 3. Cloud-Native Kubernetes Manifests (`--target k8s`)

Running `bee deploy --target k8s` emits production-ready specs into `k8s/`:
- **`deployment.yaml`**: 3 replicas, RollingUpdate strategy, non-root security context, and strict resource quotas.
- **`service.yaml`**: Standard ClusterIP service definition.
- **`hpa.yaml`**: Horizontal Pod Autoscaler targeting 80% CPU utilization (2 to 10 pods).

Deploy instantly:

```bash
kubectl apply -f k8s/
```
