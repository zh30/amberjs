// Stage 93 Phase 5: 性能基准测试套件
//
// 这个模块提供全面的性能基准测试系统，支持：
// - 多种工作负载的性能测试
// - 与 Bun、Node.js 等运行时的对比
// - 性能回归检测
/// - 实时性能监控
use std::hash::Hash;
pub mod engine;
pub mod config;
pub mod result;
pub mod runtime_comparison;
pub mod workloads;
pub mod regression;
pub mod monitoring;
pub mod utils;

use config::{BenchmarkConfig, RuntimeComparison, TestSuite, WorkloadProfile};
use engine::{BenchmarkEngine, BenchmarkRun};
use monitoring::{MetricsCollector, PerformanceDashboard, RealTimeMonitor};
use regression::{PerformanceHistory, RegressionDetector, RegressionReport};
use result::{BenchmarkResult, PerformanceMetrics, Statistics};
use runtime_comparison::{ComparisonReport, ProcessLauncher, RuntimeDetector};
use std::collections::{BTreeMap, HashMap};
use workloads::{AIWorkload, ComputeWorkload, ConcurrentWorkload, IOWorkload, MemoryWorkload, WorkloadExecutor, WorkloadType};

/// 运行时类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Runtime {
    /// Amber 运行时
    Amber,
    /// Node.js 运行时
    NodeJs,
    /// Bun 运行时
    Bun,
    /// Deno 运行时
    Deno,
    /// 自定义运行时
    Custom(String),
}
impl std::fmt::Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Runtime::Amber => write!(f, "amber"),
            Runtime::NodeJs => write!(f, "node"),
            Runtime::Bun => write!(f, "bun"),
            Runtime::Deno => write!(f, "deno"),
            Runtime::Custom(name) => write!(f, "{}", name),
        }
    }
}
/// 性能指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricType {
    /// 执行时间 (毫秒)
    ExecutionTime,
    /// 内存使用 (字节)
    MemoryUsage,
    /// CPU 使用率 (百分比)
    CpuUsage,
    /// 吞吐量 (操作/秒)
    Throughput,
    /// 延迟 (毫秒)
    Latency,
}
impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::ExecutionTime => write!(f, "execution_time"),
            MetricType::MemoryUsage => write!(f, "memory_usage"),
            MetricType::CpuUsage => write!(f, "cpu_usage"),
            MetricType::Throughput => write!(f, "throughput"),
            MetricType::Latency => write!(f, "latency"),
        }
    }
}
/// 基准测试错误类型
#[derive(thiserror::Error, Debug)]
pub enum BenchmarkError {
    #[error("Runtime not available: {0}")]
    RuntimeNotAvailable(String),
    #[error("Test execution failed: {0}")]
    TestExecutionFailed(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Statistics calculation error: {0}")]
    StatisticsError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
/// 基准测试成功结果
pub type BenchmarkResult<T = ()> = std::result::Result<T, BenchmarkError>;
