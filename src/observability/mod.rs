// Observability module for Amber runtime
//
// This module provides comprehensive monitoring and observability features including:
// - Prometheus metrics export
// - Structured logging
// - Custom metrics
//
// # Examples
//
// ```rust
// use amberjs::observability::{
//     PrometheusExporter, StructuredLogger,
//     ObservabilityConfig, ObservableMetrics
// };
//
// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
// let config: _ = ObservabilityConfig::default();
// let mut observability = ObservableSystem::new(config).await?;
//
// # Ok(())
// # }
// ```
pub mod alerting;
pub mod dashboard;
pub mod jaeger_tracer;
pub mod metrics;
pub mod prometheus_exporter;
pub mod structured_logging;
pub mod visualization;

use metrics::{BusinessMetrics, CustomMetrics, PerformanceMetrics, RuntimeMetrics};
use std::collections::{BTreeMap, HashMap};

use std::net::SocketAddr;
use std::sync::RwLock;
use std::sync::{Arc, Mutex};
// use std::sync::::{Arc, Mutex, RwLock};
use std::time::Duration;
use tracing::info;

pub use dashboard::{
    ChartType, Dashboard, DashboardConfig, DashboardManager, FieldConfig, GrafanaClient, GraphType,
    GridPos, LegendConfig, MetricsCollector, PanelConfig, PanelOptions, QueryTarget,
    TemplateVariable, ThresholdsConfig, TimeRangeConfig, TooltipConfig,
};
pub use visualization::{
    AxisConfig, BarChart, BarChartBuilder, ColorPalette, DataPoint, DataSeries, EdgeStyle,
    FilterConfig, ForceLayoutParams, GraphEdge, GraphNode, GridConfig, InteractionConfig,
    LayoutAlgorithm, LayoutConfig, LegendConfig as VizLegendConfig, LineChart, LineChartBuilder,
    LineStyle, MarkerConfig, PieChart, PieChartBuilder, Position, Size,
    TooltipConfig as VizTooltipConfig, TopologyGraph, TopologyGraphBuilder, VisualizationConfig,
};
/// Configuration for observability system
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Enable Prometheus metrics export
    pub enable_prometheus: bool,
    /// Prometheus bind address
    pub prometheus_addr: SocketAddr,
    /// Enable structured logging
    pub enable_structured_logging: bool,
    /// Log level
    pub log_level: tracing::Level,
    /// Enable alerting
    pub enable_alerting: bool,
    /// Metrics update interval
    pub metrics_update_interval: std::time::Duration,
}
impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            prometheus_addr: "127.0.0.1:9090".parse().unwrap(),
            enable_structured_logging: true,
            log_level: tracing::Level::INFO,
            enable_alerting: true,
            metrics_update_interval: std::time::Duration::from_secs(5),
        }
    }
}
/// Main observability system that manages all observability components
pub struct ObservableSystem {
    config: ObservabilityConfig,
    prometheus_exporter: Option<Arc<RwLock<PrometheusExporter>>>,
    structured_logger: Option<StructuredLogger>,
    custom_metrics: Arc<RwLock<CustomMetrics>>,
    alerting_system: Option<AlertingSystem>,
}
impl ObservableSystem {
    /// Create a new observability system
    pub async fn new(config: ObservabilityConfig) -> Result<Self> {
        info!("Initializing observability system...");
        let mut system = Self {
            config: config.clone(),
            prometheus_exporter: None,
            structured_logger: None,
            custom_metrics: Arc::new(Mutex::new(CustomMetrics::new())),
            alerting_system: None,
        };
        // Initialize structured logging first
        if config.enable_structured_logging {
            system.structured_logger =
                Some(StructuredLogger::new(config.log_level, "amberjs".to_string()));
            info!("Structured logging initialized");
        }
        // Initialize Prometheus exporter
        if config.enable_prometheus {
            let exporter: _ = PrometheusExporter::new()?;
            system.prometheus_exporter = Some(Arc::new(Mutex::new(exporter)));
            info!("Prometheus exporter initialized");
        }
        // Initialize alerting system
        if config.enable_alerting {
            system.alerting_system = Some(AlertingSystem::new());
            info!("Alerting system initialized");
        }
        info!("Observability system initialized successfully");
        Ok(system)
    }
    /// Get Prometheus exporter reference
    pub fn prometheus_exporter(&self) -> Option<Arc<RwLock<PrometheusExporter>>> {
        self.prometheus_exporter.clone()
    }
    /// Get structured logger reference
    pub fn logger(&self) -> &StructuredLogger {
        self.structured_logger
            .as_ref()
            .expect("Structured logger not initialized")
    }
    /// Get custom metrics reference
    pub fn custom_metrics(&self) -> Arc<RwLock<CustomMetrics>> {
        self.custom_metrics.clone()
    }
    /// Get alerting system reference
    pub fn alerting_system(&self) -> Option<&AlertingSystem> {
        self.alerting_system.as_ref()
    }
    /// Record a script execution event
    pub async fn record_script_execution(
        &self,
        script_name: &str,
        duration: std::time::Duration,
        success: bool,
    ) {
        // Update metrics
        let metrics: _ = self.custom_metrics.write().await;
        metrics.record_script_execution(duration, success).await;
        // Log event
        if let Some(logger) = &self.structured_logger {
            let context: _ = HashMap::from([
                (
                    "script_name".to_string(),
                    Value::String(script_name.to_string()),
                ),
                (
                    "duration_ms".to_string(),
                    Value::Number(serde_json::Number::from(duration.as_millis() as u64)),
                ),
                ("success".to_string(), Value::Bool(success)),
            ]);
            if success {
                logger.info("Script executed successfully", context).await;
            } else {
                logger.warn("Script execution failed", context).await;
            }
        }
    }
    /// Record memory usage
    pub async fn record_memory_usage(&self, bytes: usize) {
        let metrics: _ = self.custom_metrics.write().await;
        metrics.record_memory_usage(bytes).await;
    }
    /// Record network I/O event
    pub async fn record_network_io(
        &self,
        operation: &str,
        bytes: usize,
        duration: std::time::Duration,
    ) {
        let metrics: _ = self.custom_metrics.write().await;
        metrics.record_network_io(operation, bytes, duration).await;
    }
    /// Get current observable metrics
    pub async fn get_metrics(&self) -> ObservableMetrics {
        let custom_metrics: _ = self.custom_metrics.read().await;
        ObservableMetrics {
            runtime: custom_metrics.runtime_metrics().await.clone(),
            performance: custom_metrics.performance_metrics().await.clone(),
            business: custom_metrics.business_metrics().await.clone(),
        }
    }
    /// Shutdown observability system
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down observability system...");
        // Shutdown alerting system
        if let Some(alerting) = &self.alerting_system {
            alerting.shutdown().await?;
        }
        // Shutdown Prometheus exporter
        if let Some(exporter) = &self.prometheus_exporter {
            exporter.write().await.shutdown().await?;
        }
        info!("Observability system shutdown complete");
        Ok(())
    }
}
/// Observable metrics container
pub struct ObservableMetrics {
    pub runtime: std::sync::Arc<tokio::sync::RwLock<RuntimeMetrics>>,
    pub performance: std::sync::Arc<tokio::sync::RwLock<PerformanceMetrics>>,
    pub business: std::sync::Arc<tokio::sync::RwLock<BusinessMetrics>>,
}
/// Runtime metrics snapshot
#[derive(Debug, Clone)]
pub struct RuntimeMetricsSnapshot {
    pub active_scripts: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
}
/// Performance metrics snapshot
#[derive(Debug, Clone)]
pub struct PerformanceMetricsSnapshot {
    pub avg_script_duration_ms: f64,
    pub p95_script_duration_ms: f64,
    pub jit_compilation_time_ms: f64,
    pub gc_pause_time_ms: f64,
    pub network_latency_ms: f64,
}
/// Business metrics snapshot
#[derive(Debug, Clone)]
pub struct BusinessMetricsSnapshot {
    pub total_scripts_executed: u64,
    pub total_packages_loaded: u64,
    pub total_hot_reloads: u64,
    pub error_rate_percent: f64,
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_observability_system_creation() {
        let config: _ = ObservabilityConfig::default();
        let system: _ = ObservableSystem::new(config).await;
        assert!(system.is_ok());
    }
    #[tokio::test]
    async fn test_record_script_execution() {
        let config: _ = ObservabilityConfig::default();
        let system: _ = ObservableSystem::new(config).await.unwrap();
        system
            .record_script_execution("test.js", std::time::Duration::from_millis(100), true)
            .await;
    }
}
