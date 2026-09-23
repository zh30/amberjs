// 分布式追踪系统
// 提供链路追踪、上下文传播和Span管理功能

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
/// 分布式追踪器
#[derive(Debug, Clone)]
pub struct DistributedTracer {
    /// 服务名称
    pub service_name: String,
}
impl DistributedTracer {
    /// 创建新的分布式追踪器
    ///
    /// # Arguments
    ///
    /// * `service_name` - 服务名称
    ///
    /// # Returns
    ///
    /// 返回新的 DistributedTracer 实例
    pub fn new(service_name: String) -> Self {
        Self { service_name }
    }
    /// 开始一个新的追踪 Span
    ///
    /// # Arguments
    ///
    /// * `operation` - 操作名称
    ///
    /// # Returns
    ///
    /// 返回新创建的 Span
    pub fn start_span(&self, operation: &str) -> Span {
        Span::new(operation, None)
    }
    /// 使用现有上下文开始新的 Span
    ///
    /// # Arguments
    ///
    /// * `operation` - 操作名称
    /// * `context` - 追踪上下文（headers）
    ///
    /// # Returns
    ///
    /// 返回新创建的 Span
    pub fn start_span_with_context(
        &self,
        operation: &str,
        context: &HashMap<String, String>,
    ) -> Span {
        let trace_id: _ = context
            .get("trace-id")
            .cloned()
            .unwrap_or_else(|| generate_trace_id());
        let parent_span_id: _ = context.get("span-id").cloned();
        Span::new_with_context(operation, trace_id, parent_span_id)
    }
    /// 注入追踪上下文到 headers
    ///
    /// # Arguments
    ///
    /// * `span` - 要注入的 Span
    /// * `headers` - 目标 headers
    ///
    pub fn inject_context(&self, span: &Span, headers: &mut HashMap<String, String>) {
        headers.insert("trace-id".to_string(), span.trace_id.clone());
        headers.insert("span-id".to_string(), span.span_id.clone());
        if let Some(ref parent_id) = span.parent_span_id {
            headers.insert("parent-span-id".to_string(), parent_id.clone());
        }
    }
    /// 从 headers 提取追踪上下文
    ///
    /// # Arguments
    ///
    /// * `headers` - 源 headers
    ///
    /// # Returns
    ///
    /// 返回追踪上下文
    pub fn extract_context(&self, headers: &HashMap<String, String>) -> Option<TraceContext> {
        let trace_id: _ = headers.get("trace-id")?.clone();
        let span_id: _ = headers.get("span-id")?.clone();
        let baggage: _ = HashMap::new();
        Some(TraceContext {
            trace_id,
            span_id,
            baggage,
        })
    }
}
/// 追踪 Span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// 追踪 ID
    pub trace_id: String,
    /// Span ID
    pub span_id: String,
    /// 父 Span ID
    pub parent_span_id: Option<String>,
    /// 操作名称
    pub operation_name: String,
    /// 开始时间
    pub start_time: SystemTime,
    /// 标签
    pub tags: HashMap<String, String>,
    /// 日志
    pub logs: Vec<String>,
}
impl Span {
    /// 创建新的 Span
    ///
    /// # Arguments
    ///
    /// * `operation` - 操作名称
    /// * `parent_span_id` - 父 Span ID
    ///
    /// # Returns
    ///
    /// 返回新创建的 Span
    fn new(operation: &str, parent_span_id: Option<String>) -> Self {
        Self {
            trace_id: generate_trace_id(),
            span_id: generate_span_id(),
            parent_span_id,
            operation_name: operation.to_string(),
            start_time: SystemTime::now(),
            tags: HashMap::new(),
            logs: Vec::new(),
        }
    }
    /// 使用指定上下文创建 Span
    ///
    /// # Arguments
    ///
    /// * `operation` - 操作名称
    /// * `trace_id` - 追踪 ID
    /// * `parent_span_id` - 父 Span ID
    ///
    /// # Returns
    ///
    /// 返回新创建的 Span
    fn new_with_context(
        operation: &str,
        trace_id: String,
        parent_span_id: Option<String>,
    ) -> Self {
        Self {
            trace_id,
            span_id: generate_span_id(),
            parent_span_id,
            operation_name: operation.to_string(),
            start_time: SystemTime::now(),
            tags: HashMap::new(),
            logs: Vec::new(),
        }
    }
    /// 添加标签
    ///
    /// # Arguments
    ///
    /// * `key` - 标签键
    /// * `value` - 标签值
    pub fn add_tag(&mut self, key: &str, value: &str) {
        self.tags.insert(key.to_string(), value.to_string());
    }
    /// 添加日志事件
    ///
    /// # Arguments
    ///
    /// * `event` - 事件名称
    pub fn log_event(&mut self, event: &str) {
        self.logs.push(event.to_string());
    }
    /// 记录操作耗时
    ///
    /// # Returns
    ///
    /// 返回操作耗时
    pub fn get_duration(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or_else(|_| Duration::from_secs(0))
    }
}
/// 追踪上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// 追踪 ID
    pub trace_id: String,
    /// Span ID
    pub span_id: String,
    /// 附加信息
    pub baggage: HashMap<String, String>,
}
impl TraceContext {
    /// 创建新的追踪上下文
    ///
    /// # Arguments
    ///
    /// * `trace_id` - 追踪 ID
    /// * `span_id` - Span ID
    ///
    /// # Returns
    ///
    /// 返回新的 TraceContext 实例
    pub fn new(trace_id: String, span_id: String) -> Self {
        Self {
            trace_id,
            span_id,
            baggage: HashMap::new(),
        }
    }
    /// 添加 baggage 项目
    ///
    /// # Arguments
    ///
    /// * `key` - 键
    /// * `value` - 值
    pub fn add_baggage(&mut self, key: &str, value: &str) {
        self.baggage.insert(key.to_string(), value.to_string());
    }
    /// 获取 baggage 项目
    ///
    /// # Arguments
    ///
    /// * `key` - 键
    ///
    /// # Returns
    ///
    /// 返回值（如果存在）
    pub fn get_baggage(&self, key: &str) -> Option<&String> {
        self.baggage.get(key)
    }
}
/// 生成追踪 ID（16 字节随机十六进制字符串）
fn generate_trace_id() -> String {
    let bytes: [u8; 16] = rand::random();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
/// 生成 Span ID（8 字节随机十六进制字符串）
fn generate_span_id() -> String {
    let bytes: [u8; 8] = rand::random();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_tracer_creation() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        assert_eq!(tracer.service_name, "amberjs-service");
    }
    #[test]
    fn test_start_span() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        let span: _ = tracer.start_span("api_request");
        assert!(!span.trace_id.is_empty());
        assert!(!span.span_id.is_empty());
        assert_eq!(span.operation_name, "api_request");
        assert_eq!(span.parent_span_id, None);
        assert_eq!(span.tags.len(), 0);
        assert_eq!(span.logs.len(), 0);
    }
    #[test]
    fn test_inject_context() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        let span: _ = tracer.start_span("database_query");
        let mut headers = HashMap::new();
        tracer.inject_context(&span, &mut headers);
        assert_eq!(headers.get("trace-id"), Some(&span.trace_id));
        assert_eq!(headers.get("span-id"), Some(&span.span_id));
        assert_eq!(headers.get("parent-span-id"), None);
    }
    #[test]
    fn test_extract_context() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        let mut headers = HashMap::new();
        headers.insert("trace-id".to_string(), "test-trace-id".to_string());
        headers.insert("span-id".to_string(), "test-span-id".to_string());
        let context: _ = tracer.extract_context(&headers).unwrap();
        assert_eq!(context.trace_id, "test-trace-id");
        assert_eq!(context.span_id, "test-span-id");
    }
    #[test]
    fn test_span_with_context() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        let mut headers = HashMap::new();
        headers.insert("trace-id".to_string(), "parent-trace".to_string());
        headers.insert("span-id".to_string(), "parent-span".to_string());
        let child_span: _ = tracer.start_span_with_context("child_operation", &headers);
        assert_eq!(child_span.trace_id, "parent-trace");
        assert_eq!(child_span.parent_span_id, Some("parent-span".to_string());
        assert_eq!(child_span.operation_name, "child_operation");
    }
    #[test]
    fn test_span_tags() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        let mut span = tracer.start_span("api_request");
        span.add_tag("user_id", "12345");
        span.add_tag("auth_method", "oauth");
        assert_eq!(span.tags.get("user_id"), Some(&"12345".to_string());
        assert_eq!(span.tags.get("auth_method"), Some(&"oauth".to_string());
    }
    #[test]
    fn test_span_logs() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        let mut span = tracer.start_span("api_request");
        span.log_event("request_received");
        span.log_event("response_sent");
        assert_eq!(span.logs.len(), 2);
        assert!(span.logs.contains(&"request_received".to_string());
        assert!(span.logs.contains(&"response_sent".to_string());
    }
    #[test]
    fn test_trace_context_baggage() {
        let mut context = TraceContext::new("trace-123".to_string(), "span-456".to_string());
        context.add_baggage("user_id", "12345");
        context.add_baggage("tenant", "acme");
        assert_eq!(context.get_baggage("user_id"), Some(&"12345".to_string());
        assert_eq!(context.get_baggage("tenant"), Some(&"acme".to_string());
    }
    #[test]
    fn test_span_duration() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        let span: _ = tracer.start_span("api_request");
        std::thread::sleep(Duration::from_millis(10));
        let duration: _ = span.get_duration();
        assert!(duration.as_millis() >= 10);
    }
    #[test]
    fn test_trace_id_generation() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        let span1: _ = tracer.start_span("op1");
        let span2: _ = tracer.start_span("op2");
        // 每个 Span 应该有唯一的 ID
        assert_ne!(span1.span_id, span2.span_id);
        // Trace ID 长度应该是 32（16 字节 * 2）
        assert_eq!(span1.trace_id.len(), 32);
    }
    #[test]
    fn test_span_id_generation() {
        let tracer: _ = DistributedTracer::new("amberjs-service".to_string());
        let span1: _ = tracer.start_span("op1");
        let span2: _ = tracer.start_span("op2");
        // 每个 Span 应该有唯一的 Span ID
        assert_ne!(span1.span_id, span2.span_id);
        // Span ID 长度应该是 16（8 字节 * 2）
        assert_eq!(span1.span_id.len(), 16);
    }
}
use std::collections::{BTreeMap, HashMap};use std::time::Duration;
