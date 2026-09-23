// Node.js http模块实现 - v0.3.87 增强版
/// HTTP API - 支持 Agent, getAllHeaders, DNS 解析等
/// v0.3.87: 添加 HTTP Server 真实监听和请求处理功能
/// v0.3.84: 添加 HTTP Agent 连接池优化
/// v0.3.73: 添加真实 HTTP 网络请求支持
use anyhow::Result;
use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::task::{Context, Poll};
use std::thread;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::tcp_async::{sync_http_request, HttpRequestOptions};
use crate::permissions::{check_global_permission, PermissionAction, PermissionKind, ResourceId};

use std::sync::atomic::AtomicU64;

// v0.3.97: 添加 SO_REUSEADDR 支持以解决端口重用问题
#[cfg(unix)]
use libc;

/// HTTP 请求消息（跨线程传递）
/// v0.3.89: 添加跨线程消息传递支持
#[derive(Debug)]
pub struct HttpRequestMessage {
    /// HTTP 方法
    pub method: String,
    /// 请求 URL
    pub url: String,
    /// 请求路径
    pub path: String,
    /// HTTP 版本
    pub http_version: String,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 请求体
    pub body: Vec<u8>,
    /// 连接 ID（用于响应时定位连接）
    pub connection_id: u64,
    /// Tokio 响应直通通道 (零锁快路径)
    pub responder: Option<tokio::sync::oneshot::Sender<HttpResponseMessage>>,
}

impl Clone for HttpRequestMessage {
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            url: self.url.clone(),
            path: self.path.clone(),
            http_version: self.http_version.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            connection_id: self.connection_id,
            responder: None,
        }
    }
}

/// HTTP 响应消息（跨线程传递）
/// v0.3.89: 添加跨线程消息传递支持
#[derive(Debug)]
pub struct HttpResponseMessage {
    /// 连接 ID
    pub connection_id: u64,
    /// 状态码
    pub status_code: u16,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体
    pub body: Vec<u8>,
}

/// HTTP 服务器消息通道
/// 用于主线程和后台线程之间的请求/响应传递
/// v0.3.89: 添加跨线程消息传递支持
pub struct HttpServerMessageChannel {
    /// 发送请求到主线程
    pub request_sender: crossbeam::channel::Sender<HttpRequestMessage>,
    /// v0.3.90: 接收来自后台线程的请求
    pub request_receiver: crossbeam::channel::Receiver<HttpRequestMessage>,
    /// 接收主线程的响应
    pub response_receiver: crossbeam::channel::Receiver<HttpResponseMessage>,
    /// v0.3.90: 发送响应到后台线程
    pub response_sender: crossbeam::channel::Sender<HttpResponseMessage>,
    /// 是否启用了消息模式
    pub enabled: bool,
    /// 下一个连接 ID
    pub next_connection_id: Arc<AtomicU64>,
}

impl std::fmt::Debug for HttpServerMessageChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpServerMessageChannel")
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl HttpServerMessageChannel {
    /// 创建新的消息通道
    #[allow(clippy::redundant_closure)]
    pub fn new(capacity: usize) -> Self {
        let (request_sender, request_receiver) = crossbeam::channel::bounded(capacity);
        let (response_sender, response_receiver) = crossbeam::channel::bounded(capacity);

        Self {
            request_sender,
            request_receiver,
            response_receiver,
            response_sender,
            enabled: true,
            next_connection_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// 生成新的连接 ID
    pub fn next_connection_id(&self) -> u64 {
        self.next_connection_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// 发送请求消息
    pub fn send_request(
        &self,
        request: HttpRequestMessage,
    ) -> Result<(), crossbeam::channel::SendError<HttpRequestMessage>> {
        self.request_sender.send(request)
    }

    /// 接收响应消息
    pub fn recv_response(&self) -> Result<HttpResponseMessage, crossbeam::channel::RecvError> {
        self.response_receiver.recv()
    }

    /// 尝试接收响应（非阻塞）
    pub fn try_recv_response(
        &self,
    ) -> Result<HttpResponseMessage, crossbeam::channel::TryRecvError> {
        self.response_receiver.try_recv()
    }

    /// v0.3.90: 发送响应消息到后台线程
    pub fn send_response(
        &self,
        response: HttpResponseMessage,
    ) -> Result<(), crossbeam::channel::SendError<HttpResponseMessage>> {
        self.response_sender.send(response)
    }
}

/// 全局 HTTP 服务器消息通道
/// v0.3.89: 添加跨线程消息传递支持
static mut HTTP_SERVER_CHANNEL: Option<Arc<Mutex<Option<HttpServerMessageChannel>>>> = None;

static ACTIVE_HTTP_SERVER_STATES: Lazy<Mutex<Vec<Arc<HttpServerState>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

fn register_http_server_state(state: Arc<HttpServerState>) {
    ACTIVE_HTTP_SERVER_STATES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(state);
}

fn stop_all_http_server_states() {
    let mut states = ACTIVE_HTTP_SERVER_STATES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for state in states.iter() {
        state.listening.store(false, Ordering::SeqCst);
    }
    states.clear();
}

fn stop_http_server_state(host: &str, port: u16) {
    let mut states = ACTIVE_HTTP_SERVER_STATES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for state in states.iter() {
        let bound = state.bound_port.load(Ordering::SeqCst) as u16;
        if state.host == host && (state.port == port || bound == port) {
            state.listening.store(false, Ordering::SeqCst);
        }
    }
    states.retain(|state| state.listening.load(Ordering::SeqCst));
}

static GLOBAL_REQUEST_SENDER: Lazy<
    std::sync::RwLock<Option<crossbeam::channel::Sender<HttpRequestMessage>>>,
> = Lazy::new(|| std::sync::RwLock::new(None));
static GLOBAL_REQUEST_RECEIVER: Lazy<
    std::sync::RwLock<Option<crossbeam::channel::Receiver<HttpRequestMessage>>>,
> = Lazy::new(|| std::sync::RwLock::new(None));

static MAIN_V8_THREAD: Lazy<std::sync::RwLock<Option<std::thread::Thread>>> =
    Lazy::new(|| std::sync::RwLock::new(None));

/// 登记主 V8 线程句柄，用于微秒级即时唤醒
pub fn register_http_dispatch_thread(thread: std::thread::Thread) {
    if let Ok(mut guard) = MAIN_V8_THREAD.write() {
        *guard = Some(thread);
    }
}

/// 立即唤醒挂起等待的 V8 主事件循环 (零延迟)
pub fn wake_http_dispatch_thread() {
    if let Ok(guard) = MAIN_V8_THREAD.read() {
        if let Some(ref thread) = *guard {
            thread.unpark();
        }
    }
}

/// HTTP 响应发送器（统一抽象 crossbeam 与 Tokio oneshot）
pub enum HttpResponseSender {
    Crossbeam(crossbeam::channel::Sender<HttpResponseMessage>),
    TokioOneshot(tokio::sync::oneshot::Sender<HttpResponseMessage>),
}

impl HttpResponseSender {
    pub fn send_response(self, msg: HttpResponseMessage) {
        match self {
            HttpResponseSender::Crossbeam(tx) => {
                let _ = tx.send(msg);
            }
            HttpResponseSender::TokioOneshot(tx) => {
                let _ = tx.send(msg);
            }
        }
    }
}

const NUM_RESPONSE_SHARDS: usize = 32;

struct ResponseWaiterShard {
    map: Mutex<HashMap<u64, HttpResponseSender>>,
}

static RESPONSE_WAITERS_SHARDS: Lazy<Vec<ResponseWaiterShard>> = Lazy::new(|| {
    (0..NUM_RESPONSE_SHARDS)
        .map(|_| ResponseWaiterShard {
            map: Mutex::new(HashMap::new()),
        })
        .collect()
});

#[inline]
fn get_response_shard(connection_id: u64) -> &'static ResponseWaiterShard {
    let idx = (connection_id as usize) % NUM_RESPONSE_SHARDS;
    &RESPONSE_WAITERS_SHARDS[idx]
}

/// 初始化全局消息通道
#[allow(static_mut_refs)]
pub fn init_http_server_channel() -> Arc<Mutex<Option<HttpServerMessageChannel>>> {
    unsafe {
        if HTTP_SERVER_CHANNEL.is_none() {
            let channel = HttpServerMessageChannel::new(32768);
            if let Ok(mut tx_guard) = GLOBAL_REQUEST_SENDER.write() {
                *tx_guard = Some(channel.request_sender.clone());
            }
            if let Ok(mut rx_guard) = GLOBAL_REQUEST_RECEIVER.write() {
                *rx_guard = Some(channel.request_receiver.clone());
            }
            HTTP_SERVER_CHANNEL = Some(Arc::new(Mutex::new(Some(channel))));
        }
        HTTP_SERVER_CHANNEL.as_ref().unwrap().clone()
    }
}

/// 获取全局消息通道
#[allow(static_mut_refs)]
pub fn get_http_server_channel() -> Option<Arc<Mutex<Option<HttpServerMessageChannel>>>> {
    unsafe { HTTP_SERVER_CHANNEL.as_ref().cloned() }
}

/// 重置全局消息通道
/// v0.3.93: 添加测试支持，用于清空通道中的残留消息
#[allow(static_mut_refs)]
pub fn reset_http_server_channel() {
    stop_all_http_server_states();
    for shard in RESPONSE_WAITERS_SHARDS.iter() {
        if let Ok(mut map) = shard.map.lock() {
            map.clear();
        }
    }
    PENDING_ASYNC_HTTP_RESPONSES.store(0, Ordering::SeqCst);

    unsafe {
        if let Some(ref channel_arc) = HTTP_SERVER_CHANNEL {
            let mut channel_guard = channel_arc
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let channel = HttpServerMessageChannel::new(32768);
            if let Ok(mut tx_guard) = GLOBAL_REQUEST_SENDER.write() {
                *tx_guard = Some(channel.request_sender.clone());
            }
            if let Ok(mut rx_guard) = GLOBAL_REQUEST_RECEIVER.write() {
                *rx_guard = Some(channel.request_receiver.clone());
            }
            *channel_guard = Some(channel);
        }
    }
}

/// 发送 HTTP 响应到等待中的工作任务（支持分片锁与 0ms 通知）
#[allow(static_mut_refs)]
pub fn send_http_response(response: HttpResponseMessage) {
    let shard = get_response_shard(response.connection_id);
    let waiter = if let Ok(mut map) = shard.map.lock() {
        map.remove(&response.connection_id)
    } else {
        None
    };

    if let Some(sender) = waiter {
        sender.send_response(response);
    } else {
        unsafe {
            if let Some(ref channel_arc) = HTTP_SERVER_CHANNEL {
                if let Some(ref channel) = *channel_arc
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                {
                    let _ = channel.send_response(response);
                }
            }
        }
    }
}

/// 获取消息接收器（用于事件循环轮询）
#[allow(static_mut_refs)]
#[deprecated(since = "0.3.90", note = "Use try_recv_http_request instead")]
pub fn get_http_request_receiver() -> Option<crossbeam::channel::Receiver<HttpRequestMessage>> {
    if let Ok(guard) = GLOBAL_REQUEST_RECEIVER.read() {
        if let Some(ref rx) = *guard {
            return Some(rx.clone());
        }
    }
    unsafe {
        HTTP_SERVER_CHANNEL.as_ref().and_then(|channel_arc| {
            let _ = channel_arc
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_ref()?;
            None
        })
    }
}

/// 尝试接收 HTTP 请求（非阻塞快路径）
#[allow(static_mut_refs)]
pub fn try_recv_http_request() -> Option<HttpRequestMessage> {
    if let Ok(guard) = GLOBAL_REQUEST_RECEIVER.read() {
        if let Some(ref rx) = *guard {
            return rx.try_recv().ok();
        }
    }
    unsafe {
        if let Some(ref channel_arc) = HTTP_SERVER_CHANNEL {
            if let Some(ref channel) = *channel_arc
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
            {
                match channel.request_receiver.try_recv() {
                    Ok(request) => Some(request),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub fn register_http_response_waiter(
    connection_id: u64,
    sender: crossbeam::channel::Sender<HttpResponseMessage>,
) {
    let shard = get_response_shard(connection_id);
    if let Ok(mut map) = shard.map.lock() {
        map.insert(connection_id, HttpResponseSender::Crossbeam(sender));
    }
}

pub fn register_tokio_response_waiter(
    connection_id: u64,
    sender: tokio::sync::oneshot::Sender<HttpResponseMessage>,
) {
    let shard = get_response_shard(connection_id);
    if let Ok(mut map) = shard.map.lock() {
        map.insert(connection_id, HttpResponseSender::TokioOneshot(sender));
    }
}

pub fn unregister_http_response_waiter(connection_id: u64) {
    let shard = get_response_shard(connection_id);
    if let Ok(mut map) = shard.map.lock() {
        map.remove(&connection_id);
    }
}

pub fn unregister_tokio_response_waiter(connection_id: u64) {
    unregister_http_response_waiter(connection_id);
}

static PENDING_ASYNC_HTTP_RESPONSES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum HttpDispatchResult {
    Response(HttpResponseMessage),
    Pending,
    NoHandler,
    Error,
}

/// True when at least one `http.Server` is still marked listening.
pub fn has_listening_http_servers() -> bool {
    ACTIVE_HTTP_SERVER_STATES
        .lock()
        .map(|states| {
            states
                .iter()
                .any(|state| state.listening.load(Ordering::SeqCst))
        })
        .unwrap_or(false)
}

/// True when a handler returned without calling `res.end` yet.
pub fn has_pending_async_http_responses() -> bool {
    PENDING_ASYNC_HTTP_RESPONSES.load(Ordering::SeqCst) > 0
}

/// True when the request channel has at least one queued message.
pub fn has_pending_http_requests() -> bool {
    if let Ok(guard) = GLOBAL_REQUEST_RECEIVER.read() {
        if let Some(ref rx) = *guard {
            return !rx.is_empty();
        }
    }
    unsafe {
        if let Some(ref channel_arc) = HTTP_SERVER_CHANNEL {
            if let Ok(guard) = channel_arc.lock() {
                if let Some(ref channel) = *guard {
                    return !channel.request_receiver.is_empty();
                }
            }
        }
    }
    false
}

static HTTP_CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Allocate a process-wide connection/request id for response matching.
pub fn allocate_http_connection_id() -> u64 {
    HTTP_CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Drain queued HTTP requests using the caller's existing V8 scope.
/// Safe to call from `execute_code` (do not nest `pump_http_messages`).
/// Drain queued HTTP requests using the caller's existing V8 scope.
/// Safe to call from `execute_code` (do not nest `pump_http_messages`).
pub fn pump_pending_http_requests_in_scope(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> usize {
    let fast_rx = if let Ok(guard) = GLOBAL_REQUEST_RECEIVER.read() {
        guard.clone()
    } else {
        None
    };

    let Some(rx) = fast_rx else {
        return 0;
    };

    let Ok(mut first_req) = rx.try_recv() else {
        return 0;
    };

    v8::scope!(let batch_scope, scope);
    let atoms = V8HttpAtoms::new(batch_scope);
    let protos = get_http_prototypes(batch_scope, context, &atoms);
    let handler = get_global_request_handler_local(batch_scope, context, &atoms);

    let mut processed = 0;
    dispatch_http_request_in_scope_fast(
        batch_scope,
        context,
        &mut first_req,
        &atoms,
        &protos,
        handler,
    );
    processed += 1;

    while let Ok(mut request) = rx.try_recv() {
        dispatch_http_request_in_scope_fast(
            batch_scope,
            context,
            &mut request,
            &atoms,
            &protos,
            handler,
        );
        processed += 1;
        if processed >= 512 {
            break;
        }
    }
    processed
}

/// Fast in-scope HTTP dispatch with pre-interned atoms and direct responder
pub fn dispatch_http_request_in_scope_fast<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    context: &v8::Local<v8::Context>,
    request: &mut HttpRequestMessage,
    atoms: &V8HttpAtoms<'a>,
    protos: &V8HttpPrototypes<'a>,
    handler: Option<v8::Local<'a, v8::Function>>,
) {
    v8::scope!(let req_scope, scope);
    match process_http_request_in_v8_inner(request, req_scope, context, handler, atoms, protos) {
        HttpDispatchResult::Response(mut response) => {
            if !response.headers.contains_key("Content-Length") {
                response.headers.insert(
                    "Content-Length".to_string(),
                    response.body.len().to_string(),
                );
            }
            if let Some(responder) = request.responder.take() {
                let _ = responder.send(response);
            } else {
                send_http_response(response);
            }
        }
        HttpDispatchResult::Pending => {
            if let Some(responder) = request.responder.take() {
                register_tokio_response_waiter(request.connection_id, responder);
            }
        }
        HttpDispatchResult::NoHandler => {
            let resp = create_http_response(request.connection_id, 404, "No handler", "text/plain");
            if let Some(responder) = request.responder.take() {
                let _ = responder.send(resp);
            } else {
                send_http_response(resp);
            }
        }
        HttpDispatchResult::Error => {
            let resp =
                create_http_response(request.connection_id, 500, "Handler error", "text/plain");
            if let Some(responder) = request.responder.take() {
                let _ = responder.send(resp);
            } else {
                send_http_response(resp);
            }
        }
    }
}

/// Dispatch one request to the JS handler and send the matching response.
pub fn dispatch_http_request_in_scope(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
    request: &HttpRequestMessage,
) {
    let mut req_clone = request.clone();
    let atoms = V8HttpAtoms::new(scope);
    let protos = get_http_prototypes(scope, context, &atoms);
    let handler = get_global_request_handler_local(scope, context, &atoms);
    dispatch_http_request_in_scope_fast(scope, context, &mut req_clone, &atoms, &protos, handler);
}

/// v0.3.90: 创建简单的 HTTP 响应消息
pub fn create_http_response(
    connection_id: u64,
    status_code: u16,
    body: &str,
    content_type: &str,
) -> HttpResponseMessage {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), content_type.to_string());
    headers.insert("Content-Length".to_string(), body.len().to_string());
    // v0.3.97: 不设置默认 Connection 头，让服务器根据 Keep-Alive 决定

    HttpResponseMessage {
        connection_id,
        status_code,
        headers,
        body: body.as_bytes().to_vec(),
    }
}

/// 连接键：用于标识唯一的服务器端点
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ConnectionKey {
    host: String,
    port: u16,
}

/// 池化连接信息
#[derive(Debug)]
struct PooledConnection {
    /// 最后使用时间
    last_used: Instant,
    /// 连接是否仍然有效
    is_valid: bool,
}

impl PooledConnection {
    fn new() -> Self {
        Self {
            last_used: Instant::now(),
            is_valid: true,
        }
    }
}

/// HTTP 连接池管理器 - v0.3.84
#[derive(Debug)]
struct HttpConnectionPool {
    /// 空闲连接池：按主机端口分组
    free_connections: HashMap<ConnectionKey, Vec<PooledConnection>>,
    /// 当前活跃连接数
    active_connections: usize,
    /// 最大空闲连接数
    max_free_sockets: usize,
    /// 最大总连接数
    max_sockets: usize,
    /// 是否启用 keepAlive
    keep_alive: bool,
    /// 连接超时时间（秒）
    connection_timeout: u64,
}

impl HttpConnectionPool {
    fn new(max_free_sockets: usize, max_sockets: usize, keep_alive: bool) -> Self {
        Self {
            free_connections: HashMap::new(),
            active_connections: 0,
            max_free_sockets,
            max_sockets,
            keep_alive,
            connection_timeout: 30, // 30秒超时
        }
    }

    /// 获取连接键
    fn get_key(host: &str, port: u16) -> ConnectionKey {
        ConnectionKey {
            host: host.to_lowercase(), // 主机名不区分大小写
            port,
        }
    }

    /// 从池中获取一个空闲连接
    fn acquire(&mut self, host: &str, port: u16) -> bool {
        // 检查是否超出总连接限制
        if self.active_connections >= self.max_sockets {
            return false;
        }

        let key = Self::get_key(host, port);

        if let Some(connections) = self.free_connections.get_mut(&key) {
            // 清理超时的连接
            connections.retain(|conn| {
                conn.is_valid
                    && conn.last_used.elapsed() < Duration::from_secs(self.connection_timeout)
            });

            // 如果有可用的空闲连接
            if let Some(conn) = connections.first() {
                if conn.is_valid {
                    self.active_connections += 1;
                    return true;
                }
            }
        }

        // 没有可用连接，需要新建
        self.active_connections += 1;
        true
    }

    /// 释放一个连接到池中
    fn release(&mut self, host: &str, port: u16) {
        let key = Self::get_key(host, port);

        // 统计当前该 key 的空闲连接数
        let current_free = self
            .free_connections
            .get(&key)
            .map(|v| v.len())
            .unwrap_or(0);

        if self.keep_alive && current_free < self.max_free_sockets {
            // 添加到空闲池
            let conn = PooledConnection::new();
            self.free_connections.entry(key).or_default().push(conn);
        } else {
            // 不 keepAlive 或超出限制，关闭连接
            // 这里只是减少计数，实际连接由 tcp_async 处理
        }

        self.active_connections = self.active_connections.saturating_sub(1);
    }

    /// 获取当前活跃连接数
    fn active_count(&self) -> usize {
        self.active_connections
    }

    /// 清理所有超时连接
    fn cleanup(&mut self) {
        let timeout = Duration::from_secs(self.connection_timeout);

        for connections in self.free_connections.values_mut() {
            connections.retain(|conn| conn.is_valid && conn.last_used.elapsed() < timeout);
        }

        // 清理空的 key
        self.free_connections.retain(|_, v| !v.is_empty());
    }
}

/// 全局 HTTP 连接池 - 使用 Mutex 确保线程安全
static mut HTTP_CONNECTION_POOL: Option<Arc<Mutex<HttpConnectionPool>>> = None;

/// 初始化全局连接池
pub fn init_http_connection_pool(max_free_sockets: usize, max_sockets: usize, keep_alive: bool) {
    unsafe {
        HTTP_CONNECTION_POOL = Some(Arc::new(Mutex::new(HttpConnectionPool::new(
            max_free_sockets,
            max_sockets,
            keep_alive,
        ))));
    }
}

/// 从全局连接池获取连接
pub fn acquire_http_connection(host: &str, port: u16) -> bool {
    unsafe {
        if let Some(ref pool) = HTTP_CONNECTION_POOL {
            return pool.lock().unwrap().acquire(host, port);
        }
        false
    }
}

/// 释放连接到全局连接池
pub fn release_http_connection(host: &str, port: u16) {
    unsafe {
        if let Some(ref pool) = HTTP_CONNECTION_POOL {
            pool.lock().unwrap().release(host, port);
        }
    }
}

/// 获取全局连接池状态
pub fn get_connection_pool_stats() -> String {
    unsafe {
        if let Some(ref pool) = HTTP_CONNECTION_POOL {
            let pool = pool.lock().unwrap();
            format!(
                "active: {}, total_free: {}",
                pool.active_count(),
                pool.free_connections
                    .values()
                    .map(|v| v.len())
                    .sum::<usize>()
            )
        } else {
            "pool not initialized".to_string()
        }
    }
}

/// 清理全局连接池中的超时连接
pub fn cleanup_connection_pool() {
    unsafe {
        if let Some(ref pool) = HTTP_CONNECTION_POOL {
            pool.lock().unwrap().cleanup();
        }
    }
}

/// 设置http API
pub fn setup_http_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let http_obj: _ = v8::Object::new(scope);

    // createServer - 使用普通 callback
    // v0.3.93: callback 中会直接从 context 获取全局对象
    let create_server_func =
        v8::FunctionTemplate::new(scope, http_create_server_with_global_callback);
    let create_server_instance: _ = create_server_func.get_function(scope).unwrap();
    let create_server_key: _ = v8::String::new(scope, "createServer").unwrap();
    http_obj.set(
        scope,
        create_server_key.into(),
        create_server_instance.into(),
    );

    // request
    let request_func: _ = v8::FunctionTemplate::new(scope, http_request_callback);
    let request_instance: _ = request_func.get_function(scope).unwrap();
    let request_key: _ = v8::String::new(scope, "request").unwrap();
    http_obj.set(scope, request_key.into(), request_instance.into());
    // get
    let get_func: _ = v8::FunctionTemplate::new(scope, http_get_callback);
    let get_instance: _ = get_func.get_function(scope).unwrap();
    let get_key: _ = v8::String::new(scope, "get").unwrap();
    http_obj.set(scope, get_key.into(), get_instance.into());
    // Agent - v0.3.64: 添加 Agent 支持
    let agent_func: _ = v8::FunctionTemplate::new(scope, http_agent_callback);
    let agent_instance: _ = agent_func.get_function(scope).unwrap();
    let agent_key: _ = v8::String::new(scope, "Agent").unwrap();
    http_obj.set(scope, agent_key.into(), agent_instance.into());
    // 全局 Agent 实例 - v0.3.64: 修正：设置到 http 对象而非构造函数上
    let global_agent: _ = create_default_agent(scope);
    let global_agent_key: _ = v8::String::new(scope, "globalAgent").unwrap();
    http_obj.set(scope, global_agent_key.into(), global_agent.into());
    // 设置到全局
    let global: _ = context.global(scope);
    let http_key: _ = v8::String::new(scope, "http").unwrap();
    global.set(scope, http_key.into(), http_obj.into());

    let http_bootstrap_script = r#"
    (function() {
        if (!globalThis.http) return;
        const http = globalThis.http;

        http.METHODS = [
            'ACL', 'BIND', 'CHECKOUT', 'CONNECT', 'COPY', 'DELETE', 'GET', 'HEAD',
            'LINK', 'LOCK', 'M-SEARCH', 'MERGE', 'MKACTIVITY', 'MKCALENDAR', 'MKCOL',
            'MOVE', 'NOTIFY', 'OPTIONS', 'PATCH', 'POST', 'PRI', 'PROPFIND', 'PROPPATCH',
            'PURGE', 'PUT', 'REBIND', 'REPORT', 'SEARCH', 'SOURCE', 'SUBSCRIBE',
            'TRACE', 'UNBIND', 'UNLINK', 'UNLOCK', 'UNSUBSCRIBE'
        ];

        http.STATUS_CODES = {
            100: 'Continue',
            101: 'Switching Protocols',
            102: 'Processing',
            103: 'Early Hints',
            200: 'OK',
            201: 'Created',
            202: 'Accepted',
            203: 'Non-Authoritative Information',
            204: 'No Content',
            205: 'Reset Content',
            206: 'Partial Content',
            207: 'Multi-Status',
            208: 'Already Reported',
            226: 'IM Used',
            300: 'Multiple Choices',
            301: 'Moved Permanently',
            302: 'Found',
            303: 'See Other',
            304: 'Not Modified',
            305: 'Use Proxy',
            307: 'Temporary Redirect',
            308: 'Permanent Redirect',
            400: 'Bad Request',
            401: 'Unauthorized',
            402: 'Payment Required',
            403: 'Forbidden',
            404: 'Not Found',
            405: 'Method Not Allowed',
            406: 'Not Acceptable',
            407: 'Proxy Authentication Required',
            408: 'Request Timeout',
            409: 'Conflict',
            410: 'Gone',
            411: 'Length Required',
            412: 'Precondition Failed',
            413: 'Payload Too Large',
            414: 'URI Too Long',
            415: 'Unsupported Media Type',
            416: 'Range Not Satisfiable',
            417: 'Expectation Failed',
            418: "I'm a Teapot",
            421: 'Misdirected Request',
            422: 'Unprocessable Entity',
            423: 'Locked',
            424: 'Failed Dependency',
            425: 'Too Early',
            426: 'Upgrade Required',
            428: 'Precondition Required',
            429: 'Too Many Requests',
            431: 'Request Header Fields Too Large',
            451: 'Unavailable For Legal Reasons',
            500: 'Internal Server Error',
            501: 'Not Implemented',
            502: 'Bad Gateway',
            503: 'Service Unavailable',
            504: 'Gateway Timeout',
            505: 'HTTP Version Not Supported',
            506: 'Variant Also Negotiates',
            507: 'Insufficient Storage',
            508: 'Loop Detected',
            509: 'Bandwidth Limit Exceeded',
            510: 'Not Extended',
            511: 'Network Authentication Required'
        };

        const EE = globalThis.EventEmitter || function() {};
        const proto = (EE.prototype || Object.prototype);

        function getSocket() {
            if (!this._socket) {
                this._socket = new Socket();
            }
            return this._socket;
        }
        function setSocket(s) {
            this._socket = s;
        }

        function IncomingMessage() {
            if (typeof EE === 'function') EE.call(this);
            this._events = Object.create(null);
            this._eventsCount = 0;
            this.headers = Object.create(null);
            this.url = '/';
            this.method = 'GET';
            this.httpVersion = '1.1';
            this.complete = false;
        }
        IncomingMessage.prototype = Object.create(proto);
        IncomingMessage.prototype.constructor = IncomingMessage;
        Object.defineProperty(IncomingMessage.prototype, 'socket', { get: getSocket, set: setSocket, configurable: true, enumerable: true });
        Object.defineProperty(IncomingMessage.prototype, 'connection', { get: getSocket, set: setSocket, configurable: true, enumerable: true });
        Object.defineProperty(IncomingMessage.prototype, 'rawHeaders', {
            get() {
                if (!this._rawHeaders) {
                    this._rawHeaders = [];
                    if (this.headers) {
                        for (const k of Object.keys(this.headers)) {
                            this._rawHeaders.push(k, this.headers[k]);
                        }
                    }
                }
                return this._rawHeaders;
            },
            set(v) { this._rawHeaders = v; },
            configurable: true,
            enumerable: true
        });
        IncomingMessage.prototype.on = function(event, listener) {
            if (typeof listener === 'function') {
                if (event === 'data') {
                    this._dataListeners = this._dataListeners || [];
                    this._dataListeners.push(listener);
                } else if (event === 'end') {
                    this._endListeners = this._endListeners || [];
                    this._endListeners.push(listener);
                }
            }
            return (typeof EE === 'function' && EE.prototype.on ? EE.prototype.on : function(){}).apply(this, arguments);
        };
        IncomingMessage.prototype.addListener = IncomingMessage.prototype.on;
        IncomingMessage.prototype.setEncoding = function(encoding) {
            this._encoding = encoding;
            return this;
        };
        IncomingMessage.prototype.pause = function() {
            this._paused = true;
            return this;
        };
        IncomingMessage.prototype.resume = function() {
            this._paused = false;
            return this;
        };
        http.IncomingMessage = IncomingMessage;

        function ServerResponse() {
            if (typeof EE === 'function') EE.call(this);
            this._events = Object.create(null);
            this._eventsCount = 0;
            this.headers = Object.create(null);
            this._headersArray = [];
            this.statusCode = 200;
            this.statusMessage = 'OK';
            this._body = '';
            this._ended = false;
            this._responseSent = false;
            this._asyncPending = false;
            this.headersSent = false;
            this._connectionId = 0;
        }
        ServerResponse.prototype = Object.create(proto);
        ServerResponse.prototype.constructor = ServerResponse;
        Object.defineProperty(ServerResponse.prototype, 'socket', { get: getSocket, set: setSocket, configurable: true, enumerable: true });
        Object.defineProperty(ServerResponse.prototype, 'connection', { get: getSocket, set: setSocket, configurable: true, enumerable: true });
        ServerResponse.prototype.setHeader = function(name, value) {
            if (!this.headers) this.headers = Object.create(null);
            this.headers[name] = value;
            if (!this._headersArray) this._headersArray = [];
            this._headersArray.push(name, value);
            return this;
        };
        ServerResponse.prototype.getHeader = function(name) {
            if (!this.headers) return undefined;
            if (this.headers[name] !== undefined) return this.headers[name];
            const lower = String(name).toLowerCase();
            for (const k in this.headers) {
                if (k.toLowerCase() === lower) return this.headers[k];
            }
            return undefined;
        };
        ServerResponse.prototype.hasHeader = function(name) {
            if (!this.headers) return false;
            if (this.headers[name] !== undefined) return true;
            const lower = String(name).toLowerCase();
            for (const k in this.headers) {
                if (k.toLowerCase() === lower) return true;
            }
            return false;
        };
        ServerResponse.prototype.removeHeader = function(name) {
            if (this.headers) {
                delete this.headers[name];
                const lower = String(name).toLowerCase();
                for (const k in this.headers) {
                    if (k.toLowerCase() === lower) delete this.headers[k];
                }
            }
            if (this._headersArray) {
                const lower = String(name).toLowerCase();
                for (let i = 0; i < this._headersArray.length; i += 2) {
                    if (String(this._headersArray[i]).toLowerCase() === lower) {
                        this._headersArray.splice(i, 2);
                        i -= 2;
                    }
                }
            }
            return this;
        };
        ServerResponse.prototype.writeHead = function(statusCode, statusMessage, headers) {
            this.statusCode = statusCode;
            let hdrs = headers;
            if (typeof statusMessage === 'object' && statusMessage !== null && !headers) {
                hdrs = statusMessage;
            } else if (typeof statusMessage === 'string') {
                this.statusMessage = statusMessage;
            }
            if (hdrs && typeof hdrs === 'object') {
                for (const k of Object.keys(hdrs)) {
                    this.setHeader(k, hdrs[k]);
                }
            }
            return this;
        };
        ServerResponse.prototype.getHeaderNames = function() {
            return this.headers ? Object.keys(this.headers) : [];
        };
        ServerResponse.prototype.getHeaders = function() {
            return this.headers ? Object.assign(Object.create(null), this.headers) : Object.create(null);
        };
        ServerResponse.prototype.flushHeaders = function() {
            this.headersSent = true;
        };
        ServerResponse.prototype.write = function(chunk) {
            if (chunk !== undefined && chunk !== null) {
                this._body = (this._body || '') + String(chunk);
            }
            return true;
        };
        // Express & Hono compatibility helpers (v1.5.0)
        ServerResponse.prototype.status = function(code) {
            this.statusCode = code;
            return this;
        };
        ServerResponse.prototype.set = ServerResponse.prototype.setHeader;
        ServerResponse.prototype.header = ServerResponse.prototype.setHeader;
        ServerResponse.prototype.get = ServerResponse.prototype.getHeader;
        ServerResponse.prototype.json = function(body) {
            if (!this.hasHeader('content-type')) {
                this.setHeader('content-type', 'application/json');
            }
            const str = JSON.stringify(body);
            if (typeof this.end === 'function') {
                return this.end(str);
            }
            this.write(str);
            this._ended = true;
            return this;
        };
        ServerResponse.prototype.send = function(body) {
            if (typeof body === 'object' && body !== null) {
                return this.json(body);
            }
            if (typeof this.end === 'function') {
                return this.end(body);
            }
            if (body !== undefined && body !== null) {
                this.write(body);
            }
            this._ended = true;
            return this;
        };
        http.ServerResponse = ServerResponse;

        function Socket() {
            if (typeof EE === 'function') EE.call(this);
            this.remoteAddress = '127.0.0.1';
            this.remotePort = 12345;
            this.encrypted = false;
        }
        Socket.prototype = Object.create(proto);
        Socket.prototype.constructor = Socket;
        Socket.prototype.destroy = function() { return this; };
        Socket.prototype.ref = function() { return this; };
        Socket.prototype.unref = function() { return this; };
        Socket.prototype.setTimeout = function(msecs, callback) {
            if (typeof callback === 'function') callback();
            return this;
        };
        http.Socket = Socket;

        function Server(options, requestListener) {
            if (typeof EE === 'function') EE.call(this);
            this._events = Object.create(null);
            this._eventsCount = 0;
            if (typeof options === 'function') {
                requestListener = options;
                options = {};
            }
            if (typeof requestListener === 'function') {
                this.on('request', requestListener);
            }
        }
        Server.prototype = Object.create(proto);
        Server.prototype.constructor = Server;
        Server.prototype.setTimeout = function(msecs, callback) {
            if (typeof callback === 'function') this.on('timeout', callback);
            return this;
        };
        Server.prototype.ref = function() { return this; };
        Server.prototype.unref = function() { return this; };
        Server.prototype.closeAllConnections = function() { return this; };
        Server.prototype.closeIdleConnections = function() { return this; };
        Server.prototype.address = function() {
            return {
                port: this._serverPort || 0,
                family: (this._serverHost && this._serverHost.includes(':') && !this._serverHost.includes('.')) ? 'IPv6' : 'IPv4',
                address: this._serverHost || '0.0.0.0'
            };
        };
        Server.prototype.on = function(event, listener) {
            if (event === 'request' && typeof listener === 'function') {
                this._requestHandler = listener;
                if (typeof globalThis !== 'undefined') {
                    globalThis._httpServerRequestHandler = listener;
                }
            }
            return (typeof EE === 'function' && EE.prototype.on ? EE.prototype.on : function(){}).apply(this, arguments);
        };
        Server.prototype.addListener = Server.prototype.on;
        http.Server = Server;

        ServerResponse.prototype.assignSocket = function(socket) {
            this.socket = socket;
            this.connection = socket;
            if (typeof this.emit === 'function') {
                this.emit('socket', socket);
            }
            return this;
        };

        function ClientRequest() {
            if (typeof EE === 'function') EE.call(this);
        }
        ClientRequest.prototype = Object.create(proto);
        ClientRequest.prototype.constructor = ClientRequest;
        ClientRequest.prototype.assignSocket = function(socket) {
            this.socket = socket;
            this.connection = socket;
            if (typeof this.emit === 'function') {
                this.emit('socket', socket);
            }
            return this;
        };
        http.ClientRequest = ClientRequest;

        const _nativeCreateServer = http.createServer;
        http.createServer = function(options, requestListener) {
            const server = _nativeCreateServer.call(http, options, requestListener);
            Object.setPrototypeOf(server, Server.prototype);
            server._events = Object.create(null);
            server._eventsCount = 0;
            if (typeof options === 'function') {
                requestListener = options;
            }
            if (typeof requestListener === 'function') {
                server.on('request', requestListener);
            }
            return server;
        };

        const _fastSocket = {
            remoteAddress: '127.0.0.1',
            remotePort: 12345,
            encrypted: false,
            destroy() {},
            end() {},
        };

        function FastIncomingMessage(method, url, path, httpVersion, headers, complete, body) {
            this._events = Object.create(null);
            this._eventsCount = 0;
            this.method = method;
            this.url = url;
            this.path = path;
            this.httpVersion = httpVersion;
            this.headers = headers;
            const rh = [];
            for (const k in headers) {
                rh.push(k, headers[k]);
            }
            this.rawHeaders = rh;
            this.complete = complete;
            this.body = body;
            this._rawBody = body;
            this.socket = _fastSocket;
            this.connection = _fastSocket;
        }
        FastIncomingMessage.prototype = IncomingMessage.prototype;

        function FastServerResponse(req, connId) {
            this._events = Object.create(null);
            this._eventsCount = 0;
            this.req = req;
            this.headers = Object.create(null);
            this._headersArray = [];
            this.statusCode = 200;
            this.statusMessage = 'OK';
            this._body = '';
            this._ended = false;
            this._responseSent = false;
            this._asyncPending = false;
            this.headersSent = false;
            this._connectionId = connId;
        }
        FastServerResponse.prototype = ServerResponse.prototype;

        globalThis.__dispatchHttpRequest = function(method, url, path, httpVersion, headers, body, connId) {
            const req = new FastIncomingMessage(method, url, path, httpVersion, headers, true, body);
            const res = new FastServerResponse(req, connId);
            if (typeof globalThis._httpServerRequestHandler === 'function') {
                globalThis._httpServerRequestHandler(req, res);
                if (body && typeof req.emit === 'function') {
                    req.emit('data', body);
                    req.emit('end');
                }
                return res;
            }
            return null;
        };
    })();
    "#;
    if let Some(code) = v8::String::new(scope, http_bootstrap_script) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    // Attach native end callback to http.ServerResponse.prototype
    let sr_key = v8::String::new(scope, "ServerResponse").unwrap();
    if let Some(sr_val) = http_obj.get(scope, sr_key.into()) {
        if let Ok(sr_fn) = v8::Local::<v8::Function>::try_from(sr_val) {
            let proto_key = v8::String::new(scope, "prototype").unwrap();
            if let Some(proto_val) = sr_fn.get(scope, proto_key.into()) {
                if let Ok(proto_obj) = v8::Local::<v8::Object>::try_from(proto_val) {
                    let end_fn = v8::FunctionTemplate::new(scope, http_res_end_callback);
                    if let Some(end_instance) = end_fn.get_function(scope) {
                        let end_key = v8::String::new(scope, "end").unwrap();
                        proto_obj.set(scope, end_key.into(), end_instance.into());
                    }
                }
            }
        }
    }

    Ok(())
}

/// 创建默认的 Agent 实例 - v0.3.84 集成连接池
fn create_default_agent<'a>(scope: &mut v8::PinScope<'a, '_>) -> v8::Local<'a, v8::Object> {
    let agent_obj: _ = v8::Object::new(scope);

    // v0.3.84: 从全局获取或创建默认 Agent 配置
    let max_free_sockets = 10;
    let max_sockets = 20;
    let keep_alive = false;

    // maxFreeSockets
    let max_free_key: _ = v8::String::new(scope, "maxFreeSockets").unwrap();
    let max_free_val: _ = v8::Integer::new(scope, max_free_sockets as i32);
    agent_obj.set(scope, max_free_key.into(), max_free_val.into());
    // maxSockets
    let max_sockets_key: _ = v8::String::new(scope, "maxSockets").unwrap();
    let max_sockets_val: _ = v8::Integer::new(scope, max_sockets as i32);
    agent_obj.set(scope, max_sockets_key.into(), max_sockets_val.into());
    // keepAlive
    let keep_alive_key: _ = v8::String::new(scope, "keepAlive").unwrap();
    let keep_alive_val: _ = v8::Boolean::new(scope, keep_alive);
    agent_obj.set(scope, keep_alive_key.into(), keep_alive_val.into());

    // createConnection - v0.3.84: 返回连接池状态
    let create_conn_func: _ =
        v8::FunctionTemplate::new(scope, http_agent_create_connection_callback);
    let create_conn_instance: _ = create_conn_func.get_function(scope).unwrap();
    let create_conn_key: _ = v8::String::new(scope, "createConnection").unwrap();
    agent_obj.set(scope, create_conn_key.into(), create_conn_instance.into());

    // v0.3.84: 添加 getPoolStats 方法
    let get_stats_func: _ = v8::FunctionTemplate::new(scope, http_agent_get_pool_stats_callback);
    let get_stats_instance: _ = get_stats_func.get_function(scope).unwrap();
    let get_stats_key: _ = v8::String::new(scope, "getPoolStats").unwrap();
    agent_obj.set(scope, get_stats_key.into(), get_stats_instance.into());

    // v0.3.84: 添加 sockets 访问器
    let sockets_key: _ = v8::String::new(scope, "sockets").unwrap();
    let sockets_val: _ = v8::String::new(scope, &get_connection_pool_stats()).unwrap();
    agent_obj.set(scope, sockets_key.into(), sockets_val.into());

    agent_obj
}

/// Agent.getPoolStats() 回调 - v0.3.84
fn http_agent_get_pool_stats_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let stats = get_connection_pool_stats();
    let stats_val: _ = v8::String::new(scope, &stats).unwrap();
    retval.set(stats_val.into());
}
/// Shared by `http.createServer` and `https.createServer`.
pub(crate) fn build_http_server_object<'a>(
    scope: &mut v8::PinScope<'a, '_>,
) -> v8::Local<'a, v8::Object> {
    let server_obj = v8::Object::new(scope);

    let listen_func = v8::FunctionTemplate::new(scope, http_server_listen_callback);
    let listen_instance = listen_func.get_function(scope).unwrap();
    let listen_key = v8::String::new(scope, "listen").unwrap();
    server_obj.set(scope, listen_key.into(), listen_instance.into());

    let close_func = v8::FunctionTemplate::new(scope, http_server_close_callback);
    let close_instance = close_func.get_function(scope).unwrap();
    let close_key = v8::String::new(scope, "close").unwrap();
    server_obj.set(scope, close_key.into(), close_instance.into());

    let _message_channel = init_http_server_channel();
    let channel_key = v8::String::new(scope, "_messageChannel").unwrap();
    let channel_initialized = v8::Boolean::new(scope, true);
    server_obj.set(scope, channel_key.into(), channel_initialized.into());

    server_obj
}

/// v0.3.93: http.createServer callback 版本，可以访问全局对象
fn http_create_server_with_global_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let (options, request_handler) = if args.get(0).is_function() {
        (v8::undefined(scope).into(), args.get(0))
    } else if args.get(0).is_object() {
        (args.get(0), args.get(1))
    } else {
        (v8::undefined(scope).into(), args.get(0))
    };

    let server_obj = build_http_server_object(scope);
    if request_handler.is_function() {
        let handler_key = v8::String::new(scope, "_requestHandler").unwrap();
        server_obj.set(scope, handler_key.into(), request_handler);
        let context = scope.get_current_context();
        let global = context.global(scope);
        let global_handler_key = v8::String::new(scope, "_httpServerRequestHandler").unwrap();
        global.set(scope, global_handler_key.into(), request_handler);
    }
    attach_tls_options_to_server(scope, server_obj, options);
    retval.set(server_obj.into());
}

pub(crate) fn attach_tls_options_to_server(
    scope: &mut v8::PinScope,
    server_obj: v8::Local<v8::Object>,
    options: v8::Local<v8::Value>,
) {
    let cert = extract_string_option(scope, &options, "cert", "");
    let key = extract_string_option(scope, &options, "key", "");
    if cert.is_empty() && key.is_empty() {
        return;
    }
    let cert_key = v8::String::new(scope, "_tlsCert").unwrap();
    let cert_val = v8::String::new(scope, &cert).unwrap();
    server_obj.set(scope, cert_key.into(), cert_val.into());
    let key_key = v8::String::new(scope, "_tlsKey").unwrap();
    let key_val = v8::String::new(scope, &key).unwrap();
    server_obj.set(scope, key_key.into(), key_val.into());
    let tls_flag = v8::Boolean::new(scope, true);
    let tls_key = v8::String::new(scope, "_beeUsesTls").unwrap();
    server_obj.set(scope, tls_key.into(), tls_flag.into());
}

fn tls_config_from_server_object(
    scope: &mut v8::PinScope,
    server_obj: v8::Local<v8::Object>,
) -> Option<Arc<rustls::ServerConfig>> {
    let tls_flag_key = v8::String::new(scope, "_beeUsesTls").unwrap();
    let uses_tls = server_obj
        .get(scope, tls_flag_key.into())
        .map(|val| val.is_true())
        .unwrap_or(false);
    if !uses_tls {
        return None;
    }

    let cert_key = v8::String::new(scope, "_tlsCert").unwrap();
    let key_key = v8::String::new(scope, "_tlsKey").unwrap();
    let cert = server_obj
        .get(scope, cert_key.into())
        .filter(|value| !value.is_undefined() && !value.is_null())
        .and_then(|value| value.to_string(scope))
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let key = server_obj
        .get(scope, key_key.into())
        .filter(|value| !value.is_undefined() && !value.is_null())
        .and_then(|value| value.to_string(scope))
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if cert.is_empty() || key.is_empty() {
        return None;
    }
    match load_tls_material(&cert, &key) {
        Ok(cert_material) => Some(create_tls_server_config(
            &cert_material,
            &HttpsServerConfig::default(),
        )),
        Err(error) => {
            eprintln!("[Amber] Failed to load TLS certificate: {error}");
            None
        }
    }
}

/// http.Agent 构造函数回调
fn http_agent_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let options: _ = args.get(0);
    let agent_obj: _ = v8::Object::new(scope);

    // 解析 options 或使用默认值
    let max_free_sockets = extract_integer_option(scope, &options, "maxFreeSockets", 10);
    let max_sockets = extract_integer_option(scope, &options, "maxSockets", 20);
    let keep_alive = extract_boolean_option(scope, &options, "keepAlive", false);

    // 创建所有值再设置，避免 borrow checker 问题
    let max_free_val = v8::Integer::new(scope, max_free_sockets);
    let max_sockets_val = v8::Integer::new(scope, max_sockets);
    let keep_alive_val = v8::Boolean::new(scope, keep_alive);

    // maxFreeSockets
    let max_free_key: _ = v8::String::new(scope, "maxFreeSockets").unwrap();
    agent_obj.set(scope, max_free_key.into(), max_free_val.into());

    // maxSockets
    let max_sockets_key: _ = v8::String::new(scope, "maxSockets").unwrap();
    agent_obj.set(scope, max_sockets_key.into(), max_sockets_val.into());

    // keepAlive
    let keep_alive_key: _ = v8::String::new(scope, "keepAlive").unwrap();
    agent_obj.set(scope, keep_alive_key.into(), keep_alive_val.into());

    // createConnection
    let create_conn_func: _ =
        v8::FunctionTemplate::new(scope, http_agent_create_connection_callback);
    let create_conn_instance: _ = create_conn_func.get_function(scope).unwrap();
    let create_conn_key: _ = v8::String::new(scope, "createConnection").unwrap();
    agent_obj.set(scope, create_conn_key.into(), create_conn_instance.into());

    retval.set(agent_obj.into());
}

/// 提取整数选项
fn extract_integer_option(
    scope: &mut v8::PinScope,
    options: &v8::Local<v8::Value>,
    key: &str,
    default: i32,
) -> i32 {
    if options.is_undefined() || options.is_null() {
        return default;
    }
    if let Ok(obj) = v8::Local::<v8::Object>::try_from(*options) {
        let key_str: _ = v8::String::new(scope, key).unwrap();
        if let Some(val) = obj.get(scope, key_str.into()) {
            if val.is_number() {
                return val
                    .to_integer(scope)
                    .unwrap_or(v8::Integer::new(scope, default))
                    .value() as i32;
            }
        }
    }
    default
}

/// 提取布尔选项
fn extract_boolean_option(
    scope: &mut v8::PinScope,
    options: &v8::Local<v8::Value>,
    key: &str,
    default: bool,
) -> bool {
    if options.is_undefined() || options.is_null() {
        return default;
    }
    if let Ok(obj) = v8::Local::<v8::Object>::try_from(*options) {
        let key_str: _ = v8::String::new(scope, key).unwrap();
        if let Some(val) = obj.get(scope, key_str.into()) {
            return val.to_boolean(scope).is_true();
        }
    }
    default
}

/// 提取字符串选项 - v0.3.65
fn extract_string_option(
    scope: &mut v8::PinScope,
    options: &v8::Local<v8::Value>,
    key: &str,
    default: &str,
) -> String {
    if options.is_undefined() || options.is_null() {
        return default.to_string();
    }
    if let Ok(obj) = v8::Local::<v8::Object>::try_from(*options) {
        let key_str: _ = v8::String::new(scope, key).unwrap();
        if let Some(val) = obj.get(scope, key_str.into()) {
            if val.is_undefined() || val.is_null() {
                return default.to_string();
            }
            if let Some(s) = val.to_string(scope) {
                return s.to_rust_string_lossy(scope);
            }
        }
    }
    default.to_string()
}

/// DNS 解析辅助函数 - v0.3.68
/// 将主机名解析为 IP 地址
fn resolve_hostname(hostname: &str, port: u16) -> Result<SocketAddr, String> {
    // 处理 localhost
    if hostname == "localhost" {
        // 尝试创建 IPv4 SocketAddr
        let addr: SocketAddr = ([127, 0, 0, 1], port).into();
        return Ok(addr);
    }

    // 尝试解析为 IP 地址（IPv4 或 IPv6）
    if let Ok(addr) = format!("{}:{}", hostname, port).parse::<SocketAddr>() {
        return Ok(addr);
    }

    // 执行 DNS 解析
    let addr_format = format!("{}:{}", hostname, port);
    match addr_format.to_socket_addrs() {
        Ok(addrs) => {
            // 将迭代器收集为 Vec
            let addrs_vec: Vec<SocketAddr> = addrs.collect();
            // 返回第一个地址
            addrs_vec
                .first()
                .copied()
                .ok_or_else(|| "No addresses found".to_string())
        }
        Err(e) => Err(format!("DNS resolution failed: {}", e)),
    }
}

fn http_network_resource_url(host: &str, port: u16, path: &str) -> String {
    let normalized_path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };
    let formatted_host = if host.contains(':') && !host.starts_with('[') {
        format!("[{}]", host)
    } else {
        host.to_string()
    };
    format!("http://{}:{}{}", formatted_host, port, normalized_path)
}

fn check_http_network_permission(host: &str, port: u16, path: &str) -> Result<(), String> {
    check_global_permission(
        PermissionKind::Network,
        PermissionAction::Connect,
        ResourceId::Url(http_network_resource_url(host, port, path)),
    )
    .map_err(|error| error.to_string())
}

fn check_http_network_listen_permission(host: &str, port: u16) -> Result<(), String> {
    check_global_permission(
        PermissionKind::Network,
        PermissionAction::Listen,
        ResourceId::Url(http_network_resource_url(host, port, "/")),
    )
    .map_err(|error| error.to_string())
}

fn throw_http_permission_error(scope: &mut v8::PinScope, message: &str) {
    let error_message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    scope.throw_exception(error);
}

/// 从 options 中提取 port - v0.3.68
fn extract_port(scope: &mut v8::PinScope, options: &v8::Local<v8::Value>, default: u16) -> u16 {
    if options.is_undefined() || options.is_null() {
        return default;
    }
    if let Ok(obj) = v8::Local::<v8::Object>::try_from(*options) {
        let key_str = v8::String::new(scope, "port").unwrap();
        if let Some(val) = obj.get(scope, key_str.into()) {
            if val.is_number() {
                return val.to_int32(scope).unwrap().value() as u16;
            }
        }
    }
    default
}

/// Agent.createConnection 回调
fn http_agent_create_connection_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 返回一个模拟的 socket 对象
    let socket_obj: _ = v8::Object::new(scope);
    let connect_key: _ = v8::String::new(scope, "connect").unwrap();
    let connect_val: _ = v8::String::new(scope, "[Socket connected]").unwrap();
    socket_obj.set(scope, connect_key.into(), connect_val.into());
    retval.set(socket_obj.into());
}

/// http.Server.close 回调 - v0.3.87 更新
fn http_server_close_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();

    let port = {
        let port_key = v8::String::new(scope, "_serverPort").unwrap();
        this.get(scope, port_key.into())
            .and_then(|value| value.to_integer(scope))
            .map(|value| value.value() as u16)
    };

    let host = {
        let host_key = v8::String::new(scope, "_serverHost").unwrap();
        this.get(scope, host_key.into())
            .and_then(|value| value.to_string(scope))
            .map(|value| value.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "0.0.0.0".to_string())
    };

    if let Some(port) = port {
        stop_http_server_state(&host, port);
    }

    // 设置 listening 为 false
    let listening_key = v8::String::new(scope, "listening").unwrap();
    let listening_val = v8::Boolean::new(scope, false);
    this.set(scope, listening_key.into(), listening_val.into());

    // 打印关闭信息
    eprintln!("[Amber] HTTP Server closed");

    retval.set(this.into());
}
fn http_request_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let options: _ = args.get(0);
    let callback: _ = args.get(1);

    // 解析请求选项
    let method = extract_string_option(scope, &options, "method", "GET");
    let hostname = extract_string_option(scope, &options, "hostname", "localhost");
    let port = extract_port(scope, &options, 80);
    let path = extract_string_option(scope, &options, "path", "/");

    // 创建请求对象
    let req_obj: _ = v8::Object::new(scope);

    // 存储请求选项到对象属性
    let method_key: _ = v8::String::new(scope, "method").unwrap();
    let method_val: _ = v8::String::new(scope, &method).unwrap();
    req_obj.set(scope, method_key.into(), method_val.into());

    let hostname_key: _ = v8::String::new(scope, "hostname").unwrap();
    let hostname_val: _ = v8::String::new(scope, &hostname).unwrap();
    req_obj.set(scope, hostname_key.into(), hostname_val.into());

    let port_key: _ = v8::String::new(scope, "port").unwrap();
    let port_val: _ = v8::Integer::new(scope, port as i32);
    req_obj.set(scope, port_key.into(), port_val.into());

    let path_key: _ = v8::String::new(scope, "path").unwrap();
    let path_val: _ = v8::String::new(scope, &path).unwrap();
    req_obj.set(scope, path_key.into(), path_val.into());

    // v0.3.68: 执行 DNS 解析并存储解析结果
    let resolved_addr_key: _ = v8::String::new(scope, "_resolvedAddress").unwrap();
    match resolve_hostname(&hostname, port) {
        Ok(socket_addr) => {
            let addr_val: _ = v8::String::new(scope, &socket_addr.to_string()).unwrap();
            req_obj.set(scope, resolved_addr_key.into(), addr_val.into());
        }
        Err(e) => {
            let undefined: _ = v8::undefined(scope);
            req_obj.set(scope, resolved_addr_key.into(), undefined.into());
            // 可以在控制台输出错误（可选）
            eprintln!("[Amber] DNS resolution warning for '{}': {}", hostname, e);
        }
    }

    // 提取 headers
    let headers_key_str: _ = v8::String::new(scope, "headers").unwrap();
    let headers = options
        .is_object()
        .then(|| {
            let obj = v8::Local::<v8::Object>::try_from(options).ok()?;
            obj.get(scope, headers_key_str.into())
        })
        .flatten()
        .unwrap_or(v8::undefined(scope).into());
    let headers_key: _ = v8::String::new(scope, "_headers").unwrap();
    req_obj.set(scope, headers_key.into(), headers);

    // end 方法 - 发送请求并触发回调
    let end_func: _ = v8::FunctionTemplate::new(scope, http_req_end_callback);
    let end_instance: _ = end_func.get_function(scope).unwrap();
    let end_key: _ = v8::String::new(scope, "end").unwrap();
    req_obj.set(scope, end_key.into(), end_instance.into());

    // write 方法 - 写入请求体
    let write_func: _ = v8::FunctionTemplate::new(scope, http_req_write_callback);
    let write_instance: _ = write_func.get_function(scope).unwrap();
    let write_key: _ = v8::String::new(scope, "write").unwrap();
    req_obj.set(scope, write_key.into(), write_instance.into());

    // 设置响应回调
    let response_callback_key: _ = v8::String::new(scope, "_responseCallback").unwrap();
    if callback.is_function() {
        req_obj.set(scope, response_callback_key.into(), callback);
    }

    retval.set(req_obj.into());
}
fn http_get_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let options: _ = args.get(0);
    let callback: _ = args.get(1);

    // 解析请求选项（http.get 固定为 GET 方法）
    let hostname = extract_string_option(scope, &options, "hostname", "localhost");
    let port = extract_port(scope, &options, 80);
    let path = extract_string_option(scope, &options, "path", "/");

    // 创建请求对象
    let req_obj: _ = v8::Object::new(scope);

    // method - 固定为 GET
    let method_key: _ = v8::String::new(scope, "method").unwrap();
    let method_val: _ = v8::String::new(scope, "GET").unwrap();
    req_obj.set(scope, method_key.into(), method_val.into());

    let hostname_key: _ = v8::String::new(scope, "hostname").unwrap();
    let hostname_val: _ = v8::String::new(scope, &hostname).unwrap();
    req_obj.set(scope, hostname_key.into(), hostname_val.into());

    let port_key: _ = v8::String::new(scope, "port").unwrap();
    let port_val: _ = v8::Integer::new(scope, port as i32);
    req_obj.set(scope, port_key.into(), port_val.into());

    let path_key: _ = v8::String::new(scope, "path").unwrap();
    let path_val: _ = v8::String::new(scope, &path).unwrap();
    req_obj.set(scope, path_key.into(), path_val.into());

    // v0.3.68: 执行 DNS 解析并存储解析结果
    let resolved_addr_key: _ = v8::String::new(scope, "_resolvedAddress").unwrap();
    match resolve_hostname(&hostname, port) {
        Ok(socket_addr) => {
            let addr_val: _ = v8::String::new(scope, &socket_addr.to_string()).unwrap();
            req_obj.set(scope, resolved_addr_key.into(), addr_val.into());
        }
        Err(_) => {
            let undefined: _ = v8::undefined(scope);
            req_obj.set(scope, resolved_addr_key.into(), undefined.into());
        }
    }

    // 提取 headers
    let headers_key_str: _ = v8::String::new(scope, "headers").unwrap();
    let headers = options
        .is_object()
        .then(|| {
            let obj = v8::Local::<v8::Object>::try_from(options).ok()?;
            obj.get(scope, headers_key_str.into())
        })
        .flatten()
        .unwrap_or(v8::undefined(scope).into());
    let headers_key: _ = v8::String::new(scope, "_headers").unwrap();
    req_obj.set(scope, headers_key.into(), headers);

    // end 方法
    let end_func: _ = v8::FunctionTemplate::new(scope, http_req_end_callback);
    let end_instance: _ = end_func.get_function(scope).unwrap();
    let end_key: _ = v8::String::new(scope, "end").unwrap();
    req_obj.set(scope, end_key.into(), end_instance.into());

    // write 方法
    let write_func: _ = v8::FunctionTemplate::new(scope, http_req_write_callback);
    let write_instance: _ = write_func.get_function(scope).unwrap();
    let write_key: _ = v8::String::new(scope, "write").unwrap();
    req_obj.set(scope, write_key.into(), write_instance.into());

    // 设置回调
    let response_callback_key: _ = v8::String::new(scope, "_responseCallback").unwrap();
    if callback.is_function() {
        req_obj.set(scope, response_callback_key.into(), callback);
    }

    retval.set(req_obj.into());
}
fn http_server_listen_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();

    // 解析参数: 支持多种调用方式
    // - listen(port)
    // - listen(port, callback)
    // - listen(port, host, callback)
    // - listen(options, callback)
    let (port, host, callback) = {
        let arg0 = args.get(0);
        if arg0.is_object() && !arg0.is_function() {
            let opts = arg0.to_object(scope).unwrap();
            let port_key = v8::String::new(scope, "port").unwrap();
            let host_key = v8::String::new(scope, "host").unwrap();
            let p = opts
                .get(scope, port_key.into())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as u16)
                .unwrap_or(3000);
            let h = opts
                .get(scope, host_key.into())
                .and_then(|v| v.to_string(scope))
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "0.0.0.0".to_string());
            let cb = if args.get(1).is_function() {
                args.get(1)
            } else {
                let cb_key = v8::String::new(scope, "cb").unwrap();
                opts.get(scope, cb_key.into())
                    .unwrap_or_else(|| v8::undefined(scope).into())
            };
            (p, h, cb)
        } else {
            let p = arg0
                .to_integer(scope)
                .map(|i| i.value() as u16)
                .unwrap_or(3000);
            let arg1 = args.get(1);
            let (h, cb) = if arg1.is_undefined() || arg1.is_null() {
                ("0.0.0.0".to_string(), args.get(2))
            } else if arg1.is_function() {
                ("0.0.0.0".to_string(), arg1)
            } else if arg1.is_string() {
                let host_str = arg1
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "0.0.0.0".to_string());
                (host_str, args.get(2))
            } else {
                ("0.0.0.0".to_string(), args.get(2))
            };
            (p, h, cb)
        }
    };

    if let Err(error) = check_http_network_listen_permission(&host, port) {
        let listening_key = v8::String::new(scope, "listening").unwrap();
        let listening_val = v8::Boolean::new(scope, false);
        this.set(scope, listening_key.into(), listening_val.into());
        throw_http_permission_error(scope, &error);
        return;
    }

    let channel = get_http_server_channel();
    let use_message_channel = channel.is_some();
    let tls_config = tls_config_from_server_object(scope, this);
    let server_state = Arc::new(HttpServerState {
        listening: Arc::new(AtomicBool::new(true)),
        port,
        host: host.clone(),
        use_message_channel,
        bound_port: Arc::new(AtomicU64::new(port as u64)),
        tls_config,
        channel,
    });
    register_http_server_state(server_state.clone());

    // Always bind. `on('request')` after listen still needs the accept thread.
    let state_clone = server_state.clone();
    thread::spawn(move || {
        run_http_server(state_clone, "handler".to_string());
    });
    thread::sleep(Duration::from_millis(5));
    let advertised_port = {
        let bound = server_state.bound_port.load(Ordering::SeqCst) as u16;
        if bound != 0 {
            bound
        } else {
            port
        }
    };

    // 设置属性
    let listening_key = v8::String::new(scope, "listening").unwrap();
    let listening_val = v8::Boolean::new(scope, true);
    this.set(scope, listening_key.into(), listening_val.into());

    let port_key = v8::String::new(scope, "port").unwrap();
    let port_val = v8::Integer::new(scope, advertised_port as i32);
    this.set(scope, port_key.into(), port_val.into());

    let address_fn = v8::FunctionTemplate::new(scope, http_server_address_callback);
    let address_instance = address_fn.get_function(scope).unwrap();
    let address_key = v8::String::new(scope, "address").unwrap();
    this.set(scope, address_key.into(), address_instance.into());

    let server_port_key = v8::String::new(scope, "_serverPort").unwrap();
    let server_port_val = v8::Integer::new(scope, advertised_port as i32);
    this.set(scope, server_port_key.into(), server_port_val.into());

    let server_host_key = v8::String::new(scope, "_serverHost").unwrap();
    let server_host_val = v8::String::new(scope, &host).unwrap();
    this.set(scope, server_host_key.into(), server_host_val.into());

    // 如果提供了回调函数，挂载为 'listening' 的一次性监听器
    if callback.is_function() {
        let once_key = v8::String::new(scope, "once").unwrap();
        if let Some(once_val) = this.get(scope, once_key.into()) {
            if let Ok(once_func) = v8::Local::<v8::Function>::try_from(once_val) {
                let listening_event = v8::String::new(scope, "listening").unwrap();
                let _ = once_func.call(scope, this.into(), &[listening_event.into(), callback]);
            }
        }
    }

    // 触发 'listening' 事件
    let emit_key = v8::String::new(scope, "emit").unwrap();
    if let Some(emit_val) = this.get(scope, emit_key.into()) {
        if let Ok(emit_func) = v8::Local::<v8::Function>::try_from(emit_val) {
            let listening_event = v8::String::new(scope, "listening").unwrap();
            let _ = emit_func.call(scope, this.into(), &[listening_event.into()]);
        }
    }

    // 返回 this
    retval.set(this.into());
}

fn http_server_address_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let port = {
        let port_key = v8::String::new(scope, "_serverPort").unwrap();
        this.get(scope, port_key.into())
            .and_then(|value| value.to_integer(scope))
            .map(|value| value.value() as i32)
            .unwrap_or(0)
    };
    let host = {
        let host_key = v8::String::new(scope, "_serverHost").unwrap();
        this.get(scope, host_key.into())
            .and_then(|value| value.to_string(scope))
            .map(|value| value.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "0.0.0.0".to_string())
    };
    let family = if host.contains(':') && !host.contains('.') {
        "IPv6"
    } else {
        "IPv4"
    };

    let obj = v8::Object::new(scope);
    let port_key = v8::String::new(scope, "port").unwrap();
    let port_val = v8::Integer::new(scope, port);
    obj.set(scope, port_key.into(), port_val.into());
    let family_key = v8::String::new(scope, "family").unwrap();
    let family_val = v8::String::new(scope, family).unwrap();
    obj.set(scope, family_key.into(), family_val.into());
    let address_key = v8::String::new(scope, "address").unwrap();
    let address_val = v8::String::new(scope, &host).unwrap();
    obj.set(scope, address_key.into(), address_val.into());
    retval.set(obj.into());
}

#[allow(dead_code)]
fn http_server_on_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let listener: _ = args.get(1);
    if !listener.is_function() {
        retval.set(v8::null(scope).into());
        return;
    }

    // v0.3.83: Store listener for 'request' events (real HTTP handling coming later)
    if event == "request" {
        // Store the request handler for later use
        let handler_key = v8::String::new(scope, "_requestHandler").unwrap();
        this.set(scope, handler_key.into(), listener);

        let context = scope.get_current_context();
        if let Ok(handler_fn) = v8::Local::<v8::Function>::try_from(listener) {
            set_global_request_handler(scope, &context, handler_fn);
        }
    }

    // 支持链式调用
    retval.set(this.into());
}
/// http.request().end() 回调 - v0.3.84 集成连接池
fn http_req_end_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let callback: _ = args.get(0);

    // 从请求对象提取选项
    let method =
        extract_string_property(scope, &this, "method").unwrap_or_else(|| "GET".to_string());
    let host = extract_string_property(scope, &this, "hostname")
        .unwrap_or_else(|| "localhost".to_string());
    let port = extract_integer_property(scope, &this, "port").unwrap_or(80);
    let path = extract_string_property(scope, &this, "path").unwrap_or_else(|| "/".to_string());
    let body = extract_string_property(scope, &this, "_body").unwrap_or_default();

    if let Err(error) = check_http_network_permission(&host, port as u16, &path) {
        throw_http_permission_error(scope, &error);
        return;
    }

    // v0.3.84: 从连接池获取连接
    let connection_acquired = acquire_http_connection(&host, port as u16);
    if !connection_acquired {
        eprintln!(
            "[Amber] HTTP connection pool exhausted for {}:{}, active: {}",
            host,
            port,
            get_connection_pool_stats()
        );
    }

    // v0.3.73: 尝试发送真实的 HTTP 请求
    let http_response = sync_http_request(
        HttpRequestOptions {
            method: method.clone(),
            host: host.clone(),
            port: port as u16,
            path: path.clone(),
            headers: vec![],
            body: body.into_bytes(),
        },
        10, // 10秒超时
    );

    // v0.3.84: 释放连接回连接池
    release_http_connection(&host, port as u16);

    // 使用真实响应或回退到模拟响应
    let (status_code, status_message, response_headers, response_body) = match http_response {
        Ok(resp) => (
            resp.status_code as i32,
            resp.status_message,
            resp.headers,
            resp.body,
        ),
        Err(e) => {
            eprintln!("[Amber] HTTP request failed: {}", e);
            (200, "OK".to_string(), vec![], vec![])
        }
    };

    // 创建响应对象
    let res_obj = create_response_object_with_data(
        scope,
        status_code,
        &status_message,
        &response_headers,
        &response_body,
    );

    // v0.3.84: 在响应对象中存储连接池统计
    let pool_stats_key: _ = v8::String::new(scope, "_poolStats").unwrap();
    let pool_stats_val: _ = v8::String::new(scope, &get_connection_pool_stats()).unwrap();
    res_obj.set(scope, pool_stats_key.into(), pool_stats_val.into());

    // 优先使用传入的回调，其次使用请求对象中存储的回调
    let response_callback = if callback.is_function() {
        callback
    } else {
        let cb_key: _ = v8::String::new(scope, "_responseCallback").unwrap();
        this.get(scope, cb_key.into())
            .unwrap_or(v8::undefined(scope).into())
    };

    if response_callback.is_function() {
        if let Ok(cb_func) = v8::Local::<v8::Function>::try_from(response_callback) {
            let call_args: &[v8::Local<v8::Value>] = &[res_obj.into()];
            cb_func.call(scope, this.into(), call_args);
        }
    }
    retval.set(this.into());
}

/// 创建响应对象 - v0.3.65
#[allow(dead_code)]
fn create_response_object<'a>(scope: &mut v8::PinScope<'a, '_>) -> v8::Local<'a, v8::Object> {
    let res_obj: _ = v8::Object::new(scope);

    // statusCode
    let status_code_key: _ = v8::String::new(scope, "statusCode").unwrap();
    let status_val: _ = v8::Integer::new(scope, 200);
    res_obj.set(scope, status_code_key.into(), status_val.into());

    // statusMessage
    let status_msg_key: _ = v8::String::new(scope, "statusMessage").unwrap();
    let status_msg_val: _ = v8::String::new(scope, "OK").unwrap();
    res_obj.set(scope, status_msg_key.into(), status_msg_val.into());

    // headers
    let headers_key: _ = v8::String::new(scope, "headers").unwrap();
    let headers_obj: _ = v8::Object::new(scope);
    let content_type_key: _ = v8::String::new(scope, "content-type").unwrap();
    let content_type_val: _ = v8::String::new(scope, "text/plain").unwrap();
    headers_obj.set(scope, content_type_key.into(), content_type_val.into());
    res_obj.set(scope, headers_key.into(), headers_obj.into());

    // getAllHeaders
    let get_headers_func: _ = v8::FunctionTemplate::new(scope, http_res_get_all_headers_callback);
    let get_headers_instance: _ = get_headers_func.get_function(scope).unwrap();
    let get_headers_key: _ = v8::String::new(scope, "getAllHeaders").unwrap();
    res_obj.set(scope, get_headers_key.into(), get_headers_instance.into());

    // getHeader
    let get_header_func: _ = v8::FunctionTemplate::new(scope, http_res_get_header_callback);
    let get_header_instance: _ = get_header_func.get_function(scope).unwrap();
    let get_header_key: _ = v8::String::new(scope, "getHeader").unwrap();
    res_obj.set(scope, get_header_key.into(), get_header_instance.into());

    // setHeader
    let set_header_func: _ = v8::FunctionTemplate::new(scope, http_res_set_header_callback);
    let set_header_instance: _ = set_header_func.get_function(scope).unwrap();
    let set_header_key: _ = v8::String::new(scope, "setHeader").unwrap();
    res_obj.set(scope, set_header_key.into(), set_header_instance.into());

    // end
    let end_func: _ = v8::FunctionTemplate::new(scope, http_res_end_callback);
    let end_instance: _ = end_func.get_function(scope).unwrap();
    let end_key: _ = v8::String::new(scope, "end").unwrap();
    res_obj.set(scope, end_key.into(), end_instance.into());

    // writeHead
    let write_head_func: _ = v8::FunctionTemplate::new(scope, http_res_write_head_callback);
    let write_head_instance: _ = write_head_func.get_function(scope).unwrap();
    let write_head_key: _ = v8::String::new(scope, "writeHead").unwrap();
    res_obj.set(scope, write_head_key.into(), write_head_instance.into());

    // removeHeader - v0.3.87
    let remove_header_func: _ = v8::FunctionTemplate::new(scope, http_res_remove_header_callback);
    let remove_header_instance: _ = remove_header_func.get_function(scope).unwrap();
    let remove_header_key: _ = v8::String::new(scope, "removeHeader").unwrap();
    res_obj.set(
        scope,
        remove_header_key.into(),
        remove_header_instance.into(),
    );

    res_obj
}

/// 创建响应对象（带真实数据）- v0.3.73
fn create_response_object_with_data<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    status_code: i32,
    status_message: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> v8::Local<'a, v8::Object> {
    let res_obj: _ = v8::Object::new(scope);

    // statusCode
    let status_code_key: _ = v8::String::new(scope, "statusCode").unwrap();
    let status_val: _ = v8::Integer::new(scope, status_code);
    res_obj.set(scope, status_code_key.into(), status_val.into());

    // statusMessage
    let status_msg_key: _ = v8::String::new(scope, "statusMessage").unwrap();
    let status_msg_val: _ = v8::String::new(scope, status_message).unwrap();
    res_obj.set(scope, status_msg_key.into(), status_msg_val.into());

    // headers
    let headers_key: _ = v8::String::new(scope, "headers").unwrap();
    let headers_obj: _ = v8::Object::new(scope);
    for (key, value) in headers {
        let key_str: _ = v8::String::new(scope, key).unwrap();
        let value_str: _ = v8::String::new(scope, value).unwrap();
        headers_obj.set(scope, key_str.into(), value_str.into());
    }
    res_obj.set(scope, headers_key.into(), headers_obj.into());

    // body - 存储为字符串
    let body_key: _ = v8::String::new(scope, "body").unwrap();
    let body_str = match std::str::from_utf8(body) {
        Ok(s) => v8::String::new(scope, s).unwrap(),
        Err(_) => v8::String::new(scope, "[binary data]").unwrap(),
    };
    res_obj.set(scope, body_key.into(), body_str.into());

    // bodyLength
    let body_length_key: _ = v8::String::new(scope, "bodyLength").unwrap();
    let body_length_val = v8::Integer::new(scope, body.len() as i32);
    res_obj.set(scope, body_length_key.into(), body_length_val.into());

    // getAllHeaders
    let get_headers_func: _ = v8::FunctionTemplate::new(scope, http_res_get_all_headers_callback);
    let get_headers_instance: _ = get_headers_func.get_function(scope).unwrap();
    let get_headers_key: _ = v8::String::new(scope, "getAllHeaders").unwrap();
    res_obj.set(scope, get_headers_key.into(), get_headers_instance.into());

    // getHeader
    let get_header_func: _ = v8::FunctionTemplate::new(scope, http_res_get_header_callback);
    let get_header_instance: _ = get_header_func.get_function(scope).unwrap();
    let get_header_key: _ = v8::String::new(scope, "getHeader").unwrap();
    res_obj.set(scope, get_header_key.into(), get_header_instance.into());

    // setHeader
    let set_header_func: _ = v8::FunctionTemplate::new(scope, http_res_set_header_callback);
    let set_header_instance: _ = set_header_func.get_function(scope).unwrap();
    let set_header_key: _ = v8::String::new(scope, "setHeader").unwrap();
    res_obj.set(scope, set_header_key.into(), set_header_instance.into());

    // end
    let end_func: _ = v8::FunctionTemplate::new(scope, http_res_end_callback);
    let end_instance: _ = end_func.get_function(scope).unwrap();
    let end_key: _ = v8::String::new(scope, "end").unwrap();
    res_obj.set(scope, end_key.into(), end_instance.into());

    // writeHead
    let write_head_func: _ = v8::FunctionTemplate::new(scope, http_res_write_head_callback);
    let write_head_instance: _ = write_head_func.get_function(scope).unwrap();
    let write_head_key: _ = v8::String::new(scope, "writeHead").unwrap();
    res_obj.set(scope, write_head_key.into(), write_head_instance.into());

    res_obj
}

/// 从 V8 对象提取字符串属性 - v0.3.73
fn extract_string_property(
    scope: &mut v8::PinScope,
    obj: &v8::Local<v8::Object>,
    key: &str,
) -> Option<String> {
    let key_str: _ = v8::String::new(scope, key).unwrap();
    if let Some(val) = obj.get(scope, key_str.into()) {
        if val.is_string() {
            let s = val.to_string(scope).unwrap();
            return Some(s.to_rust_string_lossy(scope));
        }
    }
    None
}

/// 从 V8 对象提取整数属性 - v0.3.73
fn extract_integer_property(
    scope: &mut v8::PinScope,
    obj: &v8::Local<v8::Object>,
    key: &str,
) -> Option<i32> {
    let key_str: _ = v8::String::new(scope, key).unwrap();
    if let Some(val) = obj.get(scope, key_str.into()) {
        if val.is_number() {
            if let Some(int_val) = val.to_int32(scope) {
                return Some(int_val.value() as i32);
            }
        }
    }
    None
}

/// http.request().write() 回调 - v0.3.65
fn http_req_write_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let chunk: _ = args.get(0);

    // 存储写入的数据
    if !chunk.is_undefined() {
        let body_key: _ = v8::String::new(scope, "_body").unwrap();
        // 获取现有 body
        let existing_body = this
            .get(scope, body_key.into())
            .unwrap_or(v8::undefined(scope).into());

        // 追加新数据
        if existing_body.is_string() {
            // 字符串拼接 - 预先构建 Rust 字符串避免双重借用
            let existing_str = existing_body.to_string(scope).unwrap();
            let existing_rust = existing_str.to_rust_string_lossy(scope);

            let chunk_rust = if chunk.is_string() {
                let chunk_str = chunk.to_string(scope).unwrap();
                chunk_str.to_rust_string_lossy(scope)
            } else {
                "[chunk]".to_string()
            };

            let combined_rust = format!("{}{}", existing_rust, chunk_rust);
            let combined = v8::String::new(scope, &combined_rust).unwrap();
            this.set(scope, body_key.into(), combined.into());
        } else {
            // 存储新数据
            this.set(scope, body_key.into(), chunk);
        }
    }

    retval.set(this.into());
}

/// response.getAllHeaders() 回调 - v0.3.64
fn http_res_get_all_headers_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();

    // 获取 headers 对象
    let headers_key: _ = v8::String::new(scope, "headers").unwrap();
    let headers: _ = this.get(scope, headers_key.into());

    if let Some(h) = headers {
        retval.set(h);
    } else {
        // 如果没有 headers，返回空数组
        let empty_array: _ = v8::Array::new(scope, 0);
        retval.set(empty_array.into());
    }
}

/// response.getHeader() 回调 - v0.3.64
fn http_res_get_header_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let name: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let headers_key: _ = v8::String::new(scope, "headers").unwrap();
    let headers_obj: _ = this.get(scope, headers_key.into());

    if let Ok(obj) =
        v8::Local::<v8::Object>::try_from(headers_obj.unwrap_or(v8::undefined(scope).into()))
    {
        let name_key: _ = v8::String::new(scope, &name).unwrap();
        let value: _ = obj.get(scope, name_key.into());
        if let Some(v) = value {
            retval.set(v);
        } else {
            retval.set(v8::undefined(scope).into());
        }
    } else {
        retval.set(v8::undefined(scope).into());
    }
}

/// response.setHeader() 回调 - v0.3.64
pub fn http_res_set_header_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let name: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let value: _ = args.get(1);

    let headers_key: _ = v8::String::new(scope, "headers").unwrap();
    let headers_obj = if let Ok(obj) = v8::Local::<v8::Object>::try_from(
        this.get(scope, headers_key.into())
            .unwrap_or(v8::undefined(scope).into()),
    ) {
        obj
    } else {
        v8::Object::new(scope)
    };

    let name_key: _ = v8::String::new(scope, &name).unwrap();
    headers_obj.set(scope, name_key.into(), value);
    this.set(scope, headers_key.into(), headers_obj.into());

    retval.set(this.into());
}

/// response.hasHeader() 回调 - 不区分大小写检查响应头存在性
pub fn http_res_has_header_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let name = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default()
        .to_lowercase();

    let mut found = false;
    let headers_key = v8::String::new(scope, "headers").unwrap();
    if let Some(headers_val) = this.get(scope, headers_key.into()) {
        if let Ok(headers_obj) = v8::Local::<v8::Object>::try_from(headers_val) {
            let props = headers_obj
                .get_property_names(scope, Default::default())
                .unwrap_or(v8::Array::new(scope, 0));
            for i in 0..props.length() {
                if let Some(key_val) = props.get_index(scope, i) {
                    if let Some(key_str) = key_val.to_string(scope) {
                        if key_str.to_rust_string_lossy(scope).to_lowercase() == name {
                            found = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    retval.set(v8::Boolean::new(scope, found).into());
}

/// response.write() 回调 - 支持字符串和 Uint8Array/Buffer 流式数据追加
pub fn http_res_write_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let data = args.get(0);

    if !data.is_undefined() && !data.is_null() {
        let body_key = v8::String::new(scope, "_body").unwrap();
        let existing_body = this
            .get(scope, body_key.into())
            .unwrap_or(v8::undefined(scope).into());

        let existing_rust = if existing_body.is_string() {
            existing_body
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default()
        } else {
            String::new()
        };

        let chunk_rust = if data.is_string() {
            data.to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default()
        } else if data.is_array_buffer_view() || data.is_uint8_array() {
            if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(data) {
                let mut buffer = vec![0u8; view.byte_length()];
                let _ = view.copy_contents(buffer.as_mut_slice());
                String::from_utf8_lossy(&buffer).to_string()
            } else {
                String::new()
            }
        } else {
            data.to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default()
        };

        let combined_rust = format!("{}{}", existing_rust, chunk_rust);
        let combined_v8 = v8::String::new(scope, &combined_rust).unwrap();
        this.set(scope, body_key.into(), combined_v8.into());
    }

    retval.set(v8::Boolean::new(scope, true).into());
}

/// response.writeHead() 回调 - 支持二参数 status/headers 与三参数重载
pub fn http_res_write_head_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let status_code: i32 = args
        .get(0)
        .to_integer(scope)
        .unwrap_or(v8::Integer::new(scope, 200))
        .value() as i32;

    let arg1 = args.get(1);
    let (status_message, headers) = if arg1.is_string() {
        let msg = arg1
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "OK".to_string());
        (msg, args.get(2))
    } else if arg1.is_object() {
        ("OK".to_string(), arg1)
    } else {
        ("OK".to_string(), args.get(2))
    };

    let status_code_val = v8::Integer::new(scope, status_code);
    let status_msg_val = v8::String::new(scope, &status_message).unwrap();

    let status_code_key: _ = v8::String::new(scope, "statusCode").unwrap();
    this.set(scope, status_code_key.into(), status_code_val.into());

    let status_msg_key: _ = v8::String::new(scope, "statusMessage").unwrap();
    this.set(scope, status_msg_key.into(), status_msg_val.into());

    if !headers.is_undefined() && headers.is_object() {
        let headers_key: _ = v8::String::new(scope, "headers").unwrap();
        let existing_headers = if let Some(h) = this.get(scope, headers_key.into()) {
            if let Ok(obj) = v8::Local::<v8::Object>::try_from(h) {
                obj
            } else {
                v8::Object::new(scope)
            }
        } else {
            v8::Object::new(scope)
        };

        if let Ok(new_headers_obj) = v8::Local::<v8::Object>::try_from(headers) {
            let props = new_headers_obj
                .get_property_names(scope, Default::default())
                .unwrap_or(v8::Array::new(scope, 0));
            for i in 0..props.length() {
                if let Some(k) = props.get_index(scope, i) {
                    if let Some(v) = new_headers_obj.get(scope, k) {
                        existing_headers.set(scope, k, v);
                    }
                }
            }
        }
        this.set(scope, headers_key.into(), existing_headers.into());
    }

    let headers_sent_key = v8::String::new(scope, "headersSent").unwrap();
    let headers_sent_val = v8::Boolean::new(scope, true);
    this.set(scope, headers_sent_key.into(), headers_sent_val.into());

    retval.set(this.into());
}
/// response.end() 回调 - v0.3.64
pub fn extract_http_body_string(scope: &mut v8::PinScope, data: v8::Local<v8::Value>) -> String {
    if data.is_string() {
        return data
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default();
    }
    if data.is_array_buffer_view() || data.is_uint8_array() {
        if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(data) {
            let mut buffer = vec![0u8; view.byte_length()];
            let _ = view.copy_contents(buffer.as_mut_slice());
            return String::from_utf8_lossy(&buffer).to_string();
        }
    }
    if data.is_array_buffer() {
        if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(data) {
            let bs = ab.get_backing_store();
            let ptr = bs
                .data()
                .map(|p| p.as_ptr() as *const u8)
                .unwrap_or(std::ptr::null());
            if !ptr.is_null() {
                let slice = unsafe { std::slice::from_raw_parts(ptr, ab.byte_length()) };
                return String::from_utf8_lossy(slice).into_owned();
            }
        }
    }
    if let Ok(obj) = v8::Local::<v8::Object>::try_from(data) {
        let buf_key = v8::String::new(scope, "buffer").unwrap();
        if let Some(buf_val) = obj.get(scope, buf_key.into()) {
            if buf_val.is_array_buffer() {
                if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf_val) {
                    let len_key = v8::String::new(scope, "length").unwrap();
                    let len = obj
                        .get(scope, len_key.into())
                        .and_then(|v| v.to_integer(scope))
                        .map(|i| i.value() as usize)
                        .unwrap_or_else(|| ab.byte_length());
                    let bs = ab.get_backing_store();
                    let ptr = bs
                        .data()
                        .map(|p| p.as_ptr() as *const u8)
                        .unwrap_or(std::ptr::null());
                    if !ptr.is_null() {
                        let actual_len = len.min(ab.byte_length());
                        let slice = unsafe { std::slice::from_raw_parts(ptr, actual_len) };
                        return String::from_utf8_lossy(slice).into_owned();
                    }
                }
            }
        }
    }
    data.to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

pub fn http_res_end_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let data: _ = args.get(0);

    // 处理 end() 的数据参数，存储到 _endData 避免双重字符串拷贝
    if !data.is_undefined() && !data.is_null() {
        let end_data_key = v8::String::new(scope, "_endData").unwrap();
        this.set(scope, end_data_key.into(), data);

        let body_key: _ = v8::String::new(scope, "_body").unwrap();
        // 获取现有 body (仅在先前有 write 时合并)
        let existing_body = this
            .get(scope, body_key.into())
            .unwrap_or(v8::undefined(scope).into());

        let existing_rust = if existing_body.is_string() {
            let existing_str = existing_body.to_string(scope).unwrap();
            if existing_str.length() > 0 {
                existing_str.to_rust_string_lossy(scope)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if !existing_rust.is_empty() {
            let data_rust = extract_http_body_string(scope, data);
            let combined_rust = format!("{}{}", existing_rust, data_rust);
            let combined = v8::String::new(scope, &combined_rust).unwrap();
            this.set(scope, body_key.into(), combined.into());
        }
    }

    let ended_key = v8::String::new(scope, "_ended").unwrap();
    let ended_val = v8::Boolean::new(scope, true);
    this.set(scope, ended_key.into(), ended_val.into());
    let headers_sent_key = v8::String::new(scope, "headersSent").unwrap();
    let headers_sent_val = v8::Boolean::new(scope, true);
    this.set(scope, headers_sent_key.into(), headers_sent_val.into());

    let async_key = v8::String::new(scope, "_asyncPending").unwrap();
    let async_pending = this
        .get(scope, async_key.into())
        .map(|value| value.boolean_value(scope))
        .unwrap_or(false);
    let sent_key = v8::String::new(scope, "_responseSent").unwrap();
    let already_sent = this
        .get(scope, sent_key.into())
        .map(|value| value.boolean_value(scope))
        .unwrap_or(false);
    if async_pending && !already_sent {
        let conn_key = v8::String::new(scope, "_connectionId").unwrap();
        let connection_id = this
            .get(scope, conn_key.into())
            .and_then(|value| value.to_number(scope))
            .map(|value| value.value() as u64)
            .unwrap_or(0);
        let sent_val = v8::Boolean::new(scope, true);
        this.set(scope, sent_key.into(), sent_val.into());
        send_http_response(extract_http_response_from_res(scope, this, connection_id));
        PENDING_ASYNC_HTTP_RESPONSES.fetch_sub(1, Ordering::SeqCst);
    }

    retval.set(this.into());
}

// ============================================================================
// v0.3.87: HTTP Server 真实监听和请求处理功能
// ============================================================================

/// HTTP 请求结构体
#[derive(Debug, Clone)]
pub struct HttpServerRequest {
    pub method: String,
    pub url: String,
    pub path: String,
    pub http_version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// HTTP 响应构建器
#[derive(Debug, Default)]
pub struct HttpServerResponse {
    pub status_code: u16,
    pub status_message: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpServerResponse {
    pub fn new() -> Self {
        Self {
            status_code: 200,
            status_message: "OK".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// 添加响应头
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }

    /// 移除响应头
    pub fn remove_header(&mut self, name: &str) {
        self.headers.remove(name);
    }

    /// 获取响应头
    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }

    /// 写入 body
    pub fn write(&mut self, data: &[u8]) {
        self.body.extend_from_slice(data);
    }

    /// 生成 HTTP 响应字符串
    pub fn to_string(&mut self) -> String {
        let mut response = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_message);

        // 添加 Content-Length
        self.headers
            .insert("Content-Length".to_string(), self.body.len().to_string());

        // 添加所有 headers
        for (key, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }

        response.push_str("\r\n");

        response
    }
}

/// HTTP 服务器状态管理
#[derive(Debug, Clone)]
pub struct HttpServerState {
    pub listening: Arc<AtomicBool>,
    pub port: u16,
    pub host: String,
    /// v0.3.89: 是否使用消息通道模式
    pub use_message_channel: bool,
    pub bound_port: Arc<AtomicU64>,
    pub tls_config: Option<Arc<rustls::ServerConfig>>,
    pub channel: Option<Arc<Mutex<Option<HttpServerMessageChannel>>>>,
}

impl HttpServerState {
    pub fn new() -> Self {
        Self {
            listening: Arc::new(AtomicBool::new(false)),
            port: 3000,
            host: "0.0.0.0".to_string(),
            use_message_channel: false,
            bound_port: Arc::new(AtomicU64::new(3000)),
            tls_config: None,
            channel: None,
        }
    }
}

fn intern_header_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.eq_ignore_ascii_case("host") {
        "host".to_string()
    } else if trimmed.eq_ignore_ascii_case("connection") {
        "connection".to_string()
    } else if trimmed.eq_ignore_ascii_case("user-agent") {
        "user-agent".to_string()
    } else if trimmed.eq_ignore_ascii_case("accept") {
        "accept".to_string()
    } else if trimmed.eq_ignore_ascii_case("content-type") {
        "content-type".to_string()
    } else if trimmed.eq_ignore_ascii_case("content-length") {
        "content-length".to_string()
    } else if trimmed.eq_ignore_ascii_case("accept-encoding") {
        "accept-encoding".to_string()
    } else {
        trimmed.to_ascii_lowercase()
    }
}

#[inline(always)]
fn find_crlf_crlf(data: &[u8]) -> Option<usize> {
    if data.len() < 4 {
        return None;
    }
    let len = data.len() - 3;
    let mut i = 0;
    while i < len {
        if data[i] == b'\r' && data[i + 1] == b'\n' && data[i + 2] == b'\r' && data[i + 3] == b'\n'
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// 解析 HTTP 请求
pub fn parse_http_request(data: &[u8]) -> Option<HttpServerRequest> {
    let (header_bytes, body_bytes) = if let Some(pos) = find_crlf_crlf(data) {
        (&data[..pos], &data[pos + 4..])
    } else {
        (data, &[][..])
    };

    let header_str = std::str::from_utf8(header_bytes).ok()?;

    // Ultra fast-path: Standard GET / HTTP/1.1
    if header_bytes.starts_with(b"GET / HTTP/1.1\r\n") {
        let mut headers = HashMap::with_capacity(8);
        for line in header_str[16..].split("\r\n") {
            if line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                let key = intern_header_name(k);
                let val_trimmed = v.trim();
                let value = if val_trimmed.eq_ignore_ascii_case("keep-alive") {
                    "keep-alive".to_string()
                } else if val_trimmed.eq_ignore_ascii_case("close") {
                    "close".to_string()
                } else {
                    val_trimmed.to_string()
                };
                headers.insert(key, value);
            }
        }
        return Some(HttpServerRequest {
            method: "GET".to_string(),
            url: "/".to_string(),
            path: "/".to_string(),
            http_version: "HTTP/1.1".to_string(),
            headers,
            body: body_bytes.to_vec(),
        });
    }

    let mut lines = header_str.split("\r\n");
    let request_line = lines.next()?;
    let mut request_parts = request_line.splitn(3, ' ');
    let method = match request_parts.next()? {
        "GET" => "GET".to_string(),
        "POST" => "POST".to_string(),
        "HEAD" => "HEAD".to_string(),
        "PUT" => "PUT".to_string(),
        "DELETE" => "DELETE".to_string(),
        "OPTIONS" => "OPTIONS".to_string(),
        m => m.to_string(),
    };
    let url_raw = request_parts.next()?;
    let url = if url_raw == "/" {
        "/".to_string()
    } else {
        url_raw.to_string()
    };
    let http_version = match request_parts.next()? {
        "HTTP/1.1" => "HTTP/1.1".to_string(),
        "HTTP/1.0" => "HTTP/1.0".to_string(),
        v => v.to_string(),
    };

    // 提取 path（去掉 query string）
    let path = if url == "/" {
        "/".to_string()
    } else {
        url.split('?').next().unwrap_or(&url).to_string()
    };

    // 解析 headers（直接使用 lowercase 键，避免后续遍历反复小写化）
    let mut headers = HashMap::with_capacity(8);
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = intern_header_name(k);
            let val_trimmed = v.trim();
            let value = if val_trimmed.eq_ignore_ascii_case("keep-alive") {
                "keep-alive".to_string()
            } else if val_trimmed.eq_ignore_ascii_case("close") {
                "close".to_string()
            } else {
                val_trimmed.to_string()
            };
            headers.insert(key, value);
        }
    }

    Some(HttpServerRequest {
        method,
        url,
        path,
        http_version,
        headers,
        body: body_bytes.to_vec(),
    })
}

/// 生成 HTTP 响应
pub fn generate_http_response(response: &mut HttpServerResponse) -> Vec<u8> {
    let output = response.to_string().into_bytes();
    let mut result = output;
    result.extend_from_slice(&response.body);
    result
}

fn http_reason_phrase(status_code: u16) -> &'static str {
    match status_code {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "OK",
    }
}

/// 生成 HTTP 响应（从 HttpResponseMessage）
/// 优化为单次内存预分配并直接写入 bytes，支持在头部生成阶段注入 Connection 属性避免后续切片拷贝
pub fn generate_http_response_v2(
    response: &HttpResponseMessage,
    default_connection: Option<&str>,
) -> Vec<u8> {
    let mut result = Vec::with_capacity(128 + response.headers.len() * 32 + response.body.len());

    // Ultra-fast static status line
    match response.status_code {
        200 => result.extend_from_slice(b"HTTP/1.1 200 OK\r\n"),
        201 => result.extend_from_slice(b"HTTP/1.1 201 Created\r\n"),
        204 => result.extend_from_slice(b"HTTP/1.1 204 No Content\r\n"),
        301 => result.extend_from_slice(b"HTTP/1.1 301 Moved Permanently\r\n"),
        302 => result.extend_from_slice(b"HTTP/1.1 302 Found\r\n"),
        304 => result.extend_from_slice(b"HTTP/1.1 304 Not Modified\r\n"),
        400 => result.extend_from_slice(b"HTTP/1.1 400 Bad Request\r\n"),
        401 => result.extend_from_slice(b"HTTP/1.1 401 Unauthorized\r\n"),
        403 => result.extend_from_slice(b"HTTP/1.1 403 Forbidden\r\n"),
        404 => result.extend_from_slice(b"HTTP/1.1 404 Not Found\r\n"),
        500 => result.extend_from_slice(b"HTTP/1.1 500 Internal Server Error\r\n"),
        502 => result.extend_from_slice(b"HTTP/1.1 502 Bad Gateway\r\n"),
        503 => result.extend_from_slice(b"HTTP/1.1 503 Service Unavailable\r\n"),
        code => {
            result.extend_from_slice(b"HTTP/1.1 ");
            result.extend_from_slice(code.to_string().as_bytes());
            result.extend_from_slice(b" ");
            result.extend_from_slice(http_reason_phrase(code).as_bytes());
            result.extend_from_slice(b"\r\n");
        }
    }

    let mut has_connection = false;
    for (name, value) in &response.headers {
        if name.eq_ignore_ascii_case("connection") {
            has_connection = true;
        }
        result.extend_from_slice(name.as_bytes());
        result.extend_from_slice(b": ");
        result.extend_from_slice(value.as_bytes());
        result.extend_from_slice(b"\r\n");
    }

    if !has_connection {
        if let Some(conn) = default_connection {
            result.extend_from_slice(b"Connection: ");
            result.extend_from_slice(conn.as_bytes());
            result.extend_from_slice(b"\r\n");
        }
    }

    // End of headers
    result.extend_from_slice(b"\r\n");

    // Body
    result.extend_from_slice(&response.body);

    result
}

#[allow(dead_code)]
fn create_reuse_port_listener(addr_str: &str) -> std::io::Result<TcpListener> {
    #[cfg(unix)]
    {
        use std::net::ToSocketAddrs;
        use std::os::unix::io::FromRawFd;

        let socket_addr = addr_str.to_socket_addrs()?.next().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid address")
        })?;

        let (domain, sockaddr_storage, sockaddr_len) = match socket_addr {
            std::net::SocketAddr::V4(v4) => {
                let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
                let sin = &mut storage as *mut libc::sockaddr_storage as *mut libc::sockaddr_in;
                unsafe {
                    (*sin).sin_family = libc::AF_INET as libc::sa_family_t;
                    (*sin).sin_port = v4.port().to_be();
                    (*sin).sin_addr = libc::in_addr {
                        s_addr: u32::from_ne_bytes(v4.ip().octets()),
                    };
                }
                (
                    libc::AF_INET,
                    storage,
                    std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                )
            }
            std::net::SocketAddr::V6(v6) => {
                let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
                let sin6 = &mut storage as *mut libc::sockaddr_storage as *mut libc::sockaddr_in6;
                unsafe {
                    (*sin6).sin6_family = libc::AF_INET6 as libc::sa_family_t;
                    (*sin6).sin6_port = v6.port().to_be();
                    (*sin6).sin6_addr = libc::in6_addr {
                        s6_addr: v6.ip().octets(),
                    };
                }
                (
                    libc::AF_INET6,
                    storage,
                    std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
                )
            }
        };

        let fd = unsafe { libc::socket(domain, libc::SOCK_STREAM, 0) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let val: libc::c_int = 1;
        let val_ptr = &val as *const libc::c_int as *const libc::c_void;
        let val_len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;

        unsafe {
            libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, val_ptr, val_len);
            #[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
            libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_REUSEPORT, val_ptr, val_len);

            let ptr = &sockaddr_storage as *const libc::sockaddr_storage as *const libc::sockaddr;
            if libc::bind(fd, ptr, sockaddr_len) != 0 {
                let err = std::io::Error::last_os_error();
                libc::close(fd);
                return Err(err);
            }

            if libc::listen(fd, 4096) != 0 {
                let err = std::io::Error::last_os_error();
                libc::close(fd);
                return Err(err);
            }

            Ok(TcpListener::from_raw_fd(fd))
        }
    }
    #[cfg(not(unix))]
    {
        TcpListener::bind(addr_str)
    }
}

static HTTP_TOKIO_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .max(2);
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_threads)
        .thread_name("amberjs-http-tokio")
        .enable_all()
        .build()
        .expect("Failed to create HTTP Tokio runtime")
});

pub fn get_http_tokio_runtime() -> &'static tokio::runtime::Runtime {
    &HTTP_TOKIO_RUNTIME
}

enum TokioServerIo {
    Plain(tokio::net::TcpStream),
    Tls(tokio_rustls::server::TlsStream<tokio::net::TcpStream>),
}

impl tokio::io::AsyncRead for TokioServerIo {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            TokioServerIo::Plain(s) => Pin::new(s).poll_read(cx, buf),
            TokioServerIo::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for TokioServerIo {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            TokioServerIo::Plain(s) => Pin::new(s).poll_write(cx, buf),
            TokioServerIo::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            TokioServerIo::Plain(s) => Pin::new(s).poll_flush(cx),
            TokioServerIo::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            TokioServerIo::Plain(s) => Pin::new(s).poll_shutdown(cx),
            TokioServerIo::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

/// HTTP 服务器运行函数（基于 Tokio 异步网络反应堆）
fn run_http_server(server_state: Arc<HttpServerState>, _handler_code: String) {
    let addr = format!("{}:{}", server_state.host, server_state.port);

    if !server_state.listening.load(Ordering::SeqCst) {
        return;
    }

    // 创建 TCP 监听器
    let std_listener = match TcpListener::bind(&addr) {
        Ok(l) => {
            if let Ok(local) = l.local_addr() {
                server_state
                    .bound_port
                    .store(local.port() as u64, Ordering::SeqCst);
            }
            l
        }
        Err(e) => {
            eprintln!("[Amber] Failed to bind to {}: {}", addr, e);
            server_state.listening.store(false, Ordering::SeqCst);
            return;
        }
    };

    if let Err(e) = std_listener.set_nonblocking(true) {
        eprintln!("[Amber] Failed to set non-blocking on listener: {}", e);
        server_state.listening.store(false, Ordering::SeqCst);
        return;
    }

    if !server_state.listening.load(Ordering::SeqCst) {
        return;
    }

    eprintln!("[Amber] HTTP Server listening on {}", addr);

    let rt = get_http_tokio_runtime();
    let state_clone = server_state.clone();

    rt.spawn(async move {
        let tokio_listener = match tokio::net::TcpListener::from_std(std_listener) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[Amber] Failed to convert listener to Tokio: {}", e);
                state_clone.listening.store(false, Ordering::SeqCst);
                return;
            }
        };

        while state_clone.listening.load(Ordering::SeqCst) {
            tokio::select! {
                res = tokio_listener.accept() => {
                    match res {
                        Ok((stream, _addr)) => {
                            let state = state_clone.clone();
                            tokio::spawn(async move {
                                let _ = stream.set_nodelay(true);
                                if let Some(ref tls_config) = state.tls_config {
                                    let acceptor = tokio_rustls::TlsAcceptor::from(tls_config.clone());
                                    match acceptor.accept(stream).await {
                                        Ok(tls_stream) => {
                                            handle_tokio_connection(TokioServerIo::Tls(tls_stream), state).await;
                                        }
                                        Err(e) => {
                                            eprintln!("[Amber] TLS handshake failed: {}", e);
                                        }
                                    }
                                } else {
                                    handle_tokio_connection(TokioServerIo::Plain(stream), state).await;
                                }
                            });
                        }
                        Err(e) => {
                            if !state_clone.listening.load(Ordering::SeqCst) {
                                break;
                            }
                            eprintln!("[Amber] Accept failed: {}", e);
                            tokio::time::sleep(Duration::from_millis(5)).await;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Periodic listening check
                }
            }
        }
        eprintln!("[Amber] HTTP Server stopped");
    });
}

/// 处理单个连接
/// 检查是否应该保持连接（Keep-Alive）
fn header_value_ignore_case<'a>(
    headers: &'a HashMap<String, String>,
    name: &str,
) -> Option<&'a str> {
    headers.iter().find_map(|(key, value)| {
        if key.eq_ignore_ascii_case(name) {
            Some(value.as_str())
        } else {
            None
        }
    })
}

fn is_websocket_upgrade(headers: &HashMap<String, String>) -> bool {
    header_value_ignore_case(headers, "Upgrade")
        .map(|value| value.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}

struct PrefixedStream<S> {
    prefix: std::io::Cursor<Vec<u8>>,
    inner: S,
}

impl<S: Read> Read for PrefixedStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if (self.prefix.position() as usize) < self.prefix.get_ref().len() {
            let read = std::io::Read::read(&mut self.prefix, buf)?;
            if read > 0 {
                return Ok(read);
            }
        }
        self.inner.read(buf)
    }
}

impl<S: Write> Write for PrefixedStream<S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn handle_same_port_websocket_upgrade<S: Read + Write>(stream: S, request_data: Vec<u8>) {
    let prefixed = PrefixedStream {
        prefix: std::io::Cursor::new(request_data),
        inner: stream,
    };
    match tungstenite::accept(prefixed) {
        Ok(mut websocket) => loop {
            match websocket.read() {
                Ok(message) if message.is_close() => break,
                Ok(message) if message.is_text() || message.is_binary() => {
                    if websocket.send(message).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        },
        Err(error) => eprintln!("[Amber] WebSocket upgrade failed: {error}"),
    }
}

fn should_keep_alive(headers: &HashMap<String, String>, http_version: &str) -> bool {
    let conn = header_value_ignore_case(headers, "connection").map(|s| s.to_ascii_lowercase());
    // HTTP/1.1 默认 Keep-Alive，HTTP/1.0 默认 Close
    if http_version == "HTTP/1.1" {
        match conn.as_deref() {
            Some("close") => false,
            Some("keep-alive") => true,
            _ => true,
        }
    } else {
        match conn.as_deref() {
            Some("keep-alive") => true,
            _ => false,
        }
    }
}

/// 基于 Tokio 异步任务处理 HTTP 连接（全异步非阻塞 + Keep-Alive 复用）
async fn handle_tokio_connection(mut stream: TokioServerIo, _server_state: Arc<HttpServerState>) {
    const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(30);

    let mut buffer = [0u8; 8192];
    let mut request_data = Vec::with_capacity(4096);

    loop {
        request_data.clear();
        let mut connection_close = false;

        // 等待下一个请求的首包（活跃 keep-alive 连接尝试 try_read 零定时器快路径）
        let first_read = match stream {
            TokioServerIo::Plain(ref mut s) => match s.try_read(&mut buffer) {
                Ok(0) => Ok(Ok(0)),
                Ok(n) => Ok(Ok(n)),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::timeout(KEEP_ALIVE_TIMEOUT, s.read(&mut buffer)).await
                }
                Err(e) => Ok(Err(e)),
            },
            TokioServerIo::Tls(ref mut s) => {
                tokio::time::timeout(KEEP_ALIVE_TIMEOUT, s.read(&mut buffer)).await
            }
        };
        match first_read {
            Ok(Ok(0)) | Ok(Err(_)) | Err(_) => {
                connection_close = true;
            }
            Ok(Ok(n)) => {
                request_data.extend_from_slice(&buffer[..n]);
            }
        }

        if connection_close || request_data.is_empty() {
            break;
        }

        // 读取剩余请求头（活跃数据传输中直接非阻塞读取，消除重复定时器分配）
        while find_crlf_crlf(&request_data).is_none() {
            match stream.read(&mut buffer).await {
                Ok(0) | Err(_) => {
                    connection_close = true;
                    break;
                }
                Ok(n) => {
                    request_data.extend_from_slice(&buffer[..n]);
                    if request_data.len() > 1024 * 1024 {
                        connection_close = true;
                        break;
                    }
                }
            }
        }

        if connection_close || request_data.is_empty() {
            break;
        }

        // 解析 HTTP 请求
        let parsed_request = match parse_http_request(&request_data) {
            Some(req) => req,
            None => break,
        };

        if is_websocket_upgrade(&parsed_request.headers) {
            if let TokioServerIo::Plain(plain_stream) = stream {
                if let Ok(std_stream) = plain_stream.into_std() {
                    let _ = std_stream.set_nonblocking(false);
                    handle_same_port_websocket_upgrade(std_stream, request_data);
                }
            }
            return;
        }

        // 判断是否 Keep-Alive
        let is_keep_alive = !connection_close
            && should_keep_alive(&parsed_request.headers, &parsed_request.http_version);

        let connection_id = allocate_http_connection_id();
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<HttpResponseMessage>();
        let request_msg = HttpRequestMessage {
            method: parsed_request.method,
            url: parsed_request.url,
            path: parsed_request.path,
            http_version: parsed_request.http_version,
            headers: parsed_request.headers,
            body: parsed_request.body,
            connection_id,
            responder: Some(resp_tx),
        };

        let mut message_channel_used = false;
        if let Ok(tx_guard) = GLOBAL_REQUEST_SENDER.read() {
            if let Some(ref tx) = *tx_guard {
                if tx.send(request_msg).is_ok() {
                    message_channel_used = true;
                    wake_http_dispatch_thread();
                }
            }
        }

        if !message_channel_used {
            let fallback_body = "Amber HTTP server dispatcher unavailable";
            let response_data = format!(
                "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                fallback_body.len(),
                fallback_body
            );
            let _ = stream.write_all(response_data.as_bytes()).await;
            let _ = stream.shutdown().await;
            break;
        }

        // 等待 V8 响应（快路径直通，无需定时器轮盘开销）
        match resp_rx.await {
            Ok(response) => {
                let response_has_close = response.headers.iter().any(|(k, v)| {
                    k.eq_ignore_ascii_case("connection") && v.trim().eq_ignore_ascii_case("close")
                });
                let keep_alive = is_keep_alive && !response_has_close;
                let connection_header = if keep_alive { "keep-alive" } else { "close" };

                let response_data = generate_http_response_v2(&response, Some(connection_header));
                let write_res = match stream {
                    TokioServerIo::Plain(ref mut s) => match s.try_write(&response_data) {
                        Ok(n) if n == response_data.len() => Ok(()),
                        Ok(n) => s.write_all(&response_data[n..]).await,
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            s.write_all(&response_data).await
                        }
                        Err(e) => Err(e),
                    },
                    TokioServerIo::Tls(ref mut s) => s.write_all(&response_data).await,
                };
                if write_res.is_err() {
                    break;
                }

                if !keep_alive {
                    let _ = stream.shutdown().await;
                    break;
                }
            }
            Err(_) => {
                let _ = stream.shutdown().await;
                break;
            }
        }
    }

    let _ = stream.shutdown().await;
}

/// response.removeHeader() 回调 - v0.3.87
fn http_res_remove_header_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let name: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let headers_key: _ = v8::String::new(scope, "headers").unwrap();
    let headers_obj = if let Ok(obj) = v8::Local::<v8::Object>::try_from(
        this.get(scope, headers_key.into())
            .unwrap_or(v8::undefined(scope).into()),
    ) {
        obj
    } else {
        v8::Object::new(scope)
    };

    let name_key: _ = v8::String::new(scope, &name).unwrap();
    let _undefined_val: _ = v8::undefined(scope);
    headers_obj.delete(scope, name_key.into());
    this.set(scope, headers_key.into(), headers_obj.into());

    retval.set(this.into());
}

// ============================================================================
// v1.15.0: Hyper-Zero V8 HTTP 静态原子与零拷贝直通引擎
// ============================================================================

pub struct V8HttpAtoms<'a> {
    pub method: v8::Local<'a, v8::String>,
    pub url: v8::Local<'a, v8::String>,
    pub path: v8::Local<'a, v8::String>,
    pub http_version: v8::Local<'a, v8::String>,
    pub headers: v8::Local<'a, v8::String>,
    pub raw_headers: v8::Local<'a, v8::String>,
    pub complete: v8::Local<'a, v8::String>,
    pub body: v8::Local<'a, v8::String>,
    pub raw_body: v8::Local<'a, v8::String>,
    pub data_listeners: v8::Local<'a, v8::String>,
    pub end_listeners: v8::Local<'a, v8::String>,
    pub socket: v8::Local<'a, v8::String>,
    pub connection: v8::Local<'a, v8::String>,
    pub req: v8::Local<'a, v8::String>,
    pub status_code: v8::Local<'a, v8::String>,
    pub status_message: v8::Local<'a, v8::String>,
    pub res_body: v8::Local<'a, v8::String>,
    pub ended: v8::Local<'a, v8::String>,
    pub response_sent: v8::Local<'a, v8::String>,
    pub async_pending: v8::Local<'a, v8::String>,
    pub headers_sent: v8::Local<'a, v8::String>,
    pub connection_id: v8::Local<'a, v8::String>,
    pub remote_address: v8::Local<'a, v8::String>,
    pub remote_port: v8::Local<'a, v8::String>,
    pub encrypted: v8::Local<'a, v8::String>,
    pub headers_array: v8::Local<'a, v8::String>,
    pub end_data: v8::Local<'a, v8::String>,
    pub ok_str: v8::Local<'a, v8::String>,
    pub empty_str: v8::Local<'a, v8::String>,
    pub localhost_str: v8::Local<'a, v8::String>,
    pub handler_key: v8::Local<'a, v8::String>,
    pub get_str: v8::Local<'a, v8::String>,
    pub post_str: v8::Local<'a, v8::String>,
    pub slash_str: v8::Local<'a, v8::String>,
    pub http_1_1_str: v8::Local<'a, v8::String>,
    pub host_key: v8::Local<'a, v8::String>,
    pub connection_key: v8::Local<'a, v8::String>,
    pub user_agent_key: v8::Local<'a, v8::String>,
    pub accept_key: v8::Local<'a, v8::String>,
    pub content_type_key: v8::Local<'a, v8::String>,
    pub content_length_key: v8::Local<'a, v8::String>,
    pub keep_alive_val: v8::Local<'a, v8::String>,
    pub close_val: v8::Local<'a, v8::String>,
    pub text_plain_val: v8::Local<'a, v8::String>,
    pub app_json_val: v8::Local<'a, v8::String>,
    pub dispatch_fn_key: v8::Local<'a, v8::String>,
}

impl<'a> V8HttpAtoms<'a> {
    pub fn new(scope: &mut v8::PinScope<'a, '_>) -> Self {
        Self {
            method: v8::String::new(scope, "method").unwrap(),
            url: v8::String::new(scope, "url").unwrap(),
            path: v8::String::new(scope, "path").unwrap(),
            http_version: v8::String::new(scope, "httpVersion").unwrap(),
            headers: v8::String::new(scope, "headers").unwrap(),
            raw_headers: v8::String::new(scope, "rawHeaders").unwrap(),
            complete: v8::String::new(scope, "complete").unwrap(),
            body: v8::String::new(scope, "body").unwrap(),
            raw_body: v8::String::new(scope, "_rawBody").unwrap(),
            data_listeners: v8::String::new(scope, "_dataListeners").unwrap(),
            end_listeners: v8::String::new(scope, "_endListeners").unwrap(),
            socket: v8::String::new(scope, "socket").unwrap(),
            connection: v8::String::new(scope, "connection").unwrap(),
            req: v8::String::new(scope, "req").unwrap(),
            status_code: v8::String::new(scope, "statusCode").unwrap(),
            status_message: v8::String::new(scope, "statusMessage").unwrap(),
            res_body: v8::String::new(scope, "_body").unwrap(),
            ended: v8::String::new(scope, "_ended").unwrap(),
            response_sent: v8::String::new(scope, "_responseSent").unwrap(),
            async_pending: v8::String::new(scope, "_asyncPending").unwrap(),
            headers_sent: v8::String::new(scope, "headersSent").unwrap(),
            connection_id: v8::String::new(scope, "_connectionId").unwrap(),
            remote_address: v8::String::new(scope, "remoteAddress").unwrap(),
            remote_port: v8::String::new(scope, "remotePort").unwrap(),
            encrypted: v8::String::new(scope, "encrypted").unwrap(),
            headers_array: v8::String::new(scope, "_headersArray").unwrap(),
            end_data: v8::String::new(scope, "_endData").unwrap(),
            ok_str: v8::String::new(scope, "OK").unwrap(),
            empty_str: v8::String::new(scope, "").unwrap(),
            localhost_str: v8::String::new(scope, "127.0.0.1").unwrap(),
            handler_key: v8::String::new(scope, "_httpServerRequestHandler").unwrap(),
            get_str: v8::String::new(scope, "GET").unwrap(),
            post_str: v8::String::new(scope, "POST").unwrap(),
            slash_str: v8::String::new(scope, "/").unwrap(),
            http_1_1_str: v8::String::new(scope, "HTTP/1.1").unwrap(),
            host_key: v8::String::new(scope, "host").unwrap(),
            connection_key: v8::String::new(scope, "connection").unwrap(),
            user_agent_key: v8::String::new(scope, "user-agent").unwrap(),
            accept_key: v8::String::new(scope, "accept").unwrap(),
            content_type_key: v8::String::new(scope, "content-type").unwrap(),
            content_length_key: v8::String::new(scope, "content-length").unwrap(),
            keep_alive_val: v8::String::new(scope, "keep-alive").unwrap(),
            close_val: v8::String::new(scope, "close").unwrap(),
            text_plain_val: v8::String::new(scope, "text/plain").unwrap(),
            app_json_val: v8::String::new(scope, "application/json").unwrap(),
            dispatch_fn_key: v8::String::new(scope, "__dispatchHttpRequest").unwrap(),
        }
    }
}

pub struct V8HttpPrototypes<'a> {
    pub im_proto: Option<v8::Local<'a, v8::Value>>,
    pub sr_proto: Option<v8::Local<'a, v8::Value>>,
    pub sock_proto: Option<v8::Local<'a, v8::Value>>,
    pub dispatch_fn: Option<v8::Local<'a, v8::Function>>,
}

pub fn get_http_prototypes<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    context: &v8::Local<v8::Context>,
    atoms: &V8HttpAtoms<'a>,
) -> V8HttpPrototypes<'a> {
    let global = context.global(scope);
    let http_key = v8::String::new(scope, "http").unwrap();
    let http_obj_val = global.get(scope, http_key.into());

    let dispatch_fn = global
        .get(scope, atoms.dispatch_fn_key.into())
        .and_then(|v| v8::Local::<v8::Function>::try_from(v).ok());

    if let Some(http_val) = http_obj_val {
        if let Ok(http_obj) = v8::Local::<v8::Object>::try_from(http_val) {
            let im_key = v8::String::new(scope, "IncomingMessage").unwrap();
            let sr_key = v8::String::new(scope, "ServerResponse").unwrap();
            let proto_key = v8::String::new(scope, "prototype").unwrap();
            let sock_key = atoms.socket;

            let im_p = http_obj.get(scope, im_key.into()).and_then(|c| {
                v8::Local::<v8::Function>::try_from(c)
                    .ok()?
                    .get(scope, proto_key.into())
            });
            let sr_p = http_obj.get(scope, sr_key.into()).and_then(|c| {
                v8::Local::<v8::Function>::try_from(c)
                    .ok()?
                    .get(scope, proto_key.into())
            });
            let sock_p = http_obj.get(scope, sock_key.into()).and_then(|c| {
                v8::Local::<v8::Function>::try_from(c)
                    .ok()?
                    .get(scope, proto_key.into())
            });
            return V8HttpPrototypes {
                im_proto: im_p,
                sr_proto: sr_p,
                sock_proto: sock_p,
                dispatch_fn,
            };
        }
    }
    V8HttpPrototypes {
        im_proto: None,
        sr_proto: None,
        sock_proto: None,
        dispatch_fn,
    }
}

pub fn get_global_request_handler_local<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    context: &v8::Local<v8::Context>,
    atoms: &V8HttpAtoms<'a>,
) -> Option<v8::Local<'a, v8::Function>> {
    let global = context.global(scope);
    let handler_val = global.get(scope, atoms.handler_key.into())?;
    if !handler_val.is_function() {
        return None;
    }
    v8::Local::<v8::Function>::try_from(handler_val).ok()
}

pub fn extract_http_body_bytes(scope: &mut v8::PinScope, data: v8::Local<v8::Value>) -> Vec<u8> {
    if data.is_string() {
        if let Some(s) = data.to_string(scope) {
            let len = s.utf8_length(scope);
            let mut vec = vec![0u8; len];
            s.write_utf8_v2(scope, &mut vec, v8::WriteFlags::empty(), None);
            return vec;
        }
    }
    if data.is_array_buffer_view() || data.is_typed_array() {
        if let Ok(ab_view) = v8::Local::<v8::ArrayBufferView>::try_from(data) {
            let mut vec = vec![0u8; ab_view.byte_length()];
            ab_view.copy_contents(&mut vec);
            return vec;
        }
    }
    if data.is_array_buffer() {
        if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(data) {
            let bs = ab.get_backing_store();
            if let Some(ptr) = bs.data() {
                let slice = unsafe {
                    std::slice::from_raw_parts(ptr.as_ptr() as *const u8, ab.byte_length())
                };
                return slice.to_vec();
            }
        }
    }
    if data.is_object() {
        if let Ok(obj) = v8::Local::<v8::Object>::try_from(data) {
            let buf_key = v8::String::new(scope, "_buffer").unwrap();
            if let Some(buf_val) = obj.get(scope, buf_key.into()) {
                if buf_val.is_array_buffer() {
                    if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf_val) {
                        let len_key = v8::String::new(scope, "length").unwrap();
                        let len = obj
                            .get(scope, len_key.into())
                            .and_then(|v| v.to_integer(scope))
                            .map(|i| i.value() as usize)
                            .unwrap_or_else(|| ab.byte_length());
                        let bs = ab.get_backing_store();
                        if let Some(ptr) = bs.data() {
                            let actual_len = len.min(ab.byte_length());
                            let slice = unsafe {
                                std::slice::from_raw_parts(ptr.as_ptr() as *const u8, actual_len)
                            };
                            return slice.to_vec();
                        }
                    }
                }
            }
        }
    }
    data.to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope).into_bytes())
        .unwrap_or_default()
}

pub fn extract_http_response_from_res_fast<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    res_obj: v8::Local<v8::Object>,
    connection_id: u64,
    atoms: &V8HttpAtoms<'a>,
) -> HttpResponseMessage {
    let status_code = res_obj
        .get(scope, atoms.status_code.into())
        .and_then(|v| v.to_int32(scope))
        .map(|i| i.value() as u16)
        .unwrap_or(200);

    // Fast-path body: check _endData first (avoiding string roundtrip)
    let body_bytes = if let Some(end_data_val) = res_obj.get(scope, atoms.end_data.into()) {
        if !end_data_val.is_undefined() && !end_data_val.is_null() {
            extract_http_body_bytes(scope, end_data_val)
        } else {
            let body_val = res_obj
                .get(scope, atoms.res_body.into())
                .unwrap_or_else(|| atoms.empty_str.into());
            body_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope).into_bytes())
                .unwrap_or_default()
        }
    } else {
        let body_val = res_obj
            .get(scope, atoms.res_body.into())
            .unwrap_or_else(|| atoms.empty_str.into());
        body_val
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope).into_bytes())
            .unwrap_or_default()
    };

    // Fast-path headers: check _headersArray [k1, v1, k2, v2, ...] first
    let mut response_headers = HashMap::with_capacity(8);
    let mut headers_parsed = false;
    if let Some(arr_val) = res_obj.get(scope, atoms.headers_array.into()) {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(arr_val) {
            let len = arr.length();
            if len > 0 {
                let mut i = 0;
                while i + 1 < len {
                    if let (Some(k_val), Some(v_val)) =
                        (arr.get_index(scope, i), arr.get_index(scope, i + 1))
                    {
                        if let (Some(k_str), Some(v_str)) =
                            (k_val.to_string(scope), v_val.to_string(scope))
                        {
                            response_headers.insert(
                                k_str.to_rust_string_lossy(scope),
                                v_str.to_rust_string_lossy(scope),
                            );
                        }
                    }
                    i += 2;
                }
                headers_parsed = true;
            }
        }
    }

    // Fallback headers reflection if _headersArray was empty or not populated
    if !headers_parsed {
        if let Some(headers_val) = res_obj.get(scope, atoms.headers.into()) {
            if let Ok(headers_obj) = v8::Local::<v8::Object>::try_from(headers_val) {
                let props = headers_obj
                    .get_property_names(scope, Default::default())
                    .unwrap_or_else(|| v8::Array::new(scope, 0));
                for i in 0..props.length() {
                    if let Some(key_val) = props.get_index(scope, i) {
                        if let Some(key_str) = key_val.to_string(scope) {
                            let key = key_str.to_rust_string_lossy(scope);
                            if let Some(value_val) = headers_obj.get(scope, key_val) {
                                if let Some(value_str) = value_val.to_string(scope) {
                                    response_headers
                                        .insert(key, value_str.to_rust_string_lossy(scope));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !response_headers.contains_key("Content-Type") {
        response_headers.insert(
            "Content-Type".to_string(),
            "text/plain; charset=utf-8".to_string(),
        );
    }
    if !response_headers.contains_key("Content-Length") {
        response_headers.insert("Content-Length".to_string(), body_bytes.len().to_string());
    }

    HttpResponseMessage {
        connection_id,
        status_code,
        headers: response_headers,
        body: body_bytes,
    }
}

pub fn extract_http_response_from_res(
    scope: &mut v8::PinScope,
    res_obj: v8::Local<v8::Object>,
    connection_id: u64,
) -> HttpResponseMessage {
    let atoms = V8HttpAtoms::new(scope);
    extract_http_response_from_res_fast(scope, res_obj, connection_id, &atoms)
}

#[allow(dead_code)]
fn emit_incoming_request_body_events<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    req_obj: v8::Local<v8::Object>,
    body_text: &str,
    atoms: &V8HttpAtoms<'a>,
) {
    if !body_text.is_empty() {
        if let Some(list_val) = req_obj.get(scope, atoms.data_listeners.into()) {
            if let Ok(list) = v8::Local::<v8::Array>::try_from(list_val) {
                if list.length() > 0 {
                    let chunk = v8::String::new(scope, body_text).unwrap();
                    for i in 0..list.length() {
                        if let Some(listener) = list.get_index(scope, i) {
                            if let Ok(func) = v8::Local::<v8::Function>::try_from(listener) {
                                let _ = func.call(scope, req_obj.into(), &[chunk.into()]);
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(list_val) = req_obj.get(scope, atoms.end_listeners.into()) {
        if let Ok(list) = v8::Local::<v8::Array>::try_from(list_val) {
            if list.length() > 0 {
                for i in 0..list.length() {
                    if let Some(listener) = list.get_index(scope, i) {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(listener) {
                            let _ = func.call(scope, req_obj.into(), &[]);
                        }
                    }
                }
            }
        }
    }
}

pub fn process_http_request_in_v8_inner<'a>(
    request: &HttpRequestMessage,
    scope: &mut v8::PinScope<'a, '_>,
    _context: &v8::Local<v8::Context>,
    request_handler: Option<v8::Local<'a, v8::Function>>,
    atoms: &V8HttpAtoms<'a>,
    protos: &V8HttpPrototypes<'a>,
) -> HttpDispatchResult {
    let method_val = if request.method == "GET" {
        atoms.get_str
    } else if request.method == "POST" {
        atoms.post_str
    } else {
        v8::String::new(scope, &request.method).unwrap()
    };

    let url_val = if request.url == "/" {
        atoms.slash_str
    } else {
        v8::String::new(scope, &request.url).unwrap()
    };

    let path_val = if request.path == "/" {
        atoms.slash_str
    } else {
        v8::String::new(scope, &request.path).unwrap()
    };

    let headers_obj = v8::Object::new(scope);
    for (name, value) in &request.headers {
        let name_key = if name == "host" {
            atoms.host_key
        } else if name == "connection" {
            atoms.connection_key
        } else if name == "user-agent" {
            atoms.user_agent_key
        } else if name == "accept" {
            atoms.accept_key
        } else if name == "content-type" {
            atoms.content_type_key
        } else if name == "content-length" {
            atoms.content_length_key
        } else {
            v8::String::new(scope, name).unwrap()
        };
        let value_val = if value == "keep-alive" {
            atoms.keep_alive_val
        } else if value == "close" {
            atoms.close_val
        } else {
            v8::String::new(scope, value).unwrap()
        };
        headers_obj.set(scope, name_key.into(), value_val.into());
    }

    let body_val = if request.body.is_empty() {
        atoms.empty_str
    } else {
        let body_text = String::from_utf8_lossy(&request.body);
        v8::String::new(scope, &body_text).unwrap()
    };

    let conn_val = v8::Number::new(scope, request.connection_id as f64);

    // Fast-path: __dispatchHttpRequest in JavaScript (monomorphic JIT hidden classes)
    if let Some(dispatch_fn) = protos.dispatch_fn {
        let undefined = v8::undefined(scope).into();
        let args = [
            method_val.into(),
            url_val.into(),
            path_val.into(),
            atoms.http_1_1_str.into(),
            headers_obj.into(),
            body_val.into(),
            conn_val.into(),
        ];

        let call_res = {
            v8::tc_scope!(let tc_scope, scope);
            let r = dispatch_fn.call(tc_scope, undefined, &args);
            if r.is_none() && tc_scope.has_caught() {
                if let Some(exc) = tc_scope.exception() {
                    let msg = exc.to_rust_string_lossy(tc_scope);
                    eprintln!("[Amber HTTP Dispatch Error] {}", msg);
                }
            }
            r
        };

        if let Some(res_val) = call_res {
            if res_val.is_null() {
                return HttpDispatchResult::NoHandler;
            }
            if let Ok(res_obj) = v8::Local::<v8::Object>::try_from(res_val) {
                let ended = res_obj
                    .get(scope, atoms.ended.into())
                    .map(|value| value.boolean_value(scope))
                    .unwrap_or(false);
                if !ended {
                    let true_val = v8::Boolean::new(scope, true);
                    res_obj.set(scope, atoms.async_pending.into(), true_val.into());
                    PENDING_ASYNC_HTTP_RESPONSES.fetch_add(1, Ordering::SeqCst);
                    return HttpDispatchResult::Pending;
                }
                return HttpDispatchResult::Response(extract_http_response_from_res_fast(
                    scope,
                    res_obj,
                    request.connection_id,
                    atoms,
                ));
            }
        }
        return HttpDispatchResult::Error;
    }

    // Fallback path: manual dispatch with handler_fn
    let Some(handler_fn) = request_handler else {
        return HttpDispatchResult::NoHandler;
    };

    let req_obj = v8::Object::new(scope);
    if let Some(p) = protos.im_proto {
        req_obj.set_prototype(scope, p);
    }
    req_obj.set(scope, atoms.method.into(), method_val.into());
    req_obj.set(scope, atoms.url.into(), url_val.into());
    req_obj.set(scope, atoms.path.into(), path_val.into());
    req_obj.set(scope, atoms.http_version.into(), atoms.http_1_1_str.into());
    req_obj.set(scope, atoms.headers.into(), headers_obj.into());

    let complete_val = v8::Boolean::new(scope, true);
    req_obj.set(scope, atoms.complete.into(), complete_val.into());
    req_obj.set(scope, atoms.body.into(), body_val.into());
    req_obj.set(scope, atoms.raw_body.into(), body_val.into());

    let res_obj = v8::Object::new(scope);
    if let Some(p) = protos.sr_proto {
        res_obj.set_prototype(scope, p);
    }
    res_obj.set(scope, atoms.req.into(), req_obj.into());

    let res_headers_obj = v8::Object::new(scope);
    res_obj.set(scope, atoms.headers.into(), res_headers_obj.into());
    let res_headers_arr = v8::Array::new(scope, 0);
    res_obj.set(scope, atoms.headers_array.into(), res_headers_arr.into());

    let status_code_val = v8::Integer::new(scope, 200);
    res_obj.set(scope, atoms.status_code.into(), status_code_val.into());
    res_obj.set(scope, atoms.status_message.into(), atoms.ok_str.into());
    res_obj.set(scope, atoms.res_body.into(), atoms.empty_str.into());

    let false_val = v8::Boolean::new(scope, false);
    res_obj.set(scope, atoms.ended.into(), false_val.into());
    res_obj.set(scope, atoms.response_sent.into(), false_val.into());
    res_obj.set(scope, atoms.async_pending.into(), false_val.into());
    res_obj.set(scope, atoms.headers_sent.into(), false_val.into());
    res_obj.set(scope, atoms.connection_id.into(), conn_val.into());

    let this_val = v8::undefined(scope).into();
    let args = [req_obj.into(), res_obj.into()];

    let call_res = {
        v8::tc_scope!(let tc_scope, scope);
        let r = handler_fn.call(tc_scope, this_val, &args);
        if r.is_none() && tc_scope.has_caught() {
            if let Some(exc) = tc_scope.exception() {
                let msg = exc.to_rust_string_lossy(tc_scope);
                eprintln!("[Amber HTTP Handler Error] {}", msg);
            }
        }
        r
    };
    if call_res.is_none() {
        return HttpDispatchResult::Error;
    }

    let ended = res_obj
        .get(scope, atoms.ended.into())
        .map(|value| value.boolean_value(scope))
        .unwrap_or(false);
    if !ended {
        let true_val = v8::Boolean::new(scope, true);
        res_obj.set(scope, atoms.async_pending.into(), true_val.into());
        PENDING_ASYNC_HTTP_RESPONSES.fetch_add(1, Ordering::SeqCst);
        return HttpDispatchResult::Pending;
    }

    HttpDispatchResult::Response(extract_http_response_from_res_fast(
        scope,
        res_obj,
        request.connection_id,
        atoms,
    ))
}

pub fn process_http_request_in_v8(
    request: &HttpRequestMessage,
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
    request_handler: Option<v8::Global<v8::Function>>,
) -> HttpDispatchResult {
    let atoms = V8HttpAtoms::new(scope);
    let protos = get_http_prototypes(scope, context, &atoms);
    let handler_local = request_handler.as_ref().map(|h| v8::Local::new(scope, h));
    process_http_request_in_v8_inner(request, scope, context, handler_local, &atoms, &protos)
}

/// 在 V8 上下文中处理 HTTP 请求（获取 response 对象）
/// 用于 event_loop.rs 中轮询消息队列
/// v0.3.91: 新增功能
///
/// 返回响应的 body 字符串和状态码
pub fn handle_http_request_v8(
    request: &HttpRequestMessage,
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> Option<(u16, Vec<u8>)> {
    // 获取全局 request handler
    let handler = get_global_request_handler(scope, context)?;

    // 处理请求
    match process_http_request_in_v8(request, scope, context, Some(handler)) {
        HttpDispatchResult::Response(response) => Some((response.status_code, response.body)),
        _ => None,
    }
}

/// 获取全局 request handler
/// v0.3.91: 新增功能
pub fn get_global_request_handler(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> Option<v8::Global<v8::Function>> {
    let global = context.global(scope);

    // 查找 _httpServerRequestHandler
    let handler_key = v8::String::new(scope, "_httpServerRequestHandler").unwrap();
    let handler_val = global.get(scope, handler_key.into());

    let handler_val = match handler_val {
        Some(v) => v,
        None => {
            return None;
        }
    };

    if handler_val.is_undefined() {
        return None;
    }

    if !handler_val.is_function() {
        return None;
    }

    let handler_fn = v8::Local::<v8::Function>::try_from(handler_val).ok()?;
    Some(v8::Global::new(scope, handler_fn))
}

/// 设置全局 request handler（供 JS 代码使用）
/// v0.3.91: 新增功能
pub fn set_global_request_handler(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
    handler: v8::Local<v8::Function>,
) {
    let global = context.global(scope);
    let handler_key = v8::String::new(scope, "_httpServerRequestHandler").unwrap();
    global.set(scope, handler_key.into(), handler.into());
}

// ============================================================================
// v0.3.98: HTTPS Server Support (TLS/SSL)
// ============================================================================

use std::fs::File;
use std::io::BufReader;

/// HTTPS/TLS 配置
/// v0.3.98: 新增结构体
#[derive(Debug, Clone)]
pub struct HttpsServerConfig {
    /// TLS 证书文件路径
    pub cert_path: String,
    /// TLS 私钥文件路径
    pub key_path: String,
    /// 服务器端口
    pub port: u16,
    /// 服务器主机
    pub host: String,
    /// 是否验证客户端证书
    pub verify_client: bool,
    /// ALPN 协议列表
    pub alpn_protocols: Vec<Vec<u8>>,
}

impl Default for HttpsServerConfig {
    fn default() -> Self {
        Self {
            cert_path: String::new(),
            key_path: String::new(),
            port: 443,
            host: "0.0.0.0".to_string(),
            verify_client: false,
            alpn_protocols: vec![b"h2".to_vec(), b"http/1.1".to_vec()],
        }
    }
}

/// TLS 证书加载结果
/// v0.3.98: 新增
#[derive(Debug)]
pub struct TlsCertificate {
    /// 证书链
    pub cert_chain: Vec<rustls::Certificate>,
    /// 私钥
    pub private_key: rustls::PrivateKey,
}

/// 加载 TLS 证书和私钥
/// v0.3.98: 新增功能
///
/// # 参数
/// - `cert_path`: 证书文件路径 (PEM 格式)
/// - `key_path`: 私钥文件路径 (PEM 格式)
///
/// # 返回
/// - `Ok(TlsCertificate)` 加载成功
/// - `Err(String)` 加载失败
pub fn load_tls_certificate(cert_path: &str, key_path: &str) -> Result<TlsCertificate, String> {
    // 加载证书文件
    let cert_file =
        File::open(cert_path).map_err(|e| format!("Failed to open certificate file: {}", e))?;
    let mut cert_reader = BufReader::new(cert_file);

    // 使用 rustls-pemfile 解析证书
    let certs_result = rustls_pemfile::certs(&mut cert_reader);
    let mut certs = Vec::new();
    for cert in certs_result {
        let cert_der = cert.map_err(|e| format!("Failed to parse certificate: {}", e))?;
        certs.push(rustls::Certificate(cert_der.to_vec()));
    }

    if certs.is_empty() {
        return Err("No certificates found in file".to_string());
    }

    // 加载私钥文件
    let key_file = File::open(key_path).map_err(|e| format!("Failed to open key file: {}", e))?;
    let mut key_reader = BufReader::new(key_file);

    // 首先尝试解析 RSA 私钥
    let keys_result = rustls_pemfile::rsa_private_keys(&mut key_reader);
    let mut keys = Vec::new();
    for key in keys_result {
        let key_bytes = key.map_err(|e| format!("Failed to parse private key: {}", e))?;
        keys.push(rustls::PrivateKey(key_bytes.secret_pkcs1_der().to_vec()));
    }

    // 如果没有 RSA 密钥，尝试 PKCS8 格式
    if keys.is_empty() {
        drop(key_reader);
        let key_file =
            File::open(key_path).map_err(|e| format!("Failed to reopen key file: {}", e))?;
        let mut key_reader = BufReader::new(key_file);

        let pkcs8_result = rustls_pemfile::pkcs8_private_keys(&mut key_reader);
        for key in pkcs8_result {
            let key_bytes = key.map_err(|e| format!("Failed to parse PKCS8 private key: {}", e))?;
            keys.push(rustls::PrivateKey(key_bytes.secret_pkcs8_der().to_vec()));
        }
    }

    if keys.is_empty() {
        return Err("No private key found in file".to_string());
    }

    Ok(TlsCertificate {
        cert_chain: certs,
        private_key: keys.remove(0),
    })
}

fn pem_or_path_bytes(value: &str) -> Result<Vec<u8>, String> {
    let trimmed = value.trim();
    if trimmed.contains("-----BEGIN") {
        return Ok(trimmed.as_bytes().to_vec());
    }
    std::fs::read(trimmed)
        .map_err(|error| format!("Failed to read TLS material {trimmed}: {error}"))
}

/// Load TLS material from PEM text or filesystem paths.
pub fn load_tls_material(cert: &str, key: &str) -> Result<TlsCertificate, String> {
    let cert_bytes = pem_or_path_bytes(cert)?;
    let key_bytes = pem_or_path_bytes(key)?;
    load_tls_certificate_from_pem(&cert_bytes, &key_bytes)
}

pub fn load_tls_certificate_from_pem(
    cert_pem: &[u8],
    key_pem: &[u8],
) -> Result<TlsCertificate, String> {
    let mut cert_reader = BufReader::new(std::io::Cursor::new(cert_pem));
    let certs_result = rustls_pemfile::certs(&mut cert_reader);
    let mut certs = Vec::new();
    for cert in certs_result {
        let cert_der = cert.map_err(|e| format!("Failed to parse certificate: {}", e))?;
        certs.push(rustls::Certificate(cert_der.to_vec()));
    }
    if certs.is_empty() {
        return Err("No certificates found in PEM".to_string());
    }

    let mut key_reader = BufReader::new(std::io::Cursor::new(key_pem));
    let mut keys = Vec::new();
    let keys_result = rustls_pemfile::rsa_private_keys(&mut key_reader);
    for key in keys_result {
        let key_bytes = key.map_err(|e| format!("Failed to parse private key: {}", e))?;
        keys.push(rustls::PrivateKey(key_bytes.secret_pkcs1_der().to_vec()));
    }
    if keys.is_empty() {
        let mut key_reader = BufReader::new(std::io::Cursor::new(key_pem));
        let pkcs8_result = rustls_pemfile::pkcs8_private_keys(&mut key_reader);
        for key in pkcs8_result {
            let key_bytes = key.map_err(|e| format!("Failed to parse PKCS8 private key: {}", e))?;
            keys.push(rustls::PrivateKey(key_bytes.secret_pkcs8_der().to_vec()));
        }
    }
    if keys.is_empty() {
        return Err("No private key found in PEM".to_string());
    }

    Ok(TlsCertificate {
        cert_chain: certs,
        private_key: keys.remove(0),
    })
}

/// 创建 TLS 服务器配置
/// v0.3.98: 新增功能
///
/// # 参数
/// - `cert`: TLS 证书
/// - `config`: HTTPS 服务器配置
///
/// # 返回
/// - `Arc<rustls::ServerConfig>` TLS 服务器配置
pub fn create_tls_server_config(
    cert: &TlsCertificate,
    config: &HttpsServerConfig,
) -> Arc<rustls::ServerConfig> {
    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain.clone(), cert.private_key.clone())
        .expect("Failed to create TLS config");

    // 设置 ALPN 协议
    tls_config.alpn_protocols = config.alpn_protocols.clone();

    Arc::new(tls_config)
}

/// rustls server config for `amber serve --https` (HTTP/1.1 only).
pub fn try_create_tls_server_config_http11(
    cert: &TlsCertificate,
) -> Result<Arc<rustls::ServerConfig>, String> {
    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain.clone(), cert.private_key.clone())
        .map_err(|e| format!("Failed to create TLS config: {e}"))?;
    tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(tls_config))
}

/// HTTPS 服务器状态
/// v0.3.98: 新增
#[derive(Debug, Clone)]
pub struct HttpsServerState {
    pub listening: Arc<AtomicBool>,
    pub port: u16,
    pub host: String,
    pub tls_config: Option<Arc<rustls::ServerConfig>>,
}

impl HttpsServerState {
    pub fn new() -> Self {
        Self {
            listening: Arc::new(AtomicBool::new(false)),
            port: 443,
            host: "0.0.0.0".to_string(),
            tls_config: None,
        }
    }

    /// 检查是否已配置 TLS
    pub fn is_tls_configured(&self) -> bool {
        self.tls_config.is_some()
    }
}

/// 解析 HTTPS URL
/// v0.3.98: 新增功能
///
/// HTTPS 请求解析与 HTTP 相同，只是传输层使用 TLS
pub fn parse_https_request(data: &[u8]) -> Option<HttpServerRequest> {
    // HTTPS 使用与 HTTP 相同的请求解析逻辑
    parse_http_request(data)
}

/// 生成 HTTPS 响应
/// v0.3.98: 新增功能
///
/// HTTPS 响应与 HTTP 响应格式相同
pub fn generate_https_response(response: &mut HttpServerResponse) -> Vec<u8> {
    generate_http_response(response)
}

// ============================================================================
// V8 API 集成 - HTTPS 服务器
// ============================================================================

/// 创建 HTTPS 服务器配置的 JavaScript API
/// v0.3.98: 新增功能
pub fn create_https_config_js<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    cert_path: String,
    key_path: String,
    port: u16,
) -> Option<v8::Local<'a, v8::Object>> {
    let config_obj = v8::Object::new(scope);

    // certPath
    let cert_path_key = v8::String::new(scope, "certPath").unwrap();
    let cert_path_val = v8::String::new(scope, &cert_path).unwrap();
    config_obj.set(scope, cert_path_key.into(), cert_path_val.into());

    // keyPath
    let key_path_key = v8::String::new(scope, "keyPath").unwrap();
    let key_path_val = v8::String::new(scope, &key_path).unwrap();
    config_obj.set(scope, key_path_key.into(), key_path_val.into());

    // port
    let port_key = v8::String::new(scope, "port").unwrap();
    let port_val = v8::Number::new(scope, port as f64);
    config_obj.set(scope, port_key.into(), port_val.into());

    Some(config_obj)
}

/// 加载 TLS 证书的 JavaScript API
/// v0.3.98: 新增功能
pub fn load_tls_certificate_js<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    args: v8::FunctionCallbackArguments<'a>,
) -> Option<v8::Local<'a, v8::Object>> {
    let cert_path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let key_path: String = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    // 尝试加载证书
    match load_tls_certificate(&cert_path, &key_path) {
        Ok(_) => {
            let result_obj = v8::Object::new(scope);
            let success_key = v8::String::new(scope, "success").unwrap();
            let success_val = v8::Boolean::new(scope, true);
            result_obj.set(scope, success_key.into(), success_val.into());
            Some(result_obj)
        }
        Err(e) => {
            let result_obj = v8::Object::new(scope);
            let success_key = v8::String::new(scope, "success").unwrap();
            let success_val = v8::Boolean::new(scope, false);
            result_obj.set(scope, success_key.into(), success_val.into());
            let error_key = v8::String::new(scope, "error").unwrap();
            let error_val = v8::String::new(scope, &e).unwrap();
            result_obj.set(scope, error_key.into(), error_val.into());
            Some(result_obj)
        }
    }
}
