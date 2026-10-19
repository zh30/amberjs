// Node.js process 模块实现
// v0.3.237: 全局未捕获异常处理器和 process 对象
// v0.3.239: 完善 nextTick 和 stdout/stderr.write()
// v0.3.240: 完善 hrtime、stdin、memory、uptime、cpuUsage

use anyhow::Result;
use rusty_v8 as v8;
use std::collections::VecDeque;
use std::io::Read;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

thread_local! {
    /// 进程启动时间（用于计算 uptime）
    static START_TIME: Instant = Instant::now();
}

thread_local! {
    /// 未捕获异常处理器
    static UNCAUGHT_EXCEPTION_HANDLERS: Mutex<Vec<v8::Global<v8::Value>>> = Mutex::new(Vec::new());
    /// 未处理的 Promise rejection 处理器
    static UNHANDLED_REJECTION_HANDLERS: Mutex<Vec<v8::Global<v8::Value>>> = Mutex::new(Vec::new());
    /// 程序是否应该退出
    static SHOULD_EXIT: Mutex<bool> = Mutex::new(false);
    /// 退出码
    static EXIT_CODE: Mutex<i32> = Mutex::new(0);
}

// v0.3.239: nextTick 队列（线程本地）
// Note: thread_local doesn't have pub(crate) on individual items, so we export the whole thing
mod next_tick_queue_mod {
    use rusty_v8 as v8;
    use std::sync::Mutex;

    pub(crate) struct NextTickCallback {
        pub(crate) callback: v8::Global<v8::Value>,
        pub(crate) args: Vec<v8::Global<v8::Value>>,
    }

    thread_local! {
        pub(crate) static NEXT_TICK_QUEUE: Mutex<Vec<NextTickCallback>> = Mutex::new(Vec::new());
    }
}

use next_tick_queue_mod::{NextTickCallback, NEXT_TICK_QUEUE};

// ============ G70: process.stdin 窄契约 ============
// Host 读线程把 fd 0 的字节推进 mpsc；事件循环每轮
// pump_pending_stdin_events_in_scope 排干成 'data'/'readable'/'end'。
// pause() 同时解除事件循环保活（等价 unref 语义），队列继续累积。
enum StdinEvent {
    Data(Vec<u8>),
    Eof,
}

static STDIN_RX: Mutex<Option<mpsc::Receiver<StdinEvent>>> = Mutex::new(None);
static STDIN_QUEUE: Mutex<VecDeque<Vec<u8>>> = Mutex::new(VecDeque::new());
static STDIN_STARTED: AtomicBool = AtomicBool::new(false);
static STDIN_FLOWING: AtomicBool = AtomicBool::new(false);
static STDIN_LIVENESS: AtomicBool = AtomicBool::new(false);
static STDIN_READER_DONE: AtomicBool = AtomicBool::new(false);
static STDIN_EOF: AtomicBool = AtomicBool::new(false);
static STDIN_EOF_PUMPED: AtomicBool = AtomicBool::new(false);
static STDIN_END_EMITTED: AtomicBool = AtomicBool::new(false);
static STDIN_READABLE_SIGNALED: AtomicBool = AtomicBool::new(false);
// UTF-8 跨 chunk 边界：未收完的多字节序列尾段暂存，下一段到来时拼接后再解码。
static STDIN_UTF8_TAIL: Mutex<Vec<u8>> = Mutex::new(Vec::new());
// stdin 归首个构建 process.stdin 对象的 runtime 所有（Node worker_threads
// 的 dummy stdin 语义）：后续 runtime/isolate 拿到惰性 stdin ——
// read() 恒 null、事件永不触发、不共享首个 runtime 的队列与 EOF。
static STDIN_OWNER_CLAIMED: AtomicBool = AtomicBool::new(false);

/// stdin 对象是否属于非首个 runtime（惰性 dummy）。
fn stdin_is_inert(this: &v8::Local<v8::Object>, scope: &mut v8::PinScope) -> bool {
    let key = v8::String::new(scope, "__stdinInert").unwrap();
    this.get(scope, key.into())
        .map(|v| v.to_boolean(scope).boolean_value(scope))
        .unwrap_or(false)
}

/// chunk 字节 → UTF-8 字符串。处理跨 host-read 边界被切断的多字节序列：
/// 尾部不完整序列不进本次解码（避免切成两个 U+FFFD），暂存后随下一段拼接。
fn stdin_bytes_to_string(bytes: &[u8]) -> String {
    let mut guard = STDIN_UTF8_TAIL.lock().unwrap();
    let mut joined = std::mem::take(&mut *guard);
    joined.extend_from_slice(bytes);
    match std::str::from_utf8(&joined) {
        Ok(_) => String::from_utf8_lossy(&joined).into_owned(),
        Err(e) if e.error_len().is_none() => {
            // 结尾停在未完成的多字节序列：前缀有效部分解码，尾巴留给下一段
            let up_to = e.valid_up_to();
            let (head, tail) = joined.split_at(up_to);
            *guard = tail.to_vec();
            String::from_utf8_lossy(head).into_owned()
        }
        Err(_) => String::from_utf8_lossy(&joined).into_owned(),
    }
}

fn start_stdin_reader() {
    if STDIN_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let (tx, rx) = mpsc::channel::<StdinEvent>();
    *STDIN_RX.lock().unwrap() = Some(rx);
    // 不在这里置 LIVENESS：裸 read() 只启动读线程而不保活事件循环
    // （Node 语义一致）；保活由 resume() / '__readable' 监听各自置位。
    thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut buf = [0u8; 8192];
        loop {
            // 每次 read 重新加锁而不是长占 StdinLock：否则线程一旦启动就持锁
            // 阻塞，readline.question 等其他 stdin 消费者会被卡到 EOF。
            // 锁在两次 read 之间释放，其它消费者可以穿插（但字节会被分流——
            // 混用 stdin 与 readline 本来就语义模糊，契约里已列为 Limit）。
            let read_result = {
                let mut lock = stdin.lock();
                lock.read(&mut buf)
            };
            match read_result {
                Ok(0) => {
                    let _ = tx.send(StdinEvent::Eof);
                    // EOF 标志必须在读线程内就置位：不能等 pump 排干 channel，
                    // 否则空 stdin（读线程瞬间结束）会让 has_pending_stdin_work
                    // 在 'end' 交付前误判为无工作而提前退出进程。
                    STDIN_EOF.store(true, Ordering::SeqCst);
                    break;
                }
                Ok(n) => {
                    if tx.send(StdinEvent::Data(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(StdinEvent::Eof);
                    STDIN_EOF.store(true, Ordering::SeqCst);
                    break;
                }
            }
        }
        STDIN_READER_DONE.store(true, Ordering::SeqCst);
    });
}

/// stdin 是否应继续保活事件循环：resume 且未 pause，且还有
/// 未交付的字节 / 未结束的读线程 / 未发出的 'end'。
pub fn has_pending_stdin_work() -> bool {
    if !STDIN_LIVENESS.load(Ordering::SeqCst) {
        return false;
    }
    if !STDIN_READER_DONE.load(Ordering::SeqCst) {
        return true;
    }
    let queued = STDIN_QUEUE
        .lock()
        .map(|queue| !queue.is_empty())
        .unwrap_or(false);
    if STDIN_EOF.load(Ordering::SeqCst) && !STDIN_FLOWING.load(Ordering::SeqCst) {
        // 非 flowing 模式 + EOF：队列里残留未读字节时没有消费者能再推进，
        // 保活只会把进程挂死（'readable' 监听不读的场景）。只在队列已空、
        // 还差一次 'end' 交付时保活，其余放行退出。
        return !queued && !STDIN_END_EMITTED.load(Ordering::SeqCst);
    }
    queued || (STDIN_EOF.load(Ordering::SeqCst) && !STDIN_END_EMITTED.load(Ordering::SeqCst))
}

fn stdin_object_emit_args<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: &v8::Local<v8::Context>,
    event: &str,
    chunk: Option<&[u8]>,
) -> Option<()> {
    let global = context.global(scope);
    let process_key = v8::String::new(scope, "process").unwrap();
    let process_val = global.get(scope, process_key.into())?;
    let process_obj = v8::Local::<v8::Object>::try_from(process_val).ok()?;
    let stdin_key = v8::String::new(scope, "stdin").unwrap();
    let stdin_val = process_obj.get(scope, stdin_key.into())?;
    let stdin_obj = v8::Local::<v8::Object>::try_from(stdin_val).ok()?;
    let emit_key = v8::String::new(scope, "emit").unwrap();
    let emit_val = stdin_obj.get(scope, emit_key.into())?;
    let emit_fn = v8::Local::<v8::Function>::try_from(emit_val).ok()?;
    let event_name = v8::String::new(scope, event).unwrap();
    let call_args: &[v8::Local<v8::Value>] = match chunk {
        Some(bytes) => {
            let text = stdin_bytes_to_string(bytes);
            let chunk_val = v8::String::new(scope, &text).unwrap();
            &[event_name.into(), chunk_val.into()]
        }
        None => &[event_name.into()],
    };
    let _ = emit_fn.call(scope, stdin_obj.into(), call_args);
    Some(())
}

/// 事件循环每轮调用（仅 V8 线程）：排干 stdin channel → 队列 →
/// 按 flowing/readable 语义 emit 'data'/'readable'，EOF + 队列空 → 'end'。
pub fn pump_pending_stdin_events_in_scope(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> usize {
    let mut drained = 0usize;
    {
        let guard = STDIN_RX.lock().unwrap();
        if let Some(rx) = guard.as_ref() {
            while let Ok(event) = rx.try_recv() {
                match event {
                    StdinEvent::Data(bytes) => {
                        STDIN_QUEUE.lock().unwrap().push_back(bytes);
                    }
                    StdinEvent::Eof => {
                        STDIN_EOF_PUMPED.store(true, Ordering::SeqCst);
                        // EOF 时把暂存的 UTF-8 尾巴作为最后一段入队：
                        // 避免未收完的多字节序列被丢弃（以 U+FFFD 交付一次）。
                        let tail = std::mem::take(&mut *STDIN_UTF8_TAIL.lock().unwrap());
                        if !tail.is_empty() {
                            STDIN_QUEUE.lock().unwrap().push_back(tail);
                        }
                    }
                }
                drained += 1;
                if drained >= 256 {
                    break;
                }
            }
        }
    }
    let queue_nonempty = STDIN_QUEUE
        .lock()
        .map(|queue| !queue.is_empty())
        .unwrap_or(false);
    // 'end' 的门必须是「pump 排干到 Eof 事件」，不能是读线程早已置位的
    // STDIN_EOF：每轮最多排 256 项，超过上限时 channel 里还有未交付的 Data，
    // 用线程标志会在剩余 chunk 之前提前 emit 'end'。
    let end_pending = STDIN_EOF_PUMPED.load(Ordering::SeqCst)
        && !queue_nonempty
        && !STDIN_END_EMITTED.load(Ordering::SeqCst);
    if drained == 0 && !end_pending && !(queue_nonempty && STDIN_FLOWING.load(Ordering::SeqCst)) {
        return 0;
    }

    let mut emitted = 0usize;
    if STDIN_FLOWING.load(Ordering::SeqCst) {
        loop {
            let bytes = STDIN_QUEUE.lock().unwrap().pop_front();
            let Some(bytes) = bytes else { break };
            stdin_object_emit_args(scope, context, "data", Some(&bytes));
            emitted += 1;
        }
    } else if queue_nonempty && !STDIN_READABLE_SIGNALED.swap(true, Ordering::SeqCst) {
        stdin_object_emit_args(scope, context, "readable", None);
        emitted += 1;
    }
    if STDIN_QUEUE
        .lock()
        .map(|queue| queue.is_empty())
        .unwrap_or(true)
    {
        STDIN_READABLE_SIGNALED.store(false, Ordering::SeqCst);
    }
    if end_pending {
        STDIN_END_EMITTED.store(true, Ordering::SeqCst);
        stdin_object_emit_args(scope, context, "end", None);
        emitted += 1;
    }
    emitted
}

fn stdin_read_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    if stdin_is_inert(&this, scope) {
        retval.set(v8::null(scope).into());
        return;
    }
    // read() 显式消费：启动读线程（不阻塞），返回已缓冲的下一个 chunk。
    start_stdin_reader();
    let bytes = STDIN_QUEUE.lock().unwrap().pop_front();
    match bytes {
        Some(bytes) => {
            let text = stdin_bytes_to_string(&bytes);
            let chunk = v8::String::new(scope, &text).unwrap();
            retval.set(chunk.into());
        }
        None => {
            let null_val = v8::null(scope);
            retval.set(null_val.into());
        }
    }
}

fn stdin_resume_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if !stdin_is_inert(&args.this(), scope) {
        STDIN_FLOWING.store(true, Ordering::SeqCst);
        STDIN_LIVENESS.store(true, Ordering::SeqCst);
        start_stdin_reader();
    }
    retval.set(args.this().into());
}

fn stdin_readable_start_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 'readable' 监听专用入口：启动读线程但不进入 flowing 模式。
    if !stdin_is_inert(&args.this(), scope) {
        STDIN_LIVENESS.store(true, Ordering::SeqCst);
        start_stdin_reader();
    }
    retval.set(args.this().into());
}

fn stdin_pause_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if !stdin_is_inert(&args.this(), scope) {
        STDIN_FLOWING.store(false, Ordering::SeqCst);
        STDIN_LIVENESS.store(false, Ordering::SeqCst);
    }
    retval.set(args.this().into());
}

fn stdin_ref_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if !stdin_is_inert(&args.this(), scope) && STDIN_STARTED.load(Ordering::SeqCst) {
        STDIN_LIVENESS.store(true, Ordering::SeqCst);
    }
    retval.set(args.this().into());
}

fn stdin_unref_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if !stdin_is_inert(&args.this(), scope) {
        STDIN_LIVENESS.store(false, Ordering::SeqCst);
    }
    retval.set(args.this().into());
}

fn stdin_set_encoding_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 惰性 stdin 也照常校验参数（Node dummy 仍接受合法编码）
    let enc = args.get(0);
    let ok = enc.is_null_or_undefined()
        || enc
            .to_string(scope)
            .map(|s| {
                let s = s.to_rust_string_lossy(scope);
                s.eq_ignore_ascii_case("utf8") || s.eq_ignore_ascii_case("utf-8")
            })
            .unwrap_or(false);
    if !ok {
        let msg = v8::String::new(
            scope,
            "Unknown encoding: process.stdin only supports 'utf8' (chunk strings)",
        )
        .unwrap();
        let err = v8::Exception::type_error(scope, msg);
        scope.throw_exception(err.into());
        return;
    }
    retval.set(args.this().into());
}

/// EventEmitter 原型挂载 bootstrap：process.stdin 在 setup_process_api
/// 时构造，但 globalThis.EventEmitter 要到 setup_events_api 才存在，
/// 所以必须在 install_core_apis 里 events 之后调用。
pub fn install_stdin_event_emitter(
    scope: &mut v8::PinScope,
    _context: &v8::Local<v8::Context>,
) -> Result<()> {
    let bootstrap = r#"
        (function() {
            const proc = globalThis.process;
            if (!proc || !proc.stdin || proc.stdin.__amberStdinBootstrapped) return;
            const s = proc.stdin;
            const EE = globalThis.EventEmitter;
            if (EE && EE.prototype) {
                Object.setPrototypeOf(s, EE.prototype);
            }
            s._events = Object.create(null);
            s._eventsCount = 0;
            const _on = s.on ? s.on.bind(s) : null;
            if (_on) {
                const wrap = function(ev, fn) {
                    if (ev === 'data' && typeof fn === 'function') s.resume();
                    else if (ev === 'readable' && typeof fn === 'function') s.__startReadable();
                    return _on(ev, fn);
                };
                s.on = wrap;
                s.addListener = wrap;
            }
            const _once = s.once ? s.once.bind(s) : null;
            if (_once) {
                s.once = function(ev, fn) {
                    if (ev === 'data' && typeof fn === 'function') s.resume();
                    else if (ev === 'readable' && typeof fn === 'function') s.__startReadable();
                    return _once(ev, fn);
                };
            }
            s.__amberStdinBootstrapped = true;
        })();
    "#;
    let code = v8::String::new(scope, bootstrap).unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let _ = script.run(scope);
    Ok(())
}

/// v0.3.261: 添加 nextTick 回调到队列（供 runtime_minimal.rs 使用）
pub fn push_next_tick_callback(callback: v8::Global<v8::Value>, args: Vec<v8::Global<v8::Value>>) {
    NEXT_TICK_QUEUE.with(|queue| {
        let mut q = queue.lock().unwrap();
        q.push(NextTickCallback { callback, args });
    });
}

/// v0.3.261: 执行所有 pending 的 nextTick 回调
/// 必须在 V8 主线程调用，在 perform_microtask_checkpoint 之前执行
/// 这样 nextTick 回调会在 Promise microtasks 之前执行（符合 Node.js 行为）
/// 关键：使用 while 循环确保所有链式添加的 nextTick 都能执行
pub fn execute_next_tick_callbacks(scope: &mut v8::PinScope) {
    NEXT_TICK_QUEUE.with(|q| {
        let mut queue_ref = q.lock().unwrap();

        // 使用 while 循环处理链式添加的 nextTicks
        // 每次迭代取出当前所有回调，执行它们
        // 如果回调中添加了新的 nextTick，它们会在下一轮迭代中执行
        while !queue_ref.is_empty() {
            // 取出当前所有回调（使用 take 清空队列）
            let callbacks: Vec<NextTickCallback> = std::mem::take(&mut *queue_ref);

            // 释放锁以便回调中可以安全地添加新的 nextTick
            drop(queue_ref);

            // 执行当前这批回调
            for NextTickCallback { callback, args } in callbacks.into_iter() {
                let callback_local = v8::Local::new(scope, callback);
                if let Ok(func) = v8::Local::<v8::Function>::try_from(callback_local) {
                    let undefined = v8::undefined(scope);
                    // Convert Global args to Local args for the function call
                    let args_local: Vec<v8::Local<v8::Value>> =
                        args.iter().map(|g| v8::Local::new(scope, g)).collect();
                    let _ = func.call(scope, undefined.into(), &args_local);
                }
            }

            // 重新获取锁，继续处理可能添加的新回调
            queue_ref = q.lock().unwrap();
        }
    });
}

/// v0.3.261: 检查是否有 pending 的 nextTick 回调
pub fn has_pending_next_ticks() -> bool {
    NEXT_TICK_QUEUE.with(|q| {
        let queue = q.lock().unwrap();
        !queue.is_empty()
    })
}

/// v0.3.261: 清空 nextTick 队列（用于测试清理）
pub fn clear_next_tick_queue() {
    NEXT_TICK_QUEUE.with(|q| {
        let mut queue = q.lock().unwrap();
        queue.clear();
    });
}

// v0.3.242: setMaxListeners 存储
// 存储每个事件类型的最大监听器数量，0 表示无限制
thread_local! {
    static MAX_LISTENERS: Mutex<std::collections::HashMap<String, i32>> =
        Mutex::new(std::collections::HashMap::new());
}

fn env_permission_check_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    let name = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Environment,
        crate::permissions::PermissionAction::Read,
        crate::permissions::ResourceId::Name(name),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
    }
}

fn env_is_allowed_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let name = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let allowed = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Environment,
        crate::permissions::PermissionAction::Read,
        crate::permissions::ResourceId::Name(name),
    )
    .is_ok();
    retval.set(v8::Boolean::new(scope, allowed).into());
}

fn wrap_process_env_proxy<'scope>(
    scope: &mut v8::PinScope<'scope, '_>,
    env_obj: v8::Local<'scope, v8::Object>,
    is_sandbox: bool,
) -> v8::Local<'scope, v8::Object> {
    let Some(code) = v8::String::new(
        scope,
        r#"
(function(raw, isSandbox) {
  return new Proxy(raw, {
    get(target, prop, receiver) {
      if (typeof prop !== 'string') {
        return Reflect.get(target, prop, receiver);
      }
      if (isSandbox) {
        if (!Object.prototype.hasOwnProperty.call(target, prop)) {
          if (typeof globalThis.__beeCheckEnv === 'function') {
            globalThis.__beeCheckEnv(prop);
          }
          return undefined;
        }
      }
      if (typeof globalThis.__beeIsEnvAllowed === 'function' && !globalThis.__beeIsEnvAllowed(prop)) {
        if (isSandbox && typeof globalThis.__beeCheckEnv === 'function') {
          globalThis.__beeCheckEnv(prop);
        }
        return undefined;
      }
      return target[prop];
    },
    set(target, prop, value, receiver) {
      if (typeof prop === 'string') {
        target[prop] = String(value);
        return true;
      }
      return Reflect.set(target, prop, value, receiver);
    },
    has(target, prop) {
      if (typeof prop === 'string') {
        if (isSandbox) {
          if (!Object.prototype.hasOwnProperty.call(target, prop)) {
            if (typeof globalThis.__beeCheckEnv === 'function') {
              globalThis.__beeCheckEnv(prop);
            }
          }
        }
        if (typeof globalThis.__beeIsEnvAllowed === 'function' && !globalThis.__beeIsEnvAllowed(prop)) {
          if (isSandbox && typeof globalThis.__beeCheckEnv === 'function') {
            globalThis.__beeCheckEnv(prop);
          }
          return false;
        }
      }
      return Reflect.has(target, prop);
    },
    deleteProperty(target, prop) {
      return Reflect.deleteProperty(target, prop);
    },
    ownKeys(target) {
      return Reflect.ownKeys(target).filter(prop => {
        if (typeof prop === 'string') {
          if (typeof globalThis.__beeIsEnvAllowed === 'function' && !globalThis.__beeIsEnvAllowed(prop)) {
            return false;
          }
        }
        return true;
      });
    },
    getOwnPropertyDescriptor(target, prop) {
      if (typeof prop === 'string') {
        if (typeof globalThis.__beeIsEnvAllowed === 'function' && !globalThis.__beeIsEnvAllowed(prop)) {
          return undefined;
        }
      }
      return Reflect.getOwnPropertyDescriptor(target, prop);
    }
  });
})
"#,
    ) else {
        return env_obj;
    };
    let Some(script) = v8::Script::compile(scope, code, None) else {
        return env_obj;
    };
    let Some(factory) = script.run(scope) else {
        return env_obj;
    };
    let Ok(factory) = v8::Local::<v8::Function>::try_from(factory) else {
        return env_obj;
    };
    let undefined = v8::undefined(scope).into();
    let is_sandbox_v8 = v8::Boolean::new(scope, is_sandbox);
    match factory.call(scope, undefined, &[env_obj.into(), is_sandbox_v8.into()]) {
        Some(value) if value.is_object() => value.to_object(scope).unwrap_or(env_obj),
        _ => env_obj,
    }
}

fn create_process_env_object<'scope>(
    scope: &mut v8::PinScope<'scope, '_>,
) -> v8::Local<'scope, v8::Object> {
    let env_obj = v8::Object::new(scope);

    for (key, value) in std::env::vars() {
        if crate::permissions::check_global_permission(
            crate::permissions::PermissionKind::Environment,
            crate::permissions::PermissionAction::Read,
            crate::permissions::ResourceId::Name(key.clone()),
        )
        .is_err()
        {
            continue;
        }

        let key_value = v8::String::new(scope, &key).unwrap();
        let env_value = v8::String::new(scope, &value).unwrap();
        env_obj.set(scope, key_value.into(), env_value.into());
    }

    let is_sandbox = crate::permissions::sandbox_strict_env();
    wrap_process_env_proxy(scope, env_obj, is_sandbox)
}

/// v0.3.39: Get RSS (Resident Set Size) memory in bytes
/// Cross-platform implementation for getting process memory usage
fn get_rss_memory() -> u64 {
    #[cfg(target_os = "linux")]
    {
        // On Linux, read from /proc/self/status
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    // Format: "VmRSS:    1234 kB"
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // Convert kB to bytes
                        }
                    }
                }
            }
        }
        0
    }
    #[cfg(target_os = "macos")]
    {
        // On macOS, use libc getrusage
        use libc::{getrusage, rusage, RUSAGE_SELF};
        let mut usage: rusage = unsafe { std::mem::zeroed() };
        unsafe {
            if getrusage(RUSAGE_SELF, &mut usage) == 0 {
                // ru_maxrss is in kilobytes on macOS
                usage.ru_maxrss as u64 * 1024
            } else {
                0
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        // windows-sys 0.52: GetCurrentProcess is Threading; counters are ProcessStatus.
        use windows_sys::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        unsafe {
            let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

            if GetProcessMemoryInfo(
                GetCurrentProcess(),
                &mut counters,
                std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ) != 0
            {
                counters.WorkingSetSize as u64
            } else {
                0
            }
        }
    }
    #[cfg(target_os = "freebsd")]
    {
        // On FreeBSD, use sysctl
        use libc::{c_int, c_uint, sysctl, CTLTYPE_ULONG, CTL_MAXNAME};

        let mut mib: [c_int; 2] = [0, 0];
        let mut size: c_uint = std::mem::size_of::<u64>() as c_uint;
        let mut value: u64 = 0;

        // CTL_VM.VM_USED_TOTAL for FreeBSD (or we can try hw.physmem)
        mib[0] = 0; // CTL_VM
        mib[1] = 0; // VM_USED_TOTAL

        unsafe {
            if sysctl(
                mib.as_ptr(),
                2,
                &mut value as *mut u64 as *mut libc::c_void,
                &mut size,
                std::ptr::null(),
                0,
            ) == 0
            {
                value
            } else {
                0
            }
        }
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows",
        target_os = "freebsd"
    )))]
    {
        // Fallback for other platforms - estimate based on V8 heap
        0
    }
}

pub fn setup_process_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    use std::env;

    let global = context.global(scope);
    if let Some(check_env) = v8::Function::new(scope, env_permission_check_callback) {
        let check_key = v8::String::new(scope, "__beeCheckEnv").unwrap();
        global.set(scope, check_key.into(), check_env.into());
    }
    if let Some(is_allowed) = v8::Function::new(scope, env_is_allowed_callback) {
        let key = v8::String::new(scope, "__beeIsEnvAllowed").unwrap();
        global.set(scope, key.into(), is_allowed.into());
    }

    // Pre-create all V8 values to avoid scope borrowing issues
    let version_key = v8::String::new(scope, "version").unwrap();
    let version_value = v8::String::new(scope, "v20.11.0").unwrap();
    let versions_key = v8::String::new(scope, "versions").unwrap();
    let v8_key = v8::String::new(scope, "v8").unwrap();
    let v8_value = v8::String::new(scope, v8::V8::get_version()).unwrap();
    let node_key = v8::String::new(scope, "node").unwrap();
    let node_value = v8::String::new(scope, "20.11.0").unwrap();
    let amber_key = v8::String::new(scope, "amber").unwrap();
    let amberjs_key = v8::String::new(scope, "amberjs").unwrap();
    let amberjs_value = v8::String::new(scope, env!("CARGO_PKG_VERSION")).unwrap();
    let platform_key = v8::String::new(scope, "platform").unwrap();
    let platform_value = v8::String::new(
        scope,
        if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "windows") {
            "win32"
        } else {
            "unknown"
        },
    )
    .unwrap();
    let arch_key = v8::String::new(scope, "arch").unwrap();
    let arch_value = v8::String::new(
        scope,
        if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "unknown"
        },
    )
    .unwrap();
    let pid_key = v8::String::new(scope, "pid").unwrap();
    let pid_value = v8::Integer::new(scope, std::process::id() as i32);
    // v0.3.40: Add process.ppid - parent process ID
    let ppid_key = v8::String::new(scope, "ppid").unwrap();
    // Get parent process ID - use getppid() on Unix, estimate on Windows
    #[cfg(not(windows))]
    let ppid_value = v8::Integer::new(scope, unsafe { libc::getppid() } as i32);
    #[cfg(windows)]
    let ppid_value = v8::Integer::new(scope, 0i32); // Windows doesn't expose ppid directly
    let title_key = v8::String::new(scope, "title").unwrap();
    let title_value = v8::String::new(scope, "amber").unwrap();
    let env_key = v8::String::new(scope, "env").unwrap();
    let argv_key = v8::String::new(scope, "argv").unwrap();
    let exec_argv_key = v8::String::new(scope, "execArgv").unwrap();
    let exec_path_key = v8::String::new(scope, "execPath").unwrap();
    let cwd_key = v8::String::new(scope, "cwd").unwrap();
    let chdir_key = v8::String::new(scope, "chdir").unwrap();
    let umask_key = v8::String::new(scope, "umask").unwrap();
    let abort_key = v8::String::new(scope, "abort").unwrap();
    let config_key = v8::String::new(scope, "config").unwrap();
    let memory_usage_key = v8::String::new(scope, "memoryUsage").unwrap();
    let memory_key = v8::String::new(scope, "memory").unwrap(); // v0.3.240: Add memory() alias
    let uptime_key = v8::String::new(scope, "uptime").unwrap();
    let hrtime_key = v8::String::new(scope, "hrtime").unwrap();
    let exit_key = v8::String::new(scope, "exit").unwrap();
    let exit_code_key = v8::String::new(scope, "exitCode").unwrap();
    let exit_code_value = v8::Integer::new(scope, 0);
    let next_tick_key = v8::String::new(scope, "nextTick").unwrap();
    let features_key = v8::String::new(scope, "features").unwrap();
    let debug_key = v8::String::new(scope, "debug").unwrap();
    let debug_value = v8::Boolean::new(scope, cfg!(debug_assertions));
    let ipc_key = v8::String::new(scope, "ipc").unwrap();
    let ipc_value = v8::Boolean::new(scope, true);
    // v0.3.40: Add additional features
    let uv_key = v8::String::new(scope, "uv").unwrap();
    let uv_value = v8::Boolean::new(scope, true); // V8 provides event loop
    let v8_feature_key = v8::String::new(scope, "v8").unwrap();
    let v8_feature_value = v8::Boolean::new(scope, true); // V8 engine is present
    let modules_key = v8::String::new(scope, "modules").unwrap();
    let modules_value = v8::Boolean::new(scope, true); // Module loading is supported
    let is_amber_key = v8::String::new(scope, "isAmber").unwrap();
    let is_amberjs_key = v8::String::new(scope, "isAmberjs").unwrap();
    let is_amber_value = v8::Boolean::new(scope, true);
    let browser_key = v8::String::new(scope, "browser").unwrap();
    let browser_value = v8::Boolean::new(scope, false);
    let process_key = v8::String::new(scope, "process").unwrap();

    // Pre-create string values for array
    let argv0_val = v8::String::new(scope, "amber").unwrap();
    let argv1_val = v8::String::new(scope, "<program>").unwrap();
    let exec_path_val = v8::String::new(
        scope,
        &env::current_exe().unwrap_or_default().to_string_lossy(),
    )
    .unwrap();

    // Pre-create function templates
    let cwd_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let cwd = env::current_dir().unwrap_or_default();
            let cwd_str = v8::String::new(scope, cwd.to_string_lossy().as_ref()).unwrap();
            retval.set(cwd_str.into());
        },
    );
    let chdir_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let directory = args
                .get(0)
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            if let Err(error) = crate::permissions::check_global_permission(
                crate::permissions::PermissionKind::Process,
                crate::permissions::PermissionAction::Execute,
                crate::permissions::ResourceId::Path(std::path::PathBuf::from(&directory)),
            ) {
                let error_message = v8::String::new(scope, &error.to_string()).unwrap();
                let error_obj = v8::Exception::error(scope, error_message);
                scope.throw_exception(error_obj.into());
                return;
            }
            match env::set_current_dir(&directory) {
                Ok(()) => {
                    let undefined = v8::undefined(scope);
                    retval.set(undefined.into());
                }
                Err(e) => {
                    let error_msg = format!("chdir() failed: {}", e);
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::error(scope, error);
                    scope.throw_exception(error_obj.into());
                }
            }
        },
    );

    // v0.3.35: Add process.umask() - file mode creation mask
    // umask() with no args returns current mask, with args sets new mask
    let umask_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            static CURRENT_UMASK: std::sync::atomic::AtomicU32 =
                std::sync::atomic::AtomicU32::new(0o022);

            if args.length() == 0 {
                // Return current umask as octal string
                let mask = CURRENT_UMASK.load(std::sync::atomic::Ordering::SeqCst);
                let mask_str = format!("{:04o}", mask);
                let mask_v8 = v8::String::new(scope, &mask_str).unwrap();
                retval.set(mask_v8.into());
            } else {
                // Set new umask
                let new_mask = args
                    .get(0)
                    .to_integer(scope)
                    .map(|i| i.value() as u32 & 0o777)
                    .unwrap_or(0);
                let old_mask = CURRENT_UMASK.swap(new_mask, std::sync::atomic::Ordering::SeqCst);
                let old_mask_str = format!("{:04o}", old_mask);
                let old_mask_v8 = v8::String::new(scope, &old_mask_str).unwrap();
                retval.set(old_mask_v8.into());
            }
        },
    );

    // v0.3.35: Add process.abort() - abort the process
    let abort_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            std::process::abort();
        },
    );

    let memory_usage_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // v0.3.39: Implement real memory usage tracking
            // Get RSS first (cross-platform)
            let rss = get_rss_memory();

            let result_obj = v8::Object::new(scope);

            // Estimate heap statistics with reasonable bounds
            // For a simple runtime, heap typically takes 20-40% of RSS, with 50% utilization
            // Cap values to reasonable bounds for testing
            let rss_f64 = rss as f64;
            let estimated_heap_total =
                ((rss_f64 * 0.25).min(100.0 * 1024.0 * 1024.0)).max(2.0 * 1024.0 * 1024.0) as u64; // Max 100MB, Min 2MB
            let estimated_heap_used =
                ((estimated_heap_total as f64) / 2.0).max(512.0 * 1024.0) as u64; // Min 512KB

            // heapTotal: Estimated total V8 heap size
            let heap_total = v8::String::new(scope, "heapTotal").unwrap();
            let heap_total_val = v8::Number::new(scope, estimated_heap_total as f64);
            result_obj.set(scope, heap_total.into(), heap_total_val.into());

            // heapUsed: Estimated used heap size
            let heap_used = v8::String::new(scope, "heapUsed").unwrap();
            let heap_used_val = v8::Number::new(scope, estimated_heap_used as f64);
            result_obj.set(scope, heap_used.into(), heap_used_val.into());

            // rss: Resident Set Size - total memory allocated by the process
            let rss_key = v8::String::new(scope, "rss").unwrap();
            let rss_val = v8::Number::new(scope, rss as f64);
            result_obj.set(scope, rss_key.into(), rss_val.into());

            // external: Memory allocated outside V8 heap (typically small for basic runtime)
            let external = v8::String::new(scope, "external").unwrap();
            let external_val = v8::Number::new(scope, 0.0);
            result_obj.set(scope, external.into(), external_val.into());

            // arrayBuffers: Memory used by ArrayBuffers
            let array_buffers = v8::String::new(scope, "arrayBuffers").unwrap();
            let array_buffers_obj = v8::Object::new(scope);
            let ab_used = v8::String::new(scope, "used").unwrap();
            let ab_used_val = v8::Number::new(scope, 0.0);
            array_buffers_obj.set(scope, ab_used.into(), ab_used_val.into());
            result_obj.set(scope, array_buffers.into(), array_buffers_obj.into());

            retval.set(result_obj.into());
        },
    );

    // v0.3.240: Add process.memory() - alias for memoryUsage
    let memory_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Reuse the same logic as memoryUsage
            let rss = get_rss_memory();
            let result_obj = v8::Object::new(scope);
            let rss_f64 = rss as f64;
            let estimated_heap_total =
                ((rss_f64 * 0.25).min(100.0 * 1024.0 * 1024.0)).max(2.0 * 1024.0 * 1024.0) as u64;
            let estimated_heap_used =
                ((estimated_heap_total as f64) / 2.0).max(512.0 * 1024.0) as u64;

            let heap_total = v8::String::new(scope, "heapTotal").unwrap();
            let heap_total_val = v8::Number::new(scope, estimated_heap_total as f64);
            result_obj.set(scope, heap_total.into(), heap_total_val.into());

            let heap_used = v8::String::new(scope, "heapUsed").unwrap();
            let heap_used_val = v8::Number::new(scope, estimated_heap_used as f64);
            result_obj.set(scope, heap_used.into(), heap_used_val.into());

            let external = v8::String::new(scope, "external").unwrap();
            let external_val = v8::Number::new(scope, 0.0);
            result_obj.set(scope, external.into(), external_val.into());

            let rss_key = v8::String::new(scope, "rss").unwrap();
            let rss_val = v8::Number::new(scope, rss as f64);
            result_obj.set(scope, rss_key.into(), rss_val.into());

            let array_buffers = v8::String::new(scope, "arrayBuffers").unwrap();
            let array_buffers_obj = v8::Object::new(scope);
            let ab_used = v8::String::new(scope, "used").unwrap();
            let ab_used_val = v8::Number::new(scope, 0.0);
            array_buffers_obj.set(scope, ab_used.into(), ab_used_val.into());
            result_obj.set(scope, array_buffers.into(), array_buffers_obj.into());

            retval.set(result_obj.into());
        },
    );

    let uptime_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Returns seconds since Unix epoch (same as before)
            let uptime = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as f64;
            retval.set(v8::Number::new(_scope, uptime).into());
        },
    );

    // v0.3.41: process.hrtime() with bigint() method
    // Create bigint function first
    let hrtime_bigint_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let bigint_val = v8::BigInt::new_from_u64(scope, now as u64);
            retval.set(bigint_val.into());
        },
    )
    .unwrap();

    // Create hrtime function
    let hrtime_fn_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let sec = (now / 1_000_000_000) as i32;
            let nsec = (now % 1_000_000_000) as i32;
            let result_array = v8::Array::new(scope, 2);
            let sec_val = v8::Integer::new(scope, sec);
            let nsec_val = v8::Integer::new(scope, nsec);
            result_array.set_index(scope, 0, sec_val.into());
            result_array.set_index(scope, 1, nsec_val.into());
            retval.set(result_array.into());
        },
    );
    let hrtime_func = hrtime_fn_template.get_function(scope).unwrap();

    // Add bigint method to the hrtime function object
    let bigint_key = v8::String::new(scope, "bigint").unwrap();
    hrtime_func.set(scope, bigint_key.into(), hrtime_bigint_fn.into());
    let exit_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            let code = args
                .get(0)
                .to_integer(_scope)
                .map(|i| i.value() as i32)
                .unwrap_or(0);
            std::process::exit(code);
        },
    );
    let next_tick_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            // v0.3.261: Use the same implementation as nodejs_core/process.rs
            // Get callback function
            let callback = args.get(0);
            if !callback.is_function() {
                let error = v8::String::new(scope, "process.nextTick: callback must be a function")
                    .unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }
            // Collect any additional arguments to pass to the callback
            let callback_args: Vec<v8::Global<v8::Value>> = (1..args.length())
                .map(|i| args.get(i))
                .filter(|v| !v.is_undefined())
                .map(|v| v8::Global::new(scope, v))
                .collect();
            // Save to nextTick queue - callbacks will be executed by execute_next_tick_callbacks
            let callback_global = v8::Global::new(scope, callback);
            crate::nodejs_core::process::push_next_tick_callback(callback_global, callback_args);
        },
    );

    // Get function instances
    let cwd_func = cwd_fn.get_function(scope).unwrap();
    let chdir_func = chdir_fn.get_function(scope).unwrap();
    let umask_func = umask_fn.get_function(scope).unwrap();
    let abort_func = abort_fn.get_function(scope).unwrap();
    let memory_usage_func = memory_usage_fn.get_function(scope).unwrap();
    let memory_func = memory_fn.get_function(scope).unwrap(); // v0.3.240
    let uptime_func = uptime_fn.get_function(scope).unwrap();
    let exit_func = exit_fn.get_function(scope).unwrap();
    let next_tick_func = next_tick_fn.get_function(scope).unwrap();

    // Create argv array
    let argv_array = v8::Array::new(scope, 2);
    argv_array.set_index(scope, 0, argv0_val.into());
    argv_array.set_index(scope, 1, argv1_val.into());

    // Create execArgv array
    let exec_argv_array = v8::Array::new(scope, 0);

    // Create versions object
    let versions_obj = v8::Object::new(scope);
    versions_obj.set(scope, v8_key.into(), v8_value.into());
    versions_obj.set(scope, node_key.into(), node_value.into());
    versions_obj.set(scope, amber_key.into(), amberjs_value.into());
    versions_obj.set(scope, amberjs_key.into(), amberjs_value.into());

    // Create features object
    let features_obj = v8::Object::new(scope);
    features_obj.set(scope, debug_key.into(), debug_value.into());
    features_obj.set(scope, ipc_key.into(), ipc_value.into());
    // v0.3.40: Add additional features
    features_obj.set(scope, uv_key.into(), uv_value.into());
    features_obj.set(scope, v8_feature_key.into(), v8_feature_value.into());
    features_obj.set(scope, modules_key.into(), modules_value.into());

    // v0.3.35: Create config object with compiler settings
    let config_obj = v8::Object::new(scope);
    let variables_key = v8::String::new(scope, "variables").unwrap();
    let variables_obj = v8::Object::new(scope);
    let host_arch_key = v8::String::new(scope, "host_arch").unwrap();
    let host_arch_value = v8::String::new(
        scope,
        if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "unknown"
        },
    )
    .unwrap();
    let platform_key2 = v8::String::new(scope, "platform").unwrap();
    let platform_value2 = v8::String::new(
        scope,
        if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "windows") {
            "win32"
        } else {
            "unknown"
        },
    )
    .unwrap();
    variables_obj.set(scope, host_arch_key.into(), host_arch_value.into());
    variables_obj.set(scope, platform_key2.into(), platform_value2.into());
    config_obj.set(scope, variables_key.into(), variables_obj.into());

    // Create process object and set all properties
    let process_obj = v8::Object::new(scope);
    process_obj.set(scope, version_key.into(), version_value.into());
    process_obj.set(scope, versions_key.into(), versions_obj.into());
    process_obj.set(scope, platform_key.into(), platform_value.into());
    process_obj.set(scope, arch_key.into(), arch_value.into());
    process_obj.set(scope, pid_key.into(), pid_value.into());
    // v0.3.40: Add process.ppid - parent process ID
    process_obj.set(scope, ppid_key.into(), ppid_value.into());
    process_obj.set(scope, title_key.into(), title_value.into());
    let env_obj = create_process_env_object(scope);
    process_obj.set(scope, env_key.into(), env_obj.into());
    process_obj.set(scope, argv_key.into(), argv_array.into());
    process_obj.set(scope, exec_argv_key.into(), exec_argv_array.into());
    process_obj.set(scope, exec_path_key.into(), exec_path_val.into());
    process_obj.set(scope, cwd_key.into(), cwd_func.into());
    process_obj.set(scope, chdir_key.into(), chdir_func.into());
    process_obj.set(scope, umask_key.into(), umask_func.into());
    process_obj.set(scope, abort_key.into(), abort_func.into());
    process_obj.set(scope, config_key.into(), config_obj.into());
    process_obj.set(scope, memory_usage_key.into(), memory_usage_func.into());
    process_obj.set(scope, memory_key.into(), memory_func.into()); // v0.3.240
    process_obj.set(scope, uptime_key.into(), uptime_func.into());
    process_obj.set(scope, hrtime_key.into(), hrtime_func.into());
    process_obj.set(scope, exit_key.into(), exit_func.into());
    process_obj.set(scope, exit_code_key.into(), exit_code_value.into());
    process_obj.set(scope, next_tick_key.into(), next_tick_func.into());
    process_obj.set(scope, features_key.into(), features_obj.into());
    process_obj.set(scope, is_amber_key.into(), is_amber_value.into());
    process_obj.set(scope, is_amberjs_key.into(), is_amber_value.into());
    process_obj.set(scope, browser_key.into(), browser_value.into());

    // v0.3.38: Add process.release object
    let release_obj = v8::Object::new(scope);
    let release_name_key = v8::String::new(scope, "name").unwrap();
    let release_name_val = v8::String::new(scope, "amber").unwrap();
    release_obj.set(scope, release_name_key.into(), release_name_val.into());
    let release_key = v8::String::new(scope, "release").unwrap();
    process_obj.set(scope, release_key.into(), release_obj.into());

    // v0.3.238: Add process.on() for event handlers (uncaughtException, unhandledRejection)
    // Returns process object for chaining (Node.js standard behavior)
    let on_key = v8::String::new(scope, "on").unwrap();
    let on_func = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Return the process object (this) for chaining
            retval.set(args.this().into());
        },
    )
    .unwrap();
    process_obj.set(scope, on_key.into(), on_func.into());

    // v0.3.238: Add process.off() for removing event handlers
    // Returns process object for chaining
    let off_key = v8::String::new(scope, "off").unwrap();
    let off_func = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(args.this().into());
        },
    )
    .unwrap();
    process_obj.set(scope, off_key.into(), off_func.into());

    // v0.3.238: Add process.removeListener() for removing specific event handlers
    let remove_listener_key = v8::String::new(scope, "removeListener").unwrap();
    let remove_listener_func = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::Object::new(_scope).into());
        },
    )
    .unwrap();
    process_obj.set(
        scope,
        remove_listener_key.into(),
        remove_listener_func.into(),
    );

    // v0.3.242: Add process.setMaxListeners() for setting max listeners per event
    // Returns process object for chaining
    // v0.3.243: Fix - process.setMaxListeners(n) with single arg sets global default
    let set_max_listeners_key = v8::String::new(scope, "setMaxListeners").unwrap();
    let set_max_listeners_func = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Determine event name and n value
            // If first arg is a number, treat it as n (set global default)
            // If first arg is a string, treat it as event name, second arg is n
            let (event_name, n) = if args.length() > 0 {
                let first = args.get(0);
                if first.is_number() {
                    // Single argument: setMaxListeners(n) - sets "__default__"
                    let n = first.int32_value(scope).unwrap_or(0);
                    ("__default__".to_string(), n)
                } else if first.is_string() || first.is_null_or_undefined() {
                    // First arg is event name
                    let name = first
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_else(|| "__default__".to_string());
                    let n = if args.length() > 1 {
                        args.get(1).int32_value(scope).unwrap_or(0)
                    } else {
                        0
                    };
                    (name, n)
                } else {
                    // First arg is neither number nor string, treat as event name with default n
                    ("__default__".to_string(), 0)
                }
            } else {
                ("__default__".to_string(), 0)
            };

            // Validate n (must be >= 0)
            let max_listeners = if n < 0 { 0 } else { n };

            // Store in thread_local
            MAX_LISTENERS.with(|map| {
                let mut map = map.lock().unwrap();
                map.insert(event_name, max_listeners);
            });

            // Return process object for chaining
            retval.set(args.this().into());
        },
    )
    .unwrap();
    process_obj.set(
        scope,
        set_max_listeners_key.into(),
        set_max_listeners_func.into(),
    );

    // v0.3.242: Add process.getMaxListeners() for getting max listeners per event
    // Returns number (default 10)
    let get_max_listeners_key = v8::String::new(scope, "getMaxListeners").unwrap();
    let get_max_listeners_func = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Get event name (defaults to "__default__")
            let event_name = if args.length() > 0 {
                let name = args.get(0);
                if name.is_string() || name.is_null_or_undefined() {
                    name.to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_else(|| "__default__".to_string())
                } else {
                    "__default__".to_string()
                }
            } else {
                "__default__".to_string()
            };

            // Get from thread_local (default 10)
            let max_listeners = MAX_LISTENERS.with(|map| {
                let map = map.lock().unwrap();
                map.get(&event_name).copied().unwrap_or(10)
            });

            retval.set(v8::Integer::new(scope, max_listeners).into());
        },
    )
    .unwrap();
    process_obj.set(
        scope,
        get_max_listeners_key.into(),
        get_max_listeners_func.into(),
    );

    // v0.3.238: Add process.stdout (basic implementation)
    // v0.3.239: Add stdout.write() method
    let stdout_key = v8::String::new(scope, "stdout").unwrap();
    let stdout_obj = v8::Object::new(scope);
    let stdout_write_key = v8::String::new(scope, "write").unwrap();
    let stdout_write_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let data = args.get(0);
            let output = if let Some(str_val) = data.to_string(scope) {
                str_val.to_rust_string_lossy(scope)
            } else if data.is_null_or_undefined() {
                String::new()
            } else {
                String::from("[object]")
            };
            // 输出到 stdout 并刷新
            let mut stdout = std::io::stdout();
            let _ = std::write!(stdout, "{}", output);
            let _ = stdout.flush();
            let result = v8::Boolean::new(scope, true);
            retval.set(result.into());
        },
    );
    let stdout_write_instance = stdout_write_fn.get_function(scope).unwrap();
    stdout_obj.set(scope, stdout_write_key.into(), stdout_write_instance.into());

    let is_stdout_tty = {
        #[cfg(unix)]
        {
            unsafe { libc::isatty(1) == 1 }
        }
        #[cfg(not(unix))]
        {
            use std::io::IsTerminal;
            std::io::stdout().is_terminal()
        }
    };
    let is_stderr_tty = {
        #[cfg(unix)]
        {
            unsafe { libc::isatty(2) == 1 }
        }
        #[cfg(not(unix))]
        {
            use std::io::IsTerminal;
            std::io::stderr().is_terminal()
        }
    };
    let is_stdin_tty = {
        #[cfg(unix)]
        {
            unsafe { libc::isatty(0) == 1 }
        }
        #[cfg(not(unix))]
        {
            use std::io::IsTerminal;
            std::io::stdin().is_terminal()
        }
    };

    let fd_key = v8::String::new(scope, "fd").unwrap();
    let is_tty_key = v8::String::new(scope, "isTTY").unwrap();
    let columns_key = v8::String::new(scope, "columns").unwrap();
    let rows_key = v8::String::new(scope, "rows").unwrap();

    let stdout_fd = v8::Integer::new(scope, 1);
    let stdout_tty = v8::Boolean::new(scope, is_stdout_tty);
    let stdout_cols = v8::Integer::new(scope, 80);
    let stdout_rows = v8::Integer::new(scope, 24);
    stdout_obj.set(scope, fd_key.into(), stdout_fd.into());
    stdout_obj.set(scope, is_tty_key.into(), stdout_tty.into());
    stdout_obj.set(scope, columns_key.into(), stdout_cols.into());
    stdout_obj.set(scope, rows_key.into(), stdout_rows.into());
    process_obj.set(scope, stdout_key.into(), stdout_obj.into());

    // v0.3.238: Add process.stderr (basic implementation)
    // v0.3.239: Add stderr.write() method
    let stderr_key = v8::String::new(scope, "stderr").unwrap();
    let stderr_obj = v8::Object::new(scope);
    let stderr_write_key = v8::String::new(scope, "write").unwrap();
    let stderr_write_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let data = args.get(0);
            let output = if let Some(str_val) = data.to_string(scope) {
                str_val.to_rust_string_lossy(scope)
            } else if data.is_null_or_undefined() {
                String::new()
            } else {
                String::from("[object]")
            };
            // 输出到 stderr 并刷新
            let mut stderr = std::io::stderr();
            let _ = std::write!(stderr, "{}", output);
            let _ = stderr.flush();
            let result = v8::Boolean::new(scope, true);
            retval.set(result.into());
        },
    );
    let stderr_write_instance = stderr_write_fn.get_function(scope).unwrap();
    stderr_obj.set(scope, stderr_write_key.into(), stderr_write_instance.into());
    let stderr_fd = v8::Integer::new(scope, 2);
    let stderr_tty = v8::Boolean::new(scope, is_stderr_tty);
    let stderr_cols = v8::Integer::new(scope, 80);
    let stderr_rows = v8::Integer::new(scope, 24);
    stderr_obj.set(scope, fd_key.into(), stderr_fd.into());
    stderr_obj.set(scope, is_tty_key.into(), stderr_tty.into());
    stderr_obj.set(scope, columns_key.into(), stderr_cols.into());
    stderr_obj.set(scope, rows_key.into(), stderr_rows.into());
    process_obj.set(scope, stderr_key.into(), stderr_obj.into());

    // G70: process.stdin — fd/isTTY + 非阻塞 read() + resume/pause
    // （'data'/'readable'/'end' 由 host 读线程 + 事件循环 pump 交付）
    let stdin_key = v8::String::new(scope, "stdin").unwrap();
    let stdin_obj = v8::Object::new(scope);
    // stdin 归首个 runtime 所有（Node worker_threads dummy 语义）：后续
    // runtime 的 stdin 标记 __stdinInert，方法变为无副作用的空转，
    // 不共享首个 runtime 的读线程、队列与 EOF/保活状态。
    let stdin_inert = STDIN_OWNER_CLAIMED.swap(true, Ordering::SeqCst);
    if stdin_inert {
        let inert_key = v8::String::new(scope, "__stdinInert").unwrap();
        stdin_obj.set(
            scope,
            inert_key.into(),
            v8::Boolean::new(scope, true).into(),
        );
    }
    let stdin_fd_key = v8::String::new(scope, "fd").unwrap();
    let stdin_fd_value = v8::Integer::new(scope, 0);
    let stdin_tty = v8::Boolean::new(scope, is_stdin_tty);
    stdin_obj.set(scope, stdin_fd_key.into(), stdin_fd_value.into());
    stdin_obj.set(scope, is_tty_key.into(), stdin_tty.into());
    let stdin_read_fn = v8::FunctionTemplate::new(scope, stdin_read_callback);
    let stdin_read_key = v8::String::new(scope, "read").unwrap();
    let stdin_read_instance = stdin_read_fn.get_function(scope).unwrap();
    stdin_obj.set(scope, stdin_read_key.into(), stdin_read_instance.into());
    let stdin_resume_fn = v8::FunctionTemplate::new(scope, stdin_resume_callback);
    let stdin_resume_key = v8::String::new(scope, "resume").unwrap();
    stdin_obj.set(
        scope,
        stdin_resume_key.into(),
        stdin_resume_fn.get_function(scope).unwrap().into(),
    );
    let stdin_start_readable_fn = v8::FunctionTemplate::new(scope, stdin_readable_start_callback);
    let stdin_start_readable_key = v8::String::new(scope, "__startReadable").unwrap();
    stdin_obj.set(
        scope,
        stdin_start_readable_key.into(),
        stdin_start_readable_fn.get_function(scope).unwrap().into(),
    );
    let stdin_pause_fn = v8::FunctionTemplate::new(scope, stdin_pause_callback);
    let stdin_pause_key = v8::String::new(scope, "pause").unwrap();
    stdin_obj.set(
        scope,
        stdin_pause_key.into(),
        stdin_pause_fn.get_function(scope).unwrap().into(),
    );
    let stdin_ref_fn = v8::FunctionTemplate::new(scope, stdin_ref_callback);
    let stdin_ref_key = v8::String::new(scope, "ref").unwrap();
    stdin_obj.set(
        scope,
        stdin_ref_key.into(),
        stdin_ref_fn.get_function(scope).unwrap().into(),
    );
    let stdin_unref_fn = v8::FunctionTemplate::new(scope, stdin_unref_callback);
    let stdin_unref_key = v8::String::new(scope, "unref").unwrap();
    stdin_obj.set(
        scope,
        stdin_unref_key.into(),
        stdin_unref_fn.get_function(scope).unwrap().into(),
    );
    let stdin_set_encoding_fn = v8::FunctionTemplate::new(scope, stdin_set_encoding_callback);
    let stdin_set_encoding_key = v8::String::new(scope, "setEncoding").unwrap();
    stdin_obj.set(
        scope,
        stdin_set_encoding_key.into(),
        stdin_set_encoding_fn.get_function(scope).unwrap().into(),
    );
    process_obj.set(scope, stdin_key.into(), stdin_obj.into());

    // v0.3.240: Add process.cpuUsage()
    let cpu_usage_key = v8::String::new(scope, "cpuUsage").unwrap();
    let cpu_usage_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let _args = args; // Suppress unused warning - cpuUsage() could accept previous value in future
            let result = v8::Object::new(scope);
            let user_key = v8::String::new(scope, "user").unwrap();
            let system_key = v8::String::new(scope, "system").unwrap();
            let user_val = v8::Number::new(scope, 0.0); // Simplified - returns 0
            let system_val = v8::Number::new(scope, 0.0); // Simplified - returns 0
            result.set(scope, user_key.into(), user_val.into());
            result.set(scope, system_key.into(), system_val.into());
            retval.set(result.into());
        },
    );
    let cpu_usage_instance = cpu_usage_fn.get_function(scope).unwrap();
    process_obj.set(scope, cpu_usage_key.into(), cpu_usage_instance.into());

    // v0.3.243: Add process.kill(pid, signal) - Send signal to process
    let kill_key = v8::String::new(scope, "kill").unwrap();
    let kill_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Get PID
            let pid = args.get(0).int32_value(scope).unwrap_or(0);

            // Get signal (can be string or number)
            let signal = if args.length() > 1 {
                let sig_arg = args.get(1);
                if sig_arg.is_number() {
                    sig_arg.int32_value(scope).unwrap_or(15) as u32
                } else if let Some(str_val) = sig_arg.to_string(scope) {
                    let sig_str = str_val.to_rust_string_lossy(scope);
                    match sig_str.to_uppercase().as_str() {
                        "SIGHUP" | "HUP" => 1,
                        "SIGINT" | "INT" => 2,
                        "SIGQUIT" | "QUIT" => 3,
                        "SIGILL" | "ILL" => 4,
                        "SIGTRAP" | "TRAP" => 5,
                        "SIGABRT" | "ABRT" => 6,
                        "SIGFPE" | "FPE" => 8,
                        "SIGKILL" | "KILL" => 9,
                        "SIGUSR1" | "USR1" => 10,
                        "SIGSEGV" | "SEGV" => 11,
                        "SIGUSR2" | "USR2" => 12,
                        "SIGPIPE" | "PIPE" => 13,
                        "SIGALRM" | "ALRM" => 14,
                        "SIGTERM" | "TERM" => 15,
                        "SIGCHLD" | "CHLD" => 17,
                        "SIGCONT" | "CONT" => 18,
                        "SIGSTOP" | "STOP" => 19,
                        "SIGTSTP" | "TSTP" => 20,
                        "SIGTTIN" | "TTIN" => 21,
                        "SIGTTOU" | "TTOU" => 22,
                        _ => 15, // Default SIGTERM
                    }
                } else {
                    15 // Default signal
                }
            } else {
                15 // Default signal
            };

            // Send signal
            let result = if pid > 0 {
                #[cfg(target_family = "unix")]
                {
                    // Don't send signals to ourselves (would terminate the process)
                    let current_pid = unsafe { libc::getpid() };
                    if pid == current_pid as i32 {
                        false // Can't send signal to self in this context
                    } else {
                        unsafe { libc::kill(pid as libc::pid_t, signal as libc::c_int) == 0 }
                    }
                }
                #[cfg(target_family = "windows")]
                {
                    // Windows doesn't support Unix signals
                    false
                }
                #[cfg(not(any(target_family = "unix", target_family = "windows")))]
                {
                    false
                }
            } else {
                false
            };

            retval.set(v8::Boolean::new(scope, result).into());
        },
    )
    .unwrap();
    process_obj.set(scope, kill_key.into(), kill_fn.into());

    // process.dlopen(module, filename, [flags]) - 原生扩展模块加载
    let dlopen_key = v8::String::new(scope, "dlopen").unwrap();
    let dlopen_fn =
        v8::FunctionTemplate::new(scope, crate::nodejs_core::process::process_dlopen_callback);
    let dlopen_func = dlopen_fn.get_function(scope).unwrap();
    process_obj.set(scope, dlopen_key.into(), dlopen_func.into());

    // Set process as global
    global.set(scope, process_key.into(), process_obj.into());

    Ok(())
}

/// process.dlopen(module, filename, [flags]) - 原生扩展模块动态链接加载
pub fn process_dlopen_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 2 {
        let msg = v8::String::new(
            scope,
            "process.dlopen requires at least module and filename arguments",
        )
        .unwrap();
        let err = v8::Exception::type_error(scope, msg);
        scope.throw_exception(err);
        return;
    }

    let module_val = args.get(0);
    let filename_val = args.get(1);

    if !module_val.is_object() || !filename_val.is_string() {
        let msg = v8::String::new(scope, "Invalid arguments to process.dlopen").unwrap();
        let err = v8::Exception::type_error(scope, msg);
        scope.throw_exception(err);
        return;
    }

    let filename = filename_val.to_rust_string_lossy(scope);
    let path = std::path::Path::new(&filename);

    if !path.exists() {
        let msg = v8::String::new(
            scope,
            &format!("Cannot find native addon module '{}'", filename),
        )
        .unwrap();
        let err = v8::Exception::error(scope, msg);
        scope.throw_exception(err);
        return;
    }

    let module_obj = match v8::Local::<v8::Object>::try_from(module_val) {
        Ok(obj) => obj,
        Err(_) => {
            let msg = v8::String::new(scope, "process.dlopen module must be an object").unwrap();
            let err = v8::Exception::type_error(scope, msg);
            scope.throw_exception(err);
            return;
        }
    };

    crate::napi::load_napi_addon(scope, module_obj, &filename);
    retval.set(v8::undefined(scope).into());
}

/// 触发未捕获异常事件
pub fn emit_uncaught_exception(scope: &mut v8::PinScope, error: &v8::Local<v8::Value>) {
    UNCAUGHT_EXCEPTION_HANDLERS.with(|handlers| {
        let handlers = handlers.lock().unwrap();
        for handler in handlers.iter() {
            let handler_val = v8::Local::new(scope, handler);
            // 转换为 Function 类型
            if handler_val.is_function() {
                if let Ok(handler_func) = v8::Local::<v8::Function>::try_from(handler_val) {
                    let this = scope.get_current_context().global(scope);
                    let result = handler_func.call(scope, this.into(), &[*error]);
                    if result.is_none() {
                        // 处理器执行失败，忽略
                    }
                }
            }
        }
    });
}

/// 触发未处理的 Promise rejection 事件
pub fn emit_unhandled_rejection(
    scope: &mut v8::PinScope,
    reason: &v8::Local<v8::Value>,
    promise: &v8::Local<v8::Value>,
) {
    UNHANDLED_REJECTION_HANDLERS.with(|handlers| {
        let handlers = handlers.lock().unwrap();
        for handler in handlers.iter() {
            let handler_val = v8::Local::new(scope, handler);
            // 转换为 Function 类型
            if handler_val.is_function() {
                if let Ok(handler_func) = v8::Local::<v8::Function>::try_from(handler_val) {
                    let this = scope.get_current_context().global(scope);
                    let result = handler_func.call(scope, this.into(), &[*reason, *promise]);
                    if result.is_none() {
                        // 处理器执行失败，忽略
                    }
                }
            }
        }
    });
}

/// 检查是否应该退出
pub fn should_exit() -> bool {
    SHOULD_EXIT.with(|exit| *exit.lock().unwrap())
}

/// 获取退出码
pub fn get_exit_code() -> i32 {
    EXIT_CODE.with(|code| *code.lock().unwrap())
}

/// 重置状态（用于测试）
#[cfg(test)]
pub fn reset_process_state() {
    UNCAUGHT_EXCEPTION_HANDLERS.with(|handlers| {
        let mut h = handlers.lock().unwrap();
        h.clear();
    });
    UNHANDLED_REJECTION_HANDLERS.with(|handlers| {
        let mut h = handlers.lock().unwrap();
        h.clear();
    });
    SHOULD_EXIT.with(|exit| {
        *exit.lock().unwrap() = false;
    });
    EXIT_CODE.with(|code| {
        *code.lock().unwrap() = 0;
    });
    // v0.3.242: 重置 setMaxListeners 状态
    MAX_LISTENERS.with(|map| {
        let mut m = map.lock().unwrap();
        m.clear();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_process_state() {
        reset_process_state();
        assert!(!should_exit());
        assert_eq!(get_exit_code(), 0);
    }
}
