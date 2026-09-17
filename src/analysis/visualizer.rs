// Performance visualization module
//
// This module provides tools to generate visualizations of performance data,
// including charts, graphs, and HTML reports.

use crate::analysis::bottleneck_detector::{Bottleneck, BottleneckSeverity};
use crate::performance_analyzer::PerformanceReport;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Chart types for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    /// Line chart for time series data
    LineChart,
    /// Bar chart for categorical data
    BarChart,
    /// Pie chart for percentage data
    PieChart,
    /// Histogram for distribution data
    Histogram,
    /// Scatter plot for correlation data
    ScatterPlot,
    /// Heatmap for matrix data
    Heatmap,
}
/// Output format for visualizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    /// HTML with embedded JavaScript
    HTML,
    /// JSON data
    JSON,
    /// SVG graphics
    SVG,
    /// PNG image
    PNG,
    /// Markdown report
    Markdown,
}
/// Visualization configuration
#[derive(Debug, Clone)]
pub struct VisualizationConfig {
    /// Width of the chart in pixels
    pub width: u32,
    /// Height of the chart in pixels
    pub height: u32,
    /// Whether to show grid lines
    pub show_grid: bool,
    /// Whether to show legend
    pub show_legend: bool,
    /// Color theme
    pub theme: String,
    /// Output format
    pub output_format: OutputFormat,
}
impl Default for VisualizationConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            show_grid: true,
            show_legend: true,
            theme: "default".to_string(),
            output_format: OutputFormat::HTML,
        }
    }
}
/// Performance visualizer
pub struct PerformanceVisualizer {
    config: VisualizationConfig,
}
impl PerformanceVisualizer {
    /// Create a new performance visualizer with default configuration
    pub fn new() -> Self {
        Self {
            config: VisualizationConfig::default(),
        }
    }
    /// Create a new performance visualizer with custom configuration
    pub fn with_config(config: VisualizationConfig) -> Self {
        Self { config }
    }
    /// Generate an HTML performance report
    pub fn generate_html_report(
        &self,
        report: &PerformanceReport,
        bottlenecks: &[Bottleneck],
        suggestions: &[OptimizationSuggestion],
    ) -> String {
        let mut html = String::new();
        // HTML header
        html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Amber Performance Report</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@3.9.1/dist/chart.min.js"></script>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background: #f5f5f5;
        }
        .container {
            background: white;
            border-radius: 8px;
            padding: 30px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin-bottom: 20px;
        }
        h1 {
            color: #333;
            border-bottom: 3px solid #4CAF50;
            padding-bottom: 10px;
        }
        h2 {
            color: #555;
            margin-top: 30px;
        }
        .metric {
            display: inline-block;
            background: #f0f0f0;
            padding: 15px;
            margin: 10px;
            border-radius: 5px;
            min-width: 200px;
        }
        .metric-value {
            font-size: 2em;
            font-weight: bold;
            color: #4CAF50;
        }
        .metric-label {
            font-size: 0.9em;
            color: #666;
            margin-top: 5px;
        }
        .chart-container {
            position: relative;
            height: 400px;
            margin: 20px 0;
        }
        .bottleneck {
            border-left: 4px solid #ff9800;
            padding: 15px;
            margin: 10px 0;
            background: #fff3e0;
            border-radius: 4px;
        }
        .bottleneck.critical {
            border-left-color: #f44336;
            background: #ffebee;
        }
        .bottleneck.high {
            border-left-color: #ff9800;
            background: #fff3e0;
        }
        .bottleneck.medium {
            border-left-color: #ffc107;
            background: #fffde7;
        }
        .bottleneck.low {
            border-left-color: #4CAF50;
            background: #e8f5e9;
        }
        .suggestion {
            border: 1px solid #ddd;
            padding: 15px;
            margin: 10px 0;
            border-radius: 5px;
            background: #fafafa;
        }
        .priority {
            display: inline-block;
            padding: 3px 8px;
            border-radius: 3px;
            font-size: 0.85em;
            font-weight: bold;
            color: white;
        }
        .priority.critical { background: #f44336; }
        .priority.high { background: #ff9800; }
        .priority.medium { background: #ffc107; color: #333; }
        .priority.low { background: #4CAF50; }
        .code-example {
            background: #263238;
            color: #aed581;
            padding: 15px;
            border-radius: 5px;
            overflow-x: auto;
            font-family: 'Courier New', monospace;
            margin: 10px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Amber Performance Report</h1>
        <p>Generated at: "#);
        // Add timestamp
        html.push_str(&chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
        html.push_str(r#"</p>
    </div>
    <div class="container">
        <h2>📊 Key Metrics</h2>
        <div class="metric">
            <div class="metric-value">"#);
        html.push_str(&format!("{:.2}", report.average_time_ms));
        html.push_str(r#"</div>
            <div class="metric-label">Average Execution Time (ms)</div>
        </div>
        <div class="metric">
            <div class="metric-value">"#);
        html.push_str(&format!("{:.1}", report.cache_hit_rate));
        html.push_str(r#"%</div>
            <div class="metric-label">Cache Hit Rate</div>
        </div>
        <div class="metric">
            <div class="metric-value">"#);
        html.push_str(&format!("{}", report.total_executions));
        html.push_str(r#"</div>
            <div class="metric-label">Total Executions</div>
        </div>
        <div class="metric">
            <div class="metric-value">"#);
        html.push_str(&format!("{:.2}", report.min_time_ms));
        html.push_str(r#"</div>
            <div class="metric-label">Min Time (ms)</div>
        </div>
        <div class="metric">
            <div class="metric-value">"#);
        html.push_str(&format!("{:.2}", report.max_time_ms));
        html.push_str(r#"</div>
            <div class="metric-label">Max Time (ms)</div>
        </div>
    </div>
    <div class="container">
        <h2>📈 Performance Timeline</h2>
        <div class="chart-container">
            <canvas id="performanceChart"></canvas>
        </div>
    </div>
    <div class="container">
        <h2>⚠️ Detected Bottlenecks</h2>"#);
        // Add bottlenecks
        if bottlenecks.is_empty() {
            html.push_str("<p>✅ No bottlenecks detected!</p>");
        } else {
            for bottleneck in bottlenecks {
                html.push_str(&format!(
                    r#"<div class="bottleneck {severity_class}">
                        <h3>{}</h3>
                        <p><strong>Type:</strong> {:?}</p>
                        <p><strong>Severity:</strong> {}</p>
                        <p><strong>Suggestion:</strong> {}</p>
                    </div>"#,
                    bottleneck.description,
                    bottleneck.bottleneck_type,
                    self.severity_to_string(&bottleneck.severity),
                    bottleneck.suggestion,
                    severity_class = match bottleneck.severity {
                        BottleneckSeverity::Critical => "critical",
                        BottleneckSeverity::High => "high",
                        BottleneckSeverity::Medium => "medium",
                        BottleneckSeverity::Low => "low",
                        BottleneckSeverity::Info => "low",
                    }
                ));
            }
        }
        html.push_str(r#"
    </div>
    <div class="container">
        <h2>💡 Optimization Suggestions</h2>"#);
        // Add suggestions
        if suggestions.is_empty() {
            html.push_str("<p>✅ No suggestions at this time.</p>");
        } else {
            for suggestion in suggestions {
                html.push_str(&format!(
                    r#"<div class="suggestion">
                        <h3>{}</h3>
                        <p><strong>Category:</strong> {:?}</p>
                        <p><strong>Priority:</strong> <span class="priority {priority_class}">{}</span></p>
                        <p><strong>Description:</strong> {}</p>
                        <p><strong>Estimated Improvement:</strong> {}</p>
                        <p><strong>Implementation Effort:</strong> {}</p>
                        <h4>Implementation Steps:</h4>
                        <ol>"#,
                    suggestion.title,
                    suggestion.category,
                    self.priority_to_string(&suggestion.priority),
                    suggestion.description,
                    suggestion.estimated_improvement,
                    suggestion.implementation_effort,
                    priority_class = match suggestion.priority {
                        crate::analysis::optimizer::OptimizationPriority::Critical => "critical",
                        crate::analysis::optimizer::OptimizationPriority::High => "high",
                        crate::analysis::optimizer::OptimizationPriority::Medium => "medium",
                        crate::analysis::optimizer::OptimizationPriority::Low => "low",
                    }
                ));
                for step in &suggestion.steps {
                    html.push_str(&format!("<li>{}</li>", step));
                }
                html.push_str("</ol>");
                if !suggestion.code_examples.is_empty() {
                    html.push_str("<h4>Code Example:</h4>");
                    for example in &suggestion.code_examples {
                        html.push_str(&format!(
                            r#"<div class="code-example"><pre>{}</pre></div>"#,
                            example
                        ));
                    }
                }
                html.push_str("</div>");
            }
        }
        html.push_str(r#"
    </div>
    <div class="container">
        <h2>📊 Cache Hit Rate Distribution</h2>
        <div class="chart-container">
            <canvas id="cacheChart"></canvas>
        </div>
    </div>
    <script>
        // Performance Timeline Chart
        const ctx1 = document.getElementById('performanceChart').getContext('2d');
        new Chart(ctx1, {
            type: 'line',
            data: {
                labels: ["#);
        // Add sample data
        for i in 0..report.total_executions.min(20) {
            if i > 0 {
                html.push_str(", ");
            }
            html.push_str(&format!("{}", i + 1));
        }
        html.push_str(r#"],
                datasets: [{
                    label: 'Execution Time (ms)',
                    data: ["#);
        // Add sample execution times
        for i in 0..report.total_executions.min(20) {
            if i > 0 {
                html.push_str(", ");
            }
            // Generate sample data based on the report
            let variance: _ = (report.max_time_ms - report.min_time_ms) / 2.0;
            let time: _ = report.average_time_ms + (i as f64 - 10.0) * variance / 10.0;
            html.push_str(&format!("{:.2}", time.max(report.min_time_ms).min(report.max_time_ms)));
        }
        html.push_str(r#"],
                    borderColor: '#4CAF50',
                    backgroundColor: 'rgba(76, 175, 80, 0.1)',
                    tension: 0.4
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                scales: {
                    y: {
                        beginAtZero: true,
                        title: {
                            display: true,
                            text: 'Time (ms)'
                        }
                    },
                    x: {
                        title: {
                            display: true,
                            text: 'Execution #'
                        }
                    }
                }
            }
        });
        // Cache Hit Rate Chart
        const ctx2 = document.getElementById('cacheChart').getContext('2d');
        new Chart(ctx2, {
            type: 'doughnut',
            data: {
                labels: ['Cache Hits', 'Cache Misses'],
                datasets: [{
                    data: ["#);
        html.push_str(&format!("{:.2}, {:.2}", report.cache_hit_rate, 100.0 - report.cache_hit_rate));
        html.push_str(r#"],
                    backgroundColor: [
                        '#4CAF50',
                        '#ff9800'
                    ]
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        position: 'bottom'
                    }
                }
            }
        });
    </script>
</body>
</html>"#);
        html
    }
    /// Generate a Markdown report
    pub fn generate_markdown_report(
        &self,
        report: &PerformanceReport,
        bottlenecks: &[Bottleneck],
        suggestions: &[OptimizationSuggestion],
    ) -> String {
        let mut md = String::new();
        // Title
        md.push_str("# 🚀 Amber Performance Report\n\n");
        md.push_str(&format!("**Generated at:** {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        // Key Metrics
        md.push_str("## 📊 Key Metrics\n\n");
        md.push_str(&format!("- **Average Execution Time:** {:.2} ms\n", report.average_time_ms));
        md.push_str(&format!("- **Cache Hit Rate:** {:.1}%\n", report.cache_hit_rate));
        md.push_str(&format!("- **Total Executions:** {}\n", report.total_executions));
        md.push_str(&format!("- **Min Time:** {:.2} ms\n", report.min_time_ms));
        md.push_str(&format!("- **Max Time:** {:.2} ms\n\n", report.max_time_ms));
        // Bottlenecks
        md.push_str("## ⚠️ Detected Bottlenecks\n\n");
        if bottlenecks.is_empty() {
            md.push_str("✅ No bottlenecks detected!\n\n");
        } else {
            for bottleneck in bottlenecks {
                md.push_str(&format!(
                    "### {}\n**Type:** {:?}\n**Severity:** {}\n{}\n\n",
                    bottleneck.description,
                    bottleneck.bottleneck_type,
                    self.severity_to_string(&bottleneck.severity),
                    bottleneck.suggestion
                ));
            }
        }
        // Suggestions
        md.push_str("## 💡 Optimization Suggestions\n\n");
        if suggestions.is_empty() {
            md.push_str("✅ No suggestions at this time.\n\n");
        } else {
            for suggestion in suggestions {
                md.push_str(&format!(
                    "### {}\n**Category:** {:?}\n**Priority:** {}\n**Description:** {}\n**Estimated Improvement:** {}\n**Implementation Effort:** {}\n\n",
                    suggestion.title,
                    suggestion.category,
                    self.priority_to_string(&suggestion.priority),
                    suggestion.description,
                    suggestion.estimated_improvement,
                    suggestion.implementation_effort
                ));
                if !suggestion.steps.is_empty() {
                    md.push_str("**Implementation Steps:**\n");
                    for step in &suggestion.steps {
                        md.push_str(&format!("1. {}\n", step));
                    }
                    md.push('\n');
                }
                if !suggestion.code_examples.is_empty() {
                    md.push_str("**Code Example:**\n```\n");
                    for example in &suggestion.code_examples {
                        md.push_str(&format!("{}\n", example));
                    }
                    md.push_str("```\n\n");
                }
            }
        }
        md
    }
    /// Save report to file
    pub fn save_report(&self, report: &str, filename: &str) -> std::io::Result<()> {
        let mut file = File::create(filename)?;
        file.write_all(report.as_bytes())?;
        Ok(())
    }
    /// Convert severity to string
    fn severity_to_string(&self, severity: &BottleneckSeverity) -> String {
        match severity {
            BottleneckSeverity::Critical => "Critical".to_string(),
            BottleneckSeverity::High => "High".to_string(),
            BottleneckSeverity::Medium => "Medium".to_string(),
            BottleneckSeverity::Low => "Low".to_string(),
            BottleneckSeverity::Info => "Info".to_string(),
        }
    }
    /// Convert priority to string
    fn priority_to_string(&self, priority: &crate::analysis::optimizer::OptimizationPriority) -> String {
        match priority {
            crate::analysis::optimizer::OptimizationPriority::Critical => "Critical".to_string(),
            crate::analysis::optimizer::OptimizationPriority::High => "High".to_string(),
            crate::analysis::optimizer::OptimizationPriority::Medium => "Medium".to_string(),
            crate::analysis::optimizer::OptimizationPriority::Low => "Low".to_string(),
        }
    }
}
impl Default for PerformanceVisualizer {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_visualizer_creation() {
        let visualizer: _ = PerformanceVisualizer::new();
        assert_eq!(visualizer.config.width, 800);
        assert_eq!(visualizer.config.height, 600);
    }
    #[test]
    fn test_generate_html_report() {
        let visualizer: _ = PerformanceVisualizer::new();
        let report: _ = PerformanceReport {
            total_executions: 10,
            average_time_ms: 15.0,
            min_time_ms: 5.0,
            max_time_ms: 30.0,
            cache_hit_rate: 70.0,
            total_code_executed: 1000,
        };
        let bottlenecks: _ = vec![
            crate::analysis::bottleneck_detector::Bottleneck {
                bottleneck_type: crate::analysis::bottleneck_detector::BottleneckType::SlowExecution,
                severity: BottleneckSeverity::High,
                description: "Slow execution detected".to_string(),
                affected_metrics: vec!["average_time_ms".to_string()],
                suggestion: "Optimize code".to_string(),
                code_location: None,
            }
        ];
        let suggestions: _ = vec![];
        let html: _ = visualizer.generate_html_report(&report, &bottlenecks, &suggestions);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Amber Performance Report"));
        assert!(html.contains("15.00"));
    }
    #[test]
    fn test_generate_markdown_report() {
        let visualizer: _ = PerformanceVisualizer::new();
        let report: _ = PerformanceReport {
            total_executions: 10,
            average_time_ms: 15.0,
            min_time_ms: 5.0,
            max_time_ms: 30.0,
            cache_hit_rate: 70.0,
            total_code_executed: 1000,
        };
        let bottlenecks: _ = vec![];
        let suggestions: _ = vec![];
        let md: _ = visualizer.generate_markdown_report(&report, &bottlenecks, &suggestions);
        assert!(md.contains("# 🚀 Amber Performance Report"));
        assert!(md.contains("Average Execution Time"));
    }
    #[test]
    fn test_save_report() {
        let visualizer: _ = PerformanceVisualizer::new();
        let report: _ = "# Test Report\n";
        let result: _ = visualizer.save_report(report, "/tmp/test_report.html");
        assert!(result.is_ok());
    }
    #[test]
    fn test_severity_to_string() {
        let visualizer: _ = PerformanceVisualizer::new();
        assert_eq!(visualizer.severity_to_string(&BottleneckSeverity::Critical), "Critical");
        assert_eq!(visualizer.severity_to_string(&BottleneckSeverity::High), "High");
        assert_eq!(visualizer.severity_to_string(&BottleneckSeverity::Medium), "Medium");
        assert_eq!(visualizer.severity_to_string(&BottleneckSeverity::Low), "Low");
    }
}