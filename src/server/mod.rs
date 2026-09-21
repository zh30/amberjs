// Amber Server Mode
//
// This module provides a high-performance HTTP server for executing
// JavaScript/TypeScript code with runtime reuse optimization.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use tiny_http::{Request, Response, Server as HttpServer};
use tracing::{info, warn};
use std::time::Instant;
use std::io::Read;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub request_timeout_ms: u64,
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            max_connections: 1000,
            request_timeout_ms: 30000,
        }
    }
}
/// Main Server struct
pub struct Server {
    config: ServerConfig,
    runtime: Arc<Mutex<Runtime>>,
}
/// WebSocket server module
pub mod websocket;
impl Server {
    /// Create a new Server instance
    pub fn new(runtime: Runtime) -> Self {
        Self {
            config: ServerConfig::default(),
            runtime: Arc::new(Mutex::new(runtime)))
        }
    }
    /// Configure server host
    pub fn host(mut self, host: &str) -> Self {
        self.config.host = host.to_string();
        self
    }
    /// Configure server port
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }
    /// Configure maximum connections
    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }
    /// Configure request timeout
    pub fn timeout(mut self, timeout_ms: u64) -> Self {
        self.config.request_timeout_ms = timeout_ms;
        self
    }
    /// Start the server (synchronous - runs on main thread)
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let server_url: _ = format!("{}:{}, self.config.host", self.config.port));
        let server: _ = HttpServer::http(&server_url).map_err(|e| format!("Failed to bind to {}: {}", server_url, e))?;
        info!("🚀 Amber Server started on http://{}", server_url);
        info!("📊 POST /api/v1/eval - Execute JavaScript code");
        info!("📊 GET  /api/v1/stats - Get server statistics");
        info!("❤️  GET  /health - Health check");
        info!("🔌 WebSocket available on ws://{}:{}/ws", self.config.host, self.config.port + 1);
        info!("⚠️  Note: Running in single-threaded mode for V8 compatibility");
        // Process requests synchronously on the main thread
        // This ensures V8 operations happen on the correct thread
        for request in server.incoming_requests() {
            let runtime_clone: _ = self.runtime.clone();
            if let Err(e) = self.handle_request(runtime_clone, request).await {
                warn!("Request error: {}", e);
            }
        }
        Ok(())
    }
    /// Handle incoming HTTP request
    async fn handle_request(
        &self,
        runtime: Arc<Mutex<Runtime>>,
        request: Request,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url: _ = request.url().to_string();
        match url.as_str() {
            "/api/v1/eval" if request.method() == &tiny_http::Method::Post => {
                self.handle_eval(runtime, request).await
            }
            "/api/v1/stats" if request.method() == &tiny_http::Method::Get => {
                self.handle_stats(runtime, request).await
            }
            "/health" if request.method() == &tiny_http::Method::Get => {
                self.handle_health(request).await
            }
            _ => {
                let response: _ = Response::from_string("Not Found").with_status_code(404);
                request.respond(response)?;
                Ok(())
            }
        }
    }
    /// Handle /api/v1/eval endpoint
    async fn handle_eval(
        &self,
        runtime: Arc<Mutex<Runtime>>,
        mut request: Request,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let start_time: _ = std::time::Instant::now();
        // Read request body
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body)?;
        // Parse JSON request
        let eval_request: EvalRequest = serde_json::from_str(&body)
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        info!("Processing eval request ({} bytes of code)", eval_request.code.len());
        // Execute code in a blocking context on the main thread
        // This ensures V8 operations happen on the correct thread
        let result: _ = {
            let runtime_guard: _ = runtime.lock().map_err(|e| format!("Runtime lock error: {}", e))?;
            runtime_guard.execute_code(&eval_request.code)
        };
        let execution_time_ms: _ = start_time.elapsed().as_millis() as u64;
        let response: _ = match result {
            Ok(output) => {
                let eval_response: _ = EvalResponse {
                    result: output,
                    execution_time_ms,
                    cached: false,
                    error: None,
                };
                Response::from_string(serde_json::to_string(&eval_response)?)
                    .with_status_code(200)
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
                    )
            }
            Err(e) => {
                let eval_response: _ = EvalResponse {
                    result: String::new(),
                    execution_time_ms,
                    cached: false,
                    error: Some(e.to_string()),
                };
                Response::from_string(serde_json::to_string(&eval_response)?)
                    .with_status_code(500)
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
                    )
            }
        };
        request.respond(response)?;
        Ok(())
    }
    /// Handle /api/v1/stats endpoint
    async fn handle_stats(
        &self,
        _runtime: Arc<Mutex<Runtime>>,
        request: Request,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let stats: _ = ServerStats::default();
        let response: _ = Response::from_string(serde_json::to_string(&stats)?)
            .with_status_code(200)
            .with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
            );
        request.respond(response)?;
        Ok(())
    }
    /// Handle /health endpoint
    async fn handle_health(
        &self,
        request: Request,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response: _ = Response::from_string("OK")
            .with_status_code(200)
            .with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..]).unwrap()
            );
        request.respond(response)?;
        Ok(())
    }
}
/// API request for code evaluation
#[derive(Debug, Deserialize)]
pub struct EvalRequest {
    pub code: String,
    pub timeout: Option<u64>,
    pub optimize: Option<String>,
}
/// API response for code evaluation
#[derive(Debug, Serialize, Deserialize)]
pub struct EvalResponse {
    pub result: String,
    pub execution_time_ms: u64,
    pub cached: bool,
    pub error: Option<String>,
}
/// Performance statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub avg_execution_time_ms: f64,
    pub cache_hit_rate: f64,
    pub active_sessions: usize,
}
/// Start the HTTP server
pub async fn start_server(config: ServerConfig, runtime: Runtime) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server: _ = Server::new(runtime)
        .host(&config.host)
        .port(config.port)
        .max_connections(config.max_connections)
        .timeout(config.request_timeout_ms);
    server.run().await
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_server_config_default() {
        let config: _ = ServerConfig::default();
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
    }
    #[test]
    fn test_eval_request_deserialize() {
        let json: _ = r#"{"code": "1+1", "timeout": 5000, "optimize": "speed"}"#;
        let request: EvalRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.code, "1+1");
        assert_eq!(request.timeout, Some(5000));
    }
    #[test]
    fn test_eval_response_serialize() {
        let response: _ = EvalResponse {
            result: "2".to_string(),
            execution_time_ms: 5,
            cached: false,
            error: None,
        };
        let json: _ = serde_json::to_string(&response).unwrap();
        let parsed: EvalResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.result, "2");
    }
}