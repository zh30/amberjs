//! 性能回归检测器
//!
//! 这个工具用于自动化检测 Amber 的性能回归，通过与历史基线对比、
//! 统计检验和阈值检查，识别性能退化问题。支持基线管理、回归分析、
//! 报告生成和 CI/CD 集成。

use amberjs::performance_analyzer::PerformanceAnalyzer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

/// 性能基线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub version: String,
    pub timestamp: String,
    pub benchmarks: HashMap<String, BenchmarkStats>,
}

/// 基准测试统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStats {
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
    pub std_dev: f64,
    pub samples: usize,
    pub unit: String,
}

/// 性能结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceResults {
    pub version: String,
    pub timestamp: String,
    pub benchmarks: HashMap<String, BenchmarkResult>,
}

/// 单个基准测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
    pub std_dev: f64,
    pub samples: usize,
    pub unit: String,
}

/// 回归问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionIssue {
    pub benchmark: String,
    pub baseline_score: f64,
    pub current_score: f64,
    pub regression_percentage: f64,
    pub severity: RegressionSeverity,
    pub statistical_significance: bool,
    pub confidence_level: f64,
}

/// 回归严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegressionSeverity {
    Critical,
    Major,
    Minor,
    Warning,
}

/// 统计检验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalTest {
    pub test_name: String,
    pub p_value: f64,
    pub is_significant: bool,
    pub confidence_level: f64,
    pub effect_size: f64,
}

/// 性能阈值配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    pub regression_threshold: f64,
    pub critical_threshold: f64,
    pub confidence_level: f64,
    pub min_samples: usize,
    pub statistical_test: String,
}

/// 基线管理器
pub struct BaselineManager {
    baselines: HashMap<String, PerformanceBaseline>,
    current_baseline: Option<PerformanceBaseline>,
}

/// 回归分析器
pub struct RegressionAnalyzer {
    thresholds: HashMap<String, PerformanceThresholds>,
    confidence_level: f64,
}

/// 统计检验器
pub struct StatisticalTests;

/// 阈值检查器
pub struct ThresholdChecker {
    thresholds: HashMap<String, PerformanceThresholds>,
}

/// 报告生成器
pub struct ReportGenerator {
    issues: Vec<RegressionIssue>,
    baseline_version: String,
    current_version: String,
}

impl BaselineManager {
    /// 创建新的基线管理器
    pub fn new() -> Self {
        Self {
            baselines: HashMap::new(),
            current_baseline: None,
        }
    }

    /// 加载基线文件
    pub fn load_baseline<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let baseline: PerformanceBaseline = serde_json::from_str(&content)?;

        self.baselines.insert(baseline.version.clone(), baseline.clone());
        self.current_baseline = Some(baseline);

        println!("📊 已加载基线: {} (时间戳: {})",
            baseline.version, baseline.timestamp);

        Ok(())
    }

    /// 保存基线
    pub fn save_baseline<P: AsRef<Path>>(
        &self,
        version: &str,
        results: &PerformanceResults,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let baseline = PerformanceBaseline {
            version: version.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            benchmarks: results.benchmarks.iter()
                .map(|(name, result)| {
                    (
                        name.clone(),
                        BenchmarkStats {
                            mean: result.mean,
                            median: result.median,
                            p95: result.p95,
                            p99: result.p99,
                            std_dev: result.std_dev,
                            samples: result.samples,
                            unit: result.unit.clone(),
                        }
                    )
                })
                .collect(),
        };

        let content = serde_json::to_string_pretty(&baseline)?;
        fs::write(path, content)?;

        println!("💾 基线已保存: {}", version);

        Ok(())
    }

    /// 获取基线
    pub fn get_baseline(&self, version: &str) -> Option<&PerformanceBaseline> {
        self.baselines.get(version)
    }

    /// 获取当前基线
    pub fn get_current_baseline(&self) -> Option<&PerformanceBaseline> {
        self.current_baseline.as_ref()
    }

    /// 列出所有基线版本
    pub fn list_baselines(&self) -> Vec<String> {
        self.baselines.keys().cloned().collect()
    }

    /// 验证基线有效性
    pub fn validate_baseline(&self, baseline: &PerformanceBaseline) -> ValidationResult {
        let mut issues = vec![];

        if baseline.benchmarks.is_empty() {
            issues.push("基线不包含任何基准测试".to_string());
        }

        for (name, stats) in &baseline.benchmarks {
            if stats.samples == 0 {
                issues.push(format!("基准测试 '{}' 样本数为 0", name));
            }

            if stats.mean <= 0.0 {
                issues.push(format!("基准测试 '{}' 平均值为非正值", name));
            }

            if stats.std_dev < 0.0 {
                issues.push(format!("基准测试 '{}' 标准差为负值", name));
            }
        }

        ValidationResult {
            is_valid: issues.is_empty(),
            issues,
        }
    }
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub issues: Vec<String>,
}

impl RegressionAnalyzer {
    /// 创建新的回归分析器
    pub fn new(thresholds: HashMap<String, PerformanceThresholds>, confidence_level: f64) -> Self {
        Self {
            thresholds,
            confidence_level,
        }
    }

    /// 检测性能回归
    pub async fn detect_regression(
        &self,
        baseline: &PerformanceBaseline,
        current: &PerformanceResults,
    ) -> Result<Vec<RegressionIssue>, Box<dyn std::error::Error>> {
        println!("🔍 开始性能回归检测...");

        let mut issues = vec![];

        for (benchmark_name, current_result) in &current.benchmarks {
            if let Some(baseline_stats) = baseline.benchmarks.get(benchmark_name) {
                // 计算性能差异
                let diff_percentage = self.calculate_difference(current_result, baseline_stats);

                // 统计显著性检验
                let statistical_test = self.statistical_test(current_result, baseline_stats);

                // 阈值检查
                let thresholds = self.thresholds.get(benchmark_name)
                    .unwrap_or(&PerformanceThresholds {
                        regression_threshold: 0.05,
                        critical_threshold: 0.10,
                        confidence_level: self.confidence_level,
                        min_samples: 100,
                        statistical_test: "welch_t_test".to_string(),
                    });

                let exceeds_threshold = diff_percentage.abs() > thresholds.regression_threshold;
                let is_significant = statistical_test.is_significant;

                if exceeds_threshold && is_significant {
                    let severity = self.calculate_severity(diff_percentage.abs(), thresholds);

                    issues.push(RegressionIssue {
                        benchmark: benchmark_name.clone(),
                        baseline_score: baseline_stats.mean,
                        current_score: current_result.mean,
                        regression_percentage: diff_percentage,
                        severity,
                        statistical_significance: is_significant,
                        confidence_level: thresholds.confidence_level,
                    });

                    println!("⚠️  发现回归: {} (性能下降 {:.2}%)",
                        benchmark_name, diff_percentage.abs());
                }
            } else {
                println!("⚠️  警告: 基准测试 '{}' 在基线中未找到", benchmark_name);
            }
        }

        println!("✅ 回归检测完成，发现 {} 个问题", issues.len());

        Ok(issues)
    }

    /// 计算性能差异
    fn calculate_difference(
        &self,
        current: &BenchmarkResult,
        baseline: &BenchmarkStats,
    ) -> f64 {
        // 对于性能指标（如 ops/sec），值越高越好
        // 如果当前值小于基线值，则为性能回归（负值）
        // 如果当前值大于基线值，则为性能提升（正值）
        ((current.mean - baseline.mean) / baseline.mean) * 100.0
    }

    /// 执行统计检验
    fn statistical_test(
        &self,
        current: &BenchmarkResult,
        baseline: &BenchmarkStats,
    ) -> StatisticalTest {
        // 简化的 Welch's t-test 实现
        // 在实际应用中，应使用统计库进行精确计算

        let n1 = current.samples as f64;
        let n2 = baseline.samples as f64;
        let mean1 = current.mean;
        let mean2 = baseline.mean;
        let std1 = current.std_dev;
        let std2 = baseline.std_dev;

        // 计算 t 统计量
        let pooled_se = ((std1 * std1 / n1) + (std2 * std2 / n2)).sqrt();
        let t_stat = (mean1 - mean2) / pooled_se;

        // 简化的 p 值计算（实际应使用 t 分布表）
        let p_value = 2.0 * (1.0 - normal_cdf(t_stat.abs()));

        StatisticalTest {
            test_name: "welch_t_test".to_string(),
            p_value,
            is_significant: p_value < 0.05,
            confidence_level: self.confidence_level,
            effect_size: (mean1 - mean2).abs() / ((std1 + std2) / 2.0),
        }
    }

    /// 计算严重程度
    fn calculate_severity(
        &self,
        regression_percentage: f64,
        thresholds: &PerformanceThresholds,
    ) -> RegressionSeverity {
        if regression_percentage >= thresholds.critical_threshold {
            RegressionSeverity::Critical
        } else if regression_percentage >= thresholds.critical_threshold * 0.75 {
            RegressionSeverity::Major
        } else if regression_percentage >= thresholds.regression_threshold * 1.5 {
            RegressionSeverity::Minor
        } else {
            RegressionSeverity::Warning
        }
    }
}

/// 标准正态分布累积分布函数的近似
fn normal_cdf(x: f64) -> f64 {
    // 使用近似公式： Abramowitz and Stegun approximation
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989423 * (-0.5 * x * x).exp();
    let prob = 1.0 - d * t * (0.3193815 + t * (-0.3565638 + t * (1.781478 + t * (-1.821256 + t * 1.330274))));

    if x >= 0.0 {
        prob
    } else {
        1.0 - prob
    }
}

impl StatisticalTests {
    /// 创建新的统计检验器
    pub fn new() -> Self {
        Self
    }

    /// Welch's t-test
    pub fn welch_t_test(
        &self,
        sample1: &[f64],
        sample2: &[f64],
    ) -> Result<StatisticalTest, Box<dyn std::error::Error>> {
        if sample1.is_empty() || sample2.is_empty() {
            return Err("样本数据不能为空".into());
        }

        let n1 = sample1.len() as f64;
        let n2 = sample2.len() as f64;
        let mean1: f64 = sample1.iter().sum::<f64>() / n1;
        let mean2: f64 = sample2.iter().sum::<f64>() / n2;
        let var1: f64 = sample1.iter().map(|x| (x - mean1).powi(2)).sum::<f64>() / (n1 - 1.0);
        let var2: f64 = sample2.iter().map(|x| (x - mean2).powi(2)).sum::<f64>() / (n2 - 1.0);

        let pooled_se = (var1 / n1 + var2 / n2).sqrt();
        let t_stat = (mean1 - mean2) / pooled_se;

        // 自由度（Welch-Satterthwaite equation）
        let df = (var1 / n1 + var2 / n2).powi(2) /
            ((var1 / n1).powi(2) / (n1 - 1.0) + (var2 / n2).powi(2) / (n2 - 1.0));

        // 简化的 p 值计算
        let p_value = 2.0 * (1.0 - normal_cdf(t_stat.abs()));

        Ok(StatisticalTest {
            test_name: "welch_t_test".to_string(),
            p_value,
            is_significant: p_value < 0.05,
            confidence_level: 0.95,
            effect_size: (mean1 - mean2).abs() / ((var1.sqrt() + var2.sqrt()) / 2.0),
        })
    }

    /// ANOVA 检验
    pub fn anova_test(
        &self,
        samples: Vec<Vec<f64>>,
    ) -> Result<StatisticalTest, Box<dyn std::error::Error>> {
        if samples.len() < 2 {
            return Err("至少需要两个样本组".into());
        }

        let all_data: Vec<f64> = samples.iter().flatten().cloned().collect();
        let total_n = all_data.len() as f64;
        let total_mean: f64 = all_data.iter().sum::<f64>() / total_n;

        // 组间平方和
        let ss_between: f64 = samples.iter().map(|sample| {
            let group_mean: f64 = sample.iter().sum::<f64>() / sample.len() as f64;
            (group_mean - total_mean).powi(2) * sample.len() as f64
        }).sum();

        // 组内平方和
        let ss_within: f64 = samples.iter().map(|sample| {
            let group_mean: f64 = sample.iter().sum::<f64>() / sample.len() as f64;
            sample.iter().map(|x| (x - group_mean).powi(2)).sum::<f64>()
        }).sum();

        let df_between = samples.len() as f64 - 1.0;
        let df_within = total_n - samples.len() as f64;

        let ms_between = ss_between / df_between;
        let ms_within = ss_within / df_within;

        let f_stat = ms_between / ms_within;

        // 简化的 p 值计算（实际应使用 F 分布）
        let p_value = 1.0 - f_cdf(f_stat, df_between, df_within);

        Ok(StatisticalTest {
            test_name: "anova".to_string(),
            p_value,
            is_significant: p_value < 0.05,
            confidence_level: 0.95,
            effect_size: ss_between / (ss_between + ss_within),
        })
    }
}

/// F 分布累积分布函数的近似
fn f_cdf(f: f64, df1: f64, df2: f64) -> f64 {
    // 简化的 F 分布 CDF 近似
    // 实际应用中应使用统计库
    let x = (df1 * f) / (df1 * f + df2);
    beta_cdf(x, df1 / 2.0, df2 / 2.0)
}

/// Beta 分布累积分布函数的近似
fn beta_cdf(x: f64, a: f64, b: f64) -> f64 {
    // 简化的 Beta 分布 CDF
    // 实际应用中应使用统计库
    if x <= 0.0 {
        0.0
    } else if x >= 1.0 {
        1.0
    } else {
        // 非常简化的近似
        (x.powf(a) * (1.0 - x).powf(b)) / (a * beta_function(a, b))
    }
}

/// Beta 函数近似
fn beta_function(a: f64, b: f64) -> f64 {
    gamma_function(a) * gamma_function(b) / gamma_function(a + b)
}

/// Gamma 函数近似（斯特林公式）
fn gamma_function(z: f64) -> f64 {
    // 简化的 Gamma 函数实现
    // 实际应用中应使用更精确的实现
    (2.0 * std::f64::consts::PI).sqrt() * (z - 0.5).powf(z - 0.5) * (-z).exp()
}

impl ThresholdChecker {
    /// 创建新的阈值检查器
    pub fn new(thresholds: HashMap<String, PerformanceThresholds>) -> Self {
        Self { thresholds }
    }

    /// 检查阈值
    pub fn check_thresholds(
        &self,
        current: &PerformanceResults,
    ) -> Result<Vec<RegressionIssue>, Box<dyn std::error::Error>> {
        let mut issues = vec![];

        for (benchmark_name, result) in &current.benchmarks {
            if let Some(threshold) = self.thresholds.get(benchmark_name) {
                // 这里可以添加具体的阈值检查逻辑
                // 例如：检查 mean, p95, p99 等指标是否超过阈值
                let _ = result; // 避免未使用警告
            }
        }

        Ok(issues)
    }

    /// 添加阈值配置
    pub fn add_threshold(&mut self, benchmark_name: String, threshold: PerformanceThresholds) {
        self.thresholds.insert(benchmark_name, threshold);
    }

    /// 移除阈值配置
    pub fn remove_threshold(&mut self, benchmark_name: &str) {
        self.thresholds.remove(benchmark_name);
    }

    /// 列出所有阈值配置
    pub fn list_thresholds(&self) -> HashMap<String, PerformanceThresholds> {
        self.thresholds.clone()
    }
}

impl ReportGenerator {
    /// 创建新的报告生成器
    pub fn new(
        issues: Vec<RegressionIssue>,
        baseline_version: String,
        current_version: String,
    ) -> Self {
        Self {
            issues,
            baseline_version,
            current_version,
        }
    }

    /// 生成 JSON 报告
    pub fn generate_json_report(&self) -> Result<String, Box<dyn std::error::Error>> {
        let report = serde_json::json!({
            "summary": {
                "baseline_version": self.baseline_version,
                "current_version": self.current_version,
                "total_issues": self.issues.len(),
                "critical_issues": self.issues.iter().filter(|i| i.severity == RegressionSeverity::Critical).count(),
                "major_issues": self.issues.iter().filter(|i| i.severity == RegressionSeverity::Major).count(),
                "minor_issues": self.issues.iter().filter(|i| i.severity == RegressionSeverity::Minor).count(),
                "warning_issues": self.issues.iter().filter(|i| i.severity == RegressionSeverity::Warning).count(),
            },
            "issues": self.issues,
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "version": "1.0.0"
        });

        Ok(serde_json::to_string_pretty(&report)?)
    }

    /// 生成 HTML 报告
    pub fn generate_html_report(&self) -> Result<String, Box<dyn std::error::Error>> {
        let critical_count = self.issues.iter().filter(|i| i.severity == RegressionSeverity::Critical).count();
        let major_count = self.issues.iter().filter(|i| i.severity == RegressionSeverity::Major).count();
        let minor_count = self.issues.iter().filter(|i| i.severity == RegressionSeverity::Minor).count();
        let warning_count = self.issues.iter().filter(|i| i.severity == RegressionSeverity::Warning).count();

        let html = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Performance Regression Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; border-radius: 8px; margin-bottom: 20px; }}
        .summary {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin: 20px 0; }}
        .metric {{ background: #f8f9fa; padding: 20px; border-radius: 8px; text-align: center; }}
        .metric h3 {{ margin: 0 0 10px 0; color: #495057; }}
        .metric .value {{ font-size: 32px; font-weight: bold; }}
        .critical {{ color: #dc3545; }}
        .major {{ color: #fd7e14; }}
        .minor {{ color: #ffc107; }}
        .warning {{ color: #17a2b8; }}
        .issues {{ margin-top: 30px; }}
        .issue {{ background: #f8f9fa; padding: 15px; margin: 10px 0; border-radius: 8px; border-left: 4px solid #007bff; }}
        .severity-critical {{ border-left-color: #dc3545; }}
        .severity-major {{ border-left-color: #fd7e14; }}
        .severity-minor {{ border-left-color: #ffc107; }}
        .severity-warning {{ border-left-color: #17a2b8; }}
        table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #dee2e6; }}
        th {{ background: #e9ecef; font-weight: 600; }}
        .badge {{ display: inline-block; padding: 4px 8px; border-radius: 4px; font-size: 12px; font-weight: 500; }}
        .badge-critical {{ background: #dc3545; color: white; }}
        .badge-major {{ background: #fd7e14; color: white; }}
        .badge-minor {{ background: #ffc107; color: #212529; }}
        .badge-warning {{ background: #17a2b8; color: white; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Performance Regression Report</h1>
            <p>Baseline: {} → Current: {}</p>
        </div>

        <div class="summary">
            <div class="metric">
                <h3>Total Issues</h3>
                <div class="value">{}</div>
            </div>
            <div class="metric">
                <h3>Critical</h3>
                <div class="value critical">{}</div>
            </div>
            <div class="metric">
                <h3>Major</h3>
                <div class="value major">{}</div>
            </div>
            <div class="metric">
                <h3>Minor</h3>
                <div class="value minor">{}</div>
            </div>
            <div class="metric">
                <h3>Warning</h3>
                <div class="value warning">{}</div>
            </div>
        </div>

        <div class="issues">
            <h2>Regression Details</h2>
            <table>
                <thead>
                    <tr>
                        <th>Benchmark</th>
                        <th>Baseline</th>
                        <th>Current</th>
                        <th>Regression</th>
                        <th>Severity</th>
                        <th>Significance</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>
</body>
</html>
"#,
            self.baseline_version,
            self.current_version,
            self.issues.len(),
            critical_count,
            major_count,
            minor_count,
            warning_count,
            self.issues.iter().map(|issue| format!(
                "<tr><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}%</td><td><span class=\"badge badge-{:?}\">{:?}</span></td><td>{:.2}%</td></tr>",
                issue.benchmark,
                issue.baseline_score,
                issue.current_score,
                issue.regression_percentage.abs(),
                issue.severity,
                issue.severity,
                issue.confidence_level * 100.0
            )).collect::<Vec<_>>().join("\n")
        );

        Ok(html)
    }

    /// 保存报告
    pub fn save_report<P: AsRef<Path>>(&self, output_path: P, format: ReportFormat) -> Result<(), Box<dyn std::error::Error>> {
        let output_path = output_path.as_ref();
        let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        match format {
            ReportFormat::Json => {
                let json = self.generate_json_report()?;
                fs::write(output_path.with_extension("json"), json)?;
            }
            ReportFormat::Html => {
                let html = self.generate_html_report()?;
                fs::write(output_path.with_extension("html"), html)?;
            }
        }

        println!("📄 报告已保存到: {}", output_path.display());

        Ok(())
    }
}

/// 报告格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportFormat {
    Json,
    Html,
}

/// 性能回归检测器主结构
pub struct PerformanceRegressionDetector {
    baseline_manager: BaselineManager,
    analyzer: RegressionAnalyzer,
    threshold_checker: ThresholdChecker,
    statistical_tests: StatisticalTests,
}

/// 性能回归检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionDetectionResult {
    pub issues: Vec<RegressionIssue>,
    pub summary: DetectionSummary,
}

/// 检测摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionSummary {
    pub total_benchmarks: usize,
    pub issues_found: usize,
    pub critical_issues: usize,
    pub major_issues: usize,
    pub minor_issues: usize,
    pub warning_issues: usize,
    pub detection_time: Duration,
}

impl PerformanceRegressionDetector {
    /// 创建新的性能回归检测器
    pub fn new(
        baseline_path: &str,
        threshold_config_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut baseline_manager = BaselineManager::new();
        baseline_manager.load_baseline(baseline_path)?;

        // 加载阈值配置
        let threshold_content = fs::read_to_string(threshold_config_path)?;
        let thresholds: HashMap<String, PerformanceThresholds> = serde_json::from_str(&threshold_content)?;

        let analyzer = RegressionAnalyzer::new(thresholds.clone(), 0.95);
        let threshold_checker = ThresholdChecker::new(thresholds);
        let statistical_tests = StatisticalTests::new();

        Ok(Self {
            baseline_manager,
            analyzer,
            threshold_checker,
            statistical_tests,
        })
    }

    /// 检测回归
    pub async fn detect(
        &self,
        current_results: &PerformanceResults,
    ) -> Result<RegressionDetectionResult, Box<dyn std::error::Error>> {
        let start_time = Instant::now();

        let baseline = self.baseline_manager.get_current_baseline()
            .ok_or("没有可用的性能基线")?;

        let issues = self.analyzer.detect_regression(baseline, current_results)?;

        let detection_time = start_time.elapsed();

        let summary = DetectionSummary {
            total_benchmarks: current_results.benchmarks.len(),
            issues_found: issues.len(),
            critical_issues: issues.iter().filter(|i| i.severity == RegressionSeverity::Critical).count(),
            major_issues: issues.iter().filter(|i| i.severity == RegressionSeverity::Major).count(),
            minor_issues: issues.iter().filter(|i| i.severity == RegressionSeverity::Minor).count(),
            warning_issues: issues.iter().filter(|i| i.severity == RegressionSeverity::Warning).count(),
            detection_time,
        };

        Ok(RegressionDetectionResult { issues, summary })
    }

    /// 生成报告
    pub fn generate_report(
        &self,
        result: &RegressionDetectionResult,
        output_path: &str,
        format: ReportFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let baseline_version = self.baseline_manager.get_current_baseline()
            .map(|b| b.version.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let generator = ReportGenerator::new(
            result.issues.clone(),
            baseline_version,
            "current".to_string(),
        );

        generator.save_report(output_path, format)?;

        Ok(())
    }

    /// 更新基线
    pub fn update_baseline(
        &self,
        version: &str,
        results: &PerformanceResults,
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.baseline_manager.save_baseline(version, results, output_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_cdf() {
        assert!(normal_cdf(0.0) > 0.4 && normal_cdf(0.0) < 0.6);
        assert!(normal_cdf(1.96) > 0.95 && normal_cdf(1.96) < 0.98);
    }

    #[test]
    fn test_calculate_severity() {
        let thresholds = PerformanceThresholds {
            regression_threshold: 0.05,
            critical_threshold: 0.10,
            confidence_level: 0.95,
            min_samples: 100,
            statistical_test: "welch_t_test".to_string(),
        };

        let analyzer = RegressionAnalyzer::new(HashMap::new(), 0.95);

        assert_eq!(
            analyzer.calculate_severity(0.15, &thresholds),
            RegressionSeverity::Critical
        );
        assert_eq!(
            analyzer.calculate_severity(0.08, &thresholds),
            RegressionSeverity::Major
        );
        assert_eq!(
            analyzer.calculate_severity(0.06, &thresholds),
            RegressionSeverity::Minor
        );
        assert_eq!(
            analyzer.calculate_severity(0.04, &thresholds),
            RegressionSeverity::Warning
        );
    }
}
