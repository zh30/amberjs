// 实时指标收集器
// 收集 Amber 运行时的各种性能指标，支持 Prometheus 导出

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, atomic::Ordering};

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use std::sync::atomic::AtomicU64;
use std::time::SystemTime;
/// Tenant ID type
pub type TenantId = String;
/// Cluster ID type
pub type ClusterId = String;
/// 指标收集器
#[derive(Debug)]
pub struct MetricsCollector {
    /// 总请求数
    pub requests_total: AtomicU64,
    /// 总延迟时间（毫秒）
    pub total_latency_ms: AtomicU64,
    /// 当前活跃连接数
    pub active_connections: AtomicU64,
    /// 内存使用量（字节）
    pub memory_usage_bytes: AtomicU64,
    /// CPU 使用率（百分比）
    pub cpu_usage_percent: AtomicU64,
    /// 最后更新时间
    pub last_update: SystemTime,
}
impl MetricsCollector {
    /// 创建新的指标收集器
    pub fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            memory_usage_bytes: AtomicU64::new(0),
            cpu_usage_percent: AtomicU64::new(0),
            last_update: SystemTime::now(),
        }
    }
    /// 记录请求指标
    ///
    /// # Arguments
    ///
    /// * `latency` - 请求延迟时间
    /// * `status` - 请求状态
    pub fn record_request(&mut self, latency: Duration, status: RequestStatus) {
        self.requests_total.fetch_add(1, Ordering::SeqCst);
        self.total_latency_ms
            .fetch_add(latency.as_millis() as u64, Ordering::SeqCst);
        self.last_update = SystemTime::now();
    }
    /// 记录内存使用情况
    ///
    /// # Arguments
    ///
    /// * `bytes` - 内存使用量（字节）
    pub fn record_memory_usage(&mut self, bytes: u64) {
        self.memory_usage_bytes
            .fetch_add(bytes, Ordering::SeqCst);
        self.last_update = SystemTime::now();
    }
    /// 更新活跃连接数
    ///
    /// # Arguments
    ///
    /// * `count` - 活跃连接数
    pub fn update_active_connections(&mut self, count: u64) {
        self.active_connections.store(count, Ordering::SeqCst);
        self.last_update = SystemTime::now();
    }
    /// 更新 CPU 使用率
    ///
    /// # Arguments
    ///
    /// * `percent` - CPU 使用率（0-100）
    pub fn update_cpu_usage(&mut self, percent: f64) {
        let percent: _ = percent.round() as u64;
        self.cpu_usage_percent.store(percent, Ordering::SeqCst);
        self.last_update = SystemTime::now();
    }
    /// 获取平均请求延迟（毫秒）
    pub fn get_average_latency_ms(&self) -> f64 {
        let total_requests: _ = self.requests_total.load(Ordering::SeqCst);
        if total_requests == 0 {
            return 0.0;
        }
        let total_latency: _ = self.total_latency_ms.load(Ordering::SeqCst);
        total_latency as f64 / total_requests as f64
    }
    /// 获取当前指标快照
    pub fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            requests_total: self.requests_total.load(Ordering::SeqCst),
            total_latency_ms: self.total_latency_ms.load(Ordering::SeqCst),
            active_connections: self.active_connections.load(Ordering::SeqCst),
            memory_usage_bytes: self.memory_usage_bytes.load(Ordering::SeqCst),
            cpu_usage_percent: self.cpu_usage_percent.load(Ordering::SeqCst),
            last_update: self.last_update,
        }
    }
    /// 导出 Prometheus 格式的指标
    ///
    /// # Returns
    ///
    /// 返回 Prometheus 格式的指标字符串
    pub fn export_prometheus(&self) -> Result<String> {
        let snapshot: _ = self.get_metrics_snapshot();
        let average_latency: _ = self.get_average_latency_ms();
        let output: _ = format!(
            "# HELP amberjs_requests_total Total number of requests processed\n\
             # TYPE amberjs_requests_total counter\n\
             amberjs_requests_total {}\n\
             # HELP amberjs_request_duration_ms_total Total request duration in milliseconds\n\
             # TYPE amberjs_request_duration_ms_total counter\n\
             amberjs_request_duration_ms_total {}\n\
             # HELP amberjs_request_duration_ms Average request duration in milliseconds\n\
             # TYPE amberjs_request_duration_ms gauge\n\
             amberjs_request_duration_ms {}\n\
             # HELP amberjs_active_connections Number of active connections\n\
             # TYPE amberjs_active_connections gauge\n\
             amberjs_active_connections {}\n\
             # HELP amberjs_memory_usage_bytes Memory usage in bytes\n\
             # TYPE amberjs_memory_usage_bytes gauge\n\
             amberjs_memory_usage_bytes {}\n\
             # HELP amberjs_cpu_usage_percent CPU usage percentage\n\
             # TYPE amberjs_cpu_usage_percent gauge\n\
             amberjs_cpu_usage_percent {}\n",
            snapshot.requests_total,
            snapshot.total_latency_ms,
            average_latency,
            snapshot.active_connections,
            snapshot.memory_usage_bytes,
            snapshot.cpu_usage_percent
        );
        Ok(output)
    }
}
impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
/// 请求状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestStatus {
    Success,
    Error(String),
}
/// 指标快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub requests_total: u64,
    pub total_latency_ms: u64,
    pub active_connections: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: u64,
    pub last_update: SystemTime,
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_metrics_collector_creation() {
        let collector: _ = MetricsCollector::new();
        assert_eq!(collector.requests_total.load(Ordering::SeqCst), 0);
        assert_eq!(collector.total_latency_ms.load(Ordering::SeqCst), 0);
        assert_eq!(collector.active_connections.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn test_record_request() {
        let mut collector = MetricsCollector::new();
        collector.record_request(
            Duration::from_millis(150),
            RequestStatus::Success,
        );
        assert_eq!(collector.requests_total.load(Ordering::SeqCst), 1);
        assert_eq!(collector.total_latency_ms.load(Ordering::SeqCst), 150);
    }
    #[test]
    fn test_get_average_latency() {
        let mut collector = MetricsCollector::new();
        collector.record_request(
            Duration::from_millis(100),
            RequestStatus::Success,
        );
        collector.record_request(
            Duration::from_millis(200),
            RequestStatus::Success,
        );
        collector.record_request(
            Duration::from_millis(150),
            RequestStatus::Success,
        );
        let average: _ = collector.get_average_latency_ms();
        assert_eq!(average, 150.0); // (100 + 200 + 150) / 3
    }
    #[test]
    fn test_export_prometheus() {
        let mut collector = MetricsCollector::new();
        collector.record_request(
            Duration::from_millis(150),
            RequestStatus::Success,
        );
        collector.update_active_connections(5);
        collector.record_memory_usage(1024 * 1024); // 1MB
        collector.update_cpu_usage(45.5);
        let output: _ = collector.export_prometheus().unwrap();
        // 验证 Prometheus 格式
        assert!(output.contains("amberjs_requests_total 1"));
        assert!(output.contains("amberjs_request_duration_ms_total 150"));
        assert!(output.contains("amberjs_active_connections 5"));
        assert!(output.contains("amberjs_memory_usage_bytes 1048576"));
        assert!(output.contains("amberjs_cpu_usage_percent 46")); // 45.5 舍入为 46
        // 验证 HELP 和 TYPE 注释
        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
    }
    #[test]
    fn test_memory_usage() {
        let mut collector = MetricsCollector::new();
        collector.record_memory_usage(2048);
        assert_eq!(collector.memory_usage_bytes.load(Ordering::SeqCst), 2048);
        collector.record_memory_usage(1024);
        assert_eq!(collector.memory_usage_bytes.load(Ordering::SeqCst), 3072);
    }
    #[test]
    fn test_active_connections() {
        let mut collector = MetricsCollector::new();
        collector.update_active_connections(10);
        assert_eq!(collector.active_connections.load(Ordering::SeqCst), 10);
        collector.update_active_connections(5);
        assert_eq!(collector.active_connections.load(Ordering::SeqCst), 5);
    }
}
/// Enterprise Metrics Collector
/// 企业级指标收集器，支持多租户和集群级监控
/// 集群指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    pub cluster_id: ClusterId,
    pub timestamp: SystemTime,
    pub total_nodes: u32,
    pub ready_nodes: u32,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub network_io_mbps: f64,
    pub disk_io_ops: u64,
    pub active_pods: u32,
    pub failed_pods: u32,
    pub pending_pods: u32,
}
/// 租户指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMetrics {
    pub tenant_id: TenantId,
    pub timestamp: SystemTime,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub storage_usage_gb: f64,
    pub network_throughput_mbps: f64,
    pub active_sessions: u32,
    pub request_rate: f64,
    pub error_rate: f64,
    pub response_time_ms: f64,
}
/// 告警配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub name: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub duration_seconds: u64,
    pub enabled: bool,
}
/// 告警严重程度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}
/// 告警条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    CpuUsageAbove,
    MemoryUsageAbove,
    DiskUsageAbove,
    ErrorRateAbove,
    ResponseTimeAbove,
    PodFailureRateAbove,
}
/// 告警事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub config_name: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: SystemTime,
    pub resolved: bool,
    pub cluster_id: Option<ClusterId>,
    pub tenant_id: Option<TenantId>,
}
/// 告警动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertAction {
    Notify,
    Scale,
    Restart,
    Suspend,
    Log,
}
/// 企业级指标收集器
#[derive(Debug)]
pub struct EnterpriseMetricsCollector {
    prometheus_registry: Arc<PrometheusRegistry>,
    cluster_collectors: std::collections::BTreeMap<ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector, ClusterId, MetricsCollector>>,
    tenant_collectors: std::collections::BTreeMap<TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector, TenantId, MetricsCollector>>,
    alert_configs: std::collections::BTreeMap<String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig, String, AlertConfig>>,
    alerts: std::collections::BTreeMap<Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert, Uuid, Alert>>,
}
/// Prometheus 注册表
#[derive(Debug)]
pub struct PrometheusRegistry {
    metrics: Arc<std::sync::Mutex<std::collections::BTreeMap<String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String, String>>>,
}
impl PrometheusRegistry {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(std::sync::Mutex::new(std::collections::BTreeMap::new()))
        }
    }
    pub fn register_metric(&self, name: String, value: String) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.insert(name, value);
    }
    pub fn get_metrics(&self) -> String {
        let metrics: _ = self.metrics.lock().unwrap();
        metrics
            .iter()
            .map(|(k, v)| format!("{} {}", k, v))
            .collect::<Vec<_>()
            .join("\n")
    }
}
impl EnterpriseMetricsCollector {
    /// 创建新的企业级指标收集器
    pub fn new() -> Self {
        Self {
            prometheus_registry: Arc::new(Mutex::new(PrometheusRegistry::new()))
            cluster_collectors: std::collections::BTreeMap::new(),
            tenant_collectors: std::collections::BTreeMap::new(),
            alert_configs: std::collections::BTreeMap::new(),
            alerts: std::collections::BTreeMap::new(),
        }
    }
    /// 收集集群指标
    pub async fn collect_cluster_metrics(&self, cluster_id: &ClusterId) -> Result<ClusterMetrics> {
        let collector: _ = self
            .cluster_collectors
            .get(cluster_id)
            .context("Cluster collector not found")?;
        let snapshot: _ = collector.get_metrics_snapshot();
        // 模拟集群指标收集（实际实现中会从 Kubernetes API 获取）
        let metrics: _ = ClusterMetrics {
            cluster_id: cluster_id.clone(),
            timestamp: SystemTime::now(),
            total_nodes: 10,
            ready_nodes: 9,
            cpu_usage_percent: snapshot.cpu_usage_percent as f64,
            memory_usage_percent: (snapshot.memory_usage_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) * 100.0,
            network_io_mbps: 100.0,
            disk_io_ops: 1000,
            active_pods: snapshot.active_connections as u32,
            failed_pods: 0,
            pending_pods: 1,
        };
        Ok(metrics)
    }
    /// 收集租户指标
    pub async fn collect_tenant_metrics(&self, tenant_id: &TenantId) -> Result<TenantMetrics> {
        let collector: _ = self
            .tenant_collectors
            .get(tenant_id)
            .context("Tenant collector not found")?;
        let snapshot: _ = collector.get_metrics_snapshot();
        // 模拟租户指标收集（实际实现中会从租户级监控获取）
        let metrics: _ = TenantMetrics {
            tenant_id: tenant_id.clone(),
            timestamp: SystemTime::now(),
            cpu_usage_percent: snapshot.cpu_usage_percent as f64,
            memory_usage_percent: (snapshot.memory_usage_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) * 100.0,
            storage_usage_gb: 10.0,
            network_throughput_mbps: 50.0,
            active_sessions: snapshot.active_connections as u32,
            request_rate: snapshot.requests_total as f64 / 60.0,
            error_rate: 0.01,
            response_time_ms: collector.get_average_latency_ms(),
        };
        Ok(metrics)
    }
    /// 设置告警规则
    pub async fn setup_alerts(&mut self, configs: Vec<AlertConfig>) -> Result<()> {
        for config in configs {
            self.alert_configs.insert(config.name.clone(), config);
            info!("Added alert rule: {}", config.name);
        }
        Ok(())
    }
    /// 处理告警事件
    pub async fn handle_alert(&mut self, alert: Alert) -> Result<AlertAction> {
        let action: _ = match alert.severity {
            AlertSeverity::Critical => AlertAction::Scale,
            AlertSeverity::Warning => AlertAction::Notify,
            AlertSeverity::Info => AlertAction::Log,
        };
        self.alerts.insert(alert.id, alert);
        info!("Handled alert event: {}", action_to_string(&action));
        Ok(action)
    }
    /// 检查告警条件
    pub async fn check_alerts(&mut self) -> Result<Vec<Alert> {
        let mut triggered_alerts = Vec::new();
        for (name, config) in &self.alert_configs {
            if !config.enabled {
                continue;
            }
            let should_alert: _ = match &config.condition {
                AlertCondition::CpuUsageAbove => self.check_cpu_threshold(config.threshold),
                AlertCondition::MemoryUsageAbove => self.check_memory_threshold(config.threshold),
                AlertCondition::ErrorRateAbove => self.check_error_rate_threshold(config.threshold),
                _ => false,
            };
            if should_alert {
                let alert: _ = Alert {
                    id: Uuid::new_v4(),
                    config_name: name.clone(),
                    severity: config.severity.clone(),
                    message: format!(
                        "Alert condition met: {} above threshold {}",
                        name, config.threshold
                    ),
                    timestamp: SystemTime::now(),
                    resolved: false,
                    cluster_id: None,
                    tenant_id: None,
                };
                triggered_alerts.push(alert);
            }
        }
        Ok(triggered_alerts)
    }
    /// 检查 CPU 阈值
    fn check_cpu_threshold(&self, threshold: f64) -> bool {
        for collector in self.cluster_collectors.values() {
            let cpu_usage: _ = collector.cpu_usage_percent.load(Ordering::SeqCst);
            if cpu_usage as f64 > threshold {
                return true;
            }
        }
        false
    }
    /// 检查内存阈值
    fn check_memory_threshold(&self, threshold: f64) -> bool {
        for collector in self.cluster_collectors.values() {
            let memory_bytes: _ = collector.memory_usage_bytes.load(Ordering::SeqCst);
            let memory_gb: _ = memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            if memory_gb > threshold {
                return true;
            }
        }
        false
    }
    /// 检查错误率阈值
    fn check_error_rate_threshold(&self, threshold: f64) -> bool {
        for collector in self.cluster_collectors.values() {
            let requests: _ = collector.requests_total.load(Ordering::SeqCst);
            // 简化错误率计算
            let error_rate: _ = if requests > 0 { 0.05 } else { 0.0 };
            if error_rate > threshold {
                return true;
            }
        }
        false
    }
    /// 添加集群收集器
    pub fn add_cluster_collector(&mut self, cluster_id: ClusterId, collector: MetricsCollector) {
        self.cluster_collectors.insert(cluster_id, collector);
    }
    /// 添加租户收集器
    pub fn add_tenant_collector(&mut self, tenant_id: TenantId, collector: MetricsCollector) {
        self.tenant_collectors.insert(tenant_id, collector);
    }
    /// 导出所有指标到 Prometheus
    pub fn export_all_metrics(&self) -> Result<String> {
        let mut output = String::new();
        // 导出集群指标
        for (cluster_id, collector) in &self.cluster_collectors {
            let cluster_metrics: _ = format!(
                "# Cluster: {}\n{}",
                cluster_id,
                collector.export_prometheus()?
            );
            output.push_str(&cluster_metrics);
        }
        // 导出租户指标
        for (tenant_id, collector) in &self.tenant_collectors {
            let tenant_metrics: _ = format!(
                "# Tenant: {}\n{}",
                tenant_id,
                collector.export_prometheus()?
            );
            output.push_str(&tenant_metrics);
        }
        Ok(output)
    }
}
/// 告警动作转字符串
fn action_to_string(action: &AlertAction) -> &'static str {
    match action {
        AlertAction::Notify => "Notify",
        AlertAction::Scale => "Scale",
        AlertAction::Restart => "Restart",
        AlertAction::Suspend => "Suspend",
        AlertAction::Log => "Log",
    }
}
#[cfg(test)]
mod enterprise_tests {
use std::collections::{HashMap, BTreeMap};
use std::time::Duration;
use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn test_enterprise_metrics_collector() {
        let mut collector = EnterpriseMetricsCollector::new();
        // 添加测试集群
        let cluster_collector: _ = MetricsCollector::new();
        collector.add_cluster_collector("test-cluster".to_string(), cluster_collector);
        // 添加测试租户
        let tenant_collector: _ = MetricsCollector::new();
        collector.add_tenant_collector("test-tenant".to_string(), tenant_collector);
        // 测试集群指标收集
        let cluster_metrics: _ = collector
            .collect_cluster_metrics(&"test-cluster".to_string())
            .await
            .unwrap();
        assert_eq!(cluster_metrics.cluster_id, "test-cluster");
        assert!(cluster_metrics.total_nodes > 0);
        // 测试租户指标收集
        let tenant_metrics: _ = collector
            .collect_tenant_metrics(&"test-tenant".to_string())
            .await
            .unwrap();
        assert_eq!(tenant_metrics.tenant_id, "test-tenant");
        assert!(tenant_metrics.cpu_usage_percent >= 0.0);
    }
    #[tokio::test]
    async fn test_alert_setup() {
        let mut collector = EnterpriseMetricsCollector::new();
        let alert_configs: _ = vec![AlertConfig {
            name: "high_cpu".to_string(),
            description: "CPU usage above 80%".to_string(),
            severity: AlertSeverity::Warning,
            condition: AlertCondition::CpuUsageAbove,
            threshold: 80.0,
            duration_seconds: 60,
            enabled: true,
        }];
        collector.setup_alerts(alert_configs).await.unwrap();
        assert_eq!(collector.alert_configs.len(), 1);
        assert!(collector.alert_configs.contains_key("high_cpu"));
    }
    #[test]
    fn test_prometheus_registry() {
        let registry: _ = PrometheusRegistry::new();
        registry.register_metric("test_metric".to_string(), "100".to_string());
        let metrics: _ = registry.get_metrics();
        assert!(metrics.contains("test_metric 100"));
    }
}