// Dashboard Manager - Grafana Integration
//
// This module provides the core dashboard management functionality:
// - Create and manage Grafana dashboards
// - Real-time metric collection and visualization
// - Custom panel configuration
// - Template variable support

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context, Result};
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::sync::RwLock;
use std::time::Duration;
use std::time::SystemTime;
use tracing::{debug, error, info, warn};
/// Dashboard Manager - Main entry point for Grafana integration
pub struct DashboardManager {
    /// Configuration
    config: DashboardConfig,
    /// HTTP client for Grafana API
    http_client: HttpClient,
    /// Active dashboards
    dashboards: Arc<RwLock<HashMap<String, Dashboard>>>,
    /// Grafana client
    grafana_client: Arc<GrafanaClient>,
    /// Metrics collector
    metrics_collector: Arc<MetricsCollector>,
}
/// Grafana API Client
pub struct GrafanaClient {
    /// Base URL for Grafana instance
    base_url: String,
    /// API key for authentication
    api_key: Option<String>,
    /// HTTP client
    http_client: HttpClient,
}
/// Dashboard definition
#[derive(Debug, Clone)]
pub struct Dashboard {
    /// Dashboard UID
    pub uid: String,
    /// Dashboard title
    pub title: String,
    /// Dashboard description
    pub description: Option<String>,
    /// Dashboard tags
    pub tags: Vec<String>,
    /// Panels in dashboard
    pub panels: Vec<PanelConfig>,
    /// Template variables
    pub templating: Vec<TemplateVariable>,
    /// Time range
    pub time: TimeRangeConfig,
    /// Refresh configuration
    pub refresh: RefreshConfig,
    /// Dashboard version
    pub version: i32,
    /// Created timestamp
    pub created_at: std::time::SystemTime,
    /// Updated timestamp
    pub updated_at: std::time::SystemTime,
}
/// Metrics Collector for real-time dashboard updates
pub struct MetricsCollector {
    /// Collection interval
    interval: std::time::Duration,
    /// Collected metrics
    metrics: Arc<RwLock<HashMap<String, Value>>>,
    /// Active collectors
    collectors: Vec<Box<dyn MetricsCollectorTrait + Send + Sync>>,
}
/// Metrics Collector Trait
#[async_trait::async_trait]
pub trait MetricsCollectorTrait {
    async fn collect(&self) -> Result<HashMap<String, Value>>;
    fn name(&self) -> &str;
}
/// Prometheus Metrics Collector
pub struct PrometheusCollector {
    /// Prometheus endpoint
    endpoint: String,
}
impl PrometheusCollector {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }
}
#[async_trait::async_trait]
impl MetricsCollectorTrait for PrometheusCollector {
    async fn collect(&self) -> Result<HashMap<String, Value>> {
        let client: _ = HttpClient::new();
        let response: _ = client
            .get(&format!("{}/api/v1/query", self.endpoint))
            .send()
            .await
            .context("Failed to query Prometheus")?;
        let data: Value = response
            .json()
            .await
            .context("Failed to parse Prometheus response")?;
        let mut metrics = HashMap::new();
        if let Value::Object(obj) = data {
            if let Some(Value::Array(results)) = obj.get("data") {
                for result in results {
                    if let Value::Object(result_obj) = result {
                        if let (Some(Value::String(metric_name)), Some(Value::Array(values))) =
                            (result_obj.get("metric"), result_obj.get("values"))
                        {
                            let values_vec: Vec<Value> = values.clone();
                            metrics.insert(metric_name.clone(), Value::Array(values_vec));
                        }
                    }
                }
            }
        }
        Ok(metrics)
    }
    fn name(&self) -> &str {
        "prometheus"
    }
}
/// Dashboard Configuration
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Grafana server URL
    pub grafana_url: String,
    /// Grafana API key (optional for anonymous access)
    pub api_key: Option<String>,
    /// Dashboard refresh interval (seconds)
    pub refresh_interval: u64,
    /// Metrics collection interval (seconds)
    pub metrics_interval: u64,
    /// Default time range (hours)
    pub default_time_range_hours: u64,
    /// Enable real-time updates
    pub enable_realtime: bool,
    /// Enable template variables
    pub enable_templating: bool,
}
impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            grafana_url: "http://localhost:3000".to_string(),
            api_key: None,
            refresh_interval: 5,
            metrics_interval: 1,
            default_time_range_hours: 1,
            enable_realtime: true,
            enable_templating: true,
        }
    }
}
impl DashboardManager {
    /// Create a new dashboard manager
    pub async fn new(config: DashboardConfig) -> Result<Self> {
        info!("Initializing Dashboard Manager...");
        let http_client: _ = HttpClient::new();
        let grafana_client: _ = Arc::new(Mutex::new(GrafanaClient::new(
            config.grafana_url.clone(),
            config.api_key.clone(),
            http_client.clone(),
        )));
        let metrics_collector: _ = Arc::new(Mutex::new(MetricsCollector::new(
            std::time::Duration::from_secs(config.metrics_interval),
        )));
        // Add default Prometheus collector
        if config.enable_realtime {
            metrics_collector.add_collector(Box::new(PrometheusCollector::new(
                "http://localhost:9090".to_string(),
            )));
        }
        let dashboards: _ = Arc::new(Mutex::new(HashMap::new()));
        // Initialize built-in dashboards
        Self::initialize_builtin_dashboards(&dashboards).await?;
        info!("Dashboard Manager initialized successfully");
        Ok(Self {
            config,
            http_client,
            dashboards,
            grafana_client,
            metrics_collector,
        })
    }
    /// Initialize built-in dashboards
    async fn initialize_builtin_dashboards(
        dashboards: &Arc<RwLock<HashMap<String, Dashboard>>>,
    ) -> Result<()> {
        // Create overview dashboard
        let overview_dashboard: _ = Dashboard::new(
            "amberjs-overview",
            "Amber Runtime Overview",
            "Real-time overview of Amber runtime performance and metrics",
            vec!["amberjs".to_string(), "overview".to_string()],
        );
        let mut dashboards_write = dashboards.write().await;
        dashboards_write.insert("amberjs-overview".to_string(), overview_dashboard);
        Ok(())
    }
    /// Create a new dashboard
    pub async fn create_dashboard(&self, title: &str) -> Result<String> {
        debug!("Creating dashboard: {}", title);
        let dashboard: _ = Dashboard::new(title, title, None, vec!["amberjs".to_string()]);
        let uid: _ = dashboard.uid.clone();
        let mut dashboards = self.dashboards.write().await;
        dashboards.insert(uid.clone(), dashboard);
        info!("Dashboard created: {} (UID: {})", title, uid);
        Ok(uid)
    }
    /// Get dashboard by UID
    pub async fn get_dashboard(&self, uid: &str) -> Option<Dashboard> {
        let dashboards: _ = self.dashboards.read().await;
        dashboards.get(uid).cloned()
    }
    /// List all dashboards
    pub async fn list_dashboards(&self) -> Vec<String> {
        let dashboards: _ = self.dashboards.read().await;
        dashboards.keys().cloned().collect()
    }
    /// Add panel to dashboard
    pub async fn add_panel(&self, dashboard_uid: &str, panel: PanelConfig) -> Result<()> {
        debug!(
            "Adding panel to dashboard {}: {}",
            dashboard_uid, panel.title
        );
        let mut dashboards = self.dashboards.write().await;
        let dashboard: _ = dashboards
            .get_mut(dashboard_uid)
            .ok_or_else(|| anyhow!("Dashboard not found: {}", dashboard_uid))?;
        dashboard.panels.push(panel);
        dashboard.updated_at = std::time::SystemTime::now();
        info!(
            "Panel added to dashboard {}: {}",
            dashboard_uid, panel.title
        );
        Ok(())
    }
    /// Update panel in dashboard
    pub async fn update_panel(
        &self,
        dashboard_uid: &str,
        panel_id: &str,
        panel: PanelConfig,
    ) -> Result<()> {
        debug!(
            "Updating panel in dashboard {}: {}",
            dashboard_uid, panel_id
        );
        let mut dashboards = self.dashboards.write().await;
        let dashboard: _ = dashboards
            .get_mut(dashboard_uid)
            .ok_or_else(|| anyhow!("Dashboard not found: {}", dashboard_uid))?;
        if let Some(pos) = dashboard.panels.iter_mut().position(|p| p.id == panel_id) {
            dashboard.panels[pos] = panel;
            dashboard.updated_at = std::time::SystemTime::now();
            info!("Panel updated in dashboard {}: {}", dashboard_uid, panel_id);
        } else {
            return Err(anyhow!("Panel not found: {}", panel_id));
        }
        Ok(())
    }
    /// Remove panel from dashboard
    pub async fn remove_panel(&self, dashboard_uid: &str, panel_id: &str) -> Result<()> {
        debug!(
            "Removing panel from dashboard {}: {}",
            dashboard_uid, panel_id
        );
        let mut dashboards = self.dashboards.write().await;
        let dashboard: _ = dashboards
            .get_mut(dashboard_uid)
            .ok_or_else(|| anyhow!("Dashboard not found: {}", dashboard_uid))?;
        if let Some(pos) = dashboard.panels.iter().position(|p| p.id == panel_id) {
            dashboard.panels.remove(pos);
            dashboard.updated_at = std::time::SystemTime::now();
            info!(
                "Panel removed from dashboard {}: {}",
                dashboard_uid, panel_id
            );
        } else {
            return Err(anyhow!("Panel not found: {}", panel_id));
        }
        Ok(())
    }
    /// Export dashboard to Grafana JSON format
    pub async fn export_dashboard(&self, uid: &str) -> Result<Value> {
        let dashboards: _ = self.dashboards.read().await;
        let dashboard: _ = dashboards
            .get(uid)
            .ok_or_else(|| anyhow!("Dashboard not found: {}", uid))?;
        let grafana_dashboard: _ = self.grafana_client.convert_to_grafana_format(dashboard)?;
        Ok(grafana_dashboard)
    }
    /// Deploy dashboard to Grafana server
    pub async fn deploy_dashboard(&self, uid: &str) -> Result<()> {
        info!("Deploying dashboard to Grafana: {}", uid);
        let grafana_dashboard: _ = self.export_dashboard(uid).await?;
        self.grafana_client
            .create_or_update_dashboard(grafana_dashboard)
            .await?;
        info!("Dashboard deployed successfully: {}", uid);
        Ok(())
    }
    /// Start real-time metrics collection
    pub async fn start_metrics_collection(&self) -> Result<()> {
        if !self.config.enable_realtime {
            warn!("Real-time metrics collection is disabled");
            return Ok(());
        }
        info!("Starting real-time metrics collection...");
        self.metrics_collector.start().await?;
        Ok(())
    }
    /// Stop real-time metrics collection
    pub async fn stop_metrics_collection(&self) -> Result<()> {
        info!("Stopping real-time metrics collection...");
        self.metrics_collector.stop().await?;
        Ok(())
    }
    /// Get dashboard metrics snapshot
    pub async fn get_metrics_snapshot(&self) -> Result<HashMap<String, Value>> {
        self.metrics_collector.get_snapshot().await
    }
    /// Shutdown dashboard manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Dashboard Manager...");
        self.stop_metrics_collection().await?;
        info!("Dashboard Manager shutdown complete");
        Ok(())
    }
}
impl GrafanaClient {
    /// Create a new Grafana client
    pub fn new(base_url: String, api_key: Option<String>, http_client: HttpClient) -> Self {
        Self {
            base_url,
            api_key,
            http_client,
        }
    }
    /// Convert internal dashboard to Grafana JSON format
    pub fn convert_to_grafana_format(&self, dashboard: &Dashboard) -> Result<Value> {
        let panels: Vec<Value> = dashboard
            .panels
            .iter()
            .enumerate()
            .map(|(idx, panel)| {
                json!({
                    "id": idx + 1,
                    "title": panel.title,
                    "type": panel.panel_type,
                    "gridPos": {
                        "h": panel.grid_pos.h,
                        "w": panel.grid_pos.w,
                        "x": panel.grid_pos.x,
                        "y": panel.grid_pos.y
                    },
                    "targets": panel.targets.iter().map(|t| {
                        json!({
                            "refId": t.ref_id,
                            "expr": t.query,
                            "interval": t.interval,
                            "legendFormat": t.legend_format
                        })
                    }).collect::<Vec<_>>(),
                    "fieldConfig": {
                        "defaults": {
                            "min": panel.field_config.min,
                            "max": panel.field_config.max,
                            "unit": panel.field_config.unit,
                            "decimals": panel.field_config.decimals,
                            "thresholds": panel.field_config.thresholds
                        }
                    },
                    "options": {
                        "legend": panel.options.legend,
                        "tooltip": panel.options.tooltip
                    }
                })
            })
            .collect();
        let grafana_dashboard: _ = json!({
            "dashboard": {
                "id": null,
                "uid": dashboard.uid,
                "title": dashboard.title,
                "description": dashboard.description,
                "tags": dashboard.tags,
                "style": "dark",
                "timezone": "browser",
                "panels": panels,
                "time": {
                    "from": dashboard.time.from,
                    "to": dashboard.time.to
                },
                "refresh": dashboard.refresh.interval,
                "schemaVersion": 30,
                "version": dashboard.version,
                "links": []
            },
            "folderId": 0,
            "overwrite": true
        });
        Ok(grafana_dashboard)
    }
    /// Create or update dashboard in Grafana
    pub async fn create_or_update_dashboard(&self, dashboard: Value) -> Result<()> {
        let url: _ = format!("{}/api/dashboards/db", self.base_url);
        let mut request = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json");
        if let Some(ref api_key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        let response: _ = request
            .json(&dashboard)
            .send()
            .await
            .context("Failed to send dashboard to Grafana")?;
        if !response.status().is_success() {
            let status: _ = response.status();
            let text: _ = response.text().await.unwrap_or_default();
            return Err(anyhow!("Grafana API error: {} - {}", status, text));
        }
        Ok(())
    }
    /// Get dashboard from Grafana
    pub async fn get_dashboard(&self, uid: &str) -> Result<Value> {
        let url: _ = format!("{}/api/dashboards/uid/{}", self.base_url, uid);
        let mut request = self.http_client.get(&url);
        if let Some(ref api_key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        let response: _ = request
            .send()
            .await
            .context("Failed to get dashboard from Grafana")?;
        let dashboard: Value = response
            .json()
            .await
            .context("Failed to parse Grafana response")?;
        Ok(dashboard)
    }
    /// Delete dashboard from Grafana
    pub async fn delete_dashboard(&self, uid: &str) -> Result<()> {
        let url: _ = format!("{}/api/dashboards/uid/{}", self.base_url, uid);
        let mut request = self.http_client.delete(&url);
        if let Some(ref api_key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        let response: _ = request
            .send()
            .await
            .context("Failed to delete dashboard from Grafana")?;
        if !response.status().is_success() {
            let status: _ = response.status();
            let text: _ = response.text().await.unwrap_or_default();
            return Err(anyhow!("Grafana API error: {} - {}", status, text));
        }
        Ok(())
    }
}
impl Dashboard {
    /// Create a new dashboard
    pub fn new(uid: &str, title: &str, description: Option<&str>, tags: Vec<String>) -> Self {
        let now: _ = std::time::SystemTime::now();
        Self {
            uid: uid.to_string(),
            title: title.to_string(),
            description: description.map(|s| s.to_string()),
            tags,
            panels: Vec::new(),
            templating: Vec::new(),
            time: TimeRangeConfig {
                from: "now-1h".to_string(),
                to: "now".to_string(),
            },
            refresh: RefreshConfig {
                interval: "5s".to_string(),
                pause: false,
            },
            version: 1,
            created_at: now,
            updated_at: now,
        }
    }
}
impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(interval: std::time::Duration) -> Self {
        Self {
            interval,
            metrics: Arc::new(Mutex::new(HashMap::new())),
            collectors: Vec::new(),
        }
    }
    /// Add a metrics collector
    pub fn add_collector(&mut self, collector: Box<dyn MetricsCollectorTrait + Send + Sync>) {
        self.collectors.push(collector);
    }
    /// Start metrics collection
    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting metrics collection with {} collectors",
            self.collectors.len()
        );
        let interval: _ = self.interval;
        let metrics: _ = self.metrics.clone();
        let collectors: _ = self.collectors.clone();
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                let mut all_metrics = HashMap::new();
                for collector in &collectors {
                    match collector.collect().await {
                        Ok(mut collector_metrics) => {
                            debug!("Collected metrics from: {}", collector.name());
                            all_metrics.extend(collector_metrics);
                        }
                        Err(e) => {
                            error!("Failed to collect metrics from {}: {}", collector.name(), e);
                        }
                    }
                }
                let mut metrics_write = metrics.write().await;
                *metrics_write = all_metrics;
            }
        });
        Ok(())
    }
    /// Stop metrics collection
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping metrics collection...");
        // In a real implementation, we would cancel the collection task
        Ok(())
    }
    /// Get current metrics snapshot
    pub async fn get_snapshot(&self) -> Result<HashMap<String, Value>> {
        let metrics: _ = self.metrics.read().await;
        Ok(metrics.clone())
    }
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_dashboard_manager_creation() {
        let config: _ = DashboardConfig::default();
        let manager: _ = DashboardManager::new(config).await;
        assert!(manager.is_ok());
    }
    #[tokio::test]
    async fn test_create_dashboard() {
        let config: _ = DashboardConfig::default();
        let manager: _ = DashboardManager::new(config).await.unwrap();
        let uid: _ = manager.create_dashboard("test-dashboard").await;
        assert!(uid.is_ok());
        let uid: _ = uid.unwrap();
        let dashboard: _ = manager.get_dashboard(&uid).await;
        assert!(dashboard.is_some());
        assert_eq!(dashboard.unwrap().title, "test-dashboard");
    }
    #[test]
    fn test_dashboard_creation() {
        let dashboard: _ = Dashboard::new(
            "test-uid",
            "Test Dashboard",
            Some("A test dashboard"),
            vec!["test".to_string()],
        );
        assert_eq!(dashboard.uid, "test-uid");
        assert_eq!(dashboard.title, "Test Dashboard");
        assert_eq!(dashboard.description, Some("A test dashboard".to_string()));
        assert_eq!(dashboard.tags, vec!["test".to_string()]);
        assert_eq!(dashboard.version, 1);
        assert!(dashboard.panels.is_empty());
    }
    #[tokio::test]
    async fn test_add_panel_to_dashboard() {
        let config: _ = DashboardConfig::default();
        let manager: _ = DashboardManager::new(config).await.unwrap();
        let uid: _ = manager.create_dashboard("test-dashboard").await.unwrap();
        let panel: _ = PanelConfig {
            id: "panel-1".to_string(),
            title: "Test Panel".to_string(),
            panel_type: "graph".to_string(),
            grid_pos: GridPos {
                x: 0,
                y: 0,
                w: 12,
                h: 8,
            },
            datasource: "Prometheus".to_string(),
            targets: vec![],
            field_config: FieldConfig {
                min: None,
                max: None,
                unit: None,
                decimals: None,
                thresholds: None,
            },
            options: PanelOptions {
                legend: LegendConfig {
                    show: true,
                    position: "bottom".to_string(),
                },
                tooltip: TooltipConfig {
                    mode: "multi".to_string(),
                    sort: "none".to_string(),
                },
                time: TimeRangeConfig {
                    from: "now-1h".to_string(),
                    to: "now".to_string(),
                },
            },
        };
        let result: _ = manager.add_panel(&uid, panel).await;
        assert!(result.is_ok());
        let dashboard: _ = manager.get_dashboard(&uid).await.unwrap();
        assert_eq!(dashboard.panels.len(), 1);
        assert_eq!(dashboard.panels[0].id, "panel-1");
    }
    #[tokio::test]
    async fn test_grafana_client_conversion() {
        let client: _ =
            GrafanaClient::new("http://localhost:3000".to_string(), None, HttpClient::new());
        let dashboard: _ = Dashboard::new("test-uid", "Test Dashboard", None, vec![]);
        let grafana_dashboard: _ = client.convert_to_grafana_format(&dashboard);
        assert!(grafana_dashboard.is_ok());
        let dashboard_json: _ = grafana_dashboard.unwrap();
        assert!(dashboard_json.get("dashboard").is_some());
    }
    #[tokio::test]
    async fn test_metrics_collection() {
        let mut collector = MetricsCollector::new(std::time::Duration::from_millis(100));
        // Add a mock collector
        collector.add_collector(Box::new(PrometheusCollector::new(
            "http://localhost:9090".to_string(),
        )));
        let result: _ = collector.start().await;
        assert!(result.is_ok());
        // Give it a moment to collect
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let snapshot: _ = collector.get_snapshot().await;
        assert!(snapshot.is_ok());
    }
}
