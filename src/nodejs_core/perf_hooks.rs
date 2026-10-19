// Node.js perf_hooks 模块实现

use anyhow::Result;
use rusty_v8 as v8;

/// Shared JS snippet installing the real `PerformanceObserver` machinery and
/// `performance.timerify` on top of the Rust-side entry store:
///
/// - `globalThis.__perf_observers` — live observer registry. Rust code in
///   `performance.rs::notify_perf_observers` pushes `{name, entryType,
///   startTime, duration}` objects into each observing observer's `_records`
///   and schedules `__perf_observer_flush` via `queueMicrotask`.
/// - `globalThis.__perf_observer_flush` — drains scheduled observers and calls
///   `callback(list, observer)` with a Node-shaped entry-list object.
/// - `globalThis.__PerfObserver` — the `PerformanceObserver` class. It is kept
///   module-scoped (Node exposes it through `require('perf_hooks')`, not as a
///   global), so the bag builders re-export it from here.
/// - `performance.timerify(fn)` — wraps `fn`; every call records a
///   `'function'` entry through the `__perf_now` / `__perf_record_fn` natives.
pub const PERF_OBSERVER_JS: &str = r#"
(function() {
    globalThis.__perf_observers = globalThis.__perf_observers || [];
    const __perfListProto = {
        getEntries() { return this._e.slice(); },
        getEntriesByType(t) { return this._e.filter((x) => x.entryType === t); },
        getEntriesByName(n) { return this._e.filter((x) => x.name === n); },
    };
    globalThis.__perf_list_proto = __perfListProto;
    globalThis.__perf_observer_flush = function __perf_observer_flush() {
        for (const obs of globalThis.__perf_observers.slice()) {
            if (!obs._scheduled) continue;
            obs._scheduled = false;
            const records = obs._records.splice(0, obs._records.length);
            if (records.length === 0 || typeof obs._cb !== 'function') continue;
            const list = Object.create(__perfListProto);
            list._e = records;
            obs._cb.call(obs, list, obs);
        }
    };
    const VALID_ENTRY_TYPES = ['mark', 'measure', 'function'];
    class __PerfObserver {
        constructor(callback) {
            if (typeof callback !== 'function') {
                throw new TypeError('PerformanceObserver requires a callback function');
            }
            this._cb = callback;
            this._records = [];
            this._types = null;
            this._scheduled = false;
        }
        observe(options) {
            const types = options && options.entryTypes;
            if (!Array.isArray(types) || types.length === 0) {
                throw new TypeError('PerformanceObserver.observe requires a non-empty entryTypes array');
            }
            for (const t of types) {
                if (!VALID_ENTRY_TYPES.includes(t)) {
                    throw new TypeError('Unknown performance entry type: ' + String(t));
                }
            }
            this._types = types.slice();
            if (!globalThis.__perf_observers.includes(this)) {
                globalThis.__perf_observers.push(this);
            }
        }
        takeRecords() { return this._records.splice(0, this._records.length); }
        disconnect() {
            const i = globalThis.__perf_observers.indexOf(this);
            if (i >= 0) globalThis.__perf_observers.splice(i, 1);
            this._types = null;
            this._records = [];
            this._scheduled = false;
        }
    }
    globalThis.__PerfObserver = __PerfObserver;
    const perf = globalThis.performance;
    // `timerify` needs the Rust natives; where they are absent (paths that
    // never ran `setup_performance_api`), the method stays absent instead of
    // shipping a wrapper that would die on first call — fail-closed surface.
    if (perf && typeof perf.timerify !== 'function'
        && typeof globalThis.__perf_now === 'function'
        && typeof globalThis.__perf_record_fn === 'function') {
        perf.timerify = function timerify(fn) {
            if (typeof fn !== 'function') {
                throw new TypeError('performance.timerify requires a function');
            }
            const wrapped = function() {
                const start = globalThis.__perf_now();
                try { return fn.apply(this, arguments); }
                finally { globalThis.__perf_record_fn(fn.name || 'anonymous', start); }
            };
            try {
                Object.defineProperty(wrapped, 'name', {
                    value: 'timerified ' + (fn.name || 'anonymous'),
                    configurable: true,
                });
            } catch (_) {}
            return wrapped;
        };
    }
})();
"#;

pub fn setup_perf_hooks_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Install the shared observer/timerify snippet, then expose the
    // require('perf_hooks') bag. `performance` prefers the real object
    // (`performance.rs`); the fallback shim only covers environments that
    // never ran `setup_performance_api`.
    let observer_src = v8::String::new(scope, PERF_OBSERVER_JS).unwrap();
    if let Some(script) = v8::Script::compile(scope, observer_src, None) {
        let _ = script.run(scope);
    }

    let js_code = r#"
    (function() {
        const perf = globalThis.performance || {
            now: () => Date.now(),
            timeOrigin: Date.now(),
            mark: () => {},
            measure: () => {},
            getEntries: () => [],
            getEntriesByName: () => [],
            getEntriesByType: () => [],
            clearMarks: () => {},
            clearMeasures: () => {}
        };

        const perfHooks = {
            performance: perf,
            PerformanceObserver: globalThis.__PerfObserver,
            default: {
                performance: perf,
                PerformanceObserver: globalThis.__PerfObserver
            }
        };

        globalThis.__perf_hooks = perfHooks;
        globalThis.perf_hooks = perfHooks;
    })();
    "#;

    let script_source = v8::String::new(scope, js_code).unwrap();
    if let Some(script) = v8::Script::compile(scope, script_source, None) {
        let _ = script.run(scope);
    }

    let perf_hooks_key = v8::String::new(scope, "__perf_hooks").unwrap();
    if let Some(ph_val) = global.get(scope, perf_hooks_key.into()) {
        let ph_key = v8::String::new(scope, "perf_hooks").unwrap();
        global.set(scope, ph_key.into(), ph_val);
    }

    Ok(())
}
