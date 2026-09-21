// 集群可视化控制台模块
// 提供实时集群状态监控、性能分析和可视化界面功能
//
// Stage 29.7: 分布式监控与调试 - 实时性能指标和监控

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use tracing::{info, warn};
use std::time::{Duration, Instant};
use std::time::SystemTime;
/// 告警级别
#[derive(Debug, Clone, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}
/// 告警消息
#[derive(Debug, Clone)]
pub struct AlertMessage {
    pub id: String,
    pub level: AlertLevel,
    pub title: String,
    pub description: String,
    pub timestamp: u64, // 使用 u64 而不是 Instant，便于序列化
    pub source: String,
    pub acknowledged: bool,
}
/// 集群状态概览
#[derive(Debug, Clone)]
pub struct ClusterOverview {
    pub cluster_name: String,
    pub total_nodes: u32,
    pub healthy_nodes: u32,
    pub unhealthy_nodes: u32,
    pub total_tasks: u64,
    pub active_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub availability: f64, // 0-100
    pub performance_score: f64, // 0-100
    pub last_updated: Instant,
}
/// 节点状态详情
#[derive(Debug, Clone)]
pub struct NodeStatusDetail {
    pub node_id: String,
    pub status: String,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_tasks: u32,
    pub task_queue_size: u32,
    pub uptime: Duration,
    pub last_heartbeat: Instant,
    pub location: String,
    pub capabilities: Vec<String>,
}
/// 性能指标详情
#[derive(Debug, Clone)]
pub struct PerformanceMetricsDetail {
    pub throughput: f64, // tasks/sec
    pub latency_p50: f64, // ms
    pub latency_p90: f64, // ms
    pub latency_p99: f64, // ms
    pub error_rate: f64, // 0-100
    pub resource_utilization: ResourceUtilization,
}
/// 资源利用率
#[derive(Debug, Clone)]
pub struct ResourceUtilization {
    pub cpu_avg: f64,
    pub memory_avg: f64,
    pub network_avg: f64,
    pub disk_avg: f64,
}
/// 追踪分析
#[derive(Debug, Clone)]
pub struct TraceAnalysis {
    pub total_traces: u64,
    pub slow_traces: Vec<SlowTrace>,
    pub error_traces: Vec<ErrorTrace>,
    pub operation_performance: HashMap<String, OperationPerformance>,
}
/// 慢追踪
#[derive(Debug, Clone)]
pub struct SlowTrace {
    pub trace_id: String,
    pub duration: Duration,
    pub operation_name: String,
    pub service_name: String,
}
/// 错误追踪
#[derive(Debug, Clone)]
pub struct ErrorTrace {
    pub trace_id: String,
    pub error_message: String,
    pub operation_name: String,
    pub service_name: String,
    pub timestamp: u64, // 使用 u64 而不是 Instant，便于序列化
}
/// 操作性能
#[derive(Debug, Clone)]
pub struct OperationPerformance {
    pub operation_name: String,
    pub call_count: u64,
    pub average_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub error_count: u64,
}
/// 控制台配置
#[derive(Debug, Clone)]
pub struct ConsoleConfig {
    pub refresh_interval: Duration,
    pub alert_threshold_cpu: f64,
    pub alert_threshold_memory: f64,
    pub alert_threshold_latency: f64,
    pub enable_notifications: bool,
    pub max_alerts: usize,
}
/// 集群控制台
#[derive(Clone, Debug)]
pub struct ClusterConsole {
    config: ConsoleConfig,
    distributed_metrics: Arc<DistributedMetrics>,
    distributed_tracer: Arc<DistributedTracer>,
    node_manager: Arc<NodeManager>,
    health_monitor: Arc<HealthMonitor>,
    fault_detector: Arc<FaultDetector>,
    cluster_overview: Arc<RwLock<Option<ClusterOverview>>>,
    node_status: Arc<RwLock<HashMap<String, NodeStatusDetail>>>,
    performance_metrics: Arc<RwLock<Option<PerformanceMetricsDetail>>>,
    alerts: Arc<RwLock<Vec<AlertMessage>>>,
    trace_analysis: Arc<RwLock<Option<TraceAnalysis>>>,
}
impl ClusterConsole {
    /// 创建新的集群控制台
    pub fn new(
        config: ConsoleConfig,
        distributed_metrics: Arc<DistributedMetrics>,
        distributed_tracer: Arc<DistributedTracer>,
        node_manager: Arc<NodeManager>,
        health_monitor: Arc<HealthMonitor>,
        fault_detector: Arc<FaultDetector>,
    ) -> Self {
        Self {
            config,
            distributed_metrics,
            distributed_tracer,
            node_manager,
            health_monitor,
            fault_detector,
            cluster_overview: Arc::new(Mutex::new(None)),
            node_status: Arc::new(Mutex::new(HashMap::new())),
            performance_metrics: Arc::new(Mutex::new(None)),
            alerts: Arc::new(Mutex::new(Vec::new())),
            trace_analysis: Arc::new(Mutex::new(None)),
        }
    }
    /// 启动集群控制台
    pub async fn start(&self) -> Result<(), String> {
        info!("Starting cluster console...");
        let console: _ = self.clone();
        tokio::spawn(async move {
            let mut interval_timer = interval(console.config.refresh_interval);
            loop {
                interval_timer.tick().await;
                if let Err(e) = console.update_cluster_overview().await {
                    warn!("Failed to update cluster overview: {}", e);
                }
                if let Err(e) = console.update_node_status().await {
                    warn!("Failed to update node status: {}", e);
                }
                if let Err(e) = console.update_performance_metrics().await {
                    warn!("Failed to update performance metrics: {}", e);
                }
                if let Err(e) = console.update_trace_analysis().await {
                    warn!("Failed to update trace analysis: {}", e);
                }
                if let Err(e) = console.check_alerts().await {
                    warn!("Failed to check alerts: {}", e);
                }
                if let Err(e) = console.cleanup_old_alerts().await {
                    warn!("Failed to cleanup old alerts: {}", e);
                }
            }
        });
        info!("Cluster console started");
        Ok(())
    }
    /// 获取集群概览
    pub async fn get_cluster_overview(&self) -> Option<ClusterOverview> {
        let overview: _ = self.cluster_overview.read().await;
        overview.clone()
    }
    /// 获取节点状态
    pub async fn get_node_status(&self) -> HashMap<String, NodeStatusDetail> {
        let status: _ = self.node_status.read().await;
        status.clone()
    }
    /// 获取性能指标
    pub async fn get_performance_metrics(&self) -> Option<PerformanceMetricsDetail> {
        let metrics: _ = self.performance_metrics.read().await;
        metrics.clone()
    }
    /// 获取告警
    pub async fn get_alerts(&self) -> Vec<AlertMessage> {
        let alerts: _ = self.alerts.read().await;
        alerts.clone()
    }
    /// 确认告警
    pub async fn acknowledge_alert(&self, alert_id: &str) -> Result<(), String> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            Ok(())
        } else {
            Err(format!("Alert not found: {}", alert_id))
        }
    }
    /// 获取追踪分析
    pub async fn get_trace_analysis(&self) -> Option<TraceAnalysis> {
        let analysis: _ = self.trace_analysis.read().await;
        analysis.clone()
    }
    /// 更新集群概览
    async fn update_cluster_overview(&self) -> Result<(), String> {
        let real_time_metrics: _ = self.distributed_metrics.get_real_time_metrics().await;
        if let Some(metrics) = real_time_metrics {
            let overview: _ = ClusterOverview {
                cluster_name: "amberjs-cluster".to_string(),
                total_nodes: metrics.cluster_summary.total_nodes,
                healthy_nodes: metrics.cluster_summary.healthy_nodes,
                unhealthy_nodes: metrics.cluster_summary.total_nodes - metrics.cluster_summary.healthy_nodes,
                total_tasks: metrics.cluster_summary.total_tasks,
                active_tasks: metrics.cluster_summary.active_tasks,
                completed_tasks: metrics.cluster_summary.completed_tasks,
                failed_tasks: metrics.cluster_summary.failed_tasks,
                availability: metrics.cluster_summary.availability * 100.0,
                performance_score: self.calculate_performance_score(&metrics).await,
                last_updated: Instant::now(),
            };
            let mut current_overview = self.cluster_overview.write().await;
            *current_overview = Some(overview);
        }
        Ok(())
    }
    /// 更新节点状态
    async fn update_node_status(&self) -> Result<(), String> {
        let real_time_metrics: _ = self.distributed_metrics.get_real_time_metrics().await;
        if let Some(metrics) = real_time_metrics {
            let mut node_status_map = HashMap::new();
            for (node_id, node_metrics) in metrics.node_metrics {
                let status: _ = NodeStatusDetail {
                    node_id: node_id.clone(),
                    status: if node_metrics.cpu_usage > 90.0 {
                        "Overloaded".to_string()
                    } else if node_metrics.cpu_usage > 70.0 {
                        "High Load".to_string()
                    } else {
                        "Healthy".to_string()
                    },
                    cpu_usage: node_metrics.cpu_usage,
                    memory_usage: node_metrics.memory_usage,
                    active_tasks: node_metrics.active_tasks,
                    task_queue_size: node_metrics.task_queue_size,
                    uptime: Duration::from_secs(node_metrics.uptime_seconds),
                    last_heartbeat: Instant::now(),
                    location: "local".to_string(),
                    capabilities: vec![
                        "js-execution".to_string(),
                        "ts-compilation".to_string(),
                    ],
                };
                node_status_map.insert(node_id, status);
            }
            let mut current_status = self.node_status.write().await;
            *current_status = node_status_map;
        }
        Ok(())
    }
    /// 更新性能指标
    async fn update_performance_metrics(&self) -> Result<(), String> {
        let real_time_metrics: _ = self.distributed_metrics.get_real_time_metrics().await;
        let perf_stats: _ = self.distributed_tracer.get_performance_stats().await;
        if let Some(metrics) = real_time_metrics {
            let perf_detail: _ = PerformanceMetricsDetail {
                throughput: metrics.cluster_summary.average_throughput,
                latency_p50: self.convert_duration_to_ms(perf_stats.p50_trace_duration),
                latency_p90: self.convert_duration_to_ms(perf_stats.p90_trace_duration),
                latency_p99: self.convert_duration_to_ms(perf_stats.p99_trace_duration),
                error_rate: if metrics.cluster_summary.total_tasks > 0 {
                    (metrics.cluster_summary.failed_tasks as f64 / metrics.cluster_summary.total_tasks as f64) * 100.0
                } else {
                    0.0
                },
                resource_utilization: ResourceUtilization {
                    cpu_avg: self.calculate_average_cpu_usage(&metrics).await,
                    memory_avg: self.calculate_average_memory_usage(&metrics).await,
                    network_avg: self.calculate_average_network_usage(&metrics).await,
                    disk_avg: self.calculate_average_disk_usage(&metrics).await,
                },
            };
            let mut current_metrics = self.performance_metrics.write().await;
            *current_metrics = Some(perf_detail);
        }
        Ok(())
    }
    /// 更新追踪分析
    async fn update_trace_analysis(&self) -> Result<(), String> {
        let traces: _ = self.distributed_tracer.get_completed_traces().await;
        let perf_stats: _ = self.distributed_tracer.get_performance_stats().await;
        let mut slow_traces = Vec::new();
        let mut error_traces = Vec::new();
        let mut operation_performance = HashMap::new();
        // 分类追踪
        for trace in traces {
            if let Some(duration) = trace.duration() {
                if self.convert_duration_to_ms(duration) > 1000.0 {
                    slow_traces.push(SlowTrace {
                        trace_id: trace.trace_id.clone(),
                        duration,
                        operation_name: trace.root_span.operation_name.clone(),
                        service_name: trace.root_span.service_name.clone(),
                    });
                }
                // 检查是否有错误事件
                for span in trace.spans.values() {
                    for event in &span.events {
                        if event.tags.contains_key("error") {
                            error_traces.push(ErrorTrace {
                                trace_id: trace.trace_id.clone(),
                                error_message: event.tags.get("error").unwrap_or(&"Unknown error".to_string()).clone(),
                                operation_name: span.operation_name.clone(),
                                service_name: span.service_name.clone(),
                                timestamp: event.timestamp,
                            });
                        }
                    }
                }
            }
            // 统计操作性能
            for span in trace.spans.values() {
                if let Some(duration) = span.duration() {
                    let op_name: _ = span.operation_name.clone();
                    let entry: _ = operation_performance.entry(op_name).or_insert(OperationPerformance {
                        operation_name: span.operation_name.clone(),
                        call_count: 0,
                        average_duration: Duration::from_millis(0),
                        min_duration: Duration::from_secs(u64::MAX),
                        max_duration: Duration::from_millis(0),
                        error_count: 0,
                    });
                    entry.call_count += 1;
                    entry.min_duration = entry.min_duration.min(duration);
                    entry.max_duration = entry.max_duration.max(duration);
                    // 更新平均持续时间
                    let total_ms: _ = entry.average_duration.as_millis() as u64 * (entry.call_count - 1);
                    entry.average_duration = Duration::from_millis((total_ms + duration.as_millis() as u64) / entry.call_count);
                }
            }
        }
        // 排序慢追踪
        slow_traces.sort_by(|a, b| b.duration.cmp(&a.duration));
        slow_traces.truncate(10);
        let analysis: _ = TraceAnalysis {
            total_traces: perf_stats.total_traces,
            slow_traces,
            error_traces,
            operation_performance,
        };
        let mut current_analysis = self.trace_analysis.write().await;
        *current_analysis = Some(analysis);
        Ok(())
    }
    /// 检查告警
    async fn check_alerts(&self) -> Result<(), String> {
        let real_time_metrics: _ = self.distributed_metrics.get_real_time_metrics().await;
        let perf_metrics: _ = self.performance_metrics.read().await;
        if let Some(metrics) = real_time_metrics {
            // 检查 CPU 使用率告警
            for (node_id, node_metrics) in &metrics.node_metrics {
                if node_metrics.cpu_usage > self.config.alert_threshold_cpu {
                    self.create_alert(
                        AlertLevel::Critical,
                        format!("High CPU usage on node {}", node_id),
                        format!("CPU usage is {}%", node_metrics.cpu_usage),
                        node_id.clone(),
                    ).await;
                }
                if node_metrics.memory_usage > self.config.alert_threshold_memory {
                    self.create_alert(
                        AlertLevel::Warning,
                        format!("High memory usage on node {}", node_id),
                        format!("Memory usage is {}%", node_metrics.memory_usage),
                        node_id.clone(),
                    ).await;
                }
            }
        }
        // 检查延迟告警
        if let Some(perf) = perf_metrics.as_ref() {
            if perf.latency_p99 > self.config.alert_threshold_latency {
                self.create_alert(
                    AlertLevel::Warning,
                    "High latency detected".to_string(),
                    format!("P99 latency is {}ms", perf.latency_p99),
                    "cluster".to_string(),
                ).await;
            }
        }
        Ok(())
    }
    /// 创建告警
    async fn create_alert(&self, level: AlertLevel, title: String, description: String, source: String) {
        let alert_id: _ = format!("alert-{:x}", rand::random::<u64>());
        let alert: _ = AlertMessage {
            id: alert_id,
            level,
            title,
            description,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            source,
            acknowledged: false,
        };
        let mut alerts = self.alerts.write().await;
        alerts.push(alert);
        // 限制告警数量
        if alerts.len() > self.config.max_alerts {
            alerts.remove(0);
        }
    }
    /// 清理旧告警
    async fn cleanup_old_alerts(&self) -> Result<(), String> {
        let cutoff: _ = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 3600; // 1 hour
        let mut alerts = self.alerts.write().await;
        alerts.retain(|alert| alert.timestamp > cutoff || !alert.acknowledged);
        Ok(())
    }
    // 辅助方法
    async fn calculate_performance_score(&self, metrics: &RealTimeMetrics) -> f64 {
        let cpu_score: _ = (100.0 - metrics.system_metrics.load_average * 10.0).max(0.0);
        let memory_score: _ = (100.0 - metrics.system_metrics.memory_pressure * 100.0).max(0.0);
        let availability_score: _ = metrics.cluster_summary.availability * 100.0;
        (cpu_score + memory_score + availability_score) / 3.0
    }
    fn convert_duration_to_ms(&self, duration: Duration) -> f64 {
        duration.as_millis() as f64
    }
    async fn calculate_average_cpu_usage(&self, metrics: &RealTimeMetrics) -> f64 {
        if metrics.node_metrics.is_empty() {
            return 0.0;
        }
        let sum: f64 = metrics.node_metrics.values()
            .map(|m| m.cpu_usage)
            .sum();
        sum / metrics.node_metrics.len() as f64
    }
    async fn calculate_average_memory_usage(&self, metrics: &RealTimeMetrics) -> f64 {
        if metrics.node_metrics.is_empty() {
            return 0.0;
        }
        let sum: f64 = metrics.node_metrics.values()
            .map(|m| m.memory_usage)
            .sum();
        sum / metrics.node_metrics.len() as f64
    }
    async fn calculate_average_network_usage(&self, metrics: &RealTimeMetrics) -> f64 {
        if metrics.node_metrics.is_empty() {
            return 0.0;
        }
        let sum: f64 = metrics.node_metrics.values()
            .map(|m| m.network_rx_mbps + m.network_tx_mbps)
            .sum();
        sum / metrics.node_metrics.len() as f64
    }
    async fn calculate_average_disk_usage(&self, metrics: &RealTimeMetrics) -> f64 {
        if metrics.node_metrics.is_empty() {
            return 0.0;
        }
        let sum: f64 = metrics.node_metrics.values()
            .map(|m| m.disk_read_mbps + m.disk_write_mbps)
            .sum();
        sum / metrics.node_metrics.len() as f64
    }
}