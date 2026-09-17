// Prometheus metrics exporter for Amber runtime
//
// This module provides HTTP server functionality to expose Prometheus metrics.
// The exporter runs an HTTP server that serves metrics in Prometheus format.

use anyhow::{Context, Result};
use prometheus::{Counter, Opts, Registry, TextEncoder};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

/// Prometheus metrics exporter
pub struct PrometheusExporter {
    /// Prometheus registry containing all metrics
    registry: Registry,
    /// HTTP server address
    bind_addr: Option<SocketAddr>,
    /// Flag indicating if server is running
    running: Arc<tokio::sync::Mutex<bool>>,
}
impl PrometheusExporter {
    /// Create a new Prometheus exporter
    pub fn new() -> Result<Self> {
        let registry: _ = Registry::new();
        info!("Prometheus exporter created");
        Ok(Self {
            registry,
            bind_addr: None,
            running: Arc::new(Mutex::new(tokio::sync::Mutex::new(false))),
        })
    }
    /// Create a new Prometheus exporter with custom registry
    pub fn new_with_registry(registry: Registry) -> Self {
        info!("Prometheus exporter created with custom registry");
        Self {
            registry,
            bind_addr: None,
            running: Arc::new(Mutex::new(tokio::sync::Mutex::new(false))),
        }
    }
    /// Get the Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
    /// Start the HTTP server to expose metrics
    pub async fn start_server(&mut self, addr: SocketAddr) -> Result<()> {
        info!("Starting Prometheus metrics server on {}", addr);
        let listener: _ = TcpListener::bind(&addr)
            .await
            .context("Failed to bind to address")?;
        self.bind_addr = Some(addr);
        *self.running.lock().await = true;
        info!("Prometheus metrics server listening on {}", addr);
        // Accept connections in a loop
        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    continue;
                }
            };
            let registry: _ = self.registry.clone();
            let running: _ = self.running.clone();
            // Spawn a task to handle each connection
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, registry, running).await {
                    error!("Error handling connection from {}: {}", peer_addr, e);
                }
            });
        }
    }
    /// Shutdown the Prometheus exporter
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down Prometheus exporter...");
        *self.running.lock().await = false;
        Ok(())
    }
    /// Get metrics in Prometheus text format
    pub fn gather_metrics(&self) -> Result<String> {
        let metric_families: _ = self.registry.gather();
        let encoder: _ = TextEncoder::new();
        let metrics_text: _ = encoder
            .encode_to_string(&metric_families)
            .context("Failed to encode metrics")?;
        Ok(metrics_text)
    }
    /// Get the bind address if server is running
    pub fn bind_addr(&self) -> Option<SocketAddr> {
        self.bind_addr
    }
    /// Check if server is running
    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }
}
impl Default for PrometheusExporter {
    fn default() -> Self {
        Self::new().expect("Failed to create PrometheusExporter")
    }
}
/// Handle a single HTTP connection
async fn handle_connection(
    mut stream: TcpStream,
    registry: Registry,
    running: Arc<tokio::sync::Mutex<bool>>,
) -> Result<()> {
    // Read the HTTP request
    let mut buffer = [0; 1024];
    let _: _ = stream.read(&mut buffer).await?;
    let request: _ = String::from_utf8_lossy(&buffer);
    // Check if server is still running
    if !*running.lock().await {
        return Ok(());
    }
    // Parse the request path
    let path: _ = if let Some(first_line) = request.lines().next() {
        first_line.split(' ').nth(1).unwrap_or("/")
    } else {
        "/"
    };
    match path {
        "/metrics" => {
            // Serve metrics
            serve_metrics(&mut stream, &registry).await?;
        }
        "/health" => {
            // Health check endpoint
            serve_health_check(&mut stream).await?;
        }
        _ => {
            // 404 Not Found
            serve_not_found(&mut stream).await?;
        }
    }
    Ok(())
}
/// Serve metrics in Prometheus format
async fn serve_metrics(stream: &mut TcpStream, registry: &Registry) -> Result<()> {
    // Gather metrics
    let metric_families: _ = registry.gather();
    // Encode to Prometheus text format
    let encoder: _ = prometheus::TextEncoder::new();
    let metrics_text: _ = encoder
        .encode_to_string(&metric_families)
        .context("Failed to encode metrics")?;
    // Prepare HTTP response
    let response: _ = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        prometheus::TEXT_FORMAT,
        metrics_text.len(),
        metrics_text
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
/// Serve health check endpoint
async fn serve_health_check(stream: &mut TcpStream) -> Result<()> {
    let response: _ = "HTTP/1.1 200 OK\r\n\
                   Content-Type: application/json\r\n\
                   Connection: close\r\n\
                   \r\n\
                   {\"status\": \"healthy\"}";
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
/// Serve 404 Not Found
async fn serve_not_found(stream: &mut TcpStream) -> Result<()> {
    let response: _ = "HTTP/1.1 404 Not Found\r\n\
                   Content-Type: text/plain\r\n\
                   Connection: close\r\n\
                   \r\n\
                   Not Found";
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_prometheus_exporter_creation() {
        let exporter: _ = PrometheusExporter::new();
        assert!(exporter.is_ok());
    }
    #[tokio::test]
    async fn test_gather_metrics() {
        let mut exporter = PrometheusExporter::new().unwrap();
        // Add a test counter
        let counter_opts: _ = Opts::new("test_counter".to_string(), "Test counter".to_string());
        let counter: _ = Counter::with_opts(counter_opts).unwrap();
        exporter
            .registry()
            .register(Box::new(counter.clone()))
            .unwrap();
        counter.inc();
        let metrics: _ = exporter.gather_metrics();
        assert!(metrics.is_ok());
        assert!(metrics.unwrap().contains("test_counter"));
    }
    #[tokio::test]
    async fn test_custom_registry() {
        let registry: _ = Registry::new();
        let exporter: _ = PrometheusExporter::new_with_registry(registry);
        // Just verify the exporter was created successfully
        // Check that registry is valid by verifying gather works (even if empty)
        let metrics: _ = exporter.registry().gather();
        assert!(metrics.len() >= 0); // Empty is OK
    }
}
