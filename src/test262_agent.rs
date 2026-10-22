//! test262 `$262.agent` host — real concurrent agents on OS threads.
//!
//! Each `$262.agent.start(script)` spawns an independent V8 isolate that shares
//! SharedArrayBuffer memory via backing-store handoff. Broadcast blocks until
//! every live agent has retrieved the SAB (ack before the receiver callback).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use rusty_v8 as v8;

/// Process-start monotonic clock for `$262.agent.monotonicNow`.
static MONO_ORIGIN: Lazy<Instant> = Lazy::new(Instant::now);

/// SharedArrayBuffer backing stores are cross-thread by design; the Rust
/// bindings mark `BackingStore: Send` but not `Sync`, so `SharedRef` is not
/// `Send`. Wrap for agent broadcast channels.
struct SendSab(v8::SharedRef<v8::BackingStore>);

unsafe impl Send for SendSab {}
unsafe impl Sync for SendSab {}

impl Clone for SendSab {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Clone)]
enum BroadcastId {
    Number(f64),
    BigInt(i64),
}

struct BroadcastMsg {
    sab: SendSab,
    id: BroadcastId,
    /// Incremented by each agent after it has retrieved the message.
    ack: Arc<AtomicUsize>,
}

struct AgentHandle {
    broadcast_tx: Sender<BroadcastMsg>,
}

struct AgentHostState {
    agents: Vec<AgentHandle>,
    reports: VecDeque<String>,
    report_tx: Sender<String>,
    report_rx: Receiver<String>,
}

impl AgentHostState {
    fn new() -> Self {
        let (report_tx, report_rx) = mpsc::channel();
        Self {
            agents: Vec::new(),
            reports: VecDeque::new(),
            report_tx,
            report_rx,
        }
    }

    fn drain_reports(&mut self) {
        while let Ok(msg) = self.report_rx.try_recv() {
            self.reports.push_back(msg);
        }
    }
}

static AGENT_HOST: Lazy<Mutex<AgentHostState>> = Lazy::new(|| Mutex::new(AgentHostState::new()));

thread_local! {
    /// Set only on agent OS threads.
    static AGENT_RX: std::cell::RefCell<Option<Receiver<BroadcastMsg>>> =
        const { std::cell::RefCell::new(None) };
    static AGENT_REPORT_TX: std::cell::RefCell<Option<Sender<String>>> =
        const { std::cell::RefCell::new(None) };
}

/// Register parent-side `$262.agent` natives on the main isolate.
pub fn setup_agent_host(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
    context: &v8::Local<v8::Context>,
) {
    let global = context.global(scope);

    let start = v8::Function::new(scope, agent_start_cb).unwrap();
    let start_key = v8::String::new(scope, "__amber_agent_start").unwrap();
    global.set(scope, start_key.into(), start.into());

    let broadcast = v8::Function::new(scope, agent_broadcast_cb).unwrap();
    let broadcast_key = v8::String::new(scope, "__amber_agent_broadcast").unwrap();
    global.set(scope, broadcast_key.into(), broadcast.into());

    let get_report = v8::Function::new(scope, agent_get_report_cb).unwrap();
    let get_report_key = v8::String::new(scope, "__amber_agent_getReport").unwrap();
    global.set(scope, get_report_key.into(), get_report.into());

    let sleep = v8::Function::new(scope, agent_sleep_cb).unwrap();
    let sleep_key = v8::String::new(scope, "__amber_agent_sleep").unwrap();
    global.set(scope, sleep_key.into(), sleep.into());

    let mono = v8::Function::new(scope, agent_monotonic_now_cb).unwrap();
    let mono_key = v8::String::new(scope, "__amber_agent_monotonicNow").unwrap();
    global.set(scope, mono_key.into(), mono.into());
}

fn monotonic_now_ms() -> f64 {
    MONO_ORIGIN.elapsed().as_secs_f64() * 1000.0
}

fn agent_monotonic_now_cb(
    scope: &mut v8::PinScope<'_, '_>,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    rv.set(v8::Number::new(scope, monotonic_now_ms()).into());
}

fn agent_sleep_cb(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let ms = args.get(0).number_value(scope).unwrap_or(0.0).max(0.0);
    if ms > 0.0 {
        thread::sleep(Duration::from_secs_f64(ms / 1000.0));
    }
}

fn agent_get_report_cb(
    scope: &mut v8::PinScope<'_, '_>,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let mut host = AGENT_HOST.lock().unwrap();
    host.drain_reports();
    match host.reports.pop_front() {
        Some(msg) => {
            let s = v8::String::new(scope, &msg).unwrap();
            rv.set(s.into());
        }
        None => rv.set(v8::null(scope).into()),
    }
}

fn agent_start_cb(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let script = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let (broadcast_tx, broadcast_rx) = mpsc::channel::<BroadcastMsg>();
    let report_tx = {
        let host = AGENT_HOST.lock().unwrap();
        host.report_tx.clone()
    };

    let ready = Arc::new((Mutex::new(false), Condvar::new()));
    let ready_agent = ready.clone();

    let join = thread::Builder::new()
        .name("test262-agent".to_string())
        .spawn(move || {
            if let Err(err) = run_agent_thread(script, broadcast_rx, report_tx, ready_agent) {
                eprintln!("[test262-agent] fatal: {err}");
            }
        });

    match join {
        Ok(_handle) => {
            // Keep the JoinHandle dropped — agents may block in Atomics.wait
            // until process exit; joining would hang the parent.
            let (lock, cv) = &*ready;
            let mut is_ready = lock.lock().unwrap();
            while !*is_ready {
                is_ready = cv.wait(is_ready).unwrap();
            }
            let mut host = AGENT_HOST.lock().unwrap();
            host.agents.push(AgentHandle { broadcast_tx });
        }
        Err(err) => {
            let msg = format!("$262.agent.start failed to spawn agent: {err}");
            let s = v8::String::new(scope, &msg).unwrap();
            let exc = v8::Exception::error(scope, s);
            scope.throw_exception(exc);
        }
    }
}

fn agent_broadcast_cb(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let sab_val = args.get(0);
    if !sab_val.is_shared_array_buffer() {
        let s =
            v8::String::new(scope, "$262.agent.broadcast requires a SharedArrayBuffer").unwrap();
        let exc = v8::Exception::type_error(scope, s);
        scope.throw_exception(exc);
        return;
    }
    let sab = v8::Local::<v8::SharedArrayBuffer>::try_from(sab_val).unwrap();
    let backing = sab.get_backing_store();

    let id = if args.length() < 2 || args.get(1).is_undefined() {
        BroadcastId::Number(0.0)
    } else if args.get(1).is_big_int() {
        let bi = v8::Local::<v8::BigInt>::try_from(args.get(1)).unwrap();
        let (val, _lossy) = bi.i64_value();
        BroadcastId::BigInt(val)
    } else {
        BroadcastId::Number(args.get(1).number_value(scope).unwrap_or(0.0))
    };

    let agents: Vec<Sender<BroadcastMsg>> = {
        let host = AGENT_HOST.lock().unwrap();
        host.agents.iter().map(|a| a.broadcast_tx.clone()).collect()
    };
    let n = agents.len();
    if n == 0 {
        return;
    }

    let ack = Arc::new(AtomicUsize::new(0));
    let send_sab = SendSab(backing);
    for tx in &agents {
        let msg = BroadcastMsg {
            sab: send_sab.clone(),
            id: id.clone(),
            ack: ack.clone(),
        };
        if tx.send(msg).is_err() {
            // Agent already exited; still count toward ack to avoid hang.
            ack.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Block until every agent has retrieved the broadcast.
    let start = Instant::now();
    while ack.load(Ordering::SeqCst) < n {
        if start.elapsed() > Duration::from_secs(60) {
            let s = v8::String::new(scope, "$262.agent.broadcast timed out waiting for agents")
                .unwrap();
            let exc = v8::Exception::error(scope, s);
            scope.throw_exception(exc);
            return;
        }
        thread::sleep(Duration::from_millis(1));
    }
}

fn run_agent_thread(
    script: String,
    broadcast_rx: Receiver<BroadcastMsg>,
    report_tx: Sender<String>,
    ready: Arc<(Mutex<bool>, Condvar)>,
) -> Result<(), String> {
    crate::initialize_v8().map_err(|e| e.to_string())?;

    AGENT_RX.with(|cell| {
        *cell.borrow_mut() = Some(broadcast_rx);
    });
    AGENT_REPORT_TX.with(|cell| {
        *cell.borrow_mut() = Some(report_tx);
    });

    let params = v8::CreateParams::default().allow_atomics_wait(true);
    let mut isolate = v8::Isolate::new(params);
    isolate.set_allow_atomics_wait(true);

    {
        v8::scope!(let scope, &mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let global = context.global(scope);

        // Minimal console for agent diagnostics.
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
                eprintln!("[test262-agent] {}", parts.join(" "));
            },
        )
        .unwrap();
        let log_key = v8::String::new(scope, "log").unwrap();
        console.set(scope, log_key.into(), log_fn.into());
        let console_key = v8::String::new(scope, "console").unwrap();
        global.set(scope, console_key.into(), console.into());

        install_agent_side_262(scope, &global);

        // Signal parent that the agent is running before evaluating the
        // script (which typically blocks in receiveBroadcast).
        {
            let (lock, cv) = &*ready;
            let mut is_ready = lock.lock().unwrap();
            *is_ready = true;
            cv.notify_one();
        }

        let source = v8::String::new(scope, &script).ok_or_else(|| "oom".to_string())?;
        v8::tc_scope!(let tc, scope);
        let compiled = v8::Script::compile(tc, source, None)
            .ok_or_else(|| "agent script compile failed".to_string())?;
        if compiled.run(tc).is_none() {
            let msg = tc
                .exception()
                .and_then(|e| e.to_string(tc))
                .map(|s| s.to_rust_string_lossy(tc))
                .unwrap_or_else(|| "agent script threw".to_string());
            return Err(msg);
        }
    }

    Ok(())
}

fn install_agent_side_262(scope: &mut v8::PinScope<'_, '_>, global: &v8::Local<v8::Object>) {
    let agent = v8::Object::new(scope);

    let recv = v8::Function::new(scope, agent_receive_broadcast_cb).unwrap();
    let recv_key = v8::String::new(scope, "receiveBroadcast").unwrap();
    agent.set(scope, recv_key.into(), recv.into());

    let report = v8::Function::new(scope, agent_report_cb).unwrap();
    let report_key = v8::String::new(scope, "report").unwrap();
    agent.set(scope, report_key.into(), report.into());

    let sleep = v8::Function::new(scope, agent_sleep_cb).unwrap();
    let sleep_key = v8::String::new(scope, "sleep").unwrap();
    agent.set(scope, sleep_key.into(), sleep.into());

    let leaving = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope, _args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {},
    )
    .unwrap();
    let leaving_key = v8::String::new(scope, "leaving").unwrap();
    agent.set(scope, leaving_key.into(), leaving.into());

    let mono = v8::Function::new(scope, agent_monotonic_now_cb).unwrap();
    let mono_key = v8::String::new(scope, "monotonicNow").unwrap();
    agent.set(scope, mono_key.into(), mono.into());

    let host = v8::Object::new(scope);
    let agent_key = v8::String::new(scope, "agent").unwrap();
    host.set(scope, agent_key.into(), agent.into());

    let key = v8::String::new(scope, "$262").unwrap();
    global.set(scope, key.into(), host.into());
}

fn agent_report_cb(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let msg = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_else(|| "undefined".to_string());
    AGENT_REPORT_TX.with(|cell| {
        if let Some(ref tx) = *cell.borrow() {
            let _ = tx.send(msg);
        }
    });
}

fn agent_receive_broadcast_cb(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let cb_val = args.get(0);
    let Ok(callback) = v8::Local::<v8::Function>::try_from(cb_val) else {
        let s = v8::String::new(scope, "$262.agent.receiveBroadcast requires a function").unwrap();
        let exc = v8::Exception::type_error(scope, s);
        scope.throw_exception(exc);
        return;
    };

    // Block until a broadcast arrives (may already be queued).
    let msg = AGENT_RX.with(|cell| {
        let borrow = cell.borrow();
        let rx = borrow.as_ref().expect("agent rx missing");
        rx.recv()
    });

    let Ok(msg) = msg else {
        let s = v8::String::new(scope, "agent broadcast channel closed").unwrap();
        let exc = v8::Exception::error(scope, s);
        scope.throw_exception(exc);
        return;
    };

    let sab = v8::SharedArrayBuffer::with_backing_store(scope, &msg.sab.0);
    let id_val: v8::Local<v8::Value> = match msg.id {
        BroadcastId::Number(n) => v8::Number::new(scope, n).into(),
        BroadcastId::BigInt(n) => v8::BigInt::new_from_i64(scope, n).into(),
    };

    // Ack retrieval before invoking the receiver (matches V8 harness-agent.js).
    msg.ack.fetch_add(1, Ordering::SeqCst);

    let recv = v8::undefined(scope).into();
    let _ = callback.call(scope, recv, &[sab.into(), id_val]);
}
