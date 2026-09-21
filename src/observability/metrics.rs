// Custom metrics definitions for Amber runtime
//
// This module provides a comprehensive metrics system for monitoring
// runtime performance, resource usage, and business metrics.

use prometheus::{Counter, CounterVec, Gauge, HistogramOpts, HistogramVec, Opts, Registry};
use std::collections::VecDeque;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Custom metrics system that manages all runtime metrics
pub struct CustomMetrics {
    /// Runtime performance metrics
    runtime_metrics: Arc<RwLock<RuntimeMetrics>>,
    /// Performance metrics
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Business metrics
    business_metrics: Arc<RwLock<BusinessMetrics>>,
    /// Prometheus registry
    registry: Registry,
    /// Collection of all registered metrics
    metric_handles: Vec<Box<dyn Collector + Send + Sync>>,
}
impl CustomMetrics {
    /// Create a new metrics system
    pub fn new() -> Self {
        let registry: _ = Registry::new();
        let mut metric_handles = Vec::new();
        let runtime_metrics: _ = Arc::new(Mutex::new(RuntimeMetrics::new(
            &registry,
            &mut metric_handles,
        )));
        let performance_metrics: _ = Arc::new(Mutex::new(PerformanceMetrics::new(
            &registry,
            &mut metric_handles,
        )));
        let business_metrics: _ = Arc::new(Mutex::new(BusinessMetrics::new(
            &registry,
            &mut metric_handles,
        )));
        debug!(
            "Custom metrics system initialized with {} collectors",
            metric_handles.len()
        );
        Self {
            runtime_metrics,
            performance_metrics,
            business_metrics,
            registry,
            metric_handles,
        }
    }
    /// Get runtime metrics snapshot
    pub async fn runtime_metrics(&self) -> Arc<RwLock<RuntimeMetrics>> {
        self.runtime_metrics.clone()
    }
    /// Get performance metrics snapshot
    pub async fn performance_metrics(&self) -> Arc<RwLock<PerformanceMetrics>> {
        self.performance_metrics.clone()
    }
    /// Get business metrics snapshot
    pub async fn business_metrics(&self) -> Arc<RwLock<BusinessMetrics>> {
        self.business_metrics.clone()
    }
    /// Record a script execution
    pub async fn record_script_execution(&self, duration: Duration, success: bool) {
        {
            let metrics: _ = self.runtime_metrics.write().await;
            metrics.record_execution(duration).await;
        }
        {
            let metrics: _ = self.performance_metrics.write().await;
            metrics.record_script_execution(duration).await;
        }
        {
            let metrics: _ = self.business_metrics.write().await;
            metrics.record_script(success).await;
        }
    }
    /// Record memory usage
    pub async fn record_memory_usage(&self, bytes: usize) {
        let metrics: _ = self.runtime_metrics.write().await;
        metrics.record_memory(bytes).await;
    }
    /// Record JIT compilation
    pub async fn record_jit_compilation(&self, duration: Duration) {
        let metrics: _ = self.performance_metrics.write().await;
        metrics.record_jit_compilation(duration).await;
    }
    /// Record GC pause
    pub async fn record_gc_pause(&self, duration: Duration) {
        let metrics: _ = self.performance_metrics.write().await;
        metrics.record_gc_pause(duration).await;
    }
    /// Record network I/O
    pub async fn record_network_io(&self, operation: &str, bytes: usize, duration: Duration) {
        let metrics: _ = self.performance_metrics.write().await;
        metrics.record_network_io(operation, bytes, duration).await;
    }
    /// Record package load
    pub async fn record_package_load(&self, package_name: &str, size_bytes: usize) {
        let metrics: _ = self.business_metrics.write().await;
        metrics.record_package_load(package_name, size_bytes).await;
    }
    /// Record hot reload
    pub async fn record_hot_reload(&self, file_path: &str) {
        let metrics: _ = self.business_metrics.write().await;
        metrics.record_hot_reload(file_path).await;
    }
    /// Get Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
    /// Gather all metrics for Prometheus export
    pub fn gather_metrics(&self) -> Result<Vec<prometheus::proto::MetricFamily>> {
        Ok(self.registry.gather())
    }
}
impl Default for CustomMetrics {
    fn default() -> Self {
        Self::new()
    }
}
/// Runtime metrics tracking
pub struct RuntimeMetrics {
    /// Active scripts gauge
    active_scripts: Gauge,
    /// Memory usage gauge
    memory_usage_bytes: Gauge,
    /// CPU usage gauge
    cpu_usage_percent: Gauge,
    /// Recent execution durations for P95 calculation
    recent_executions: Arc<RwLock<VecDeque<Duration>>>,
}
impl RuntimeMetrics {
    const MAX_RECENT_EXECUTIONS: usize = 1000;
    pub fn new(
        registry: &Registry,
        metric_handles: &mut Vec<Box<dyn Collector + Send + Sync>>,
    ) -> Self {
        // Active scripts gauge
        let active_scripts_opts: _ = Opts::new(
            "amberjs_active_scripts".to_string(),
            "Number of currently executing scripts".to_string(),
        );
        let active_scripts: _ = Gauge::with_opts(active_scripts_opts).unwrap();
        registry.register(Box::new(active_scripts.clone())).unwrap();
        metric_handles.push(Box::new(active_scripts.clone()));
        // Memory usage gauge
        let memory_usage_opts: _ = Opts::new(
            "amberjs_memory_usage_bytes".to_string(),
            "Current memory usage in bytes".to_string(),
        );
        let memory_usage_bytes: _ = Gauge::with_opts(memory_usage_opts).unwrap();
        registry
            .register(Box::new(memory_usage_bytes.clone()))
            .unwrap();
        metric_handles.push(Box::new(memory_usage_bytes.clone()));
        // CPU usage gauge
        let cpu_usage_opts: _ = Opts::new(
            "amberjs_cpu_usage_percent".to_string(),
            "Current CPU usage percentage".to_string(),
        );
        let cpu_usage_percent: _ = Gauge::with_opts(cpu_usage_opts).unwrap();
        registry
            .register(Box::new(cpu_usage_percent.clone()))
            .unwrap();
        metric_handles.push(Box::new(cpu_usage_percent.clone()));
        Self {
            active_scripts,
            memory_usage_bytes,
            cpu_usage_percent,
            recent_executions: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    pub async fn record_execution(&self, duration: Duration) {
        let mut recent = self.recent_executions.write().await;
        recent.push_back(duration);
        if recent.len() > Self::MAX_RECENT_EXECUTIONS {
            recent.pop_front();
        }
    }
    pub async fn record_memory(&self, bytes: usize) {
        self.memory_usage_bytes.set(bytes as f64);
    }
    pub async fn set_active_scripts(&self, count: u64) {
        self.active_scripts.set(count as f64);
    }
    pub async fn set_cpu_usage(&self, percent: f64) {
        self.cpu_usage_percent.set(percent);
    }
    /// Calculate P95 from recent executions
    pub async fn calculate_p95_duration(&self) -> Duration {
        let recent: _ = self.recent_executions.read().await;
        if recent.is_empty() {
            return Duration::from_millis(0);
        }
        let mut durations: Vec<Duration> = recent.iter().cloned().collect();
        durations.sort_by_key(|d| d.as_millis() as u64);
        let index: _ = (durations.len() as f64 * 0.95) as usize;
        durations[index.min(durations.len() - 1)]
    }
}
/// Performance metrics tracking
pub struct PerformanceMetrics {
    /// Script execution histogram
    script_execution_duration: HistogramVec,
    /// JIT compilation histogram
    jit_compilation_duration: HistogramVec,
    /// GC pause histogram
    gc_pause_duration: HistogramVec,
    /// Network latency histogram
    network_latency: HistogramVec,
    /// Network throughput counter
    network_throughput: CounterVec,
    /// Execution counter
    execution_counter: CounterVec,
}
impl PerformanceMetrics {
    const EXECUTION_BUCKETS: &[f64] = &[
        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];
    const JIT_BUCKETS: &[f64] = &[
        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
    ];
    const GC_BUCKETS: &[f64] = &[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0];
    const NETWORK_BUCKETS: &[f64] = &[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5];
    pub fn new(
        registry: &Registry,
        metric_handles: &mut Vec<Box<dyn Collector + Send + Sync>>,
    ) -> Self {
        // Script execution duration
        let script_execution_opts: _ = HistogramOpts::new(
            "amberjs_script_execution_duration_seconds".to_string(),
            "Script execution duration in seconds".to_string(),
        )
        .buckets(Self::EXECUTION_BUCKETS.to_vec());
        let script_execution_duration: _ =
            HistogramVec::new(script_execution_opts, &["script_name"]).unwrap();
        registry
            .register(Box::new(script_execution_duration.clone()))
            .unwrap();
        metric_handles.push(Box::new(script_execution_duration.clone()));
        // JIT compilation duration
        let jit_compilation_opts: _ = HistogramOpts::new(
            "amberjs_jit_compilation_duration_seconds".to_string(),
            "JIT compilation duration in seconds".to_string(),
        )
        .buckets(Self::JIT_BUCKETS.to_vec());
        let jit_compilation_duration: _ =
            HistogramVec::new(jit_compilation_opts, &["operation"]).unwrap();
        registry
            .register(Box::new(jit_compilation_duration.clone()))
            .unwrap();
        metric_handles.push(Box::new(jit_compilation_duration.clone()));
        // GC pause duration
        let gc_pause_opts: _ = HistogramOpts::new(
            "amberjs_gc_pause_duration_seconds".to_string(),
            "Garbage collection pause duration in seconds".to_string(),
        )
        .buckets(Self::GC_BUCKETS.to_vec());
        let gc_pause_duration: _ = HistogramVec::new(gc_pause_opts, &["generation"]).unwrap();
        registry
            .register(Box::new(gc_pause_duration.clone()))
            .unwrap();
        metric_handles.push(Box::new(gc_pause_duration.clone()));
        // Network latency
        let network_latency_opts: _ = HistogramOpts::new(
            "amberjs_network_latency_seconds".to_string(),
            "Network operation latency in seconds".to_string(),
        )
        .buckets(Self::NETWORK_BUCKETS.to_vec());
        let network_latency: _ = HistogramVec::new(network_latency_opts, &["operation"]).unwrap();
        registry
            .register(Box::new(network_latency.clone()))
            .unwrap();
        metric_handles.push(Box::new(network_latency.clone()));
        // Network throughput
        let network_throughput_opts: _ = Opts::new(
            "amberjs_network_throughput_bytes_total".to_string(),
            "Total network throughput in bytes".to_string(),
        );
        let network_throughput: _ =
            CounterVec::new(network_throughput_opts, &["operation", "direction"]).unwrap();
        registry
            .register(Box::new(network_throughput.clone()))
            .unwrap();
        metric_handles.push(Box::new(network_throughput.clone()));
        // Execution counter
        let execution_counter_opts: _ = Opts::new(
            "amberjs_script_executions_total".to_string(),
            "Total number of script executions".to_string(),
        );
        let execution_counter: _ = CounterVec::new(execution_counter_opts, &["status"]).unwrap();
        registry
            .register(Box::new(execution_counter.clone()))
            .unwrap();
        metric_handles.push(Box::new(execution_counter.clone()));
        Self {
            script_execution_duration,
            jit_compilation_duration,
            gc_pause_duration,
            network_latency,
            network_throughput,
            execution_counter,
        }
    }
    pub async fn record_script_execution(&self, duration: Duration) {
        let duration_seconds: _ = duration.as_secs_f64();
        self.script_execution_duration
            .with_label_values(&["general"])
            .observe(duration_seconds);
    }
    pub async fn record_jit_compilation(&self, duration: Duration) {
        let duration_seconds: _ = duration.as_secs_f64();
        self.jit_compilation_duration
            .with_label_values(&["compile"])
            .observe(duration_seconds);
    }
    pub async fn record_gc_pause(&self, duration: Duration) {
        let duration_seconds: _ = duration.as_secs_f64();
        self.gc_pause_duration
            .with_label_values(&["major"])
            .observe(duration_seconds);
    }
    pub async fn record_network_io(&self, operation: &str, bytes: usize, duration: Duration) {
        let latency_seconds: _ = duration.as_secs_f64();
        self.network_latency
            .with_label_values(&[operation])
            .observe(latency_seconds);
        self.network_throughput
            .with_label_values(&[operation, "sent"])
            .inc_by(bytes as f64);
    }
}
/// Business metrics tracking
pub struct BusinessMetrics {
    /// Scripts loaded counter
    scripts_loaded: Counter,
    /// Packages installed counter
    packages_installed: Counter,
    /// Hot reloads counter
    hot_reloads: Counter,
    /// Concurrent executions gauge
    concurrent_executions: Gauge,
    /// Error counter
    error_counter: Counter,
    /// Success counter
    success_counter: Counter,
}
impl BusinessMetrics {
    pub fn new(
        registry: &Registry,
        metric_handles: &mut Vec<Box<dyn Collector + Send + Sync>>,
    ) -> Self {
        // Scripts loaded
        let scripts_loaded_opts: _ = Opts::new(
            "amberjs_scripts_loaded_total".to_string(),
            "Total number of scripts loaded".to_string(),
        );
        let scripts_loaded: _ = Counter::with_opts(scripts_loaded_opts).unwrap();
        registry.register(Box::new(scripts_loaded.clone())).unwrap();
        metric_handles.push(Box::new(scripts_loaded.clone()));
        // Packages installed
        let packages_installed_opts: _ = Opts::new(
            "amberjs_packages_loaded_total".to_string(),
            "Total number of packages loaded".to_string(),
        );
        let packages_installed: _ = Counter::with_opts(packages_installed_opts).unwrap();
        registry
            .register(Box::new(packages_installed.clone()))
            .unwrap();
        metric_handles.push(Box::new(packages_installed.clone()));
        // Hot reloads
        let hot_reloads_opts: _ = Opts::new(
            "amberjs_hot_reloads_total".to_string(),
            "Total number of hot reloads performed".to_string(),
        );
        let hot_reloads: _ = Counter::with_opts(hot_reloads_opts).unwrap();
        registry.register(Box::new(hot_reloads.clone())).unwrap();
        metric_handles.push(Box::new(hot_reloads.clone()));
        // Concurrent executions
        let concurrent_executions_opts: _ = Opts::new(
            "amberjs_concurrent_executions".to_string(),
            "Current number of concurrent script executions".to_string(),
        );
        let concurrent_executions: _ = Gauge::with_opts(concurrent_executions_opts).unwrap();
        registry
            .register(Box::new(concurrent_executions.clone()))
            .unwrap();
        metric_handles.push(Box::new(concurrent_executions.clone()));
        // Error counter
        let error_counter_opts: _ = Opts::new(
            "amberjs_script_errors_total".to_string(),
            "Total number of script execution errors".to_string(),
        );
        let error_counter: _ = Counter::with_opts(error_counter_opts).unwrap();
        registry.register(Box::new(error_counter.clone())).unwrap();
        metric_handles.push(Box::new(error_counter.clone()));
        // Success counter
        let success_counter_opts: _ = Opts::new(
            "amberjs_script_successes_total".to_string(),
            "Total number of successful script executions".to_string(),
        );
        let success_counter: _ = Counter::with_opts(success_counter_opts).unwrap();
        registry
            .register(Box::new(success_counter.clone()))
            .unwrap();
        metric_handles.push(Box::new(success_counter.clone()));
        Self {
            scripts_loaded,
            packages_installed,
            hot_reloads,
            concurrent_executions,
            error_counter,
            success_counter,
        }
    }
    pub async fn record_script(&self, success: bool) {
        if success {
            self.success_counter.inc();
        } else {
            self.error_counter.inc();
        }
    }
    pub async fn record_package_load(&self, _package_name: &str, _size_bytes: usize) {
        self.packages_installed.inc();
    }
    pub async fn record_hot_reload(&self, _file_path: &str) {
        self.hot_reloads.inc();
    }
    pub async fn set_concurrent_executions(&self, count: u64) {
        self.concurrent_executions.set(count as f64);
    }
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_custom_metrics_creation() {
        let metrics: _ = CustomMetrics::new();
        assert!(metrics.registry().gather().len() > 0);
    }
    #[tokio::test]
    async fn test_record_script_execution() {
        let metrics: _ = CustomMetrics::new();
        metrics
            .record_script_execution(Duration::from_millis(100), true)
            .await;
        metrics
            .record_script_execution(Duration::from_millis(200), false)
            .await;
    }
    #[tokio::test]
    async fn test_record_memory_usage() {
        let metrics: _ = CustomMetrics::new();
        metrics.record_memory_usage(1024 * 1024).await; // 1MB
    }
}
