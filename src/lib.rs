#![allow(clippy::all)]
// Beejs: Rust + V8 JavaScript/TypeScript runtime
//
// Fact sources for capability claims: Cargo.toml, this file, src/main.rs,
// docs/CURRENT_SCOPE.md, and executable tests. Do not treat historical STAGE_*
// reports as the current product surface.
//
// Default path: MinimalRuntime (src/runtime_minimal.rs) via the `bee` CLI.
// Node/Web compatibility is incremental; AI features behind `feature = "ai"`
// are experimental and must not claim unverified performance multipliers.
use std::collections::HashMap;
use std::sync::Once;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use rusty_v8 as v8;
use std::hash::Hash;
// 模块声明
// Stage 92: AI 原生性能引擎 (enabled via feature "ai")
// Historical `src/ai/*` still does not rustc (std RwLock used with `.await`).
// Default `bee:ai` is `src/nodejs_core/ai.rs` and is compiled without this feature.
// #[cfg(feature = "ai")]
// pub mod ai { ... }

#[cfg(feature = "benchmarks")]
pub mod benchmarks;
#[cfg(feature = "benchmarks")]
pub mod performance_analyzer;
#[cfg(feature = "benchmarks")]
pub mod performance_comparison;
#[cfg(feature = "benchmarks")]
pub mod performance_regression;
#[cfg(feature = "benchmarks")]
pub mod performance_reporter;

// pub mod automation;  // Temporarily disabled - depends on missing types
// pub mod analysis;  // Temporarily disabled - compilation issues

// #[cfg(feature = "observability")]
// pub mod monitor;

pub mod agent; // Tool export, JSON-RPC session, MCP stdio
pub mod event_loop;
pub mod isolate_prewarmer; // Modern Isolate pre-warming and pool system
pub mod mcp; // Official Model Context Protocol (MCP) Server & Client
pub mod nodejs_core; // Core Node.js compatible modules (fs, path, crypto, http, net, timers)
pub mod package_manager; // Package manager
pub mod permissions;
pub mod runtime; // Split runtime surface (re-exports MinimalRuntime)
pub mod runtime_minimal; // Minimal runtime for basic JavaScript execution
pub mod v8_snapshot; // V8 snapshot and warmup system
pub mod watcher; // File watcher for hot reload
pub mod watcher_websocket; // WebSocket hot reload
pub mod web_api; // Web standards API (Streams, Blob, Worker, Crypto)

// Historical observability/monitor trees do not rustc (serde_json::Value / `_` fields).
// #[cfg(feature = "observability")]
// pub mod observability;

pub mod ecosystem_lite;

#[cfg(feature = "enterprise")]
pub mod security;

// Historical `aiops` / `ai_inference` trees are not compiled with `feature = "ai"`.
// They still fail rustc (`TokioDuration` aliases, `_` struct fields). Default
// `bee:ai` lives in `src/nodejs_core/ai.rs` and does not need these modules.
// #[cfg(feature = "ai")]
// pub mod aiops;
// #[cfg(feature = "ai")]
// pub mod ai_inference;

#[cfg(feature = "multilang")]
pub mod multilang;

#[cfg(feature = "multilang")]
pub mod platform;

#[cfg(feature = "cloudnative")]
pub mod cloud_native;

pub mod ai_engine; // Native text embeddings and semantic vector engine
pub mod bus; // Multi-Agent Message Bus & PubSub Channel Fabric (bee:bus)
pub mod capability; // Enterprise capability-based security (bee:security / bee:permissions)
pub mod checkpoint; // Agent State Checkpoint & Time-Travel Snapshot Engine (bee:checkpoint)
pub mod database; // Embedded SQLite and Vector similarity database
pub mod error; // Unified error handling system
pub mod fallback; // Fallback mechanisms
pub mod ffi; // Native zero-dependency C ABI foreign function interface (bee:ffi)
pub mod grammar; // Streaming Structured JSON & LLM Token Grammar Engine (bee:grammar)
pub mod kv; // Embedded Persistent Key-Value & Durable State Engine (bee:kv)
pub mod memory; // Memory management
pub mod napi; // Minimal N-API host for process.dlopen (hello addons)
pub mod pool; // High-density multi-tenant IsolatePool for serverless & multi-agent execution (bee:pool)
pub mod repl; // Enhanced interactive REPL
pub mod replay; // Deterministic Agent Replay engine (bee:replay)
pub mod sandbox; // In-memory deterministic Virtual Filesystem (VFS) sandbox
pub mod sockets; // WinterTC Sockets API native engine (bee:sockets)
pub mod std_lib; // Modern standard library (dotenv, cli, fs, crypto, assert)
pub mod task_runner; // Task runner for package.json scripts
pub mod testing; // Testing framework support
pub mod tooling; // Developer tooling (formatter, linter, benchmark, compiler, coverage, profiler)
pub mod tools; // Agent Tool Auto-Synthesis & OpenAPI Schema Compiler (bee:tools)
pub mod types_export; // TypeScript types exporter
pub mod typescript;
pub mod wasm; // WebAssembly 2.0 zero-copy shared memory bridge (bee:wasm)
pub mod weights; // Native GGUF and SafeTensors model weights loader (bee:weights)

pub use repl::{Repl, ReplConfig};
// pub use wasm_integration::{initialize_wasm, check_wasm_support};
// pub use io::{DmaEngine, DmaBuffer, DmaDirection, MemoryMapper, MappedFile, MapOptions, MemoryAdvice};

// Define OptimizeMode here since it's used by multiple modules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptimizeMode {
    Speed,
    Size,
    Auto,
}
// 重新导出主要类型
// pub use benchmarks::{
//     BenchmarkFramework, BenchmarkResult, MetricType, BenchmarkConfig,
//     startup::StartupBenchmark,
//     execution::ExecutionBenchmark,
//     memory::MemoryBenchmark,
//     concurrent::ConcurrentBenchmark,
// };
// pub use performance_comparison::{
//     BenchmarkRunner, RuntimeConfig, TestCase,
//     ResultCollector, ComparisonResult, BenchmarkComparison,
//     ReportGenerator as ComparisonReportGenerator, ReportFormat as ComparisonReportFormat, ReportConfig,
//     PerformanceComparisonResult, PerformanceSummary,
// };
// pub use performance_regression::{
//     PerformanceRegressionDetector, RegressionTestSuite, RegressionDetectionResult,
//     PerformanceThresholds, PerformanceBaseline,
// };
// pub use automation::{  // Temporarily disabled - compilation issues
//     test_runner::{AutomatedTestRunner, TestSuiteResults, TestType, TestPlanConfig},
//     threshold::{ThresholdManager, ThresholdConfig},
//     report_generator::{ReportGenerator, ReportFormat, ReportOutput, ReportType},
// };
// 别名
// pub type TestRunner = AutomatedTestRunner;
// pub type TestRunnerConfig = TestPlanConfig;
// pub use monitor::{  // Temporarily disabled - compilation issues
//     // 性能监控器
//     PerformanceMonitor, MonitorConfig, MetricValue, AggregatedMetric,
//     CollectionStats, ThresholdViolation, ThresholdSeverity,
//     // 数据存储
//     DataStore, DataStoreConfig, DataPoint, QueryCondition, ExportFormat,
//     CompressedData, QueryIndex, DataStoreStats,
//     // 告警系统
//     AlertRule, AlertCondition, AlertSeverity, AlertInstance, AlertData,
//     AlertStatus, NotificationChannel, NotificationType, NotificationMessage,
//     AlertStats, AlertSystem, AlertSystemConfig, SilenceRule, NotificationResult,
//     // Web 仪表板
//     DashboardConfig, ChartConfig, DashboardLayout, LayoutConfig,
//     BreakpointConfig, WebDashboard, ConnectionStats, DashboardData, ApiResponse,
//     ExportConfig, ChartData, Dataset,
// };
// 重新导出监控相关的 MetricType，避免与 benchmarks 中的冲突
// pub use monitor::MetricType as MonitorMetricType;  // Temporarily disabled
// pub use monitor::ThresholdConfig as MonitorThresholdConfig;  // Temporarily disabled
// 重新导出可观测性相关类型
// pub use observability::{  // Temporarily disabled
//     ObservableSystem, ObservabilityConfig,
//     // PrometheusExporter, StructuredLogger, CustomMetrics,
//     // AlertingSystem, JaegerTracer,
// };
// 重新导出包管理器相关类型
// pub use package_manager::{  // Temporarily disabled
//     PackageManager, PackageManagerConfig, PackageJson, PackageInfo, PackageVersion,
//     PackageDist, Repository, ResolutionResult,
// };
// 重新导出热重载器相关类型
// pub use watcher::{  // Temporarily disabled
//     HotReloader, WatcherConfig, WatcherStats, WatcherStatsSummary, WatcherConfigBuilder,
//     FileChange, FileChangeType,
// };
// 重新导出并发执行相关类型
// pub use concurrent_execution::{  // Temporarily disabled
//     WorkStealingScheduler, Task, TaskResult, StealStats,
//     ConcurrentConfig, ConcurrentExecutionStats, ScriptResult,
//     ConcurrentExecutionError, BatchExecutor, ConcurrentRuntimePool,
// };
// 重新导出内存共享相关类型
// pub use shared_memory::{  // Temporarily disabled
//     SharedMemoryManager, SharedMemoryConfig, SharedMemoryRegion,
//     SharedMemoryHandle, SharedMemoryStats,
// };
// 重新导出网络相关类型
// pub use network::{  // Temporarily disabled
//     // NetworkBufferPool, ConnectionPool, NetworkIoStatistics,
//     NetworkConfig, NetworkError,
// };
// 重新导出进程池相关类型
// pub use process_pool::{  // Temporarily disabled
//     ProcessPoolConfig, WorkerMetrics, TaskComplexity, ProcessPoolStats, ProcessPool,
// };
// 重新导出预热相关类型
pub use isolate_prewarmer::{global_prewarmer, IsolatePrewarmer, PrewarmConfig, PrewarmStats};
// 重新导出预编译缓存类型
// pub use precompiled_cache::PrecompiledModuleCache;  // Moved to startup module
// 重新导出运行时最小版
pub use runtime_minimal::MinimalRuntime;
// 重新导出 V8 简单运行时
// pub use lib_minimal::Runtime;
// 重新导出 AI 批处理相关类型 (temporarily disabled)
// pub use ai::ai_batch_processor::{
//     AiBatchProcessor, BatchConfig,
//     AiTaskType, AiTaskResult,
// };
// 重新导出 AI 内存池相关类型 (temporarily disabled)
// pub use ai::ai_memory_pool::{
//     AiMemoryPool, ModelMemoryConfig, create_llm_memory_pool,
// };
// 重新导出云原生集成相关类型
// pub use cloud_native::{  // Temporarily disabled
//     // Kubernetes CRDs
//     BeejsCluster, BeejsClusterSpec, BeejsWorkload, BeejsWorkloadSpec,
//     ClusterPhase, Condition, ConditionStatus, ConditionType,
//     DistributedConfig, HPAConfig, MonitoringConfig, NetworkPolicyConfig,
//     PodAffinity, PodAntiAffinity, PreferredSchedulingTerm,
//     ResourceRequirements, RetryConfig, SecurityConfig, SecurityContext,
//     ServiceDiscoveryConfig, ServiceMonitorConfig, Toleration, WorkloadPhase,
//     // Container
//     MultiStageBuilder, BuilderStage, RuntimeStage, Optimization,
//     SecurityScanner, ContainerImage, ImageLayer, Vulnerability,
//     VulnerabilitySeverity, ComplianceIssue, ComplianceSeverity,
//     Secret, ScanReport, ScanConfig, Optimizer, OptimizationStrategy,
//     OptimizationSuggestion, ImpactLevel, LayerMinimizationStrategy,
//     BaseImageOptimizationStrategy, CacheOptimizationStrategy,
//     SecurityHardeningStrategy, SizeOptimizationStrategy,
//     DockerfileError, SecurityError,
//     // Service Mesh
//     IstioConfigManager, IstioConfig, IstioService, TrafficPolicyConfig,
//     LoadBalancerAlgorithm, ConnectionPoolConfig, OutlierDetectionConfig,
//     TrafficManager, FaultType, TrafficSplit, DistributedTracer,
//     TraceContext, SpanRecord, SpanStatus, SpanEvent, PerformanceAnalysis,
//     MetricsCollector, RequestMetrics, LatencyMetrics, ErrorMetrics,
//     MetricsReport, IstioError,
//     // CI/CD
//     GitOpsManager, ArgoCDApplication, FluxHelmRelease, PipelineManager,
//     GitHubActionsWorkflow, GitLabCIPipeline, JenkinsPipeline,
//     DeploymentStrategy, BlueGreenDeployment, CanaryDeployment,
//     RollingDeployment, PipelineStage, PipelineStatus, PipelineEvent,
//     GitOpsConfig, PipelineConfig, DeploymentConfig, DeploymentStatus,
//     CICDError,
// };
// 重新导出智能进程池相关类型
// pub use stage_38_smart_process_pool::{  // Temporarily disabled
//     SmartProcessPool, SmartWarmupStrategy, TaskPattern, SmartLoadBalancer,
//     MemorySharingManager, PerformancePredictor,
//     LoadBalancingStrategy, MemoryPoolConfig, PerformanceEvent, ScaleOperation,
//     GlobalPerformanceStats, WorkerPerformanceRecord, TaskExecutionRecord,
//     TaskPrediction, PerformanceBottleneckPrediction, LinearRegressionModel,
//     PerformanceDataPoint,
// };
// 测试套件类型
#[derive(Debug, Clone)]
pub struct TestSuite {
    pub name: String,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
    pub file: Option<String>,
    pub total_duration: Option<std::time::Duration>,
}
/// HTTP 服务器类型（占位符）
#[derive(Debug, Clone)]
pub struct Server {
    port: u16,
}
impl Server {
    /// 创建新的服务器实例
    pub fn new(port: u16) -> Self {
        Self { port }
    }
    /// 获取端口号
    pub fn port(&self) -> u16 {
        self.port
    }
}
// use tracing::{debug, info, warn, error}; // Unused
// 核心运行时
use anyhow::{anyhow, Result};
/// Global result to track V8 initialization state.
static V8_INIT_RESULT: std::sync::OnceLock<std::result::Result<(), String>> =
    std::sync::OnceLock::new();
/// Initialize V8 engine (idempotent - safe to call multiple times)
pub fn initialize_v8() -> Result<()> {
    let init_result = V8_INIT_RESULT.get_or_init(|| {
        use rusty_v8 as v8;
        // Stage 92: V8 初始化优化 - 使用高性能运行时配置
        // 参考 Bun 和 Node.js 的优化策略
        let mut v8_flags: Vec<String> = vec![
            // JIT 编译器优化（生产环境高性能配置）
            "--opt".to_string(),                      // 启用 TurboFan JIT 优化
            "--turbo-inlining".to_string(),           // 深度函数内联优化
            "--concurrent-recompilation".to_string(), // 启用后台多线程 JIT 编译
            "--max-old-space-size=4096".to_string(),  // 4GB 老生代堆
            "--min-semi-space-size=16".to_string(), // 16MB 初始新生代空间，避免数组与对象微基准的频繁 GC 抖动
            "--max-semi-space-size=64".to_string(), // 64MB 最大新生代空间
            "--turbo-fast-api-calls".to_string(),   // 快速 C++/Rust API 调用
            "--harmony".to_string(),                // ECMAScript 高性能特性
        ];
        if let Ok(extra) = std::env::var("V8_FLAGS") {
            v8_flags.push(extra);
        }
        if let Ok(extra) = std::env::var("BEE_V8_FLAGS") {
            v8_flags.push(extra);
        }
        let v8_flags_str: _ = v8_flags.join(" ");
        v8::V8::set_flags_from_string(&v8_flags_str);
        // Create platform
        let platform = v8::new_default_platform(0, false).make_shared();
        // Initialize V8
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
        Ok(())
    });

    init_result
        .as_ref()
        .map(|_| ())
        .map_err(|message| anyhow!(message.clone()))
}
/// Check if V8 is initialized
pub fn is_v8_initialized() -> bool {
    matches!(V8_INIT_RESULT.get(), Some(Ok(())))
}
/// Check if V8 is available for use in tests
/// Returns true if V8 can be safely initialized, false if already poisoned
pub fn is_v8_available() -> bool {
    static CHECK: Once = Once::new();
    static mut AVAILABLE: bool = true;
    CHECK.call_once(|| {
        // Try to initialize V8 if not already done
        if let Err(_) = initialize_v8() {
            unsafe {
                AVAILABLE = false;
            }
        }
    });
    unsafe { AVAILABLE }
}
/// 性能配置
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub pool_size: usize,
    pub max_memory: usize,
    pub enable_optimization: bool,
    pub performance_monitoring: bool,
}
impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            pool_size: num_cpus::get(),
            max_memory: 1024 * 1024 * 1024, // 1GB
            enable_optimization: true,
            performance_monitoring: true,
        }
    }
}
/// 主要的 Beejs 运行时
pub struct Runtime {
    config: PerformanceConfig,
    _verbose: bool,
    /// 持久化的运行时实例，用于保持模块缓存
    lite_runtime: std::cell::RefCell<Option<MinimalRuntime>>,
}
impl Runtime {
    /// 创建新的运行时实例
    pub fn new(
        pool_size: usize,
        max_memory: usize,
        enable_optimization: bool,
        verbose: bool,
    ) -> Self {
        Self {
            config: PerformanceConfig {
                pool_size,
                max_memory,
                enable_optimization,
                performance_monitoring: true,
            },
            _verbose: verbose,
            lite_runtime: std::cell::RefCell::new(None),
        }
    }
    /// 创建默认配置的运行时
    pub fn new_default() -> Self {
        Self::new(num_cpus::get(), 1024 * 1024 * 1024, true, false)
    }
    /// 创建带优化配置的运行时
    pub fn new_with_optimization(
        pool_size: usize,
        max_memory: usize,
        optimize_mode: OptimizeMode,
        verbose: bool,
    ) -> Self {
        let enable_optimization: _ = match optimize_mode {
            OptimizeMode::Speed => true,
            OptimizeMode::Size => false,
            OptimizeMode::Auto => true,
        };
        Self::new(pool_size, max_memory, enable_optimization, verbose)
    }
    /// 运行基准测试 (Temporarily disabled)
    pub fn run_benchmarks(&self) -> Vec<()> {
        // Temporarily disabled - benchmarks module disabled
        vec![]
    }
    /// 获取性能配置
    pub fn get_config(&self) -> &PerformanceConfig {
        &self.config
    }
    /// 为错误处理提供上下文
    pub fn context(self, _msg: &str) -> Result<Self, anyhow::Error> {
        Ok(self)
    }
    /// 执行 JavaScript 代码（复用运行时实例以保持模块缓存）
    pub fn execute_code(&self, code: &str) -> Result<String> {
        // 获取或创建持久化的运行时实例
        let mut runtime_ref = self.lite_runtime.borrow_mut();
        let runtime = runtime_ref.get_or_insert_with(|| {
            crate::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime")
        });
        runtime.execute_code(code)
    }
    /// 执行 JavaScript 文件（复用运行时实例以支持循环依赖）
    pub fn execute_file(&self, path: &std::path::Path) -> Result<String> {
        let code: _ = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read file {}: {}", path.display(), e))?;

        // Get the directory and file path for __dirname and __filename
        let dir_path = path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("."));
        let file_path = path.to_string_lossy().to_string();

        // Wrap code to set __dirname and __filename correctly
        let wrapped_code = format!(
            "(function() {{ globalThis.__dirname = '{}'; globalThis.__filename = '{}'; }})();\n{}",
            dir_path.replace("\\", "\\\\"),
            file_path.replace("\\", "\\\\"),
            code
        );

        let mut runtime_ref = self.lite_runtime.borrow_mut();
        let runtime = runtime_ref.get_or_insert_with(|| {
            crate::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime")
        });
        runtime.set_main_module_path(path);
        runtime.execute_code(&wrapped_code)
    }
}
/// 获取智能运行时（根据代码特征自动优化）
pub fn get_smart_runtime(
    _code: Option<&str>,
    _stack_size: usize,
    max_heap: usize,
    verbose: bool,
    optimize_mode: OptimizeMode,
) -> Result<Runtime> {
    if verbose {
        println!("[beejs] Initializing smart runtime...");
    }
    // TODO: 根据代码特征选择最佳优化策略
    // 目前使用默认实现
    let enable_optimization: _ = match optimize_mode {
        OptimizeMode::Speed => true,
        OptimizeMode::Size => false,
        OptimizeMode::Auto => true,
    };
    Ok(Runtime::new(
        num_cpus::get(),
        max_heap,
        enable_optimization,
        verbose,
    ))
}
/// 获取全局运行时实例
pub fn get_global_runtime(
    stack_size: usize,
    max_heap: usize,
    verbose: bool,
    optimize_mode: OptimizeMode,
) -> Result<Runtime> {
    if verbose {
        println!("[beejs] Initializing global runtime...");
    }
    get_smart_runtime(None, stack_size, max_heap, verbose, optimize_mode)
}
/// 运行完整的性能测试套件 (Temporarily disabled - automation module disabled)
pub fn run_performance_suite() -> Result<(), Box<dyn std::error::Error>> {
    // Temporarily disabled due to automation module compilation issues
    println!("⚠️  Performance suite is temporarily disabled");
    Ok(())
}

/// 生成性能报告 (Temporarily disabled - automation module disabled)
pub fn generate_performance_report(
    _results: &[()],
    format: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    // Temporarily disabled due to automation module compilation issues
    let output_dir = std::path::PathBuf::from("performance_reports");
    std::fs::create_dir_all(&output_dir)?;
    let report_path = output_dir.join(format!("report.{}.txt", format));
    std::fs::write(&report_path, "Report generation temporarily disabled")?;
    Ok(report_path)
}
/// Console callback functions for V8 integration
pub fn console_log_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    // Convert all arguments to strings and print them
    let mut output = String::new();
    for i in 0..args.length() {
        if i > 0 {
            output.push(' ');
        }
        let arg: _ = args.get(i);
        let arg_str: _ = arg
            .to_string(_scope)
            .unwrap_or_else(|| v8::String::new(_scope, "<unknown>").unwrap());
        output.push_str(&arg_str.to_rust_string_lossy(_scope));
    }
    println!("{}", output);
}
fn format_console_args(scope: &mut v8::PinScope, args: &v8::FunctionCallbackArguments) -> String {
    let mut output = String::new();
    for i in 0..args.length() {
        if i > 0 {
            output.push(' ');
        }
        let arg = args.get(i);
        if arg.is_native_error() {
            let stack_key = v8::String::new(scope, "stack").unwrap();
            if let Some(obj) = arg.to_object(scope) {
                if let Some(stack_val) = obj.get(scope, stack_key.into()) {
                    if !stack_val.is_undefined() {
                        output.push_str(&stack_val.to_rust_string_lossy(scope));
                        continue;
                    }
                }
            }
        }
        let arg_str = arg
            .to_string(scope)
            .unwrap_or_else(|| v8::String::new(scope, "<unknown>").unwrap());
        output.push_str(&arg_str.to_rust_string_lossy(scope));
    }
    output
}

pub fn console_error_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    eprintln!("{}", format_console_args(scope, &args));
}

pub fn console_warn_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    eprintln!("{}", format_console_args(scope, &args));
}

pub fn console_info_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    println!("{}", format_console_args(scope, &args));
}

pub fn console_debug_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    println!("{}", format_console_args(scope, &args));
}

/// Console table callback - formats data as a table
/// Supports optional columns parameter for column filtering
pub fn console_table_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() == 0 {
        println!("console.table: no data provided");
        return;
    }

    let data = args.get(0);

    // Handle optional columns parameter (second argument)
    let columns: Vec<String> = if args.length() > 1 {
        let cols_arg = args.get(1);
        if cols_arg.is_array() {
            let cols_arr = v8::Local::<v8::Array>::try_from(cols_arg).unwrap();
            let cols_len = cols_arr.length();
            let mut cols = Vec::new();
            for i in 0..cols_len {
                if let Some(col) = cols_arr.get_index(scope, i) {
                    let col_str = col.to_string(scope).unwrap().to_rust_string_lossy(scope);
                    cols.push(col_str);
                }
            }
            cols
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let use_columns = !columns.is_empty();

    // Handle different data types
    if data.is_null_or_undefined() {
        println!("console.table: null");
        return;
    }

    if data.is_array() {
        // Array of objects
        let arr = v8::Local::<v8::Array>::try_from(data).unwrap();
        let length = arr.length();

        if length == 0 {
            println!("console.table: []");
            return;
        }

        // Check if array contains objects
        let first_item = arr.get_index(scope, 0).unwrap();
        if first_item.is_object() {
            // Print table header
            if use_columns {
                println!("┌─────────────────────────────────┐");
                println!("│         Console Table           │");
                println!("├─────────────────────────────────┤");

                for i in 0..length {
                    if let Some(item) = arr.get_index(scope, i) {
                        let obj = v8::Local::<v8::Object>::try_from(item).unwrap();
                        let mut row = String::new();
                        for (j, col_name) in columns.iter().enumerate() {
                            let key = v8::String::new(scope, col_name).unwrap().into();
                            let value = obj.get(scope, key);
                            let value_str = match value {
                                Some(v) => v.to_string(scope).unwrap().to_rust_string_lossy(scope),
                                None => String::from("undefined"),
                            };
                            row.push_str(&format!("{}: {}", col_name, value_str));
                            if j < columns.len() - 1 {
                                row.push_str(", ");
                            }
                        }
                        println!("│ {}", format!("{:<33}", row));
                    }
                }
                println!("└─────────────────────────────────┘");
            } else {
                // Original behavior - show all keys
                println!("┌─────────────────────────────────┐");
                println!("│         Console Table           │");
                println!("├─────────────────────────────────┤");

                for i in 0..length {
                    if let Some(item) = arr.get_index(scope, i) {
                        let obj = v8::Local::<v8::Object>::try_from(item).unwrap();
                        let keys = obj
                            .get_own_property_names(scope, Default::default())
                            .unwrap();
                        let key_count = keys.length();

                        let mut row = String::new();
                        for j in 0..key_count {
                            let key = keys.get_index(scope, j).unwrap();
                            let key_str = key.to_string(scope).unwrap().to_rust_string_lossy(scope);
                            let value = obj.get(scope, key).unwrap();
                            let value_str =
                                value.to_string(scope).unwrap().to_rust_string_lossy(scope);
                            row.push_str(&format!("{}: {}", key_str, value_str));
                            if j < key_count - 1 {
                                row.push_str(", ");
                            }
                        }
                        println!("│ {}", format!("{:<33}", row));
                    }
                }
                println!("└─────────────────────────────────┘");
            }
        } else {
            // Simple array of primitives
            println!("┌─────────┐");
            println!("│  Index  │  Value");
            println!("├─────────┤");
            for i in 0..length {
                if let Some(item) = arr.get_index(scope, i) {
                    let value_str = item.to_string(scope).unwrap().to_rust_string_lossy(scope);
                    println!("│ {:>6}  │  {}", i, value_str);
                }
            }
            println!("└─────────┘");
        }
    } else if data.is_object() {
        // Plain object - display as key-value pairs
        let obj = v8::Local::<v8::Object>::try_from(data).unwrap();
        let keys = obj
            .get_own_property_names(scope, Default::default())
            .unwrap();
        let length = keys.length();

        println!("┌──────────────────┬───────────────┐");
        println!("│      Key         │     Value     │");
        println!("├──────────────────┼───────────────┤");
        for i in 0..length {
            let key = keys.get_index(scope, i).unwrap();
            let key_str = key.to_string(scope).unwrap().to_rust_string_lossy(scope);

            // Filter by columns if specified
            if use_columns && !columns.contains(&key_str) {
                continue;
            }

            let value = obj.get(scope, key).unwrap();
            let value_str = value.to_string(scope).unwrap().to_rust_string_lossy(scope);
            println!("│ {:<16} │ {:<13} │", key_str, value_str);
        }
        println!("└──────────────────┴───────────────┘");
    } else {
        // Primitive value
        let value_str = data.to_string(scope).unwrap().to_rust_string_lossy(scope);
        println!("console.table: {}", value_str);
    }
}

/// Timer storage for console.time/timeEnd (v0.3.256)
static TIMER_STORAGE: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
/// Get the timer storage, initializing if needed
fn get_timer_storage() -> &'static Mutex<HashMap<String, Instant>> {
    TIMER_STORAGE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Counter storage for console.count/countReset (v0.3.259)
static COUNTER_STORAGE: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
/// Get the counter storage, initializing if needed
fn get_counter_storage() -> &'static Mutex<HashMap<String, u32>> {
    COUNTER_STORAGE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Console time callback - starts a timer
pub fn console_time_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let label = if args.length() > 0 {
        let arg = args.get(0);
        arg.to_string(_scope).unwrap().to_rust_string_lossy(_scope)
    } else {
        "default".to_string()
    };

    // Store the start time
    let storage = get_timer_storage();
    let mut timers = storage.lock().unwrap();
    timers.insert(label.clone(), Instant::now());

    println!("console.time: {}", label);
}

/// Console timeEnd callback - ends a timer and prints elapsed time
pub fn console_time_end_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let label = if args.length() > 0 {
        let arg = args.get(0);
        arg.to_string(_scope).unwrap().to_rust_string_lossy(_scope)
    } else {
        "default".to_string()
    };

    // Calculate elapsed time
    let storage = get_timer_storage();
    let mut timers = storage.lock().unwrap();
    let elapsed = if let Some(start_time) = timers.remove(&label) {
        let duration = start_time.elapsed();
        duration.as_secs_f64() * 1000.0 // Convert to milliseconds
    } else {
        0.0
    };

    println!("console.timeEnd: {}: {:.2}ms", label, elapsed);
}

/// Console count callback - increments and prints a count (v0.3.259)
pub fn console_count_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let label = if args.length() > 0 {
        let arg = args.get(0);
        arg.to_string(_scope).unwrap().to_rust_string_lossy(_scope)
    } else {
        "default".to_string()
    };

    // Increment the counter
    let counter_storage = get_counter_storage();
    let mut counters = counter_storage.lock().unwrap();
    let count = counters.entry(label.clone()).or_insert(0);
    *count += 1;
    println!("console.count: {} {}", label, count);
}

/// Console countReset callback - resets a count (v0.3.259)
pub fn console_count_reset_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let label = if args.length() > 0 {
        let arg = args.get(0);
        arg.to_string(_scope).unwrap().to_rust_string_lossy(_scope)
    } else {
        "default".to_string()
    };

    // Reset the counter to 0
    let counter_storage = get_counter_storage();
    let mut counters = counter_storage.lock().unwrap();
    counters.insert(label.clone(), 0);
    println!("console.countReset: {}", label);
}

/// Console group callback - starts a new group
pub fn console_group_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let label = if args.length() > 0 {
        let arg = args.get(0);
        arg.to_string(_scope).unwrap().to_rust_string_lossy(_scope)
    } else {
        "console.group".to_string()
    };
    println!("▼ {}", label);
}

/// Console groupEnd callback - ends a group
pub fn console_group_end_callback(
    _scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    println!("▲ group ended");
}

/// Console trace callback - prints a stack trace
pub fn console_trace_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let message = if args.length() > 0 {
        let arg = args.get(0);
        arg.to_string(_scope).unwrap().to_rust_string_lossy(_scope)
    } else {
        "console.trace".to_string()
    };
    println!("Trace: {}", message);
    println!("    at <anonymous>");
}

/// Console assert callback - asserts a condition
pub fn console_assert_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() == 0 {
        return;
    }

    let assertion = args.get(0);
    let is_truthy = assertion.is_true();

    if !is_truthy {
        // Assertion failed - print message
        let mut message = "Assertion failed".to_string();
        for i in 1..args.length() {
            let arg = args.get(i);
            let arg_str = arg.to_string(scope).unwrap().to_rust_string_lossy(scope);
            message.push_str(&format!(": {}", arg_str));
        }
        println!("{}", message);
    }
}

/// Console dir callback - prints object representation
pub fn console_dir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() == 0 {
        println!("console.dir: no object provided");
        return;
    }

    let obj = args.get(0);
    if obj.is_object() {
        let obj_local = v8::Local::<v8::Object>::try_from(obj).unwrap();
        let keys = obj_local
            .get_own_property_names(scope, Default::default())
            .unwrap();
        let length = keys.length();

        println!("{{");
        for i in 0..length {
            let key = keys.get_index(scope, i).unwrap();
            let key_str = key.to_string(scope).unwrap().to_rust_string_lossy(scope);
            let value = obj_local.get(scope, key).unwrap();
            let value_str = value.to_string(scope).unwrap().to_rust_string_lossy(scope);
            println!("  {}: {},", key_str, value_str);
        }
        println!("}}");
    } else {
        let value_str = obj.to_string(scope).unwrap().to_rust_string_lossy(scope);
        println!("{}", value_str);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn test_minimal_runtime_creation() {
        let runtime = crate::runtime_minimal::MinimalRuntime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    #[serial_test::serial]
    fn test_minimal_js_execution() {
        let mut runtime = crate::runtime_minimal::MinimalRuntime::new().unwrap();
        let result = runtime.execute_code("1 + 1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "2");
    }

    #[test]
    #[serial_test::serial]
    fn test_minimal_js_function() {
        let mut runtime = crate::runtime_minimal::MinimalRuntime::new().unwrap();
        let result = runtime.execute_code("let x = 5; let y = 10; x + y;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "15");
    }

    #[test]
    fn test_runtime_creation() {
        let runtime: _ = Runtime::new(4, 512 * 1024 * 1024, true, false);
        assert_eq!(runtime.get_config().pool_size, 4);
        assert_eq!(runtime.get_config().max_memory, 512 * 1024 * 1024);
    }

    // #[test]
    // fn test_benchmark_framework() {  // Temporarily disabled
    //     let framework: _ = BenchmarkFramework::new_default();
    //     let result: _ = framework.run_benchmark(
    //         "test",
    //         MetricType::ExecutionTime,
    //         || {
    //             std::thread::sleep(Duration::from_millis(1));
    //             42
    //         },
    //     );
    //     assert_eq!(result.name, "test");
    //     assert_eq!(result.metric_type, MetricType::ExecutionTime);
    //     assert!(result.iterations > 0);
    // }

    // Other tests temporarily disabled due to disabled modules
}
// Stage 56.3: Node.js polyfill
// pub mod nodejs_polyfill;  // Temporarily disabled
