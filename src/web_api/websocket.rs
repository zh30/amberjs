// WebSocket API implementation for Web standard
// Provides real WebSocket client with network connectivity

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::{CloseFrame, Message},
};

// v0.3.334: Import error_event for ErrorEvent integration
use crate::web_api::error_event::create_error_event_object;

/// WebSocket ready state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadyState {
    Connecting = 0,
    Open = 1,
    Closing = 2,
    Closed = 3,
}
/// WebSocket event type
#[derive(Debug, Clone)]
pub enum WebSocketEvent {
    Open,
    Message(String),
    Binary(Vec<u8>),
    Close(Option<u16>, Option<String>),
    Error(String),
}
/// Command sent to WebSocket connection
#[derive(Debug)]
pub enum WebSocketCommand {
    Send(String),
    SendBinary(Vec<u8>),
    Close(Option<u16>, Option<String>),
}
/// WebSocket connection handle
pub struct WebSocketConnection {
    pub id: u64,
    pub url: String,
    pub ready_state: Arc<Mutex<ReadyState>>,
    pub cmd_tx: mpsc::UnboundedSender<WebSocketCommand>,
    pub event_rx: Arc<Mutex<mpsc::UnboundedReceiver<WebSocketEvent>>>,
    thread_id: ThreadId,
    epoch: u64,
}

thread_local! {
    static EXECUTION_EPOCH: Cell<u64> = const { Cell::new(0) };
}

static MANAGER_READY: AtomicBool = AtomicBool::new(false);

fn current_epoch() -> u64 {
    EXECUTION_EPOCH.with(Cell::get)
}

fn connection_is_current(conn: &WebSocketConnection) -> bool {
    conn.thread_id == std::thread::current().id() && conn.epoch == current_epoch()
}

fn set_ready(ready_state: &Arc<Mutex<ReadyState>>, state: ReadyState) {
    *ready_state.lock().unwrap() = state;
}

fn close_parts(frame: Option<CloseFrame<'_>>) -> (Option<u16>, Option<String>) {
    match frame {
        Some(frame) => (Some(frame.code.into()), Some(frame.reason.to_string())),
        None => (Some(1005), Some(String::new())),
    }
}

fn fail_connection(
    ready_state: &Arc<Mutex<ReadyState>>,
    event_tx: &mpsc::UnboundedSender<WebSocketEvent>,
    message: String,
) {
    set_ready(ready_state, ReadyState::Closed);
    let _ = event_tx.send(WebSocketEvent::Error(message));
    let _ = event_tx.send(WebSocketEvent::Close(Some(1006), Some(String::new())));
}
/// Global WebSocket manager
pub struct WebSocketManager {
    connections: Arc<Mutex<HashMap<u64, WebSocketConnection>>>,
    next_id: AtomicU64,
    runtime: Arc<Runtime>,
}
impl WebSocketManager {
    pub fn new() -> Self {
        let runtime: _ = Runtime::new().expect("Failed to create tokio runtime");
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(1),
            runtime: Arc::new(runtime),
        }
    }
    /// Create a new WebSocket connection
    pub fn connect(&self, url: String) -> Result<u64> {
        let id: _ = self.next_id.fetch_add(1, Ordering::SeqCst);
        let ready_state: _ = Arc::new(Mutex::new(ReadyState::Connecting));
        let ready_state_clone: _ = ready_state.clone();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<WebSocketCommand>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<WebSocketEvent>();
        let url_clone: _ = url.clone();
        let thread_id = std::thread::current().id();
        let epoch = current_epoch();
        // One task owns the socket so a close frame is observed instead of
        // aborting the reader before the peer's reply arrives.
        // tokio-tungstenite 0.21 does not negotiate permessage-deflate.
        self.runtime.spawn(async move {
            let ws_stream = match connect_async(&url_clone).await {
                Ok((ws_stream, _response)) => ws_stream,
                Err(error) => {
                    fail_connection(
                        &ready_state_clone,
                        &event_tx,
                        format!("Connection failed: {error}"),
                    );
                    return;
                }
            };
            {
                let mut state = ready_state_clone.lock().unwrap();
                *state = ReadyState::Open;
            }
            let _ = event_tx.send(WebSocketEvent::Open);
            let (mut write, mut read) = ws_stream.split();
            let mut close_wait_until: Option<tokio::time::Instant> = None;
            let mut local_close: Option<(u16, String)> = None;
            loop {
                tokio::select! {
                    biased;
                    message = read.next() => {
                        match message {
                            Some(Ok(Message::Text(text))) => {
                                let _ = event_tx.send(WebSocketEvent::Message(text.to_string()));
                            }
                            Some(Ok(Message::Binary(data))) => {
                                let _ = event_tx.send(WebSocketEvent::Binary(data));
                            }
                            Some(Ok(Message::Close(frame))) => {
                                let (code, reason) = close_parts(frame);
                                set_ready(&ready_state_clone, ReadyState::Closed);
                                let _ = event_tx.send(WebSocketEvent::Close(code, reason));
                                break;
                            }
                            Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                            Some(Err(error)) => {
                                fail_connection(
                                    &ready_state_clone,
                                    &event_tx,
                                    error.to_string(),
                                );
                                break;
                            }
                            None => {
                                let (code, reason) = local_close
                                    .clone()
                                    .map(|(code, reason)| (Some(code), Some(reason)))
                                    .unwrap_or((Some(1006), Some(String::new())));
                                set_ready(&ready_state_clone, ReadyState::Closed);
                                let _ = event_tx.send(WebSocketEvent::Close(code, reason));
                                break;
                            }
                        }
                    }
                    command = cmd_rx.recv(), if close_wait_until.is_none() => {
                        match command {
                            Some(WebSocketCommand::Send(text)) => {
                                if let Err(error) = write.send(Message::Text(text)).await {
                                    fail_connection(
                                        &ready_state_clone,
                                        &event_tx,
                                        error.to_string(),
                                    );
                                    break;
                                }
                            }
                            Some(WebSocketCommand::SendBinary(data)) => {
                                if let Err(error) = write.send(Message::Binary(data)).await {
                                    fail_connection(
                                        &ready_state_clone,
                                        &event_tx,
                                        error.to_string(),
                                    );
                                    break;
                                }
                            }
                            Some(WebSocketCommand::Close(code, reason)) => {
                                let code = code.unwrap_or(1000);
                                let reason = reason.unwrap_or_default();
                                set_ready(&ready_state_clone, ReadyState::Closing);
                                let close_frame = Some(CloseFrame {
                                    code: code.into(),
                                    reason: Cow::Owned(reason.clone()),
                                });
                                if write.send(Message::Close(close_frame)).await.is_err() {
                                    set_ready(&ready_state_clone, ReadyState::Closed);
                                    let _ = event_tx.send(WebSocketEvent::Close(
                                        Some(code),
                                        Some(reason),
                                    ));
                                    break;
                                }
                                local_close = Some((code, reason));
                                close_wait_until =
                                    Some(tokio::time::Instant::now() + Duration::from_secs(2));
                            }
                            None => {
                                set_ready(&ready_state_clone, ReadyState::Closed);
                                break;
                            }
                        }
                    }
                    _ = async {
                        match close_wait_until {
                            Some(deadline) => tokio::time::sleep_until(deadline).await,
                            None => std::future::pending::<()>().await,
                        }
                    } => {
                        let (code, reason) = local_close.clone().unwrap_or((1006, String::new()));
                        set_ready(&ready_state_clone, ReadyState::Closed);
                        let _ = event_tx.send(WebSocketEvent::Close(Some(code), Some(reason)));
                        break;
                    }
                }
            }
        });
        let connection: _ = WebSocketConnection {
            id,
            url,
            ready_state,
            cmd_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            thread_id,
            epoch,
        };
        self.connections.lock().unwrap().insert(id, connection);
        Ok(id)
    }
    /// Send message on a WebSocket connection
    pub fn send(&self, id: u64, message: String) -> Result<()> {
        let connections: _ = self.connections.lock().unwrap();
        if let Some(conn) = connections.get(&id) {
            let state: _ = *conn.ready_state.lock().unwrap();
            if state != ReadyState::Open {
                return Err(anyhow::anyhow!(
                    "WebSocket is not open (state: {:?})",
                    state
                ));
            }
            conn.cmd_tx.send(WebSocketCommand::Send(message))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("WebSocket not found: {}", id))
        }
    }
    /// Send binary message on a WebSocket connection
    pub fn send_binary(&self, id: u64, message: Vec<u8>) -> Result<()> {
        let connections: _ = self.connections.lock().unwrap();
        if let Some(conn) = connections.get(&id) {
            let state: _ = *conn.ready_state.lock().unwrap();
            if state != ReadyState::Open {
                return Err(anyhow::anyhow!(
                    "WebSocket is not open (state: {:?})",
                    state
                ));
            }
            conn.cmd_tx.send(WebSocketCommand::SendBinary(message))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("WebSocket not found: {}", id))
        }
    }
    /// Close a WebSocket connection. Closing twice is a no-op.
    pub fn close(&self, id: u64, code: Option<u16>, reason: Option<String>) -> Result<()> {
        let connections: _ = self.connections.lock().unwrap();
        if let Some(conn) = connections.get(&id) {
            let state = *conn.ready_state.lock().unwrap();
            if state == ReadyState::Closing || state == ReadyState::Closed {
                return Ok(());
            }
            conn.cmd_tx.send(WebSocketCommand::Close(code, reason))?;
            Ok(())
        } else {
            Ok(())
        }
    }
    /// Get ready state of a WebSocket connection
    pub fn get_ready_state(&self, id: u64) -> Option<ReadyState> {
        let connections: _ = self.connections.lock().unwrap();
        connections
            .get(&id)
            .map(|conn| *conn.ready_state.lock().unwrap())
    }
    /// Poll for events (non-blocking)
    pub fn poll_events(&self, id: u64) -> Vec<WebSocketEvent> {
        let connections: _ = self.connections.lock().unwrap();
        if let Some(conn) = connections.get(&id) {
            let mut events = Vec::new();
            let mut rx = conn.event_rx.lock().unwrap();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
            events
        } else {
            Vec::new()
        }
    }
    /// Remove a closed connection
    pub fn remove(&self, id: u64) {
        self.connections.lock().unwrap().remove(&id);
    }
}
// Global WebSocket manager instance
pub static WS_MANAGER: Lazy<WebSocketManager> = Lazy::new(|| {
    let manager = WebSocketManager::new();
    MANAGER_READY.store(true, Ordering::Release);
    manager
});

/// Drop sockets from a previous `execute_code` on this thread so they cannot
/// keep a later script's event loop alive.
pub fn begin_execution_epoch() {
    EXECUTION_EPOCH.with(|epoch| epoch.set(epoch.get().wrapping_add(1)));
    if !MANAGER_READY.load(Ordering::Acquire) {
        return;
    }
    let thread_id = std::thread::current().id();
    let epoch = current_epoch();
    let mut connections = WS_MANAGER.connections.lock().unwrap();
    connections.retain(|_, conn| conn.thread_id != thread_id || conn.epoch == epoch);
}

/// True when this execution still has a socket that is not finished, or a
/// finished socket whose open/message/close/error events have not been
/// delivered yet.
pub fn has_pending_websocket_work() -> bool {
    if !MANAGER_READY.load(Ordering::Acquire) {
        return false;
    }
    let connections = WS_MANAGER.connections.lock().unwrap();
    connections.values().any(|conn| {
        if !connection_is_current(conn) {
            return false;
        }
        let state = *conn.ready_state.lock().unwrap();
        if state != ReadyState::Closed {
            return true;
        }
        !conn.event_rx.lock().unwrap().is_empty()
    })
}
/// Setup WebSocket API in V8 context
pub fn setup_websocket_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    // Create WebSocket constructor
    let websocket_template: _ = v8::FunctionTemplate::new(scope, websocket_constructor_callback);
    let constructor: _ = websocket_template.get_function(scope).unwrap();
    // Set WebSocket to global
    let global: _ = context.global(scope);
    let websocket_key: _ = v8::String::new(scope, "WebSocket").unwrap();
    global.set(scope, websocket_key.into(), constructor.into());
    // Add ReadyState constants to constructor
    let connecting_key: _ = v8::String::new(scope, "CONNECTING").unwrap();
    let connecting_val: _ = v8::Integer::new(scope, 0).into();
    constructor.set(scope, connecting_key.into(), connecting_val);
    let open_key: _ = v8::String::new(scope, "OPEN").unwrap();
    let open_val: _ = v8::Integer::new(scope, 1).into();
    constructor.set(scope, open_key.into(), open_val);
    let closing_key: _ = v8::String::new(scope, "CLOSING").unwrap();
    let closing_val: _ = v8::Integer::new(scope, 2).into();
    constructor.set(scope, closing_key.into(), closing_val);
    let closed_key: _ = v8::String::new(scope, "CLOSED").unwrap();
    let closed_val: _ = v8::Integer::new(scope, 3).into();
    constructor.set(scope, closed_key.into(), closed_val);
    let proto_key: _ = v8::String::new(scope, "prototype").unwrap();
    if let Some(proto_val) = constructor.get(scope, proto_key.into()) {
        if let Some(proto) = proto_val.to_object(scope) {
            for (name, value) in [
                ("CONNECTING", 0),
                ("OPEN", 1),
                ("CLOSING", 2),
                ("CLOSED", 3),
            ] {
                let key = v8::String::new(scope, name).unwrap();
                let val = v8::Integer::new(scope, value);
                proto.set(scope, key.into(), val.into());
            }
        }
    }
    Ok(())
}

const SOCKET_LIST_KEY: &str = "__amberWebSockets";

fn register_socket_object(scope: &mut v8::PinScope, socket: v8::Local<v8::Object>) {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let key = v8::String::new(scope, SOCKET_LIST_KEY).unwrap();
    let existing = global.get(scope, key.into());
    let list = existing.and_then(|value| v8::Local::<v8::Array>::try_from(value).ok());
    let list = if let Some(list) = list {
        list
    } else {
        let list = v8::Array::new(scope, 0);
        global.set(scope, key.into(), list.into());
        list
    };
    let index = list.length();
    list.set_index(scope, index, socket.into());
}

fn bytes_from_value(data: v8::Local<v8::Value>) -> Option<Vec<u8>> {
    if let Ok(buffer) = v8::Local::<v8::ArrayBuffer>::try_from(data) {
        return Some(copy_array_buffer(buffer));
    }
    if let Ok(view) = v8::Local::<v8::Uint8Array>::try_from(data) {
        let len = view.byte_length();
        let mut bytes = vec![0u8; len];
        if len == 0 || view.copy_contents(&mut bytes) == len {
            return Some(bytes);
        }
    }
    None
}

fn copy_array_buffer(buffer: v8::Local<v8::ArrayBuffer>) -> Vec<u8> {
    let len = buffer.byte_length();
    let mut bytes = vec![0u8; len];
    if len == 0 {
        return bytes;
    }
    let store = buffer.get_backing_store();
    if let Some(ptr) = store.data() {
        unsafe {
            std::ptr::copy_nonoverlapping(ptr.as_ptr() as *const u8, bytes.as_mut_ptr(), len);
        }
    }
    bytes
}

fn array_buffer_from_bytes<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    data: &[u8],
) -> v8::Local<'a, v8::ArrayBuffer> {
    let buffer = v8::ArrayBuffer::new(scope, data.len());
    if data.is_empty() {
        return buffer;
    }
    let store = buffer.get_backing_store();
    if let Some(ptr) = store.data() {
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.as_ptr() as *mut u8, data.len());
        }
    }
    buffer
}

fn set_property(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    name: &str,
    value: v8::Local<v8::Value>,
) {
    let key = v8::String::new(scope, name).unwrap();
    object.set(scope, key.into(), value);
}

fn call_handler(
    scope: &mut v8::PinScope,
    socket: v8::Local<v8::Object>,
    name: &str,
    event: v8::Local<v8::Object>,
) -> bool {
    let key = v8::String::new(scope, name).unwrap();
    let Some(handler) = socket.get(scope, key.into()) else {
        return false;
    };
    if !handler.is_function() {
        return true;
    }
    let Ok(function) = v8::Local::<v8::Function>::try_from(handler) else {
        return true;
    };
    let receiver: v8::Local<v8::Value> = socket.into();
    let argument: v8::Local<v8::Value> = event.into();
    function.call(scope, receiver, &[argument]).is_some()
}

/// Deliver queued socket events to `onopen` / `onmessage` / `onclose` / `onerror`.
/// Returns true when at least one event was delivered.
pub fn pump_websocket_events(scope: &mut v8::PinScope) -> bool {
    if !MANAGER_READY.load(Ordering::Acquire) {
        return false;
    }
    let context = scope.get_current_context();
    let global = context.global(scope);
    let key = v8::String::new(scope, SOCKET_LIST_KEY).unwrap();
    let Some(existing) = global.get(scope, key.into()) else {
        return false;
    };
    let Ok(list) = v8::Local::<v8::Array>::try_from(existing) else {
        return false;
    };
    let mut delivered = false;
    let len = list.length();
    for index in 0..len {
        let Some(value) = list.get_index(scope, index) else {
            continue;
        };
        let Some(socket) = value.to_object(scope) else {
            continue;
        };
        let Some(id) = get_ws_id(scope, socket) else {
            continue;
        };
        let events = WS_MANAGER.poll_events(id);
        for event in events {
            let state = WS_MANAGER.get_ready_state(id).unwrap_or(ReadyState::Closed) as i32;
            let state_value: v8::Local<v8::Value> = v8::Integer::new(scope, state).into();
            set_property(scope, socket, "readyState", state_value);
            let event_obj = match &event {
                WebSocketEvent::Open => {
                    let obj = v8::Object::new(scope);
                    let type_value: v8::Local<v8::Value> =
                        v8::String::new(scope, "open").unwrap().into();
                    set_property(scope, obj, "type", type_value);
                    obj
                }
                WebSocketEvent::Message(data) => {
                    let obj = v8::Object::new(scope);
                    let type_value: v8::Local<v8::Value> =
                        v8::String::new(scope, "message").unwrap().into();
                    set_property(scope, obj, "type", type_value);
                    let data_value: v8::Local<v8::Value> =
                        v8::String::new(scope, data).unwrap().into();
                    set_property(scope, obj, "data", data_value);
                    obj
                }
                WebSocketEvent::Binary(data) => {
                    let obj = v8::Object::new(scope);
                    let type_value: v8::Local<v8::Value> =
                        v8::String::new(scope, "message").unwrap().into();
                    set_property(scope, obj, "type", type_value);
                    let buffer = array_buffer_from_bytes(scope, data);
                    set_property(scope, obj, "data", buffer.into());
                    obj
                }
                WebSocketEvent::Close(code, reason) => {
                    let obj = v8::Object::new(scope);
                    let type_value: v8::Local<v8::Value> =
                        v8::String::new(scope, "close").unwrap().into();
                    set_property(scope, obj, "type", type_value);
                    let code_number = code.unwrap_or(1005);
                    let code_value: v8::Local<v8::Value> =
                        v8::Integer::new(scope, i32::from(code_number)).into();
                    set_property(scope, obj, "code", code_value);
                    let reason_text = reason.clone().unwrap_or_default();
                    let reason_value: v8::Local<v8::Value> =
                        v8::String::new(scope, &reason_text).unwrap().into();
                    set_property(scope, obj, "reason", reason_value);
                    let clean = code_number == 1000;
                    let clean_value: v8::Local<v8::Value> = v8::Boolean::new(scope, clean).into();
                    set_property(scope, obj, "wasClean", clean_value);
                    obj
                }
                WebSocketEvent::Error(message) => {
                    create_error_event_object(scope, message, "WebSocket", 0, 0, None)
                }
            };
            let handler = match event {
                WebSocketEvent::Open => "onopen",
                WebSocketEvent::Message(_) | WebSocketEvent::Binary(_) => "onmessage",
                WebSocketEvent::Close(_, _) => "onclose",
                WebSocketEvent::Error(_) => "onerror",
            };
            delivered = true;
            if !call_handler(scope, socket, handler, event_obj) {
                break;
            }
        }
    }
    delivered
}
/// WebSocket constructor callback
fn websocket_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Get URL argument
    let url: _ = if args.length() > 0 {
        args.get(0)
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    } else {
        let error: _ = v8::String::new(scope, "WebSocket URL required").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    };
    // Validate URL
    if url.is_empty() || (!url.starts_with("ws://") && !url.starts_with("wss://")) {
        let error: _ = v8::String::new(scope, "Invalid WebSocket URL").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    if args.length() > 1 {
        let protocol = args.get(1);
        if !protocol.is_null() && !protocol.is_undefined() {
            let error: _ = v8::String::new(scope, "WebSocket protocols are not supported").unwrap();
            let error_obj: _ = v8::Exception::type_error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    }
    if !args.is_construct_call() {
        let error: _ =
            v8::String::new(scope, "WebSocket constructor must be called with new").unwrap();
        let error_obj: _ = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Network,
        crate::permissions::PermissionAction::Connect,
        crate::permissions::ResourceId::Url(url.clone()),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }
    // Create real WebSocket connection
    let ws_id: _ = match WS_MANAGER.connect(url.clone()) {
        Ok(id) => id,
        Err(e) => {
            let error: _ =
                v8::String::new(scope, &format!("WebSocket connection failed: {}", e)).unwrap();
            let error_obj: _ = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };
    // Use the constructed object so `instanceof WebSocket` holds.
    let ws_obj: _ = args.this();
    // Store WebSocket ID as internal property
    let id_key: _ = v8::String::new(scope, "__wsId").unwrap();
    let id_val: v8::Local<v8::Value> = v8::Number::new(scope, ws_id as f64).into();
    ws_obj.set(scope, id_key.into(), id_val);
    // Set properties directly
    let url_key: _ = v8::String::new(scope, "url").unwrap();
    let url_val: v8::Local<v8::Value> = v8::String::new(scope, &url).unwrap().into();
    ws_obj.set(scope, url_key.into(), url_val);
    let ready_state_key: _ = v8::String::new(scope, "readyState").unwrap();
    let ready_state_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
    ws_obj.set(scope, ready_state_key.into(), ready_state_val);
    let buffered_key: _ = v8::String::new(scope, "bufferedAmount").unwrap();
    let buffered_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
    ws_obj.set(scope, buffered_key.into(), buffered_val);
    let ext_key: _ = v8::String::new(scope, "extensions").unwrap();
    let ext_val: v8::Local<v8::Value> = v8::String::new(scope, "").unwrap().into();
    ws_obj.set(scope, ext_key.into(), ext_val);
    let protocol_key: _ = v8::String::new(scope, "protocol").unwrap();
    let protocol_val: v8::Local<v8::Value> = v8::String::new(scope, "").unwrap().into();
    ws_obj.set(scope, protocol_key.into(), protocol_val);
    let binary_type_key: _ = v8::String::new(scope, "binaryType").unwrap();
    let binary_type_val: v8::Local<v8::Value> =
        v8::String::new(scope, "arraybuffer").unwrap().into();
    ws_obj.set(scope, binary_type_key.into(), binary_type_val);
    // Set event handler properties (initial null)
    let null_val: v8::Local<v8::Value> = v8::null(scope).into();
    let onopen_key: _ = v8::String::new(scope, "onopen").unwrap();
    ws_obj.set(scope, onopen_key.into(), null_val);
    let onmessage_key: _ = v8::String::new(scope, "onmessage").unwrap();
    let null_val: v8::Local<v8::Value> = v8::null(scope).into();
    ws_obj.set(scope, onmessage_key.into(), null_val);
    let onclose_key: _ = v8::String::new(scope, "onclose").unwrap();
    let null_val: v8::Local<v8::Value> = v8::null(scope).into();
    ws_obj.set(scope, onclose_key.into(), null_val);
    let onerror_key: _ = v8::String::new(scope, "onerror").unwrap();
    // Preserve Amber' existing callable default so code can invoke onerror
    // without first installing a handler.
    let onerror_handler = v8::Function::new(scope, websocket_noop_error_handler).unwrap();
    ws_obj.set(scope, onerror_key.into(), onerror_handler.into());
    // Add methods
    let send_key: _ = v8::String::new(scope, "send").unwrap();
    let send_func: _ = v8::Function::new(scope, websocket_send_callback).unwrap();
    ws_obj.set(scope, send_key.into(), send_func.into());
    let close_key: _ = v8::String::new(scope, "close").unwrap();
    let close_func: _ = v8::Function::new(scope, websocket_close_callback).unwrap();
    ws_obj.set(scope, close_key.into(), close_func.into());
    let add_event_key: _ = v8::String::new(scope, "addEventListener").unwrap();
    let add_event_func: _ =
        v8::Function::new(scope, websocket_add_event_listener_callback).unwrap();
    ws_obj.set(scope, add_event_key.into(), add_event_func.into());
    let remove_event_key: _ = v8::String::new(scope, "removeEventListener").unwrap();
    let remove_event_func: _ =
        v8::Function::new(scope, websocket_remove_event_listener_callback).unwrap();
    ws_obj.set(scope, remove_event_key.into(), remove_event_func.into());
    register_socket_object(scope, ws_obj);
    let poll_events_key: _ = v8::String::new(scope, "_pollEvents").unwrap();
    let poll_events_func: _ = v8::Function::new(scope, websocket_poll_events_callback).unwrap();
    ws_obj.set(scope, poll_events_key.into(), poll_events_func.into());
    let update_ready_key: _ = v8::String::new(scope, "_updateReadyState").unwrap();
    let update_ready_func: _ =
        v8::Function::new(scope, websocket_update_ready_state_callback).unwrap();
    ws_obj.set(scope, update_ready_key.into(), update_ready_func.into());
    retval.set(ws_obj.into());
}

fn websocket_noop_error_handler(
    _scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
}

/// Get WebSocket ID from JS object
fn get_ws_id(scope: &mut v8::PinScope, this: v8::Local<v8::Object>) -> Option<u64> {
    let id_key: _ = v8::String::new(scope, "__wsId").unwrap();
    let id_val: _ = this.get(scope, id_key.into())?;
    if id_val.is_number() {
        Some(id_val.number_value(scope)? as u64)
    } else {
        None
    }
}
/// WebSocket send callback
fn websocket_send_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() == 0 {
        let error: _ = v8::String::new(scope, "send requires data").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    let this: _ = args.this();
    let ws_id: _ = match get_ws_id(scope, this) {
        Some(id) => id,
        None => {
            let error: _ = v8::String::new(scope, "Invalid WebSocket object").unwrap();
            let error_obj: _ = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };
    let data: _ = args.get(0);
    let send_result = if let Some(bytes) = bytes_from_value(data) {
        WS_MANAGER.send_binary(ws_id, bytes)
    } else {
        let message: _ = data.to_string(scope).unwrap().to_rust_string_lossy(scope);
        WS_MANAGER.send(ws_id, message)
    };
    if let Err(e) = send_result {
        let error: _ = v8::String::new(scope, &format!("WebSocket send failed: {}", e)).unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
    }
}
/// WebSocket close callback
fn websocket_close_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let this: _ = args.this();
    let ws_id: _ = match get_ws_id(scope, this) {
        Some(id) => id,
        None => {
            let error: _ = v8::String::new(scope, "Invalid WebSocket object").unwrap();
            let error_obj: _ = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
            return;
        }
    };
    let code: _ = if args.length() > 0 && args.get(0).is_number() {
        Some(args.get(0).number_value(scope).unwrap() as u16)
    } else {
        None
    };
    let reason: _ = if args.length() > 1 && args.get(1).is_string() {
        Some(
            args.get(1)
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope),
        )
    } else {
        None
    };
    if let Err(e) = WS_MANAGER.close(ws_id, code, reason) {
        let error: _ = v8::String::new(scope, &format!("WebSocket close failed: {}", e)).unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
    }
}
/// WebSocket addEventListener callback
fn websocket_add_event_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() < 2 {
        let error: _ =
            v8::String::new(scope, "addEventListener requires type and listener").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    let event_type: _ = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let listener: _ = args.get(1);
    if !listener.is_function() {
        let error: _ = v8::String::new(scope, "Listener must be a function").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    // Store listener in appropriate on* property
    let this: _ = args.this();
    let prop_name: _ = format!("on{}", event_type);
    let prop_key: _ = v8::String::new(scope, &prop_name).unwrap();
    this.set(scope, prop_key.into(), listener);
}
/// WebSocket removeEventListener callback
fn websocket_remove_event_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    if args.length() < 2 {
        let error: _ =
            v8::String::new(scope, "removeEventListener requires type and listener").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }
    let event_type: _ = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let this: _ = args.this();
    let prop_name: _ = format!("on{}", event_type);
    let prop_key: _ = v8::String::new(scope, &prop_name).unwrap();
    let null_val: v8::Local<v8::Value> = v8::null(scope).into();
    this.set(scope, prop_key.into(), null_val);
}
/// Poll for WebSocket events (used internally for event loop integration)
fn websocket_poll_events_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this: _ = args.this();
    let ws_id: _ = match get_ws_id(scope, this) {
        Some(id) => id,
        None => {
            rv.set(v8::Array::new(scope, 0).into());
            return;
        }
    };
    let events: _ = WS_MANAGER.poll_events(ws_id);
    let arr: _ = v8::Array::new(scope, events.len() as i32);
    // Get global context for accessing Blob constructor
    let context: _ = scope.get_current_context();
    let global: _ = context.global(scope);
    for (i, event) in events.iter().enumerate() {
        let mut event_obj: _ = v8::Object::new(scope);
        match event {
            WebSocketEvent::Open => {
                let type_key: _ = v8::String::new(scope, "type").unwrap();
                let type_val: _ = v8::String::new(scope, "open").unwrap();
                event_obj.set(scope, type_key.into(), type_val.into());
            }
            WebSocketEvent::Message(data) => {
                let type_key: _ = v8::String::new(scope, "type").unwrap();
                let type_val: _ = v8::String::new(scope, "message").unwrap();
                event_obj.set(scope, type_key.into(), type_val.into());
                let data_key: _ = v8::String::new(scope, "data").unwrap();
                let data_val: _ = v8::String::new(scope, data).unwrap();
                event_obj.set(scope, data_key.into(), data_val.into());
            }
            WebSocketEvent::Binary(data) => {
                let type_key: _ = v8::String::new(scope, "type").unwrap();
                let type_val: _ = v8::String::new(scope, "message").unwrap();
                event_obj.set(scope, type_key.into(), type_val.into());
                let data_key: _ = v8::String::new(scope, "data").unwrap();

                // v0.3.332: Check binaryType and return appropriate type
                // For AI workloads, binaryType='arraybuffer' is essential for
                // passing model weights, tensor data, and other binary content
                let binary_type_key: _ = v8::String::new(scope, "binaryType").unwrap();
                let binary_type_val: _ = this.get(scope, binary_type_key.into());

                if let Some(bt) = binary_type_val {
                    let bt_str: _ = bt.to_string(scope).unwrap().to_rust_string_lossy(scope);
                    if bt_str == "arraybuffer" {
                        // Create ArrayBuffer from binary data
                        let buffer: _ = v8::ArrayBuffer::new(scope, data.len());
                        // Copy data into ArrayBuffer's backing store
                        let store = buffer.get_backing_store();
                        unsafe {
                            let ptr = store
                                .data()
                                .map(|p| p.as_ptr() as *mut u8)
                                .unwrap_or(std::ptr::null_mut());
                            std::slice::from_raw_parts_mut(ptr, data.len()).copy_from_slice(&data);
                        }
                        event_obj.set(scope, data_key.into(), buffer.into());
                    } else if bt_str == "blob" {
                        // Create Blob from binary data
                        // Blob constructor: new Blob([data], { type: 'application/octet-stream' })
                        let blob_ctor_key: _ = v8::String::new(scope, "Blob").unwrap();
                        let blob_ctor: _ = global.get(scope, blob_ctor_key.into()).unwrap();
                        if blob_ctor.is_function() {
                            let blob_ctor_func: v8::Local<v8::Function> =
                                blob_ctor.try_into().unwrap();
                            // Create buffer first to avoid borrow issues
                            let buffer: _ = v8::ArrayBuffer::new(scope, data.len());
                            let buffer_val: v8::Local<v8::Value> = buffer.into();
                            let data_array: _ = v8::Array::new_with_elements(scope, &[buffer_val]);
                            let options_key: _ = v8::String::new(scope, "type").unwrap();
                            let options_val: _ = v8::Object::new(scope);
                            let mime_type: _ =
                                v8::String::new(scope, "application/octet-stream").unwrap();
                            options_val.set(scope, options_key.into(), mime_type.into());

                            let blob_args: &[v8::Local<v8::Value>] =
                                &[data_array.into(), options_val.into()];
                            if let Some(blob) = blob_ctor_func.new_instance(scope, blob_args) {
                                event_obj.set(scope, data_key.into(), blob.into());
                            } else {
                                // Fallback: set data as ArrayBuffer using the buffer we already created
                                // Copy data into the buffer's backing store
                                let store = buffer.get_backing_store();
                                unsafe {
                                    let ptr = store
                                        .data()
                                        .map(|p| p.as_ptr() as *mut u8)
                                        .unwrap_or(std::ptr::null_mut());
                                    std::slice::from_raw_parts_mut(ptr, data.len())
                                        .copy_from_slice(&data);
                                }
                                event_obj.set(scope, data_key.into(), buffer.into());
                            }
                        } else {
                            // Fallback to ArrayBuffer
                            let buffer: _ = v8::ArrayBuffer::new(scope, data.len());
                            let store = buffer.get_backing_store();
                            unsafe {
                                let ptr = store
                                    .data()
                                    .map(|p| p.as_ptr() as *mut u8)
                                    .unwrap_or(std::ptr::null_mut());
                                std::slice::from_raw_parts_mut(ptr, data.len())
                                    .copy_from_slice(&data);
                            }
                            event_obj.set(scope, data_key.into(), buffer.into());
                        }
                    } else {
                        // Default: treat as string (legacy behavior)
                        let data_str: _ = String::from_utf8_lossy(data);
                        let data_val: _ = v8::String::new(scope, &data_str).unwrap();
                        event_obj.set(scope, data_key.into(), data_val.into());
                    }
                } else {
                    // Default: treat as string (legacy behavior)
                    let data_str: _ = String::from_utf8_lossy(data);
                    let data_val: _ = v8::String::new(scope, &data_str).unwrap();
                    event_obj.set(scope, data_key.into(), data_val.into());
                }
            }
            WebSocketEvent::Close(code, reason) => {
                let type_key: _ = v8::String::new(scope, "type").unwrap();
                let type_val: _ = v8::String::new(scope, "close").unwrap();
                event_obj.set(scope, type_key.into(), type_val.into());
                if let Some(c) = code {
                    let code_key: _ = v8::String::new(scope, "code").unwrap();
                    let code_val: _ = v8::Integer::new(scope, *c as i32);
                    event_obj.set(scope, code_key.into(), code_val.into());
                }
                if let Some(r) = reason {
                    let reason_key: _ = v8::String::new(scope, "reason").unwrap();
                    let reason_val: _ = v8::String::new(scope, r).unwrap();
                    event_obj.set(scope, reason_key.into(), reason_val.into());
                }
            }
            WebSocketEvent::Error(msg) => {
                // v0.3.334: Use ErrorEvent for proper error event structure
                // This provides type, message, filename, lineno, colno, and error properties
                let error_event = create_error_event_object(scope, msg, "WebSocket", 0, 0, None);
                // Copy ErrorEvent properties to the event object
                let type_key = v8::String::new(scope, "type").unwrap();
                let error_type = v8::String::new(scope, "error").unwrap();
                error_event.set(scope, type_key.into(), error_type.into());
                event_obj = error_event;
            }
        }
        arr.set_index(scope, i as u32, event_obj.into());
    }
    rv.set(arr.into());
}
/// Update readyState from native state
fn websocket_update_ready_state_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let this: _ = args.this();
    let ws_id: _ = match get_ws_id(scope, this) {
        Some(id) => id,
        None => {
            rv.set(v8::Integer::new(scope, 3).into()); // CLOSED
            return;
        }
    };
    let state: _ = WS_MANAGER
        .get_ready_state(ws_id)
        .unwrap_or(ReadyState::Closed);
    let state_int: _ = state as i32;
    // Update the readyState property
    let ready_state_key: _ = v8::String::new(scope, "readyState").unwrap();
    let ready_state_val: _ = v8::Integer::new(scope, state_int);
    this.set(scope, ready_state_key.into(), ready_state_val.into());
    rv.set(ready_state_val.into());
}
#[cfg(test)]
mod tests {
    use super::{ReadyState, WebSocketManager};
    use std::sync::atomic::Ordering;

    #[test]
    fn test_ready_state_constants() {
        assert_eq!(ReadyState::Connecting as u8, 0);
        assert_eq!(ReadyState::Open as u8, 1);
        assert_eq!(ReadyState::Closing as u8, 2);
        assert_eq!(ReadyState::Closed as u8, 3);
    }
    #[test]
    fn test_websocket_manager_creation() {
        // Just test that the manager can be created
        let manager: _ = WebSocketManager::new();
        assert_eq!(manager.next_id.load(Ordering::SeqCst), 1);
    }
}
