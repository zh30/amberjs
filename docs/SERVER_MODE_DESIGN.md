# Amber Server 模式设计方案

> 发布校验说明（2026-05-26）：本文件是历史设计稿。当前 public CLI 为 `amber serve [--host HOST] [--port PORT]`，不是 `amberjs server`。

## 概述

Amber Server 模式将运行时从单次执行模式扩展为长期运行的服务器，支持多个客户端并发执行 JavaScript 代码，彻底避免重复初始化开销。

## 核心优势

### 1. 极致性能
- **零重复初始化**: 单次 V8 初始化，多次复用
- **并发执行**: 支持多客户端同时执行
- **智能缓存**: 编译结果和运行时状态复用

### 2. 架构简洁
- **HTTP API**: 标准的 RESTful 接口
- **WebSocket**: 实时代码执行和流式输出
- **进程内复用**: 避免跨进程通信开销

### 3. AI 工作负载优化
- **批量处理**: 支持 AI 模型批量推理
- **内存复用**: 预分配内存池，复用中间结果
- **异步队列**: 高性能任务调度

## 技术架构

### 组件设计

```
┌─────────────────────────────────────────────────────────────┐
│                    Amber Server                             │
├─────────────────────────────────────────────────────────────┤
│  HTTP Server  │  WebSocket Server  │  CLI Interface         │
│  (/eval)      │  (实时执行)          │  (交互模式)            │
├─────────────────────────────────────────────────────────────┤
│              运行时管理器 (Runtime Manager)                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐│
│  │ Runtime Lite│ │ Full Runtime│ │    Shared State         ││
│  │ (轻量级)     │ │ (完整优化)   │ │  (全局缓存/统计)         ││
│  └─────────────┘ └─────────────┘ └─────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│              V8 引擎层                                       │
│  ┌─────────────────────────────────────────────────────────┐│
│  │        单次初始化 · 多次复用                              ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### 核心模块

#### 1. Server 核心 (`src/server/`)
- `mod.rs` - 服务器入口
- `http_server.rs` - HTTP API 实现
- `websocket.rs` - WebSocket 支持
- `router.rs` - 路由管理

#### 2. 运行时管理 (`src/runtime_manager/`)
- `mod.rs` - 运行时管理器入口
- `pool.rs` - Runtime 实例池
- `scheduler.rs` - 任务调度
- `stats.rs` - 性能统计

#### 3. 客户端管理 (`src/client_manager/`)
- `mod.rs` - 客户端管理入口
- `session.rs` - 会话管理
- `auth.rs` - 认证授权
- `rate_limit.rs` - 限流控制

## API 设计

### HTTP API

#### 1. 执行代码
```http
POST /api/v1/eval
Content-Type: application/json

{
  "code": "1+1",
  "timeout": 5000,
  "optimize": "speed"
}
```

响应:
```json
{
  "result": "2",
  "execution_time_ms": 5,
  "cached": true
}
```

#### 2. 执行文件
```http
POST /api/v1/exec
Content-Type: application/json

{
  "file_path": "/path/to/script.js",
  "timeout": 10000
}
```

#### 3. 性能统计
```http
GET /api/v1/stats
```

响应:
```json
{
  "total_executions": 10000,
  "avg_execution_time_ms": 5.2,
  "cache_hit_rate": 0.85,
  "active_sessions": 10
}
```

### WebSocket API

#### 连接
```javascript
const ws = new WebSocket('ws://localhost:3000/ws');
ws.send(JSON.stringify({
  type: 'eval',
  code: 'console.log("Hello"); 1+1'
}));
```

#### 消息格式
```json
{
  "type": "eval",
  "id": "req-123",
  "code": "1+1"
}
```

```json
{
  "type": "result",
  "id": "req-123",
  "result": "2",
  "execution_time_ms": 5,
  "cached": false
}
```

## 性能优化策略

### 1. Runtime 池化
- **轻量级 Runtime**: 用于简单表达式
- **完整 Runtime**: 用于复杂脚本
- **智能选择**: 基于代码复杂度自动选择

### 2. 缓存策略
- **编译缓存**: V8 字节码缓存
- **结果缓存**: 纯函数结果缓存
- **上下文缓存**: V8 Context 复用

### 3. 并发控制
- **连接池**: 限制最大并发连接数
- **任务队列**: 异步任务调度
- **资源隔离**: 每个客户端独立上下文

## 部署方案

### 1. 独立服务器
```bash
# 启动服务器
amber serve --port 3000 --host 0.0.0.0

# 后台运行
amber serve --host 0.0.0.0 --port 3000
```

### 2. 集成模式
```rust
use amberjs::Server;

let server = Server::new()
    .port(3000)
    .max_connections(1000)
    .enable_websocket(true);

server.run()?;
```

### 3. Docker 部署
```dockerfile
FROM amberjs:latest
EXPOSE 3000
CMD ["amberjs", "server", "--host", "0.0.0.0"]
```

## 安全考虑

### 1. 代码执行安全
- **沙箱隔离**: 限制文件系统访问
- **超时控制**: 防止无限循环
- **资源限制**: CPU/内存使用限制

### 2. 访问控制
- **API Key**: 基于密钥的认证
- **速率限制**: 防止滥用
- **IP 白名单**: 限制访问源

### 3. 监控和日志
- **执行日志**: 记录所有代码执行
- **性能监控**: 实时性能指标
- **错误追踪**: 详细的错误信息

## 测试策略

### 1. 单元测试
- 各个模块独立测试
- Mock V8 和网络接口

### 2. 集成测试
- 完整服务器启动测试
- API 接口测试
- 并发执行测试

### 3. 性能测试
- 负载测试 (1000+ 并发)
- 压力测试 (长时间运行)
- 基准测试 (vs 单次执行)

## 实施计划

### 阶段 1: 基础服务器 (1周)
- [ ] 创建 server 模块结构
- [ ] 实现基础 HTTP 服务器
- [ ] 添加 /eval API 端点
- [ ] 基本错误处理

### 阶段 2: 运行时池化 (1周)
- [ ] 实现 RuntimeManager
- [ ] 添加 Runtime 实例池
- [ ] 智能运行时选择
- [ ] 性能统计

### 阶段 3: 高级功能 (1周)
- [ ] WebSocket 支持
- [ ] 批量执行 API
- [ ] 会话管理
- [ ] 缓存优化

### 阶段 4: 优化和测试 (1周)
- [ ] 并发性能优化
- [ ] 完整测试套件
- [ ] 压力测试
- [ ] 文档和示例

---

**创建时间**: 2025-12-18
**负责人**: Amber Server 团队
**状态**: 设计完成，准备实施
