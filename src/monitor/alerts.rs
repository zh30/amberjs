// 告警系统模块
// 负责管理告警规则、触发告警、通知渠道和告警历史

use crate::monitor::performance_monitor::{MetricType, ThresholdSeverity, ThresholdViolation};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::SystemTime;

/// 告警规则
#[derive(Debug, Clone)]
pub struct AlertRule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 告警条件
    pub condition: AlertCondition,
    /// 严重程度
    pub severity: AlertSeverity,
    /// 是否启用
    pub enabled: bool,
    /// 标签
    pub tags: HashMap<String, String>,
    /// 静默期 (秒)
    pub silence_period: Duration,
}
/// 告警条件
#[derive(Debug, Clone)]
pub enum AlertCondition {
    /// 大于阈值
    GreaterThan(f64),
    /// 小于阈值
    LessThan(f64),
    /// 等于值
    Equals(f64),
    /// 范围外
    OutOfRange(f64, f64),
    /// 趋势异常 (slope > threshold)
    TrendAnomaly(f64),
}
/// 告警严重程度
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}
/// 告警实例
#[derive(Debug, Clone)]
pub struct AlertInstance {
    /// 告警 ID
    pub id: String,
    /// 规则 ID
    pub rule_id: String,
    /// 告警消息
    pub message: String,
    /// 严重程度
    pub severity: AlertSeverity,
    /// 触发时间
    pub triggered_at: u64,
    /// 解决时间 (可选)
    pub resolved_at: Option<u64>,
    /// 告警数据
    pub data: AlertData,
    /// 状态
    pub status: AlertStatus,
}
/// 告警数据
#[derive(Debug, Clone)]
pub struct AlertData {
    /// 当前值
    pub current_value: f64,
    /// 阈值
    pub threshold: f64,
    /// 指标类型
    pub metric_type: MetricType,
    /// 标签
    pub tags: HashMap<String, String>,
}
/// 告警状态
#[derive(Debug, Clone, PartialEq)]
pub enum AlertStatus {
    Firing,
    Resolved,
    Silenced,
}
/// 通知渠道配置
#[derive(Debug, Clone)]
pub struct NotificationChannel {
    /// 渠道 ID
    pub id: String,
    /// 渠道名称
    pub name: String,
    /// 渠道类型
    pub channel_type: NotificationType,
    /// 配置
    pub config: HashMap<String, String>,
    /// 是否启用
    pub enabled: bool,
}
/// 通知类型
#[derive(Debug, Clone)]
pub enum NotificationType {
    Webhook,
    Email,
    Sms,
    Slack,
    Discord,
}
/// 通知消息
#[derive(Debug, Clone)]
pub struct NotificationMessage {
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
    /// 严重程度
    pub severity: AlertSeverity,
    /// 标签
    pub tags: HashMap<String, String>,
    /// 时间戳
    pub timestamp: u64,
}
/// 告警统计
#[derive(Debug, Clone)]
pub struct AlertStats {
    /// 总告警数
    pub total_alerts: u64,
    /// 正在告警数
    pub active_alerts: u64,
    /// 已解决告警数
    pub resolved_alerts: u64,
    /// 静默告警数
    pub silenced_alerts: u64,
    /// 通知次数
    pub notification_count: u64,
    /// 最后告警时间
    pub last_alert_at: Option<u64>,
}
/// 告警系统配置
#[derive(Debug, Clone)]
pub struct AlertSystemConfig {
    /// 最大历史记录数
    pub max_history: usize,
    /// 默认静默期
    pub default_silence_period: Duration,
    /// 启用通知
    pub enable_notifications: bool,
    /// 通知重试次数
    pub notification_retry_count: u32,
    /// 通知重试间隔
    pub notification_retry_interval: Duration,
}
/// 告警系统
#[derive(Debug)]
pub struct AlertSystem {
    /// 配置
    config: AlertSystemConfig,
    /// 告警规则
    rules: Arc<Mutex<HashMap<String, AlertRule>>>,
    /// 活跃告警
    active_alerts: Arc<Mutex<HashMap<String, AlertInstance>>>,
    /// 告警历史
    alert_history: Arc<Mutex<VecDeque<AlertInstance>>>,
    /// 通知渠道
    notification_channels: Arc<Mutex<HashMap<String, NotificationChannel>>>,
    /// 静默规则
    silence_rules: Arc<Mutex<HashMap<String, SilenceRule>>>,
    /// 统计信息
    stats: Arc<Mutex<AlertStats>>,
}
/// 静默规则
#[derive(Debug, Clone)]
pub struct SilenceRule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: u64,
    /// 匹配的标签
    pub match_tags: HashMap<String, String>,
    /// 创建时间
    pub created_at: u64,
}
/// 通知结果
#[derive(Debug, Clone)]
pub struct NotificationResult {
    /// 是否成功
    pub success: bool,
    /// 错误信息 (如果失败)
    pub error: Option<String>,
    /// 发送时间
    pub sent_at: u64,
}
impl AlertSystem {
    /// 创建新的告警系统
    pub fn new(config: AlertSystemConfig) -> Self {
        Self {
            config,
            rules: Arc::new(Mutex::new(HashMap::new())),
            active_alerts: Arc::new(Mutex::new(HashMap::new())),
            alert_history: Arc::new(Mutex::new(VecDeque::new())),
            notification_channels: Arc::new(Mutex::new(HashMap::new())),
            silence_rules: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(AlertStats {
                total_alerts: 0,
                active_alerts: 0,
                resolved_alerts: 0,
                silenced_alerts: 0,
                notification_count: 0,
                last_alert_at: None,
            })),
        }
    }
    /// 创建默认配置的告警系统
    pub fn with_default_config() -> Self {
        let config: _ = AlertSystemConfig {
            max_history: 10000,
            default_silence_period: Duration::from_secs(300), // 5分钟
            enable_notifications: true,
            notification_retry_count: 3,
            notification_retry_interval: Duration::from_secs(30),
        };
        Self::new(config)
    }
    /// 添加告警规则
    pub fn add_rule(&self, rule: AlertRule) -> Result<(), String> {
        let mut rules = self.rules.lock().map_err(|e| e.to_string())?;
        rules.insert(rule.id.clone(), rule);
        Ok(())
    }
    /// 更新告警规则
    pub fn update_rule(&self, rule: AlertRule) -> Result<(), String> {
        let mut rules = self.rules.lock().map_err(|e| e.to_string())?;
        if !rules.contains_key(&rule.id) {
            return Err(format!("Rule '{}' not found", rule.id));
        }
        rules.insert(rule.id.clone(), rule);
        Ok(())
    }
    /// 删除告警规则
    pub fn delete_rule(&self, rule_id: &str) -> Result<(), String> {
        let mut rules = self.rules.lock().map_err(|e| e.to_string())?;
        rules.remove(rule_id);
        Ok(())
    }
    /// 获取所有告警规则
    pub fn get_rules(&self) -> Result<Vec<AlertRule>, String> {
        let rules: _ = self.rules.lock().map_err(|e| e.to_string())?;
        Ok(rules.values().cloned().collect())
    }
    /// 添加静默规则
    pub fn add_silence_rule(&self, silence_rule: SilenceRule) -> Result<(), String> {
        let mut silence_rules = self.silence_rules.lock().map_err(|e| e.to_string())?;
        silence_rules.insert(silence_rule.id.clone(), silence_rule);
        Ok(())
    }
    /// 删除静默规则
    pub fn delete_silence_rule(&self, rule_id: &str) -> Result<(), String> {
        let mut silence_rules = self.silence_rules.lock().map_err(|e| e.to_string())?;
        silence_rules.remove(rule_id);
        Ok(())
    }
    /// 检查是否应该静默
    fn should_silence(&self, tags: &HashMap<String, String>) -> bool {
        let silence_rules: _ = self.silence_rules.lock().unwrap();
        let current_time: _ = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for silence_rule in silence_rules.values() {
            if current_time >= silence_rule.start_time && current_time <= silence_rule.end_time {
                let mut matches = true;
                for (key, value) in &silence_rule.match_tags {
                    if tags.get(key) != Some(value) {
                        matches = false;
                        break;
                    }
                }
                if matches {
                    return true;
                }
            }
        }
        false
    }
    /// 处理阈值违规
    pub fn handle_threshold_violations(
        &self,
        violations: Vec<ThresholdViolation>,
    ) -> Result<Vec<AlertInstance>, String> {
        let rules: _ = self.rules.lock().map_err(|e| e.to_string())?;
        let mut active_alerts = self.active_alerts.lock().map_err(|e| e.to_string())?;
        let mut alert_history = self.alert_history.lock().map_err(|e| e.to_string())?;
        let mut stats = self.stats.lock().map_err(|e| e.to_string())?;
        let mut triggered_alerts = Vec::new();
        let current_time: _ = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for violation in violations {
            // 查找匹配的规则
            let matching_rules: Vec<AlertRule> = rules
                .values()
                .filter(|rule| rule.enabled && rule.metric_type == violation.metric_type)
                .cloned()
                .collect();
            for rule in matching_rules {
                // 检查条件
                if !self.evaluate_condition(&rule.condition, violation.current_value) {
                    continue;
                }
                // 检查是否应该静默
                if self.should_silence(&HashMap::new()) {
                    continue;
                }
                // 生成告警 ID
                let alert_id: _ = format!("{}_{}", rule.id, current_time);
                // 检查是否已经存在相同规则的活跃告警
                let existing_alert: _ = active_alerts
                    .values()
                    .find(|alert| alert.rule_id == rule.id && alert.status == AlertStatus::Firing);
                if existing_alert.is_some() {
                    continue; // 告警已存在，跳过
                }
                // 创建告警实例
                let alert_instance: _ = AlertInstance {
                    id: alert_id.clone(),
                    rule_id: rule.id.clone(),
                    message: self.generate_alert_message(&rule, violation.current_value),
                    severity: rule.severity.clone(),
                    triggered_at: current_time,
                    resolved_at: None,
                    data: AlertData {
                        current_value: violation.current_value,
                        threshold: violation.threshold_value,
                        metric_type: violation.metric_type.clone(),
                        tags: HashMap::new(),
                    },
                    status: AlertStatus::Firing,
                };
                // 添加到活跃告警
                active_alerts.insert(alert_id.clone(), alert_instance.clone());
                triggered_alerts.push(alert_instance.clone());
                // 更新统计
                stats.total_alerts += 1;
                stats.active_alerts += 1;
                stats.last_alert_at = Some(current_time);
                // 发送到通知渠道
                if self.config.enable_notifications {
                    self.send_notifications(&alert_instance)?;
                }
            }
        }
        // 添加到历史记录
        for alert in &triggered_alerts {
            alert_history.push_back(alert.clone());
            // 限制历史记录大小
            while alert_history.len() > self.config.max_history {
                alert_history.pop_front();
            }
        }
        Ok(triggered_alerts)
    }
    /// 评估告警条件
    fn evaluate_condition(&self, condition: &AlertCondition, value: f64) -> bool {
        match condition {
            AlertCondition::GreaterThan(threshold) => value > *threshold,
            AlertCondition::LessThan(threshold) => value < *threshold,
            AlertCondition::Equals(value_eq) => (value - *value_eq).abs() < f64::EPSILON,
            AlertCondition::OutOfRange(min, max) => value < *min || value > *max,
            AlertCondition::TrendAnomaly(_) => {
                // 简化实现，实际应该计算趋势
                false
            }
        }
    }
    /// 生成告警消息
    fn generate_alert_message(&self, rule: &AlertRule, current_value: f64) -> String {
        format!(
            "Alert: {} - Current value: {:.2}, Threshold: {:.2}",
            rule.name,
            current_value,
            match &rule.condition {
                AlertCondition::GreaterThan(t) => *t,
                AlertCondition::LessThan(t) => *t,
                AlertCondition::Equals(t) => *t,
                AlertCondition::OutOfRange(_, t) => *t,
                AlertCondition::TrendAnomaly(t) => *t,
            }
        )
    }
    /// 解决告警
    pub fn resolve_alert(&self, alert_id: &str) -> Result<(), String> {
        let mut active_alerts = self.active_alerts.lock().map_err(|e| e.to_string())?;
        let mut alert_history = self.alert_history.lock().map_err(|e| e.to_string())?;
        let mut stats = self.stats.lock().map_err(|e| e.to_string())?;
        if let Some(mut alert) = active_alerts.remove(alert_id) {
            alert.status = AlertStatus::Resolved;
            alert.resolved_at = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            alert_history.push_back(alert.clone());
            stats.active_alerts = stats.active_alerts.saturating_sub(1);
            stats.resolved_alerts += 1;
            Ok(())
        } else {
            Err(format!("Alert '{}' not found", alert_id))
        }
    }
    /// 获取活跃告警
    pub fn get_active_alerts(&self) -> Result<Vec<AlertInstance>, String> {
        let active_alerts: _ = self.active_alerts.lock().map_err(|e| e.to_string())?;
        Ok(active_alerts.values().cloned().collect())
    }
    /// 获取告警历史
    pub fn get_alert_history(&self, limit: Option<usize>) -> Result<Vec<AlertInstance>, String> {
        let alert_history: _ = self.alert_history.lock().map_err(|e| e.to_string())?;
        let mut history = alert_history.iter().cloned().collect::<Vec<_>>();
        history.sort_by(|a, b| b.triggered_at.cmp(&a.triggered_at));
        if let Some(limit) = limit {
            history.truncate(limit);
        }
        Ok(history)
    }
    /// 添加通知渠道
    pub fn add_notification_channel(&self, channel: NotificationChannel) -> Result<(), String> {
        let mut notification_channels = self
            .notification_channels
            .lock()
            .map_err(|e| e.to_string())?;
        notification_channels.insert(channel.id.clone(), channel);
        Ok(())
    }
    /// 发送通知
    fn send_notifications(&self, alert: &AlertInstance) -> Result<(), String> {
        let notification_channels: _ = self
            .notification_channels
            .lock()
            .map_err(|e| e.to_string())?;
        let mut stats = self.stats.lock().map_err(|e| e.to_string())?;
        let message: _ = NotificationMessage {
            title: format!("Amber Alert: {}", alert.severity.as_str()),
            content: alert.message.clone(),
            severity: alert.severity.clone(),
            tags: alert.data.tags.clone(),
            timestamp: alert.triggered_at,
        };
        // 简化实现，实际应该发送到真实渠道
        for channel in notification_channels.values() {
            if channel.enabled {
                // 模拟发送
                self.simulate_notification_send(channel, &message)?;
                stats.notification_count += 1;
            }
        }
        Ok(())
    }
    /// 模拟通知发送
    fn simulate_notification_send(
        &self,
        channel: &NotificationChannel,
        message: &NotificationMessage,
    ) -> Result<NotificationResult, String> {
        // 简化实现，实际应该根据渠道类型发送通知
        Ok(NotificationResult {
            success: true,
            error: None,
            sent_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
    /// 获取统计信息
    pub fn get_stats(&self) -> Result<AlertStats, String> {
        let stats: _ = self.stats.lock().map_err(|e| e.to_string())?;
        Ok(stats.clone())
    }
    /// 清理过期告警
    pub fn cleanup_expired_alerts(&self) -> Result<u64, String> {
        let mut active_alerts = self.active_alerts.lock().map_err(|e| e.to_string())?;
        let mut alert_history = self.alert_history.lock().map_err(|e| e.to_string())?;
        let current_time: _ = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let silence_period: _ = self.config.default_silence_period.as_secs();
        let mut cleaned_count = 0;
        // 清理静默的告警
        let silence_rule_tags: HashMap<String, String> = HashMap::new();
        active_alerts.retain(|_, alert| {
            if alert.status == AlertStatus::Silenced {
                if current_time - alert.triggered_at > silence_period {
                    cleaned_count += 1;
                    return false;
                }
            }
            true
        });
        // 限制历史记录大小
        while alert_history.len() > self.config.max_history {
            alert_history.pop_front();
            cleaned_count += 1;
        }
        Ok(cleaned_count)
    }
}
impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "INFO",
            AlertSeverity::Warning => "WARNING",
            AlertSeverity::Critical => "CRITICAL",
        }
    }
    pub fn as_u8(&self) -> u8 {
        match self {
            AlertSeverity::Info => 1,
            AlertSeverity::Warning => 2,
            AlertSeverity::Critical => 3,
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_alert_system_creation() {
        let alert_system: _ = AlertSystem::with_default_config();
        assert!(alert_system.get_stats().is_ok());
    }
    #[test]
    fn test_add_alert_rule() {
        let alert_system: _ = AlertSystem::with_default_config();
        let rule: _ = AlertRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            description: "Test description".to_string(),
            metric_type: MetricType::CpuUsage,
            condition: AlertCondition::GreaterThan(80.0),
            severity: AlertSeverity::Warning,
            enabled: true,
            tags: HashMap::new(),
            silence_period: Duration::from_secs(300),
        };
        assert!(alert_system.add_rule(rule).is_ok());
        let rules: _ = alert_system.get_rules().unwrap();
        assert_eq!(rules.len(), 1);
    }
    #[test]
    fn test_handle_threshold_violations() {
        let alert_system: _ = AlertSystem::with_default_config();
        // 添加规则
        let rule: _ = AlertRule {
            id: "cpu_rule".to_string(),
            name: "CPU High".to_string(),
            description: "CPU usage too high".to_string(),
            metric_type: MetricType::CpuUsage,
            condition: AlertCondition::GreaterThan(80.0),
            severity: AlertSeverity::Critical,
            enabled: true,
            tags: HashMap::new(),
            silence_period: Duration::from_secs(300),
        };
        alert_system.add_rule(rule).unwrap();
        // 创建阈值违规
        let violation: _ = ThresholdViolation {
            metric_type: MetricType::CpuUsage,
            current_value: 95.0,
            threshold_value: 80.0,
            severity: ThresholdSeverity::Critical,
            timestamp: 1234567890,
        };
        let alerts: _ = alert_system
            .handle_threshold_violations(vec![violation])
            .unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
    }
    #[test]
    fn test_resolve_alert() {
        let alert_system: _ = AlertSystem::with_default_config();
        // 添加规则
        let rule: _ = AlertRule {
            id: "cpu_rule".to_string(),
            name: "CPU High".to_string(),
            description: "CPU usage too high".to_string(),
            metric_type: MetricType::CpuUsage,
            condition: AlertCondition::GreaterThan(80.0),
            severity: AlertSeverity::Critical,
            enabled: true,
            tags: HashMap::new(),
            silence_period: Duration::from_secs(300),
        };
        alert_system.add_rule(rule).unwrap();
        // 创建阈值违规
        let violation: _ = ThresholdViolation {
            metric_type: MetricType::CpuUsage,
            current_value: 95.0,
            threshold_value: 80.0,
            severity: ThresholdSeverity::Critical,
            timestamp: 1234567890,
        };
        let alerts: _ = alert_system
            .handle_threshold_violations(vec![violation])
            .unwrap();
        let alert_id: _ = &alerts[0].id;
        // 解决告警
        assert!(alert_system.resolve_alert(alert_id).is_ok());
        let active_alerts: _ = alert_system.get_active_alerts().unwrap();
        assert_eq!(active_alerts.len(), 0);
    }
    #[test]
    fn test_get_alert_history() {
        let alert_system: _ = AlertSystem::with_default_config();
        // 添加规则
        let rule: _ = AlertRule {
            id: "cpu_rule".to_string(),
            name: "CPU High".to_string(),
            description: "CPU usage too high".to_string(),
            metric_type: MetricType::CpuUsage,
            condition: AlertCondition::GreaterThan(80.0),
            severity: AlertSeverity::Critical,
            enabled: true,
            tags: HashMap::new(),
            silence_period: Duration::from_secs(300),
        };
        alert_system.add_rule(rule).unwrap();
        // 创建阈值违规
        let violation: _ = ThresholdViolation {
            metric_type: MetricType::CpuUsage,
            current_value: 95.0,
            threshold_value: 80.0,
            severity: ThresholdSeverity::Critical,
            timestamp: 1234567890,
        };
        alert_system
            .handle_threshold_violations(vec![violation])
            .unwrap();
        // 获取历史
        let history: _ = alert_system.get_alert_history(None).unwrap();
        assert_eq!(history.len(), 1);
    }
    #[test]
    fn test_evaluate_condition() {
        let alert_system: _ = AlertSystem::with_default_config();
        let conditions: _ = vec![
            (AlertCondition::GreaterThan(50.0), 60.0, true),
            (AlertCondition::LessThan(50.0), 40.0, true),
            (AlertCondition::Equals(50.0), 50.0, true),
            (AlertCondition::OutOfRange(10.0, 90.0), 95.0, true),
        ];
        for (condition, value, expected) in conditions {
            assert_eq!(alert_system.evaluate_condition(&condition, value), expected);
        }
    }
    #[test]
    fn test_add_silence_rule() {
        let alert_system: _ = AlertSystem::with_default_config();
        let silence_rule: _ = SilenceRule {
            id: "silence_1".to_string(),
            name: "Silence Test".to_string(),
            start_time: 1234567890,
            end_time: 1234567890 + 3600,
            match_tags: HashMap::new(),
            created_at: 1234567890,
        };
        assert!(alert_system.add_silence_rule(silence_rule).is_ok());
    }
    #[test]
    fn test_notification_channel() {
        let alert_system: _ = AlertSystem::with_default_config();
        let channel: _ = NotificationChannel {
            id: "webhook_1".to_string(),
            name: "Webhook".to_string(),
            channel_type: NotificationType::Webhook,
            config: HashMap::new(),
            enabled: true,
        };
        assert!(alert_system.add_notification_channel(channel).is_ok());
    }
    #[test]
    fn test_alert_severity() {
        assert_eq!(AlertSeverity::Info.as_str(), "INFO");
        assert_eq!(AlertSeverity::Warning.as_str(), "WARNING");
        assert_eq!(AlertSeverity::Critical.as_str(), "CRITICAL");
        assert_eq!(AlertSeverity::Info.as_u8(), 1);
        assert_eq!(AlertSeverity::Warning.as_u8(), 2);
        assert_eq!(AlertSeverity::Critical.as_u8(), 3);
    }
}
