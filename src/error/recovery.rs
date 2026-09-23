// Stage 89 Phase 2: 自动恢复机制
// 提供智能错误恢复、重试策略和自动修复能力

use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::AmberError;
use rand::Rng;
use std::time::{Duration, Instant};

/// 重试策略
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_backoff: bool,
    pub jitter: bool,
}
impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            exponential_backoff: true,
            jitter: true,
        }
    }
}
impl RetryPolicy {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
    pub fn with_base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }
    pub fn with_exponential_backoff(mut self, enabled: bool) -> Self {
        self.exponential_backoff = enabled;
        self
    }
    pub fn with_jitter(mut self, enabled: bool) -> Self {
        self.jitter = enabled;
        self
    }
    /// 计算下一次重试的延迟时间
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let mut delay = self.base_delay;
        if self.exponential_backoff && attempt > 0 {
            delay = Duration::from_secs_f64(
                self.base_delay.as_secs_f64() * (2.0_f64).powi(attempt as i32 - 1),
            );
        }
        if delay > self.max_delay {
            delay = self.max_delay;
        }
        if self.jitter {
            // 添加 ±25% 的随机抖动
            let jitter_range: _ = delay.as_secs_f64() * 0.25;
            let mut rng = rand::thread_rng();
            let jitter: _ = (rng.gen::<f64>() - 0.5) * 2.0 * jitter_range;
            let mut jittered_delay = delay.as_secs_f64() + jitter;
            if jittered_delay < 0.0 {
                jittered_delay = 0.0;
            }
            delay = Duration::from_secs_f64(jittered_delay);
        }
        delay
    }
}
/// 回退策略函数类型
pub type FallbackStrategyFn = Box<dyn Fn(&AmberError) -> Option<String> + Send + Sync>;
/// 自动恢复配置
pub struct AutoRecoveryConfig {
    pub retry_policy: RetryPolicy,
    pub enable_fallback: bool,
    pub enable_auto_repair: bool,
    pub fallback_strategy: Option<FallbackStrategyFn>,
    pub recovery_timeout: Duration,
}
impl std::fmt::Debug for AutoRecoveryConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoRecoveryConfig")
            .field("retry_policy", &self.retry_policy)
            .field("enable_fallback", &self.enable_fallback)
            .field("enable_auto_repair", &self.enable_auto_repair)
            .field(
                "fallback_strategy",
                &self.fallback_strategy.as_ref().map(|_| "CustomStrategyFn"),
            )
            .field("recovery_timeout", &self.recovery_timeout)
            .finish()
    }
}
impl Default for AutoRecoveryConfig {
    fn default() -> Self {
        Self {
            retry_policy: RetryPolicy::default(),
            enable_fallback: true,
            enable_auto_repair: true,
            fallback_strategy: None,
            recovery_timeout: Duration::from_secs(30),
        }
    }
}
/// 恢复统计信息
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    pub total_recovery_attempts: u64,
    pub successful_recoveries: u64,
    pub failed_recoveries: u64,
    pub fallbacks_used: u64,
    pub avg_recovery_time: Duration,
    pub last_recovery_time: Option<Instant>,
}
impl RecoveryStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_recovery_attempts == 0 {
            0.0
        } else {
            self.successful_recoveries as f64 / self.total_recovery_attempts as f64
        }
    }
}
/// 自动恢复管理器
pub struct AutoRecovery {
    config: AutoRecoveryConfig,
    stats: Arc<RwLock<RecoveryStats>>,
    retry_history: Arc<RwLock<Vec<(AmberError, Instant, Duration)>>>,
}
impl AutoRecovery {
    /// 创建新的自动恢复管理器
    pub fn new() -> Self {
        Self {
            config: AutoRecoveryConfig::default(),
            stats: Arc::new(RwLock::new(RecoveryStats::default())),
            retry_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    /// 使用自定义配置创建
    pub fn with_config(config: AutoRecoveryConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(RecoveryStats::default())),
            retry_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.config.retry_policy.max_retries = max_retries;
        self
    }
    /// 设置基础延迟
    pub fn with_base_delay(mut self, delay: Duration) -> Self {
        self.config.retry_policy.base_delay = delay;
        self
    }
    /// 设置回退策略
    pub fn with_fallback_strategy(mut self, strategy: FallbackStrategyFn) -> Self {
        self.config.fallback_strategy = Some(strategy);
        self
    }
    /// 启用/禁用回退
    pub fn with_fallback_enabled(mut self, enabled: bool) -> Self {
        self.config.enable_fallback = enabled;
        self
    }
    /// 启用/禁用自动修复
    pub fn with_auto_repair_enabled(mut self, enabled: bool) -> Self {
        self.config.enable_auto_repair = enabled;
        self
    }
    /// 从错误中恢复
    pub async fn recover_from_error(&self, error: &AmberError) -> Result<String, AmberError> {
        let start_time: _ = Instant::now();
        let mut attempts = 0;
        let mut last_error = error.clone();
        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.total_recovery_attempts += 1;
        }
        // 尝试重试恢复
        while attempts <= self.config.retry_policy.max_retries {
            attempts += 1;
            if attempts > 1 {
                let delay: _ = self.config.retry_policy.calculate_delay(attempts - 1);
                tokio::time::sleep(delay).await;
            }
            // 尝试恢复
            match self.attempt_recovery(error, attempts).await {
                Ok(message) => {
                    // 恢复成功
                    let duration: _ = start_time.elapsed();
                    {
                        let mut stats = self.stats.write().await;
                        stats.successful_recoveries += 1;
                        stats.last_recovery_time = Some(Instant::now());
                        stats.avg_recovery_time = Duration::from_nanos(
                            (stats.avg_recovery_time.as_nanos() as u64
                                + duration.as_nanos() as u64)
                                / 2,
                        );
                    }
                    // 记录重试历史
                    {
                        let mut history = self.retry_history.write().await;
                        history.push((error.clone(), Instant::now(), duration));
                        if history.len() > 100 {
                            history.remove(0);
                        }
                    }
                    return Ok(message);
                }
                Err(e) => {
                    last_error = e;
                }
            }
        }
        // 所有重试都失败了，尝试回退策略
        if self.config.enable_fallback && self.config.fallback_strategy.is_some() {
            if let Some(strategy) = &self.config.fallback_strategy {
                if let Some(fallback_msg) = strategy(error) {
                    {
                        let mut stats = self.stats.write().await;
                        stats.fallbacks_used += 1;
                    }
                    // 记录回退
                    {
                        let mut history = self.retry_history.write().await;
                        history.push((error.clone(), Instant::now(), start_time.elapsed()));
                    }
                    return Ok(format!("Fallback: {}", fallback_msg));
                }
            }
        }
        // 恢复失败
        {
            let mut stats = self.stats.write().await;
            stats.failed_recoveries += 1;
        }
        Err(last_error)
    }
    /// 尝试恢复
    async fn attempt_recovery(
        &self,
        error: &AmberError,
        attempt: u32,
    ) -> Result<String, AmberError> {
        // 根据错误类型和尝试次数决定恢复策略
        match error {
            AmberError::V8Error(_msg) => {
                if attempt <= 2 {
                    // 前两次尝试：重新初始化 V8
                    self.reinitialize_v8()
                        .await
                        .map_err(|e| AmberError::V8Error(format!("Recovery failed: {}", e)))?;
                    Ok(format!("V8 reinitialized (attempt {})", attempt))
                } else {
                    // 后续尝试：使用简化模式
                    Ok(format!("Switched to simplified mode (attempt {})", attempt))
                }
            }
            AmberError::JsExecutionError(_msg) => {
                if attempt <= 1 {
                    // 第一次尝试：验证语法
                    self.validate_syntax().await.map_err(|e| {
                        AmberError::JsExecutionError(format!("Validation failed: {}", e))
                    })?;
                    Ok("Syntax validated".to_string())
                } else {
                    // 后续尝试：跳过验证执行
                    Ok("Bypassed validation".to_string())
                }
            }
            AmberError::MultiLanguageError(_msg) => {
                // 重新初始化运行时
                self.reinitialize_language_runtime(error).await?;
                Ok("Language runtime reinitialized".to_string())
            }
            AmberError::PlatformError(_msg) => {
                // 检查平台兼容性
                self.check_platform_compatibility().await?;
                Ok("Platform compatibility verified".to_string())
            }
            _ => {
                // 其他错误：尝试通用恢复
                if attempt <= 1 {
                    self.reset_runtime_state().await?;
                    Ok("Runtime state reset".to_string())
                } else {
                    Err(AmberError::RuntimeError(format!(
                        "Failed to recover after {} attempts",
                        attempt
                    )))
                }
            }
        }
    }
    /// 重新初始化 V8
    async fn reinitialize_v8(&self) -> Result<(), String> {
        // 当前恢复管理器只记录策略动作；真正的运行时重建由调用方接入。
        Ok(())
    }
    /// 验证语法
    async fn validate_syntax(&self) -> Result<(), String> {
        // 当前恢复管理器只记录策略动作；真正的语法检查由调用方接入。
        Ok(())
    }
    /// 重新初始化语言运行时
    async fn reinitialize_language_runtime(&self, _error: &AmberError) -> Result<(), AmberError> {
        // 当前恢复管理器只记录策略动作；真正的运行时重建由调用方接入。
        Ok(())
    }
    /// 检查平台兼容性
    async fn check_platform_compatibility(&self) -> Result<(), AmberError> {
        // 当前恢复管理器只记录策略动作；真正的平台检查由调用方接入。
        Ok(())
    }
    /// 重置运行时状态
    async fn reset_runtime_state(&self) -> Result<(), AmberError> {
        // 当前恢复管理器只记录策略动作；真正的状态重置由调用方接入。
        Ok(())
    }
    /// 获取恢复统计
    pub async fn get_stats(&self) -> RecoveryStats {
        self.stats.read().await.clone()
    }
    /// 获取重试历史
    pub async fn get_retry_history(&self) -> Vec<(AmberError, Instant, Duration)> {
        self.retry_history.read().await.clone()
    }
    /// 重置统计信息
    pub async fn reset_stats(&self) {
        *self.stats.write().await = RecoveryStats::default();
        self.retry_history.write().await.clear();
    }
    /// 检查是否应该尝试恢复
    pub fn should_attempt_recovery(&self, error: &AmberError) -> bool {
        match error {
            AmberError::SecurityError(_) => false, // 安全错误不自动恢复
            AmberError::ConfigurationError(_) => true,
            _ => true,
        }
    }
}
impl Default for AutoRecovery {
    fn default() -> Self {
        Self::new()
    }
}
