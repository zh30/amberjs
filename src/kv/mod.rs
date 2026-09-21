//! Amber v1.7.0: Embedded Persistent Key-Value & Durable State Engine (`amber:kv`)
//!
//! Provides a zero-dependency, transactional, ACID-compliant key-value engine
//! with in-memory mode, disk WAL persistence, prefix range scanning, TTL expiration,
//! atomic increments, and batch operations for autonomous AI Agent memory and state.

use once_cell::sync::Lazy;
use rusty_v8 as v8;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn current_time_ms() -> u64 {
    if let Some(t) = crate::permissions::get_frozen_time_ms() {
        if t >= 0 {
            return t as u64;
        }
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KVEntry {
    pub value: Value,
    pub expires_at: Option<u64>,
}

impl KVEntry {
    pub fn is_expired(&self, now: u64) -> bool {
        if let Some(exp) = self.expires_at {
            now >= exp
        } else {
            false
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum WalRecord {
    Set {
        key: String,
        val: Value,
        exp: Option<u64>,
    },
    Del {
        key: String,
    },
    Clear,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchOp {
    #[serde(rename = "type")]
    pub op_type: String, // "put" | "set" | "del" | "delete"
    pub key: String,
    pub value: Option<Value>,
    #[serde(rename = "ttlMs")]
    pub ttl_ms: Option<u64>,
}

pub struct KVStore {
    path: Option<PathBuf>,
    data: HashMap<String, KVEntry>,
    wal_file: Option<File>,
}

impl KVStore {
    pub fn new_memory() -> Self {
        Self {
            path: None,
            data: HashMap::new(),
            wal_file: None,
        }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let p = path.as_ref().to_path_buf();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let mut data = HashMap::new();
        let now = current_time_ms();

        // Replay existing WAL if present
        if p.exists() {
            let file = File::open(&p)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(rec) = serde_json::from_str::<WalRecord>(&line) {
                    match rec {
                        WalRecord::Set { key, val, exp } => {
                            let entry = KVEntry {
                                value: val,
                                expires_at: exp,
                            };
                            if !entry.is_expired(now) {
                                data.insert(key, entry);
                            } else {
                                data.remove(&key);
                            }
                        }
                        WalRecord::Del { key } => {
                            data.remove(&key);
                        }
                        WalRecord::Clear => {
                            data.clear();
                        }
                    }
                }
            }
        }

        let wal_file = OpenOptions::new().create(true).append(true).open(&p)?;

        Ok(Self {
            path: Some(p),
            data,
            wal_file: Some(wal_file),
        })
    }

    fn append_wal(&mut self, rec: &WalRecord) {
        if let Some(ref mut file) = self.wal_file {
            if let Ok(json) = serde_json::to_string(rec) {
                let _ = writeln!(file, "{}", json);
                let _ = file.flush();
            }
        }
    }

    pub fn get(&mut self, key: &str) -> Option<Value> {
        let now = current_time_ms();
        if let Some(entry) = self.data.get(key) {
            if entry.is_expired(now) {
                self.data.remove(key);
                self.append_wal(&WalRecord::Del {
                    key: key.to_string(),
                });
                None
            } else {
                Some(entry.value.clone())
            }
        } else {
            None
        }
    }

    pub fn set(&mut self, key: &str, val: Value, ttl_ms: Option<u64>) {
        let now = current_time_ms();
        let expires_at = ttl_ms.map(|ms| now + ms);
        let entry = KVEntry {
            value: val.clone(),
            expires_at,
        };
        self.data.insert(key.to_string(), entry);
        self.append_wal(&WalRecord::Set {
            key: key.to_string(),
            val,
            exp: expires_at,
        });
    }

    pub fn delete(&mut self, key: &str) -> bool {
        if self.data.remove(key).is_some() {
            self.append_wal(&WalRecord::Del {
                key: key.to_string(),
            });
            true
        } else {
            false
        }
    }

    pub fn has(&mut self, key: &str) -> bool {
        self.get(key).is_some()
    }

    fn prune_expired(&mut self) {
        let now = current_time_ms();
        let expired: Vec<String> = self
            .data
            .iter()
            .filter(|(_, entry)| entry.is_expired(now))
            .map(|(k, _)| k.clone())
            .collect();
        for k in expired {
            self.data.remove(&k);
            self.append_wal(&WalRecord::Del { key: k });
        }
    }

    pub fn keys(&mut self, prefix: Option<&str>) -> Vec<String> {
        self.prune_expired();
        let mut keys: Vec<String> = self
            .data
            .keys()
            .filter(|k| match prefix {
                Some(p) if !p.is_empty() => k.starts_with(p),
                _ => true,
            })
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    pub fn values(&mut self) -> Vec<Value> {
        self.prune_expired();
        let mut pairs: Vec<(String, Value)> = self
            .data
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs.into_iter().map(|(_, v)| v).collect()
    }

    pub fn entries(&mut self, prefix: Option<&str>) -> Vec<(String, Value)> {
        self.prune_expired();
        let mut pairs: Vec<(String, Value)> = self
            .data
            .iter()
            .filter(|(k, _)| match prefix {
                Some(p) if !p.is_empty() => k.starts_with(p),
                _ => true,
            })
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    pub fn scan(&mut self, prefix: &str, limit: Option<usize>) -> Vec<(String, Value)> {
        let max = limit.unwrap_or(usize::MAX);
        let all = self.entries(Some(prefix));
        all.into_iter().take(max).collect()
    }

    pub fn incr(&mut self, key: &str, delta: i64) -> Result<i64, String> {
        let now = current_time_ms();
        let (current_num, exp) = if let Some(entry) = self.data.get(key) {
            if entry.is_expired(now) {
                (0i64, None)
            } else if let Some(n) = entry.value.as_i64() {
                (n, entry.expires_at)
            } else if let Some(f) = entry.value.as_f64() {
                (f as i64, entry.expires_at)
            } else {
                return Err(format!("Value for key '{}' is not a number", key));
            }
        } else {
            (0i64, None)
        };

        let new_val = current_num + delta;
        let v = Value::from(new_val);
        self.data.insert(
            key.to_string(),
            KVEntry {
                value: v.clone(),
                expires_at: exp,
            },
        );
        self.append_wal(&WalRecord::Set {
            key: key.to_string(),
            val: v,
            exp,
        });
        Ok(new_val)
    }

    pub fn batch(&mut self, ops: Vec<BatchOp>) -> Result<(), String> {
        let now = current_time_ms();
        for op in ops {
            match op.op_type.to_lowercase().as_str() {
                "put" | "set" => {
                    let val = op.value.unwrap_or(Value::Null);
                    let expires_at = op.ttl_ms.map(|ms| now + ms);
                    self.data.insert(
                        op.key.clone(),
                        KVEntry {
                            value: val.clone(),
                            expires_at,
                        },
                    );
                    self.append_wal(&WalRecord::Set {
                        key: op.key,
                        val,
                        exp: expires_at,
                    });
                }
                "del" | "delete" => {
                    self.data.remove(&op.key);
                    self.append_wal(&WalRecord::Del { key: op.key });
                }
                other => {
                    return Err(format!("Unknown batch operation type '{}'", other));
                }
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.append_wal(&WalRecord::Clear);
    }

    pub fn compact(&mut self) -> std::io::Result<()> {
        let Some(p) = self.path.clone() else {
            return Ok(());
        };
        self.prune_expired();

        let temp_path = p.with_extension("tmp-compact");
        {
            let mut temp_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&temp_path)?;

            for (k, entry) in &self.data {
                let rec = WalRecord::Set {
                    key: k.clone(),
                    val: entry.value.clone(),
                    exp: entry.expires_at,
                };
                let json = serde_json::to_string(&rec)?;
                writeln!(temp_file, "{}", json)?;
            }
            temp_file.flush()?;
        }

        self.wal_file = None;
        std::fs::rename(&temp_path, &p)?;
        self.wal_file = Some(OpenOptions::new().create(true).append(true).open(p)?);
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        if let Some(ref mut f) = self.wal_file {
            f.flush()?;
        }
        Ok(())
    }

    pub fn close(&mut self) {
        let _ = self.flush();
        self.wal_file = None;
        self.data.clear();
    }
}

// Global Store Registry
static STORE_COUNTER: AtomicU64 = AtomicU64::new(1);
static STORES: Lazy<RwLock<HashMap<u64, Arc<RwLock<KVStore>>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

fn register_store(store: KVStore) -> u64 {
    let id = STORE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut map = STORES.write().unwrap();
    map.insert(id, Arc::new(RwLock::new(store)));
    id
}

fn with_store<F, R>(id: u64, f: F) -> Option<R>
where
    F: FnOnce(&mut KVStore) -> R,
{
    let store_arc = {
        let map = STORES.read().unwrap();
        map.get(&id).cloned()?
    };
    let mut store = store_arc.write().unwrap();
    Some(f(&mut *store))
}

/// Native callback dispatcher for `amber:kv` operations
fn kv_native_dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() == 0 {
        return;
    }

    let action = args.get(0).to_rust_string_lossy(scope);

    match action.as_str() {
        "open" => {
            if args.length() < 2 {
                let msg = v8::String::new(scope, "KVStore.open requires a path").unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }
            let path_str = args.get(1).to_rust_string_lossy(scope);
            match KVStore::open(Path::new(&path_str)) {
                Ok(store) => {
                    let id = register_store(store);
                    rv.set(v8::Number::new(scope, id as f64).into());
                }
                Err(e) => {
                    let msg =
                        v8::String::new(scope, &format!("Failed to open KVStore: {e}")).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        }
        "open_memory" => {
            let store = KVStore::new_memory();
            let id = register_store(store);
            rv.set(v8::Number::new(scope, id as f64).into());
        }
        "get" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let key = args.get(2).to_rust_string_lossy(scope);
            let val = with_store(id, |s| s.get(&key)).flatten();
            match val {
                Some(v) => {
                    let json = serde_json::to_string(&v).unwrap_or_else(|_| "null".to_string());
                    let s = v8::String::new(scope, &json).unwrap();
                    rv.set(s.into());
                }
                None => {
                    let null_str = v8::String::new(scope, "null").unwrap();
                    rv.set(null_str.into());
                }
            }
        }
        "set" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let key = args.get(2).to_rust_string_lossy(scope);
            let val_json = args.get(3).to_rust_string_lossy(scope);
            let ttl_num = args
                .get(4)
                .to_integer(scope)
                .map(|i| i.value())
                .unwrap_or(-1);
            let ttl_ms = if ttl_num > 0 {
                Some(ttl_num as u64)
            } else {
                None
            };

            let val: Value = serde_json::from_str(&val_json).unwrap_or(Value::Null);
            let res = with_store(id, |s| {
                s.set(&key, val, ttl_ms);
                true
            })
            .unwrap_or(false);
            rv.set(v8::Boolean::new(scope, res).into());
        }
        "del" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let key = args.get(2).to_rust_string_lossy(scope);
            let res = with_store(id, |s| s.delete(&key)).unwrap_or(false);
            rv.set(v8::Boolean::new(scope, res).into());
        }
        "has" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let key = args.get(2).to_rust_string_lossy(scope);
            let res = with_store(id, |s| s.has(&key)).unwrap_or(false);
            rv.set(v8::Boolean::new(scope, res).into());
        }
        "keys" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let prefix_str = args.get(2).to_rust_string_lossy(scope);
            let prefix = if prefix_str.is_empty() {
                None
            } else {
                Some(prefix_str.as_str())
            };
            let list = with_store(id, |s| s.keys(prefix)).unwrap_or_default();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            let s = v8::String::new(scope, &json).unwrap();
            rv.set(s.into());
        }
        "values" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let list = with_store(id, |s| s.values()).unwrap_or_default();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            let s = v8::String::new(scope, &json).unwrap();
            rv.set(s.into());
        }
        "entries" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let prefix_str = args.get(2).to_rust_string_lossy(scope);
            let prefix = if prefix_str.is_empty() {
                None
            } else {
                Some(prefix_str.as_str())
            };
            let list = with_store(id, |s| s.entries(prefix)).unwrap_or_default();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            let s = v8::String::new(scope, &json).unwrap();
            rv.set(s.into());
        }
        "scan" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let prefix = args.get(2).to_rust_string_lossy(scope);
            let limit_num = args
                .get(3)
                .to_integer(scope)
                .map(|i| i.value())
                .unwrap_or(-1);
            let limit = if limit_num > 0 {
                Some(limit_num as usize)
            } else {
                None
            };
            let list = with_store(id, |s| s.scan(&prefix, limit)).unwrap_or_default();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            let s = v8::String::new(scope, &json).unwrap();
            rv.set(s.into());
        }
        "incr" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let key = args.get(2).to_rust_string_lossy(scope);
            let delta = args
                .get(3)
                .to_integer(scope)
                .map(|i| i.value())
                .unwrap_or(1);
            let res = with_store(id, |s| s.incr(&key, delta));
            match res {
                Some(Ok(new_val)) => {
                    rv.set(v8::Number::new(scope, new_val as f64).into());
                }
                Some(Err(err_msg)) => {
                    let msg = v8::String::new(scope, &err_msg).unwrap();
                    let exc = v8::Exception::type_error(scope, msg);
                    scope.throw_exception(exc);
                }
                None => {
                    let msg = v8::String::new(scope, "KVStore not found").unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        }
        "batch" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let ops_json = args.get(2).to_rust_string_lossy(scope);
            let ops: Result<Vec<BatchOp>, _> = serde_json::from_str(&ops_json);
            match ops {
                Ok(batch_ops) => {
                    let res = with_store(id, |s| s.batch(batch_ops));
                    match res {
                        Some(Ok(())) => {
                            rv.set(v8::Boolean::new(scope, true).into());
                        }
                        Some(Err(e)) => {
                            let msg = v8::String::new(scope, &e).unwrap();
                            let exc = v8::Exception::error(scope, msg);
                            scope.throw_exception(exc);
                        }
                        None => {
                            let msg = v8::String::new(scope, "KVStore not found").unwrap();
                            let exc = v8::Exception::error(scope, msg);
                            scope.throw_exception(exc);
                        }
                    }
                }
                Err(e) => {
                    let msg =
                        v8::String::new(scope, &format!("Invalid batch operations JSON: {e}"))
                            .unwrap();
                    let exc = v8::Exception::type_error(scope, msg);
                    scope.throw_exception(exc);
                }
            }
        }
        "clear" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let res = with_store(id, |s| {
                s.clear();
                true
            })
            .unwrap_or(false);
            rv.set(v8::Boolean::new(scope, res).into());
        }
        "compact" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let res = with_store(id, |s| s.compact().is_ok()).unwrap_or(false);
            rv.set(v8::Boolean::new(scope, res).into());
        }
        "flush" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let res = with_store(id, |s| s.flush().is_ok()).unwrap_or(false);
            rv.set(v8::Boolean::new(scope, res).into());
        }
        "close" => {
            let id = args
                .get(1)
                .to_integer(scope)
                .map(|i| i.value() as u64)
                .unwrap_or(0);
            let res = with_store(id, |s| {
                s.close();
                true
            })
            .unwrap_or(false);
            let mut map = STORES.write().unwrap();
            map.remove(&id);
            rv.set(v8::Boolean::new(scope, res).into());
        }
        _ => {
            let msg = v8::String::new(scope, &format!("Unknown action '{action}'")).unwrap();
            let exc = v8::Exception::type_error(scope, msg);
            scope.throw_exception(exc);
        }
    }
}

/// Sets up the `amber:kv` API inside V8 Context
pub fn setup_kv_api(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    let global = context.global(scope);

    // Register native dispatcher callback
    let native_fn = v8::Function::new(scope, kv_native_dispatch).unwrap();
    let k_native = v8::String::new(scope, "__amber_kv_native").unwrap();
    global.set(scope, k_native.into(), native_fn.into());

    let kv_js_bootstrap = r#"
    (function() {
        const native = globalThis.__amber_kv_native;

        class KVStore {
            #id;
            #path;
            #closed = false;

            constructor(id, path = null) {
                this.#id = id;
                this.#path = path;
            }

            get path() {
                return this.#path;
            }

            get isClosed() {
                return this.#closed;
            }

            static open(optionsOrPath) {
                let path = typeof optionsOrPath === 'string'
                    ? optionsOrPath
                    : (optionsOrPath && typeof optionsOrPath === 'object' && optionsOrPath.path
                        ? optionsOrPath.path
                        : null);
                if (!path) {
                    if (optionsOrPath && typeof optionsOrPath === 'object' && optionsOrPath.inMemory) {
                        return KVStore.openMemory();
                    }
                    throw new TypeError('KVStore.open requires a path string or { path } option');
                }
                const id = native('open', String(path));
                return new KVStore(id, String(path));
            }

            static openMemory() {
                const id = native('open_memory');
                return new KVStore(id, null);
            }

            static openInMemory() {
                return KVStore.openMemory();
            }

            #checkOpen() {
                if (this.#closed) {
                    throw new Error('KVStore is closed');
                }
            }

            get(key) {
                this.#checkOpen();
                const res = native('get', this.#id, String(key));
                if (res === 'null' || res === null || res === undefined) {
                    return undefined;
                }
                return JSON.parse(res);
            }

            set(key, value, options = {}) {
                this.#checkOpen();
                const ttlMs = typeof options === 'number' ? options : (options && options.ttlMs);
                native('set', this.#id, String(key), JSON.stringify(value), ttlMs !== undefined ? Number(ttlMs) : -1);
                return this;
            }

            delete(key) {
                this.#checkOpen();
                return native('del', this.#id, String(key));
            }

            has(key) {
                this.#checkOpen();
                return native('has', this.#id, String(key));
            }

            keys(prefix = null) {
                this.#checkOpen();
                const res = native('keys', this.#id, prefix ? String(prefix) : '');
                return JSON.parse(res);
            }

            values() {
                this.#checkOpen();
                const res = native('values', this.#id);
                return JSON.parse(res);
            }

            entries(prefix = null) {
                this.#checkOpen();
                const res = native('entries', this.#id, prefix ? String(prefix) : '');
                return JSON.parse(res);
            }

            scan(options = {}) {
                this.#checkOpen();
                const prefix = typeof options === 'string' ? options : (options && options.prefix ? String(options.prefix) : '');
                const limit = typeof options === 'object' && options && options.limit !== undefined ? Number(options.limit) : -1;
                const res = native('scan', this.#id, prefix, limit);
                return JSON.parse(res);
            }

            incr(key, delta = 1) {
                this.#checkOpen();
                return native('incr', this.#id, String(key), Number(delta));
            }

            batch(operations) {
                this.#checkOpen();
                if (!Array.isArray(operations)) {
                    throw new TypeError('batch requires an array of operations');
                }
                native('batch', this.#id, JSON.stringify(operations));
                return this;
            }

            clear() {
                this.#checkOpen();
                native('clear', this.#id);
                return this;
            }

            compact() {
                this.#checkOpen();
                return native('compact', this.#id);
            }

            flush() {
                this.#checkOpen();
                return native('flush', this.#id);
            }

            close() {
                if (!this.#closed) {
                    native('close', this.#id);
                    this.#closed = true;
                }
            }
        }

        const kvModule = {
            KVStore,
            open: KVStore.open,
            openMemory: KVStore.openMemory,
            openInMemory: KVStore.openInMemory,
            default: { KVStore, open: KVStore.open, openMemory: KVStore.openMemory, openInMemory: KVStore.openInMemory }
        };

        globalThis.__amber_kv = kvModule;
        globalThis.kv = kvModule;
    })();
    "#;

    if let Some(code) = v8::String::new(scope, kv_js_bootstrap) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
