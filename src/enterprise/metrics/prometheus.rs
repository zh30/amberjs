// Prometheus Metrics Integration for Amber
// 实现 Prometheus 指标收集和导出功能


use anyhow::{Result, Context};
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec,
    IntCounter, IntCounterVec, IntGauge, IntGaugeVec, LinearBuckets, Opts,
    Registry, TextEncoder,
};
use tokio::time::{Duration, Instant};
use tracing::{info, warn, debug};
/// Prometheus metrics configuration
#[derive(Debug, Clone)]
pub struct PrometheusConfig {
    /// Metrics endpoint port
    pub port: u16,
    /// Metrics endpoint path
    pub path: String,
    /// Metrics namespace
    pub namespace: String,
    /// Enable custom metrics
    pub custom_metrics: bool,
    /// Histogram buckets
    pub histogram_buckets: Vec<f64>,
}
/// Core runtime metrics
pub struct RuntimeMetrics {
    /// Process start time
    pub process_start_time: IntGauge,
    /// Active isolates count
    pub active_isolates: IntGauge,
    /// Total executions
    pub total_executions: IntCounter,
    /// Execution errors
    pub execution_errors: IntCounter,
    /// Current memory usage (bytes)
    pub memory_usage_bytes: IntGauge,
    /// CPU usage percentage
    pub cpu_usage_percent: Gauge,
}
/// Performance metrics
pub struct PerformanceMetrics {
    /// Execution time histogram
    pub execution_duration: HistogramVec,
    /// JIT compilation time
    pub jit_compilation_time: HistogramVec,
    /// JIT cache hit rate
    pub jit_cache_hit_rate: Gauge,
    /// Memory allocation rate
    pub memory_allocation_rate: Gauge,
    /// GC pause time
    pub gc_pause_time: HistogramVec,
    /// GC frequency
    pub gc_frequency: CounterVec,
}
/// Network metrics
pub struct NetworkMetrics {
    /// Network requests total
    pub network_requests_total: IntCounterVec,
    /// Network request duration
    pub network_request_duration: HistogramVec,
    /// Network errors
    pub network_errors: IntCounterVec,
    /// Active connections
    pub active_connections: IntGauge,
    /// Network throughput (bytes/sec)
    pub network_throughput: GaugeVec,
}
/// Business metrics
pub struct BusinessMetrics {
    /// Requests per second
    pub requests_per_second: Gauge,
    /// Response time p50
    pub response_time_p50: Gauge,
    /// Response time p95
    pub response_time_p95: Gauge,
    /// Response time p99
    pub response_time_p99: Gauge,
    /// Error rate
    pub error_rate: Gauge,
    /// Uptime
    pub uptime_seconds: IntGauge,
}
/// Cluster metrics
pub struct ClusterMetrics {
    /// Cluster nodes total
    pub cluster_nodes_total: IntGauge,
    /// Cluster nodes ready
    pub cluster_nodes_ready: IntGauge,
    /// Cluster CPU usage
    pub cluster_cpu_usage: GaugeVec,
    /// Cluster memory usage
    pub cluster_memory_usage: GaugeVec,
    /// Pod restarts
    pub pod_restarts: IntCounterVec,
    /// Upgrade progress
    pub upgrade_progress: GaugeVec,
}
/// Prometheus metrics manager
#[derive(Debug)]
pub struct PrometheusManager {
    /// Prometheus registry
    registry: Registry,
    /// Runtime metrics
    runtime: RuntimeMetrics,
    /// Performance metrics
    performance: PerformanceMetrics,
    /// Network metrics
    network: NetworkMetrics,
    /// Business metrics
    business: BusinessMetrics,
    /// Cluster metrics
    cluster: ClusterMetrics,
    /// Configuration
    config: PrometheusConfig,
    /// Last collection time
    last_collection: Instant,
}
impl PrometheusManager {
    /// Create a new PrometheusManager
    pub fn new(config: PrometheusConfig) -> Result<Self> {
        let registry: _ = Registry::new();
        // Initialize runtime metrics
        let runtime: _ = RuntimeMetrics::new(&registry, &config.namespace)?;
        // Initialize performance metrics
        let performance: _ = PerformanceMetrics::new(&registry, &config.namespace)?;
        // Initialize network metrics
        let network: _ = NetworkMetrics::new(&registry, &config.namespace)?;
        // Initialize business metrics
        let business: _ = BusinessMetrics::new(&registry, &config.namespace)?;
        // Initialize cluster metrics
        let cluster: _ = ClusterMetrics::new(&registry, &config.namespace)?;
        Ok(Self {
            registry,
            runtime,
            performance,
            network,
            business,
            cluster,
            config,
            last_collection: Instant::now(),
        })
    }
    /// Get the Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
    /// Record execution
    pub fn record_execution(&self, duration_ms: f64, success: bool) {
        // Update runtime metrics
        self.runtime.total_executions.inc();
        if !success {
            self.runtime.execution_errors.inc();
        }
        // Update performance metrics
        self.performance
            .execution_duration
            .with_label_values(&["total"])
            .observe(duration_ms);
        if success {
            self.performance
                .execution_duration
                .with_label_values(&["success"])
                .observe(duration_ms);
        } else {
            self.performance
                .execution_duration
                .with_label_values(&["error"])
                .observe(duration_ms);
        }
    }
    /// Record JIT compilation
    pub fn record_jit_compilation(&self, compilation_time_ms: f64, cache_hit: bool) {
        self.performance
            .jit_compilation_time
            .with_label_values(&[if cache_hit { "hit" } else { "miss" }])
            .observe(compilation_time_ms);
        // Update cache hit rate (simple moving average)
        let current_hit_rate: _ = self.performance.jit_cache_hit_rate.get();
        let new_hit_rate: _ = if cache_hit {
            current_hit_rate * 0.9 + 0.1
        } else {
            current_hit_rate * 0.9
        };
        self.performance.jit_cache_hit_rate.set(new_hit_rate);
    }
    /// Record memory usage
    pub fn record_memory_usage(&self, bytes: usize) {
        self.runtime.memory_usage_bytes.set(bytes as i64);
    }
    /// Record GC event
    pub fn record_gc_event(&self, pause_time_ms: f64, gc_type: &str) {
        self.performance
            .gc_pause_time
            .with_label_values(&[gc_type])
            .observe(pause_time_ms);
        self.performance
            .gc_frequency
            .with_label_values(&[gc_type])
            .inc();
    }
    /// Record network request
    pub fn record_network_request(
        &self,
        method: &str,
        endpoint: &str,
        status_code: u16,
        duration_ms: f64,
    ) {
        self.network
            .network_requests_total
            .with_label_values(&[method, endpoint, &status_code.to_string()])
            .inc();
        self.network
            .network_request_duration
            .with_label_values(&[method, endpoint])
            .observe(duration_ms);
        if status_code >= 400 {
            self.network
                .network_errors
                .with_label_values(&[method, endpoint, &status_code.to_string()])
                .inc();
        }
    }
    /// Update business metrics
    pub fn update_business_metrics(
        &self,
        rps: f64,
        p50: f64,
        p95: f64,
        p99: f64,
        error_rate: f64,
    ) {
        self.business.requests_per_second.set(rps);
        self.business.response_time_p50.set(p50);
        self.business.response_time_p95.set(p95);
        self.business.response_time_p99.set(p99);
        self.business.error_rate.set(error_rate);
    }
    /// Update cluster metrics
    pub fn update_cluster_metrics(
        &self,
        nodes_total: i64,
        nodes_ready: i64,
        cpu_usage: f64,
        memory_usage: f64,
    ) {
        self.cluster.cluster_nodes_total.set(nodes_total);
        self.cluster.cluster_nodes_ready.set(nodes_ready);
        self.cluster
            .cluster_cpu_usage
            .with_label_values(&["total"])
            .set(cpu_usage);
        self.cluster
            .cluster_memory_usage
            .with_label_values(&["total"])
            .set(memory_usage);
    }
    /// Record pod restart
    pub fn record_pod_restart(&self, namespace: &str, pod_name: &str) {
        self.cluster
            .pod_restarts
            .with_label_values(&[namespace, pod_name])
            .inc();
    }
    /// Update upgrade progress
    pub fn update_upgrade_progress(&self, cluster_name: &str, percentage: f64) {
        self.cluster
            .upgrade_progress
            .with_label_values(&[cluster_name])
            .set(percentage);
    }
    /// Collect and export metrics
    pub async fn collect_and_export(&self) -> Result<String> {
        // Collect system metrics
        self.collect_system_metrics().await;
        // Generate Prometheus text format
        let encoder: _ = TextEncoder::new();
        let metric_families: _ = self.registry.gather();
        let mut output = String::new();
        encoder.encode_utf8(&metric_families, &mut output)
            .context("Failed to encode metrics")?;
        debug!("Exported {} metric families", metric_families.len());
        Ok(output)
    }
    /// Collect system metrics
    async fn collect_system_metrics(&self) {
        // Update process start time if needed
        // This would typically be done once at startup
        // Update memory usage
        if let Ok(mem_usage) = sysinfo::System::new_all().total_memory() {
            self.record_memory_usage(mem_usage as usize);
        }
        // Update uptime
        let uptime: _ = self.last_collection.elapsed().as_secs();
        self.business.uptime_seconds.set(uptime as i64);
        info!("Collected system metrics (uptime: {}s)", uptime);
    }
}
/// Implement RuntimeMetrics
impl RuntimeMetrics {
    fn new(registry: &Registry, namespace: &str) -> Result<Self> {
        let ns: _ = format!("{}_", namespace));
        let process_start_time: _ = IntGauge::with_opts(
            Opts::new(
                format!("{}process_start_time_seconds", ns),
                "Process start time in seconds".to_string(),
            )
        )
        .context("Failed to create process_start_time metric")?;
        let active_isolates: _ = IntGauge::with_opts(
            Opts::new(
                format!("{}active_isolates", ns),
                "Number of active isolates".to_string(),
            )
        )
        .context("Failed to create active_isolates metric")?;
        let total_executions: _ = IntCounter::with_opts(
            Opts::new(
                format!("{}executions_total", ns),
                "Total number of executions".to_string(),
            )
        )
        .context("Failed to create executions_total metric")?;
        let execution_errors: _ = IntCounter::with_opts(
            Opts::new(
                format!("{}execution_errors_total", ns),
                "Total number of execution errors".to_string(),
            )
        )
        .context("Failed to create execution_errors metric")?;
        let memory_usage_bytes: _ = IntGauge::with_opts(
            Opts::new(
                format!("{}memory_usage_bytes", ns),
                "Current memory usage in bytes".to_string(),
            )
        )
        .context("Failed to create memory_usage metric")?;
        let cpu_usage_percent: _ = Gauge::with_opts(
            Opts::new(
                format!("{}cpu_usage_percent", ns),
                "Current CPU usage percentage".to_string(),
            )
        )
        .context("Failed to create cpu_usage metric")?;
        // Register metrics
        registry.register(Box::new(process_start_time.clone())?;
        registry.register(Box::new(active_isolates.clone())?;
        registry.register(Box::new(total_executions.clone())?;
        registry.register(Box::new(execution_errors.clone())?;
        registry.register(Box::new(memory_usage_bytes.clone())?;
        registry.register(Box::new(cpu_usage_percent.clone())?;
        Ok(Self {
            process_start_time,
            active_isolates,
            total_executions,
            execution_errors,
            memory_usage_bytes,
            cpu_usage_percent,
        })
    }
}
/// Implement PerformanceMetrics
impl PerformanceMetrics {
    fn new(registry: &Registry, namespace: &str) -> Result<Self> {
        let ns: _ = format!("{}_", namespace));
        let execution_duration: _ = HistogramVec::new(
            HistogramOpts::new(
                format!("{}execution_duration_seconds", ns),
                "Execution duration in seconds".to_string(),
            )
            .buckets(vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0]),
            &["result"],
        )
        .context("Failed to create execution_duration metric")?;
        let jit_compilation_time: _ = HistogramVec::new(
            HistogramOpts::new(
                format!("{}jit_compilation_time_seconds", ns),
                "JIT compilation time in seconds".to_string(),
            )
            .buckets(vec![0.001, 0.01, 0.1, 0.5]),
            &["cache_status"],
        )
        .context("Failed to create jit_compilation_time metric")?;
        let jit_cache_hit_rate: _ = Gauge::with_opts(
            Opts::new(
                format!("{}jit_cache_hit_rate", ns),
                "JIT cache hit rate (0-1)".to_string(),
            )
        )
        .context("Failed to create jit_cache_hit_rate metric")?;
        let memory_allocation_rate: _ = Gauge::with_opts(
            Opts::new(
                format!("{}memory_allocation_rate_bytes_per_second", ns),
                "Memory allocation rate in bytes per second".to_string(),
            )
        )
        .context("Failed to create memory_allocation_rate metric")?;
        let gc_pause_time: _ = HistogramVec::new(
            HistogramOpts::new(
                format!("{}gc_pause_time_seconds", ns),
                "Garbage collection pause time in seconds".to_string(),
            )
            .buckets(vec![0.001, 0.01, 0.1, 0.5]),
            &["gc_type"],
        )
        .context("Failed to create gc_pause_time metric")?;
        let gc_frequency: _ = CounterVec::new(
            Opts::new(
                format!("{}gc_runs_total", ns),
                "Total number of GC runs".to_string(),
            ),
            &["gc_type"],
        )
        .context("Failed to create gc_frequency metric")?;
        // Register metrics
        registry.register(Box::new(execution_duration.clone())?;
        registry.register(Box::new(jit_compilation_time.clone())?;
        registry.register(Box::new(jit_cache_hit_rate.clone())?;
        registry.register(Box::new(memory_allocation_rate.clone())?;
        registry.register(Box::new(gc_pause_time.clone())?;
        registry.register(Box::new(gc_frequency.clone())?;
        Ok(Self {
            execution_duration,
            jit_compilation_time,
            jit_cache_hit_rate,
            memory_allocation_rate,
            gc_pause_time,
            gc_frequency,
        })
    }
}
/// Implement NetworkMetrics
impl NetworkMetrics {
    fn new(registry: &Registry, namespace: &str) -> Result<Self> {
        let ns: _ = format!("{}_", namespace));
        let network_requests_total: _ = IntCounterVec::new(
            Opts::new(
                format!("{}network_requests_total", ns),
                "Total number of network requests".to_string(),
            ),
            &["method", "endpoint", "status_code"],
        )
        .context("Failed to create network_requests_total metric")?;
        let network_request_duration: _ = HistogramVec::new(
            HistogramOpts::new(
                format!("{}network_request_duration_seconds", ns),
                "Network request duration in seconds".to_string(),
            )
            .buckets(vec![0.01, 0.1, 0.5, 1.0, 5.0]),
            &["method", "endpoint"],
        )
        .context("Failed to create network_request_duration metric")?;
        let network_errors: _ = IntCounterVec::new(
            Opts::new(
                format!("{}network_errors_total", ns),
                "Total number of network errors".to_string(),
            ),
            &["method", "endpoint", "status_code"],
        )
        .context("Failed to create network_errors metric")?;
        let active_connections: _ = IntGauge::with_opts(
            Opts::new(
                format!("{}active_connections", ns),
                "Number of active network connections".to_string(),
            )
        )
        .context("Failed to create active_connections metric")?;
        let network_throughput: _ = GaugeVec::new(
            Opts::new(
                format!("{}network_throughput_bytes_per_second", ns),
                "Network throughput in bytes per second".to_string(),
            ),
            &["direction"],
        )
        .context("Failed to create network_throughput metric")?;
        // Register metrics
        registry.register(Box::new(network_requests_total.clone())?;
        registry.register(Box::new(network_request_duration.clone())?;
        registry.register(Box::new(network_errors.clone())?;
        registry.register(Box::new(active_connections.clone())?;
        registry.register(Box::new(network_throughput.clone())?;
        Ok(Self {
            network_requests_total,
            network_request_duration,
            network_errors,
            active_connections,
            network_throughput,
        })
    }
}
/// Implement BusinessMetrics
impl BusinessMetrics {
    fn new(registry: &Registry, namespace: &str) -> Result<Self> {
        let ns: _ = format!("{}_", namespace));
        let requests_per_second: _ = Gauge::with_opts(
            Opts::new(
                format!("{}requests_per_second", ns),
                "Requests per second".to_string(),
            )
        )
        .context("Failed to create requests_per_second metric")?;
        let response_time_p50: _ = Gauge::with_opts(
            Opts::new(
                format!("{}response_time_p50_seconds", ns),
                "50th percentile response time in seconds".to_string(),
            )
        )
        .context("Failed to create response_time_p50 metric")?;
        let response_time_p95: _ = Gauge::with_opts(
            Opts::new(
                format!("{}response_time_p95_seconds", ns),
                "95th percentile response time in seconds".to_string(),
            )
        )
        .context("Failed to create response_time_p95 metric")?;
        let response_time_p99: _ = Gauge::with_opts(
            Opts::new(
                format!("{}response_time_p99_seconds", ns),
                "99th percentile response time in seconds".to_string(),
            )
        )
        .context("Failed to create response_time_p99 metric")?;
        let error_rate: _ = Gauge::with_opts(
            Opts::new(
                format!("{}error_rate", ns),
                "Error rate (0-1)".to_string(),
            )
        )
        .context("Failed to create error_rate metric")?;
        let uptime_seconds: _ = IntGauge::with_opts(
            Opts::new(
                format!("{}uptime_seconds", ns),
                "Uptime in seconds".to_string(),
            )
        )
        .context("Failed to create uptime_seconds metric")?;
        // Register metrics
        registry.register(Box::new(requests_per_second.clone())?;
        registry.register(Box::new(response_time_p50.clone())?;
        registry.register(Box::new(response_time_p95.clone())?;
        registry.register(Box::new(response_time_p99.clone())?;
        registry.register(Box::new(error_rate.clone())?;
        registry.register(Box::new(uptime_seconds.clone())?;
        Ok(Self {
            requests_per_second,
            response_time_p50,
            response_time_p95,
            response_time_p99,
            error_rate,
            uptime_seconds,
        })
    }
}
/// Implement ClusterMetrics
impl ClusterMetrics {
    fn new(registry: &Registry, namespace: &str) -> Result<Self> {
        let ns: _ = format!("{}_", namespace));
        let cluster_nodes_total: _ = IntGauge::with_opts(
            Opts::new(
                format!("{}cluster_nodes_total", ns),
                "Total number of cluster nodes".to_string(),
            )
        )
        .context("Failed to create cluster_nodes_total metric")?;
        let cluster_nodes_ready: _ = IntGauge::with_opts(
            Opts::new(
                format!("{}cluster_nodes_ready", ns),
                "Number of ready cluster nodes".to_string(),
            )
        )
        .context("Failed to create cluster_nodes_ready metric")?;
        let cluster_cpu_usage: _ = GaugeVec::new(
            Opts::new(
                format!("{}cluster_cpu_usage_percent", ns),
                "Cluster CPU usage percentage".to_string(),
            ),
            &["scope"],
        )
        .context("Failed to create cluster_cpu_usage metric")?;
        let cluster_memory_usage: _ = GaugeVec::new(
            Opts::new(
                format!("{}cluster_memory_usage_bytes", ns),
                "Cluster memory usage in bytes".to_string(),
            ),
            &["scope"],
        )
        .context("Failed to create cluster_memory_usage metric")?;
        let pod_restarts: _ = IntCounterVec::new(
            Opts::new(
                format!("{}pod_restarts_total", ns),
                "Total number of pod restarts".to_string(),
            ),
            &["namespace", "pod"],
        )
        .context("Failed to create pod_restarts metric")?;
        let upgrade_progress: _ = GaugeVec::new(
            Opts::new(
                format!("{}upgrade_progress_percent", ns),
                "Upgrade progress percentage".to_string(),
            ),
            &["cluster"],
        )
        .context("Failed to create upgrade_progress metric")?;
        // Register metrics
        registry.register(Box::new(cluster_nodes_total.clone())?;
        registry.register(Box::new(cluster_nodes_ready.clone())?;
        registry.register(Box::new(cluster_cpu_usage.clone())?;
        registry.register(Box::new(cluster_memory_usage.clone())?;
        registry.register(Box::new(pod_restarts.clone())?;
        registry.register(Box::new(upgrade_progress.clone())?;
        Ok(Self {
            cluster_nodes_total,
            cluster_nodes_ready,
            cluster_cpu_usage,
            cluster_memory_usage,
            pod_restarts,
            upgrade_progress,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
use std::collections::{HashMap, BTreeMap};
use std::time::{Duration, Instant};
    #[test]
    fn test_prometheus_manager_creation() {
        let config: _ = PrometheusConfig {
            port: 9090,
            path: "/metrics".to_string(),
            namespace: "amberjs".to_string(),
            custom_metrics: true,
            histogram_buckets: vec![0.1, 0.5, 1.0],
        };
        let manager: _ = PrometheusManager::new(config);
        assert!(manager.is_ok());
    }
    #[test]
    fn test_record_execution() {
        let config: _ = PrometheusConfig {
            port: 9090,
            path: "/metrics".to_string(),
            namespace: "amberjs".to_string(),
            custom_metrics: true,
            histogram_buckets: vec![0.1, 0.5, 1.0],
        };
        let manager: _ = PrometheusManager::new(config).unwrap();
        // Record successful execution
        manager.record_execution(10.0, true);
        // Record failed execution
        manager.record_execution(5.0, false);
        // Verify metrics can be collected
        let output: _ = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(manager.collect_and_export());
        assert!(output.is_ok());
        let text: _ = output.unwrap();
        assert!(text.contains("amberjs_executions_total"));
        assert!(text.contains("amberjs_execution_errors_total"));
    }
    #[test]
    fn test_record_jit_compilation() {
        let config: _ = PrometheusConfig {
            port: 9090,
            path: "/metrics".to_string(),
            namespace: "amberjs".to_string(),
            custom_metrics: true,
            histogram_buckets: vec![0.1, 0.5, 1.0],
        };
        let manager: _ = PrometheusManager::new(config).unwrap();
        // Record JIT compilation
        manager.record_jit_compilation(5.0, true);
        manager.record_jit_compilation(10.0, false);
        // Verify metrics can be collected
        let output: _ = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(manager.collect_and_export());
        assert!(output.is_ok());
        let text: _ = output.unwrap();
        assert!(text.contains("amberjs_jit_compilation_time_seconds"));
    }
}