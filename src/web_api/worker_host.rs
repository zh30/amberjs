//! Multi-isolate WorkerHost.
//!
//! Each worker runs on its own OS thread with an independent V8 isolate.
//! Supports bi-directional postMessage communication, workerData,
//! error and exit events, and unref/ref lifecycle control.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

static NEXT_WORKER_ID: AtomicU32 = AtomicU32::new(1);
static HOST: Lazy<Mutex<WorkerHost>> = Lazy::new(|| Mutex::new(WorkerHost::new()));

thread_local! {
    static WORKER_THREAD_CHANNEL: RefCell<Option<(u32, Sender<ParentMessage>)>> = const { RefCell::new(None) };
}

#[derive(Debug)]
pub enum WorkerMessage {
    PostMessage(String),
    Terminate,
}

#[derive(Debug, Clone)]
pub enum ParentMessage {
    Message { worker_id: u32, payload: String },
    Error { worker_id: u32, error: String },
    Exit { worker_id: u32, exit_code: i32 },
}

pub struct WorkerHandle {
    pub id: u32,
    pub script_url: String,
    pub is_refed: bool,
    pub is_terminated: bool,
    tx: Sender<WorkerMessage>,
    join: Option<JoinHandle<()>>,
}

pub struct WorkerHost {
    workers: HashMap<u32, WorkerHandle>,
    parent_tx: Sender<ParentMessage>,
    parent_rx: Receiver<ParentMessage>,
    pending_messages: Vec<ParentMessage>,
}

impl WorkerHost {
    pub fn new() -> Self {
        let (parent_tx, parent_rx) = mpsc::channel();
        Self {
            workers: HashMap::new(),
            parent_tx,
            parent_rx,
            pending_messages: Vec::new(),
        }
    }

    pub fn spawn(
        script_source: String,
        script_url: String,
        worker_data_json: Option<String>,
    ) -> Result<u32> {
        let id = NEXT_WORKER_ID.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel::<WorkerMessage>();
        let worker_id = id;
        let url_clone = script_url.clone();

        let parent_tx_clone = {
            let host = HOST.lock().unwrap();
            host.parent_tx.clone()
        };

        let join = thread::Builder::new()
            .name(format!("amber-worker-{}", id))
            .spawn(move || {
                if let Err(err) = run_worker_thread(
                    worker_id,
                    script_source,
                    worker_data_json,
                    parent_tx_clone,
                    rx,
                ) {
                    eprintln!("[WorkerHost {}] fatal: {}", worker_id, err);
                }
            })
            .map_err(|e| anyhow!("failed to spawn worker thread: {}", e))?;

        let mut host = HOST.lock().unwrap();
        host.workers.insert(
            id,
            WorkerHandle {
                id,
                script_url: url_clone,
                is_refed: true,
                is_terminated: false,
                tx,
                join: Some(join),
            },
        );
        Ok(id)
    }

    pub fn post_message(id: u32, payload_json: String) -> Result<()> {
        let host = HOST.lock().unwrap();
        let worker = host
            .workers
            .get(&id)
            .ok_or_else(|| anyhow!("unknown worker {}", id))?;
        if worker.is_terminated {
            return Err(anyhow!("Worker {} has already terminated", id));
        }
        worker
            .tx
            .send(WorkerMessage::PostMessage(payload_json))
            .map_err(|e| anyhow!("postMessage failed: {}", e))
    }

    pub fn terminate(id: u32) -> Result<()> {
        let mut handle_to_join = None;
        {
            let mut host = HOST.lock().unwrap();
            if let Some(worker) = host.workers.get_mut(&id) {
                worker.is_terminated = true;
                let _ = worker.tx.send(WorkerMessage::Terminate);
                handle_to_join = worker.join.take();
            }
        }
        if let Some(join) = handle_to_join {
            let _ = join.join();
        }
        Ok(())
    }

    pub fn set_ref(id: u32, is_refed: bool) {
        let mut host = HOST.lock().unwrap();
        if let Some(worker) = host.workers.get_mut(&id) {
            worker.is_refed = is_refed;
        }
    }

    pub fn has_active_workers() -> bool {
        let host = HOST.lock().unwrap();
        host.workers
            .values()
            .any(|w| !w.is_terminated && w.is_refed)
    }

    pub fn has_parent_messages() -> bool {
        let mut host = HOST.lock().unwrap();
        host.poll_messages();
        !host.pending_messages.is_empty()
    }

    fn poll_messages(&mut self) {
        while let Ok(msg) = self.parent_rx.try_recv() {
            if let ParentMessage::Exit { worker_id, .. } = &msg {
                if let Some(w) = self.workers.get_mut(worker_id) {
                    w.is_terminated = true;
                }
            }
            self.pending_messages.push(msg);
        }
    }

    pub fn drain_parent_messages() -> Vec<ParentMessage> {
        let mut host = HOST.lock().unwrap();
        host.poll_messages();
        std::mem::take(&mut host.pending_messages)
    }

    pub fn pump_parent_messages(scope: &mut v8::PinScope) {
        let messages = Self::drain_parent_messages();
        if messages.is_empty() {
            return;
        }

        let context = scope.get_current_context();
        let global = context.global(scope);

        let msg_fn_key = v8::String::new(scope, "__amber_dispatch_parent_message").unwrap();
        let err_fn_key = v8::String::new(scope, "__amber_dispatch_parent_error").unwrap();
        let exit_fn_key = v8::String::new(scope, "__amber_dispatch_parent_exit").unwrap();

        for msg in messages {
            match msg {
                ParentMessage::Message { worker_id, payload } => {
                    if let Some(handler) = global.get(scope, msg_fn_key.into()) {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(handler) {
                            let id_val = v8::Integer::new(scope, worker_id as i32);
                            let payload_val = v8::String::new(scope, &payload).unwrap();
                            let _ = func.call(
                                scope,
                                global.into(),
                                &[id_val.into(), payload_val.into()],
                            );
                        }
                    }
                }
                ParentMessage::Error { worker_id, error } => {
                    if let Some(handler) = global.get(scope, err_fn_key.into()) {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(handler) {
                            let id_val = v8::Integer::new(scope, worker_id as i32);
                            let err_val = v8::String::new(scope, &error).unwrap();
                            let _ =
                                func.call(scope, global.into(), &[id_val.into(), err_val.into()]);
                        }
                    }
                }
                ParentMessage::Exit {
                    worker_id,
                    exit_code,
                } => {
                    if let Some(handler) = global.get(scope, exit_fn_key.into()) {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(handler) {
                            let id_val = v8::Integer::new(scope, worker_id as i32);
                            let code_val = v8::Integer::new(scope, exit_code);
                            let _ =
                                func.call(scope, global.into(), &[id_val.into(), code_val.into()]);
                        }
                    }
                }
            }
        }
    }

    pub fn active_count() -> usize {
        let host = HOST.lock().unwrap();
        host.workers.values().filter(|w| !w.is_terminated).count()
    }
}

fn run_worker_thread(
    id: u32,
    script_source: String,
    worker_data_json: Option<String>,
    parent_tx: Sender<ParentMessage>,
    rx: Receiver<WorkerMessage>,
) -> Result<()> {
    // Independent isolate lifetime for this OS thread.
    crate::initialize_v8()?;
    let mut isolate = v8::Isolate::new(Default::default());

    WORKER_THREAD_CHANNEL.with(|cell| {
        *cell.borrow_mut() = Some((id, parent_tx.clone()));
    });

    {
        v8::scope!(let scope, &mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let global = context.global(scope);

        // Console for worker
        let console = v8::Object::new(scope);
        let log_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             _rv: v8::ReturnValue| {
                let mut parts = Vec::new();
                for i in 0..args.length() {
                    if let Some(s) = args.get(i).to_string(scope) {
                        parts.push(s.to_rust_string_lossy(scope));
                    }
                }
                println!("{}", parts.join(" "));
            },
        )
        .unwrap();
        let log_key = v8::String::new(scope, "log").unwrap();
        let error_key = v8::String::new(scope, "error").unwrap();
        let warn_key = v8::String::new(scope, "warn").unwrap();
        let info_key = v8::String::new(scope, "info").unwrap();
        console.set(scope, log_key.into(), log_fn.into());
        console.set(scope, error_key.into(), log_fn.into());
        console.set(scope, warn_key.into(), log_fn.into());
        console.set(scope, info_key.into(), log_fn.into());
        let console_key = v8::String::new(scope, "console").unwrap();
        global.set(scope, console_key.into(), console.into());

        // Native callback: __amber_worker_post_message(payload_str)
        let send_fn = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             _rv: v8::ReturnValue| {
                let payload = args
                    .get(0)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_else(|| "null".to_string());
                WORKER_THREAD_CHANNEL.with(|cell| {
                    if let Some((worker_id, ref tx)) = *cell.borrow() {
                        let _ = tx.send(ParentMessage::Message { worker_id, payload });
                    }
                });
            },
        )
        .unwrap();
        let send_key = v8::String::new(scope, "__amber_worker_post_message").unwrap();
        global.set(scope, send_key.into(), send_fn.into());

        // Worker bootstrap script
        let worker_data_js = worker_data_json.as_deref().unwrap_or("undefined");
        let bootstrap_source = format!(
            r#"
            (function() {{
                globalThis.__amber_worker_listening = false;

                class WorkerEventEmitter {{
                    constructor() {{ this._events = {{}}; }}
                    on(event, fn) {{
                        (this._events[event] = this._events[event] || []).push(fn);
                        if (event === 'message') globalThis.__amber_worker_listening = true;
                        return this;
                    }}
                    once(event, fn) {{
                        const wrapper = (...args) => {{ this.off(event, wrapper); fn.apply(this, args); }};
                        wrapper._original = fn;
                        return this.on(event, wrapper);
                    }}
                    emit(event, ...args) {{
                        const list = (this._events[event] || []).slice();
                        for (const fn of list) {{ fn.apply(this, args); }}
                        return list.length > 0;
                    }}
                    off(event, fn) {{
                        if (!this._events[event]) return this;
                        this._events[event] = this._events[event].filter(cb => cb !== fn && cb._original !== fn);
                        if (event === 'message' && this._events[event].length === 0) {{
                            globalThis.__amber_worker_listening = false;
                        }}
                        return this;
                    }}
                    removeListener(event, fn) {{ return this.off(event, fn); }}
                    removeAllListeners(event) {{
                        if (event) {{
                            delete this._events[event];
                            if (event === 'message') globalThis.__amber_worker_listening = false;
                        }} else {{
                            this._events = {{}};
                            globalThis.__amber_worker_listening = false;
                        }}
                        return this;
                    }}
                    addListener(event, fn) {{ return this.on(event, fn); }}
                }}

                const parentPort = new WorkerEventEmitter();
                parentPort.postMessage = function(data) {{
                    __amber_worker_post_message(typeof data === 'string' ? JSON.stringify(data) : JSON.stringify(data));
                }};
                parentPort.close = function() {{
                    globalThis.__amber_worker_listening = false;
                }};

                globalThis.postMessage = function(data) {{
                    parentPort.postMessage(data);
                }};
                globalThis.self = globalThis;

                let parsedWorkerData;
                try {{
                    parsedWorkerData = {worker_data_js};
                }} catch (_) {{
                    parsedWorkerData = undefined;
                }}

                const wtModule = {{
                    isMainThread: false,
                    parentPort: parentPort,
                    workerData: parsedWorkerData,
                    threadId: {id},
                    Worker: function() {{ throw new Error("Nested Worker is not supported"); }}
                }};

                globalThis.__amber_worker_threads = wtModule;
                globalThis.require = function(mod) {{
                    if (mod === 'worker_threads' || mod === 'node:worker_threads') {{
                        return wtModule;
                    }}
                    if (mod === 'events' || mod === 'node:events') {{
                        return {{ EventEmitter: WorkerEventEmitter }};
                    }}
                    throw new Error("Worker isolate cannot require '" + mod + "'");
                }};

                globalThis.addEventListener = function(event, fn) {{
                    if (event === 'message') {{
                        globalThis.__amber_worker_listening = true;
                        parentPort.on('message', (data) => fn({{ data }}));
                    }}
                }};

                let userOnmessage = null;
                Object.defineProperty(globalThis, 'onmessage', {{
                    get() {{ return userOnmessage; }},
                    set(fn) {{
                        userOnmessage = fn;
                        if (fn) {{
                            globalThis.__amber_worker_listening = true;
                        }}
                    }}
                }});

                globalThis.__amber_dispatch_message = function(raw) {{
                    let data;
                    try {{
                        data = JSON.parse(raw);
                    }} catch (_) {{
                        data = raw;
                    }}
                    parentPort.emit('message', data);
                    if (typeof userOnmessage === 'function') {{
                        userOnmessage({{ data }});
                    }}
                }};
            }})();
            "#,
            worker_data_js = worker_data_js,
            id = id
        );

        let bs_source = v8::String::new(scope, &bootstrap_source).unwrap();
        if let Some(script) = v8::Script::compile(scope, bs_source, None) {
            let _ = script.run(scope);
        }

        // Run user script with TryCatch
        v8::tc_scope!(let scope, scope);
        let user_src = v8::String::new(scope, &script_source).unwrap();
        let compile_ok = match v8::Script::compile(scope, user_src, None) {
            Some(script) => {
                let run_res = script.run(scope);
                run_res.is_some()
            }
            None => false,
        };

        if !compile_ok || scope.has_caught() {
            let err_msg = scope
                .exception()
                .and_then(|exc| exc.to_string(scope))
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "Unknown worker script error".to_string());
            let _ = parent_tx.send(ParentMessage::Error {
                worker_id: id,
                error: err_msg,
            });
            let _ = parent_tx.send(ParentMessage::Exit {
                worker_id: id,
                exit_code: 1,
            });
            WORKER_THREAD_CHANNEL.with(|cell| *cell.borrow_mut() = None);
            return Ok(());
        }

        // Check if worker registered message listeners
        let is_listening = {
            let key = v8::String::new(scope, "__amber_worker_listening").unwrap();
            global
                .get(scope, key.into())
                .map(|v| v.to_boolean(scope).is_true())
                .unwrap_or(false)
        };

        if is_listening {
            loop {
                match rx.recv() {
                    Ok(WorkerMessage::PostMessage(payload)) => {
                        let dispatch_key =
                            v8::String::new(scope, "__amber_dispatch_message").unwrap();
                        if let Some(handler) = global.get(scope, dispatch_key.into()) {
                            if let Ok(func) = v8::Local::<v8::Function>::try_from(handler) {
                                let arg = v8::String::new(scope, &payload).unwrap();
                                let _ = func.call(scope, global.into(), &[arg.into()]);
                            }
                        }
                    }
                    Ok(WorkerMessage::Terminate) | Err(_) => break,
                }
            }
        }

        let _ = parent_tx.send(ParentMessage::Exit {
            worker_id: id,
            exit_code: 0,
        });
    }

    WORKER_THREAD_CHANNEL.with(|cell| *cell.borrow_mut() = None);
    Ok(())
}

/// Extract the script body from a `data:[<mediatype>][;base64],<data>` URL.
pub fn decode_data_url(url: &str) -> Result<String> {
    let body = url
        .strip_prefix("data:")
        .ok_or_else(|| anyhow!("not a data URL: {}", url))?;
    let (metadata, payload) = body
        .split_once(',')
        .ok_or_else(|| anyhow!("data URL is missing the ',' separator: {}", url))?;

    if metadata
        .rsplit(';')
        .next()
        .is_some_and(|last| last.eq_ignore_ascii_case("base64"))
    {
        use base64::Engine as _;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(payload.trim())
            .map_err(|e| anyhow!("data URL has invalid base64 payload: {}", e))?;
        String::from_utf8(decoded).map_err(|e| anyhow!("data URL is not valid UTF-8: {}", e))
    } else {
        Ok(percent_encoding::percent_decode_str(payload)
            .decode_utf8()
            .map_err(|e| anyhow!("data URL is not valid UTF-8: {}", e))?
            .into_owned())
    }
}

pub fn resolve_script_source(script_ref: &str) -> Result<(String, String)> {
    if script_ref.starts_with("data:") {
        let code = decode_data_url(script_ref)?;
        Ok((code, script_ref.to_string()))
    } else if script_ref.contains('\n') {
        Ok((script_ref.to_string(), "[inline]".to_string()))
    } else {
        let path = Path::new(script_ref);
        let target_path = if path.is_file() {
            path.to_path_buf()
        } else {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let rel = cwd.join(path);
            if rel.is_file() {
                rel
            } else {
                return Err(anyhow!(
                    "WorkerHost could not load script '{}'. Pass a readable file path.",
                    script_ref
                ));
            }
        };
        let code = std::fs::read_to_string(&target_path).map_err(|_| {
            anyhow!(
                "WorkerHost could not load script '{}'. Pass a readable file path.",
                script_ref
            )
        })?;
        Ok((code, target_path.display().to_string()))
    }
}

/// Setup Worker and worker_threads API in the main V8 context.
pub fn setup_worker_host_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Native: __amber_spawn_worker(source, url, workerDataJson) -> id
    let spawn_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 2 {
                let msg = v8::String::new(scope, "Worker requires script source and url").unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }
            let source = args
                .get(0)
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let url = args
                .get(1)
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let worker_data_json = if args.length() > 2 && !args.get(2).is_null_or_undefined() {
                args.get(2)
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
            } else {
                None
            };

            match WorkerHost::spawn(source, url, worker_data_json) {
                Ok(id) => {
                    rv.set(v8::Integer::new(scope, id as i32).into());
                }
                Err(err) => {
                    let msg = v8::String::new(scope, &format!("{}", err)).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        },
    )
    .unwrap();
    let spawn_key = v8::String::new(scope, "__amber_spawn_worker").unwrap();
    global.set(scope, spawn_key.into(), spawn_fn.into());

    // Native: __amber_worker_post(id, payloadJson)
    let post_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let id = args
                .get(0)
                .to_uint32(scope)
                .map(|v| v.value() as u32)
                .unwrap_or(0);
            let payload = args
                .get(1)
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "null".to_string());
            if let Err(err) = WorkerHost::post_message(id, payload) {
                let msg = v8::String::new(scope, &format!("{}", err)).unwrap();
                let exc = v8::Exception::error(scope, msg);
                scope.throw_exception(exc);
                return;
            }
            rv.set(v8::undefined(scope).into());
        },
    )
    .unwrap();
    let post_key = v8::String::new(scope, "__amber_worker_post").unwrap();
    global.set(scope, post_key.into(), post_fn.into());

    // Native: __amber_worker_terminate(id)
    let term_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let id = args
                .get(0)
                .to_uint32(scope)
                .map(|v| v.value() as u32)
                .unwrap_or(0);
            let _ = WorkerHost::terminate(id);
            rv.set(v8::undefined(scope).into());
        },
    )
    .unwrap();
    let term_key = v8::String::new(scope, "__amber_worker_terminate").unwrap();
    global.set(scope, term_key.into(), term_fn.into());

    // Native: __amber_worker_ref(id, is_refed)
    let ref_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let id = args
                .get(0)
                .to_uint32(scope)
                .map(|v| v.value() as u32)
                .unwrap_or(0);
            let is_refed = args.get(1).to_boolean(scope).is_true();
            WorkerHost::set_ref(id, is_refed);
            rv.set(v8::undefined(scope).into());
        },
    )
    .unwrap();
    let ref_key = v8::String::new(scope, "__amber_worker_ref").unwrap();
    global.set(scope, ref_key.into(), ref_fn.into());

    // Native: __amber_resolve_script_file(script_ref) -> JSON string { source, url }
    let resolve_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let script_ref = args
                .get(0)
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            match resolve_script_source(&script_ref) {
                Ok((source, url)) => {
                    let obj = v8::Object::new(scope);
                    let src_key = v8::String::new(scope, "source").unwrap();
                    let src_val = v8::String::new(scope, &source).unwrap();
                    obj.set(scope, src_key.into(), src_val.into());
                    let url_key = v8::String::new(scope, "url").unwrap();
                    let url_val = v8::String::new(scope, &url).unwrap();
                    obj.set(scope, url_key.into(), url_val.into());
                    rv.set(obj.into());
                }
                Err(err) => {
                    let msg = v8::String::new(scope, &format!("{}", err)).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        },
    )
    .unwrap();
    let resolve_key = v8::String::new(scope, "__amber_resolve_script_file").unwrap();
    global.set(scope, resolve_key.into(), resolve_fn.into());

    // Install JavaScript Worker and worker_threads bootstrap
    let js_bootstrap = r#"
    (function() {
        globalThis.__amber_workers = {};

        class BeeWorkerEventEmitter {
            constructor() { this._events = {}; }
            on(event, fn) {
                (this._events[event] = this._events[event] || []).push(fn);
                return this;
            }
            once(event, fn) {
                const wrapper = (...args) => { this.off(event, wrapper); fn.apply(this, args); };
                wrapper._original = fn;
                return this.on(event, wrapper);
            }
            emit(event, ...args) {
                const list = (this._events[event] || []).slice();
                for (const fn of list) { fn.apply(this, args); }
                return list.length > 0;
            }
            off(event, fn) {
                if (!this._events[event]) return this;
                this._events[event] = this._events[event].filter(cb => cb !== fn && cb._original !== fn);
                return this;
            }
            removeListener(event, fn) { return this.off(event, fn); }
            removeAllListeners(event) {
                if (event) delete this._events[event];
                else this._events = {};
                return this;
            }
            addListener(event, fn) { return this.on(event, fn); }
        }

        class Worker extends BeeWorkerEventEmitter {
            constructor(filename, options = {}) {
                super();
                if (!filename || typeof filename !== 'string') {
                    throw new TypeError("Worker requires a script URL or source");
                }

                let source = '';
                let scriptUrl = filename;

                if (options && options.eval) {
                    source = filename;
                    scriptUrl = '[eval]';
                } else {
                    const resolved = __amber_resolve_script_file(filename);
                    source = resolved.source;
                    scriptUrl = resolved.url;
                }

                let workerDataJson = null;
                if (options && 'workerData' in options && options.workerData !== undefined) {
                    workerDataJson = JSON.stringify(options.workerData);
                }

                const id = __amber_spawn_worker(source, scriptUrl, workerDataJson);
                this.threadId = id;
                this._workerId = id;
                this._isTerminated = false;
                this.onmessage = null;
                this.onerror = null;

                globalThis.__amber_workers[id] = this;
            }

            postMessage(data) {
                if (this._isTerminated) {
                    throw new Error("Cannot postMessage to a terminated worker");
                }
                const payload = JSON.stringify(data);
                __amber_worker_post(this._workerId, payload);
            }

            terminate() {
                if (this._isTerminated) return;
                this._isTerminated = true;
                __amber_worker_terminate(this._workerId);
                delete globalThis.__amber_workers[this._workerId];
                this.emit('exit', 0);
            }

            ref() {
                __amber_worker_ref(this._workerId, true);
                return this;
            }

            unref() {
                __amber_worker_ref(this._workerId, false);
                return this;
            }

            addEventListener(event, fn) {
                return this.on(event, fn);
            }

            removeEventListener(event, fn) {
                return this.off(event, fn);
            }
        }

        globalThis.__amber_dispatch_parent_message = function(id, rawPayload) {
            const worker = globalThis.__amber_workers[id];
            if (!worker) return;
            let data;
            try {
                data = JSON.parse(rawPayload);
            } catch (_) {
                data = rawPayload;
            }
            worker.emit('message', data);
            if (typeof worker.onmessage === 'function') {
                worker.onmessage({ data });
            }
        };

        globalThis.__amber_dispatch_parent_error = function(id, errorMessage) {
            const worker = globalThis.__amber_workers[id];
            if (!worker) return;
            const err = new Error(errorMessage);
            worker.emit('error', err);
            if (typeof worker.onerror === 'function') {
                worker.onerror(err);
            }
        };

        globalThis.__amber_dispatch_parent_exit = function(id, exitCode) {
            const worker = globalThis.__amber_workers[id];
            if (!worker) return;
            worker._isTerminated = true;
            delete globalThis.__amber_workers[id];
            worker.emit('exit', exitCode);
        };

        const wt = {
            isMainThread: true,
            parentPort: null,
            workerData: undefined,
            threadId: 0,
            Worker: Worker,
            MessagePort: BeeWorkerEventEmitter,
            MessageChannel: class MessageChannel {
                constructor() {
                    this.port1 = new BeeWorkerEventEmitter();
                    this.port2 = new BeeWorkerEventEmitter();
                }
            }
        };

        globalThis.Worker = Worker;
        globalThis.BeeWorkerHost = Worker;
        globalThis.worker_threads = wt;
    })();
    "#;

    let bs_src = v8::String::new(scope, js_bootstrap).unwrap();
    if let Some(script) = v8::Script::compile(scope, bs_src, None) {
        let _ = script.run(scope);
    }

    Ok(())
}

impl Default for WorkerHost {
    fn default() -> Self {
        Self::new()
    }
}
