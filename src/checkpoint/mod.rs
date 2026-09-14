// Beejs Agent State Checkpoint & Time-Travel Snapshot Engine (bee:checkpoint)
// Lightweight state snapshotting, structural delta diffing, Tree-of-Thought branching, and bee:kv persistence.

use once_cell::sync::Lazy;
use rusty_v8 as v8;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

static MGR_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static CP_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub branch: String,
    pub timestamp: u64,
    pub state: Value,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct StateDiff {
    pub added: HashMap<String, Value>,
    pub modified: HashMap<String, ValueDiff>,
    pub deleted: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValueDiff {
    pub from: Value,
    pub to: Value,
}

pub struct CheckpointManager {
    pub id: u64,
    pub checkpoints: HashMap<String, Checkpoint>,
    pub order: Vec<String>, // insertion order
    pub current_id: Option<String>,
    pub current_branch: String,
}

impl CheckpointManager {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            checkpoints: HashMap::new(),
            order: Vec::new(),
            current_id: None,
            current_branch: "main".to_string(),
        }
    }

    pub fn save(
        &mut self,
        custom_id: Option<String>,
        state: Value,
        branch: Option<String>,
        metadata: HashMap<String, Value>,
    ) -> Checkpoint {
        let cp_id = custom_id.unwrap_or_else(|| {
            let num = CP_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
            format!("cp_{num:x}")
        });

        let target_branch = branch.unwrap_or_else(|| self.current_branch.clone());
        let parent_id = self.current_id.clone();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let checkpoint = Checkpoint {
            id: cp_id.clone(),
            parent_id,
            branch: target_branch,
            timestamp,
            state,
            metadata,
        };

        self.current_id = Some(cp_id.clone());
        self.checkpoints.insert(cp_id.clone(), checkpoint.clone());
        self.order.push(cp_id);

        checkpoint
    }

    pub fn get(&self, id: &str) -> Option<&Checkpoint> {
        self.checkpoints.get(id)
    }

    pub fn list(&self, branch_filter: Option<&str>) -> Vec<Checkpoint> {
        self.order
            .iter()
            .filter_map(|id| self.checkpoints.get(id))
            .filter(|cp| {
                if let Some(b) = branch_filter {
                    cp.branch == b
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    pub fn restore(&mut self, id: &str) -> Option<Value> {
        if let Some(cp) = self.checkpoints.get(id) {
            self.current_id = Some(id.to_string());
            self.current_branch = cp.branch.clone();
            Some(cp.state.clone())
        } else {
            None
        }
    }

    pub fn diff(&self, from_id: &str, to_id: &str) -> Option<StateDiff> {
        let cp_a = self.checkpoints.get(from_id)?;
        let cp_b = self.checkpoints.get(to_id)?;
        Some(compute_diff(&cp_a.state, &cp_b.state))
    }

    pub fn delete(&mut self, id: &str) -> bool {
        if self.checkpoints.remove(id).is_some() {
            self.order.retain(|k| k != id);
            if self.current_id.as_deref() == Some(id) {
                self.current_id = self.order.last().cloned();
            }
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.checkpoints.clear();
        self.order.clear();
        self.current_id = None;
        self.current_branch = "main".to_string();
    }
}

pub fn compute_diff(a: &Value, b: &Value) -> StateDiff {
    let mut diff = StateDiff::default();

    match (a, b) {
        (Value::Object(map_a), Value::Object(map_b)) => {
            // Check additions and modifications
            for (k, val_b) in map_b {
                if let Some(val_a) = map_a.get(k) {
                    if val_a != val_b {
                        diff.modified.insert(
                            k.clone(),
                            ValueDiff {
                                from: val_a.clone(),
                                to: val_b.clone(),
                            },
                        );
                    }
                } else {
                    diff.added.insert(k.clone(), val_b.clone());
                }
            }
            // Check deletions
            for k in map_a.keys() {
                if !map_b.contains_key(k) {
                    diff.deleted.push(k.clone());
                }
            }
        }
        _ => {
            if a != b {
                diff.modified.insert(
                    "root".to_string(),
                    ValueDiff {
                        from: a.clone(),
                        to: b.clone(),
                    },
                );
            }
        }
    }

    diff
}

static MANAGERS: Lazy<RwLock<HashMap<u64, CheckpointManager>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

fn checkpoint_native_dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() < 1 {
        return;
    }

    let action = args.get(0).to_rust_string_lossy(scope);

    match action.as_str() {
        "create_manager" => {
            let id = MGR_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
            let mgr = CheckpointManager::new(id);
            let mut map = MANAGERS.write().unwrap();
            map.insert(id, mgr);
            rv.set(v8::Integer::new(scope, id as i32).into());
        }
        "save" => {
            let mgr_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let custom_id = args.get(2).to_rust_string_lossy(scope);
            let opt_id = if custom_id.is_empty() {
                None
            } else {
                Some(custom_id)
            };
            let state_raw = args.get(3).to_rust_string_lossy(scope);
            let branch = args.get(4).to_rust_string_lossy(scope);
            let opt_branch = if branch.is_empty() {
                None
            } else {
                Some(branch)
            };
            let meta_raw = args.get(5).to_rust_string_lossy(scope);

            let state: Value = serde_json::from_str(&state_raw).unwrap_or(Value::Null);
            let meta: HashMap<String, Value> = serde_json::from_str(&meta_raw).unwrap_or_default();

            let mut map = MANAGERS.write().unwrap();
            if let Some(mgr) = map.get_mut(&mgr_id) {
                let cp = mgr.save(opt_id, state, opt_branch, meta);
                let json = serde_json::to_string(&cp).unwrap_or_else(|_| "{}".to_string());
                let s = v8::String::new(scope, &json).unwrap();
                rv.set(s.into());
            } else {
                let msg = v8::String::new(scope, "Manager not found").unwrap();
                let exc = v8::Exception::error(scope, msg);
                scope.throw_exception(exc);
            }
        }
        "get" => {
            let mgr_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let cp_id = args.get(2).to_rust_string_lossy(scope);
            let map = MANAGERS.read().unwrap();
            if let Some(mgr) = map.get(&mgr_id) {
                if let Some(cp) = mgr.get(&cp_id) {
                    let json = serde_json::to_string(cp).unwrap();
                    let s = v8::String::new(scope, &json).unwrap();
                    rv.set(s.into());
                } else {
                    rv.set(v8::null(scope).into());
                }
            }
        }
        "restore" => {
            let mgr_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let cp_id = args.get(2).to_rust_string_lossy(scope);
            let mut map = MANAGERS.write().unwrap();
            if let Some(mgr) = map.get_mut(&mgr_id) {
                if let Some(state) = mgr.restore(&cp_id) {
                    let json = serde_json::to_string(&state).unwrap();
                    let s = v8::String::new(scope, &json).unwrap();
                    rv.set(s.into());
                } else {
                    let msg =
                        v8::String::new(scope, &format!("Checkpoint '{cp_id}' not found")).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        }
        "list" => {
            let mgr_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let branch = args.get(2).to_rust_string_lossy(scope);
            let opt_branch = if branch.is_empty() {
                None
            } else {
                Some(branch.as_str())
            };
            let map = MANAGERS.read().unwrap();
            if let Some(mgr) = map.get(&mgr_id) {
                let list = mgr.list(opt_branch);
                let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
                let s = v8::String::new(scope, &json).unwrap();
                rv.set(s.into());
            }
        }
        "diff" => {
            let mgr_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let from_id = args.get(2).to_rust_string_lossy(scope);
            let to_id = args.get(3).to_rust_string_lossy(scope);
            let map = MANAGERS.read().unwrap();
            if let Some(mgr) = map.get(&mgr_id) {
                if let Some(d) = mgr.diff(&from_id, &to_id) {
                    let json = serde_json::to_string(&d).unwrap();
                    let s = v8::String::new(scope, &json).unwrap();
                    rv.set(s.into());
                } else {
                    let msg = v8::String::new(scope, "Checkpoints for diff not found").unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        }
        "delete" => {
            let mgr_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let cp_id = args.get(2).to_rust_string_lossy(scope);
            let mut map = MANAGERS.write().unwrap();
            let res = map
                .get_mut(&mgr_id)
                .map(|m| m.delete(&cp_id))
                .unwrap_or(false);
            rv.set(v8::Boolean::new(scope, res).into());
        }
        "clear" => {
            let mgr_id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(1);
            let mut map = MANAGERS.write().unwrap();
            if let Some(m) = map.get_mut(&mgr_id) {
                m.clear();
            }
            rv.set(v8::Boolean::new(scope, true).into());
        }
        _ => {
            let msg =
                v8::String::new(scope, &format!("Unknown checkpoint action '{action}'")).unwrap();
            let exc = v8::Exception::type_error(scope, msg);
            scope.throw_exception(exc);
        }
    }
}

/// Sets up the `bee:checkpoint` API in V8 context
pub fn setup_checkpoint_api(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    let global = context.global(scope);

    // Register native dispatcher callback
    let native_fn = v8::Function::new(scope, checkpoint_native_dispatch).unwrap();
    let k_native = v8::String::new(scope, "__bee_checkpoint_native").unwrap();
    global.set(scope, k_native.into(), native_fn.into());

    let checkpoint_js_bootstrap = r#"
    (function() {
        const native = globalThis.__bee_checkpoint_native;

        class CheckpointManager {
            #id;
            #branch;

            constructor(id = null, branch = 'main') {
                this.#id = typeof id === 'number' ? id : native('create_manager');
                this.#branch = String(branch || 'main');
            }

            get id() {
                return this.#id;
            }

            get currentBranch() {
                return this.#branch;
            }

            save(idOrOptions, stateValue = undefined, metadata = {}) {
                let id = '';
                let state = stateValue;
                let branch = this.#branch;
                let meta = metadata || {};

                if (typeof idOrOptions === 'object' && idOrOptions !== null && stateValue === undefined) {
                    // Passed options object: save({ id, state, branch, metadata })
                    id = idOrOptions.id ? String(idOrOptions.id) : '';
                    state = idOrOptions.state !== undefined ? idOrOptions.state : idOrOptions;
                    branch = idOrOptions.branch ? String(idOrOptions.branch) : this.#branch;
                    meta = idOrOptions.metadata || {};
                } else if (typeof idOrOptions === 'string') {
                    id = idOrOptions;
                } else if (stateValue === undefined) {
                    // Passed raw state directly: save(state)
                    state = idOrOptions;
                }

                const raw = native(
                    'save',
                    this.#id,
                    id,
                    JSON.stringify(state !== undefined ? state : null),
                    branch,
                    JSON.stringify(meta)
                );
                return JSON.parse(raw);
            }

            get(id) {
                if (!id) return undefined;
                const raw = native('get', this.#id, String(id));
                if (!raw) return undefined;
                return JSON.parse(raw);
            }

            restore(id) {
                if (!id) throw new TypeError('restore requires a checkpoint id');
                const raw = native('restore', this.#id, String(id));
                return JSON.parse(raw);
            }

            list(options = {}) {
                const branch = options.branch ? String(options.branch) : '';
                const raw = native('list', this.#id, branch);
                return JSON.parse(raw);
            }

            diff(fromId, toId) {
                if (!fromId || !toId) {
                    throw new TypeError('diff requires fromId and toId');
                }
                const raw = native('diff', this.#id, String(fromId), String(toId));
                return JSON.parse(raw);
            }

            fork(fromId, branchName) {
                if (!fromId || !branchName) {
                    throw new TypeError('fork requires fromId and branchName');
                }
                const cp = this.get(fromId);
                if (!cp) {
                    throw new Error(`Cannot fork from non-existent checkpoint '${fromId}'`);
                }

                // Create a new branching CheckpointManager sharing current checkpoints
                const forked = new CheckpointManager(null, branchName);
                // Preload history up to fromId
                const all = this.list();
                for (const item of all) {
                    forked.save({
                        id: item.id,
                        state: item.state,
                        branch: item.branch,
                        metadata: item.metadata
                    });
                    if (item.id === fromId) break;
                }
                return forked;
            }

            delete(id) {
                if (!id) return false;
                return native('delete', this.#id, String(id));
            }

            clear() {
                return native('clear', this.#id);
            }

            // Persist all checkpoints to a bee:kv store instance
            persist(kvStore, prefix = 'cp:') {
                if (!kvStore || typeof kvStore.set !== 'function') {
                    throw new TypeError('persist requires a valid bee:kv store instance');
                }
                const list = this.list();
                for (const cp of list) {
                    kvStore.set(`${prefix}${cp.id}`, cp);
                }
                kvStore.set(`${prefix}__order__`, list.map(c => c.id));
                return list.length;
            }

            // Restore checkpoints from a bee:kv store instance
            restoreFromKV(kvStore, prefix = 'cp:') {
                if (!kvStore || typeof kvStore.get !== 'function') {
                    throw new TypeError('restoreFromKV requires a valid bee:kv store instance');
                }
                const order = kvStore.get(`${prefix}__order__`);
                if (!Array.isArray(order)) return 0;
                let count = 0;
                for (const id of order) {
                    const cp = kvStore.get(`${prefix}${id}`);
                    if (cp) {
                        this.save({
                            id: cp.id,
                            state: cp.state,
                            branch: cp.branch,
                            metadata: cp.metadata
                        });
                        count++;
                    }
                }
                return count;
            }
        }

        const defaultManager = new CheckpointManager();

        const checkpointModule = {
            CheckpointManager,
            createCheckpointManager: (id, branch) => new CheckpointManager(id, branch),
            getDefaultManager: () => defaultManager,
            save: (...args) => defaultManager.save(...args),
            restore: (...args) => defaultManager.restore(...args),
            get: (...args) => defaultManager.get(...args),
            list: (...args) => defaultManager.list(...args),
            diff: (...args) => defaultManager.diff(...args),
            fork: (...args) => defaultManager.fork(...args),
            clear: () => defaultManager.clear(),
            default: defaultManager
        };

        globalThis.__bee_checkpoint = checkpointModule;
        globalThis.checkpoint = checkpointModule;
    })();
    "#;

    if let Some(code) = v8::String::new(scope, checkpoint_js_bootstrap) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
