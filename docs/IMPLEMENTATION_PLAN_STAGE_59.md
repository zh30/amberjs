# Stage 59 实施计划 - 可视化调试与 Chrome DevTools 集成

## 📋 阶段概述

Stage 59 专注于为 Amber 实现完整的可视化调试能力，包括 CLI 调试命令集成、Chrome DevTools 协议支持、Web UI 调试界面和 VS Code 扩展支持。这将把 Amber 打造成专业级的 JavaScript/TypeScript 调试平台。

**目标**: 构建完整的调试生态系统，支持命令行调试、可视化调试和远程调试，为开发者提供世界级的调试体验。

---

## 🎯 成功标准

### CLI 调试集成
- [ ] **Debug 命令**: `amberjs debug <file>` 完整实现
- [ ] **调试操作**: break, continue, next, step, finish, print, backtrace
- [ ] **REPL 调试**: `.debug` 命令集成
- [ ] **远程调试**: `--inspect-brk` 和 `--inspect` 支持

### Chrome DevTools 协议
- [ ] **协议实现**: 完整的 Chrome DevTools Protocol v1.3
- [ ] **WebSocket 服务器**: 调试端口监听和连接管理
- [ ] **事件转发**: Breakpoint、Exception、PromiseRejection 事件
- [ ] **命令处理**: Runtime.enable、Debugger.enable、Paused、Resumed 等

### 可视化界面
- [ ] **Web UI 调试器**: 基于浏览器的调试界面
- [ ] **实时调试**: 断点、单步执行、变量查看
- [ ] **性能分析**: CPU 性能剖析和内存分析界面
- [ ] **源码查看**: 语法高亮和行号显示

### IDE 集成
- [ ] **VS Code 扩展**: 完整的 VS Code 调试扩展
- [ ] **launch.json 支持**: 标准调试配置
- [ ] **智能感知**: 变量、调用栈、断点视图
- [ ] **多会话支持**: 同时调试多个脚本

---

## 📝 任务分解

### 阶段 1: CLI 调试命令完善
**优先级**: 🔴 高
**预计时间**: 3-4 小时

#### 1.1 Debug 子命令实现
- [ ] **添加 DebugCommand 枚举**
  - `Script { file, break_at, port }` - 调试脚本
  - `Attach { pid, port }` - 附加到进程
  - `Inspect { port }` - 启动检查器

- [ ] **实现调试模式**
  - 创建调试专用 Runtime
  - 集成调试器引擎
  - 设置初始断点（可选）

#### 1.2 调试操作实现
- [ ] **断点管理**
  - `break <line>` - 设置断点
  - `break <file>:<line>` - 文件断点
  - `break <function>` - 函数断点
  - `condition <id> <expr>` - 条件断点

- [ ] **执行控制**
  - `continue` - 继续执行到下一断点
  - `next` - 下一步（不进入函数）
  - `step` - 单步执行（进入函数）
  - `finish` - 完成当前函数
  - `pause` - 暂停执行

- [ ] **状态检查**
  - `print <expr>` - 打印变量值
  - `inspect <expr>` - 详细检查对象
  - `backtrace` - 显示调用栈
  - `list` - 显示当前代码行

#### 1.3 REPL 调试集成
- [ ] **调试模式切换**
  - `.debug` - 进入调试模式
  - `.continue` - 继续执行
  - `.step` - 单步执行
  - `.exit-debug` - 退出调试模式

### 阶段 2: Chrome DevTools 协议
**优先级**: 🔴 高
**预计时间**: 5-6 小时

#### 2.1 协议基础
- [ ] **WebSocket 服务器**
  - 实现 `amberjs --inspect` 端口监听
  - WebSocket 连接管理
  - 多客户端支持
  - 连接生命周期管理

#### 2.2 Runtime 域实现
- [ ] **Runtime.enable**
  - 启用运行时域
  - 发送 `Runtime.executionContextCreated`
  - 上下文 ID 管理

- [ ] **Runtime.disable**
  - 禁用运行时域
  - 清理上下文

- [ ] **Runtime.evaluate**
  - 表达式求值
  - 安全执行环境
  - 结果序列化

- [ ] **Runtime.callFunctionOn**
  - 对象方法调用
  - 上下文保持

#### 2.3 Debugger 域实现
- [ ] **Debugger.enable**
  - 启用调试器域
  - 脚本 ID 分配
  - 脚本缓存管理

- [ ] **Debugger.disable**
  - 禁用调试器域

- [ ] **Debugger.setBreakpointsActive**
  - 断点启用/禁用
  - 全局断点状态

- [ ] **Debugger.setBreakpointByUrl**
  - 按 URL 设置断点
  - 行号断点
  - 条件断点支持

- [ ] **Debugger.setBreakpoint**
  - 精确断点设置
  - 脚本位置断点

- [ ] **Debugger.removeBreakpoint**
  - 删除断点
  - 清理断点状态

- [ ] **Debugger.continueToLocation**
  - 执行到指定位置

#### 2.4 调试事件
- [ ] **Debugger.paused**
  - 断点命中事件
  - 异常停止事件
  - Promise 拒绝事件
  - 手动暂停事件

- [ ] **Debugger.resumed**
  - 执行恢复事件

- [ ] **Debugger.scriptParsed**
  - 脚本解析完成
  - 源码映射信息
  - 脚本 URL 和 ID

### 阶段 3: Web UI 调试器
**优先级**: 🟡 中
**预计时间**: 6-8 小时

#### 3.1 Web 服务器
- [ ] **内置 Web 服务器**
  - `amberjs debug --web` 启动 Web UI
  - 静态文件服务
  - WebSocket 代理

#### 3.2 前端界面
- [ ] **调试控制台**
  - 断点列表和状态
  - 执行控制按钮
  - 调试状态显示

- [ ] **源码查看器**
  - 语法高亮
  - 行号显示
  - 断点标记
  - 当前执行行高亮

- [ ] **变量视图**
  - 当前 - 局部变量和作用域变量
 参数
  - 全局变量
  - Watch 表达式

- [ ] **调用栈**
  - 栈帧列表
  - 函数名称和位置
  - 点击跳转

- [ ] **控制台输出**
  - console.log 输出
  - 错误信息
  - 调试命令历史

#### 3.3 高级功能
- [ ] **性能分析**
  - CPU 性能剖析
  - 内存使用分析
  - 热力图显示

- [ ] **网络监控**
  - Fetch/XHR 请求
  - 响应时间和状态
  - 请求/响应头

### 阶段 4: VS Code 扩展
**优先级**: 🟡 中
**预计时间**: 8-10 小时

#### 4.1 扩展基础
- [ ] **扩展结构**
  - package.json 配置
  - TypeScript 源码
  - 调试适配器

#### 4.2 调试适配器
- [ ] **Debug Adapter Protocol**
  - 实现 DAP 服务器
  - 消息路由
  - 状态管理

- [ ] **配置支持**
  - launch.json 解析
  - 环境变量
  - 参数传递

#### 4.3 VS Code 集成
- [ ] **调试视图**
  - 断点视图
  - 变量视图
  - 调用栈视图
  - 监视视图

- [ ] **编辑器集成**
  - 断点标记
  - 当前行高亮
  - 悬停提示

- [ ] **命令面板**
  - 调试命令
  - 快速操作

### 阶段 5: 高级调试特性
**优先级**: 🟢 低
**预计时间**: 10-12 小时

#### 5.1 时间旅行调试
- [ ] **执行历史记录**
  - 每个执行步骤记录
  - 状态快照
  - 内存快照

- [ ] **时间导航**
  - 前进/后退操作
  - 状态回溯
  - 重放执行

#### 5.2 并发调试
- [ ] **多脚本调试**
  - 并发脚本管理
  - 独立调试会话
  - 交叉调试

#### 5.3 分布式追踪
- [ ] **跨进程调试**
  - 子进程调试
  - Worker 调试
  - 消息传递追踪

---

## 🛠️ 技术实现方案

### Chrome DevTools 协议实现

```rust
// WebSocket 调试服务器
pub struct DebugServer {
    port: u16,
    runtime: Arc<RuntimeLite>,
    debugger: Arc<DebuggerEngine>,
    connections: Arc<Mutex<Vec<WebSocketConnection>>>,
}

impl DebugServer {
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port)).await?;
        loop {
            let (socket, _) = listener.accept().await?;
            let connection = WebSocketConnection::new(socket);
            self.handle_connection(connection).await;
        }
    }
}

// Chrome DevTools 协议消息
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromeMessage {
    id: Option<u64>,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromeResponse {
    id: u64,
    result: serde_json::Value,
}

// 协议域实现
pub struct RuntimeDomain {
    contexts: HashMap<ExecutionContextId, v8::Global<v8::Context>>,
}

impl RuntimeDomain {
    pub fn handle_evaluate(&self, params: &EvaluateParams) -> Result<EvaluateResult> {
        // 在指定上下文中求值
        let context = self.contexts.get(&params.context_id)?;
        // 执行表达式并返回结果
    }
}
```

### Web UI 调试器

```rust
// Web UI 服务器
pub struct WebDebugger {
    server: TinyHttpServer,
    debugger: Arc<DebuggerEngine>,
    websocket_tx: mpsc::UnboundedSender<WebSocketMessage>,
}

impl WebDebugger {
    pub fn start(port: u16) -> Result<Self> {
        let routes = vec![
            ("GET", "/", serve_index_html),
            ("GET", "/client.js", serve_client_js),
            ("GET", "/style.css", serve_style_css),
            ("WS", "/ws", handle_websocket),
        ];
        // 启动 HTTP 服务器
    }
}
```

### CLI 调试命令

```rust
// Debug 子命令
#[derive(Subcommand)]
pub enum DebugCommand {
    /// 调试脚本文件
    Script {
        file: PathBuf,
        #[arg(long)]
        break_at: Option<u32>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        web: bool,
    },
    /// 附加到进程
    Attach {
        pid: u32,
        #[arg(long)]
        port: Option<u16>,
    },
    /// 启动检查器
    Inspect {
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        web: bool,
    },
}

// 调试会话
pub struct DebugSession {
    runtime: RuntimeLite,
    debugger: DebuggerEngine,
    console: DebugConsole,
}

impl DebugSession {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            let command = self.console.read_command().await?;
            match command {
                DebugCommand::Break(line) => self.set_breakpoint(line),
                DebugCommand::Continue => self.continue_execution(),
                DebugCommand::Next => self.next(),
                DebugCommand::Print(expr) => self.print_variable(expr),
                DebugCommand::Backtrace => self.show_backtrace(),
                DebugCommand::Exit => break,
            }
        }
        Ok(())
    }
}
```

---

## 📊 测试策略

### 单元测试
- [ ] **CLI 命令测试**
  - 调试命令解析
  - 参数验证
  - 错误处理

- [ ] **Chrome DevTools 协议测试**
  - 消息序列化/反序列化
  - 协议域方法测试
  - WebSocket 连接测试

- [ ] **调试器引擎测试**
  - 断点设置和触发
  - 执行控制测试
  - 变量检查测试

### 集成测试
- [ ] **完整调试会话**
  - CLI 调试流程
  - Web UI 调试
  - VS Code 扩展调试

- [ ] **Chrome DevTools 集成**
  - Chrome 浏览器连接
  - 断点调试
  - 变量查看

### 性能测试
- [ ] **调试性能**
  - 调试模式启动时间
  - WebSocket 延迟
  - 大量断点性能

---

## 📈 性能目标

- **CLI 调试启动**: < 50ms
- **WebSocket 连接**: < 10ms
- **断点响应**: < 1ms
- **变量检查**: < 5ms（复杂对象）
- **Web UI 加载**: < 100ms
- **Chrome DevTools 同步**: < 5ms
- **VS Code 集成**: < 20ms

---

## 🔮 后续工作（Stage 60）

### 高级调试特性
- 时间旅行调试完善
- 并发调试优化
- 分布式追踪增强

### 生态系统
- 更多 IDE 插件（IntelliJ, Eclipse）
- 调试器扩展 API
- 社区调试工具

---

## 📝 总结

Stage 59 将为 Amber 构建完整的可视化调试生态系统，使其成为专业级的 JavaScript/TypeScript 调试平台。通过 CLI 调试、Chrome DevTools 协议、Web UI 和 VS Code 扩展，Amber 将为开发者提供世界级的调试体验。

**预计完成时间**: 32-40 小时
**主要文件数量**: 25-35 个新文件
**测试覆盖**: 90%+
**性能影响**: 调试模式 < 15% 性能开销

---

**状态**: 📝 计划制定完成
**下一步**: 开始实现 CLI 调试命令
**维护者**: Henry Zhang & Claude Code Assistant
**版本**: v0.1.0 (Stage 59 Planning Complete - Visual Debugging & Chrome DevTools)
