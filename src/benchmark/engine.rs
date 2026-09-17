// 基准测试引擎
//
// 提供完整的基准测试执行引擎，支持：
// - 灵活的测试配置
// - 并行测试执行
// - 实时性能监控
// - 结果收集和分析

use std::collections::{HashMap};
use std::sync::{Arc, Mutex};
use tokio::task::::{JoinHandle, spawn_blocking};
use tokio::sync::{TokioMutex, Semaphore};
use tracing::{debug, error, info, instrument, warn};
use std::time::{Duration, Instant};
use std::path::PathBuf;

    BenchmarkConfig, BenchmarkTest, TestSuite, WorkloadProfile, RuntimeComparison,
    BenchmarkResult, BenchmarkResultSet, EnvironmentInfo, Statistics,
    Runtime, MetricType, BenchmarkError, BenchmarkResult as Result,
};
/// 基准测试引擎
#[derive(Debug)]
pub struct BenchmarkEngine {
    /// 配置
    config: BenchmarkConfig,
    /// 测试套件
    test_suite: Option<TestSuite>,
    /// 结果集合
    results: Arc<Mutex<BenchmarkResultSet>>,
    /// 并发控制
    semaphore: Arc<Semaphore>,
    /// 环境信息
    environment: EnvironmentInfo,
    /// 进度回调
    progress_callback: Option<Arc<dyn Fn(ProgressInfo) + Send + Sync>>,
}
impl BenchmarkEngine {
    /// 创建新的基准测试引擎
    pub fn new(config: BenchmarkConfig) -> Self {
        let semaphore: _ = Arc::new(Mutex::new(Semaphore::new(config.workers as usize)),;
        let results: _ = Arc::new(Mutex::new(BenchmarkResultSet::new(&config.name)),;
        Self {
            config,
            test_suite: None,
            results,
            semaphore,
            environment: EnvironmentInfo::default(),
            progress_callback: None,
        }
    }
    /// 设置测试套件
    pub fn test_suite(mut self, suite: TestSuite) -> Self {
        self.test_suite = Some(suite);
        self
    }
    /// 设置环境信息
    pub fn environment(mut self, env: EnvironmentInfo) -> Self {
        self.environment = env;
        self
    }
    /// 设置进度回调
    pub fn progress_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(ProgressInfo) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(Mutex::new(callback)),;
        self
    }
    /// 执行所有基准测试
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<BenchmarkResultSet> {
        info!("Starting benchmark execution: {}", self.config.name);
        // 验证配置
        self.validate_config()?;
        // 初始化结果集合
        {
            let mut results = self.results.lock().await;
            results.environment = self.environment.clone();
        }
        // 执行预热迭代
        if self.config.warmup_iterations > 0 {
            self.run_warmup().await?;
        }
        // 执行测试套件或单个测试
        if let Some(ref suite) = self.test_suite {
            self.run_test_suite(suite).await?;
        } else {
            return Err(BenchmarkError::ConfigError(
                "No test suite configured".to_string());
        }
        // 收集结果
        let final_results: _ = self.results.lock().await.clone();
        info!("Benchmark execution completed: {}", self.config.name);
        Ok(final_results)
    }
    /// 验证配置
    fn validate_config(&self) -> Result<()> {
        if self.config.iterations == 0 {
            return Err(BenchmarkError::ConfigError(
                "Iterations must be greater than 0".to_string());
        }
        if self.config.workers == 0 {
            return Err(BenchmarkError::ConfigError(
                "Workers must be greater than 0".to_string());
        }
        if self.config.timeout == Duration::from_secs(0) {
            return Err(BenchmarkError::ConfigError(
                "Timeout must be greater than 0".to_string());
        }
        Ok(())
    }
    /// 执行预热迭代
    async fn run_warmup(&self) -> Result<()> {
        info!("Running warmup iterations: {}", self.config.warmup_iterations);
        // 这里可以实现预热逻辑
        // 例如运行一些简单的代码来让 JIT 编译
        Ok(())
    }
    /// 执行测试套件
    async fn run_test_suite(&self, suite: &TestSuite) -> Result<()> {
        info!("Running test suite: {}", suite.name);
        // 运行设置脚本
        if let Some(ref setup_script) = suite.setup_script {
            self.run_setup_script(setup_script).await?;
        }
        // 并行执行基准测试
        let mut handles: Vec<JoinHandle<Result<()>> = Vec::new();
        for (index, benchmark) in suite.benchmarks.iter().enumerate() {
            if !benchmark.enabled {
                continue;
            }
            let semaphore: _ = self.semaphore.clone();
            let results: _ = self.results.clone();
            let config: _ = self.config.clone();
            let suite_env: _ = suite.environment.clone();
            let handle: _ = spawn(async move {
                // 获取并发许可
                let _permit: _ = semaphore.acquire().await.unwrap();
                // 执行基准测试
                Self::run_single_benchmark(&config, &benchmark, &suite_env, results, index).await
            });
            handles.push(handle);
        }
        // 等待所有测试完成
        for handle in handles {
            handle.await??;
        }
        // 运行清理脚本
        if let Some(ref cleanup_script) = suite.cleanup_script {
            self.run_cleanup_script(cleanup_script).await?;
        }
        Ok(())
    }
    /// 执行单个基准测试
    async fn run_single_benchmark(
        config: &BenchmarkConfig,
        benchmark: &BenchmarkTest,
        suite_env: &HashMap<String, String>,
        results: Arc<Mutex<BenchmarkResultSet>>,
        index: usize,
    ) -> Result<()> {
        let test_name: _ = &benchmark.name;
        info!("Running benchmark: {}", test_name);
        // 创建结果
        let mut result = BenchmarkResult::new(test_name, Runtime::Amber);
        // 添加环境变量
        for (key, value) in suite_env {
            result.add_metadata(&format!("env_{}", key), value);
        }
        // 添加测试特定的环境变量
        for (key, value) in &benchmark.environment {
            result.add_metadata(key, value);
        }
        // 开始测试
        result.start();
        // 确定迭代次数
        let iterations: _ = benchmark.iterations.unwrap_or(config.iterations);
        let timeout: _ = benchmark.timeout.unwrap_or(config.timeout);
        // 执行迭代
        for i in 0..iterations {
            // 检查超时
            if i > 0 && i % 10 == 0 {
                let elapsed: _ = result.start_time.elapsed();
                if elapsed > timeout {
                    result.add_error(&format!("Test timed out after {:?}", elapsed));
                    break;
                }
            }
            // 执行单次迭代
            match Self::execute_single_iteration(benchmark, i).await {
                Ok(duration) => {
                    result.add_iteration(duration);
                }
                Err(e) => {
                    result.add_error(&format!("Iteration {} failed: {}", i, e));
                    // 如果是致命错误，停止测试
                    if i < 3 {
                        break;
                    }
                }
            }
        }
        // 完成测试
        result.finish();
        // 添加元数据
        result.add_metadata("benchmark_index", &index.to_string());
        result.add_metadata("total_iterations", &iterations.to_string());
        // 保存结果
        {
            let mut results = results.lock().await;
            results.add_result(result);
        }
        info!("Completed benchmark: {}", test_name);
        Ok(())
    }
    /// 执行单次迭代
    async fn execute_single_iteration(
        benchmark: &BenchmarkTest,
        iteration: usize,
    ) -> Result<Duration> {
        let start: _ = Instant::now();
        // 这里应该实际执行代码
        // 暂时使用模拟实现
        spawn_blocking(move || {
            // 模拟代码执行时间 (随机 1-10ms)
            let execution_time: _ = Duration::from_millis(1 + (iteration % 10));
            std::thread::sleep(execution_time);
            Ok::<Duration, BenchmarkError>(start.elapsed())
        }).await??;
        Ok(start.elapsed())
    }
    /// 运行设置脚本
    async fn run_setup_script(&self, script: &PathBuf) -> Result<()> {
        info!("Running setup script: {:?}", script);
        // 这里应该执行设置脚本
        // 暂时跳过实现
        Ok(())
    }
    /// 运行清理脚本
    async fn run_cleanup_script(&self, script: &PathBuf) -> Result<()> {
        info!("Running cleanup script: {:?}", script);
        // 这里应该执行清理脚本
        // 暂时跳过实现
        Ok(())
    }
    /// 保存结果到文件
    pub async fn save_results(&self, results: &BenchmarkResultSet, output_path: &PathBuf) -> Result<()> {
        info!("Saving results to: {:?}", output_path);
        // 创建输出目录
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        // 序列化结果
        let json: _ = serde_json::to_string_pretty(results)
            .map_err(BenchmarkError::JsonError)?;
        // 写入文件
        tokio::fs::write(output_path, json).await?;
        Ok(())
    }
    /// 生成报告
    pub async fn generate_report(&self, results: &BenchmarkResultSet) -> Result<String> {
        info!("Generating report");
        // 这里应该生成 HTML 或其他格式的报告
        // 暂时返回 JSON
        let json: _ = serde_json::to_string_pretty(results)
            .map_err(BenchmarkError::JsonError)?;
        Ok(json)
    }
    /// 获取进度信息
    pub async fn get_progress(&self) -> ProgressInfo {
        let results: _ = self.results.lock().await;
        let total_tests: _ = results.results.len();
        let completed_tests: _ = results.results.iter()
            .filter(|r| r.success)
            .count();
        ProgressInfo {
            total_tests,
            completed_tests,
            failed_tests: total_tests - completed_tests,
            elapsed_time: results.run_time.elapsed(),
            estimated_remaining: if completed_tests > 0 {
                let avg_per_test: _ = results.run_time.elapsed() / completed_tests as u32;
                Some(avg_per_test * (total_tests - completed_tests) as u32)
            } else {
                None
            },
        }
    }
}
/// 进度信息
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// 总测试数
    pub total_tests: usize,
    /// 已完成测试数
    pub completed_tests: usize,
    /// 失败测试数
    pub failed_tests: usize,
    /// 已过时间
    pub elapsed_time: Duration,
    /// 预计剩余时间
    pub estimated_remaining: Option<Duration>,
}
impl ProgressInfo {
    /// 获取完成百分比
    pub fn completion_percentage(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.completed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }
    /// 检查是否完成
    pub fn is_complete(&self) -> bool {
        self.completed_tests + self.failed_tests >= self.total_tests
    }
    /// 获取预计总时间
    pub fn estimated_total_time(&self) -> Option<Duration> {
        if self.completed_tests > 0 {
            let avg_per_test: _ = self.elapsed_time / self.completed_tests as u32;
            Some(avg_per_test * self.total_tests as u32)
        } else {
            None
        }
    }
}
/// 基准测试运行器 (用于测试)
#[cfg(test)]
pub struct BenchmarkRun {
    /// 测试名称
    pub name: String,
    /// 测试代码
    pub code: String,
    /// 迭代次数
    pub iterations: u32,
}
#[cfg(test)]
impl BenchmarkRun {
    /// 创建新的基准测试运行器
    pub fn new(name: &str, code: &str, iterations: u32) -> Self {
        Self {
            name: name.to_string(),
            code: code.to_string(),
            iterations,
        }
    }
    /// 执行基准测试
    pub async fn run(&self) -> Result<BenchmarkResult> {
        let mut result = BenchmarkResult::new(&self.name, Runtime::Amber);
        result.start();
        for i in 0..self.iterations {
            let start: _ = Instant::now();
            // 执行代码
            // 这里应该实际执行代码，暂时使用模拟
            tokio::time::sleep(Duration::from_millis(1)).await;
            let duration: _ = start.elapsed();
            result.add_iteration(duration);
        }
        result.finish();
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_benchmark_engine_creation() {
        let config: _ = BenchmarkConfig::default();
        let engine: _ = BenchmarkEngine::new(config);
        assert_eq!(engine.config.name, "default");
        assert_eq!(engine.config.iterations, 10);
    }
    #[tokio::test]
    async fn test_benchmark_run() {
        let run: _ = BenchmarkRun::new("test", "console.log('hello')", 5);
        let result: _ = run.run().await.unwrap();
        assert_eq!(result.name, "test");
        assert_eq!(result.actual_iterations, 5);
        assert!(result.success);
        assert!(result.average_duration() > Duration::from_millis(1));
    }
    #[tokio::test]
    async fn test_benchmark_result_statistics() {
        let mut result = BenchmarkResult::new("test", Runtime::Amber);
        result.start();
        // 添加一些迭代
        for i in 1..=10 {
            result.add_iteration(Duration::from_millis(i));
        }
        result.finish();
        assert_eq!(result.actual_iterations, 10);
        assert!(result.statistics.sample_count, 10);
        assert!(result.statistics.mean > Duration::from_millis(0));
    }
    #[tokio::test]
    async fn test_progress_info() {
        let progress: _ = ProgressInfo {
            total_tests: 100,
            completed_tests: 50,
            failed_tests: 0,
            elapsed_time: Duration::from_secs(10),
            estimated_remaining: Some(Duration::from_secs(10)),
        };
        assert_eq!(progress.completion_percentage(), 50.0);
        assert!(!progress.is_complete());
        assert!(progress.estimated_total_time().is_some());
    }
}