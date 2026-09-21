// Log Aggregation System for Amber
// 实现结构化日志记录和聚合功能

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions};
use std::sync::{Arc, Mutex};

use tracing::{debug, error, info, warn};
use std::time::Duration;
use std::path::Path;
use std::io::Write;

/// Log level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    /// TRACE level
    Trace,
    /// DEBUG level
    Debug,
    /// INFO level
    Info,
    /// WARN level
    Warn,
    /// ERROR level
    Error,
    /// FATAL level
    Fatal,
}
impl LogLevel {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }
    /// Convert from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            "FATAL" => Some(LogLevel::Fatal),
            _ => None,
        }
    }
}
/// Structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unique ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Log level
    pub level: LogLevel,
    /// Service name
    pub service: String,
    /// Operation name
    pub operation: Option<String>,
    /// Message
    pub message: String,
    /// Additional fields
    pub fields: HashMap<String, serde_json::Value>>,
    /// Source file
    pub file: Option<String>,
    /// Source line
    pub line: Option<u32>,
    /// Source column
    pub column: Option<u32>,
    /// Thread ID
    pub thread_id: Option<String>,
    /// Trace ID (for distributed tracing)
    pub trace_id: Option<String>,
    /// Span ID (for distributed tracing)
    pub span_id: Option<String>,
}
impl LogEntry {
    /// Create a new log entry
    pub fn new(
        level: LogLevel,
        service: String,
        message: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level,
            service,
            operation: None,
            message,
            fields: HashMap::new(),
            file: None,
            line: None,
            column: None,
            thread_id: None,
            trace_id: None,
            span_id: None,
        }
    }
    /// Add a field
    pub fn field<T: Serialize>(&mut self, key: &str, value: T) -> Result<&mut Self> {
        let json_value: _ = serde_json::to_value(value)
            .context("Failed to serialize field value")?;
        self.fields.insert(key.to_string(), json_value);
        Ok(self)
    }
    /// Set operation
    pub fn operation(&mut self, operation: &str) -> &mut Self {
        self.operation = Some(operation.to_string());
        self
    }
    /// Set source location
    pub fn location(&mut self, file: &str, line: u32, column: u32) -> &mut Self {
        self.file = Some(file.to_string());
        self.line = Some(line);
        self.column = Some(column);
        self
    }
    /// Set thread ID
    pub fn thread_id(&mut self, thread_id: &str) -> &mut Self {
        self.thread_id = Some(thread_id.to_string());
        self
    }
    /// Set trace context
    pub fn trace_context(&mut self, trace_id: &str, span_id: &str) -> &mut Self {
        self.trace_id = Some(trace_id.to_string());
        self.span_id = Some(span_id.to_string());
        self
    }
    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
            .context("Failed to serialize log entry to JSON")
    }
    /// Serialize to pretty JSON string
    pub fn to_pretty_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .context("Failed to serialize log entry to pretty JSON")
    }
}
/// Log aggregator configuration
#[derive(Debug, Clone)]
pub struct LogAggregatorConfig {
    /// Service name
    pub service_name: String,
    /// Log directory
    pub log_dir: String,
    /// Maximum log file size (bytes)
    pub max_file_size: u64,
    /// Maximum number of log files to keep
    pub max_files: usize,
    /// Log rotation interval
    pub rotation_interval: chrono::Duration,
    /// Enable JSON formatting
    pub json_format: bool,
    /// Enable file logging
    pub enable_file: bool,
    /// Enable console logging
    pub enable_console: bool,
    /// Minimum log level
    pub min_level: LogLevel,
    /// Enable ELK integration
    pub elk_enabled: bool,
    /// Elasticsearch endpoint
    pub elasticsearch_endpoint: Option<String>,
    /// Logstash endpoint
    pub logstash_endpoint: Option<String>,
}
/// Log aggregator
#[derive(Debug)]
pub struct LogAggregator {
    /// Configuration
    config: LogAggregatorConfig,
    /// Current log file
    current_file: Arc<Mutex<Option<File>>>,
    /// Log queue for async processing
    log_queue: Arc<Mutex<Vec<LogEntry>>>,
    /// Channel for log messages
    log_sender: mpsc::UnboundedSender<LogEntry>,
}
impl LogAggregator {
    /// Create a new LogAggregator
    pub fn new(config: LogAggregatorConfig) -> Result<Self> {
        // Create log directory if it doesn't exist
        if config.enable_file && !Path::new(&config.log_dir).exists() {
            std::fs::create_dir_all(&config.log_dir)
                .context("Failed to create log directory")?;
        }
        let (log_sender, mut log_receiver) = mpsc::unbounded_channel::<LogEntry>();
        // Spawn async log processor
        let log_dir: _ = config.log_dir.clone();
        let current_file: _ = Arc::new(Mutex::new(None)),;
        let current_file_clone: _ = current_file.clone();
        tokio::spawn(async move {
            while let Some(log_entry) = log_receiver.recv().await {
                if let Err(e) = process_log_entry(&log_entry, &log_dir, &current_file_clone).await {
                    error!("Failed to process log entry: {}", e);
                }
            }
        });
        info!("Log aggregator initialized for service: {}", config.service_name);
        Ok(Self {
            config,
            current_file,
            log_queue: Arc::new(Mutex::new(Vec::new()))
            log_sender,
        })
    }
    /// Create a new log entry builder
    pub fn log(&self, level: LogLevel, message: &str) -> LogEntry {
        LogEntry::new(level, self.config.service_name.clone(), message.to_string())
    }
    /// Log at TRACE level
    pub fn trace(&self, message: &str) -> LogEntry {
        self.log(LogLevel::Trace, message)
    }
    /// Log at DEBUG level
    pub fn debug(&self, message: &str) -> LogEntry {
        self.log(LogLevel::Debug, message)
    }
    /// Log at INFO level
    pub fn info(&self, message: &str) -> LogEntry {
        self.log(LogLevel::Info, message)
    }
    /// Log at WARN level
    pub fn warn(&self, message: &str) -> LogEntry {
        self.log(LogLevel::Warn, message)
    }
    /// Log at ERROR level
    pub fn error(&self, message: &str) -> LogEntry {
        self.log(LogLevel::Error, message)
    }
    /// Log at FATAL level
    pub fn fatal(&self, message: &str) -> LogEntry {
        self.log(LogLevel::Fatal, message)
    }
    /// Write a log entry
    pub async fn write(&self, mut log_entry: LogEntry) -> Result<()> {
        // Add source location if available
        let location: _ = std::panic::Location::caller();
        log_entry.location(
            location.file(),
            location.line(),
            location.column(),
        );
        // Add thread ID
        log_entry.thread_id(&format!("{:?}", std::thread::current().id());
        // Send to log channel
        self.log_sender.send(log_entry)
            .context("Failed to send log entry to queue")?;
        Ok(())
    }
    /// Flush all buffered logs
    pub async fn flush(&self) -> Result<()> {
        let mut queue = self.log_queue.lock().unwrap();
        queue.clear();
        Ok(())
    }
    /// Get log statistics
    pub fn get_stats(&self) -> HashMap<String, usize> {
        let queue: _ = self.log_queue.lock().unwrap();
        let mut stats = HashMap::new();
        stats.insert("buffered_logs".to_string(), queue.len());
        // Count logs by level
        for entry in queue.iter() {
            let level_key: _ = format!("logs_{}", entry.level.as_str().to_lowercase());
            *stats.entry(level_key).or_insert(0) += 1;
        }
        stats
    }
}
/// Process a log entry asynchronously
async fn process_log_entry(
    log_entry: &LogEntry,
    log_dir: &str,
    current_file: &Arc<Mutex<Option<File>>>,
) -> Result<()> {
    // Check if we should log this entry based on level
    // This would be implemented based on the aggregator config
    // Format the log entry
    let formatted: _ = if log_entry.fields.is_empty() {
        format!("[{}] {}: {}",
            log_entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            log_entry.level.as_str(),
            log_entry.message)
    } else {
        let json: _ = log_entry.to_json()?;
        format!("[{}] {}: {} | {}",
            log_entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            log_entry.level.as_str(),
            log_entry.message,
            json)
    };
    // Write to file if enabled
    if let Ok(mut file) = current_file.lock() {
        if let Some(ref mut f) = *file {
            writeln!(f, "{}", formatted)
                .context("Failed to write to log file")?;
            f.flush()
                .context("Failed to flush log file")?;
        }
    }
    info!("{}", formatted); // Also log to console
    Ok(())
}
/// Initialize global logger
pub fn init_logger(config: LogAggregatorConfig) -> Result<Arc<LogAggregator>> {
    let aggregator: _ = Arc::new(Mutex::new(LogAggregator::new(config)),?);
    // Set up tracing subscriber
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    info!("Global logger initialized");
    Ok(aggregator))
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_log_entry_creation() {
        let entry: _ = LogEntry::new(
            LogLevel::Info,
            "test-service".to_string(),
            "Test message".to_string(),
        );
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.service, "test-service");
        assert_eq!(entry.message, "Test message");
        assert!(!entry.id.is_empty());
    }
    #[test]
    fn test_log_entry_fields() {
        let mut entry = LogEntry::new(
            LogLevel::Debug,
            "test-service".to_string(),
            "Test message".to_string(),
        );
        entry
            .field("user_id", 12345)
            .unwrap()
            .field("action", "login")
            .unwrap();
        assert_eq!(entry.fields.get("user_id"), Some(&serde_json::Value::from(12345));
        assert_eq!(entry.fields.get("action"), Some(&serde_json::Value::from("login"));
    }
    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }
    #[test]
    fn test_log_entry_json_serialization() {
        let mut entry = LogEntry::new(
            LogLevel::Info,
            "test-service".to_string(),
            "Test message".to_string(),
        );
        entry
            .field("key", "value")
            .unwrap()
            .operation("test_operation");
        let json: _ = entry.to_json();
        assert!(json.is_ok());
    }
    #[test]
    fn test_log_aggregator_creation() {
        let config: _ = LogAggregatorConfig {
            service_name: "test-service".to_string(),
            log_dir: "/tmp/amberjs-logs".to_string(),
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_files: 5,
            rotation_interval: chrono::Duration::hours(1),
            json_format: true,
            enable_file: true,
            enable_console: true,
            min_level: LogLevel::Info,
            elk_enabled: false,
            elasticsearch_endpoint: None,
            logstash_endpoint: None,
        };
        let aggregator: _ = LogAggregator::new(config);
        assert!(aggregator.is_ok());
    }
}