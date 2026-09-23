# Stage 58 实施计划 - 调试器集成 (Debugger Integration)

## 📋 阶段概述

Stage 58 专注于为 Amber 实现完整的调试器系统，提供生产级的调试能力，包括断点设置、单步执行、变量检查和调用栈查看。这将大幅提升开发者体验，使 Amber 成为专业级的 JavaScript/TypeScript 运行时。

**目标**: 构建完整的 V8 调试器集成，支持 CLI 调试、REPL 调试和远程调试。

---

## 🎯 成功标准

### 核心调试功能
- [ ] **断点管理**: 设置、删除、启用/禁用断点
- [ ] **单步执行**: Step Over, Step Into, Step Out
- [ ] **变量检查**: 查看和修改变量值，包括作用域链
- [ ] **调用栈**: 显示完整的调用堆栈和栈帧信息
- [ ] **条件断点**: 基于条件的断点触发
- [ ] **异常捕获**: 自动在异常处停止

### 调试界面
- [ ] **CLI 调试器**: `amberjs debug <file>` 命令
- [ ] **REPL 调试模式**: `.debug` 命令进入调试
- [ ] **可视化输出**: 友好的调试信息显示
- [ ] **热键支持**: 快速调试操作（Ctrl+C, Ctrl+N, etc.）

### 高级特性
- [ ] **远程调试**: 通过 WebSocket 或 TCP 调试
- [ ] **性能分析**: CPU 性能剖析和内存分析
- [ ] **代码覆盖率**: 行级和分支覆盖率
- [ ] **时间旅行调试**: 回溯执行状态

---

## 📝 任务分解

### 阶段 1: 调试器核心架构
**优先级**: 🔴 高
**预计时间**: 4-5 小时

#### 1.1 调试器框架
- [ ] **创建 debug 模块**
  - `src/debugger/mod.rs` - 调试器模块入口
  - `src/debugger/engine.rs` - 调试引擎核心
  - `src/debugger/breakpoint.rs` - 断点管理
  - `src/debugger/stack_trace.rs` - 调用栈管理
  - `src/debugger/variable_scope.rs` - 变量作用域

#### 1.2 V8 调试协议集成
- [ ] **V8 Debugger API**
  - 集成 `v8::Debug` 事件系统
  - 实现 `DebugEventListener`
  - 处理断点、异常、脚本编译事件

- [ ] **调试上下文**
  - 创建独立的调试 Isolate
  - 管理调试上下文生命周期
  - 同步主执行上下文状态

#### 1.3 断点系统
- [ ] **断点管理**
  - 断点创建和删除
  - 行号和条件断点
  - 断点状态管理（启用/禁用）

- [ ] **断点匹配**
  - 源码映射支持
  - 动态脚本断点
  - 匿名函数断点

### 阶段 2: 执行控制
**优先级**: 🔴 高
**预计时间**: 3-4 小时

#### 2.1 单步执行
- [ ] **Step Operations**
  - Step Over (跨过函数调用)
  - Step Into (进入函数内部)
  - Step Out (跳出当前函数)
  - Continue (继续执行到下一断点)

#### 2.2 执行状态
- [ ] **暂停/恢复**
  - 安全的暂停机制
  - 状态保存和恢复
  - 异步操作处理

#### 2.3 异常处理
- [ ] **异常断点**
  - 未捕获异常自动停止
  - Promise 拒绝处理
  - 自定义异常过滤

### 阶段 3: 变量和状态检查
**优先级**: 🔴 高
**预计时间**: 3-4 小时

#### 3.1 变量检查
- [ ] **作用域链**
  - 当前作用域变量
  - 闭包变量访问
  - 全局对象属性

- [ ] **对象检查**
  - 对象属性遍历
  - 原型链检查
  - Symbol 属性支持

#### 3.2 变量修改
- [ ] **值修改**
  - 原生类型修改
  - 对象属性更新
  - 数组元素修改

#### 3.3 表达式求值
- [ ] **调试表达式**
  - 安全表达式求值
  - 复杂表达式支持
  - 上下文感知求值

### 阶段 4: 调用栈和源码映射
**优先级**: 🟡 中
**预计时间**: 2-3 小时

#### 4.1 调用栈
- [ ] **栈帧信息**
  - 函数名称和位置
  - 参数值
  - 局部变量

- [ ] **栈导航**
  - 向上/向下导航
  - 栈帧切换
  - 返回地址

#### 4.2 源码映射
- [ ] **TypeScript 支持**
  - TS 到 JS 源码映射
  - 原始行号显示
  - 转换信息

### 阶段 5: CLI 集成
**优先级**: 🟡 中
**预计时间**: 2-3 小时

#### 5.1 Debug 命令
- [ ] **基础命令**
  - `amberjs debug <file>` - 启动调试
  - `amberjs debug --inspect-brk <port>` - 远程调试

- [ ] **调试操作**
  - `break <line>` - 设置断点
  - `continue` - 继续执行
  - `next` - 下一步
  - `step` - 单步
  - `finish` - 完成函数
  - `print <expr>` - 打印变量
  - `backtrace` - 调用栈

#### 5.2 REPL 集成
- [ ] **调试模式**
  - `.debug` - 进入调试模式
  - `.continue` - 继续执行
  - `.step` - 单步执行

### 阶段 6: 高级特性
**优先级**: 🟢 低
**预计时间**: 4-5 小时

#### 6.1 远程调试
- [ ] **协议支持**
  - Chrome DevTools Protocol
  - WebSocket 连接
  - 多客户端支持

#### 6.2 性能分析
- [ ] **CPU 分析**
  - 函数调用统计
  - 执行时间测量
  - 热力图生成

- [ ] **内存分析**
  - 堆快照
  - 泄漏检测
  - 对象分配追踪

---

## 🛠️ 技术实现方案

### 架构设计

```rust
// 调试器引擎
pub struct DebuggerEngine {
    isolate: v8::Isolate,
    debug_context: v8::Global<v8::Context>,
    breakpoints: HashMap<String, Breakpoint>,
    current_frame: Option<StackFrame>,
    event_listener: Box<dyn DebugEventListener>,
}

// 断点管理
pub struct Breakpoint {
    id: String,
    script_id: String,
    line_number: u32,
    condition: Option<String>,
    enabled: bool,
    hit_count: u32,
}

// 调用栈帧
pub struct StackFrame {
    index: u32,
    function_name: String,
    script_id: String,
    line_number: u32,
    column_number: u32,
    local_variables: HashMap<String, v8::Local<'_, v8::Value>>,
}

// 变量作用域
pub struct VariableScope {
    scope_type: ScopeType,  // Global, Local, Closure, Catch
    object: v8::Global<v8::Object>,
}
```

### V8 调试 API

```rust
// V8 调试事件处理
impl v8::DebugEventListener for DebuggerEngine {
    fn handle(&self, event: v8::DebugEvent, exec_state: v8::Global<v8::DebugExecutionState>) {
        match event {
            v8::DebugEvent::Break => self.handle_break(exec_state),
            v8::DebugEvent::Exception => self.handle_exception(exec_state),
            v8::DebugEvent::CompileError => self.handle_compile_error(exec_state),
            _ => {}
        }
    }
}

// 断点检查
fn check_breakpoints(&self, exec_state: &v8::Global<v8::DebugExecutionState>) -> bool {
    let location = exec_state.get_break_location();
    // 检查是否有断点在此位置
    self.breakpoints.values().any(|bp| {
        bp.enabled && bp.script_id == location.script_id() && bp.line_number == location.line_number()
    })
}
```

### CLI 集成

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
    },
    /// 附加到进程
    Attach {
        pid: u32,
        #[arg(long)]
        port: Option<u16>,
    },
}
```

---

## 📊 测试策略

### 单元测试
- [ ] **断点系统测试**
  - 断点设置和触发
  - 条件断点测试
  - 多断点管理

- [ ] **执行控制测试**
  - 单步执行验证
  - 暂停/恢复测试
  - 异常处理测试

### 集成测试
- [ ] **完整调试会话**
  - 断点 → 暂停 → 检查 → 继续
  - 变量修改验证
  - 调用栈导航

- [ ] **REPL 调试测试**
  - 交互式调试
  - 调试命令测试
  - 多行代码调试

### 性能测试
- [ ] **调试开销**
  - 启用/禁用调试的性能差异
  - 大量断点场景
  - 远程调试性能

---

## 📈 性能目标

- **调试启动时间**: < 100ms
- **断点响应延迟**: < 1ms
- **变量检查性能**: < 5ms（复杂对象）
- **内存开销**: < 20MB（启用调试）
- **远程调试延迟**: < 10ms

---

## 🔮 后续工作（Stage 59）

### 可视化调试界面
- Web UI 调试器
- VS Code 扩展
- Chrome DevTools 集成

### 高级调试特性
- 时间旅行调试
- 并发调试
- 分布式追踪

---

## 📝 总结

Stage 58 将为 Amber 添加完整的调试能力，使其成为专业级的 JavaScript/TypeScript 运行时。这对于开发者体验和生产力至关重要，特别是在处理复杂应用和 AI 工作负载时。

**预计完成时间**: 18-24 小时
**主要文件数量**: 15-20 个新文件
**测试覆盖**: 90%+
**性能影响**: 调试模式 < 20% 性能开销

---

**状态**: 📝 计划制定完成
**下一步**: 开始实现调试器核心架构
**维护者**: Henry Zhang & Claude Code Assistant
**版本**: v0.1.0 (Stage 58 Planning Complete - Debugger Integration)
