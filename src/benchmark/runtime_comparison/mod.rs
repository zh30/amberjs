// 运行时对比模块
//
// 提供与 Bun、Node.js 等运行时的性能对比功能

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use super::{BenchmarkError, BenchmarkResult, BenchmarkResult as Result, Runtime};
use std::time::{Duration, Instant};
use std::path::PathBuf;

/// 运行时检测器
#[derive(Debug)]
pub struct RuntimeDetector {
    /// 检测到的运行时
    detected_runtimes: HashMap<Runtime, bool>,
}
impl RuntimeDetector {
    /// 创建新的运行时检测器
    pub fn new() -> Self {
        let mut detector = Self {
            detected_runtimes: HashMap::new(),
        };
        detector.detect_all_runtimes();
        detector
    }
    /// 检测所有运行时
    pub fn detect_all_runtimes(&mut self) {
        // 检测 Amber
        self.detected_runtimes.insert(
            Runtime::Amber,
            check_command_available("amber")
        );
        // 检测 Node.js
        self.detected_runtimes.insert(
            Runtime::NodeJs,
            check_command_available("node")
        );
        // 检测 Bun
        self.detected_runtimes.insert(
            Runtime::Bun,
            check_command_available("bun")
        );
        // 检测 Deno
        self.detected_runtimes.insert(
            Runtime::Deno,
            check_command_available("deno")
        );
    }
    /// 检查特定运行时是否可用
    pub fn is_available(&self, runtime: Runtime) -> bool {
        self.detected_runtimes
            .get(&runtime)
            .cloned()
            .unwrap_or(false)
    }
    /// 获取所有可用的运行时
    pub fn get_available_runtimes(&self) -> Vec<Runtime> {
        self.detected_runtimes
            .iter()
            .filter_map(|(runtime, &available)| {
                if available {
                    Some(*runtime)
                } else {
                    None
                }
            })
            .collect()
    }
    /// 获取运行时版本
    pub fn get_version(&self, runtime: Runtime) -> Option<String> {
        match runtime {
            Runtime::Amber => get_command_version("amber"),
            Runtime::NodeJs => get_command_version("node"),
            Runtime::Bun => get_command_version("bun"),
            Runtime::Deno => get_command_version("deno"),
            Runtime::Custom(_) => None,
        }
    }
}
/// 进程启动器
#[derive(Debug)]
pub struct ProcessLauncher {
    /// 启动配置
    config: ProcessConfig,
}
impl ProcessLauncher {
    /// 创建新的进程启动器
    pub fn new(config: ProcessConfig) -> Self {
        Self { config }
    }
    /// 启动进程
    pub async fn launch(&self, code: &str, runtime: Runtime) -> Result<ProcessOutput> {
        let start_time: _ = tokio::time::Instant::now();
        match runtime {
            Runtime::Amber => {
                self.launch_amberjs(code).await
            }
            Runtime::NodeJs => {
                self.launch_nodejs(code).await
            }
            Runtime::Bun => {
                self.launch_bun(code).await
            }
            Runtime::Deno => {
                self.launch_deno(code).await
            }
            Runtime::Custom(_) => {
                Err(BenchmarkError::RuntimeNotAvailable(
                    "Custom runtime not supported".to_string())
            }
        }.map(|mut output| {
            output.elapsed_time = start_time.elapsed();
            output
        })
    }
    /// 启动 Amber 进程
    async fn launch_amberjs(&self, code: &str) -> Result<ProcessOutput> {
        let temp_file: _ = super::super::utils::create_temp_dir("amberjs_bench")?;
        let file_path: _ = temp_file.path().join("test.js");
        // 写入临时文件
        tokio::fs::write(&file_path, code).await?;
        // 启动进程
        let output: _ = tokio::process::Command::new("amber")
            .arg(&file_path)
            .output()
            .await?;
        let exit_code: _ = output.status.code().unwrap_or(-1);
        let stdout: _ = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr: _ = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(ProcessOutput {
            runtime: Runtime::Amber,
            exit_code,
            stdout,
            stderr,
            elapsed_time: tokio::time::Duration::from_secs(0),
        })
    }
    /// 启动 Node.js 进程
    async fn launch_nodejs(&self, code: &str) -> Result<ProcessOutput> {
        let temp_file: _ = super::super::utils::create_temp_dir("nodejs_bench")?;
        let file_path: _ = temp_file.path().join("test.js");
        tokio::fs::write(&file_path, code).await?;
        let output: _ = tokio::process::Command::new("node")
            .arg(&file_path)
            .output()
            .await?;
        let exit_code: _ = output.status.code().unwrap_or(-1);
        let stdout: _ = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr: _ = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(ProcessOutput {
            runtime: Runtime::NodeJs,
            exit_code,
            stdout,
            stderr,
            elapsed_time: tokio::time::Duration::from_secs(0),
        })
    }
    /// 启动 Bun 进程
    async fn launch_bun(&self, code: &str) -> Result<ProcessOutput> {
        let temp_file: _ = super::super::utils::create_temp_dir("bun_bench")?;
        let file_path: _ = temp_file.path().join("test.js");
        tokio::fs::write(&file_path, code).await?;
        let output: _ = tokio::process::Command::new("bun")
            .arg(&file_path)
            .output()
            .await?;
        let exit_code: _ = output.status.code().unwrap_or(-1);
        let stdout: _ = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr: _ = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(ProcessOutput {
            runtime: Runtime::Bun,
            exit_code,
            stdout,
            stderr,
            elapsed_time: tokio::time::Duration::from_secs(0),
        })
    }
    /// 启动 Deno 进程
    async fn launch_deno(&self, code: &str) -> Result<ProcessOutput> {
        let temp_file: _ = super::super::utils::create_temp_dir("deno_bench")?;
        let file_path: _ = temp_file.path().join("test.js");
        tokio::fs::write(&file_path, code).await?;
        let output: _ = tokio::process::Command::new("deno")
            .arg("run")
            .arg(&file_path)
            .output()
            .await?;
        let exit_code: _ = output.status.code().unwrap_or(-1);
        let stdout: _ = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr: _ = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(ProcessOutput {
            runtime: Runtime::Deno,
            exit_code,
            stdout,
            stderr,
            elapsed_time: tokio::time::Duration::from_secs(0),
        })
    }
}
/// 进程启动配置
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// 超时时间
    pub timeout: tokio::time::Duration,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 工作目录
    pub working_directory: Option<PathBuf>,
}
impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            timeout: tokio::time::Duration::from_secs(30),
            environment: HashMap::new(),
            working_directory: None,
        }
    }
}
impl ProcessConfig {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }
    /// 设置超时时间
    pub fn timeout(mut self, timeout: tokio::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }
    /// 添加环境变量
    pub fn add_env(mut self, key: &str, value: &str) -> Self {
        self.environment.insert(key.to_string(), value.to_string());
        self
    }
    /// 设置工作目录
    pub fn working_directory(mut self, dir: PathBuf) -> Self {
        self.working_directory = Some(dir);
        self
    }
}
/// 进程输出
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    /// 运行时类型
    pub runtime: Runtime,
    /// 退出码
    pub exit_code: i32,
    /// 标准输出
    pub stdout: String,
    /// 标准错误
    pub stderr: String,
    /// 执行时间
    pub elapsed_time: tokio::time::Duration,
}
impl ProcessOutput {
    /// 检查是否成功
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
    /// 获取执行时间 (毫秒)
    pub fn execution_time_ms(&self) -> f64 {
        self.elapsed_time.as_secs_f64() * 1000.0
    }
}
/// 对比报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    /// 测试名称
    pub test_name: String,
    /// 基准运行时结果
    pub baseline_result: BenchmarkResult,
    /// 对比运行时结果
    pub comparison_results: Vec<BenchmarkResult>,
    /// 性能对比
    pub performance_comparison: PerformanceComparison,
    /// 统计显著性
    pub statistical_significance: StatisticalSignificance,
}
impl ComparisonReport {
    /// 创建新的对比报告
    pub fn new(test_name: &str, baseline_result: BenchmarkResult) -> Self {
        Self {
            test_name: test_name.to_string(),
            baseline_result,
            comparison_results: Vec::new(),
            performance_comparison: PerformanceComparison::default(),
            statistical_significance: StatisticalSignificance::default(),
        }
    }
    /// 添加对比结果
    pub fn add_comparison_result(&mut self, result: BenchmarkResult) {
        self.comparison_results.push(result);
    }
    /// 生成性能对比
    pub fn generate_performance_comparison(&mut self) {
        self.performance_comparison = PerformanceComparison::generate(
            &self.baseline_result,
            &self.comparison_results,
        );
    }
    /// 生成统计显著性分析
    pub fn generate_statistical_analysis(&mut self) {
        self.statistical_significance = StatisticalSignificance::analyze(
            &self.baseline_result,
            &self.comparison_results,
        );
    }
}
/// 性能对比
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceComparison {
    /// 性能提升百分比
    pub improvements: Vec<PerformanceImprovement>,
    /// 吞吐量对比
    pub throughput_comparison: HashMap<Runtime, f64>,
    /// 延迟对比
    pub latency_comparison: HashMap<Runtime, f64>,
}
impl PerformanceComparison {
    /// 生成性能对比
    pub fn generate(
        baseline: &BenchmarkResult,
        comparisons: &[BenchmarkResult],
    ) -> Self {
        let mut improvements = Vec::new();
        let mut throughput_comparison = HashMap::new();
        let mut latency_comparison = HashMap::new();
        // 计算基准性能
        let baseline_throughput: _ = baseline.throughput();
        let baseline_latency: _ = baseline.average_duration().as_secs_f64() * 1000.0;
        for result in comparisons {
            // 计算性能提升
            let throughput_improvement: _ = if baseline_throughput > 0.0 {
                ((result.throughput() - baseline_throughput) / baseline_throughput) * 100.0
            } else {
                0.0
            };
            let latency_improvement: _ = if baseline_latency > 0.0 {
                ((baseline_latency - result.average_duration().as_secs_f64() * 1000.0) / baseline_latency) * 100.0
            } else {
                0.0
            };
            improvements.push(PerformanceImprovement {
                runtime: result.runtime,
                throughput_improvement,
                latency_improvement,
            });
            throughput_comparison.insert(result.runtime, result.throughput());
            latency_comparison.insert(result.runtime, result.average_duration().as_secs_f64() * 1000.0);
        }
        Self {
            improvements,
            throughput_comparison,
            latency_comparison,
        }
    }
}
/// 性能提升
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImprovement {
    /// 运行时类型
    pub runtime: Runtime,
    /// 吞吐量提升百分比
    pub throughput_improvement: f64,
    /// 延迟提升百分比
    pub latency_improvement: f64,
}
/// 统计显著性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatisticalSignificance {
    /// 显著性测试结果
    pub significance_tests: Vec<SignificanceTest>,
    /// 置信区间
    pub confidence_intervals: HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64), Runtime, (f64, f64), std::collections::HashMap<Runtime, (f64, f64), Runtime, (f64, f64)>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>,
}
impl StatisticalSignificance {
    /// 分析统计显著性
    pub fn analyze(
        baseline: &BenchmarkResult,
        comparisons: &[BenchmarkResult],
    ) -> Self {
        let mut significance_tests = Vec::new();
        let mut confidence_intervals = HashMap::new();
        for result in comparisons {
            let is_significant: _ = baseline.is_statistically_significant(result, 0.05);
            significance_tests.push(SignificanceTest {
                runtime: result.runtime,
                is_significant,
                p_value: 0.0, // 简化实现
            });
            confidence_intervals.insert(
                result.runtime,
                (
                    result.statistics.confidence_interval_lower.as_secs_f64() * 1000.0,
                    result.statistics.confidence_interval_upper.as_secs_f64() * 1000.0,
                )
            );
        }
        Self {
            significance_tests,
            confidence_intervals,
        }
    }
}
/// 显著性测试
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignificanceTest {
    /// 运行时类型
    pub runtime: Runtime,
    /// 是否显著
    pub is_significant: bool,
    /// P 值
    pub p_value: f64,
}
/// 检查命令是否可用
fn check_command_available(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
/// 获取命令版本
fn get_command_version(command: &str) -> Option<String> {
    let output: _ = Command::new(command)
        .arg("--version")
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_runtime_detection() {
        let detector: _ = RuntimeDetector::new();
        let available: _ = detector.get_available_runtimes();
        println!("Available runtimes: {:?}", available);
    }
    #[tokio::test]
    async fn test_process_launcher() {
        let config: _ = ProcessConfig::new();
        let launcher: _ = ProcessLauncher::new(config);
        let code: _ = r#"
            console.log('Hello, World!');
            const start = Date.now();
            // 模拟一些计算
            let sum: _ = 0;
            for (let i: _ = 0; i < 1000000; i++) {
                sum += i;
            }
            const end = Date.now();
            console.log('Time:', end - start, 'ms');
        "#;
        let output: _ = launcher.launch(code, Runtime::NodeJs).await.unwrap();
        println!("Output: {}", output.stdout);
        println!("Time: {} ms", output.execution_time_ms());
    }
}
