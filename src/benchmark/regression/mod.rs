// 性能回归检测模块
//
// 提供性能回归自动检测功能


use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::{BenchmarkResultSet, BenchmarkResult, Runtime, MetricType};
/// 性能历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceHistory {
    /// Git commit hash
    pub commit_hash: String,
    /// 时间戳
    pub timestamp: SystemTime,
    /// 测试结果
    pub results: BenchmarkResultSet,
    /// Git 分支
    pub git_branch: String,
    /// 构建信息
    pub build_info: BuildInfo,
}
impl PerformanceHistory {
    /// 创建新的历史记录
    pub fn new(
        commit_hash: String,
        results: BenchmarkResultSet,
        git_branch: String,
    ) -> Self {
        Self {
            commit_hash,
            timestamp: SystemTime::now(),
            results,
            git_branch,
            build_info: BuildInfo::default(),
        }
    }
}
/// 构建信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildInfo {
    /// Rust 版本
    pub rust_version: String,
    /// V8 版本
    pub v8_version: Option<String>,
    /// 编译时间
    pub build_time: SystemTime,
    /// 优化级别
    pub optimization_level: String,
}
/// 回归检测器
#[derive(Debug)]
pub struct RegressionDetector {
    /// 历史数据路径
    pub history_path: PathBuf,
    /// 显著性阈值
    pub significance_threshold: f64,
    /// 回归阈值 (百分比)
    pub regression_threshold: f64,
}
impl RegressionDetector {
    /// 创建新的回归检测器
    pub fn new(history_path: PathBuf) -> Self {
        Self {
            history_path,
            significance_threshold: 0.05,
            regression_threshold: 10.0, // 10% 回归视为显著
        }
    }
    /// 检测回归
    pub async fn detect_regressions(
        &self,
        current_results: &BenchmarkResultSet,
        baseline_results: &BenchmarkResultSet,
    ) -> Result<RegressionReport, super::BenchmarkError> {
        let mut report = RegressionReport::new();
        // 对比每个测试结果
        for current_result in &current_results.results {
            if let Some(baseline_result) = Self::find_baseline_result(
                baseline_results,
                &current_result.name,
                current_result.runtime,
            ) {
                let analysis: _ = self.analyze_regression(current_result, baseline_result);
                report.add_analysis(analysis);
            }
        }
        Ok(report)
    }
    /// 分析回归
    fn analyze_regression(
        &self,
        current: &BenchmarkResult,
        baseline: &BenchmarkResult,
    ) -> RegressionAnalysis {
        let performance_change: _ = self.calculate_performance_change(current, baseline);
        let is_regression: _ = performance_change < -self.regression_threshold;
        let is_significant: _ = Self::is_statistically_significant(
            current,
            baseline,
            self.significance_threshold,
        );
        RegressionAnalysis {
            test_name: current.name.clone(),
            runtime: current.runtime,
            baseline_performance: baseline.average_duration(),
            current_performance: current.average_duration(),
            performance_change_percent: performance_change,
            is_regression,
            is_significant,
            confidence_level: if is_significant { 0.95 } else { 0.0 },
        }
    }
    /// 计算性能变化
    fn calculate_performance_change(&self, current: &BenchmarkResult, baseline: &BenchmarkResult) -> f64 {
        let baseline_ns: _ = baseline.average_duration().as_nanos() as f64;
        let current_ns: _ = current.average_duration().as_nanos() as f64;
        if baseline_ns == 0.0 {
            0.0
        } else {
            ((baseline_ns - current_ns) / baseline_ns) * 100.0
        }
    }
    /// 检查统计显著性
    fn is_statistically_significant(
        current: &BenchmarkResult,
        baseline: &BenchmarkResult,
        alpha: f64,
    ) -> bool {
        current.is_statistically_significant(baseline, alpha)
    }
    /// 查找基线结果
    fn find_baseline_result(
        baseline_results: &BenchmarkResultSet,
        test_name: &str,
        runtime: Runtime,
    ) -> Option<&BenchmarkResult> {
        baseline_results.results.iter().find(|r| {
            r.name == test_name && r.runtime == runtime
        })
    }
    /// 保存历史记录
    pub async fn save_history(
        &self,
        history: &PerformanceHistory,
    ) -> Result<(), super::BenchmarkError> {
        use super::super::utils::{create_dir_if_not_exists, write_file};
        create_dir_if_not_exists(&self.history_path)?;
        let filename: _ = format!(
            "{}_{}.json",
            history.commit_hash,
            history.timestamp.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        );
        let filepath: _ = self.history_path.join(filename);
        let json: _ = serde_json::to_string_pretty(history)
            .map_err(super::BenchmarkError::JsonError)?;
        write_file(&filepath, &json)?;
        Ok(())
    }
    /// 加载历史记录
    pub async fn load_history(
        &self,
        commit_hash: &str,
    ) -> Result<PerformanceHistory, super::BenchmarkError> {
        use super::super::utils::read_file;
        // 查找匹配的历史文件
        let entries: _ = tokio::fs::read_dir(&self.history_path).await?;
        for entry in entries {
            let entry: _ = entry?;
            let filename: _ = entry.file_name().to_string_lossy().to_string();
            if filename.starts_with(commit_hash) {
                let filepath: _ = self.history_path.join(&filename);
                let content: _ = read_file(&filepath)?;
                let history: PerformanceHistory = serde_json::from_str(&content)
                    .map_err(super::BenchmarkError::JsonError)?;
                return Ok(history);
            }
        }
        Err(super::BenchmarkError::ConfigError(
            format!("History not found for commit: {}", commit_hash))
    }
}
/// 回归报告
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegressionReport {
    /// 回归分析列表
    pub analyses: Vec<RegressionAnalysis>,
    /// 总结
    pub summary: RegressionSummary,
}
impl RegressionReport {
    /// 创建新的报告
    pub fn new() -> Self {
        Self::default()
    }
    /// 添加分析
    pub fn add_analysis(&mut self, analysis: RegressionAnalysis) {
        self.analyses.push(analysis.clone());
        self.update_summary();
    }
    /// 更新总结
    fn update_summary(&mut self) {
        let total_tests: _ = self.analyses.len();
        let regressions: _ = self.analyses.iter().filter(|a| a.is_regression).count();
        let significant_regressions: _ = self.analyses
            .iter()
            .filter(|a| a.is_regression && a.is_significant)
            .count();
        self.summary = RegressionSummary {
            total_tests,
            regressions,
            significant_regressions,
            regression_rate: if total_tests > 0 {
                (regressions as f64 / total_tests as f64) * 100.0
            } else {
                0.0
            },
            max_performance_decline: self.analyses
                .iter()
                .filter(|a| a.is_regression)
                .map(|a| a.performance_change_percent.abs())
                .fold(0.0, f64::max),
        };
    }
    /// 检查是否有回归
    pub fn has_regressions(&self) -> bool {
        self.analyses.iter().any(|a| a.is_regression)
    }
    /// 检查是否有显著回归
    pub fn has_significant_regressions(&self) -> bool {
        self.analyses.iter().any(|a| a.is_regression && a.is_significant)
    }
    /// 获取回归列表
    pub fn get_regressions(&self) -> Vec<&RegressionAnalysis> {
        self.analyses.iter().filter(|a| a.is_regression).collect()
    }
}
/// 回归分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAnalysis {
    /// 测试名称
    pub test_name: String,
    /// 运行时类型
    pub runtime: Runtime,
    /// 基线性能
    pub baseline_performance: Duration,
    /// 当前性能
    pub current_performance: Duration,
    /// 性能变化百分比
    pub performance_change_percent: f64,
    /// 是否为回归
    pub is_regression: bool,
    /// 是否显著
    pub is_significant: bool,
    /// 置信度
    pub confidence_level: f64,
}
impl RegressionAnalysis {
    /// 获取性能变化描述
    pub fn get_change_description(&self) -> String {
        if self.performance_change_percent > 0.0 {
            format!("性能提升 {:.2}%", self.performance_change_percent)
        } else if self.performance_change_percent < 0.0 {
            format!("性能下降 {:.2}%", self.performance_change_percent.abs())
        } else {
            "性能无变化".to_string()
        }
    }
    /// 获取严重程度
    pub fn get_severity(&self) -> RegressionSeverity {
        if !self.is_regression {
            return RegressionSeverity::None;
        }
        let decline: _ = self.performance_change_percent.abs();
        if decline >= 50.0 {
            RegressionSeverity::Critical
        } else if decline >= 20.0 {
            RegressionSeverity::High
        } else if decline >= 10.0 {
            RegressionSeverity::Medium
        } else {
            RegressionSeverity::Low
        }
    }
}
/// 回归严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegressionSeverity {
    /// 无回归
    None,
    /// 轻微回归
    Low,
    /// 中等回归
    Medium,
    /// 严重回归
    High,
    /// 关键回归
    Critical,
}
impl std::fmt::Display for RegressionSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegressionSeverity::None => write!(f, "None"),
            RegressionSeverity::Low => write!(f, "Low"),
            RegressionSeverity::Medium => write!(f, "Medium"),
            RegressionSeverity::High => write!(f, "High"),
            RegressionSeverity::Critical => write!(f, "Critical"),
        }
    }
}
/// 回归总结
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionSummary {
    /// 总测试数
    pub total_tests: usize,
    /// 回归数
    pub regressions: usize,
    /// 显著回归数
    pub significant_regressions: usize,
    /// 回归率 (百分比)
    pub regression_rate: f64,
    /// 最大性能下降
    pub max_performance_decline: f64,
}
impl RegressionSummary {
    /// 检查是否通过
    pub fn is_pass(&self) -> bool {
        self.significant_regressions == 0
    }
    /// 获取状态
    pub fn get_status(&self) -> String {
        if self.is_pass() {
            "PASS".to_string()
        } else {
            "FAIL".to_string()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, SystemTime};
    #[test]
    fn test_regression_analysis() {
        let current: _ = super::super::result::BenchmarkResult::new("test", Runtime::Amber);
        let baseline: _ = super::super::result::BenchmarkResult::new("test", Runtime::Amber);
        let detector: _ = RegressionDetector::new(PathBuf::from("/tmp/test"));
        let analysis: _ = detector.analyze_regression(&current, &baseline);
        println!("Analysis: {:?}", analysis);
    }
    #[test]
    fn test_regression_summary() {
        let mut report = RegressionReport::new();
        // 添加一些分析
        for i in 0..5 {
            let mut analysis = RegressionAnalysis {
                test_name: format!("test_{}", i),
                runtime: Runtime::Amber,
                baseline_performance: Duration::from_millis(100),
                current_performance: Duration::from_millis(120),
                performance_change_percent: -20.0,
                is_regression: i < 3,
                is_significant: i < 2,
                confidence_level: 0.95,
            };
            report.add_analysis(analysis);
        }
        println!("Summary: {:?}", report.summary);
        println!("Has regressions: {}", report.has_regressions());
        println!("Has significant regressions: {}", report.has_significant_regressions());
    }
}