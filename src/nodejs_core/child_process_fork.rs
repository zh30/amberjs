//! Narrow `child_process.fork` with real OS-process bootstrap + TCP IPC.
//!
//! Stable contract: `docs/CHILD_PROCESS_FORK_CONTRACT.md` (G67).
//! Parent spawns this machine's `amber run <module>` with `AMBER_FORK_CHANNEL`
//! set to a localhost TCP address. Parent and child exchange length-prefixed
//! JSON frames. This is not full Node `fork` (no silent/stdio/execArgv product).

use anyhow::Result;
use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const ENV_CHANNEL: &str = "AMBER_FORK_CHANNEL";
const MAX_FRAME: usize = 16 * 1024 * 1024;

static FORK_SEQ: AtomicU64 = AtomicU64::new(1);
static FORK_ALIVE: AtomicUsize = AtomicUsize::new(0);
static FORK_CHILD_INSTALLED: AtomicBool = AtomicBool::new(false);
static FORK_CHILD_CONNECTED: AtomicBool = AtomicBool::new(false);

static FORK_EVENTS: Lazy<Mutex<VecDeque<ForkEvent>>> = Lazy::new(|| Mutex::new(VecDeque::new()));
static FORK_WRITERS: Lazy<Mutex<HashMap<u64, Arc<Mutex<TcpStream>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static FORK_CONNECTED: Lazy<Mutex<HashMap<u64, bool>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static FORK_CHILD_WRITER: Lazy<Mutex<Option<Arc<Mutex<TcpStream>>>>> =
    Lazy::new(|| Mutex::new(None));
static FORK_CHILD_INCOMING: Lazy<Mutex<VecDeque<String>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

thread_local! {
    static FORK_LISTENERS: std::cell::RefCell<HashMap<(u64, String), Vec<v8::Global<v8::Function>>>> =
        std::cell::RefCell::new(HashMap::new());
    static FORK_OBJECTS: std::cell::RefCell<HashMap<u64, v8::Global<v8::Object>>> =
        std::cell::RefCell::new(HashMap::new());
    static CHILD_MESSAGE_LISTENERS: std::cell::RefCell<Vec<v8::Global<v8::Function>>> =
        std::cell::RefCell::new(Vec::new());
}

#[derive(Debug)]
enum ForkEvent {
    Message { fork_id: u64, payload: String },
    Exit { fork_id: u64, code: i32 },
    Disconnect { fork_id: u64 },
}

fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    if payload.len() > MAX_FRAME {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fork IPC frame too large",
        ));
    }
    let len = (payload.len() as u32).to_be_bytes();
    stream.write_all(&len)?;
    stream.write_all(payload)?;
    stream.flush()
}

fn read_frame(stream: &mut TcpStream) -> std::io::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match stream.read_exact(&mut len_buf) {
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => return Ok(None),
        Err(e) => return Err(e),
        Ok(()) => {}
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "fork IPC frame too large",
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(Some(buf))
}

fn encode_message_frame(data_json: &str) -> String {
    format!(r#"{{"v":1,"t":"msg","d":{}}}"#, data_json)
}

fn decode_message_payload(frame: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(frame).ok()?;
    let t = value.get("t")?.as_str()?;
    if t != "msg" {
        return None;
    }
    let d = value.get("d")?;
    Some(d.to_string())
}

fn json_stringify(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<String> {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let json_key = v8::String::new(scope, "JSON").unwrap();
    let json_val = global.get(scope, json_key.into())?;
    let json_obj = v8::Local::<v8::Object>::try_from(json_val).ok()?;
    let stringify_key = v8::String::new(scope, "stringify").unwrap();
    let stringify_val = json_obj.get(scope, stringify_key.into())?;
    let stringify_fn = v8::Local::<v8::Function>::try_from(stringify_val).ok()?;
    let undefined = v8::undefined(scope);
    let result = stringify_fn.call(scope, undefined.into(), &[value])?;
    if result.is_undefined() || result.is_null() {
        return None;
    }
    Some(result.to_rust_string_lossy(scope))
}

fn json_parse<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    text: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let json_key = v8::String::new(scope, "JSON").unwrap();
    let json_val = global.get(scope, json_key.into())?;
    let json_obj = v8::Local::<v8::Object>::try_from(json_val).ok()?;
    let parse_key = v8::String::new(scope, "parse").unwrap();
    let parse_val = json_obj.get(scope, parse_key.into())?;
    let parse_fn = v8::Local::<v8::Function>::try_from(parse_val).ok()?;
    let undefined = v8::undefined(scope);
    let text_val = v8::String::new(scope, text)?.into();
    parse_fn.call(scope, undefined.into(), &[text_val])
}

/// True while a forked child is alive, IPC is connected, or events await delivery.
pub fn has_pending_fork_work() -> bool {
    if FORK_ALIVE.load(Ordering::SeqCst) > 0 {
        return true;
    }
    if FORK_CHILD_CONNECTED.load(Ordering::SeqCst) {
        return true;
    }
    if FORK_EVENTS.lock().map(|q| !q.is_empty()).unwrap_or(false) {
        return true;
    }
    FORK_CHILD_INCOMING
        .lock()
        .map(|q| !q.is_empty())
        .unwrap_or(false)
}

/// Drain parent/child fork IPC events into JS listeners (V8 thread only).
pub fn pump_pending_fork_jobs(scope: &mut v8::PinScope) -> usize {
    let mut n = 0;
    n += pump_parent_fork_events(scope);
    n += pump_child_incoming(scope);
    n
}

fn pump_parent_fork_events(scope: &mut v8::PinScope) -> usize {
    let mut batch = Vec::new();
    if let Ok(mut q) = FORK_EVENTS.lock() {
        while let Some(item) = q.pop_front() {
            batch.push(item);
        }
    }
    let n = batch.len();
    for event in batch {
        match event {
            ForkEvent::Message { fork_id, payload } => {
                deliver_parent_listeners(scope, fork_id, "message", &payload, None);
            }
            ForkEvent::Disconnect { fork_id } => {
                if let Ok(mut map) = FORK_CONNECTED.lock() {
                    map.insert(fork_id, false);
                }
                update_child_object_connected(scope, fork_id, false);
                deliver_parent_listeners(scope, fork_id, "disconnect", "", None);
            }
            ForkEvent::Exit { fork_id, code } => {
                if let Ok(mut map) = FORK_CONNECTED.lock() {
                    map.insert(fork_id, false);
                }
                FORK_WRITERS.lock().ok().map(|mut m| m.remove(&fork_id));
                update_child_object_exit(scope, fork_id, code);
                deliver_parent_listeners(scope, fork_id, "exit", "", Some(code));
                deliver_parent_listeners(scope, fork_id, "close", "", Some(code));
            }
        }
    }
    n
}

fn pump_child_incoming(scope: &mut v8::PinScope) -> usize {
    // Keep queued frames until at least one 'message' listener exists.
    let listener_count = CHILD_MESSAGE_LISTENERS.with(|cell| cell.borrow().len());
    if listener_count == 0 {
        return 0;
    }
    let mut batch = Vec::new();
    if let Ok(mut q) = FORK_CHILD_INCOMING.lock() {
        while let Some(item) = q.pop_front() {
            batch.push(item);
        }
    }
    let n = batch.len();
    for payload in batch {
        let Some(value) = json_parse(scope, &payload) else {
            continue;
        };
        CHILD_MESSAGE_LISTENERS.with(|cell| {
            for listener in cell.borrow().iter() {
                let func = v8::Local::new(scope, listener);
                let undefined = v8::undefined(scope);
                let _ = func.call(scope, undefined.into(), &[value]);
            }
        });
    }
    n
}

fn deliver_parent_listeners(
    scope: &mut v8::PinScope,
    fork_id: u64,
    event: &str,
    payload: &str,
    exit_code: Option<i32>,
) {
    let this = FORK_OBJECTS.with(|cell| {
        cell.borrow()
            .get(&fork_id)
            .map(|g| v8::Local::new(scope, g))
    });
    let this_val: v8::Local<v8::Value> = match this {
        Some(obj) => obj.into(),
        None => v8::undefined(scope).into(),
    };
    FORK_LISTENERS.with(|cell| {
        let borrow = cell.borrow();
        let Some(listeners) = borrow.get(&(fork_id, event.to_string())) else {
            return;
        };
        for listener in listeners {
            let func = v8::Local::new(scope, listener);
            match event {
                "message" => {
                    if let Some(value) = json_parse(scope, payload) {
                        let _ = func.call(scope, this_val, &[value]);
                    }
                }
                "exit" | "close" => {
                    let code = v8::Integer::new(scope, exit_code.unwrap_or(0));
                    let signal = v8::null(scope);
                    let _ = func.call(scope, this_val, &[code.into(), signal.into()]);
                }
                "disconnect" => {
                    let _ = func.call(scope, this_val, &[]);
                }
                _ => {}
            }
        }
    });
}

fn update_child_object_connected(scope: &mut v8::PinScope, fork_id: u64, connected: bool) {
    FORK_OBJECTS.with(|cell| {
        if let Some(global) = cell.borrow().get(&fork_id) {
            let obj = v8::Local::new(scope, global);
            let key = v8::String::new(scope, "connected").unwrap();
            let val = v8::Boolean::new(scope, connected);
            obj.set(scope, key.into(), val.into());
        }
    });
}

fn update_child_object_exit(scope: &mut v8::PinScope, fork_id: u64, code: i32) {
    FORK_OBJECTS.with(|cell| {
        if let Some(global) = cell.borrow().get(&fork_id) {
            let obj = v8::Local::new(scope, global);
            let connected_key = v8::String::new(scope, "connected").unwrap();
            obj.set(
                scope,
                connected_key.into(),
                v8::Boolean::new(scope, false).into(),
            );
            let exit_key = v8::String::new(scope, "exitCode").unwrap();
            obj.set(scope, exit_key.into(), v8::Integer::new(scope, code).into());
        }
    });
}

fn fork_id_from_this(scope: &mut v8::PinScope, this: v8::Local<v8::Object>) -> Option<u64> {
    let key = v8::String::new(scope, "_amberForkId").unwrap();
    let val = this.get(scope, key.into())?;
    if val.is_number() {
        return Some(val.number_value(scope).unwrap_or(0.0) as u64);
    }
    None
}

/// `child_process.fork(modulePath[, args][, options])`
pub fn cp_fork_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let module_path = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if module_path.is_empty() {
        let msg = v8::String::new(scope, "fork: module path is required").unwrap();
        let err = v8::Exception::error(scope, msg);
        scope.throw_exception(err);
        return;
    }
    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Process,
        crate::permissions::PermissionAction::Execute,
        crate::permissions::ResourceId::Name(crate::permissions::process_command_name(
            &module_path,
        )),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }

    let mut fork_args: Vec<String> = Vec::new();
    let arg1 = args.get(1);
    if arg1.is_array() {
        fork_args = crate::nodejs_core::child_process::string_vec_from_v8_array_value(scope, arg1);
    }

    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("fork: listen failed: {}", e)).unwrap();
            scope.throw_exception(v8::Exception::error(scope, msg));
            return;
        }
    };
    let channel_addr = match listener.local_addr() {
        Ok(a) => a.to_string(),
        Err(e) => {
            let msg = v8::String::new(scope, &format!("fork: local_addr failed: {}", e)).unwrap();
            scope.throw_exception(v8::Exception::error(scope, msg));
            return;
        }
    };

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("fork: current_exe failed: {}", e)).unwrap();
            scope.throw_exception(v8::Exception::error(scope, msg));
            return;
        }
    };

    let mut command = Command::new(&exe);
    command
        .arg("run")
        .arg(&module_path)
        .args(&fork_args)
        .env(ENV_CHANNEL, &channel_addr)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("fork: spawn failed: {}", e)).unwrap();
            scope.throw_exception(v8::Exception::error(scope, msg));
            return;
        }
    };
    let pid = child.id();
    let fork_id = FORK_SEQ.fetch_add(1, Ordering::Relaxed);
    FORK_ALIVE.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut map) = FORK_CONNECTED.lock() {
        map.insert(fork_id, true);
    }

    // Wait for the child IPC connect before returning so immediate child.send works.
    let stream = match accept_child_ipc(&listener, &mut child) {
        Ok(Some(s)) => s,
        Ok(None) => {
            // Child exited before connecting.
            let code = child.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
            push_exit(fork_id, code);
            let child_obj = make_fork_child_object(scope, fork_id, pid);
            let connected_key = v8::String::new(scope, "connected").unwrap();
            child_obj.set(
                scope,
                connected_key.into(),
                v8::Boolean::new(scope, false).into(),
            );
            let exit_key = v8::String::new(scope, "exitCode").unwrap();
            child_obj.set(scope, exit_key.into(), v8::Integer::new(scope, code).into());
            retval.set(child_obj.into());
            return;
        }
        Err(e) => {
            let _ = child.kill();
            FORK_ALIVE.fetch_sub(1, Ordering::SeqCst);
            let msg = v8::String::new(scope, &format!("fork: IPC accept failed: {}", e)).unwrap();
            scope.throw_exception(v8::Exception::error(scope, msg));
            return;
        }
    };

    let writer_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            let _ = child.kill();
            FORK_ALIVE.fetch_sub(1, Ordering::SeqCst);
            let msg = v8::String::new(scope, &format!("fork: IPC clone failed: {}", e)).unwrap();
            scope.throw_exception(v8::Exception::error(scope, msg));
            return;
        }
    };
    let writer = Arc::new(Mutex::new(writer_stream));
    if let Ok(mut map) = FORK_WRITERS.lock() {
        map.insert(fork_id, writer);
    }

    let child_obj = make_fork_child_object(scope, fork_id, pid);
    FORK_OBJECTS.with(|cell| {
        cell.borrow_mut()
            .insert(fork_id, v8::Global::new(scope, child_obj));
    });

    thread::spawn(move || {
        run_parent_ipc(fork_id, stream, child);
    });

    retval.set(child_obj.into());
}

fn accept_child_ipc(
    listener: &TcpListener,
    child: &mut Child,
) -> std::io::Result<Option<TcpStream>> {
    let _ = listener.set_nonblocking(true);
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        match listener.accept() {
            Ok((s, _)) => {
                let _ = s.set_nonblocking(false);
                return Ok(Some(s));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() > deadline {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "timed out waiting for forked child IPC connect",
                    ));
                }
                match child.try_wait() {
                    Ok(Some(_)) => return Ok(None),
                    Ok(None) => thread::sleep(Duration::from_millis(5)),
                    Err(e) => return Err(e),
                }
            }
            Err(e) => return Err(e),
        }
    }
}

fn run_parent_ipc(fork_id: u64, mut reader: TcpStream, mut child: Child) {
    let read_fork_id = fork_id;
    let reader_thread = thread::spawn(move || loop {
        match read_frame(&mut reader) {
            Ok(Some(buf)) => {
                let text = String::from_utf8_lossy(&buf).to_string();
                if let Some(payload) = decode_message_payload(&text) {
                    if let Ok(mut q) = FORK_EVENTS.lock() {
                        q.push_back(ForkEvent::Message {
                            fork_id: read_fork_id,
                            payload,
                        });
                    }
                }
            }
            Ok(None) | Err(_) => {
                if let Ok(mut q) = FORK_EVENTS.lock() {
                    q.push_back(ForkEvent::Disconnect {
                        fork_id: read_fork_id,
                    });
                }
                break;
            }
        }
    });

    let code = child.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
    let _ = reader_thread.join();
    push_exit(fork_id, code);
}

fn push_exit(fork_id: u64, code: i32) {
    if let Ok(mut map) = FORK_CONNECTED.lock() {
        map.insert(fork_id, false);
    }
    FORK_WRITERS.lock().ok().map(|mut m| m.remove(&fork_id));
    if let Ok(mut q) = FORK_EVENTS.lock() {
        q.push_back(ForkEvent::Exit { fork_id, code });
    }
    FORK_ALIVE.fetch_sub(1, Ordering::SeqCst);
}

fn make_fork_child_object<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    fork_id: u64,
    pid: u32,
) -> v8::Local<'s, v8::Object> {
    let child_obj = v8::Object::new(scope);

    let id_key = v8::String::new(scope, "_amberForkId").unwrap();
    child_obj.set(
        scope,
        id_key.into(),
        v8::Number::new(scope, fork_id as f64).into(),
    );

    let pid_key = v8::String::new(scope, "pid").unwrap();
    child_obj.set(
        scope,
        pid_key.into(),
        v8::Integer::new(scope, pid as i32).into(),
    );

    let connected_key = v8::String::new(scope, "connected").unwrap();
    child_obj.set(
        scope,
        connected_key.into(),
        v8::Boolean::new(scope, true).into(),
    );

    let killed_key = v8::String::new(scope, "killed").unwrap();
    child_obj.set(
        scope,
        killed_key.into(),
        v8::Boolean::new(scope, false).into(),
    );

    let exit_code_key = v8::String::new(scope, "exitCode").unwrap();
    child_obj.set(scope, exit_code_key.into(), v8::null(scope).into());

    let signal_key = v8::String::new(scope, "signal").unwrap();
    child_obj.set(scope, signal_key.into(), v8::null(scope).into());

    let send_fn = v8::Function::new(scope, fork_child_send_callback).unwrap();
    let send_key = v8::String::new(scope, "send").unwrap();
    child_obj.set(scope, send_key.into(), send_fn.into());

    let on_fn = v8::Function::new(scope, fork_child_on_callback).unwrap();
    let on_key = v8::String::new(scope, "on").unwrap();
    child_obj.set(scope, on_key.into(), on_fn.into());

    let disconnect_fn = v8::Function::new(scope, fork_child_disconnect_callback).unwrap();
    let disconnect_key = v8::String::new(scope, "disconnect").unwrap();
    child_obj.set(scope, disconnect_key.into(), disconnect_fn.into());

    child_obj
}

fn fork_child_send_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let Some(fork_id) = fork_id_from_this(scope, this) else {
        retval.set(v8::Boolean::new(scope, false).into());
        return;
    };
    let connected = FORK_CONNECTED
        .lock()
        .ok()
        .and_then(|m| m.get(&fork_id).copied())
        .unwrap_or(false);
    if !connected {
        retval.set(v8::Boolean::new(scope, false).into());
        return;
    }
    let Some(json) = json_stringify(scope, args.get(0)) else {
        let msg = v8::String::new(scope, "fork send: value is not JSON-serializable").unwrap();
        scope.throw_exception(v8::Exception::error(scope, msg));
        return;
    };
    let frame = encode_message_frame(&json);
    let writer = FORK_WRITERS
        .lock()
        .ok()
        .and_then(|m| m.get(&fork_id).cloned());
    let Some(writer) = writer else {
        retval.set(v8::Boolean::new(scope, false).into());
        return;
    };
    let ok = if let Ok(mut stream) = writer.lock() {
        write_frame(&mut stream, frame.as_bytes()).is_ok()
    } else {
        false
    };
    retval.set(v8::Boolean::new(scope, ok).into());
}

fn fork_child_on_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    retval.set(this.into());
    let Some(fork_id) = fork_id_from_this(scope, this) else {
        return;
    };
    let event = crate::nodejs_core::child_process::string_from_v8_value(scope, args.get(0));
    let listener = args.get(1);
    if !listener.is_function() {
        return;
    }
    let Ok(func) = v8::Local::<v8::Function>::try_from(listener) else {
        return;
    };
    if !matches!(event.as_str(), "message" | "exit" | "close" | "disconnect") {
        return;
    }
    FORK_LISTENERS.with(|cell| {
        cell.borrow_mut()
            .entry((fork_id, event))
            .or_default()
            .push(v8::Global::new(scope, func));
    });
}

fn fork_child_disconnect_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    retval.set(v8::undefined(scope).into());
    let Some(fork_id) = fork_id_from_this(scope, this) else {
        return;
    };
    if let Ok(mut map) = FORK_CONNECTED.lock() {
        map.insert(fork_id, false);
    }
    if let Ok(mut writers) = FORK_WRITERS.lock() {
        if let Some(writer) = writers.remove(&fork_id) {
            if let Ok(stream) = writer.lock() {
                let _ = stream.shutdown(Shutdown::Both);
            }
        }
    }
    update_child_object_connected(scope, fork_id, false);
    if let Ok(mut q) = FORK_EVENTS.lock() {
        q.push_back(ForkEvent::Disconnect { fork_id });
    }
}

/// When `AMBER_FORK_CHANNEL` is set, connect to the parent and install
/// `process.send` / real `process.on('message')` (other `process.on` events stay no-op).
pub fn install_fork_child_if_needed(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let Ok(addr) = std::env::var(ENV_CHANNEL) else {
        return Ok(());
    };
    if addr.is_empty() {
        return Ok(());
    }
    if FORK_CHILD_INSTALLED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let stream = match TcpStream::connect(addr.trim()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[amber fork] connect to parent failed: {}", e);
            return Ok(());
        }
    };
    let writer_stream = stream.try_clone()?;
    let writer = Arc::new(Mutex::new(writer_stream));
    *FORK_CHILD_WRITER.lock().unwrap() = Some(writer);
    FORK_CHILD_CONNECTED.store(true, Ordering::SeqCst);

    let mut reader = stream;
    thread::spawn(move || loop {
        match read_frame(&mut reader) {
            Ok(Some(buf)) => {
                let text = String::from_utf8_lossy(&buf).to_string();
                if let Some(payload) = decode_message_payload(&text) {
                    if let Ok(mut q) = FORK_CHILD_INCOMING.lock() {
                        q.push_back(payload);
                    }
                }
            }
            Ok(None) | Err(_) => {
                FORK_CHILD_CONNECTED.store(false, Ordering::SeqCst);
                *FORK_CHILD_WRITER.lock().unwrap() = None;
                break;
            }
        }
    });

    let global = context.global(scope);
    let process_key = v8::String::new(scope, "process").unwrap();
    let Some(process_val) = global.get(scope, process_key.into()) else {
        return Ok(());
    };
    let Ok(process_obj) = v8::Local::<v8::Object>::try_from(process_val) else {
        return Ok(());
    };

    let send_fn = v8::Function::new(scope, process_send_callback).unwrap();
    let send_key = v8::String::new(scope, "send").unwrap();
    process_obj.set(scope, send_key.into(), send_fn.into());

    let connected_key = v8::String::new(scope, "connected").unwrap();
    process_obj.set(
        scope,
        connected_key.into(),
        v8::Boolean::new(scope, true).into(),
    );

    // Replace process.on so 'message' is real; other events remain chain-only no-ops.
    let on_fn = v8::Function::new(scope, process_fork_on_callback).unwrap();
    let on_key = v8::String::new(scope, "on").unwrap();
    process_obj.set(scope, on_key.into(), on_fn.into());

    Ok(())
}

fn process_send_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if !FORK_CHILD_CONNECTED.load(Ordering::SeqCst) {
        retval.set(v8::Boolean::new(scope, false).into());
        return;
    }
    let Some(json) = json_stringify(scope, args.get(0)) else {
        let msg = v8::String::new(scope, "process.send: value is not JSON-serializable").unwrap();
        scope.throw_exception(v8::Exception::error(scope, msg));
        return;
    };
    let frame = encode_message_frame(&json);
    let writer = FORK_CHILD_WRITER.lock().ok().and_then(|g| g.clone());
    let Some(writer) = writer else {
        retval.set(v8::Boolean::new(scope, false).into());
        return;
    };
    let ok = if let Ok(mut stream) = writer.lock() {
        write_frame(&mut stream, frame.as_bytes()).is_ok()
    } else {
        false
    };
    retval.set(v8::Boolean::new(scope, ok).into());
}

fn process_fork_on_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    retval.set(this.into());
    let event = crate::nodejs_core::child_process::string_from_v8_value(scope, args.get(0));
    if event != "message" {
        return;
    }
    let listener = args.get(1);
    if !listener.is_function() {
        return;
    }
    let Ok(func) = v8::Local::<v8::Function>::try_from(listener) else {
        return;
    };
    CHILD_MESSAGE_LISTENERS.with(|cell| {
        cell.borrow_mut().push(v8::Global::new(scope, func));
    });
}
