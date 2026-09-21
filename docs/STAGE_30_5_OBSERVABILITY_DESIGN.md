# Stage 30.5: 生产监控与可观测性 - 架构设计

## 📋 设计概览

**目标**: 集成企业级监控系统，提供完整的可观测性能力
**模块**: `src/observability/` - 监控与可观测性核心模块
**测试**: `tests/stage_30_5_observability_tests.rs` - 完整测试套件

## 🏗️ 架构设计

### 模块结构

```
src/observability/
├── mod.rs                    # 模块导出
├── prometheus_exporter.rs    # Prometheus 指标导出
├── jaeger_tracer.rs         # Jaeger 分布式追踪
├── structured_logging.rs    # 结构化日志
├── metrics.rs               # 自定义指标定义
└── alerting.rs              # 告警系统
```

### 核心组件

#### 1. PrometheusExporter - 指标导出器
```rust
pub struct PrometheusExporter {
    registry: Registry,
    http_server: HttpServer,
    metrics: Arc<RwLock<HashMap<String, Metric>>>,
}

impl PrometheusExporter {
    // 启动 HTTP 服务器暴露指标
    pub async fn start_server(&self, addr: SocketAddr) -> Result<()>;

    // 注册自定义指标
    pub fn register_metric(&self, name: String, metric: Metric) -> Result<()>;

    // 更新指标值
    pub fn update_metric(&self, name: String, value: f64) -> Result<()>;

    // 获取所有指标
    pub fn gather_metrics(&self) -> Result<Vec<MetricFamily>>;
}
```

**支持的指标类型**:
- Counter: 累计计数器（如请求数、错误数）
- Gauge: 可变仪表盘（如内存使用、CPU使用率）
- Histogram: 直方图（如延迟分布）
- Summary: 摘要（如P95、P99延迟）

#### 2. JaegerTracer - 分布式追踪器
```rust
pub struct JaegerTracer {
    tracer: opentelemetry_jaeger::Tracer,
    span_processor: BatchSpanProcessor,
}

impl JaegerTracer {
    // 创建新的追踪span
    pub fn create_span(&self, operation_name: &str) -> Span;

    // 记录属性
    pub fn set_attribute(&self, span: &Span, key: &str, value: &str);

    // 记录事件
    pub fn add_event(&self, span: &Span, event_name: &str, attributes: HashMap<String, String>);

    // 记录错误
    pub fn record_error(&self, span: &Span, error: &dyn std::error::Error);
}
```

**追踪特性**:
- 自动追踪脚本执行
- 网络请求追踪
- 内存使用追踪
- 性能瓶颈识别

#### 3. StructuredLogger - 结构化日志
```rust
pub struct StructuredLogger {
    logger: Logger,
    json_formatter: JsonFormatter,
    level: LevelFilter,
}

impl StructuredLogger {
    // 记录信息日志
    pub fn info(&self, message: &str, context: HashMap<String, Value>);

    // 记录警告日志
    pub fn warn(&self, message: &str, context: HashMap<String, Value>);

    // 记录错误日志
    pub fn error(&self, message: &str, context: HashMap<String, Value>);

    // 记录调试日志
    pub fn debug(&self, message: &str, context: HashMap<String, Value>);
}
```

**日志特性**:
- JSON 格式输出
- 上下文关联（trace_id, span_id）
- 结构化字段支持
- 级别控制

#### 4. CustomMetrics - 自定义指标
```rust
pub struct CustomMetrics {
    runtime_metrics: RuntimeMetrics,
    performance_metrics: PerformanceMetrics,
    business_metrics: BusinessMetrics,
}

pub struct RuntimeMetrics {
    script_execution_time: Histogram<f64>,
    active_scripts: Gauge<u64>,
    memory_usage: Gauge<f64>,
    cpu_usage: Gauge<f64>,
}

pub struct PerformanceMetrics {
    jit_compilation_time: Histogram<f64>,
    gc_pause_time: Histogram<f64>,
    network_latency: Histogram<f64>,
    throughput: Counter<u64>,
}

pub struct BusinessMetrics {
    scripts_loaded: Counter<u64>,
    packages_installed: Counter<u64>,
    hot_reloads: Counter<u64>,
    concurrent_executions: Gauge<u64>,
}
```

#### 5. AlertingSystem - 告警系统
```rust
pub struct AlertingSystem {
    rules: Arc<RwLock<Vec<AlertRule>>>,
    notifiers: Vec<Box<dyn AlertNotifier>>,
}

pub struct AlertRule {
    name: String,
    metric_name: String,
    condition: AlertCondition,
    threshold: f64,
    duration: Duration,
    severity: AlertSeverity,
}

impl AlertingSystem {
    // 添加告警规则
    pub fn add_rule(&self, rule: AlertRule) -> Result<()>;

    // 检查告警条件
    pub async fn check_alerts(&self) -> Result<Vec<Alert>>;

    // 发送告警通知
    pub async fn send_alert(&self, alert: &Alert) -> Result<()>;
}
```

**告警特性**:
- 多级告警（Critical、Warning、Info）
- 阈值告警
- 趋势告警
- 多种通知方式（HTTP、Webhook、Email）

## 📊 关键指标定义

### 运行时指标
- `amberjs_scripts_executed_total`: 脚本执行总数
- `amberjs_script_execution_duration_seconds`: 脚本执行耗时
- `amberjs_active_scripts`: 活跃脚本数
- `amberjs_memory_usage_bytes`: 内存使用量
- `amberjs_cpu_usage_percent`: CPU使用率

### 性能指标
- `amberjs_jit_compilation_duration_seconds`: JIT编译耗时
- `amberjs_gc_pause_duration_seconds`: GC暂停时间
- `amberjs_network_latency_seconds`: 网络延迟
- `amberjs_throughput_bytes_total`: 网络吞吐量

### 业务指标
- `amberjs_packages_loaded_total`: 已加载包数
- `amberjs_hot_reloads_total`: 热重载次数
- `amberjs_concurrent_executions`: 并发执行数
- `amberjs_error_rate_percent`: 错误率

## 🔌 集成设计

### 与 Runtime 集成
```rust
impl Runtime {
    // 启动可观测性系统
    pub async fn start_observability(&mut self, config: ObservabilityConfig) -> Result<()>;

    // 记录脚本执行
    pub fn record_script_execution(&self, duration: Duration, success: bool);

    // 记录内存使用
    pub fn record_memory_usage(&self, bytes: usize);

    // 获取当前指标
    pub fn get_metrics(&self) -> ObservableMetrics;
}
```

### 与现有模块集成
- **concurrent_execution.rs**: 记录并发执行指标
- **jit_optimizer.rs**: 记录JIT编译指标
- **memory/**/*.rs**: 记录内存使用指标
- **network/**/*.rs**: 记录网络指标

## 🔍 追踪流程

### 脚本执行追踪
```
Start Script
  ↓
Create Root Span (script_execution)
  ↓
JIT Compilation
  ↓
Create Child Span (jit_compilation)
  ↓
Memory Allocation
  ↓
Create Child Span (memory_allocation)
  ↓
Script Execution
  ↓
Network I/O (if any)
  ↓
Create Child Span (network_io)
  ↓
Complete Script
  ↓
End Root Span
```

### 追踪数据收集
- **Trace ID**: 全局唯一标识符
- **Span ID**: 每个操作的唯一标识符
- **Parent Span ID**: 父操作标识符
- **Tags**: 关键属性（脚本名、文件路径、执行结果）
- **Logs**: 操作日志
- **Metrics**: 性能指标

## 📈 监控面板

### Prometheus 查询示例
```promql
# 脚本执行成功率
rate(amberjs_scripts_executed_total{status="success"}[5m]) /
rate(amberjs_scripts_executed_total[5m]) * 100

# P95 脚本执行延迟
histogram_quantile(0.95,
  rate(amberjs_script_execution_duration_seconds_bucket[5m])
)

# 内存使用趋势
rate(amberjs_memory_usage_bytes[5m])

# JIT 编译效率
rate(amberjs_jit_compilation_duration_seconds_sum[5m]) /
rate(amberjs_jit_compilation_duration_seconds_count[5m])
```

### Grafana 面板
- **Runtime Overview**: 运行时总览
- **Performance**: 性能指标
- **Scripts**: 脚本执行统计
- **Resources**: 资源使用情况
- **Errors**: 错误分析

## 🚨 告警规则

### 关键告警
```yaml
- alert: HighErrorRate
  expr: rate(amberjs_scripts_executed_total{status="error"}[5m]) > 0.1
  for: 2m
  labels:
    severity: critical
  annotations:
    summary: "错误率过高"
    description: "错误率超过10%"

- alert: HighLatency
  expr: histogram_quantile(0.95, rate(amberjs_script_execution_duration_seconds_bucket[5m])) > 1
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "延迟过高"
    description: "P95延迟超过1秒"

- alert: HighMemoryUsage
  expr: amberjs_memory_usage_bytes > 1073741824  # 1GB
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "内存使用过高"
    description: "内存使用超过1GB"
```

## 🔐 安全考虑

### 数据保护
- 敏感信息脱敏（密码、API Key）
- 日志访问控制
- 指标数据加密
- 追踪数据过期策略

### 访问控制
- 监控数据访问权限
- 告警规则管理权限
- 日志查看权限

## 📦 依赖项

### 新增 Crate
```toml
# Prometheus 指标
prometheus = "0.13"
opentelemetry = "0.21"
opentelemetry-prometheus = "0.14"
opentelemetry-jaeger = "0.20"

# 结构化日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "fmt"] }
tracing-appender = "0.2"

# 告警系统
reqwest = { version = "0.11", features = ["json"] }
serde_yaml = "0.9"
```

## 🧪 测试策略

### 单元测试
- PrometheusExporter 功能测试
- JaegerTracer 功能测试
- StructuredLogger 功能测试
- AlertingSystem 功能测试

### 集成测试
- 端到端指标收集测试
- 分布式追踪流程测试
- 告警触发测试
- 日志聚合测试

### 性能测试
- 指标导出性能测试
- 追踪开销测试
- 日志写入性能测试

## 🎯 成功标准

### 功能完整性
- [ ] Prometheus 指标导出正常工作
- [ ] Jaeger 追踪数据正确生成
- [ ] 结构化日志正确输出
- [ ] 告警规则正确触发

### 性能要求
- [ ] 指标导出开销 < 5%
- [ ] 追踪开销 < 3%
- [ ] 日志开销 < 2%
- [ ] 总开销 < 10%

### 可用性要求
- [ ] 监控数据 100% 可用
- [ ] 告警响应时间 < 30s
- [ ] 故障恢复时间 < 1min

---

**设计创建时间**: 2025-12-19
**预期实现**: Stage 30.5 完整实现
**版本**: v0.1.0 (Stage 30.5)
