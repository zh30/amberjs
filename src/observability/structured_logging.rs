// Structured logging for Amber runtime
//
// This module provides structured logging capabilities with JSON formatting,
// correlation IDs, and context-aware logging for better observability.

use anyhow::{Error, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::task::Context;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::{FormatEvent, FormatFields};
use tracing_subscriber::{EnvFilter, Registry};

/// Structured logger with JSON formatting and context support
pub struct StructuredLogger {
    /// Log level filter
    level: Level,
    /// Service name for all logs
    service_name: String,
    /// Environment (e.g., development, production)
    environment: String,
    /// Log file path (optional)
    log_file: Option<Arc<Mutex<File>>>,
    /// Context data (correlation IDs, etc.)
    context: Arc<RwLock<HashMap<String, Value>>>,
}
impl StructuredLogger {
    /// Create a new structured logger
    pub fn new(level: Level, service_name: String) -> Self {
        let environment: _ =
            std::env::var("AMBER_ENV").unwrap_or_else(|_| "development".to_string());
        Self {
            level,
            service_name,
            environment,
            log_file: None,
            context: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    /// Create a new structured logger with file output
    pub fn new_with_file(level: Level, service_name: String, log_file_path: &str) -> Result<Self> {
        let path: _ = Path::new(log_file_path);
        let file: _ =
            File::create(path).map_err(|e| anyhow::anyhow!("Failed to create log file: {}", e))?;
        let mut logger = Self::new(level, service_name);
        logger.log_file = Some(Arc::new(Mutex::new(file)));
        Ok(logger)
    }
    /// Set a correlation ID for all subsequent logs
    pub async fn set_correlation_id(&self, correlation_id: String) {
        let mut context = self.context.write().await;
        context.insert("correlation_id".to_string(), json!(correlation_id));
    }
    /// Add context data
    pub async fn add_context(&self, key: String, value: Value) {
        let mut context = self.context.write().await;
        context.insert(key, value);
    }
    /// Get a clone of current context
    pub async fn get_context(&self) -> HashMap<String, Value> {
        self.context.read().await.clone()
    }
    /// Log at INFO level
    pub async fn info(&self, message: &str, context: HashMap<String, Value>) {
        self.log_with_level(Level::INFO, message, context).await;
    }
    /// Log at WARN level
    pub async fn warn(&self, message: &str, context: HashMap<String, Value>) {
        self.log_with_level(Level::WARN, message, context).await;
    }
    /// Log at ERROR level
    pub async fn error(&self, message: &str, context: HashMap<String, Value>) {
        self.log_with_level(Level::ERROR, message, context).await;
    }
    /// Log at DEBUG level
    pub async fn debug(&self, message: &str, context: HashMap<String, Value>) {
        self.log_with_level(Level::DEBUG, message, context).await;
    }
    /// Log at TRACE level
    pub async fn trace(&self, message: &str, context: HashMap<String, Value>) {
        self.log_with_level(Level::TRACE, message, context).await;
    }
    /// Internal logging function
    async fn log_with_level(
        &self,
        level: Level,
        message: &str,
        mut context: HashMap<String, Value>,
    ) {
        // Merge with global context
        let global_context: _ = self.context.read().await;
        for (key, value) in global_context.iter() {
            if !context.contains_key(key) {
                context.insert(key.clone(), value.clone());
            }
        }
        // Clone context for tracing before moving it
        let context_clone: _ = context.clone();
        // Create log entry
        let log_entry: _ = self.create_log_entry(level, message, context);
        // Output to file if configured
        if let Some(file) = &self.log_file {
            if let Ok(mut file) = file.lock() {
                let log_line: _ = format!("{}\n", log_entry.to_string());
                let _: _ = file.write_all(log_line.as_bytes());
                let _: _ = file.flush();
            }
        }
        // Output to stdout using tracing
        match level {
            Level::TRACE => tracing::trace!(message, context = ?context_clone),
            Level::DEBUG => tracing::debug!(message, context = ?context_clone),
            Level::INFO => tracing::info!(message, context = ?context_clone),
            Level::WARN => tracing::warn!(message, context = ?context_clone),
            Level::ERROR => tracing::error!(message, context = ?context_clone),
        }
    }
    /// Create a JSON log entry
    fn create_log_entry(
        &self,
        level: Level,
        message: &str,
        context: HashMap<String, Value>,
    ) -> Value {
        let timestamp: _ = chrono::Utc::now();
        let mut log_entry = json!({
            "timestamp": timestamp.to_rfc3339(),
            "level": format!("{}", level).to_lowercase(),
            "message": message,
            "service": self.service_name,
            "environment": self.environment,
        });
        // Merge context
        if let Value::Object(ref mut map) = log_entry {
            for (key, value) in context {
                map.insert(key, value);
            }
        }
        log_entry
    }
    /// Get the service name
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
    /// Get the log level
    pub fn level(&self) -> Level {
        self.level
    }
}
/// JSON formatter for tracing events
pub struct JsonFormatter {
    service_name: String,
}
impl JsonFormatter {
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }
}
impl<S, N> FormatEvent<S, N> for JsonFormatter
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<S, N>,
        mut writer: Writer,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let timestamp: _ = chrono::Utc::now();
        // Extract fields
        let fields: _ = HashMap::new();
        let mut message = String::new();
        // Use simplified approach for field extraction
        let level_str: _ = "info";
        let target_str: _ = "amberjs";
        // Simplified field extraction
        message = "event".to_string();
        // Create JSON log entry
        let mut log_entry = serde_json::Map::new();
        log_entry.insert("timestamp".to_string(), json!(timestamp.to_rfc3339()));
        log_entry.insert("level".to_string(), json!(level_str));
        log_entry.insert("message".to_string(), json!(message));
        log_entry.insert("target".to_string(), json!(target_str));
        log_entry.insert("service".to_string(), json!(self.service_name));
        // Merge fields
        for (key, value) in fields {
            log_entry.insert(key, value);
        }
        // Write to writer
        write!(
            writer,
            "{}",
            serde_json::to_string(&log_entry).unwrap_or_default()
        )?;
        writeln!(writer)?;
        Ok(())
    }
}
/// Initialize structured logging
pub fn init_structured_logging(
    level: Level,
    service_name: &str,
) -> Result<Box<dyn Subscriber + Send + Sync>> {
    let env_filter: _ =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.as_str()));
    let json_formatter: _ = JsonFormatter::new(service_name.to_string());
    let subscriber: _ = Registry::default()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().event_format(json_formatter));
    Ok(Box::new(subscriber) as Box<dyn Subscriber + Send + Sync>)
}
/// Script execution logger
pub struct ScriptLogger<'a> {
    logger: &'a StructuredLogger,
    script_name: String,
    correlation_id: String,
}
impl<'a> ScriptLogger<'a> {
    /// Create a new script logger
    pub fn new(logger: &'a StructuredLogger, script_name: &str) -> Self {
        let correlation_id: _ = uuid::Uuid::new_v4().to_string();
        Self {
            logger,
            script_name: script_name.to_string(),
            correlation_id,
        }
    }
    /// Log script start
    pub async fn log_start(&self) {
        let context: _ = HashMap::from([
            ("script_name".to_string(), json!(self.script_name)),
            ("event_type".to_string(), json!("script_start")),
            ("correlation_id".to_string(), json!(self.correlation_id)),
        ]);
        self.logger.info("Script started", context).await;
    }
    /// Log script end
    pub async fn log_end(&self, duration_ms: u64, success: bool) {
        let context: _ = HashMap::from([
            ("script_name".to_string(), json!(self.script_name)),
            ("event_type".to_string(), json!("script_end")),
            ("duration_ms".to_string(), json!(duration_ms)),
            ("success".to_string(), json!(success)),
            ("correlation_id".to_string(), json!(self.correlation_id)),
        ]);
        if success {
            self.logger
                .info("Script completed successfully", context)
                .await;
        } else {
            self.logger.error("Script execution failed", context).await;
        }
    }
    /// Log script error
    pub async fn log_error(&self, error: &str) {
        let context: _ = HashMap::from([
            ("script_name".to_string(), json!(self.script_name)),
            ("event_type".to_string(), json!("script_error")),
            ("error".to_string(), json!(error)),
            ("correlation_id".to_string(), json!(self.correlation_id)),
        ]);
        self.logger.error("Script error occurred", context).await;
    }
}
/// Performance logger
pub struct PerformanceLogger<'a> {
    logger: &'a StructuredLogger,
    operation_name: String,
    correlation_id: String,
}
impl<'a> PerformanceLogger<'a> {
    /// Create a new performance logger
    pub fn new(logger: &'a StructuredLogger, operation_name: &str) -> Self {
        let correlation_id: _ = uuid::Uuid::new_v4().to_string();
        Self {
            logger,
            operation_name: operation_name.to_string(),
            correlation_id,
        }
    }
    /// Log operation start
    pub async fn log_start(&self) {
        let context: _ = HashMap::from([
            ("operation".to_string(), json!(self.operation_name)),
            ("event_type".to_string(), json!("operation_start")),
            ("correlation_id".to_string(), json!(self.correlation_id)),
        ]);
        self.logger.debug("Operation started", context).await;
    }
    /// Log operation completion
    pub async fn log_completion(&self, duration_ms: u64, success: bool) {
        let context: _ = HashMap::from([
            ("operation".to_string(), json!(self.operation_name)),
            ("event_type".to_string(), json!("operation_end")),
            ("duration_ms".to_string(), json!(duration_ms)),
            ("success".to_string(), json!(success)),
            ("correlation_id".to_string(), json!(self.correlation_id)),
        ]);
        if success {
            self.logger.debug("Operation completed", context).await;
        } else {
            self.logger.warn("Operation failed", context).await;
        }
    }
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_structured_logger_creation() {
        let logger: _ = StructuredLogger::new(Level::INFO, "amberjs".to_string());
        assert_eq!(logger.service_name(), "amberjs");
        assert_eq!(logger.level(), Level::INFO);
    }
    #[tokio::test]
    async fn test_log_with_context() {
        let logger: _ = StructuredLogger::new(Level::INFO, "amberjs".to_string());
        let context: _ = HashMap::from([
            ("key1".to_string(), json!("value1")),
            ("key2".to_string(), json!(42)),
        ]);
        logger.info("Test message", context).await;
    }
    #[tokio::test]
    async fn test_correlation_id() {
        let logger: _ = StructuredLogger::new(Level::INFO, "amberjs".to_string());
        logger
            .set_correlation_id("test-correlation-id".to_string())
            .await;
        let context: _ = HashMap::new();
        logger.info("Test message with correlation", context).await;
    }
    #[tokio::test]
    async fn test_script_logger() {
        let logger: _ = StructuredLogger::new(Level::INFO, "amberjs".to_string());
        let script_logger: _ = ScriptLogger::new(&logger, "test.js");
        script_logger.log_start().await;
        script_logger.log_end(100, true).await;
    }
    #[tokio::test]
    async fn test_performance_logger() {
        let logger: _ = StructuredLogger::new(Level::DEBUG, "amberjs".to_string());
        let perf_logger: _ = PerformanceLogger::new(&logger, "test_operation");
        perf_logger.log_start().await;
        perf_logger.log_completion(50, true).await;
    }
}
