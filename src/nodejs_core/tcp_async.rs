// 异步 TCP 连接模块 - v0.3.72
// 使用 tokio 实现真正的异步 TCP 网络连接
// v0.3.72: 添加数据缓冲区和读取支持

use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
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

    /// 开始异步读取数据（后台任务）- 数据存入缓冲区
    pub fn start_reading(&self) {
        if self.reading.swap(true, Ordering::SeqCst) {
            return; // 已经在读取
        }

        let handle = self.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            loop {
                if !handle.reading.load(Ordering::SeqCst) {
                    break;
                }

                let read_result = {
                    let mut stream_guard = handle.stream.lock().unwrap();
                    if let Some(ref mut stream) = *stream_guard {
                        // 使用闭包捕获结果
                        let result = tokio::runtime::Runtime::new().ok().and_then(|rt| {
                            let rt = rt;
                            let fut = async { stream.read(&mut buffer).await };
                            Some(rt.block_on(fut))
                        });
                        result
                    } else {
                        None
                    }
                };

                match read_result {
                    Some(Ok(0)) => {
                        // 连接关闭
                        handle.reading.store(false, Ordering::SeqCst);
                        break;
                    }
                    Some(Ok(n)) => {
                        // 收到数据，存储到缓冲区
                        let data = buffer[..n].to_vec();
                        let mut buf_guard = handle.buffer.lock().unwrap();
                        buf_guard.extend_from_slice(&data);
                    }
                    Some(Err(_)) => {
                        handle.reading.store(false, Ordering::SeqCst);
                        break;
                    }
                    None => {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        });
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

/// 同步发送 HTTP 请求并接收响应 - v0.3.73
pub fn sync_http_request(options: HttpRequestOptions, timeout_secs: u64) -> Result<HttpResponse> {
    // 建立 TCP 连接
    let handle = sync_connect(&options.host, options.port, timeout_secs)?;

    // 构建 HTTP 请求
    let mut request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\n",
        options.method, options.path, options.host
    );

    // 添加自定义请求头
    for (key, value) in &options.headers {
        request.push_str(&format!("{}: {}\r\n", key, value));
    }

    // 添加 Content-Length（如果有 body）
    if !options.body.is_empty() {
        request.push_str(&format!("Content-Length: {}\r\n", options.body.len()));
    }

    // 结束请求头
    request.push_str("\r\n");

    // 发送请求头
    sync_write(&handle, request.as_bytes())?;

    // 发送 body（如果有）
    if !options.body.is_empty() {
        sync_write(&handle, &options.body)?;
    }

    // 读取响应
    let mut response_buffer = Vec::new();
    let mut buf = [0u8; 4096];

    loop {
        match sync_read(&handle, &mut buf) {
            Ok(0) => break, // 连接关闭
            Ok(n) => {
                response_buffer.extend_from_slice(&buf[..n]);
            }
            Err(_) => break,
        }
    }

    close_connection(&handle);

    // 解析 HTTP 响应
    parse_http_response(&response_buffer)
}

/// 解析 HTTP 响应
fn parse_http_response(data: &[u8]) -> Result<HttpResponse> {
    let response_str = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => {
            // 如果不是有效 UTF-8，尝试查找 HTTP 分隔符
            return Ok(HttpResponse {
                status_code: 200,
                status_message: String::from("OK"),
                headers: vec![(
                    String::from("content-type"),
                    String::from("application/octet-stream"),
                )],
                body: data.to_vec(),
            });
        }
    };

    // 分割 headers 和 body
    let parts: Vec<&str> = response_str.split("\r\n\r\n").collect();
    let header_section = parts.get(0).unwrap_or(&response_str);
    let body = parts.get(1).unwrap_or(&"");

    // 解析状态行
    let lines: Vec<&str> = header_section.split("\r\n").collect();
    let status_line = lines.get(0).unwrap_or(&"");

    // 解析状态码
    let status_parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    let status_code = status_parts
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let status_message = status_parts.get(2).unwrap_or(&"").to_string();

    // 解析 headers
    let mut headers = Vec::new();
    for line in lines.iter().skip(1) {
        if line.is_empty() {
            continue;
        }
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();
            headers.push((key, value));
        }
    }

    Ok(HttpResponse {
        status_code,
        status_message,
        headers,
        body: body.as_bytes().to_vec(),
    })
}
