// 异步 TCP 连接模块 - v0.3.72
// 使用 tokio 实现真正的异步 TCP 网络连接
// v0.3.72: 添加数据缓冲区和读取支持

use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream as TokioTcpStream;

/// Shared Tokio runtime for net accept + socket IO (avoids http↔tcp_async cycle).
static NET_IO_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("amberjs-net-tokio")
        .enable_all()
        .build()
        .expect("Failed to create net Tokio runtime")
});

/// Process-wide Tokio runtime for `net` server accept and socket write/read.
pub fn get_net_tokio_runtime() -> &'static tokio::runtime::Runtime {
    &NET_IO_RUNTIME
}

/// TCP 连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum TcpConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// 异步 TCP 连接信息
#[derive(Debug, Clone)]
pub struct TcpConnectionInfo {
    pub remote_addr: String,
    pub remote_port: u16,
    pub local_addr: String,
    pub local_port: u16,
    pub family: String,
    pub state: TcpConnectionState,
}

/// TCP 连接句柄 - 存储活跃连接
#[derive(Debug, Clone)]
pub struct TcpConnectionHandle {
    pub id: u64,
    pub stream: Arc<Mutex<Option<TokioTcpStream>>>,
    pub info: Arc<Mutex<TcpConnectionInfo>>,
    pub buffer: Arc<Mutex<Vec<u8>>>,
    /// 读取运行状态
    pub reading: Arc<AtomicBool>,
    /// Write side half-closed via `shutdown_write` / `socket.end`.
    pub write_ended: Arc<AtomicBool>,
    /// Socket `pause()` — suppress harvest→`'data'` and stop background `try_read`.
    /// Not Node stream HWM / cork backpressure (see NET_CONNECT_CONTRACT Limits).
    pub paused: Arc<AtomicBool>,
}

impl TcpConnectionHandle {
    /// 创建新的 TCP 连接句柄
    pub fn new(id: u64) -> Self {
        Self {
            id,
            stream: Arc::new(Mutex::new(None)),
            info: Arc::new(Mutex::new(TcpConnectionInfo {
                remote_addr: String::new(),
                remote_port: 0,
                local_addr: String::new(),
                local_port: 0,
                family: String::new(),
                state: TcpConnectionState::Disconnected,
            })),
            buffer: Arc::new(Mutex::new(Vec::new())),
            reading: Arc::new(AtomicBool::new(false)),
            write_ended: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Node-shaped `socket.pause()`: stop background read and mark paused.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
        self.stop_reading();
    }

    /// Node-shaped `socket.resume()`: clear paused and restart background read when connected.
    pub fn resume_read(&self) {
        self.paused.store(false, Ordering::SeqCst);
        if self.is_connected() {
            self.start_reading();
        }
    }

    /// Adopt an already-accepted Tokio TCP stream (server accept path).
    pub fn adopt_stream(&self, stream: TokioTcpStream) -> Result<()> {
        let peer_addr = stream.peer_addr()?;
        let local_addr = stream.local_addr()?;

        {
            let mut info = self.info.lock().unwrap();
            info.remote_addr = peer_addr.ip().to_string();
            info.remote_port = peer_addr.port();
            info.local_addr = local_addr.ip().to_string();
            info.local_port = local_addr.port();
            info.family = if peer_addr.is_ipv4() {
                "IPv4".to_string()
            } else {
                "IPv6".to_string()
            };
            info.state = TcpConnectionState::Connected;
        }

        let mut stream_guard = self.stream.lock().unwrap();
        *stream_guard = Some(stream);
        Ok(())
    }

    /// 异步连接到目标地址
    pub async fn connect(&self, host: &str, port: u16) -> Result<()> {
        let addr = format!("{}:{}", host, port);

        // 更新状态为连接中
        {
            let mut info = self.info.lock().unwrap();
            info.state = TcpConnectionState::Connecting;
        }

        match TokioTcpStream::connect(&addr).await {
            Ok(stream) => {
                let peer_addr = stream.peer_addr()?;
                let local_addr = stream.local_addr()?;

                // 更新连接信息
                {
                    let mut info = self.info.lock().unwrap();
                    info.remote_addr = peer_addr.ip().to_string();
                    info.remote_port = peer_addr.port();
                    info.local_addr = local_addr.ip().to_string();
                    info.local_port = local_addr.port();
                    info.family = if peer_addr.is_ipv4() {
                        "IPv4".to_string()
                    } else {
                        "IPv6".to_string()
                    };
                    info.state = TcpConnectionState::Connected;
                }

                // 存储流
                let mut stream_guard = self.stream.lock().unwrap();
                *stream_guard = Some(stream);

                Ok(())
            }
            Err(e) => {
                {
                    let mut info = self.info.lock().unwrap();
                    info.state = TcpConnectionState::Error(e.to_string());
                }
                Err(anyhow::anyhow!("Connection failed: {}", e))
            }
        }
    }

    /// 异步写入数据
    pub async fn write(&self, data: &[u8]) -> Result<usize> {
        let mut stream_guard = self.stream.lock().unwrap();
        if let Some(ref mut stream) = *stream_guard {
            Ok(stream.write(data).await?)
        } else {
            Err(anyhow::anyhow!("Not connected"))
        }
    }

    /// 异步读取数据
    pub async fn read(&self, buf: &mut [u8]) -> Result<usize> {
        let mut stream_guard = self.stream.lock().unwrap();
        if let Some(ref mut stream) = *stream_guard {
            Ok(stream.read(buf).await?)
        } else {
            Err(anyhow::anyhow!("Not connected"))
        }
    }

    /// 检查连接是否活跃
    pub fn is_connected(&self) -> bool {
        let info = self.info.lock().unwrap();
        info.state == TcpConnectionState::Connected
    }

    /// 获取连接状态
    pub fn get_state(&self) -> TcpConnectionState {
        let info = self.info.lock().unwrap();
        info.state.clone()
    }

    /// 获取连接信息
    pub fn get_info(&self) -> TcpConnectionInfo {
        let info = self.info.lock().unwrap();
        info.clone()
    }

    /// 关闭连接
    pub fn close(&self) {
        let mut stream_guard = self.stream.lock().unwrap();
        *stream_guard = None;

        let mut info = self.info.lock().unwrap();
        info.state = TcpConnectionState::Disconnected;
    }

    /// Start background read into `buffer` using the shared net Tokio runtime.
    /// Uses `try_read` so the stream mutex is not held across awaits (writes can proceed).
    pub fn start_reading(&self) {
        if self.reading.swap(true, Ordering::SeqCst) {
            return; // already reading
        }

        let handle = self.clone();
        get_net_tokio_runtime().spawn(async move {
            let mut scratch = [0u8; 4096];
            while handle.reading.load(Ordering::SeqCst) {
                let outcome = {
                    let mut stream_guard = handle.stream.lock().unwrap();
                    match stream_guard.as_mut() {
                        Some(stream) => match stream.try_read(&mut scratch) {
                            Ok(n) => Some(Ok(n)),
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
                            Err(e) => Some(Err(e)),
                        },
                        None => Some(Err(std::io::Error::new(
                            std::io::ErrorKind::NotConnected,
                            "not connected",
                        ))),
                    }
                };

                match outcome {
                    Some(Ok(0)) => {
                        handle.reading.store(false, Ordering::SeqCst);
                        break;
                    }
                    Some(Ok(n)) => {
                        let mut buf_guard = handle.buffer.lock().unwrap();
                        buf_guard.extend_from_slice(&scratch[..n]);
                    }
                    Some(Err(_)) => {
                        handle.reading.store(false, Ordering::SeqCst);
                        break;
                    }
                    None => {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                }
            }
        });
    }

    /// Half-close the write side (Node `socket.end` after optional final chunk).
    pub fn shutdown_write(&self) -> Result<()> {
        self.write_ended.store(true, Ordering::SeqCst);
        get_net_tokio_runtime().block_on(async {
            let mut stream_guard = self.stream.lock().unwrap();
            if let Some(ref mut stream) = *stream_guard {
                stream.shutdown().await?;
            }
            Ok(())
        })
    }

    /// 停止读取
    pub fn stop_reading(&self) {
        self.reading.store(false, Ordering::SeqCst);
    }

    /// 获取缓存的数据并清空缓冲区
    pub fn consume_buffer(&self) -> Vec<u8> {
        let mut buf_guard = self.buffer.lock().unwrap();
        std::mem::take(&mut *buf_guard)
    }

    /// 检查是否有缓存数据
    pub fn has_buffered_data(&self) -> bool {
        let buf_guard = self.buffer.lock().unwrap();
        !buf_guard.is_empty()
    }
}

/// TCP 连接管理器 - 管理所有活跃连接
#[derive(Debug, Default)]
pub struct TcpConnectionManager {
    next_id: Arc<Mutex<u64>>,
    connections: Arc<Mutex<Vec<TcpConnectionHandle>>>,
}

impl TcpConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            next_id: Arc::new(Mutex::new(1)),
            connections: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 创建新连接句柄
    pub fn create_connection(&self) -> TcpConnectionHandle {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let handle = TcpConnectionHandle::new(id);

        let mut connections = self.connections.lock().unwrap();
        connections.push(handle.clone());

        handle
    }

    /// 根据 ID 获取连接
    pub fn get_connection(&self, id: u64) -> Option<TcpConnectionHandle> {
        let connections = self.connections.lock().unwrap();
        connections.iter().find(|c| c.id == id).cloned()
    }

    /// 移除连接
    pub fn remove_connection(&self, id: u64) {
        let mut connections = self.connections.lock().unwrap();
        connections.retain(|c| c.id != id);
    }

    /// 获取所有连接
    pub fn get_all_connections(&self) -> Vec<TcpConnectionHandle> {
        let connections = self.connections.lock().unwrap();
        connections.clone()
    }
}

/// 全局 TCP 连接管理器
pub static TCP_MANAGER: std::sync::LazyLock<TcpConnectionManager> =
    std::sync::LazyLock::new(TcpConnectionManager::new);

/// 异步连接任务结果
#[derive(Debug)]
pub struct AsyncConnectResult {
    pub handle_id: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// 发起异步连接（返回结果接收者）
pub fn spawn_async_connect(
    host: String,
    port: u16,
) -> (u64, tokio::sync::oneshot::Receiver<AsyncConnectResult>) {
    let handle = TCP_MANAGER.create_connection();
    let handle_id = handle.id;

    let (tx, rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let result = handle.connect(&host, port).await;
        let async_result = AsyncConnectResult {
            handle_id,
            success: result.is_ok(),
            error: result.err().map(|e| e.to_string()),
        };
        let _ = tx.send(async_result);
    });

    (handle_id, rx)
}

/// 同步连接（阻塞，等待连接完成）
pub fn sync_connect(host: &str, port: u16, timeout_secs: u64) -> Result<TcpConnectionHandle> {
    let handle = TCP_MANAGER.create_connection();
    let _handle_id = handle.id;

    // 使用 tokio runtime 执行异步连接
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let connect_future = handle.connect(host, port);
        if timeout_secs > 0 {
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), connect_future)
                .await??
        } else {
            connect_future.await?
        }
        Ok(handle)
    })
}

/// 同步写入数据
pub fn sync_write(handle: &TcpConnectionHandle, data: &[u8]) -> Result<usize> {
    get_net_tokio_runtime().block_on(async { handle.write(data).await })
}

/// 同步读取数据
pub fn sync_read(handle: &TcpConnectionHandle, buf: &mut [u8]) -> Result<usize> {
    get_net_tokio_runtime().block_on(async { handle.read(buf).await })
}

/// 关闭连接
pub fn close_connection(handle: &TcpConnectionHandle) {
    handle.close();
    TCP_MANAGER.remove_connection(handle.id);
}

/// HTTP 响应结构
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_message: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// HTTP 请求选项
pub struct HttpRequestOptions {
    pub method: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// 同步发送 HTTP 请求并接收响应 - v0.3.73 / G72 改写：
/// 用 std::net::TcpStream（阻塞+超时）。G72 契约是同步缓冲模型，
/// 不走 tokio/TcpConnectionHandle——后者在 isolate 内 block_on 会撞
/// runtime 生命周期（"context found, but it is being shutdown"）。
pub fn sync_http_request(options: HttpRequestOptions, timeout_secs: u64) -> Result<HttpResponse> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};

    // 建立 TCP 连接（阻塞 + 超时）
    let addr = format!("{}:{}", options.host, options.port);
    let sock_addr = addr
        .as_str()
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("resolve {} failed", addr))?;
    let mut stream = TcpStream::connect_timeout(
        &sock_addr,
        std::time::Duration::from_secs(timeout_secs.max(1)),
    )?;
    let rw_timeout = std::time::Duration::from_secs(timeout_secs.max(1));
    let _ = stream.set_read_timeout(Some(rw_timeout));
    let _ = stream.set_write_timeout(Some(rw_timeout));

    // 构建 HTTP 请求
    let mut request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        options.method, options.path, options.host
    );
    for (key, value) in &options.headers {
        request.push_str(&format!("{}: {}\r\n", key, value));
    }
    if !options.body.is_empty() {
        request.push_str(&format!("Content-Length: {}\r\n", options.body.len()));
    }
    request.push_str("\r\n");

    stream.write_all(request.as_bytes())?;
    if !options.body.is_empty() {
        stream.write_all(&options.body)?;
    }
    let _ = stream.shutdown(std::net::Shutdown::Write);

    // 读取响应：Connection: close 语义——读到对端关闭或读超时
    let mut response_buffer = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response_buffer.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }

    parse_http_response(&response_buffer)
}

/// 解析 HTTP 响应
fn parse_http_response(data: &[u8]) -> Result<HttpResponse> {
    // G72：按字节找 \r\n\r\n 边界——body 允许非 UTF-8；
    // 头部按 ASCII 解析（header 名转小写，对齐 Node IncomingMessage.headers）
    let sep = data
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| anyhow::anyhow!("malformed HTTP response: no header terminator"))?;
    let head = std::str::from_utf8(&data[..sep])
        .map_err(|_| anyhow::anyhow!("malformed HTTP response: non-UTF-8 headers"))?;
    let body = data[sep + 4..].to_vec();

    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let status_parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    let status_code = status_parts
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let status_message = status_parts.get(2).unwrap_or(&"").to_string();

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_ascii_lowercase();
            let value = line[pos + 1..].trim().to_string();
            headers.push((key, value));
        }
    }

    Ok(HttpResponse {
        status_code,
        status_message,
        headers,
        body,
    })
}
