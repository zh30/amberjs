// Alerting system for Amber runtime
//
// This module provides an alerting system that can monitor metrics,
// detect anomalies, and send notifications via various channels.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};

use std::path::Path;
use std::time::SystemTime;
use std::time::{Duration, Instant};
use tracing::{error, info};

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}
impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Critical => write!(f, "critical"),
            AlertSeverity::Warning => write!(f, "warning"),
            AlertSeverity::Info => write!(f, "info"),
        }
    }
}
/// Alert conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Greater than threshold
    GreaterThan(f64),
    /// Less than threshold
    LessThan(f64),
    /// Equal to value
    EqualTo(f64),
    /// Not equal to value
    NotEqualTo(f64),
    /// Between two values (inclusive)
    Between(f64, f64),
    /// Outside range
    Outside(f64, f64),
}
/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Unique identifier for the rule
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Metric name to monitor
    pub metric_name: String,
    /// Alert condition
    pub condition: AlertCondition,
    /// Threshold value
    pub threshold: f64,
    /// Duration for which condition must persist
    pub duration: Duration,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Additional labels
    pub labels: HashMap<String, String>,
    /// Description of the alert
    pub description: String,
}
/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert rule ID
    pub rule_id: String,
    /// Alert name
    pub name: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Current value
    pub value: f64,
    /// Threshold value
    pub threshold: f64,
    /// When the alert was triggered
    pub triggered_at: SystemTime,
    /// Labels
    pub labels: HashMap<String, String>,
    /// Description
    pub description: String,
}
/// Alert notifier interface
pub trait AlertNotifier: Send + Sync {
    /// Send an alert notification
    fn send(&self, alert: &Alert) -> Result<()>;
    /// Get the notifier type
    fn name(&self) -> &str;
}
/// HTTP webhook notifier
pub struct HttpWebhookNotifier {
    url: String,
    headers: HashMap<String, String>,
}
impl HttpWebhookNotifier {
    pub fn new(url: String) -> Self {
        Self {
            url,
            headers: HashMap::new(),
        }
    }
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}
impl AlertNotifier for HttpWebhookNotifier {
    fn send(&self, alert: &Alert) -> Result<()> {
        let payload: _ = serde_json::to_string(alert).context("Failed to serialize alert")?;
        let client: _ = Client::new();
        let mut request = client
            .post(&self.url)
            .body(payload)
            .header("Content-Type", "application/json");
        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        let response: _ = request.send().context("Failed to send webhook")?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Webhook returned status: {}",
                response.status()
            ));
        }
        info!("Alert sent via HTTP webhook to {}", self.url);
        Ok(())
    }
    fn name(&self) -> &str {
        "http_webhook"
    }
}
/// Console notifier (for testing)
pub struct ConsoleNotifier;
impl AlertNotifier for ConsoleNotifier {
    fn send(&self, alert: &Alert) -> Result<()> {
        println!(
            "ALERT [{}] {}: {} (value: {}, threshold: {})",
            alert.severity, alert.name, alert.description, alert.value, alert.threshold
        );
        Ok(())
    }
    fn name(&self) -> &str {
        "console"
    }
}
/// Alerting system
pub struct AlertingSystem {
    /// Alert rules
    rules: Arc<RwLock<Vec<AlertRule>>>,
    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    /// Alert notifiers
    notifiers: Arc<RwLock<Vec<Box<dyn AlertNotifier>>>>,
    /// Check interval
    check_interval: Duration,
    /// Last check time
    last_check: Arc<RwLock<Instant>>,
}
impl AlertingSystem {
    /// Create a new alerting system
    pub fn new() -> Self {
        let mut notifiers = Vec::new();
        notifiers.push(Box::new(ConsoleNotifier) as Box<dyn AlertNotifier>);
        Self {
            rules: Arc::new(Mutex::new(Vec::new())),
            active_alerts: Arc::new(Mutex::new(HashMap::new())),
            notifiers: Arc::new(Mutex::new(notifiers)),
            check_interval: Duration::from_secs(30),
            last_check: Arc::new(Mutex::new(Instant::now())),
        }
    }
    /// Create a new alerting system with custom check interval
    pub fn with_check_interval(interval: Duration) -> Self {
        let mut system = Self::new();
        system.check_interval = interval;
        system
    }
    /// Add an alert rule
    pub async fn add_rule(&self, rule: AlertRule) -> Result<()> {
        let rule_name: _ = rule.name.clone();
        let mut rules = self.rules.write().await;
        rules.push(rule);
        info!("Added alert rule: {}", rule_name);
        Ok(())
    }
    /// Remove an alert rule
    pub async fn remove_rule(&self, rule_id: &str) -> Result<()> {
        let mut rules = self.rules.write().await;
        rules.retain(|r| r.id != rule_id);
        info!("Removed alert rule: {}", rule_id);
        Ok(())
    }
    /// Load alert rules from file
    pub async fn load_rules_from_file(&self, file_path: &str) -> Result<()> {
        let content: _ =
            fs::read_to_string(Path::new(file_path)).context("Failed to read alert rules file")?;
        let rules: Vec<AlertRule> =
            serde_yaml::from_str(&content).context("Failed to parse alert rules")?;
        let mut rules_guard = self.rules.write().await;
        rules_guard.clear();
        rules_guard.extend(rules);
        info!(
            "Loaded {} alert rules from {}",
            rules_guard.len(),
            file_path
        );
        Ok(())
    }
    /// Save alert rules to file
    pub async fn save_rules_to_file(&self, file_path: &str) -> Result<()> {
        let rules: _ = self.rules.read().await;
        let content: _ =
            serde_yaml::to_string(&*rules).context("Failed to serialize alert rules")?;
        fs::write(Path::new(file_path), content).context("Failed to write alert rules file")?;
        info!("Saved {} alert rules to {}", rules.len(), file_path);
        Ok(())
    }
    /// Add an alert notifier
    pub async fn add_notifier(&self, notifier: Box<dyn AlertNotifier>) {
        let mut notifiers = self.notifiers.write().await;
        notifiers.push(notifier);
        info!("Added alert notifier");
    }
    /// Check all alert rules
    pub async fn check_alerts(&self, metrics: &[MetricFamily]) -> Result<Vec<Alert>> {
        let rules: _ = self.rules.read().await;
        let mut triggered_alerts = Vec::new();
        for rule in rules.iter() {
            if let Some(metric) = self.find_metric(metrics, &rule.metric_name) {
                if let Some(alert) = self.check_rule(rule, metric).await? {
                    triggered_alerts.push(alert);
                }
            }
        }
        Ok(triggered_alerts)
    }
    /// Check if it's time to run the alert check
    pub async fn should_check(&self) -> bool {
        let last_check: _ = *self.last_check.read().await;
        last_check.elapsed() >= self.check_interval
    }
    /// Run the alert check loop
    pub async fn run_check_loop(&self, metrics_provider: impl Fn() -> Vec<MetricFamily>) {
        let mut interval = tokio::time::interval(self.check_interval);
        loop {
            interval.tick().await;
            if !self.should_check().await {
                continue;
            }
            let metrics: _ = metrics_provider();
            match self.check_alerts(&metrics).await {
                Ok(alerts) => {
                    for alert in alerts {
                        self.trigger_alert(&alert).await;
                    }
                }
                Err(e) => {
                    error!("Failed to check alerts: {}", e);
                }
            }
            *self.last_check.write().await = Instant::now();
        }
    }
    /// Trigger an alert
    async fn trigger_alert(&self, alert: &Alert) {
        let mut active_alerts = self.active_alerts.write().await;
        // Check if alert is already active
        if active_alerts.contains_key(&alert.rule_id) {
            // Update existing alert
            active_alerts.insert(alert.rule_id.clone(), alert.clone());
            info!("Updated alert: {}", alert.name);
        } else {
            // New alert
            active_alerts.insert(alert.rule_id.clone(), alert.clone());
            info!("Triggered new alert: {}", alert.name);
            // Send notifications
            let notifiers: _ = self.notifiers.read().await;
            for notifier in notifiers.iter() {
                if let Err(e) = notifier.send(alert) {
                    error!("Failed to send alert via {}: {}", notifier.name(), e);
                }
            }
        }
    }
    /// Get active alerts
    pub async fn get_active_alerts(&self) -> HashMap<String, Alert> {
        self.active_alerts.read().await.clone()
    }
    /// Resolve an alert
    pub async fn resolve_alert(&self, rule_id: &str) {
        let mut active_alerts = self.active_alerts.write().await;
        if let Some(alert) = active_alerts.remove(rule_id) {
            info!("Resolved alert: {}", alert.name);
        }
    }
    /// Shutdown the alerting system
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down alerting system...");
        // Clean up resources if needed
        Ok(())
    }
    /// Find a metric by name
    fn find_metric<'a>(
        &self,
        metrics: &'a [MetricFamily],
        metric_name: &str,
    ) -> Option<&'a MetricFamily> {
        metrics.iter().find(|m| m.get_name() == metric_name)
    }
    /// Check a single rule against a metric
    async fn check_rule(&self, rule: &AlertRule, metric: &MetricFamily) -> Result<Option<Alert>> {
        // Get the current value from the metric
        if metric.get_metric().is_empty() {
            return Ok(None);
        }
        let metric_point: _ = &metric.get_metric()[0];
        let value: _ = if metric_point.has_gauge() {
            // Use a simple default value for now - TODO: implement proper metric extraction
            0.0
        } else if metric_point.has_counter() {
            0.0
        } else {
            return Ok(None);
        };
        // Check if condition is met
        let condition_met: _ = match &rule.condition {
            AlertCondition::GreaterThan(threshold) => value > *threshold,
            AlertCondition::LessThan(threshold) => value < *threshold,
            AlertCondition::EqualTo(threshold) => (value - *threshold).abs() < f64::EPSILON,
            AlertCondition::NotEqualTo(threshold) => (value - *threshold).abs() >= f64::EPSILON,
            AlertCondition::Between(min, max) => value >= *min && value <= *max,
            AlertCondition::Outside(min, max) => value < *min || value > *max,
        };
        if condition_met {
            let alert: _ = Alert {
                rule_id: rule.id.clone(),
                name: rule.name.clone(),
                severity: rule.severity.clone(),
                value,
                threshold: rule.threshold,
                triggered_at: SystemTime::now(),
                labels: rule.labels.clone(),
                description: rule.description.clone(),
            };
            Ok(Some(alert))
        } else {
            Ok(None)
        }
    }
}
impl Default for AlertingSystem {
    fn default() -> Self {
        Self::new()
    }
}
/// Built-in alert rules
pub struct BuiltInAlertRules;
impl BuiltInAlertRules {
    /// Get default alert rules for Amber
    pub fn get_default_rules() -> Vec<AlertRule> {
        vec![
            AlertRule {
                id: "high_error_rate".to_string(),
                name: "High Error Rate".to_string(),
                metric_name: "amberjs_script_error_rate_percent".to_string(),
                condition: AlertCondition::GreaterThan(10.0),
                threshold: 10.0,
                duration: Duration::from_secs(60),
                severity: AlertSeverity::Critical,
                labels: HashMap::new(),
                description: "Script error rate is above 10%".to_string(),
            },
            AlertRule {
                id: "high_memory_usage".to_string(),
                name: "High Memory Usage".to_string(),
                metric_name: "amberjs_memory_usage_bytes".to_string(),
                condition: AlertCondition::GreaterThan(1073741824.0), // 1GB
                threshold: 1073741824.0,
                duration: Duration::from_secs(120),
                severity: AlertSeverity::Warning,
                labels: HashMap::new(),
                description: "Memory usage is above 1GB".to_string(),
            },
            AlertRule {
                id: "high_latency".to_string(),
                name: "High Script Latency".to_string(),
                metric_name: "amberjs_script_execution_duration_seconds".to_string(),
                condition: AlertCondition::GreaterThan(1.0), // 1 second
                threshold: 1.0,
                duration: Duration::from_secs(60),
                severity: AlertSeverity::Warning,
                labels: HashMap::new(),
                description: "Script execution latency is above 1 second".to_string(),
            },
        ]
    }
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_alerting_system_creation() {
        let system: _ = AlertingSystem::new();
        assert!(system.rules.read().await.is_empty());
    }
    #[tokio::test]
    async fn test_add_rule() {
        let system: _ = AlertingSystem::new();
        let rule: _ = AlertRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            metric_name: "test_metric".to_string(),
            condition: AlertCondition::GreaterThan(100.0),
            threshold: 100.0,
            duration: Duration::from_secs(60),
            severity: AlertSeverity::Warning,
            labels: HashMap::new(),
            description: "Test alert rule".to_string(),
        };
        system.add_rule(rule).await.unwrap();
        assert_eq!(system.rules.read().await.len(), 1);
    }
    #[test]
    fn test_alert_condition() {
        assert!(AlertCondition::GreaterThan(100.0).is_triggered(150.0));
        assert!(!AlertCondition::GreaterThan(100.0).is_triggered(50.0));
        assert!(AlertCondition::LessThan(100.0).is_triggered(50.0));
        assert!(!AlertCondition::LessThan(100.0).is_triggered(150.0));
        assert!(AlertCondition::Between(50.0, 100.0).is_triggered(75.0));
        assert!(!AlertCondition::Between(50.0, 100.0).is_triggered(25.0));
    }
}
impl AlertCondition {
    fn is_triggered(&self, value: f64) -> bool {
        match self {
            AlertCondition::GreaterThan(threshold) => value > *threshold,
            AlertCondition::LessThan(threshold) => value < *threshold,
            AlertCondition::EqualTo(threshold) => (value - *threshold).abs() < f64::EPSILON,
            AlertCondition::NotEqualTo(threshold) => (value - *threshold).abs() >= f64::EPSILON,
            AlertCondition::Between(min, max) => value >= *min && value <= *max,
            AlertCondition::Outside(min, max) => value < *min || value > *max,
        }
    }
}
