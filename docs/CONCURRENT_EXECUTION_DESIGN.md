# Amber 真正的并发执行架构设计

## 🎯 目标
实现支持 10000+ 并发脚本的真正并行执行引擎，绕过 V8 线程限制，实现线性扩展性能。

## 📊 现状分析

### 现有基础设施
- ✅ **async_io.rs**: 异步I/O优化，但每脚本创建新Runtime（性能瓶颈）
- ✅ **lock_free.rs**: 无锁并发原语（计数器、调度器、分片锁）
- ✅ **zero_copy.rs**: 零拷贝数据传输（缓冲区、通道）
- ✅ **concurrent_tests.rs**: 基础并发测试框架

### 核心问题
1. **V8线程限制**: V8 Isolate必须在创建线程上执行
2. **Runtime创建开销**: 每脚本创建新Runtime实例成本高昂
3. **缺乏统一调度**: 没有工作窃取或负载均衡机制
4. **内存使用峰值**: 10000个并发脚本会导致内存爆炸

## 🏗️ 架构设计

### 核心组件

#### 1. ConcurrentRuntimePool (并发运行时池)
**目标**: 绕过V8线程限制，实现线程本地的Runtime实例复用

**设计**:
```rust
struct ConcurrentRuntimePool {
    /// 每个线程的Runtime池 (线程局部存储)
    thread_local_pools: ThreadLocal<Mutex<Vec<Runtime>>>,
    /// 运行时配置
    config: RuntimePoolConfig,
    /// 统计信息
    stats: Arc<lock_free::AtomicStats>,
}
```

**关键特性**:
- 线程本地存储：每个线程维护自己的Runtime池
- 懒加载：按需创建Runtime实例
- 自动回收：空闲Runtime实例被回收
- 预热机制：启动时预创建部分实例

#### 2. WorkStealingScheduler (工作窃取调度器)
**目标**: 实现高效的任务调度和负载均衡

**设计**:
```rust
struct WorkStealingScheduler {
    /// 每个线程的任务队列
    thread_queues: Vec<Arc<Mutex<Vec<Task>>>>,
    /// 工作窃取通道
    steal_channels: Vec<ZeroCopyChannel<Task>>,
    /// 任务调度器
    task_scheduler: Arc<lock_free::LockFreeTaskScheduler>,
}
```

**关键特性**:
- 工作窃取：空闲线程从其他线程队列窃取任务
- 优先级队列：基于 lock_free::LockFreeQueue
- 负载感知：根据队列长度动态调整窃取策略
- 零拷贝传递：使用 zero_copy.rs 通道

#### 3. BatchExecutor (批处理器)
**目标**: 提供简单的批量执行API，支持10000+脚本

**设计**:
```rust
struct BatchExecutor {
    /// 并发运行时池
    runtime_pool: Arc<ConcurrentRuntimePool>,
    /// 工作窃取调度器
    scheduler: Arc<WorkStealingScheduler>,
    /// 零拷贝管理器
    zero_copy_manager: Arc<ZeroCopyManager>,
    /// 并发限制
    max_concurrent: usize,
}
```

**关键特性**:
- 批量提交：一次性提交1000+脚本
- 流式处理：实时返回结果
- 背压控制：避免系统过载
- 性能监控：实时性能指标

#### 4. ConcurrentExecutionStats (并发执行统计)
**目标**: 详细跟踪并发执行性能

**设计**:
```rust
struct ConcurrentExecutionStats {
    /// 总提交任务数
    total_submitted: Arc<lock_free::LockFreeCounter>,
    /// 成功完成任务数
    total_completed: Arc<lock_free::LockFreeCounter>,
    /// 失败任务数
    total_failed: Arc<lock_free::LockFreeCounter>,
    /// 平均执行时间
    avg_execution_time: Arc<std::sync::atomic::AtomicU64>,
    /// 峰值并发数
    peak_concurrent: Arc<std::sync::atomic::AtomicUsize>,
}
```

## 🔄 执行流程

### 批量执行流程
```
1. 提交 10000 个脚本
   ↓
2. BatchExecutor 接收脚本
   ↓
3. 脚本分批（每批 1000 个）
   ↓
4. WorkStealingScheduler 分发任务到线程队列
   ↓
5. 每个线程从本地队列获取任务
   ↓
6. 从 ConcurrentRuntimePool 获取 Runtime 实例
   ↓
7. 执行脚本并收集结果
   ↓
8. 返回结果（流式或批量）
```

### 工作窃取流程
```
Thread A 忙碌 (队列长度: 100)
Thread B 空闲 (队列长度: 0)
   ↓
Thread B 从 Thread A 窃取 10 个任务
   ↓
Thread B 执行窃取的任务
   ↓
实现负载均衡
```

## 📈 性能优化策略

### 1. 内存优化
- **Runtime实例复用**: 避免重复创建Runtime
- **零拷贝数据传输**: 使用 Arc<[u8]> 共享缓冲区
- **对象池**: 复用Task和Result对象
- **内存预分配**: 批量分配减少内存碎片

### 2. 调度优化
- **工作窃取**: 减少锁竞争
- **分片锁**: 使用 sharded_lock 减少争用
- **原子操作**: 使用 lock_free::LockFreeCounter
- **批量处理**: 合并小任务减少调度开销

### 3. V8优化
- **线程本地池**: 绕过V8线程限制
- **JIT预热**: 预编译常用代码
- **Isolate复用**: 减少Isolate创建开销
- **上下文保持**: 保持V8上下文活跃

## 🎯 性能目标

### 目标指标
- **并发脚本数**: 10,000+
- **吞吐量**: 50,000 scripts/sec (相比当前 59 ops/sec)
- **启动时间**: < 10ms (批量执行)
- **内存使用**: < 500MB (10,000 scripts)
- **线性扩展**: CPU核心数 × 单核性能

### 基准对比
| 指标 | 当前实现 | 目标实现 | 提升倍数 |
|------|----------|----------|----------|
| 并发脚本数 | ~100 | 10,000+ | 100x |
| 吞吐量 | 59 ops/sec | 50,000 ops/sec | 847x |
| 内存使用 | 100MB/100 scripts | 500MB/10,000 scripts | 20x 效率 |

## 🔧 API 设计

### 高层API
```rust
impl Runtime {
    /// 并发执行多个脚本
    pub async fn execute_concurrent(
        &self,
        scripts: Vec<String>,
        max_concurrent: usize,
    ) -> Result<Vec<ScriptResult>, ConcurrentExecutionError> {
        // 使用 BatchExecutor 实现
    }

    /// 流式并发执行（实时返回结果）
    pub async fn execute_concurrent_stream(
        &self,
        scripts: Vec<String>,
    ) -> impl Stream<Item = ScriptResult> {
        // 流式返回结果
    }

    /// 获取并发执行统计
    pub fn get_concurrent_stats(&self) -> ConcurrentExecutionStats {
        // 返回详细统计
    }
}
```

### 中层API
```rust
/// 批处理器
impl BatchExecutor {
    pub fn new(max_concurrent: usize) -> Self;
    pub async fn execute_batch(&self, scripts: Vec<String>) -> Vec<ScriptResult>;
    pub fn get_stats(&self) -> ConcurrentExecutionStats;
}
```

### 底层API
```rust
/// 并发运行时池
impl ConcurrentRuntimePool {
    pub fn get_runtime(&self) -> Option<Runtime>;
    pub fn return_runtime(&self, runtime: Runtime);
    pub fn prewarm(&self, count: usize);
}
```

## 📝 实现计划

### 阶段 1: 基础架构
- [ ] 创建 ConcurrentRuntimePool
- [ ] 实现线程本地Runtime池
- [ ] 添加基础统计跟踪

### 阶段 2: 调度器
- [ ] 创建 WorkStealingScheduler
- [ ] 实现工作窃取算法
- [ ] 集成 lock_free 组件

### 阶段 3: 批处理器
- [ ] 创建 BatchExecutor
- [ ] 实现批量执行API
- [ ] 添加背压控制

### 阶段 4: 集成测试
- [ ] 创建完整测试套件
- [ ] 性能基准测试
- [ ] 10000+ 并发验证

### 阶段 5: 优化调优
- [ ] 性能分析和调优
- [ ] 内存使用优化
- [ ] 生产环境部署

## 🎓 技术要点

### V8 线程限制解决方案
- **问题**: V8 Isolate只能在创建线程上使用
- **解决**: 每个线程维护自己的Runtime池
- **实现**: 使用 `thread_local!` 宏和 `ThreadLocal` 类型

### 工作窃取算法
- **问题**: 传统调度器会导致负载不均
- **解决**: 工作窃取（Work Stealing）
- **实现**: 使用 lock_free 队列和 crossbeam 通道

### 零拷贝优化
- **问题**: 大量数据传输导致内存拷贝开销
- **解决**: 零拷贝缓冲区（Arc<[u8]>）
- **实现**: 使用 zero_copy.rs 模块

### 背压控制
- **问题**: 提交过快导致系统过载
- **解决**: 信号量限制并发数
- **实现**: 使用 tokio::sync::Semaphore

## 📊 测试策略

### 单元测试
- ConcurrentRuntimePool 基本操作
- WorkStealingScheduler 窃取算法
- BatchExecutor 批量执行

### 集成测试
- 1000 脚本并发执行
- 5000 脚本并发执行
- 10000 脚本并发执行

### 性能测试
- 吞吐量测试（目标: 50,000 ops/sec）
- 延迟测试（目标: < 10ms p99）
- 内存测试（目标: < 500MB for 10K scripts）

### 压力测试
- 长时间运行稳定性
- 内存泄漏检测
- 错误恢复测试

## 🚀 预期成果

### 性能提升
- **吞吐量**: 847x 提升（59 → 50,000 ops/sec）
- **并发能力**: 100x 提升（100 → 10,000+ scripts）
- **资源效率**: 20x 提升（内存使用优化）

### 架构价值
- **可扩展性**: 线性扩展至 CPU 核心数
- **稳定性**: 工作窃取保证负载均衡
- **易用性**: 简单的高层API

### 项目意义
- **竞争能力**: 缩小与 Bun 的性能差距
- **AI 工作负载**: 支持大规模并行推理
- **生产就绪**: 企业级并发执行能力

---

**状态**: 设计完成，准备实现
**下一步**: 开始 TDD 实现
