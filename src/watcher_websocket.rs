// WebSocket Hot Reload Server for Amber
//
// This module provides WebSocket-based hot reload capabilities.
// It allows browsers and other clients to receive file change notifications
// in real-time, enabling hot module replacement (HMR) and live reload.

use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Notify};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};

/// WebSocket hot reload server configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// WebSocket server port
    pub port: u16,
    /// Host to bind to
    pub host: String,
    /// Broadcast channel capacity
    pub channel_capacity: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            port: 9999,
            host: "127.0.0.1".to_string(),
            channel_capacity: 100,
        }
    }
}

/// WebSocket hot reload event payload
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HotReloadEvent {
    /// Event type: "reload", "error", "status"
    pub event_type: String,
    /// Path to the file that changed
    pub file_path: Option<String>,
    /// Change type: "created", "modified", "removed", "renamed"
    pub change_type: Option<String>,
    /// Timestamp of the event
    pub timestamp: u64,
    /// Additional message
    pub message: Option<String>,
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn check_network_bind_permission(addr: &str) -> Result<(), String> {
    crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Network,
        crate::permissions::PermissionAction::Listen,
        crate::permissions::ResourceId::Url(format!("ws://{}", addr)),
    )
    .map_err(|e| e.to_string())
}

/// WebSocket hot reload server.
///
/// `start` binds on the calling task and returns the real address (port `0`
/// included). A fresh shutdown notify is installed per `start`, so `stop`
/// cannot leak a permit into the next listen.
#[derive(Debug, Clone)]
pub struct WebSocketHotReloader {
    config: WebSocketConfig,
    /// Broadcast sender for sending events to all connected clients
    tx: broadcast::Sender<HotReloadEvent>,
    /// Server running flag
    running: Arc<AtomicBool>,
    shutdown: Arc<Mutex<Arc<Notify>>>,
    bound_addr: Arc<Mutex<Option<SocketAddr>>>,
}

impl WebSocketHotReloader {
    /// Create a new WebSocket hot reloader with default config
    pub fn new() -> Self {
        Self::with_config(WebSocketConfig::default())
    }

    /// Create a new WebSocket hot reloader with custom config
    pub fn with_config(config: WebSocketConfig) -> Self {
        let channel_capacity = config.channel_capacity.max(1);
        Self {
            config,
            tx: broadcast::channel(channel_capacity).0,
            running: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(Mutex::new(Arc::new(Notify::new()))),
            bound_addr: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the broadcast receiver for sending events
    pub fn subscribe(&self) -> broadcast::Receiver<HotReloadEvent> {
        self.tx.subscribe()
    }

    /// Send `event` to every subscribed client.
    ///
    /// Zero subscribers is success. `broadcast::Sender::send` returns `Err`
    /// in that case, and a reload must still be recorded when no browser is
    /// connected.
    pub fn broadcast(&self, event: HotReloadEvent) -> Result<(), String> {
        let _ = self.tx.send(event);
        Ok(())
    }

    /// 创建 reload 事件并广播
    pub fn broadcast_reload(&self, file_path: String, change_type: String) {
        let event = HotReloadEvent {
            event_type: "reload".to_string(),
            file_path: Some(file_path),
            change_type: Some(change_type),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            message: None,
        };
        let _ = self.broadcast(event);
    }

    /// 创建错误事件并广播
    pub fn broadcast_error(&self, message: String) {
        let event = HotReloadEvent {
            event_type: "error".to_string(),
            file_path: None,
            change_type: None,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            message: Some(message),
        };
        let _ = self.broadcast(event);
    }

    /// Bind the listen socket, then accept connections until [`stop`](Self::stop).
    ///
    /// The returned address is the socket actually bound. Permission checks and
    /// bind failures return before `is_running` stays true and before any
    /// client can connect.
    pub async fn start(&self) -> Result<SocketAddr, String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("WebSocket hot reload server is already running".to_string());
        }

        let addr = format!("{}:{}", self.config.host, self.config.port);
        if let Err(error) = check_network_bind_permission(&addr) {
            self.running.store(false, Ordering::SeqCst);
            return Err(error);
        }

        let listener = match TcpListener::bind(&addr).await {
            Ok(listener) => listener,
            Err(error) => {
                self.running.store(false, Ordering::SeqCst);
                return Err(format!("Failed to bind to {addr}: {error}"));
            }
        };
        let local = match listener.local_addr() {
            Ok(local) => local,
            Err(error) => {
                self.running.store(false, Ordering::SeqCst);
                return Err(format!("Failed to read bound address: {error}"));
            }
        };
        if let Ok(mut slot) = self.bound_addr.lock() {
            *slot = Some(local);
        }

        let shutdown = Arc::new(Notify::new());
        if let Ok(mut slot) = self.shutdown.lock() {
            *slot = shutdown.clone();
        }
        let this = self.clone();
        tokio::spawn(async move {
            this.accept_loop(listener, shutdown).await;
            this.running.store(false, Ordering::SeqCst);
        });
        Ok(local)
    }

    async fn accept_loop(&self, listener: TcpListener, shutdown: Arc<Notify>) {
        loop {
            tokio::select! {
                _ = shutdown.notified() => break,
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, peer)) => match accept_async(stream).await {
                            Ok(ws_stream) => {
                                let rx = self.subscribe();
                                let running = self.running.clone();
                                tokio::spawn(async move {
                                    handle_client(ws_stream, rx, running).await;
                                });
                                println!("\x1b[36m[amberjs]\x1b[0m 📡 Client connected: {peer}");
                            }
                            Err(error) => {
                                eprintln!("[amberjs] WebSocket handshake failed: {error}");
                            }
                        },
                        Err(error) => {
                            if self.running.load(Ordering::SeqCst) {
                                eprintln!("[amberjs] Failed to accept connection: {error}");
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Stop accepting connections. The bound port is released once the accept
    /// loop observes the shutdown notify.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Ok(shutdown) = self.shutdown.lock() {
            shutdown.notify_one();
        }
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Configured `host:port` before [`start`](Self::start), then the bound address.
    pub fn server_addr(&self) -> String {
        if let Ok(slot) = self.bound_addr.lock() {
            if let Some(addr) = *slot {
                return addr.to_string();
            }
        }
        format!("{}:{}", self.config.host, self.config.port)
    }
}

impl Default for WebSocketHotReloader {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle a single WebSocket client connection
///
/// This function:
/// 1. Receives the WebSocket stream
/// 2. Subscribes to the broadcast channel
/// 3. Converts received broadcast messages to JSON and sends to client
/// 4. Handles client disconnection
async fn handle_client(
    ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
    mut rx: broadcast::Receiver<HotReloadEvent>,
    running: Arc<AtomicBool>,
) {
    // Split the WebSocket stream into sender and receiver
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let hello = HotReloadEvent {
        event_type: "status".to_string(),
        file_path: None,
        change_type: None,
        timestamp: unix_millis(),
        message: Some("connected".to_string()),
    };
    if let Ok(json) = serde_json::to_string(&hello) {
        if ws_sender.send(Message::Text(json)).await.is_err() {
            return;
        }
    }

    // Clone running flag for use in select
    let running_clone = running.clone();

    // Handle both broadcast events and client messages using select
    loop {
        tokio::select! {
            // Handle broadcast events
            biased;
            event_result = rx.recv() => {
                match event_result {
                    Ok(event) => {
                        // Serialize event to JSON and send to client
                        if let Ok(json) = serde_json::to_string(&event) {
                            if ws_sender.send(Message::Text(json)).await.is_err() {
                                // Client disconnected
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                }
            }
            // Handle client messages
            msg_result = ws_receiver.next() => {
                match msg_result {
                    Some(Ok(Message::Text(text))) => {
                        // Handle ping/pong for keepalive
                        if text == "ping" {
                            let _ = ws_sender.send(Message::Text("pong".to_string())).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        // Client closed connection
                        break;
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types
                    }
                    Some(Err(e)) => {
                        eprintln!("[amberjs] WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        // Stream ended
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                // Periodic check if running flag changed
                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{
        global_resource_broker, PermissionAction, PermissionKind, ResourceBroker, ResourceId,
    };
    use serial_test::serial;
    use std::time::Duration;

    fn reset_global_broker() {
        *global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned") = ResourceBroker::default();
    }

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert_eq!(config.port, 9999);
        assert_eq!(config.host, "127.0.0.1");
    }

    #[test]
    fn test_hot_reload_event_serialization() {
        let event = HotReloadEvent {
            event_type: "reload".to_string(),
            file_path: Some("test.js".to_string()),
            change_type: Some("modified".to_string()),
            timestamp: 1234567890,
            message: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event_type\":\"reload\""));
        assert!(json.contains("\"file_path\":\"test.js\""));
    }

    #[tokio::test]
    #[serial]
    async fn start_uses_global_network_broker_before_binding_listener() {
        let config = WebSocketConfig {
            port: 0,
            host: "127.0.0.1".to_string(),
            channel_capacity: 4,
        };
        let reloader = WebSocketHotReloader::with_config(config);

        {
            let mut broker = global_resource_broker()
                .write()
                .expect("resource broker lock should not be poisoned");
            *broker = ResourceBroker::default();
            broker.deny(
                PermissionKind::Network,
                PermissionAction::Listen,
                ResourceId::Url("ws://127.0.0.1:0".to_string()),
            );
        }

        let result = tokio::time::timeout(Duration::from_millis(100), reloader.start()).await;
        reset_global_broker();

        let error = match result {
            Ok(Err(error)) => error,
            Ok(Ok(addr)) => {
                panic!("denied WebSocket hot reload bind must not succeed, bound {addr}")
            }
            Err(_) => panic!("denied WebSocket hot reload bind must fail before listener bind"),
        };
        assert!(
            error.contains("permission denied") && error.contains("Listen"),
            "expected permission denied, got: {error}"
        );
        assert!(
            !reloader.is_running(),
            "denied WebSocket hot reload bind must not mark server running"
        );
    }
}
