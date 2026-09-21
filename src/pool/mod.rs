// Amber Multi-Tenant Isolate Pool (amber:pool)
// High-density isolated V8 execution pool for Multi-Agent and Serverless tasks

use anyhow::{anyhow, Result};
use rusty_v8 as v8;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Pool configuration options
#[derive(Clone, Debug)]
pub struct PoolConfig {
    pub min_isolates: usize,
    pub max_isolates: usize,
    pub max_memory_mb: usize,
    pub default_timeout_ms: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_isolates: 1,
            max_isolates: 8,
            max_memory_mb: 128,
            default_timeout_ms: 10_000,
        }
    }
}

/// Statistics for an IsolatePool
#[derive(Debug, Default)]
pub struct PoolMetrics {
    pub total_created: AtomicUsize,
    pub active_tasks: AtomicUsize,
    pub tasks_completed: AtomicUsize,
    pub tasks_failed: AtomicUsize,
    pub execution_time_ns: AtomicU64,
}

type TaskResult = Result<String>;
type TaskSender = mpsc::Sender<(String, Option<u64>, mpsc::Sender<TaskResult>)>;

/// A worker thread that manages one reusable isolated runtime
struct WorkerThread {
    sender: TaskSender,
    shutdown: Arc<AtomicBool>,
}

impl WorkerThread {
    fn spawn() -> Self {
        let (tx, rx) = mpsc::channel::<(String, Option<u64>, mpsc::Sender<TaskResult>)>();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        thread::spawn(move || {
            // Lazy initialization of runtime per worker thread
            let mut runtime = match crate::runtime_minimal::MinimalRuntime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to initialize worker isolate: {}", e);
                    return;
                }
            };

            while !shutdown_clone.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Ok((code, _timeout_ms, reply_tx)) => {
                        let result = runtime
                            .execute_code_unlocked(&code)
                            .map_err(|e| anyhow!("{}", e));
                        let _ = reply_tx.send(result);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Self {
            sender: tx,
            shutdown,
        }
    }

    fn execute(&self, code: &str, timeout_ms: Option<u64>) -> Result<String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.sender
            .send((code.to_string(), timeout_ms, reply_tx))
            .map_err(|e| anyhow!("Worker channel send failed: {}", e))?;

        let timeout = Duration::from_millis(timeout_ms.unwrap_or(10_000));
        match reply_rx.recv_timeout(timeout) {
            Ok(res) => res,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                Err(anyhow!("Task execution timed out after {:?}", timeout))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(anyhow!("Worker isolate thread terminated unexpectedly"))
            }
        }
    }
}

pub struct NativeIsolatePool {
    pub config: PoolConfig,
    pub metrics: Arc<PoolMetrics>,
    workers: Mutex<Vec<WorkerThread>>,
    destroyed: AtomicBool,
}

impl NativeIsolatePool {
    pub fn new(config: PoolConfig) -> Arc<Self> {
        let metrics = Arc::new(PoolMetrics::default());
        let pool = Arc::new(Self {
            config: config.clone(),
            metrics: metrics.clone(),
            workers: Mutex::new(Vec::new()),
            destroyed: AtomicBool::new(false),
        });

        // Pre-warm minimum number of isolates
        {
            let mut workers = pool.workers.lock().unwrap();
            for _ in 0..config.min_isolates.max(1) {
                workers.push(WorkerThread::spawn());
                metrics.total_created.fetch_add(1, Ordering::Relaxed);
            }
        }

        pool
    }

    pub fn execute(&self, code: &str, timeout_ms: Option<u64>) -> Result<String> {
        if self.destroyed.load(Ordering::Relaxed) {
            return Err(anyhow!("IsolatePool has been destroyed"));
        }

        self.metrics.active_tasks.fetch_add(1, Ordering::Relaxed);
        let start = Instant::now();

        let res = {
            let mut workers = self.workers.lock().unwrap();
            let worker = if let Some(w) = workers.pop() {
                w
            } else {
                self.metrics.total_created.fetch_add(1, Ordering::Relaxed);
                WorkerThread::spawn()
            };
            drop(workers);

            let run_res = worker.execute(code, timeout_ms.or(Some(self.config.default_timeout_ms)));

            // Recycle worker back into the pool if max capacity allows
            let mut workers = self.workers.lock().unwrap();
            if workers.len() < self.config.max_isolates {
                workers.push(worker);
            } else {
                worker.shutdown.store(true, Ordering::Relaxed);
            }

            run_res
        };

        let elapsed = start.elapsed().as_nanos() as u64;
        self.metrics
            .execution_time_ns
            .fetch_add(elapsed, Ordering::Relaxed);
        self.metrics.active_tasks.fetch_sub(1, Ordering::Relaxed);

        match res {
            Ok(out) => {
                self.metrics.tasks_completed.fetch_add(1, Ordering::Relaxed);
                Ok(out)
            }
            Err(e) => {
                self.metrics.tasks_failed.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    pub fn destroy(&self) {
        self.destroyed.store(true, Ordering::Relaxed);
        let mut workers = self.workers.lock().unwrap();
        for w in workers.drain(..) {
            w.shutdown.store(true, Ordering::Relaxed);
        }
    }
}

type PoolRegistry = Arc<Mutex<HashMap<usize, Arc<NativeIsolatePool>>>>;

fn get_pool_registry() -> PoolRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<PoolRegistry> = OnceLock::new();
    REGISTRY
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

static NEXT_POOL_ID: AtomicUsize = AtomicUsize::new(1);

/// Initialize the `amber:pool` subsystem in V8 context
pub fn setup_pool_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Native pool creation callback
    let create_pool_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let mut config = PoolConfig::default();

            if args.length() > 0 && args.get(0).is_object() {
                if let Ok(obj) = v8::Local::<v8::Object>::try_from(args.get(0)) {
                    let k_min = v8::String::new(scope, "minIsolates").unwrap();
                    let k_max = v8::String::new(scope, "maxIsolates").unwrap();
                    let k_mem = v8::String::new(scope, "maxMemoryMb").unwrap();
                    let k_timeout = v8::String::new(scope, "timeoutMs").unwrap();

                    if let Some(v) = obj.get(scope, k_min.into()) {
                        if v.is_number() {
                            config.min_isolates = v.integer_value(scope).unwrap_or(1) as usize;
                        }
                    }
                    if let Some(v) = obj.get(scope, k_max.into()) {
                        if v.is_number() {
                            config.max_isolates = v.integer_value(scope).unwrap_or(8) as usize;
                        }
                    }
                    if let Some(v) = obj.get(scope, k_mem.into()) {
                        if v.is_number() {
                            config.max_memory_mb = v.integer_value(scope).unwrap_or(128) as usize;
                        }
                    }
                    if let Some(v) = obj.get(scope, k_timeout.into()) {
                        if v.is_number() {
                            config.default_timeout_ms =
                                v.integer_value(scope).unwrap_or(10_000) as u64;
                        }
                    }
                }
            }

            let pool = NativeIsolatePool::new(config);
            let pool_id = NEXT_POOL_ID.fetch_add(1, Ordering::Relaxed);

            let registry = get_pool_registry();
            registry.lock().unwrap().insert(pool_id, pool);

            rv.set(v8::Integer::new(scope, pool_id as i32).into());
        },
    )
    .unwrap();

    // Native pool execute callback (synchronous worker execution returning value)
    let run_pool_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 2 {
                let msg = v8::String::new(scope, "poolRun requires poolId and code").unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }

            let pool_id = args.get(0).int32_value(scope).unwrap_or(0) as usize;
            let code = args.get(1).to_rust_string_lossy(scope);

            let timeout_ms = if args.length() > 2 && args.get(2).is_number() {
                Some(args.get(2).integer_value(scope).unwrap_or(0) as u64)
            } else {
                None
            };

            let pool = {
                let registry = get_pool_registry();
                let guard = registry.lock().unwrap();
                match guard.get(&pool_id) {
                    Some(p) => p.clone(),
                    None => {
                        let msg =
                            v8::String::new(scope, "Invalid poolId or pool destroyed").unwrap();
                        let exc = v8::Exception::error(scope, msg);
                        scope.throw_exception(exc);
                        return;
                    }
                }
            };

            match pool.execute(&code, timeout_ms) {
                Ok(output) => {
                    let s = v8::String::new(scope, &output).unwrap();
                    rv.set(s.into());
                }
                Err(e) => {
                    let msg =
                        v8::String::new(scope, &format!("Pool Execution Error: {}", e)).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        },
    )
    .unwrap();

    // Native pool stats callback
    let stats_pool_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let pool_id = args.get(0).int32_value(scope).unwrap_or(0) as usize;
            let pool = {
                let registry = get_pool_registry();
                let guard = registry.lock().unwrap();
                match guard.get(&pool_id) {
                    Some(p) => p.clone(),
                    None => {
                        rv.set(v8::null(scope).into());
                        return;
                    }
                }
            };

            let obj = v8::Object::new(scope);
            let active = pool.metrics.active_tasks.load(Ordering::Relaxed);
            let completed = pool.metrics.tasks_completed.load(Ordering::Relaxed);
            let failed = pool.metrics.tasks_failed.load(Ordering::Relaxed);
            let created = pool.metrics.total_created.load(Ordering::Relaxed);

            let k_active = v8::String::new(scope, "active").unwrap();
            let k_completed = v8::String::new(scope, "tasksCompleted").unwrap();
            let k_failed = v8::String::new(scope, "tasksFailed").unwrap();
            let k_total = v8::String::new(scope, "totalCreated").unwrap();

            let val_active = v8::Integer::new(scope, active as i32);
            let val_completed = v8::Integer::new(scope, completed as i32);
            let val_failed = v8::Integer::new(scope, failed as i32);
            let val_total = v8::Integer::new(scope, created as i32);

            obj.set(scope, k_active.into(), val_active.into());
            obj.set(scope, k_completed.into(), val_completed.into());
            obj.set(scope, k_failed.into(), val_failed.into());
            obj.set(scope, k_total.into(), val_total.into());

            rv.set(obj.into());
        },
    )
    .unwrap();

    // Native pool destroy callback
    let destroy_pool_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let pool_id = args.get(0).int32_value(scope).unwrap_or(0) as usize;
            let registry = get_pool_registry();
            let mut guard = registry.lock().unwrap();
            if let Some(pool) = guard.remove(&pool_id) {
                pool.destroy();
                rv.set(v8::Boolean::new(scope, true).into());
            } else {
                rv.set(v8::Boolean::new(scope, false).into());
            }
        },
    )
    .unwrap();

    // Attach native binding bag
    let native_obj = v8::Object::new(scope);
    let k_create = v8::String::new(scope, "poolCreate").unwrap();
    let k_run = v8::String::new(scope, "poolRun").unwrap();
    let k_stats = v8::String::new(scope, "poolStats").unwrap();
    let k_destroy = v8::String::new(scope, "poolDestroy").unwrap();

    native_obj.set(scope, k_create.into(), create_pool_fn.into());
    native_obj.set(scope, k_run.into(), run_pool_fn.into());
    native_obj.set(scope, k_stats.into(), stats_pool_fn.into());
    native_obj.set(scope, k_destroy.into(), destroy_pool_fn.into());

    let k_native = v8::String::new(scope, "__amber_pool_native").unwrap();
    global.set(scope, k_native.into(), native_obj.into());

    // Inject high-level user-friendly JavaScript wrapper
    let js_code = r#"
    (function() {
        const native = globalThis.__amber_pool_native;

        class IsolatePool {
            constructor(options = {}) {
                this.options = Object.assign({
                    minIsolates: 1,
                    maxIsolates: 8,
                    maxMemoryMb: 128,
                    timeoutMs: 10000
                }, options);
                this.poolId = native.poolCreate(this.options);
                this._destroyed = false;
            }

            async run(code, options = {}) {
                if (this._destroyed) {
                    throw new Error("Cannot run task: IsolatePool has been destroyed");
                }
                const timeoutMs = options.timeoutMs || this.options.timeoutMs;
                return Promise.resolve().then(() => {
                    const res = native.poolRun(this.poolId, String(code), timeoutMs);
                    try {
                        return JSON.parse(res);
                    } catch {
                        return res;
                    }
                });
            }

            stats() {
                if (this._destroyed) {
                    return { active: 0, tasksCompleted: 0, tasksFailed: 0, totalCreated: 0 };
                }
                return native.poolStats(this.poolId) || {};
            }

            destroy() {
                if (!this._destroyed) {
                    this._destroyed = true;
                    return native.poolDestroy(this.poolId);
                }
                return false;
            }
        }

        const pool = {
            IsolatePool,
            createPool: (options) => new IsolatePool(options),
            version: '1.4.0'
        };

        globalThis.__amber_pool = pool;
        globalThis.pool = pool;
    })();
    "#;

    let code_str = v8::String::new(scope, js_code).unwrap();
    let script = v8::Script::compile(scope, code_str, None)
        .ok_or_else(|| anyhow!("Failed to compile pool bootstrap script"))?;
    script
        .run(scope)
        .ok_or_else(|| anyhow!("Failed to run pool bootstrap script"))?;

    Ok(())
}
