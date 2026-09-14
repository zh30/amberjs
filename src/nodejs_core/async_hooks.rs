//! Node.js `async_hooks` module implementation.
use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_async_hooks_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let script_code = r#"
    (function() {
        let _asyncIdSeq = 1;

        class AsyncResource {
            constructor(type, triggerAsyncId) {
                this._type = type || 'UNKNOWN';
                this._asyncId = ++_asyncIdSeq;
                this._triggerAsyncId = triggerAsyncId || 0;
                this._destroyed = false;
            }
            asyncId() {
                return this._asyncId;
            }
            triggerAsyncId() {
                return this._triggerAsyncId;
            }
            runInAsyncScope(fn, thisArg, ...args) {
                return fn.apply(thisArg, args);
            }
            emitDestroy() {
                this._destroyed = true;
                return this;
            }
            bind(fn, type, thisArg) {
                const self = this;
                return function(...args) {
                    return self.runInAsyncScope(fn, thisArg || this, ...args);
                };
            }
            static bind(fn, type, thisArg) {
                const resource = new AsyncResource(type || (fn && fn.name) || 'bound');
                return resource.bind(fn, type, thisArg);
            }
        }

        const _activeAlsInstances = new Set();
        class AsyncLocalStorage {
            constructor() {
                this._store = undefined;
                this._enabled = true;
                _activeAlsInstances.add(this);
            }
            disable() {
                this._store = undefined;
                this._enabled = false;
                _activeAlsInstances.delete(this);
            }
            getStore() {
                return this._enabled ? this._store : undefined;
            }
            run(store, callback, ...args) {
                if (!this._enabled) {
                    return callback(...args);
                }
                const prev = this._store;
                this._store = store;
                try {
                    const res = callback(...args);
                    if (res && typeof res.then === 'function') {
                        return Promise.resolve(res).finally(() => {
                            this._store = prev;
                        });
                    }
                    this._store = prev;
                    return res;
                } catch (err) {
                    this._store = prev;
                    throw err;
                }
            }
            exit(callback, ...args) {
                if (!this._enabled) {
                    return callback(...args);
                }
                const prev = this._store;
                this._store = undefined;
                try {
                    const res = callback(...args);
                    if (res && typeof res.then === 'function') {
                        return Promise.resolve(res).finally(() => {
                            this._store = prev;
                        });
                    }
                    this._store = prev;
                    return res;
                } catch (err) {
                    this._store = prev;
                    throw err;
                }
            }
            enterWith(store) {
                if (this._enabled) {
                    this._store = store;
                }
            }
            static snapshot() {
                const captured = [];
                for (const als of _activeAlsInstances) {
                    if (als._enabled) {
                        captured.push({ als, store: als._store });
                    }
                }
                return function runInSnapshot(cb, ...args) {
                    const prevStores = [];
                    for (const { als, store } of captured) {
                        prevStores.push({ als, prev: als._store });
                        als._store = store;
                    }
                    try {
                        const res = cb(...args);
                        if (res && typeof res.then === 'function') {
                            return Promise.resolve(res).finally(() => {
                                for (const { als, prev } of prevStores) {
                                    als._store = prev;
                                }
                            });
                        }
                        for (const { als, prev } of prevStores) {
                            als._store = prev;
                        }
                        return res;
                    } catch (err) {
                        for (const { als, prev } of prevStores) {
                            als._store = prev;
                        }
                        throw err;
                    }
                };
            }
        }

        function createHook(callbacks) {
            return {
                enable() { return this; },
                disable() { return this; }
            };
        }

        function executionAsyncId() {
            return 1;
        }

        function triggerAsyncId() {
            return 0;
        }

        function executionAsyncResource() {
            return {};
        }

        const ah = {
            AsyncResource,
            AsyncLocalStorage,
            createHook,
            executionAsyncId,
            triggerAsyncId,
            executionAsyncResource,
        };
        ah.default = ah;
        return ah;
    })();
    "#;

    let source = v8::String::new(scope, script_code)
        .ok_or_else(|| anyhow::anyhow!("Failed to create async_hooks bootstrap source"))?;
    let script = v8::Script::compile(scope, source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to compile async_hooks bootstrap"))?;
    let ah_obj = script
        .run(scope)
        .ok_or_else(|| anyhow::anyhow!("Failed to run async_hooks bootstrap"))?;

    let global = context.global(scope);
    let key = v8::String::new(scope, "async_hooks").unwrap();
    global.set(scope, key.into(), ah_obj);

    Ok(())
}
