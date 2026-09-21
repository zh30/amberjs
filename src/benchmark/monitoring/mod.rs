// 性能监控模块
//
// 提供实时性能监控和仪表板功能

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use super::{BenchmarkResult, MetricType, Runtime};
use std::time::{Duration, Instant};

/// 实时监控器
#[derive(Debug)]
pub struct RealTimeMonitor {
    /// 指标收集器
    pub metrics_collector: Arc<RwLock<MetricsCollector>>,
    /// 监控配置
    pub config: MonitorConfig,
}
impl RealTimeMonitor {
    /// 创建新的实时监控器
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            metrics_collector: Arc::new(Mutex::new(MetricsCollector::new()))
            config,
        }
    }
    /// 启动监控
    pub async fn start(&self) -> Result<(), super::BenchmarkError> {
        let metrics_collector: _ = self.metrics_collector.clone();
        // 启动定期指标收集
        let interval: _ = self.config.collection_interval;
        let mut interval_timer = tokio::time::interval(interval);
        tokio::spawn(async move {
            loop {
                interval_timer.tick().await;
                let mut collector = metrics_collector.write().await;
                collector.collect_system_metrics().await;
            }
        });
        Ok(())
    }
    /// 记录基准测试结果
    pub async fn record_benchmark_result(&self, result: &BenchmarkResult) {
        let mut collector = self.metrics_collector.write().await;
        collector.record_benchmark_result(result).await;
    }
    /// 获取当前指标
    pub async fn get_current_metrics(&self) -> CurrentMetrics {
        let collector: _ = self.metrics_collector.read().await;
        collector.get_current_metrics().await
    }
}
/// 监控配置
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// 指标收集间隔
    pub collection_interval: Duration,
    /// 最大历史记录数
    pub max_history_size: usize,
    /// 启用详细指标
    pub enable_detailed_metrics: bool,
}
impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_millis(100),
            max_history_size: 1000,
            enable_detailed_metrics: true,
        }
    }
}
impl MonitorConfig {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }
    /// 设置收集间隔
    pub fn collection_interval(mut self, interval: Duration) -> Self {
        self.collection_interval = interval;
        self
    }
    /// 设置最大历史记录数
    pub fn max_history_size(mut self, size: usize) -> Self {
        self.max_history_size = size;
        self
    }
    /// 启用详细指标
    pub fn enable_detailed_metrics(mut self, enable: bool) -> Self {
        self.enable_detailed_metrics = enable;
        self
    }
}
/// 指标收集器
#[derive(Debug, Default)]
pub struct MetricsCollector {
    /// 系统指标
    pub system_metrics: SystemMetrics,
    /// 基准测试指标
    pub benchmark_metrics: Vec<BenchmarkMetrics>,
    /// 历史记录
    pub history: Vec<HistoricalMetrics>,
    /// 收集时间
    pub last_collection: Instant,
}
impl MetricsCollector {
    /// 创建新的指标收集器
    pub fn new() -> Self {
        Self::default()
    }
    /// 收集系统指标
    pub async fn collect_system_metrics(&mut self) {
        // 收集 CPU 使用率
        self.system_metrics.cpu_usage = self.get_cpu_usage().await;
        // 收集内存使用情况
        self.system_metrics.memory_usage = self.get_memory_usage().await;
        // 收集磁盘 I/O
        self.system_metrics.disk_io = self.get_disk_io().await;
        // 收集网络 I/O
        self.system_metrics.network_io = self.get_network_io().await;
        self.last_collection = Instant::now();
    }
    /// 记录基准测试结果
    pub async fn record_benchmark_result(&mut self, result: &BenchmarkResult) {
        let metrics: _ = BenchmarkMetrics {
            test_name: result.name.clone(),
            runtime: result.runtime,
            execution_time: result.average_duration(),
            throughput: result.throughput(),
            memory_usage: result.metrics.memory_usage_bytes,
            cpu_usage: result.metrics.cpu_usage_percent,
            timestamp: Instant::now(),
        };
        self.benchmark_metrics.push(metrics);
        // 限制历史记录大小
        if self.benchmark_metrics.len() > 1000 {
            self.benchmark_metrics.remove(0);
        }
    }
    /// 获取当前指标
    pub async fn get_current_metrics(&self) -> CurrentMetrics {
        CurrentMetrics {
            system: self.system_metrics.clone(),
            latest_benchmarks: self.benchmark_metrics
                .iter()
                .rev()
                .take(10)
                .cloned()
                .collect(),
            collection_time: self.last_collection.elapsed(),
        }
    }
    /// 获取 CPU 使用率
    async fn get_cpu_usage(&self) -> f64 {
        // 简化实现 - 实际应该使用系统 API
        50.0 // 模拟值
    }
    /// 获取内存使用情况
    async fn get_memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            total: 8 * 1024 * 1024 * 1024, // 8GB
            used: 4 * 1024 * 1024 * 1024, // 4GB
            available: 4 * 1024 * 1024 * 1024, // 4GB
        }
    }
    /// 获取磁盘 I/O
    async fn get_disk_io(&self) -> DiskIo {
        DiskIo {
            read_bytes: 1024 * 1024,
            write_bytes: 512 * 1024,
            read_ops: 100,
            write_ops: 50,
        }
    }
    /// 获取网络 I/O
    async fn get_network_io(&self) -> NetworkIo {
        NetworkIo {
            bytes_sent: 1024 * 1024,
            bytes_received: 2 * 1024 * 1024,
            packets_sent: 1000,
            packets_received: 2000,
        }
    }
}
/// 当前指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentMetrics {
    /// 系统指标
    pub system: SystemMetrics,
    /// 最新基准测试结果
    pub latest_benchmarks: Vec<BenchmarkMetrics>,
    /// 收集时间
    pub collection_time: Duration,
}
/// 系统指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemMetrics {
    /// CPU 使用率 (百分比)
    pub cpu_usage: f64,
    /// 内存使用情况
    pub memory_usage: MemoryUsage,
    /// 磁盘 I/O
    pub disk_io: DiskIo,
    /// 网络 I/O
    pub network_io: NetworkIo,
    /// 负载平均值
    pub load_average: (f64, f64, f64),
    /// 进程数
    pub process_count: u32,
    /// 线程数
    pub thread_count: u32,
}
/// 内存使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// 总内存 (字节)
    pub total: u64,
    /// 已用内存 (字节)
    pub used: u64,
    /// 可用内存 (字节)
    pub available: u64,
}
/// 磁盘 I/O
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIo {
    /// 读取字节数
    pub read_bytes: u64,
    /// 写入字节数
    pub write_bytes: u64,
    /// 读取操作数
    pub read_ops: u64,
    /// 写入操作数
    pub write_ops: u64,
}
/// 网络 I/O
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIo {
    /// 发送字节数
    pub bytes_sent: u64,
    /// 接收字节数
    pub bytes_received: u64,
    /// 发送包数
    pub packets_sent: u64,
    /// 接收包数
    pub packets_received: u64,
}
/// 基准测试指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    /// 测试名称
    pub test_name: String,
    /// 运行时类型
    pub runtime: Runtime,
    /// 执行时间
    pub execution_time: Duration,
    /// 吞吐量
    pub throughput: f64,
    /// 内存使用
    pub memory_usage: u64,
    /// CPU 使用率
    pub cpu_usage: f64,
    /// 时间戳
    pub timestamp: Instant,
}
/// 历史指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMetrics {
    /// 时间戳
    pub timestamp: Instant,
    /// 系统指标
    pub system_metrics: SystemMetrics,
    /// 基准测试指标
    pub benchmark_metrics: Vec<BenchmarkMetrics>,
}
/// 性能仪表板
#[derive(Debug)]
pub struct PerformanceDashboard {
    /// 监控器
    pub monitor: RealTimeMonitor,
}
impl PerformanceDashboard {
    /// 创建新的性能仪表板
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            monitor: RealTimeMonitor::new(config),
        }
    }
    /// 启动仪表板
    pub async fn start(&self) -> Result<(), super::BenchmarkError> {
        self.monitor.start().await?;
        Ok(())
    }
    /// 生成 HTML 报告
    pub async fn generate_html_report(&self) -> Result<String, super::BenchmarkError> {
        let metrics: _ = self.monitor.get_current_metrics().await;
        let html: _ = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Amber Performance Dashboard</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .metric {{ margin: 10px 0; padding: 10px; border: 1px solid #ccc; }}
        .metric h3 {{ margin: 0 0 10px 0; }}
        .value {{ font-size: 1.2em; font-weight: bold; }}
    </style>
</head>
<body>
    <h1>Amber Performance Dashboard</h1>
    <div class="metric">
        <h3>CPU Usage</h3>
        <div class="value">{:.2}%</div>
    </div>
    <div class="metric">
        <h3>Memory Usage</h3>
        <div class="value">{} / {}</div>
    </div>
    <div class="metric">
        <h3>Collection Time</h3>
        <div class="value">{{:?}}</div>
    </div>
    <h2>Latest Benchmarks</h2>
    <table>
        <tr>
            <th>Test Name</th>
            <th>Runtime</th>
            <th>Execution Time</th>
            <th>Throughput</th>
        </tr>
        {}
    </table>
</body>
</html>
"#,
            metrics.system.cpu_usage,
            format_bytes(metrics.system.memory_usage.used),
            format_bytes(metrics.system.memory_usage.total),
            metrics.collection_time,
            metrics.latest_benchmarks
                .iter()
                .map(|bm| format!(
                    "<tr><td>{}</td><td>{}</td><td>{{:?}}</td><td>{:.2}</td></tr>",
                    bm.test_name,
                    bm.runtime,
                    bm.execution_time,
                    bm.throughput
                ))
                .collect::<Vec<_>()
                .join("\n")
        );
        Ok(html)
    }
}
/// 格式化字节数
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_index])
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_real_time_monitor() {
        let config: _ = MonitorConfig::new();
        let monitor: _ = RealTimeMonitor::new(config);
        // 创建测试结果
        let mut result = super::super::result::BenchmarkResult::new("test", Runtime::Amber);
        result.add_iteration(Duration::from_millis(100));
        result.finish();
        // 记录结果
        monitor.record_benchmark_result(&result).await;
        // 获取当前指标
        let metrics: _ = monitor.get_current_metrics().await;
        println!("Metrics: {:?}", metrics);
    }
    #[tokio::test]
    async fn test_performance_dashboard() {
        let config: _ = MonitorConfig::new();
        let dashboard: _ = PerformanceDashboard::new(config);
        let html: _ = dashboard.generate_html_report().await.unwrap();
        println!("HTML Report:\n{}", html);
    }
}