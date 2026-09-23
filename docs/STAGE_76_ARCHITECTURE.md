# Stage 76 性能分析器增强架构设计

## 现有基础设施分析

### 已有模块
1. **monitor 模块** - 性能监控、数据存储、告警系统
2. **profiler.rs** - 基础性能分析器（简单的时间/内存跟踪）

### 架构优势
- ✅ 完整的监控基础设施（DataStore、PerformanceMonitor）
- ✅ 数据存储和查询系统
- ✅ 告警和仪表板系统
- ✅ 基础的 Profiler API

### 需要增强的部分
- ❌ 函数级性能跟踪
- ❌ 调用栈分析
- ❌ 热点函数识别
- ❌ 性能报告生成
- ❌ 内存泄漏检测
- ❌ 火焰图数据生成

## 增强架构设计

### 1. 新的模块结构

```
src/monitor/profiler/           # 新的性能分析器模块
├── collector.rs                # 数据采集器（增强 V8 集成）
│   ├── FunctionTracker         # 函数调用跟踪
│   ├── CallStackAnalyzer       # 调用栈分析
│   ├── MemoryTracker           # 内存分配跟踪
│   └── EventCollector          # 性能事件收集
├── analyzer/                   # 性能分析引擎
│   ├── hotspot.rs             # 热点函数识别
│   ├── bottleneck.rs          # 瓶颈分析
│   ├── memory_analyzer.rs     # 内存分析
│   └── trend_analyzer.rs      # 趋势分析
├── storage/                    # 性能数据存储
│   ├── ring_buffer.rs         # 环形缓冲区
│   ├── sampling.rs            # 采样策略
│   └── compression.rs         # 数据压缩
├── report/                     # 报告生成器
│   ├── flamegraph.rs          # 火焰图生成
│   ├── timeline.rs            # 时间线报告
│   ├── summary.rs             # 性能摘要
│   └── json_report.rs         # JSON 格式报告
├── cli_integration.rs          # CLI 集成
└── mod.rs                      # 模块入口

src/profiler.rs                 # 增强现有 profiler
├── AdvancedProfiler            # 高级性能分析器
├── ProfilingConfig             # 分析配置
└── RealTimeProfiler            # 实时性能监控
```

### 2. 核心增强功能

#### 2.1 FunctionTracker - 函数级跟踪
```rust
pub struct FunctionTracker {
    /// V8 上下文中的函数调用钩子
    /// 自动在函数入口/出口插入性能测量代码
    /// 支持异步函数跟踪
}

impl FunctionTracker {
    /// 跟踪函数执行
    pub fn track_function(
        &self,
        isolate: &mut Isolate,
        function_name: &str,
        start_time: Instant,
    ) -> FunctionTraceHandle;

    /// 记录函数返回
    pub fn record_return(
        &self,
        handle: FunctionTraceHandle,
        end_time: Instant,
        return_value: Value,
    );

    /// 获取函数调用统计
    pub fn get_function_stats(&self, function_name: &str) -> FunctionStats;
}
```

#### 2.2 CallStackAnalyzer - 调用栈分析
```rust
pub struct CallStackAnalyzer {
    /// 调用栈深度限制
    max_depth: usize,
    /// 当前调用栈
    current_stack: Vec<StackFrame>,
}

pub struct StackFrame {
    pub function_name: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub execution_time: Duration,
    pub memory_used: usize,
}

impl CallStackAnalyzer {
    /// 分析调用栈
    pub fn analyze_stack(&self) -> CallStackAnalysis {
        CallStackAnalysis {
            hot_path: self.identify_hot_path(),
            bottlenecks: self.identify_bottlenecks(),
            recursion: self.detect_recursion(),
        }
    }
}
```

#### 2.3 HotspotAnalyzer - 热点分析
```rust
pub struct HotspotAnalyzer {
    /// 函数执行时间统计
    execution_times: HashMap<String, ExecutionTimeStats>,
    /// 内存分配统计
    memory_stats: HashMap<String, MemoryStats>,
    /// 调用次数统计
    call_counts: HashMap<String, CallCount>,
}

pub struct ExecutionTimeStats {
    pub total_time: Duration,
    pub avg_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub call_count: u64,
}

impl HotspotAnalyzer {
    /// 识别性能热点
    pub fn identify_hotspots(&self) -> Vec<Hotspot> {
        // 按执行时间排序
        // 按内存使用排序
        // 按调用次数排序
        // 综合评分算法
    }
}
```

### 3. 数据存储增强

#### 3.1 环形缓冲区
```rust
pub struct RingBuffer<T> {
    buffer: Vec<T>,
    head: usize,
    tail: usize,
    size: usize,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    /// 高性能写入（无锁）
    pub fn push(&mut self, item: T);

    /// 高性能读取（无锁）
    pub fn pop(&mut self) -> Option<T>;

    /// 批量写入
    pub fn push_batch(&mut self, items: &[T]);
}
```

#### 3.2 智能采样
```rust
pub struct SamplingStrategy {
    /// 采样率 (0.0 - 1.0)
    sample_rate: f64,
    /// 动态采样调整
    dynamic_sampling: bool,
    /// 最小采样间隔
    min_interval: Duration,
}

impl SamplingStrategy {
    /// 决定是否采样
    pub fn should_sample(&self, event: &PerformanceEvent) -> bool {
        // 动态调整采样率
        // 基于系统负载调整
        // 基于事件重要性调整
    }
}
```

### 4. 报告生成器

#### 4.1 火焰图生成
```rust
pub struct FlameGraphGenerator {
    /// 火焰图数据
    stack_data: Vec<StackSample>,
}

pub struct StackSample {
    pub frame: StackFrame,
    pub value: u64, // 执行时间（纳秒）
}

impl FlameGraphGenerator {
    /// 生成火焰图数据（Firefox Profiler 格式）
    pub fn generate_flamegraph_data(&self) -> FlameGraphData {
        FlameGraphData {
            stacks: self.convert_to_flamegraph_format(),
            frames: self.extract_unique_frames(),
        }
    }

    /// 生成 HTML 火焰图
    pub fn generate_html(&self) -> String {
        // 使用 D3.js 或自定义渲染
        // 生成可交互的火焰图
    }
}
```

#### 4.2 性能摘要报告
```rust
pub struct PerformanceSummary {
    pub total_execution_time: Duration,
    pub function_count: usize,
    pub hotspot_functions: Vec<Hotspot>,
    pub memory_summary: MemorySummary,
    pub recommendations: Vec<OptimizationRecommendation>,
}

pub struct OptimizationRecommendation {
    pub function: String,
    pub issue_type: OptimizationIssue,
    pub description: String,
    pub suggestion: String,
    pub impact: ImpactLevel,
}
```

### 5. 集成设计

#### 5.1 与现有 Monitor 模块集成
```rust
/// 增强的 PerformanceMonitor
pub struct EnhancedPerformanceMonitor {
    /// 基础监控器
    base_monitor: PerformanceMonitor,
    /// 高级分析器
    profiler: AdvancedProfiler,
    /// 数据存储
    storage: ProfilingStorage,
}

impl EnhancedPerformanceMonitor {
    /// 启动性能分析
    pub fn start_profiling(&mut self, config: ProfilingConfig);

    /// 停止性能分析并生成报告
    pub fn stop_profiling(&mut self) -> PerformanceReport;

    /// 实时性能快照
    pub fn get_realtime_snapshot(&self) -> RealtimeSnapshot;
}
```

#### 5.2 与 V8 引擎集成
```rust
/// V8 性能钩子
pub struct V8ProfilerHooks {
    /// 在函数调用前调用
    pub on_function_call: Arc<dyn Fn(String)>,
    /// 在函数返回后调用
    pub on_function_return: Arc<dyn Fn(String, Duration)>,
    /// 在内存分配时调用
    pub on_memory_allocation: Arc<dyn Fn(AllocationInfo)>,
}

impl V8ProfilerHooks {
    /// 注册到 V8 Isolate
    pub fn register(&self, isolate: &mut Isolate) {
        // 使用 V8 的 Profiler API
        // 或使用 CustomPromiseResolveCallback
    }
}
```

#### 5.3 CLI 集成
```rust
/// CLI 性能分析命令
pub struct ProfilingCommand {
    /// 输出格式
    format: ReportFormat,
    /// 输出文件
    output: Option<PathBuf>,
    /// 分析模式
    mode: ProfilingMode,
    /// 采样率
    sample_rate: f64,
}

impl ProfilingCommand {
    /// 执行性能分析
    pub fn execute(&self) -> Result<()> {
        // 启动运行时
        // 启用性能分析
        // 执行脚本
        // 生成报告
    }
}
```

### 6. 性能目标

#### 6.1 开销控制
| 功能 | 开销目标 | 实现策略 |
|------|---------|----------|
| 函数跟踪 | < 0.1% | 采样 + 优化钩子 |
| 调用栈分析 | < 0.3% | 限制深度 + 环形缓冲 |
| 热点分析 | < 0.2% | 增量计算 |
| 报告生成 | 离线 | 后台线程 |

#### 6.2 数据准确性
| 指标 | 精度目标 | 验证方法 |
|------|---------|----------|
| 执行时间 | ±1% | 系统时钟对比 |
| 内存统计 | ±5% | 系统监控对比 |
| 调用次数 | 100% | 计数器验证 |

### 7. 关键技术决策

#### 7.1 V8 集成策略
- **选择 1**: 使用 V8 内置 Profiler API ✅（推荐）
  - 优点：官方支持、准确性高
  - 缺点：API 限制

- **选择 2**: 手动插桩（装饰器模式）
  - 优点：灵活控制
  - 缺点：需要修改用户代码

#### 7.2 数据存储策略
- **选择 1**: 环形缓冲区 + 压缩 ✅（推荐）
  - 优点：内存高效、固定内存占用
  - 缺点：需要采样

- **选择 2**: 直接存储到磁盘
  - 优点：无内存限制
  - 缺点：I/O 开销大

#### 7.3 采样策略
- **选择 1**: 动态采样 ✅（推荐）
  - 优点：智能调整、准确性高
  - 缺点：实现复杂

- **选择 2**: 固定采样率
  - 优点：实现简单
  - 缺点：无法适应不同场景

### 8. 实施优先级

#### Phase 1: 基础增强 (Day 1)
1. 增强 FunctionTracker
2. 实现 CallStackAnalyzer
3. 基础数据存储

#### Phase 2: 分析引擎 (Day 2)
1. HotspotAnalyzer
2. MemoryAnalyzer
3. 性能数据聚合

#### Phase 3: 报告生成 (Day 3)
1. 火焰图生成
2. 性能摘要报告
3. CLI 集成

#### Phase 4: 优化测试 (Day 4)
1. 性能调优
2. 全面测试
3. 文档完善

## 结论

这个增强架构充分利用了现有的 monitor 基础设施，通过模块化设计实现高性能、低开销的性能分析能力。关键设计原则：

1. **零开销原则**: 监控系统开销 < 1%
2. **模块化设计**: 独立的组件，易于测试和维护
3. **智能采样**: 平衡准确性和性能
4. **向后兼容**: 不破坏现有功能

这个架构将使 Amber 具备企业级性能分析能力，为 AI 时代的 JavaScript/TypeScript 脚本优化提供强大支持。
