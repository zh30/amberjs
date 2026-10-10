// Node.js net 模块 — narrow Stable server listen/accept (G45) + existing client helpers.
// Server path: real TcpListener::bind, Tokio accept, 'connection' EventEmitter delivery.
// Client connect / Socket helpers remain Preview outside NET_SERVER_CONTRACT.

use anyhow::Result;
use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use super::tcp_async::{get_net_tokio_runtime, sync_write, TCP_MANAGER};

/// Pending accepted connection queued for the V8 pump.
#[derive(Debug, Clone)]
pub struct PendingNetConnection {
    pub server_id: u64,
    pub handle_id: u64,
    pub remote_addr: String,
    pub remote_port: u16,
    pub local_addr: String,
    pub local_port: u16,
    pub family: String,
}

struct NetServerState {
    id: u64,
    listening: Arc<AtomicBool>,
    port: u16,
    host: String,
    bound_port: Arc<AtomicU64>,
}

static NET_SERVER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static ACTIVE_NET_SERVER_STATES: Lazy<Mutex<Vec<Arc<NetServerState>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

static GLOBAL_NET_CONN_SENDER: Lazy<
    RwLock<Option<crossbeam::channel::Sender<PendingNetConnection>>>,
> = Lazy::new(|| RwLock::new(None));
static GLOBAL_NET_CONN_RECEIVER: Lazy<
    RwLock<Option<crossbeam::channel::Receiver<PendingNetConnection>>>,
> = Lazy::new(|| RwLock::new(None));

fn ensure_net_connection_channel() {
    let need_init = GLOBAL_NET_CONN_RECEIVER
        .read()
        .map(|g| g.is_none())
        .unwrap_or(true);
    if !need_init {
        return;
    }
    let (tx, rx) = crossbeam::channel::bounded(1024);
    if let Ok(mut guard) = GLOBAL_NET_CONN_SENDER.write() {
        *guard = Some(tx);
    }
    if let Ok(mut guard) = GLOBAL_NET_CONN_RECEIVER.write() {
        *guard = Some(rx);
    }
}

fn register_net_server_state(state: Arc<NetServerState>) {
    ACTIVE_NET_SERVER_STATES
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(state);
}

fn stop_net_server_state(server_id: u64) {
    let mut states = ACTIVE_NET_SERVER_STATES
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    for state in states.iter() {
        if state.id == server_id {
            state.listening.store(false, Ordering::SeqCst);
        }
    }
    states.retain(|s| s.listening.load(Ordering::SeqCst));
}

fn stop_all_net_server_states() {
    let mut states = ACTIVE_NET_SERVER_STATES
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    for state in states.iter() {
        state.listening.store(false, Ordering::SeqCst);
    }
    states.clear();
}

/// Reset accept registry + pending queue (contract tests).
pub fn reset_net_server_channel() {
    stop_all_net_server_states();
    let (tx, rx) = crossbeam::channel::bounded(1024);
    if let Ok(mut guard) = GLOBAL_NET_CONN_SENDER.write() {
        *guard = Some(tx);
    }
    if let Ok(mut guard) = GLOBAL_NET_CONN_RECEIVER.write() {
        *guard = Some(rx);
    }
}

/// True when at least one `net.Server` is still marked listening.
pub fn has_listening_net_servers() -> bool {
    ACTIVE_NET_SERVER_STATES
        .lock()
        .map(|states| {
            states
                .iter()
                .any(|state| state.listening.load(Ordering::SeqCst))
        })
        .unwrap_or(false)
}

/// True when the accept channel has queued connections.
pub fn has_pending_net_connections() -> bool {
    if let Ok(guard) = GLOBAL_NET_CONN_RECEIVER.read() {
        if let Some(ref rx) = *guard {
            return !rx.is_empty();
        }
    }
    false
}

fn enqueue_pending_connection(conn: PendingNetConnection) {
    if let Ok(guard) = GLOBAL_NET_CONN_SENDER.read() {
        if let Some(ref tx) = *guard {
            let _ = tx.try_send(conn);
        }
    }
}

/// Drain queued accepts and emit `'connection'` on the matching server.
pub fn pump_pending_net_connections_in_scope(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> usize {
    let rx = if let Ok(guard) = GLOBAL_NET_CONN_RECEIVER.read() {
        guard.clone()
    } else {
        None
    };
    let Some(rx) = rx else {
        return 0;
    };

    let global = context.global(scope);
    let registry_key = v8::String::new(scope, "_netServers").unwrap();
    let registry = match global.get(scope, registry_key.into()) {
        Some(val) if val.is_object() => match v8::Local::<v8::Object>::try_from(val) {
            Ok(obj) => obj,
            Err(_) => return 0,
        },
        _ => return 0,
    };

    let mut processed = 0usize;
    while let Ok(pending) = rx.try_recv() {
        let id_key = v8::String::new(scope, &pending.server_id.to_string()).unwrap();
        let Some(server_val) = registry.get(scope, id_key.into()) else {
            continue;
        };
        if server_val.is_undefined() || server_val.is_null() || !server_val.is_object() {
            continue;
        }
        let Ok(server_obj) = v8::Local::<v8::Object>::try_from(server_val) else {
            continue;
        };

        let socket = build_accepted_socket_object(scope, &pending);
        let emit_key = v8::String::new(scope, "emit").unwrap();
        if let Some(emit_val) = server_obj.get(scope, emit_key.into()) {
            if let Ok(emit_func) = v8::Local::<v8::Function>::try_from(emit_val) {
                let event = v8::String::new(scope, "connection").unwrap();
                let _ = emit_func.call(scope, server_obj.into(), &[event.into(), socket.into()]);
            }
        }
        processed += 1;
        if processed >= 256 {
            break;
        }
    }
    processed
}

fn build_accepted_socket_object<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    pending: &PendingNetConnection,
) -> v8::Local<'a, v8::Object> {
    let socket_obj = v8::Object::new(scope);

    let set_str =
        |scope: &mut v8::PinScope<'a, '_>, obj: v8::Local<'a, v8::Object>, key: &str, val: &str| {
            let k = v8::String::new(scope, key).unwrap();
            let v = v8::String::new(scope, val).unwrap();
            obj.set(scope, k.into(), v.into());
        };
    let set_i32 =
        |scope: &mut v8::PinScope<'a, '_>, obj: v8::Local<'a, v8::Object>, key: &str, val: i32| {
            let k = v8::String::new(scope, key).unwrap();
            let v = v8::Integer::new(scope, val);
            obj.set(scope, k.into(), v.into());
        };

    set_str(scope, socket_obj, "remoteAddress", &pending.remote_addr);
    set_i32(scope, socket_obj, "remotePort", pending.remote_port as i32);
    set_str(scope, socket_obj, "localAddress", &pending.local_addr);
    set_i32(scope, socket_obj, "localPort", pending.local_port as i32);
    set_str(scope, socket_obj, "remoteFamily", &pending.family);

    let connecting_key = v8::String::new(scope, "connecting").unwrap();
    socket_obj.set(
        scope,
        connecting_key.into(),
        v8::Boolean::new(scope, false).into(),
    );

    let handle_id_key = v8::String::new(scope, "_handleId").unwrap();
    let handle_id_val = v8::Integer::new(scope, pending.handle_id as i32);
    socket_obj.set(scope, handle_id_key.into(), handle_id_val.into());

    attach_socket_methods(scope, socket_obj);
    socket_obj
}

fn attach_socket_methods<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    socket_obj: v8::Local<'a, v8::Object>,
) {
    let write_func = v8::FunctionTemplate::new(scope, socket_write_callback);
    let write_instance = write_func.get_function(scope).unwrap();
    let write_key = v8::String::new(scope, "write").unwrap();
    socket_obj.set(scope, write_key.into(), write_instance.into());

    let end_func = v8::FunctionTemplate::new(scope, socket_end_callback);
    let end_instance = end_func.get_function(scope).unwrap();
    let end_key = v8::String::new(scope, "end").unwrap();
    socket_obj.set(scope, end_key.into(), end_instance.into());

    let on_func = v8::FunctionTemplate::new(scope, socket_on_callback);
    let on_instance = on_func.get_function(scope).unwrap();
    let on_key = v8::String::new(scope, "on").unwrap();
    socket_obj.set(scope, on_key.into(), on_instance.into());

    let once_func = v8::FunctionTemplate::new(scope, socket_once_callback);
    let once_instance = once_func.get_function(scope).unwrap();
    let once_key = v8::String::new(scope, "once").unwrap();
    socket_obj.set(scope, once_key.into(), once_instance.into());

    let emit_func = v8::FunctionTemplate::new(scope, socket_emit_callback);
    let emit_instance = emit_func.get_function(scope).unwrap();
    let emit_key = v8::String::new(scope, "emit").unwrap();
    socket_obj.set(scope, emit_key.into(), emit_instance.into());

    let destroy_func = v8::FunctionTemplate::new(scope, socket_destroy_callback);
    let destroy_instance = destroy_func.get_function(scope).unwrap();
    let destroy_key = v8::String::new(scope, "destroy").unwrap();
    socket_obj.set(scope, destroy_key.into(), destroy_instance.into());

    let set_timeout_func = v8::FunctionTemplate::new(scope, socket_set_timeout_callback);
    let set_timeout_instance = set_timeout_func.get_function(scope).unwrap();
    let set_timeout_key = v8::String::new(scope, "setTimeout").unwrap();
    socket_obj.set(scope, set_timeout_key.into(), set_timeout_instance.into());

    let set_encoding_func = v8::FunctionTemplate::new(scope, socket_set_encoding_callback);
    let set_encoding_instance = set_encoding_func.get_function(scope).unwrap();
    let set_encoding_key = v8::String::new(scope, "setEncoding").unwrap();
    socket_obj.set(scope, set_encoding_key.into(), set_encoding_instance.into());

    let pause_func = v8::FunctionTemplate::new(scope, socket_pause_callback);
    let pause_instance = pause_func.get_function(scope).unwrap();
    let pause_key = v8::String::new(scope, "pause").unwrap();
    socket_obj.set(scope, pause_key.into(), pause_instance.into());

    let resume_func = v8::FunctionTemplate::new(scope, socket_resume_callback);
    let resume_instance = resume_func.get_function(scope).unwrap();
    let resume_key = v8::String::new(scope, "resume").unwrap();
    socket_obj.set(scope, resume_key.into(), resume_instance.into());

    let read_func = v8::FunctionTemplate::new(scope, socket_read_callback);
    let read_instance = read_func.get_function(scope).unwrap();
    let read_key = v8::String::new(scope, "read").unwrap();
    socket_obj.set(scope, read_key.into(), read_instance.into());
}

/// 设置 net API
pub fn setup_net_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    ensure_net_connection_channel();

    let net_obj = v8::Object::new(scope);

    let connect_func = v8::FunctionTemplate::new(scope, net_connect_callback);
    let connect_instance = connect_func.get_function(scope).unwrap();
    let connect_key = v8::String::new(scope, "connect").unwrap();
    net_obj.set(scope, connect_key.into(), connect_instance.into());

    let create_connection_func = v8::FunctionTemplate::new(scope, net_connect_callback);
    let create_connection_instance = create_connection_func.get_function(scope).unwrap();
    let create_connection_key = v8::String::new(scope, "createConnection").unwrap();
    net_obj.set(
        scope,
        create_connection_key.into(),
        create_connection_instance.into(),
    );

    let create_server_func = v8::FunctionTemplate::new(scope, net_create_server_native_callback);
    let create_server_instance = create_server_func.get_function(scope).unwrap();
    let create_server_key = v8::String::new(scope, "createServer").unwrap();
    net_obj.set(
        scope,
        create_server_key.into(),
        create_server_instance.into(),
    );

    let server_func = v8::FunctionTemplate::new(scope, net_create_server_native_callback);
    let server_instance = server_func.get_function(scope).unwrap();
    let server_key = v8::String::new(scope, "Server").unwrap();
    net_obj.set(scope, server_key.into(), server_instance.into());

    let is_ip_func = v8::FunctionTemplate::new(scope, net_is_ip_callback);
    let is_ip_instance = is_ip_func.get_function(scope).unwrap();
    let is_ip_key = v8::String::new(scope, "isIP").unwrap();
    net_obj.set(scope, is_ip_key.into(), is_ip_instance.into());

    let is_ipv4_func = v8::FunctionTemplate::new(scope, net_is_ipv4_callback);
    let is_ipv4_instance = is_ipv4_func.get_function(scope).unwrap();
    let is_ipv4_key = v8::String::new(scope, "isIPv4").unwrap();
    net_obj.set(scope, is_ipv4_key.into(), is_ipv4_instance.into());

    let is_ipv6_func = v8::FunctionTemplate::new(scope, net_is_ipv6_callback);
    let is_ipv6_instance = is_ipv6_func.get_function(scope).unwrap();
    let is_ipv6_key = v8::String::new(scope, "isIPv6").unwrap();
    net_obj.set(scope, is_ipv6_key.into(), is_ipv6_instance.into());

    let global = context.global(scope);
    let net_key = v8::String::new(scope, "net").unwrap();
    global.set(scope, net_key.into(), net_obj.into());

    // EventEmitter-backed Server + createServer wrap (mirrors http G37 bootstrap).
    let bootstrap = r#"
        (function() {
            const net = globalThis.net;
            if (!net || net.__amberNetServerBootstrapped) return;
            const EE = globalThis.EventEmitter || function() {};
            const proto = (EE.prototype || Object.prototype);

            if (!globalThis._netServers) {
                globalThis._netServers = Object.create(null);
            }

            function Server(options, connectionListener) {
                if (typeof EE === 'function') EE.call(this);
                this._events = Object.create(null);
                this._eventsCount = 0;
                this.listening = false;
                if (typeof options === 'function') {
                    connectionListener = options;
                    options = undefined;
                }
                if (typeof connectionListener === 'function') {
                    this.on('connection', connectionListener);
                }
            }
            Server.prototype = Object.create(proto);
            Server.prototype.constructor = Server;
            Server.prototype.ref = function() { return this; };
            Server.prototype.unref = function() { return this; };
            Server.prototype.getConnections = function(cb) {
                if (typeof cb === 'function') cb(null, 0);
                return this;
            };
            net.Server = Server;

            const _nativeCreateServer = net.createServer;
            net.createServer = function(options, connectionListener) {
                const server = _nativeCreateServer.call(net, options, connectionListener);
                Object.setPrototypeOf(server, Server.prototype);
                server._events = Object.create(null);
                server._eventsCount = 0;
                if (typeof options === 'function') {
                    connectionListener = options;
                }
                if (typeof connectionListener === 'function') {
                    server.on('connection', connectionListener);
                }
                const id = server._netServerId;
                if (id != null) {
                    globalThis._netServers[String(id)] = server;
                }
                return server;
            };
            net.__amberNetServerBootstrapped = true;
        })();
    "#;
    let code = v8::String::new(scope, bootstrap).unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let _ = script.run(scope);

    Ok(())
}

/// Native createServer — builds server object with listen/close/address.
fn net_create_server_native_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    ensure_net_connection_channel();

    let server_obj = v8::Object::new(scope);
    let server_id = NET_SERVER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

    let id_key = v8::String::new(scope, "_netServerId").unwrap();
    let id_val = v8::Integer::new(scope, server_id as i32);
    server_obj.set(scope, id_key.into(), id_val.into());

    let listen_func = v8::FunctionTemplate::new(scope, server_listen_callback);
    let listen_instance = listen_func.get_function(scope).unwrap();
    let listen_key = v8::String::new(scope, "listen").unwrap();
    server_obj.set(scope, listen_key.into(), listen_instance.into());

    let close_func = v8::FunctionTemplate::new(scope, server_close_callback);
    let close_instance = close_func.get_function(scope).unwrap();
    let close_key = v8::String::new(scope, "close").unwrap();
    server_obj.set(scope, close_key.into(), close_instance.into());

    let address_func = v8::FunctionTemplate::new(scope, server_address_callback);
    let address_instance = address_func.get_function(scope).unwrap();
    let address_key = v8::String::new(scope, "address").unwrap();
    server_obj.set(scope, address_key.into(), address_instance.into());

    let listening_key = v8::String::new(scope, "listening").unwrap();
    let listening_val = v8::Boolean::new(scope, false);
    server_obj.set(scope, listening_key.into(), listening_val.into());

    // Register in _netServers early so pump can find the object after JS wrap.
    let context = scope.get_current_context();
    let global = context.global(scope);
    let registry_key = v8::String::new(scope, "_netServers").unwrap();
    let registry = if let Some(existing) = global.get(scope, registry_key.into()) {
        if existing.is_object() {
            v8::Local::<v8::Object>::try_from(existing).unwrap_or_else(|_| v8::Object::new(scope))
        } else {
            v8::Object::new(scope)
        }
    } else {
        v8::Object::new(scope)
    };
    let id_str = v8::String::new(scope, &server_id.to_string()).unwrap();
    registry.set(scope, id_str.into(), server_obj.into());
    global.set(scope, registry_key.into(), registry.into());

    // Optional connectionListener for Server ctor path (createServer wrap also registers).
    let arg0 = args.get(0);
    let arg1 = args.get(1);
    let connection_listener = if arg0.is_function() {
        arg0
    } else if arg1.is_function() {
        arg1
    } else {
        v8::undefined(scope).into()
    };
    if connection_listener.is_function() {
        let handler_key = v8::String::new(scope, "_connectionListener").unwrap();
        server_obj.set(scope, handler_key.into(), connection_listener);
    }

    retval.set(server_obj.into());
}

fn server_listen_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    ensure_net_connection_channel();

    // Parse: listen(port[, host][, cb]) | listen({port,host}[, cb]) | listen(cb)
    let (port, host, callback) = {
        let arg0 = args.get(0);
        if arg0.is_function() {
            (0u16, "0.0.0.0".to_string(), arg0)
        } else if arg0.is_object() && !arg0.is_null() {
            let opts = arg0.to_object(scope).unwrap();
            let port_key = v8::String::new(scope, "port").unwrap();
            let host_key = v8::String::new(scope, "host").unwrap();
            let p = opts
                .get(scope, port_key.into())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as u16)
                .unwrap_or(0);
            let h = opts
                .get(scope, host_key.into())
                .and_then(|v| v.to_string(scope))
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "0.0.0.0".to_string());
            let cb = if args.get(1).is_function() {
                args.get(1)
            } else {
                v8::undefined(scope).into()
            };
            (p, h, cb)
        } else if arg0.is_undefined() || arg0.is_null() {
            (0u16, "0.0.0.0".to_string(), args.get(1))
        } else {
            let p = arg0
                .to_integer(scope)
                .map(|i| i.value() as u16)
                .unwrap_or(0);
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

    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Network,
        crate::permissions::PermissionAction::Listen,
        crate::permissions::ResourceId::Url(format!("tcp://{}:{}", host, port)),
    ) {
        let listening_key = v8::String::new(scope, "listening").unwrap();
        let listening_val = v8::Boolean::new(scope, false);
        this.set(scope, listening_key.into(), listening_val.into());
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::type_error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }

    let server_id = {
        let id_key = v8::String::new(scope, "_netServerId").unwrap();
        this.get(scope, id_key.into())
            .and_then(|v| v.to_integer(scope))
            .map(|i| i.value() as u64)
            .unwrap_or(0)
    };
    if server_id == 0 {
        let msg = v8::String::new(scope, "net.Server missing _netServerId").unwrap();
        let err = v8::Exception::error(scope, msg);
        scope.throw_exception(err.into());
        return;
    }

    // Ensure registry points at this (post-prototype wrap) object.
    let context = scope.get_current_context();
    let global = context.global(scope);
    let registry_key = v8::String::new(scope, "_netServers").unwrap();
    let registry = if let Some(existing) = global.get(scope, registry_key.into()) {
        if existing.is_object() {
            v8::Local::<v8::Object>::try_from(existing).unwrap_or_else(|_| v8::Object::new(scope))
        } else {
            v8::Object::new(scope)
        }
    } else {
        v8::Object::new(scope)
    };
    let id_str = v8::String::new(scope, &server_id.to_string()).unwrap();
    registry.set(scope, id_str.into(), this.into());
    global.set(scope, registry_key.into(), registry.into());

    let server_state = Arc::new(NetServerState {
        id: server_id,
        listening: Arc::new(AtomicBool::new(true)),
        port,
        host: host.clone(),
        bound_port: Arc::new(AtomicU64::new(port as u64)),
    });
    register_net_server_state(server_state.clone());

    let state_clone = server_state.clone();
    thread::spawn(move || {
        run_net_server(state_clone);
    });
    thread::sleep(Duration::from_millis(5));

    let advertised_port = {
        let bound = server_state.bound_port.load(Ordering::SeqCst) as u16;
        if bound != 0 || port == 0 {
            // For port 0, bound may still be 0 if bind failed; check listening.
            let b = server_state.bound_port.load(Ordering::SeqCst) as u16;
            if b != 0 {
                b
            } else {
                port
            }
        } else {
            port
        }
    };

    if !server_state.listening.load(Ordering::SeqCst) {
        let listening_key = v8::String::new(scope, "listening").unwrap();
        this.set(
            scope,
            listening_key.into(),
            v8::Boolean::new(scope, false).into(),
        );
        let msg = v8::String::new(
            scope,
            &format!("listen EADDRINUSE or bind failed on {}:{}", host, port),
        )
        .unwrap();
        let err = v8::Exception::error(scope, msg);
        scope.throw_exception(err.into());
        return;
    }

    let listening_key = v8::String::new(scope, "listening").unwrap();
    this.set(
        scope,
        listening_key.into(),
        v8::Boolean::new(scope, true).into(),
    );

    let server_port_key = v8::String::new(scope, "_serverPort").unwrap();
    let server_port_val = v8::Integer::new(scope, advertised_port as i32);
    this.set(scope, server_port_key.into(), server_port_val.into());

    let server_host_key = v8::String::new(scope, "_serverHost").unwrap();
    let server_host_val = v8::String::new(scope, &host).unwrap();
    this.set(scope, server_host_key.into(), server_host_val.into());

    // Also store address object for older callers that read .address property.
    let address_obj = v8::Object::new(scope);
    let family = if host.contains(':') && !host.contains('.') {
        "IPv6"
    } else {
        "IPv4"
    };
    {
        let k = v8::String::new(scope, "address").unwrap();
        let v = v8::String::new(scope, &host).unwrap();
        address_obj.set(scope, k.into(), v.into());
        let k = v8::String::new(scope, "port").unwrap();
        let v = v8::Integer::new(scope, advertised_port as i32);
        address_obj.set(scope, k.into(), v.into());
        let k = v8::String::new(scope, "family").unwrap();
        let v = v8::String::new(scope, family).unwrap();
        address_obj.set(scope, k.into(), v.into());
    }
    let address_prop = v8::String::new(scope, "address").unwrap();
    // Keep address() method; store snapshot under _addressSnapshot.
    let snap_key = v8::String::new(scope, "_addressSnapshot").unwrap();
    this.set(scope, snap_key.into(), address_obj.into());
    let _ = address_prop;

    if callback.is_function() {
        let once_key = v8::String::new(scope, "once").unwrap();
        if let Some(once_val) = this.get(scope, once_key.into()) {
            if let Ok(once_func) = v8::Local::<v8::Function>::try_from(once_val) {
                let listening_event = v8::String::new(scope, "listening").unwrap();
                let _ = once_func.call(scope, this.into(), &[listening_event.into(), callback]);
            }
        }
    }

    let emit_key = v8::String::new(scope, "emit").unwrap();
    if let Some(emit_val) = this.get(scope, emit_key.into()) {
        if let Ok(emit_func) = v8::Local::<v8::Function>::try_from(emit_val) {
            let listening_event = v8::String::new(scope, "listening").unwrap();
            let _ = emit_func.call(scope, this.into(), &[listening_event.into()]);
        }
    }

    retval.set(this.into());
}

fn run_net_server(server_state: Arc<NetServerState>) {
    let addr = format!("{}:{}", server_state.host, server_state.port);
    if !server_state.listening.load(Ordering::SeqCst) {
        return;
    }

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
            eprintln!("[Amber] net.Server failed to bind {}: {}", addr, e);
            server_state.listening.store(false, Ordering::SeqCst);
            return;
        }
    };

    if let Err(e) = std_listener.set_nonblocking(true) {
        eprintln!("[Amber] net.Server set_nonblocking failed: {}", e);
        server_state.listening.store(false, Ordering::SeqCst);
        return;
    }

    let rt = get_net_tokio_runtime();
    let state_clone = server_state.clone();
    let server_id = server_state.id;

    rt.spawn(async move {
        let tokio_listener = match tokio::net::TcpListener::from_std(std_listener) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[Amber] net.Server tokio listener failed: {}", e);
                state_clone.listening.store(false, Ordering::SeqCst);
                return;
            }
        };

        while state_clone.listening.load(Ordering::SeqCst) {
            tokio::select! {
                res = tokio_listener.accept() => {
                    match res {
                        Ok((stream, _peer)) => {
                            let _ = stream.set_nodelay(true);
                            let handle = TCP_MANAGER.create_connection();
                            let handle_id = handle.id;
                            if let Err(e) = handle.adopt_stream(stream) {
                                eprintln!("[Amber] net.Server adopt_stream failed: {}", e);
                                TCP_MANAGER.remove_connection(handle_id);
                                continue;
                            }
                            let info = handle.get_info();
                            enqueue_pending_connection(PendingNetConnection {
                                server_id,
                                handle_id,
                                remote_addr: info.remote_addr,
                                remote_port: info.remote_port,
                                local_addr: info.local_addr,
                                local_port: info.local_port,
                                family: info.family,
                            });
                            crate::nodejs_core::http::wake_http_dispatch_thread();
                        }
                        Err(e) => {
                            if !state_clone.listening.load(Ordering::SeqCst) {
                                break;
                            }
                            eprintln!("[Amber] net.Server accept failed: {}", e);
                            tokio::time::sleep(Duration::from_millis(5)).await;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            }
        }
    });
}

fn server_close_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let server_id = {
        let id_key = v8::String::new(scope, "_netServerId").unwrap();
        this.get(scope, id_key.into())
            .and_then(|v| v.to_integer(scope))
            .map(|i| i.value() as u64)
            .unwrap_or(0)
    };
    if server_id != 0 {
        stop_net_server_state(server_id);
    }

    let listening_key = v8::String::new(scope, "listening").unwrap();
    this.set(
        scope,
        listening_key.into(),
        v8::Boolean::new(scope, false).into(),
    );

    let emit_key = v8::String::new(scope, "emit").unwrap();
    if let Some(emit_val) = this.get(scope, emit_key.into()) {
        if let Ok(emit_func) = v8::Local::<v8::Function>::try_from(emit_val) {
            let close_event = v8::String::new(scope, "close").unwrap();
            let _ = emit_func.call(scope, this.into(), &[close_event.into()]);
        }
    }

    retval.set(this.into());
}

fn server_address_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let listening_key = v8::String::new(scope, "listening").unwrap();
    let is_listening = this
        .get(scope, listening_key.into())
        .map(|v| v.boolean_value(scope))
        .unwrap_or(false);
    if !is_listening {
        retval.set(v8::null(scope).into());
        return;
    }

    let port = {
        let port_key = v8::String::new(scope, "_serverPort").unwrap();
        this.get(scope, port_key.into())
            .and_then(|v| v.to_integer(scope))
            .map(|v| v.value() as i32)
            .unwrap_or(0)
    };
    let host = {
        let host_key = v8::String::new(scope, "_serverHost").unwrap();
        this.get(scope, host_key.into())
            .and_then(|v| v.to_string(scope))
            .map(|v| v.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "0.0.0.0".to_string())
    };
    let family = if host.contains(':') && !host.contains('.') {
        "IPv6"
    } else {
        "IPv4"
    };

    let obj = v8::Object::new(scope);
    let port_key = v8::String::new(scope, "port").unwrap();
    obj.set(scope, port_key.into(), v8::Integer::new(scope, port).into());
    let address_key = v8::String::new(scope, "address").unwrap();
    let address_val = v8::String::new(scope, &host).unwrap();
    obj.set(scope, address_key.into(), address_val.into());
    let family_key = v8::String::new(scope, "family").unwrap();
    let family_val = v8::String::new(scope, family).unwrap();
    obj.set(scope, family_key.into(), family_val.into());
    retval.set(obj.into());
}

/// net.connect() / createConnection() — Preview client path (unchanged shape).
fn net_connect_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let options = args.get(0);

    let port = extract_integer_option(scope, &options, "port", 0);
    let host = extract_string_option(scope, &options, "host", "localhost");
    let local_port = extract_integer_option(scope, &options, "localPort", 0);
    let local_address = extract_string_option(scope, &options, "localAddress", "0.0.0.0");
    let connect_timeout = extract_integer_option(scope, &options, "connectTimeout", 0);

    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Network,
        crate::permissions::PermissionAction::Connect,
        crate::permissions::ResourceId::Url(format!("tcp://{}:{}", host, port)),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }

    let tcp_handle = TCP_MANAGER.create_connection();
    let handle_id = tcp_handle.id;

    let timeout_secs = if connect_timeout > 0 {
        connect_timeout as u64
    } else {
        10
    };
    let connect_result = tcp_handle.connect(&host, port as u16);
    let (is_connected, remote_addr, remote_family, local_addr_val) = {
        let rt = get_net_tokio_runtime();
        let timeout_duration = std::time::Duration::from_secs(timeout_secs);
        let result =
            rt.block_on(async { tokio::time::timeout(timeout_duration, connect_result).await });
        match result {
            Ok(Ok(())) => {
                let info = tcp_handle.get_info();
                (
                    true,
                    info.remote_addr.clone(),
                    info.family.clone(),
                    info.local_addr.clone(),
                )
            }
            _ => (
                false,
                host.clone(),
                if host.contains(':') {
                    "IPv6".to_string()
                } else {
                    "IPv4".to_string()
                },
                local_address,
            ),
        }
    };

    let socket_obj = v8::Object::new(scope);

    let port_key = v8::String::new(scope, "remotePort").unwrap();
    let port_val = v8::Integer::new(scope, port as i32);
    socket_obj.set(scope, port_key.into(), port_val.into());

    let host_key = v8::String::new(scope, "remoteAddress").unwrap();
    let host_val = v8::String::new(scope, &remote_addr).unwrap();
    socket_obj.set(scope, host_key.into(), host_val.into());

    let local_port_key = v8::String::new(scope, "localPort").unwrap();
    let local_port_val = v8::Integer::new(scope, local_port as i32);
    socket_obj.set(scope, local_port_key.into(), local_port_val.into());

    let local_addr_key = v8::String::new(scope, "localAddress").unwrap();
    let local_addr_v8_val = v8::String::new(scope, &local_addr_val).unwrap();
    socket_obj.set(scope, local_addr_key.into(), local_addr_v8_val.into());

    let family_key = v8::String::new(scope, "remoteFamily").unwrap();
    let family_val = v8::String::new(scope, &remote_family).unwrap();
    socket_obj.set(scope, family_key.into(), family_val.into());

    let connecting_key = v8::String::new(scope, "connecting").unwrap();
    let connecting_val = v8::Boolean::new(scope, !is_connected);
    socket_obj.set(scope, connecting_key.into(), connecting_val.into());

    let connect_key = v8::String::new(scope, "connect").unwrap();
    let connect_val =
        v8::String::new(scope, if is_connected { "open" } else { "opening" }).unwrap();
    socket_obj.set(scope, connect_key.into(), connect_val.into());

    let handle_id_key = v8::String::new(scope, "_handleId").unwrap();
    let handle_id_val = v8::Integer::new(scope, handle_id as i32);
    socket_obj.set(scope, handle_id_key.into(), handle_id_val.into());

    attach_socket_methods(scope, socket_obj);

    let options_key = v8::String::new(scope, "_connectOptions").unwrap();
    socket_obj.set(scope, options_key.into(), options);

    retval.set(socket_obj.into());
}

fn net_is_ip_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let ip: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let result = if is_valid_ipv4(&ip) {
        4
    } else if is_valid_ipv6(&ip) {
        6
    } else {
        0
    };
    retval.set(v8::Integer::new(scope, result).into());
}

fn net_is_ipv4_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let ip: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    retval.set(v8::Boolean::new(scope, is_valid_ipv4(&ip)).into());
}

fn net_is_ipv6_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let ip: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    retval.set(v8::Boolean::new(scope, is_valid_ipv6(&ip)).into());
}

fn is_valid_ipv4(ip: &str) -> bool {
    if ip.split('.').count() != 4 {
        return false;
    }
    ip.split('.').all(|part| part.parse::<u8>().is_ok())
}

fn is_valid_ipv6(ip: &str) -> bool {
    if !ip.contains(':') {
        return false;
    }
    ip.parse::<std::net::Ipv6Addr>().is_ok()
}

fn extract_integer_option(
    scope: &mut v8::PinScope,
    options: &v8::Local<v8::Value>,
    key: &str,
    default: i32,
) -> i32 {
    if options.is_undefined() || options.is_null() {
        return default;
    }
    if options.is_number() {
        return options
            .to_int32(scope)
            .map(|i| i.value())
            .unwrap_or(default);
    }
    if let Ok(obj) = v8::Local::<v8::Object>::try_from(*options) {
        let key_str = v8::String::new(scope, key).unwrap();
        if let Some(val) = obj.get(scope, key_str.into()) {
            if val.is_number() {
                return val.to_int32(scope).unwrap().value() as i32;
            }
        }
    }
    default
}

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
        let key_str = v8::String::new(scope, key).unwrap();
        if let Some(val) = obj.get(scope, key_str.into()) {
            if let Some(s) = val.to_string(scope) {
                return s.to_rust_string_lossy(scope);
            }
        }
    }
    default.to_string()
}

fn socket_write_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let data_val = args.get(0);

    let data_str = data_val
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let data_bytes = data_str.as_bytes();

    let handle_id_key = v8::String::new(scope, "_handleId").unwrap();
    if let Some(handle_id_val) = this.get(scope, handle_id_key.into()) {
        if let Some(id_num) = handle_id_val.to_int32(scope) {
            let handle_id = id_num.value() as u64;
            if let Some(handle) = TCP_MANAGER.get_connection(handle_id) {
                if handle.is_connected() {
                    let _ = sync_write(&handle, data_bytes);
                }
            }
        }
    }

    retval.set(v8::Boolean::new(scope, true).into());
}

fn socket_end_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    // Optional final chunk, then half-close write side by dropping stream write.
    if args.length() > 0 && !args.get(0).is_undefined() {
        let data_val = args.get(0);
        let data_str = data_val
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default();
        let handle_id_key = v8::String::new(scope, "_handleId").unwrap();
        if let Some(handle_id_val) = this.get(scope, handle_id_key.into()) {
            if let Some(id_num) = handle_id_val.to_int32(scope) {
                let handle_id = id_num.value() as u64;
                if let Some(handle) = TCP_MANAGER.get_connection(handle_id) {
                    if handle.is_connected() {
                        let _ = sync_write(&handle, data_str.as_bytes());
                    }
                }
            }
        }
    }
    retval.set(this.into());
}

fn socket_on_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let event: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let listener_val = args.get(1);
    let events_key = v8::String::new(scope, "_events").unwrap();

    let events_obj = if let Some(events) = this.get(scope, events_key.into()) {
        if events.is_object() {
            v8::Local::<v8::Object>::try_from(events).unwrap_or_else(|_| v8::Object::new(scope))
        } else {
            v8::Object::new(scope)
        }
    } else {
        v8::Object::new(scope)
    };

    let event_key = v8::String::new(scope, &event).unwrap();
    events_obj.set(scope, event_key.into(), listener_val);
    this.set(scope, events_key.into(), events_obj.into());

    if event == "connect" {
        let connecting_key = v8::String::new(scope, "connecting").unwrap();
        let is_connecting = if let Some(conn_val) = this.get(scope, connecting_key.into()) {
            conn_val.to_boolean(scope).boolean_value(scope)
        } else {
            false
        };
        if !is_connecting {
            if let Some(events) = this.get(scope, events_key.into()) {
                if let Ok(events_obj) = v8::Local::<v8::Object>::try_from(events) {
                    let connect_key = v8::String::new(scope, "connect").unwrap();
                    let callback_val = events_obj.get(scope, connect_key.into());
                    if let Some(cb) = callback_val {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(cb) {
                            let this_val: v8::Local<v8::Value> = this.into();
                            func.call(scope, this_val.into(), &[]);
                        }
                    }
                }
            }
        }
    }

    retval.set(this.into());
}

fn socket_once_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    // Minimal once: store as on for now (Preview client path).
    socket_on_callback(scope, args, retval);
}

fn socket_emit_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let event: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let events_key = v8::String::new(scope, "_events").unwrap();
    let mut called = false;
    if let Some(events) = this.get(scope, events_key.into()) {
        if let Ok(events_obj) = v8::Local::<v8::Object>::try_from(events) {
            let event_key = v8::String::new(scope, &event).unwrap();
            if let Some(cb) = events_obj.get(scope, event_key.into()) {
                if let Ok(func) = v8::Local::<v8::Function>::try_from(cb) {
                    let mut call_args: Vec<v8::Local<v8::Value>> = Vec::new();
                    for i in 1..args.length() {
                        call_args.push(args.get(i));
                    }
                    let _ = func.call(scope, this.into(), &call_args);
                    called = true;
                }
            }
        }
    }
    retval.set(v8::Boolean::new(scope, called).into());
}

fn socket_destroy_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();

    let handle_id_key = v8::String::new(scope, "_handleId").unwrap();
    if let Some(handle_id_val) = this.get(scope, handle_id_key.into()) {
        if let Some(id_num) = handle_id_val.to_int32(scope) {
            let handle_id = id_num.value() as u64;
            if let Some(handle) = TCP_MANAGER.get_connection(handle_id) {
                handle.close();
                TCP_MANAGER.remove_connection(handle_id);
            }
        }
    }

    let connecting_key = v8::String::new(scope, "connecting").unwrap();
    this.set(
        scope,
        connecting_key.into(),
        v8::Boolean::new(scope, false).into(),
    );
    let connect_key = v8::String::new(scope, "connect").unwrap();
    let connect_val = v8::String::new(scope, "closed").unwrap();
    this.set(scope, connect_key.into(), connect_val.into());
    retval.set(v8::Boolean::new(scope, true).into());
}

fn socket_set_timeout_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let timeout = args.get(0).to_int32(scope).map(|i| i.value()).unwrap_or(0);
    let timeout_key = v8::String::new(scope, "timeout").unwrap();
    let timeout_val = v8::Integer::new(scope, timeout as i32);
    this.set(scope, timeout_key.into(), timeout_val.into());
    retval.set(this.into());
}

fn socket_set_encoding_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    retval.set(this.into());
}

fn socket_pause_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    retval.set(this.into());
}

fn socket_resume_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    retval.set(this.into());
}

fn socket_read_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();

    // Prefer live buffer from TcpConnectionHandle when present.
    let handle_id_key = v8::String::new(scope, "_handleId").unwrap();
    if let Some(handle_id_val) = this.get(scope, handle_id_key.into()) {
        if let Some(id_num) = handle_id_val.to_int32(scope) {
            let handle_id = id_num.value() as u64;
            if let Some(handle) = TCP_MANAGER.get_connection(handle_id) {
                let data = handle.consume_buffer();
                if !data.is_empty() {
                    let s = String::from_utf8_lossy(&data);
                    let val = v8::String::new(scope, &s).unwrap();
                    retval.set(val.into());
                    return;
                }
            }
        }
    }

    let data_key = v8::String::new(scope, "_cachedData").unwrap();
    let cached_data = this.get(scope, data_key.into());
    if let Some(data) = cached_data {
        if !data.is_null() && !data.is_undefined() {
            let buf = v8::Local::<v8::Value>::try_from(data).unwrap();
            let null_val = v8::null(scope).into();
            this.set(scope, data_key.into(), null_val);
            retval.set(buf);
            return;
        }
    }

    retval.set(v8::null(scope).into());
}
