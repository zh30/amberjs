// 基准测试配置系统
//
// 提供灵活的基准测试配置管理，支持：
// - 测试套件配置
// - 基准测试参数
// - 工作负载配置文件
// - 运行时对比配置

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{MetricType, Runtime};
use std::time::Duration;
use std::collections::HashMap;
use std::path::PathBuf;

/// 基准测试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// 测试迭代次数
    pub iterations: u32,
    /// 预热迭代次数
    pub warmup_iterations: u32,
    /// 超时时间
    pub timeout: Duration,
    /// 输出格式
    pub output_format: OutputFormat,
    /// 启用性能分析
    pub enable_profiling: bool,
    /// 并行工作线程数
    pub workers: u32,
    /// 输出目录
    pub output_dir: PathBuf,
    /// 详细程度
    pub verbosity: Verbosity,
    /// 启用详细日志
    pub enable_logging: bool,
    /// 日志文件路径
    pub log_file: Option<PathBuf>,
    /// 基准测试名称
    pub name: String,
    /// 基准测试描述
    pub description: Option<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 分类
    pub category: Option<String>,
}
impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 10,
            warmup_iterations: 3,
            timeout: Duration::from_secs(60),
            output_format: OutputFormat::Json,
            enable_profiling: false,
            workers: num_cpus::get() as u32,
            output_dir: PathBuf::from("benchmark_results"),
            verbosity: Verbosity::Info,
            enable_logging: false,
            log_file: None,
            name: "default".to_string(),
            description: None,
            tags: Vec::new(),
            category: None,
        }
    }
}
impl BenchmarkConfig {
    /// 创建新的基准测试配置
    pub fn new() -> Self {
        Self::default()
    }
    /// 设置迭代次数
    pub fn iterations(mut self, iterations: u32) -> Self {
        self.iterations = iterations;
        self
    }
    /// 设置预热迭代次数
    pub fn warmup_iterations(mut self, warmup_iterations: u32) -> Self {
        self.warmup_iterations = warmup_iterations;
        self
    }
    /// 设置超时时间
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    /// 设置输出格式
    pub fn output_format(mut self, output_format: OutputFormat) -> Self {
        self.output_format = output_format;
        self
    }
    /// 启用性能分析
    pub fn enable_profiling(mut self, enable: bool) -> Self {
        self.enable_profiling = enable;
        self
    }
    /// 设置工作线程数
    pub fn workers(mut self, workers: u32) -> Self {
        self.workers = workers;
        self
    }
    /// 设置输出目录
    pub fn output_dir(mut self, output_dir: PathBuf) -> Self {
        self.output_dir = output_dir;
        self
    }
    /// 设置详细程度
    pub fn verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = verbosity;
        self
    }
    /// 设置基准测试名称
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
    /// 设置描述
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
    /// 添加标签
    pub fn add_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
    /// 设置分类
    pub fn category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }
}
/// 输出格式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutputFormat {
    /// JSON 格式
    Json,
    /// CSV 格式
    Csv,
    /// HTML 格式
    Html,
    /// 纯文本格式
    Text,
    /// Prometheus 格式
    Prometheus,
}
impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Json
    }
}
/// 详细程度
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Verbosity {
    /// 静默模式
    Quiet,
    /// 错误信息
    Error,
    /// 警告信息
    Warn,
    /// 信息 (默认)
    Info,
    /// 调试信息
    Debug,
    /// 详细调试信息
    Trace,
}
impl Default for Verbosity {
    fn default() -> Self {
        Verbosity::Info
    }
}
/// 测试套件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    /// 套件名称
    pub name: String,
    /// 套件描述
    pub description: String,
    /// 基准测试配置
    pub config: BenchmarkConfig,
    /// 包含的基准测试
    pub benchmarks: Vec<BenchmarkTest>,
    /// 包含的工作负载
    pub workloads: Vec<WorkloadProfile>,
    /// 包含的运行时
    pub runtimes: Vec<Runtime>,
    /// 依赖的测试套件
    pub dependencies: Vec<String>,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 设置脚本
    pub setup_script: Option<PathBuf>,
    /// 清理脚本
    pub cleanup_script: Option<PathBuf>,
}
impl TestSuite {
    /// 创建新的测试套件
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            config: BenchmarkConfig::default(),
            benchmarks: Vec::new(),
            workloads: Vec::new(),
            runtimes: vec![Runtime::Amber],
            dependencies: Vec::new(),
            environment: HashMap::new(),
            setup_script: None,
            cleanup_script: None,
        }
    }
    /// 添加基准测试
    pub fn add_benchmark(mut self, benchmark: BenchmarkTest) -> Self {
        self.benchmarks.push(benchmark);
        self
    }
    /// 添加工作负载
    pub fn add_workload(mut self, workload: WorkloadProfile) -> Self {
        self.workloads.push(workload);
        self
    }
    /// 添加运行时
    pub fn add_runtime(mut self, runtime: Runtime) -> Self {
        self.runtimes.push(runtime);
        self
    }
    /// 添加环境变量
    pub fn add_env(mut self, key: &str, value: &str) -> Self {
        self.environment.insert(key.to_string(), value.to_string());
        self
    }
    /// 设置环境变量
    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }
}
/// 基准测试定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTest {
    /// 测试名称
    pub name: String,
    /// 测试描述
    pub description: String,
    /// 测试代码
    pub code: String,
    /// 测试语言
    pub language: TestLanguage,
    /// 预期结果
    pub expected_result: Option<String>,
    /// 迭代次数 (覆盖全局配置)
    pub iterations: Option<u32>,
    /// 超时时间 (覆盖全局配置)
    pub timeout: Option<Duration>,
    /// 标签
    pub tags: Vec<String>,
    /// 分类
    pub category: Option<String>,
    /// 启用/禁用
    pub enabled: bool,
    /// 依赖项
    pub dependencies: Vec<String>,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 设置代码
    pub setup_code: Option<String>,
    /// 清理代码
    pub cleanup_code: Option<String>,
}
impl BenchmarkTest {
    /// 创建新的基准测试
    pub fn new(name: &str, description: &str, code: &str, language: TestLanguage) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            code: code.to_string(),
            language,
            expected_result: None,
            iterations: None,
            timeout: None,
            tags: Vec::new(),
            category: None,
            enabled: true,
            dependencies: Vec::new(),
            environment: HashMap::new(),
            setup_code: None,
            cleanup_code: None,
        }
    }
    /// 设置预期结果
    pub fn expected_result(mut self, result: &str) -> Self {
        self.expected_result = Some(result.to_string());
        self
    }
    /// 设置迭代次数
    pub fn iterations(mut self, iterations: u32) -> Self {
        self.iterations = Some(iterations);
        self
    }
    /// 设置超时时间
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    /// 添加标签
    pub fn add_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
    /// 设置分类
    pub fn category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }
    /// 启用测试
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// 添加依赖
    pub fn add_dependency(mut self, dependency: &str) -> Self {
        self.dependencies.push(dependency.to_string());
        self
    }
    /// 添加环境变量
    pub fn add_env(mut self, key: &str, value: &str) -> Self {
        self.environment.insert(key.to_string(), value.to_string());
        self
    }
    /// 设置环境变量
    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }
    /// 设置设置代码
    pub fn setup_code(mut self, code: &str) -> Self {
        self.setup_code = Some(code.to_string());
        self
    }
    /// 设置清理代码
    pub fn cleanup_code(mut self, code: &str) -> Self {
        self.cleanup_code = Some(code.to_string());
        self
    }
}
/// 测试语言
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TestLanguage {
    /// JavaScript
    JavaScript,
    /// TypeScript
    TypeScript,
    /// Python
    Python,
    /// Rust
    Rust,
}
impl Default for TestLanguage {
    fn default() -> Self {
        TestLanguage::JavaScript
    }
}
impl std::fmt::Display for TestLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestLanguage::JavaScript => write!(f, "javascript"),
            TestLanguage::TypeScript => write!(f, "typescript"),
            TestLanguage::Python => write!(f, "python"),
            TestLanguage::Rust => write!(f, "rust"),
        }
    }
}
/// 工作负载配置文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadProfile {
    /// 工作负载名称
    pub name: String,
    /// 工作负载类型
    pub workload_type: WorkloadType,
    /// 工作负载描述
    pub description: String,
    /// 参数配置
    pub parameters: HashMap<String, serde_json::Value>>,
    /// 资源需求
    pub resource_requirements: ResourceRequirements,
    /// 持续时间
    pub duration: Option<Duration>,
    /// 迭代次数
    pub iterations: Option<u32>,
    /// 并发级别
    pub concurrency: u32,
    /// 标签
    pub tags: Vec<String>,
    /// 分类
    pub category: Option<String>,
    /// 启用/禁用
    pub enabled: bool,
}
impl WorkloadProfile {
    /// 创建新的工作负载配置
    pub fn new(name: &str, workload_type: WorkloadType, description: &str) -> Self {
        Self {
            name: name.to_string(),
            workload_type,
            description: description.to_string(),
            parameters: HashMap::new(),
            resource_requirements: ResourceRequirements::default(),
            duration: None,
            iterations: None,
            concurrency: 1,
            tags: Vec::new(),
            category: None,
            enabled: true,
        }
    }
    /// 添加参数
    pub fn add_parameter(mut self, key: &str, value: serde_json::Value) -> Self {
        self.parameters.insert(key.to_string(), value);
        self
    }
    /// 设置参数
    pub fn parameters(mut self, parameters: HashMap<String, serde_json::Value>) -> Self {
        self.parameters = parameters;
        self
    }
    /// 设置资源需求
    pub fn resource_requirements(mut self, requirements: ResourceRequirements) -> Self {
        self.resource_requirements = requirements;
        self
    }
    /// 设置持续时间
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }
    /// 设置迭代次数
    pub fn iterations(mut self, iterations: u32) -> Self {
        self.iterations = Some(iterations);
        self
    }
    /// 设置并发级别
    pub fn concurrency(mut self, concurrency: u32) -> Self {
        self.concurrency = concurrency;
        self
    }
    /// 添加标签
    pub fn add_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
    /// 设置分类
    pub fn category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }
    /// 启用工作负载
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}
/// 工作负载类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkloadType {
    /// 计算密集型
    ComputeIntensive,
    /// I/O 密集型
    IoIntensive,
    /// 内存密集型
    MemoryIntensive,
    /// 并发型
    Concurrent,
    /// AI 工作负载
    AiWorkload,
    /// 混合型
    Mixed,
}
impl std::fmt::Display for WorkloadType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkloadType::ComputeIntensive => write!(f, "compute_intensive"),
            WorkloadType::IoIntensive => write!(f, "io_intensive"),
            WorkloadType::MemoryIntensive => write!(f, "memory_intensive"),
            WorkloadType::Concurrent => write!(f, "concurrent"),
            WorkloadType::AiWorkload => write!(f, "ai_workload"),
            WorkloadType::Mixed => write!(f, "mixed"),
        }
    }
}
/// 资源需求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// 最小内存 (字节)
    pub min_memory: Option<u64>,
    /// 最大内存 (字节)
    pub max_memory: Option<u64>,
    /// CPU 核心数
    pub cpu_cores: Option<u32>,
    /// 网络带宽 (字节/秒)
    pub network_bandwidth: Option<u64>,
    /// 磁盘 I/O (字节/秒)
    pub disk_io: Option<u64>,
    /// GPU 要求
    pub gpu_requirements: Option<GpuRequirements>,
}
impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            min_memory: None,
            max_memory: None,
            cpu_cores: None,
            network_bandwidth: None,
            disk_io: None,
            gpu_requirements: None,
        }
    }
}
impl ResourceRequirements {
    /// 创建新的资源需求
    pub fn new() -> Self {
        Self::default()
    }
    /// 设置最小内存
    pub fn min_memory(mut self, memory: u64) -> Self {
        self.min_memory = Some(memory);
        self
    }
    /// 设置最大内存
    pub fn max_memory(mut self, memory: u64) -> Self {
        self.max_memory = Some(memory);
        self
    }
    /// 设置 CPU 核心数
    pub fn cpu_cores(mut self, cores: u32) -> Self {
        self.cpu_cores = Some(cores);
        self
    }
    /// 设置网络带宽
    pub fn network_bandwidth(mut self, bandwidth: u64) -> Self {
        self.network_bandwidth = Some(bandwidth);
        self
    }
    /// 设置磁盘 I/O
    pub fn disk_io(mut self, io: u64) -> Self {
        self.disk_io = Some(io);
        self
    }
    /// 设置 GPU 要求
    pub fn gpu_requirements(mut self, requirements: GpuRequirements) -> Self {
        self.gpu_requirements = Some(requirements);
        self
    }
}
/// GPU 要求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// 最小显存 (字节)
    pub min_vram: Option<u64>,
    /// 计算能力 (CUDA Core 数量)
    pub compute_capability: Option<u32>,
    /// 必需的 GPU 数量
    pub gpu_count: Option<u32>,
}
impl GpuRequirements {
    /// 创建新的 GPU 要求
    pub fn new() -> Self {
        Self {
            min_vram: None,
            compute_capability: None,
            gpu_count: None,
        }
    }
    /// 设置最小显存
    pub fn min_vram(mut self, vram: u64) -> Self {
        self.min_vram = Some(vram);
        self
    }
    /// 设置计算能力
    pub fn compute_capability(mut self, capability: u32) -> Self {
        self.compute_capability = Some(capability);
        self
    }
    /// 设置 GPU 数量
    pub fn gpu_count(mut self, count: u32) -> Self {
        self.gpu_count = Some(count);
        self
    }
}
/// 运行时对比配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeComparison {
    /// 对比名称
    pub name: String,
    /// 对比描述
    pub description: String,
    /// 基准运行时 (Amber)
    pub baseline_runtime: Runtime,
    /// 对比运行时列表
    pub comparison_runtimes: Vec<Runtime>,
    /// 包含的基准测试
    pub benchmarks: Vec<String>,
    /// 包含的工作负载
    pub workloads: Vec<String>,
    /// 对比模式
    pub comparison_mode: ComparisonMode,
    /// 统计显著性水平
    pub significance_level: f64,
    /// 最小样本数
    pub min_samples: u32,
    /// 自动检测运行时
    pub auto_detect_runtimes: bool,
    /// 生成报告
    pub generate_report: bool,
    /// 报告格式
    pub report_format: OutputFormat,
    /// 保存历史数据
    pub save_history: bool,
    /// 历史数据路径
    pub history_path: Option<PathBuf>,
}
impl Default for RuntimeComparison {
    fn default() -> Self {
        Self {
            name: "default_comparison".to_string(),
            description: "Default runtime comparison".to_string(),
            baseline_runtime: Runtime::Amber,
            comparison_runtimes: vec![Runtime::NodeJs, Runtime::Bun],
            benchmarks: Vec::new(),
            workloads: Vec::new(),
            comparison_mode: ComparisonMode::Statistical,
            significance_level: 0.05,
            min_samples: 10,
            auto_detect_runtimes: true,
            generate_report: true,
            report_format: OutputFormat::Html,
            save_history: true,
            history_path: Some(PathBuf::from("benchmark_history")),
        }
    }
}
impl RuntimeComparison {
    /// 创建新的运行时对比
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            ..Self::default()
        }
    }
    /// 设置基准运行时
    pub fn baseline_runtime(mut self, runtime: Runtime) -> Self {
        self.baseline_runtime = runtime;
        self
    }
    /// 添加对比运行时
    pub fn add_comparison_runtime(mut self, runtime: Runtime) -> Self {
        self.comparison_runtimes.push(runtime);
        self
    }
    /// 添加基准测试
    pub fn add_benchmark(mut self, benchmark: &str) -> Self {
        self.benchmarks.push(benchmark.to_string());
        self
    }
    /// 添加工作负载
    pub fn add_workload(mut self, workload: &str) -> Self {
        self.workloads.push(workload.to_string());
        self
    }
    /// 设置对比模式
    pub fn comparison_mode(mut self, mode: ComparisonMode) -> Self {
        self.comparison_mode = mode;
        self
    }
    /// 设置统计显著性水平
    pub fn significance_level(mut self, level: f64) -> Self {
        self.significance_level = level;
        self
    }
    /// 设置最小样本数
    pub fn min_samples(mut self, samples: u32) -> Self {
        self.min_samples = samples;
        self
    }
    /// 启用自动检测运行时
    pub fn auto_detect_runtimes(mut self, auto_detect: bool) -> Self {
        self.auto_detect_runtimes = auto_detect;
        self
    }
    /// 生成报告
    pub fn generate_report(mut self, generate: bool) -> Self {
        self.generate_report = generate;
        self
    }
    /// 设置报告格式
    pub fn report_format(mut self, format: OutputFormat) -> Self {
        self.report_format = format;
        self
    }
    /// 保存历史数据
    pub fn save_history(mut self, save: bool) -> Self {
        self.save_history = save;
        self
    }
    /// 设置历史数据路径
    pub fn history_path(mut self, path: PathBuf) -> Self {
        self.history_path = Some(path);
        self
    }
}
/// 对比模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonMode {
    /// 统计对比
    Statistical,
    /// 绝对对比
    Absolute,
    /// 相对对比
    Relative,
    /// 百分比对比
    Percentage,
}
impl Default for ComparisonMode {
    fn default() -> Self {
        ComparisonMode::Statistical
    }
}