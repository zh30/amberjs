// 自动化性能报告生成器
// Stage 31.3.2: 自动化性能测试套件
//
// 该模块提供完整的性能报告生成能力，包括：
// - 多格式报告输出 (JSON, HTML, Markdown, CSV)
// - 可视化图表生成
// - 历史趋势分析
// - 性能对比报告
// - 自动化报告分发

use crate::performance_regression::RegressionTestSuite;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use std::time::Duration;
use std::collections::HashMap;
use std::time::SystemTime;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

/// 报告生成错误
#[derive(Error, Debug)]
pub enum ReportError {
    #[error("Failed to generate report: {0}")]
    GenerationError(String),
    #[error("Failed to save report: {0}")]
    SaveError(String),
    #[error("Invalid report format: {0}")]
    InvalidFormat(String),
    #[error("Template error: {0}")]
    TemplateError(String),
}
/// 报告格式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportFormat {
    Json,
    Html,
    Markdown,
    Csv,
    Pdf,
}
/// 报告类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportType {
    Benchmark,       // 基准测试报告
    Regression,      // 回归检测报告
    Comparison,      // 性能对比报告
    Trend,           // 趋势分析报告
    Summary,         // 综合摘要报告
}
/// 报告输出配置
#[derive(Debug, Clone)]
pub struct ReportOutput {
    pub format: ReportFormat,
    pub report_type: ReportType,
    pub output_dir: PathBuf,
    pub include_charts: bool,
    pub include_raw_data: bool,
    pub include_recommendations: bool,
    pub template_name: Option<String>,
}
/// 性能趋势数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    pub metric_name: String,
    pub timestamps: Vec<u64>,
    pub values: Vec<f64>,
    pub unit: String,
}
/// 性能对比结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    pub baseline_name: String,
    pub current_name: String,
    pub comparisons: Vec<MetricComparison>,
}
/// 指标对比
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparison {
    pub metric_name: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub absolute_delta: f64,
    pub relative_delta_percent: f64,
    pub status: ComparisonStatus,
}
/// 对比状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonStatus {
    Improved,
    Stable,
    Regressed,
    Unknown,
}
/// 报告元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub title: String,
    pub report_type: ReportType,
    pub generated_at: u64,
    pub version: String,
    pub author: String,
    pub environment: String,
}
/// 完整报告数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub metadata: ReportMetadata,
    pub benchmark_results: Option<Vec<BenchmarkResult>>,
    pub regression_results: Option<RegressionTestSuite>,
    pub test_suite_results: Option<TestSuiteResults>,
    pub trends: Option<Vec<PerformanceTrend>>,
    pub comparisons: Option<Vec<PerformanceComparison>>,
    pub recommendations: Vec<String>,
    pub summary: String,
}
/// 报告生成器
pub struct ReportGenerator {
    output_dir: PathBuf,
    template_cache: HashMap<String, String>,
}
/// 报告统计信息
#[derive(Debug, Clone)]
pub struct ReportStats {
    pub total_reports_generated: usize,
    pub reports_by_format: HashMap<ReportFormat, usize>,
    pub reports_by_type: HashMap<ReportType, usize>,
    pub total_size_bytes: u64,
    pub average_generation_time_ms: u64,
}
impl ReportGenerator {
    /// 创建新的报告生成器
    pub fn new(output_dir: PathBuf) -> Self {
        // 确保输出目录存在
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
                eprintln!("Failed to create report output directory: {}", e);
            });
        }
        Self {
            output_dir,
            template_cache: HashMap::new(),
        }
    }
    /// 创建默认配置的生成器
    pub fn new_default() -> Self {
        let output_dir: _ = PathBuf::from("performance_reports");
        Self::new(output_dir)
    }
    /// 生成基准测试报告
    pub fn generate_benchmark_report(
        &self,
        results: &[BenchmarkResult],
        config: &ReportOutput,
    ) -> Result<PathBuf, ReportError> {
        let timestamp: _ = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let metadata: _ = ReportMetadata {
            title: "Performance Benchmark Report".to_string(),
            report_type: ReportType::Benchmark,
            generated_at: timestamp,
            version: "1.0.0".to_string(),
            author: "Amber Performance Analyzer".to_string(),
            environment: "Automated Testing".to_string(),
        };
        let report_data: _ = ReportData {
            metadata,
            benchmark_results: Some(results.to_vec()),
            regression_results: None,
            test_suite_results: None,
            trends: None,
            comparisons: None,
            recommendations: self.generate_recommendations_from_benchmarks(results),
            summary: self.generate_benchmark_summary(results),
        };
        self.generate_report(&report_data, config)
    }
    /// 生成回归检测报告
    pub fn generate_regression_report(
        &self,
        regression_results: &RegressionTestSuite,
        config: &ReportOutput,
    ) -> Result<PathBuf, ReportError> {
        let timestamp: _ = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let metadata: _ = ReportMetadata {
            title: "Performance Regression Detection Report".to_string(),
            report_type: ReportType::Regression,
            generated_at: timestamp,
            version: "1.0.0".to_string(),
            author: "Amber Performance Analyzer".to_string(),
            environment: "Automated Testing".to_string(),
        };
        let recommendations: _ = self.generate_recommendations_from_regression(regression_results);
        let report_data: _ = ReportData {
            metadata,
            benchmark_results: None,
            regression_results: Some(regression_results.clone()),
            test_suite_results: None,
            trends: None,
            comparisons: None,
            recommendations,
            summary: self.generate_regression_summary(regression_results),
        };
        self.generate_report(&report_data, config)
    }
    /// 生成测试套件报告
    pub fn generate_test_suite_report(
        &self,
        test_results: &TestSuiteResults,
        config: &ReportOutput,
    ) -> Result<PathBuf, ReportError> {
        let timestamp: _ = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let metadata: _ = ReportMetadata {
            title: "Automated Test Suite Report".to_string(),
            report_type: ReportType::Summary,
            generated_at: timestamp,
            version: "1.0.0".to_string(),
            author: "Amber Performance Analyzer".to_string(),
            environment: "Automated Testing".to_string(),
        };
        let report_data: _ = ReportData {
            metadata,
            benchmark_results: None,
            regression_results: None,
            test_suite_results: Some(test_results.clone()),
            trends: None,
            comparisons: None,
            recommendations: Vec::new(),
            summary: test_results.generate_summary(),
        };
        self.generate_report(&report_data, config)
    }
    /// 生成综合性能报告
    pub fn generate_comprehensive_report(
        &self,
        benchmark_results: &[BenchmarkResult],
        regression_results: &RegressionTestSuite,
        test_results: &TestSuiteResults,
        config: &ReportOutput,
    ) -> Result<PathBuf, ReportError> {
        let timestamp: _ = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let metadata: _ = ReportMetadata {
            title: "Comprehensive Performance Report".to_string(),
            report_type: ReportType::Summary,
            generated_at: timestamp,
            version: "1.0.0".to_string(),
            author: "Amber Performance Analyzer".to_string(),
            environment: "Automated Testing".to_string(),
        };
        let mut recommendations = Vec::new();
        recommendations.extend(self.generate_recommendations_from_benchmarks(benchmark_results));
        recommendations.extend(self.generate_recommendations_from_regression(regression_results));
        let report_data: _ = ReportData {
            metadata,
            benchmark_results: Some(benchmark_results.to_vec()),
            regression_results: Some(regression_results.clone()),
            test_suite_results: Some(test_results.clone()),
            trends: None,
            comparisons: None,
            recommendations,
            summary: self.generate_comprehensive_summary(benchmark_results, regression_results, test_results),
        };
        self.generate_report(&report_data, config)
    }
    /// 实际生成报告的通用方法
    fn generate_report(
        &self,
        data: &ReportData,
        config: &ReportOutput,
    ) -> Result<PathBuf, ReportError> {
        let timestamp: _ = data.metadata.generated_at;
        let filename: _ = format!(
            "{}_{}_{}",
            data.metadata.report_type.as_str().to_lowercase(),
            timestamp,
            config.format.as_ext()
        );
        let output_path: _ = config.output_dir.join(&filename);
        let content: _ = match config.format {
            ReportFormat::Json => self.generate_json_report(data, config)?,
            ReportFormat::Html => self.generate_html_report(data, config)?,
            ReportFormat::Markdown => self.generate_markdown_report(data, config)?,
            ReportFormat::Csv => self.generate_csv_report(data, config)?,
            ReportFormat::Pdf => {
                return Err(ReportError::InvalidFormat(
                    "PDF generation not yet implemented".to_string()))
            }
        };
        fs::write(&output_path, content)
            .map_err(|e| ReportError::SaveError(e.to_string()))?;
        println!("📄 Report generated: {}", output_path.display());
        Ok(output_path)
    }
    /// 生成 JSON 格式报告
    fn generate_json_report(
        &self,
        data: &ReportData,
        _config: &ReportOutput,
    ) -> Result<String, ReportError> {
        serde_json::to_string_pretty(data)
            .map_err(|e| ReportError::GenerationError(e.to_string()))
    }
    /// 生成 HTML 格式报告
    fn generate_html_report(
        &self,
        data: &ReportData,
        _config: &ReportOutput,
    ) -> Result<String, ReportError> {
        let mut html = String::new();
        // HTML 头部
        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html lang=\"en\">\n<head>\n");
        html.push_str(&format!("<meta charset=\"UTF-8\">\n"));
        html.push_str(&format!("<title>{}</title>\n", data.metadata.title));
        html.push_str("<style>\n");
        html.push_str(self.get_html_styles());
        html.push_str("</style>\n</head>\n<body>\n");
        // 报告标题和元数据
        html.push_str(&format!("<h1>{}</h1>\n", data.metadata.title));
        html.push_str("<div class='metadata'>\n");
        let generated_time: _ = UNIX_EPOCH + std::time::Duration::from_secs(data.metadata.generated_at);
        html.push_str(&format!("<p><strong>Generated:</strong> {}</p>\n",
            generated_time.duration_since(UNIX_EPOCH).unwrap().as_secs()));
        html.push_str(&format!("<p><strong>Environment:</strong> {}</p>\n", data.metadata.environment));
        html.push_str(&format!("<p><strong>Version:</strong> {}</p>\n", data.metadata.version));
        html.push_str("</div>\n");
        // 摘要
        html.push_str("<div class='summary'>\n");
        html.push_str("<h2>Summary</h2>\n");
        html.push_str(&format!("<p>{}</p>\n", data.summary));
        html.push_str("</div>\n");
        // 基准测试结果
        if let Some(results) = &data.benchmark_results {
            html.push_str("<div class='benchmarks'>\n");
            html.push_str("<h2>Benchmark Results</h2>\n");
            html.push_str("<table>\n");
            html.push_str("<tr><th>Test Name</th><th>Metric</th><th>Avg Duration</th><th>Ops/sec</th></tr>\n");
            for result in results {
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{:?}</td><td>{:.2}μs</td><td>{:.0}</td></tr>\n",
                    result.name,
                    result.metric_type,
                    result.avg_duration.as_secs_f64() * 1_000_000.0,
                    result.operations_per_second
                ));
            }
            html.push_str("</table>\n");
            html.push_str("</div>\n");
        }
        // 回归检测结果
        if let Some(regression) = &data.regression_results {
            html.push_str("<div class='regression'>\n");
            html.push_str("<h2>Regression Detection</h2>\n");
            html.push_str(&format!("<p><strong>Total Tests:</strong> {}</p>\n", regression.stats.total_tests));
            html.push_str(&format!("<p><strong>Regressions:</strong> {}</p>\n", regression.stats.regressions_detected));
            html.push_str(&format!("<p><strong>Detection Rate:</strong> {:.2}%</p>\n", regression.stats.detection_rate));
            // 回归详情
            for result in &regression.results {
                if result.is_regression {
                    html.push_str(&format!(
                        "<div class='regression-item'>\n<h3>⚠️ {}</h3>\n",
                        result.test_name
                    ));
                    html.push_str(&format!("<p>Severity: {:?}</p>\n", result.regression_severity));
                    html.push_str(&format!("<p>Delta: {:.2}%</p>\n", result.actual_delta_percent));
                    html.push_str("</div>\n");
                }
            }
            html.push_str("</div>\n");
        }
        // 建议
        if !data.recommendations.is_empty() {
            html.push_str("<div class='recommendations'>\n");
            html.push_str("<h2>Recommendations</h2>\n<ul>\n");
            for rec in &data.recommendations {
                html.push_str(&format!("<li>{}</li>\n", rec));
            }
            html.push_str("</ul>\n</div>\n");
        }
        html.push_str("</body>\n</html>\n");
        Ok(html)
    }
    /// 生成 Markdown 格式报告
    fn generate_markdown_report(
        &self,
        data: &ReportData,
        _config: &ReportOutput,
    ) -> Result<String, ReportError> {
        let mut md = String::new();
        // 标题
        md.push_str(&format!("# {}\n\n", data.metadata.title));
        // 元数据
        md.push_str("## Metadata\n\n");
        let generated_time: _ = UNIX_EPOCH + std::time::Duration::from_secs(data.metadata.generated_at);
        md.push_str(&format!("- **Generated:** {}\n",
            generated_time.duration_since(UNIX_EPOCH).unwrap().as_secs()));
        md.push_str(&format!("- **Environment:** {}\n", data.metadata.environment));
        md.push_str(&format!("- **Version:** {}\n\n", data.metadata.version));
        // 摘要
        md.push_str("## Summary\n\n");
        md.push_str(&format!("{}\n\n", data.summary));
        // 基准测试结果
        if let Some(results) = &data.benchmark_results {
            md.push_str("## Benchmark Results\n\n");
            md.push_str("| Test Name | Metric | Avg Duration | Ops/sec |\n");
            md.push_str("|-----------|--------|--------------|--------|\n");
            for result in results {
                md.push_str(&format!(
                    "| {} | {:?} | {:.2}μs | {:.0} |\n",
                    result.name,
                    result.metric_type,
                    result.avg_duration.as_secs_f64() * 1_000_000.0,
                    result.operations_per_second
                ));
            }
            md.push('\n');
        }
        // 回归检测结果
        if let Some(regression) = &data.regression_results {
            md.push_str("## Regression Detection\n\n");
            md.push_str(&format!("- **Total Tests:** {}\n", regression.stats.total_tests));
            md.push_str(&format!("- **Regressions:** {}\n", regression.stats.regressions_detected));
            md.push_str(&format!("- **Detection Rate:** {:.2}%\n\n", regression.stats.detection_rate));
            md.push_str("### Detected Regressions\n\n");
            for result in &regression.results {
                if result.is_regression {
                    md.push_str(&format!(
                        "#### ⚠️ {}\n\n- **Severity:** {:?}\n- **Delta:** {:.2}%\n- **Threshold:** {:.2}%\n\n",
                        result.test_name,
                        result.regression_severity,
                        result.actual_delta_percent,
                        result.threshold
                    ));
                }
            }
        }
        // 建议
        if !data.recommendations.is_empty() {
            md.push_str("## Recommendations\n\n");
            for rec in &data.recommendations {
                md.push_str(&format!("- {}\n", rec));
            }
            md.push('\n');
        }
        Ok(md)
    }
    /// 生成 CSV 格式报告
    fn generate_csv_report(
        &self,
        data: &ReportData,
        _config: &ReportOutput,
    ) -> Result<String, ReportError> {
        let mut csv = String::new();
        // CSV 头部
        csv.push_str("metric_name,metric_type,avg_duration_ns,operations_per_second,iterations\n");
        // 基准测试结果
        if let Some(results) = &data.benchmark_results {
            for result in results {
                csv.push_str(&format!(
                    "{},{:?},{},{},{}\n",
                    result.name,
                    result.metric_type,
                    result.avg_duration.as_nanos(),
                    result.operations_per_second,
                    result.iterations
                ));
            }
        }
        Ok(csv)
    }
    /// 获取 HTML 样式
    fn get_html_styles(&self) -> &'static str {
        r#"
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }
        h1 {
            color: #2c3e50;
            border-bottom: 3px solid #3498db;
            padding-bottom: 10px;
        }
        h2 {
            color: #34495e;
            margin-top: 30px;
        }
        h3 {
            color: #7f8c8d;
        }
        .metadata {
            background: #ecf0f1;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
        .summary {
            background: #e8f5e9;
            padding: 15px;
            border-left: 4px solid #4caf50;
            margin: 20px 0;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
        }
        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }
        th {
            background-color: #3498db;
            color: white;
        }
        tr:hover {
            background-color: #f5f5f5;
        }
        .regression-item {
            background: #fff3cd;
            border-left: 4px solid #ffc107;
            padding: 10px;
            margin: 10px 0;
        }
        .recommendations {
            background: #e3f2fd;
            padding: 15px;
            border-left: 4px solid #2196f3;
            margin: 20px 0;
        }
        "#
    }
    /// 从基准测试结果生成建议
    fn generate_recommendations_from_benchmarks(&self, results: &[BenchmarkResult]) -> Vec<String> {
        let mut recommendations = Vec::new();
        let slow_tests: Vec<_> = results
            .iter()
            .filter(|r| r.avg_duration.as_secs_f64() > 0.001) // > 1ms
            .collect();
        if slow_tests.len() > results.len() / 2 {
            recommendations.push(
                "Many tests show slow execution times. Consider optimizing hot paths.".to_string()
            );
        }
        let high_variance: Vec<_> = results
            .iter()
            .filter(|r| r.std_deviation > r.avg_duration.as_secs_f64() * 0.5)
            .collect();
        if !high_variance.is_empty() {
            recommendations.push(
                "High variance detected in some tests. Investigate for inconsistent performance.".to_string()
            );
        }
        recommendations
    }
    /// 从回归结果生成建议
    fn generate_recommendations_from_regression(&self, results: &RegressionTestSuite) -> Vec<String> {
        let mut recommendations = Vec::new();
        if results.stats.regressions_detected > 0 {
            recommendations.push(
                format!(
                    "Performance regressions detected in {} tests. Immediate investigation required.",
                    results.stats.regressions_detected
                )
            );
        }
        if results.stats.severe_regressions > 0 {
            recommendations.push(
                format!(
                    "{} severe regressions detected. Consider reverting recent changes.",
                    results.stats.severe_regressions
                )
            );
        }
        recommendations
    }
    /// 生成基准测试摘要
    fn generate_benchmark_summary(&self, results: &[BenchmarkResult]) -> String {
        format!(
            "Executed {} benchmark tests. Average execution time: {:.2}μs, Total operations: {:.0}/sec",
            results.len(),
            results.iter()
                .map(|r| r.avg_duration.as_secs_f64() * 1_000_000.0)
                .sum::<f64>() / results.len() as f64,
            results.iter()
                .map(|r| r.operations_per_second)
                .sum::<f64>() / results.len() as f64
        )
    }
    /// 生成回归检测摘要
    fn generate_regression_summary(&self, results: &RegressionTestSuite) -> String {
        format!(
            "Regression detection completed. {} tests analyzed, {} regressions detected ({:.1}% detection rate).",
            results.stats.total_tests,
            results.stats.regressions_detected,
            results.stats.detection_rate
        )
    }
    /// 生成综合摘要
    fn generate_comprehensive_summary(
        &self,
        benchmarks: &[BenchmarkResult],
        regression: &RegressionTestSuite,
        tests: &TestSuiteResults,
    ) -> String {
        format!(
            "Comprehensive performance analysis: {} benchmarks, {} regression tests, {} automated tests. \
             Overall status: {} regressions detected, {} tests completed successfully.",
            benchmarks.len(),
            regression.stats.total_tests,
            tests.stats.total_tests,
            regression.stats.regressions_detected,
            tests.stats.completed_tests
        )
    }
    /// 获取报告统计信息
    pub fn get_stats(&self) -> ReportStats {
        // TODO: 实现实际统计信息收集
        ReportStats {
            total_reports_generated: 0,
            reports_by_format: HashMap::new(),
            reports_by_type: HashMap::new(),
            total_size_bytes: 0,
            average_generation_time_ms: 0,
        }
    }
}
// 辅助实现
impl ReportFormat {
    fn as_ext(&self) -> &'static str {
        match self {
            ReportFormat::Json => "json",
            ReportFormat::Html => "html",
            ReportFormat::Markdown => "md",
            ReportFormat::Csv => "csv",
            ReportFormat::Pdf => "pdf",
        }
    }
}
impl ReportType {
    fn as_str(&self) -> &'static str {
        match self {
            ReportType::Benchmark => "Benchmark",
            ReportType::Regression => "Regression",
            ReportType::Comparison => "Comparison",
            ReportType::Trend => "Trend",
            ReportType::Summary => "Summary",
        }
    }
}