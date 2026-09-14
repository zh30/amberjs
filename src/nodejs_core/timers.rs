// v0.3.248: Timer API implementation with async scheduling
// v0.3.249: 添加回调存储和执行机制
// v0.3.271: 添加 Timer 对象返回和 queueMicrotask 支持
// Implements setTimeout, setInterval, setImmediate and their clear counterparts
// Uses AsyncTimerManager for delay > 0 scheduling
// Architecture: static timer ID storage to avoid V8 closure size limits

use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use crate::event_loop::{get_async_timer_manager, TimerScheduleError};

/// Timer type enumeration
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerType {
    Timeout,
    Interval,
    Immediate,
}

/// Timer metadata (stored in global registry - thread-safe, no V8 handles)
#[derive(Clone, Debug)]
pub struct TimerMetadata {
    pub timer_type: TimerType,
    pub delay: u64, // in milliseconds
    pub is_unrefed: bool,
    pub epoch: u64, // v0.3.261: generation counter to distinguish stale timers
}

/// Global timer metadata registry (thread-safe, no V8 handles)
/// pub for access from integration tests
pub static TIMER_METADATA: Lazy<Mutex<HashMap<u64, TimerMetadata>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// Timer IDs scheduled by the current thread.
//
// The metadata registry and the async timer manager are process-global, but a
// V8 isolate belongs to exactly one thread. Keep-alive checks, fired-timer
// dispatch and teardown must therefore be scoped per thread: otherwise one
// thread's ref'd interval pins every other thread's event loop forever, and a
// runtime drop wipes timers other threads are still waiting on.
thread_local! {
    static OWNED_TIMER_IDS: std::cell::RefCell<HashSet<u64>> =
        std::cell::RefCell::new(HashSet::new());
}

/// Record `timer_id` as owned by the current thread.
fn track_owned_timer(timer_id: u64) {
    OWNED_TIMER_IDS.with(|ids| {
        ids.borrow_mut().insert(timer_id);
    });
}

/// Drop the current thread's ownership claim on `timer_id`.
fn forget_owned_timer(timer_id: u64) {
    OWNED_TIMER_IDS.with(|ids| {
        ids.borrow_mut().remove(&timer_id);
    });
}

/// True when `timer_id` was scheduled by the current thread.
fn owns_timer(timer_id: u64) -> bool {
    OWNED_TIMER_IDS.with(|ids| ids.borrow().contains(&timer_id))
}

/// Snapshot of the current thread's timer IDs.
///
/// Callers take this before locking `TIMER_METADATA` so the two are never held
/// at once.
fn owned_timer_ids() -> HashSet<u64> {
    OWNED_TIMER_IDS.with(|ids| ids.borrow().clone())
}

/// Take every timer ID owned by the current thread, clearing the claim.
fn take_owned_timer_ids() -> HashSet<u64> {
    OWNED_TIMER_IDS.with(|ids| std::mem::take(&mut *ids.borrow_mut()))
}

/// v0.3.261: Epoch counter to distinguish timers from different test runs
/// Incremented on cleanup to invalidate all previously scheduled timers
static TIMER_EPOCH: AtomicU64 = AtomicU64::new(0);

/// v0.3.261: Get current timer epoch
pub fn get_timer_epoch() -> u64 {
    TIMER_EPOCH.load(Ordering::SeqCst)
}

/// v0.3.261: Increment epoch to invalidate all stale timers
pub fn increment_timer_epoch() {
    TIMER_EPOCH.fetch_add(1, Ordering::SeqCst);
}

/// Timer callback storage - V8 Global handles (only accessed from V8 main thread)
/// Wrapped in a struct to allow unsafe Send/Sync (only used on main thread)
pub struct TimerCallbackStorage {
    callbacks: HashMap<u64, v8::Global<v8::Function>>,
    args: HashMap<u64, Vec<v8::Global<v8::Value>>>,
}

impl TimerCallbackStorage {
    fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
            args: HashMap::new(),
        }
    }

    fn insert(
        &mut self,
        timer_id: u64,
        callback: v8::Global<v8::Function>,
        args: Vec<v8::Global<v8::Value>>,
    ) {
        self.callbacks.insert(timer_id, callback);
        self.args.insert(timer_id, args);
    }

    fn remove(
        &mut self,
        timer_id: u64,
    ) -> Option<(v8::Global<v8::Function>, Vec<v8::Global<v8::Value>>)> {
        if let Some(callback) = self.callbacks.remove(&timer_id) {
            let args = self.args.remove(&timer_id).unwrap_or_default();
            Some((callback, args))
        } else {
            None
        }
    }
}

// SAFETY: TimerCallbackStorage is only ever accessed from the V8 main thread
// where the isolate is running. This is guaranteed by the design of the runtime.
unsafe impl Send for TimerCallbackStorage {}
unsafe impl Sync for TimerCallbackStorage {}

/// Global timer callback registry (only accessed from V8 main thread)
static TIMER_CALLBACKS: Lazy<Mutex<TimerCallbackStorage>> =
    Lazy::new(|| Mutex::new(TimerCallbackStorage::new()));

/// v0.3.250: Immediate callbacks queue - stores setImmediate callbacks
/// These execute in the next event loop iteration, after current code completes
/// v0.3.261: Store callbacks with a flag to distinguish "deferred" callbacks
/// Callbacks marked as "deferred" run in the next iteration
pub struct ImmediateCallbackStorage {
    /// Callbacks stored with deferred flag
    /// Tuple: (timer_id, callback, args, is_deferred)
    /// is_deferred = true means this callback should run in the NEXT iteration
    callbacks: Vec<(
        u64,
        v8::Global<v8::Function>,
        Vec<v8::Global<v8::Value>>,
        bool,
    )>,
}

impl ImmediateCallbackStorage {
    fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    fn push(
        &mut self,
        timer_id: u64,
        callback: v8::Global<v8::Function>,
        args: Vec<v8::Global<v8::Value>>,
        is_deferred: bool,
    ) {
        self.callbacks.push((timer_id, callback, args, is_deferred));
    }

    /// v0.3.261: Mark all non-deferred callbacks as deferred
    /// Called after executing callbacks to defer newly added ones to next iteration
    fn mark_all_as_deferred(&mut self) {
        for (_, _, _, deferred) in self.callbacks.iter_mut() {
            *deferred = true;
        }
    }

    /// Make deferred callbacks eligible again. Called at the start of each
    /// event-loop iteration; pairs with mark_all_as_deferred() so a callback
    /// registered during the check phase runs in the next iteration.
    fn unmark_all_deferred(&mut self) {
        for (_, _, _, deferred) in self.callbacks.iter_mut() {
            *deferred = false;
        }
    }

    /// v0.3.261: Drain non-deferred callbacks (those that should run now)
    fn drain_non_deferred(
        &mut self,
    ) -> Vec<(u64, v8::Global<v8::Function>, Vec<v8::Global<v8::Value>>)> {
        let (non_deferred, remaining): (Vec<_>, Vec<_>) = self
            .callbacks
            .drain(..)
            .partition(|(_, _, _, deferred)| !*deferred);

        // Put back deferred callbacks (they run next iteration)
        self.callbacks.extend(remaining);

        // Return only non-deferred callbacks for execution
        non_deferred
            .into_iter()
            .map(|(id, cb, args, _)| (id, cb, args))
            .collect()
    }

    /// v0.3.265: Drain all callbacks (reserved for future use)
    #[allow(dead_code)]
    fn drain(&mut self) -> Vec<(u64, v8::Global<v8::Function>, Vec<v8::Global<v8::Value>>)> {
        self.callbacks
            .drain(..)
            .map(|(id, cb, args, _)| (id, cb, args))
            .collect()
    }

    fn remove(&mut self, timer_id: u64) -> bool {
        if let Some(pos) = self
            .callbacks
            .iter()
            .position(|(id, _, _, _)| *id == timer_id)
        {
            self.callbacks.remove(pos);
            true
        } else {
            false
        }
    }

    fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }

    /// v0.3.261: Clear all callbacks and reset state
    fn clear_all(&mut self) {
        self.callbacks.clear();
    }
}

// SAFETY note: the immediate queue is thread-local, so no Send/Sync impl is
// required. setImmediate callbacks capture V8 Global handles from the isolate
// that scheduled them; a process-global queue lets callbacks leak into another
// isolate's event loop, where calling them deadlocks on that isolate's lock.

// Per-thread immediate callback queue (only accessed from the V8 thread that
// scheduled the callbacks)
thread_local! {
    static IMMEDIATE_CALLBACKS: std::cell::RefCell<ImmediateCallbackStorage> =
        std::cell::RefCell::new(ImmediateCallbackStorage::new());
}

/// Next timer ID counter (shared, thread-safe)
/// v0.3.261: Made public for timer validation in event loop
pub static NEXT_TIMER_ID: AtomicU64 = AtomicU64::new(1);

/// v0.3.261: Timer ID offset per epoch to prevent ID conflicts between tests
const TIMER_ID_EPOCH_OFFSET: u64 = 10000;

/// Get next timer ID (includes epoch offset for test isolation)
pub fn get_next_timer_id() -> u64 {
    let base_id = NEXT_TIMER_ID.fetch_add(1, Ordering::SeqCst);
    let epoch = get_timer_epoch();
    // Include epoch in timer ID to prevent conflicts between tests
    let id = base_id + (epoch * TIMER_ID_EPOCH_OFFSET);
    id
}

/// Get timer metadata by ID
pub fn get_timer_metadata(timer_id: u64) -> Option<TimerMetadata> {
    let metadata = TIMER_METADATA.lock().unwrap();
    metadata.get(&timer_id).cloned()
}

/// Get all timer metadata (for testing)
#[cfg(test)]
pub fn get_all_timer_metadata() -> Vec<(u64, TimerMetadata)> {
    let metadata = TIMER_METADATA.lock().unwrap();
    metadata
        .iter()
        .map(|(id, meta)| (*id, meta.clone()))
        .collect()
}

/// Remove timer metadata
pub fn remove_timer_metadata(timer_id: u64) {
    forget_owned_timer(timer_id);
    let mut metadata = TIMER_METADATA.lock().unwrap();
    metadata.remove(&timer_id);
}

fn throw_timer_schedule_error(scope: &mut v8::PinScope, err: TimerScheduleError) {
    let message = v8::String::new(scope, &format!("timer scheduling failed: {}", err)).unwrap();
    let error = v8::Exception::error(scope, message);
    scope.throw_exception(error.into());
}

fn cleanup_scheduled_timer_state(timer_id: u64) {
    remove_timer_metadata(timer_id);
    let _ = remove_timer_callback(timer_id);
}

/// Remove timer callback from registry
pub fn remove_timer_callback(
    timer_id: u64,
) -> Option<(v8::Global<v8::Function>, Vec<v8::Global<v8::Value>>)> {
    let mut storage = TIMER_CALLBACKS.lock().unwrap();
    storage.remove(timer_id)
}

fn clone_timer_callback(
    timer_id: u64,
) -> Option<(v8::Global<v8::Function>, Vec<v8::Global<v8::Value>>)> {
    let storage = TIMER_CALLBACKS.lock().unwrap();
    let callback = storage.callbacks.get(&timer_id)?.clone();
    let args = storage.args.get(&timer_id).cloned().unwrap_or_default();
    Some((callback, args))
}

/// v0.3.271: Create a Timer object with ref/unref/refresh methods
/// Timer objects are returned by setTimeout, setInterval, setImmediate
fn create_timer_object<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    timer_id: u64,
    timer_type: TimerType,
) -> v8::Local<'a, v8::Object> {
    let timer_obj = v8::Object::new(scope);

    // Store timer ID on the object
    let id_key = v8::String::new(scope, "_timerId").unwrap();
    let id_value = v8::Number::new(scope, timer_id as f64);
    timer_obj.set(scope, id_key.into(), id_value.into());

    // Store timer type
    let type_key = v8::String::new(scope, "_timerType").unwrap();
    let type_value = v8::String::new(
        scope,
        match timer_type {
            TimerType::Timeout => "Timeout",
            TimerType::Interval => "Interval",
            TimerType::Immediate => "Immediate",
        },
    )
    .unwrap();
    timer_obj.set(scope, type_key.into(), type_value.into());

    // Create unref method
    let unref_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let id_key = v8::String::new(scope, "_timerId").unwrap();
            let id_val = this.get(scope, id_key.into()).unwrap();
            let timer_id_val = id_val.to_integer(scope).unwrap().value() as u64;

            let mut metadata = TIMER_METADATA.lock().unwrap();
            if let Some(info) = metadata.get_mut(&timer_id_val) {
                info.is_unrefed = true;
            }

            // Return timer object for chaining
            retval.set(this.into());
        },
    )
    .unwrap();
    let unref_key = v8::String::new(scope, "unref").unwrap();
    timer_obj.set(scope, unref_key.into(), unref_fn.into());

    // Create ref method
    let ref_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let id_key = v8::String::new(scope, "_timerId").unwrap();
            let id_val = this.get(scope, id_key.into()).unwrap();
            let timer_id_val = id_val.to_integer(scope).unwrap().value() as u64;

            let mut metadata = TIMER_METADATA.lock().unwrap();
            if let Some(info) = metadata.get_mut(&timer_id_val) {
                info.is_unrefed = false;
            }

            // Return timer object for chaining
            retval.set(this.into());
        },
    )
    .unwrap();
    let ref_key = v8::String::new(scope, "ref").unwrap();
    timer_obj.set(scope, ref_key.into(), ref_fn.into());

    // Create refresh method (Node.js compatibility)
    let refresh_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let id_key = v8::String::new(scope, "_timerId").unwrap();
            let id_val = this.get(scope, id_key.into()).unwrap();
            let timer_id_val = id_val.to_integer(scope).unwrap().value() as u64;

            // Refresh timer by rescheduling it
            let mut metadata = TIMER_METADATA.lock().unwrap();
            if let Some(info) = metadata.get_mut(&timer_id_val) {
                let effective_delay = if info.delay == 0 { 1 } else { info.delay };
                drop(metadata);
                if let Err(err) = get_async_timer_manager().try_schedule_timeout(
                    Duration::from_millis(effective_delay),
                    timer_id_val,
                    || {},
                ) {
                    throw_timer_schedule_error(scope, err);
                    return;
                }
            }

            retval.set(this.into());
        },
    )
    .unwrap();
    let refresh_key = v8::String::new(scope, "refresh").unwrap();
    timer_obj.set(scope, refresh_key.into(), refresh_fn.into());

    // Create hasRef method (Node.js compatibility)
    let has_ref_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let id_key = v8::String::new(scope, "_timerId").unwrap();
            let id_val = this.get(scope, id_key.into()).unwrap();
            let timer_id_val = id_val.to_integer(scope).unwrap().value() as u64;

            let metadata = TIMER_METADATA.lock().unwrap();
            let is_unrefed = metadata
                .get(&timer_id_val)
                .map(|m| m.is_unrefed)
                .unwrap_or(true);
            retval.set(v8::Boolean::new(scope, !is_unrefed).into());
        },
    )
    .unwrap();
    let has_ref_key = v8::String::new(scope, "hasRef").unwrap();
    timer_obj.set(scope, has_ref_key.into(), has_ref_fn.into());

    // Add valueOf for numeric conversion (allows Number(timer))
    let value_of_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let id_key = v8::String::new(scope, "_timerId").unwrap();
            let id_val = this.get(scope, id_key.into()).unwrap();
            let timer_id_val = id_val.to_number(scope).unwrap().value();
            retval.set(v8::Number::new(scope, timer_id_val).into());
        },
    )
    .unwrap();
    let value_of_key = v8::String::new(scope, "valueOf").unwrap();
    timer_obj.set(scope, value_of_key.into(), value_of_fn.into());

    // v0.3.273: Add delay() method - get delay when no args, set delay when args provided
    let delay_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();

            // Get timer ID
            let id_key = v8::String::new(scope, "_timerId").unwrap();
            let id_val = this.get(scope, id_key.into()).unwrap();
            let timer_id_val = id_val.to_integer(scope).unwrap().value() as u64;

            if args.length() == 0 {
                // Get current delay
                let metadata = TIMER_METADATA.lock().unwrap();
                let delay = metadata.get(&timer_id_val).map(|m| m.delay).unwrap_or(0);
                retval.set(v8::Number::new(scope, delay as f64).into());
            } else {
                // Set new delay
                let new_delay = args
                    .get(0)
                    .to_integer(scope)
                    .map(|i| i.value().max(0) as u64)
                    .unwrap_or(0);

                // Update metadata delay
                {
                    let mut metadata = TIMER_METADATA.lock().unwrap();
                    if let Some(info) = metadata.get_mut(&timer_id_val) {
                        info.delay = new_delay;
                    }
                }

                // For setTimeout/setInterval with delay > 0, reschedule the timer
                // Get the timer type first
                let type_key = v8::String::new(scope, "_timerType").unwrap();
                let type_val = this.get(scope, type_key.into()).unwrap();
                let type_str = type_val
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope);

                if type_str != "Immediate" && new_delay > 0 {
                    // Reschedule with new delay
                    let effective_delay = if new_delay == 0 { 1 } else { new_delay };
                    if let Err(err) = get_async_timer_manager().try_schedule_timeout(
                        Duration::from_millis(effective_delay),
                        timer_id_val,
                        || {},
                    ) {
                        throw_timer_schedule_error(scope, err);
                        return;
                    }
                }

                // Return timer object for chaining
                retval.set(this.into());
            }
        },
    )
    .unwrap();
    let delay_key = v8::String::new(scope, "delay").unwrap();
    timer_obj.set(scope, delay_key.into(), delay_fn.into());

    timer_obj
}

/// Set up timer APIs in the V8 context
pub fn setup_timers_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<(), anyhow::Error> {
    let global = context.global(scope);

    // setTimeout - for delay = 0 executes immediately, delay > 0 uses async scheduling
    let set_timeout_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() < 1 {
                let error =
                    v8::String::new(scope, "setTimeout requires at least 1 argument").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let callback = args.get(0);
            if !callback.is_function() {
                let error =
                    v8::String::new(scope, "setTimeout: callback must be a function").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let delay = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value().max(0) as u64)
                .unwrap_or(0);

            // Collect additional arguments for the callback
            let callback_args: Vec<v8::Local<v8::Value>> =
                (2..args.length()).map(|i| args.get(i)).collect();

            // Get timer ID
            let timer_id = get_next_timer_id();

            // v0.3.261: Always schedule timer, even for delay = 0
            // In Node.js, setTimeout(fn, 0) still executes in the next event loop iteration,
            // after current synchronous code, nextTick, and microtasks complete.
            // This ensures proper execution order: nextTick -> microtasks -> timers -> setImmediate

            // Store callback in global registry before scheduling
            // Convert callback to Global<Function> and store with arguments
            let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();
            let callback_global = v8::Global::new(scope, callback_fn);
            let args_global: Vec<v8::Global<v8::Value>> = callback_args
                .iter()
                .map(|v| v8::Global::new(scope, v.clone()))
                .collect();

            // Store metadata
            track_owned_timer(timer_id);
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Timeout,
                    delay,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
            drop(metadata);

            // Store callback and args in global registry
            store_timer_callback(timer_id, callback_global, args_global);

            if delay == 0 {
                get_async_timer_manager().mark_timer_fired(timer_id);
            } else if let Err(err) = get_async_timer_manager().try_schedule_timeout(
                Duration::from_millis(delay),
                timer_id,
                || {},
            ) {
                cleanup_scheduled_timer_state(timer_id);
                throw_timer_schedule_error(scope, err);
                return;
            }

            // v0.3.271: Return Timer object instead of timer ID for Node.js compatibility
            let timer_obj = create_timer_object(scope, timer_id, TimerType::Timeout);
            retval.set(timer_obj.into());
        },
    )
    .unwrap();

    // setInterval
    let set_interval_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() < 1 {
                let error =
                    v8::String::new(scope, "setInterval requires at least 1 argument").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let callback = args.get(0);
            if !callback.is_function() {
                let error =
                    v8::String::new(scope, "setInterval: callback must be a function").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let delay = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value().max(0) as u64)
                .unwrap_or(0);
            let effective_delay = delay.max(1);

            // Get timer ID
            let timer_id = get_next_timer_id();

            // Collect arguments for the callback
            let callback_args: Vec<v8::Local<v8::Value>> =
                (2..args.length()).map(|i| args.get(i)).collect();

            // Convert callback to Global<Function> and store with arguments
            let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();
            let callback_global = v8::Global::new(scope, callback_fn);
            let args_global: Vec<v8::Global<v8::Value>> = callback_args
                .iter()
                .map(|v| v8::Global::new(scope, v.clone()))
                .collect();

            // Store metadata. 0ms intervals use a 1ms effective delay so the
            // event-loop drain limit can still bound un-cleared intervals.
            track_owned_timer(timer_id);
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Interval,
                    delay: effective_delay,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
            drop(metadata);

            // Store callback and args in global registry
            store_timer_callback(timer_id, callback_global, args_global);

            // Schedule with AsyncTimerManager
            if let Err(err) = get_async_timer_manager().try_schedule_interval(
                Duration::from_millis(effective_delay),
                0,
                timer_id,
                || {},
            ) {
                cleanup_scheduled_timer_state(timer_id);
                throw_timer_schedule_error(scope, err);
                return;
            }

            // v0.3.271: Return Timer object instead of timer ID for Node.js compatibility
            let timer_obj = create_timer_object(scope, timer_id, TimerType::Interval);
            retval.set(timer_obj.into());
        },
    )
    .unwrap();

    // setImmediate - v0.3.250: executes callback in next event loop iteration
    let set_immediate_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() < 1 {
                let error =
                    v8::String::new(scope, "setImmediate requires at least 1 argument").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let callback = args.get(0);
            if !callback.is_function() {
                let error =
                    v8::String::new(scope, "setImmediate: callback must be a function").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            // Collect additional arguments for the callback
            let callback_args: Vec<v8::Local<v8::Value>> =
                (1..args.length()).map(|i| args.get(i)).collect();

            // Get timer ID
            let timer_id = get_next_timer_id();

            // Store metadata in global registry
            track_owned_timer(timer_id);
            let mut metadata = TIMER_METADATA.lock().unwrap();
            metadata.insert(
                timer_id,
                TimerMetadata {
                    timer_type: TimerType::Immediate,
                    delay: 0,
                    is_unrefed: false,
                    epoch: get_timer_epoch(),
                },
            );
            drop(metadata);

            // v0.3.250: Store callback for next event loop iteration (not immediate execution)
            let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();
            let callback_global = v8::Global::new(scope, callback_fn);
            let args_global: Vec<v8::Global<v8::Value>> = callback_args
                .iter()
                .map(|v| v8::Global::new(scope, v.clone()))
                .collect();

            store_immediate_callback(timer_id, callback_global, args_global);

            // v0.3.271: Return Timer object instead of timer ID for Node.js compatibility
            let timer_obj = create_timer_object(scope, timer_id, TimerType::Immediate);
            retval.set(timer_obj.into());
        },
    )
    .unwrap();

    // clearTimeout / clearInterval / clearImmediate
    let clear_timer_fn = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            if args.length() < 1 {
                return;
            }

            let timer_arg = args.get(0);
            let mut timer_id: u64 = 0;

            // v0.3.271: Support both timer ID (number) and Timer object
            if timer_arg.is_number() {
                timer_id = timer_arg
                    .to_integer(_scope)
                    .map(|i| i.value() as u64)
                    .unwrap_or(0);
            } else if timer_arg.is_object() {
                // Try to get _timerId from Timer object
                let timer_obj = v8::Local::<v8::Object>::try_from(timer_arg).ok();
                if let Some(obj) = timer_obj {
                    let id_key = v8::String::new(_scope, "_timerId").unwrap();
                    if let Some(id_val) = obj.get(_scope, id_key.into()) {
                        timer_id = id_val
                            .to_integer(_scope)
                            .map(|i| i.value() as u64)
                            .unwrap_or(0);
                    }
                }
            }

            if timer_id > 0 {
                // Remove from metadata
                forget_owned_timer(timer_id);
                let mut metadata = TIMER_METADATA.lock().unwrap();
                metadata.remove(&timer_id);

                // Cancel in AsyncTimerManager
                let _ = get_async_timer_manager().cancel(timer_id);

                // Remove stored timeout/interval callback so fired-but-cleared IDs cannot run.
                let _ = remove_timer_callback(timer_id);

                // v0.3.250: Also remove from immediate callbacks if it's a setImmediate
                let _ = remove_immediate_callback(timer_id);
            }
        },
    )
    .unwrap();

    // Create string keys first to avoid mutable borrow conflicts
    // v0.3.250: timer.unref() - allow event loop to exit if this is the only timer
    let unref_fn = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() < 1 {
                retval.set(v8::Boolean::new(_scope, false).into());
                return;
            }

            let timer_id_val = args.get(0);
            let timer_id = timer_id_val
                .to_integer(_scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);

            if timer_id > 0 {
                let result = set_timer_unrefed(timer_id, true);
                retval.set(v8::Boolean::new(_scope, result).into());
            } else {
                retval.set(v8::Boolean::new(_scope, false).into());
            }
        },
    )
    .unwrap();

    // v0.3.250: timer.ref() - ensure event loop stays alive for this timer
    let ref_fn = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() < 1 {
                retval.set(v8::Boolean::new(_scope, false).into());
                return;
            }

            let timer_id_val = args.get(0);
            let timer_id = timer_id_val
                .to_integer(_scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);

            if timer_id > 0 {
                let result = set_timer_unrefed(timer_id, false);
                retval.set(v8::Boolean::new(_scope, result).into());
            } else {
                retval.set(v8::Boolean::new(_scope, false).into());
            }
        },
    )
    .unwrap();

    // v0.3.271: queueMicrotask - queue a callback as a microtask
    // This is similar to Promise.resolve().then() but without creating a Promise
    let queue_microtask_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {
            if args.length() < 1 {
                let error =
                    v8::String::new(scope, "queueMicrotask requires at least 1 argument").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            let callback = args.get(0);
            if !callback.is_function() {
                let error =
                    v8::String::new(scope, "queueMicrotask: callback must be a function").unwrap();
                let error_obj = v8::Exception::type_error(scope, error);
                scope.throw_exception(error_obj.into());
                return;
            }

            // Use V8's built-in microtask queue via enqueue_microtask
            // The microtask will be executed when perform_microtask_checkpoint is called
            // This happens in our event loop after nextTick callbacks
            let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();
            // enqueue_microtask takes Local<Function>, not Global
            // The callback will be executed at the next microtask checkpoint
            scope.enqueue_microtask(callback_fn);
        },
    )
    .unwrap();

    let set_timeout_key = v8::String::new(scope, "setTimeout").unwrap();
    let set_interval_key = v8::String::new(scope, "setInterval").unwrap();
    let set_immediate_key = v8::String::new(scope, "setImmediate").unwrap();
    let clear_timeout_key = v8::String::new(scope, "clearTimeout").unwrap();
    let clear_interval_key = v8::String::new(scope, "clearInterval").unwrap();
    let clear_immediate_key = v8::String::new(scope, "clearImmediate").unwrap();
    let unref_key = v8::String::new(scope, "unref").unwrap();
    let ref_key = v8::String::new(scope, "ref").unwrap();
    let queue_microtask_key = v8::String::new(scope, "queueMicrotask").unwrap();

    // Register functions on global object
    global.set(scope, set_timeout_key.into(), set_timeout_fn.into());
    global.set(scope, set_interval_key.into(), set_interval_fn.into());
    global.set(scope, set_immediate_key.into(), set_immediate_fn.into());
    global.set(scope, clear_timeout_key.into(), clear_timer_fn.into());
    global.set(scope, clear_interval_key.into(), clear_timer_fn.into());
    global.set(scope, clear_immediate_key.into(), clear_timer_fn.into());
    global.set(scope, unref_key.into(), unref_fn.into());
    global.set(scope, ref_key.into(), ref_fn.into());
    global.set(scope, queue_microtask_key.into(), queue_microtask_fn.into());

    Ok(())
}

/// v0.3.250: Set timer unref state
pub fn set_timer_unrefed(timer_id: u64, unrefed: bool) -> bool {
    let mut metadata = TIMER_METADATA.lock().unwrap();
    if let Some(meta) = metadata.get_mut(&timer_id) {
        meta.is_unrefed = unrefed;
        true
    } else {
        false
    }
}

/// v0.3.250: Get timer unref state
pub fn is_timer_unrefed(timer_id: u64) -> bool {
    let metadata = TIMER_METADATA.lock().unwrap();
    metadata
        .get(&timer_id)
        .map(|meta| meta.is_unrefed)
        .unwrap_or(false)
}

/// v0.3.248: Cancel the timers this thread scheduled in the AsyncTimerManager.
///
/// The manager and its fired queue are process-wide, so clearing them wholesale
/// would drop timers other threads are still waiting on. The global ID counter
/// and epoch are left untouched for the same reason: `get_next_timer_id` only
/// needs them to stay monotonic, and rewinding them mid-run makes IDs collide
/// between threads.
pub fn clear_all_async_timers() {
    let manager = get_async_timer_manager();
    for timer_id in owned_timer_ids() {
        let _ = manager.cancel(timer_id);
    }
}

/// v0.3.250: Clear the timer metadata owned by the current thread
pub fn clear_all_timers() {
    let owned = take_owned_timer_ids();
    if owned.is_empty() {
        return;
    }

    let manager = get_async_timer_manager();
    for timer_id in &owned {
        let _ = manager.cancel(*timer_id);
    }

    if let Ok(mut metadata) = TIMER_METADATA.lock() {
        metadata.retain(|id, _| !owned.contains(id));
    }
}

/// v0.3.256: Clear all timer callbacks - must be called before Isolate is disposed
/// This clears V8 Global handles to prevent "Handle hosted by disposed Isolate" errors
pub fn clear_all_timer_callbacks() {
    let mut storage = TIMER_CALLBACKS.lock().unwrap();
    storage.callbacks.clear();
    storage.args.clear();

    IMMEDIATE_CALLBACKS.with(|s| s.borrow_mut().clear_all());
}

/// v0.3.256: Complete cleanup - clears both callbacks and metadata
/// Call this before Isolate is disposed to avoid V8 handle errors
pub fn cleanup_all_timers() {
    clear_all_timer_callbacks();
    clear_all_timers();
    clear_all_async_timers();
}

/// v0.3.249: Store timer callback in global registry
/// Must be called from V8 main thread (where isolate is available)
pub fn store_timer_callback(
    timer_id: u64,
    callback: v8::Global<v8::Function>,
    args: Vec<v8::Global<v8::Value>>,
) {
    let mut storage = TIMER_CALLBACKS.lock().unwrap();
    storage.insert(timer_id, callback, args);
}

/// v0.3.249: Get and remove timer callback from registry
/// Returns None if timer_id not found or already executed
pub fn take_timer_callback(
    timer_id: u64,
) -> Option<(v8::Global<v8::Function>, Vec<v8::Global<v8::Value>>)> {
    let mut storage = TIMER_CALLBACKS.lock().unwrap();
    storage.remove(timer_id)
}

/// v0.3.249: Execute a fired timer callback
/// Must be called from V8 main thread with valid isolate scope
pub fn execute_timer_callback(scope: &mut v8::PinScope, timer_id: u64) -> bool {
    let timer_type = {
        let metadata = TIMER_METADATA.lock().unwrap();
        metadata.get(&timer_id).map(|meta| meta.timer_type)
    };

    let callback_entry = if matches!(timer_type, Some(TimerType::Interval)) {
        clone_timer_callback(timer_id)
    } else {
        take_timer_callback(timer_id)
    };

    if let Some((callback, args)) = callback_entry {
        let callback = v8::Local::<v8::Function>::new(scope, callback);

        // Convert stored args back to Local
        let args: Vec<v8::Local<v8::Value>> = args
            .into_iter()
            .map(|arg| v8::Local::new(scope, arg))
            .collect();

        let undefined = v8::undefined(scope);
        let result = callback.call(scope, undefined.into(), &args);

        // Remove from metadata (for non-interval timers)
        let metadata = TIMER_METADATA.lock().unwrap();
        if let Some(meta) = metadata.get(&timer_id) {
            if meta.timer_type != TimerType::Interval {
                drop(metadata);
                remove_timer_metadata(timer_id);
            }
        }

        result.is_some()
    } else {
        false
    }
}

/// v0.3.249: Execute all fired timer callbacks
/// Called from V8 main thread event loop
/// v0.3.261: Timer IDs now include epoch offset, so no epoch check needed
pub fn execute_fired_timers(scope: &mut v8::PinScope) {
    let timer_manager = get_async_timer_manager();
    let fired_timers = timer_manager.poll_fired_timers();

    for timer_id in fired_timers {
        let timer_is_active = {
            let metadata = TIMER_METADATA.lock().unwrap();
            metadata.contains_key(&timer_id)
        };
        if !timer_is_active {
            let _ = remove_timer_callback(timer_id);
            continue;
        }

        // The fired queue is process-wide. A callback belongs to the isolate of
        // the thread that scheduled it, so hand foreign timers back instead of
        // invoking their V8 handles on this isolate.
        if !owns_timer(timer_id) {
            timer_manager.mark_timer_fired(timer_id);
            continue;
        }

        // Skip if timer callback doesn't exist (was already executed or cleared)
        let callback_exists = {
            let storage = TIMER_CALLBACKS.lock().unwrap();
            storage.callbacks.contains_key(&timer_id)
        };

        if !callback_exists {
            continue;
        }

        // Execute callback
        let _ = execute_timer_callback(scope, timer_id);
    }
}

/// v0.3.250: Execute all pending setImmediate callbacks
/// v0.3.261: Modified to only execute non-deferred callbacks
/// This ensures setImmediate callbacks registered during sync code run in same iteration,
/// while callbacks registered from within other callbacks run in the next iteration
pub fn execute_immediate_callbacks(scope: &mut v8::PinScope) {
    // Drain under the lock, then release it before invoking JS. Callbacks may
    // re-enter setImmediate (chained immediates) or clearImmediate; holding the
    // mutex across callback.call would deadlock on that re-entry.
    let callbacks = IMMEDIATE_CALLBACKS.with(|s| s.borrow_mut().drain_non_deferred());

    for (timer_id, callback, args) in callbacks {
        let callback = v8::Local::<v8::Function>::new(scope, callback);

        // Convert stored args back to Local
        let args: Vec<v8::Local<v8::Value>> = args
            .into_iter()
            .map(|arg| v8::Local::new(scope, arg))
            .collect();

        let undefined = v8::undefined(scope);
        let _ = callback.call(scope, undefined.into(), &args);

        // Remove from metadata
        remove_timer_metadata(timer_id);
    }
}

/// v0.3.250: Check if there are pending setImmediate callbacks
pub fn has_pending_immediates() -> bool {
    IMMEDIATE_CALLBACKS.with(|s| !s.borrow().is_empty())
}

/// Check if any zero-delay timers scheduled by this thread are pending.
///
/// `execute_code` should wait for these to preserve same-tick `setTimeout(..., 0)`
/// semantics, but should not block on longer timers that are meant to fire later.
pub fn has_pending_zero_delay_timers() -> bool {
    let owned = owned_timer_ids();
    let metadata = TIMER_METADATA.lock().unwrap();
    metadata.iter().any(|(id, meta)| {
        owned.contains(id)
            && meta.delay == 0
            && matches!(meta.timer_type, TimerType::Timeout | TimerType::Interval)
    })
}

/// Check if any short, ref'ed one-shot timeout should keep the current eval alive.
pub fn has_pending_drainable_timeout_timers(max_delay_ms: u64) -> bool {
    let owned = owned_timer_ids();
    let metadata = TIMER_METADATA.lock().unwrap();
    metadata.iter().any(|(id, meta)| {
        owned.contains(id)
            && !meta.is_unrefed
            && meta.delay <= max_delay_ms
            && matches!(meta.timer_type, TimerType::Timeout)
    })
}

/// Check if any short, ref'ed timeout or interval should keep the current eval alive.
pub fn has_pending_drainable_timers(max_delay_ms: u64) -> bool {
    let owned = owned_timer_ids();
    let metadata = TIMER_METADATA.lock().unwrap();
    metadata.iter().any(|(id, meta)| {
        owned.contains(id)
            && !meta.is_unrefed
            && meta.delay <= max_delay_ms
            && matches!(meta.timer_type, TimerType::Timeout | TimerType::Interval)
    })
}

/// True when any ref'd timeout/interval is still scheduled (ignores delay cap).
pub fn has_pending_refed_timers() -> bool {
    has_pending_drainable_timers(u64::MAX)
}

/// v0.3.250: Store setImmediate callback for next event loop iteration
/// v0.3.261: Store with deferred=false so they run in the same iteration
/// (after timers). Callbacks registered from within other callbacks will be
/// marked deferred by mark_all_as_deferred() after execution.
pub fn store_immediate_callback(
    timer_id: u64,
    callback: v8::Global<v8::Function>,
    args: Vec<v8::Global<v8::Value>>,
) {
    IMMEDIATE_CALLBACKS.with(|s| {
        // Initially not deferred - will be marked deferred after execution
        s.borrow_mut().push(timer_id, callback, args, false);
    });
}

/// v0.3.261: Mark all callbacks as deferred
/// Called after executing setImmediate callbacks to defer any callbacks
/// registered during execution to the NEXT iteration
pub fn mark_immediate_callbacks_deferred() {
    IMMEDIATE_CALLBACKS.with(|s| s.borrow_mut().mark_all_as_deferred());
}

/// Make deferred callbacks eligible at the start of a new event-loop iteration.
/// Without this, callbacks deferred by mark_immediate_callbacks_deferred would
/// never become runnable again.
pub fn unmark_immediate_callbacks_deferred() {
    IMMEDIATE_CALLBACKS.with(|s| s.borrow_mut().unmark_all_deferred());
}

/// Drop pending setImmediate callbacks left over from a previous execution.
/// Immediate callbacks capture V8 handles from the execution that scheduled
/// them; running them in a later execute_code call is never correct, and a
/// leftover would keep that call's event loop alive on stale state.
pub fn clear_pending_immediates() {
    IMMEDIATE_CALLBACKS.with(|s| s.borrow_mut().clear_all());
}

/// v0.3.250: Remove a pending setImmediate callback (for clearImmediate)
pub fn remove_immediate_callback(timer_id: u64) -> bool {
    IMMEDIATE_CALLBACKS.with(|s| s.borrow_mut().remove(timer_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_creation() {
        let timer_id = get_next_timer_id();
        assert!(timer_id > 0);
    }

    #[test]
    fn test_timer_metadata() {
        let mut metadata = TIMER_METADATA.lock().unwrap();
        metadata.insert(
            1,
            TimerMetadata {
                timer_type: TimerType::Timeout,
                delay: 1000,
                is_unrefed: false,
                epoch: get_timer_epoch(),
            },
        );
        assert!(metadata.contains_key(&1));
    }

    #[test]
    fn test_timer_type_variants() {
        assert_eq!(TimerType::Timeout, TimerType::Timeout);
        assert_eq!(TimerType::Interval, TimerType::Interval);
        assert_eq!(TimerType::Immediate, TimerType::Immediate);
    }
}
