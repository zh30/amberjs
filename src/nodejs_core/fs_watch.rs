//! Node `fs.watch` / `fs.watchFile` — narrow Stable slice (G64).
//!
//! `fs.watch` uses the `notify` recommended backend (inotify on Linux,
//! FSEvents on macOS, ReadDirectoryChangesW on Windows). `fs.watchFile` uses
//! real host `stat` polling (Node's model). Events are queued and delivered by
//! the runtime pump — no fabricated change callbacks.

use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

use notify::event::ModifyKind;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::permissions::{check_global_permission, PermissionAction, PermissionKind, ResourceId};

/// Queued `fs.watch` delivery for the V8 pump.
#[derive(Debug, Clone)]
pub struct PendingFsWatchEvent {
    pub watcher_id: u64,
    /// Node eventType: `"rename"` or `"change"`.
    pub event_type: &'static str,
    /// Basename (or relative name) Node passes as the second `'change'` arg.
    pub filename: Option<String>,
}

/// Queued `fs.watchFile` delivery — pump re-stats and builds Stats objects.
#[derive(Debug, Clone)]
struct PendingWatchFileTick {
    watch_id: u64,
    path: PathBuf,
    prev_size: u64,
    prev_mtime: Option<SystemTime>,
    curr_size: u64,
    curr_mtime: Option<SystemTime>,
    curr_exists: bool,
}

struct ActiveFsWatcher {
    /// Registry key mirror — kept for close/debug; map key is the live lookup.
    #[allow(dead_code)]
    id: u64,
    closed: Arc<AtomicBool>,
}

struct ActiveWatchFile {
    /// Registry bookkeeping — poll thread holds cloned Arcs; these keep them alive.
    #[allow(dead_code)]
    id: u64,
    #[allow(dead_code)]
    path: PathBuf,
    stopped: Arc<AtomicBool>,
    #[allow(dead_code)]
    last_size: Arc<Mutex<u64>>,
    #[allow(dead_code)]
    last_mtime: Arc<Mutex<Option<SystemTime>>>,
    #[allow(dead_code)]
    last_exists: Arc<Mutex<bool>>,
}

static WATCHER_ID: AtomicU64 = AtomicU64::new(1);
static WATCH_FILE_ID: AtomicU64 = AtomicU64::new(1);

static ACTIVE_FS_WATCHERS: Lazy<Mutex<HashMap<u64, ActiveFsWatcher>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static ACTIVE_WATCH_FILES: Lazy<Mutex<HashMap<u64, ActiveWatchFile>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
/// path → watchFile ids (for `unwatchFile`).
static WATCH_FILE_BY_PATH: Lazy<Mutex<HashMap<PathBuf, Vec<u64>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static WATCH_TX: Lazy<RwLock<Option<crossbeam::channel::Sender<PendingFsWatchEvent>>>> =
    Lazy::new(|| RwLock::new(None));
static WATCH_RX: Lazy<RwLock<Option<crossbeam::channel::Receiver<PendingFsWatchEvent>>>> =
    Lazy::new(|| RwLock::new(None));

static WATCH_FILE_TX: Lazy<RwLock<Option<crossbeam::channel::Sender<PendingWatchFileTick>>>> =
    Lazy::new(|| RwLock::new(None));
static WATCH_FILE_RX: Lazy<RwLock<Option<crossbeam::channel::Receiver<PendingWatchFileTick>>>> =
    Lazy::new(|| RwLock::new(None));

fn ensure_channels() {
    let need = WATCH_RX.read().map(|g| g.is_none()).unwrap_or(true);
    if need {
        let (tx, rx) = crossbeam::channel::bounded(2048);
        if let Ok(mut g) = WATCH_TX.write() {
            *g = Some(tx);
        }
        if let Ok(mut g) = WATCH_RX.write() {
            *g = Some(rx);
        }
    }
    let need_wf = WATCH_FILE_RX.read().map(|g| g.is_none()).unwrap_or(true);
    if need_wf {
        let (tx, rx) = crossbeam::channel::bounded(2048);
        if let Ok(mut g) = WATCH_FILE_TX.write() {
            *g = Some(tx);
        }
        if let Ok(mut g) = WATCH_FILE_RX.write() {
            *g = Some(rx);
        }
    }
}

/// Reset registries (contract tests between cases).
pub fn reset_fs_watch_state() {
    {
        let mut map = ACTIVE_FS_WATCHERS.lock().unwrap_or_else(|p| p.into_inner());
        for w in map.values() {
            w.closed.store(true, Ordering::SeqCst);
        }
        map.clear();
    }
    {
        let mut map = ACTIVE_WATCH_FILES.lock().unwrap_or_else(|p| p.into_inner());
        for w in map.values() {
            w.stopped.store(true, Ordering::SeqCst);
        }
        map.clear();
    }
    WATCH_FILE_BY_PATH
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clear();
    let (tx, rx) = crossbeam::channel::bounded(2048);
    if let Ok(mut g) = WATCH_TX.write() {
        *g = Some(tx);
    }
    if let Ok(mut g) = WATCH_RX.write() {
        *g = Some(rx);
    }
    let (tx2, rx2) = crossbeam::channel::bounded(2048);
    if let Ok(mut g) = WATCH_FILE_TX.write() {
        *g = Some(tx2);
    }
    if let Ok(mut g) = WATCH_FILE_RX.write() {
        *g = Some(rx2);
    }
}

pub fn has_active_fs_watchers() -> bool {
    let watchers = ACTIVE_FS_WATCHERS
        .lock()
        .map(|m| m.values().any(|w| !w.closed.load(Ordering::SeqCst)))
        .unwrap_or(false);
    let files = ACTIVE_WATCH_FILES
        .lock()
        .map(|m| m.values().any(|w| !w.stopped.load(Ordering::SeqCst)))
        .unwrap_or(false);
    watchers || files
}

pub fn has_pending_fs_watch_events() -> bool {
    let a = WATCH_RX
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|rx| !rx.is_empty()))
        .unwrap_or(false);
    let b = WATCH_FILE_RX
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|rx| !rx.is_empty()))
        .unwrap_or(false);
    a || b
}

fn enqueue_watch(ev: PendingFsWatchEvent) {
    if let Ok(g) = WATCH_TX.read() {
        if let Some(ref tx) = *g {
            let _ = tx.try_send(ev);
        }
    }
}

fn enqueue_watch_file(ev: PendingWatchFileTick) {
    if let Ok(g) = WATCH_FILE_TX.read() {
        if let Some(ref tx) = *g {
            let _ = tx.try_send(ev);
        }
    }
}

fn node_event_type(kind: EventKind) -> Option<&'static str> {
    match kind {
        EventKind::Create(_) | EventKind::Remove(_) => Some("rename"),
        EventKind::Modify(ModifyKind::Name(_)) => Some("rename"),
        EventKind::Modify(_) | EventKind::Any => Some("change"),
        EventKind::Access(_) | EventKind::Other => None,
    }
}

fn basename_for_event(watch_root: &Path, event_path: &Path) -> Option<String> {
    if watch_root.is_file() {
        return watch_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
    }
    event_path
        .strip_prefix(watch_root)
        .ok()
        .map(|rel| rel.to_string_lossy().into_owned())
        .or_else(|| {
            event_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
}

fn check_fs_read(scope: &mut v8::PinScope, path: &str) -> bool {
    if !crate::permissions::has_restrictions() {
        return true;
    }
    match check_global_permission(
        PermissionKind::FileSystem,
        PermissionAction::Read,
        ResourceId::Path(PathBuf::from(path)),
    ) {
        Ok(()) => true,
        Err(error) => {
            let message = v8::String::new(scope, &error.to_string()).unwrap();
            let exception = v8::Exception::type_error(scope, message);
            scope.throw_exception(exception);
            false
        }
    }
}

fn throw_enoent(scope: &mut v8::PinScope, syscall: &str, path: &str) {
    let err = std::io::Error::from_raw_os_error(libc::ENOENT);
    let code = "ENOENT";
    let message = format!("{code}: {err}, {syscall} '{path}'");
    let message = v8::String::new(scope, &message).unwrap();
    let exception = v8::Exception::error(scope, message);
    if let Some(obj) = exception.to_object(scope) {
        let code_key = v8::String::new(scope, "code").unwrap();
        let code_val = v8::String::new(scope, code).unwrap();
        obj.set(scope, code_key.into(), code_val.into());
        let syscall_key = v8::String::new(scope, "syscall").unwrap();
        let syscall_val = v8::String::new(scope, syscall).unwrap();
        obj.set(scope, syscall_key.into(), syscall_val.into());
        let path_key = v8::String::new(scope, "path").unwrap();
        let path_val = v8::String::new(scope, path).unwrap();
        obj.set(scope, path_key.into(), path_val.into());
        let errno_key = v8::String::new(scope, "errno").unwrap();
        let errno_val = v8::Integer::new(scope, -libc::ENOENT);
        obj.set(scope, errno_key.into(), errno_val.into());
    }
    scope.throw_exception(exception);
}

fn arg_path(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> String {
    value
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

fn option_bool(scope: &mut v8::PinScope, value: v8::Local<v8::Value>, key: &str) -> bool {
    if !value.is_object() || value.is_null() || value.is_function() {
        return false;
    }
    let Ok(obj) = v8::Local::<v8::Object>::try_from(value) else {
        return false;
    };
    let Some(k) = v8::String::new(scope, key) else {
        return false;
    };
    obj.get(scope, k.into())
        .map(|v| v.to_boolean(scope).is_true())
        .unwrap_or(false)
}

fn option_u64(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
    key: &str,
    default: u64,
) -> u64 {
    if !value.is_object() || value.is_null() || value.is_function() {
        return default;
    }
    let Ok(obj) = v8::Local::<v8::Object>::try_from(value) else {
        return default;
    };
    let Some(k) = v8::String::new(scope, key) else {
        return default;
    };
    obj.get(scope, k.into())
        .and_then(|v| v.to_number(scope))
        .map(|n| n.value() as u64)
        .filter(|&n| n > 0)
        .unwrap_or(default)
}

fn metadata_snapshot(path: &Path) -> (bool, u64, Option<SystemTime>) {
    match std::fs::metadata(path) {
        Ok(meta) => (true, meta.len(), meta.modified().ok()),
        Err(_) => (false, 0, None),
    }
}

fn spawn_notify_watcher(
    watcher_id: u64,
    path: PathBuf,
    recursive: bool,
    closed: Arc<AtomicBool>,
) -> Result<(), String> {
    ensure_channels();
    let mode = if recursive && path.is_dir() {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    let watch_root = path.clone();
    let (notify_tx, notify_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(notify_tx, notify::Config::default())
        .map_err(|e| format!("watch failed: {e}"))?;
    watcher
        .watch(&path, mode)
        .map_err(|e| format!("watch failed: {e}"))?;

    thread::Builder::new()
        .name(format!("amber-fs-watch-{watcher_id}"))
        .spawn(move || {
            let _watcher = watcher;
            while !closed.load(Ordering::SeqCst) {
                match notify_rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(Ok(event)) => {
                        if closed.load(Ordering::SeqCst) {
                            break;
                        }
                        let Some(event_type) = node_event_type(event.kind) else {
                            continue;
                        };
                        if event.paths.is_empty() {
                            enqueue_watch(PendingFsWatchEvent {
                                watcher_id,
                                event_type,
                                filename: basename_for_event(&watch_root, &watch_root),
                            });
                            continue;
                        }
                        for event_path in event.paths {
                            enqueue_watch(PendingFsWatchEvent {
                                watcher_id,
                                event_type,
                                filename: basename_for_event(&watch_root, &event_path),
                            });
                        }
                    }
                    Ok(Err(_)) => {
                        // Backend error — stop this watcher; pump may deliver 'error' later if needed.
                        closed.store(true, Ordering::SeqCst);
                        break;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .map_err(|e| format!("watch thread: {e}"))?;
    Ok(())
}

fn spawn_watch_file_poller(
    watch_id: u64,
    path: PathBuf,
    interval_ms: u64,
    stopped: Arc<AtomicBool>,
    last_size: Arc<Mutex<u64>>,
    last_mtime: Arc<Mutex<Option<SystemTime>>>,
    last_exists: Arc<Mutex<bool>>,
) {
    ensure_channels();
    thread::Builder::new()
        .name(format!("amber-fs-watchfile-{watch_id}"))
        .spawn(move || {
            let interval = Duration::from_millis(interval_ms.max(1));
            while !stopped.load(Ordering::SeqCst) {
                thread::sleep(interval);
                if stopped.load(Ordering::SeqCst) {
                    break;
                }
                let (exists, size, mtime) = metadata_snapshot(&path);
                let prev_size = *last_size.lock().unwrap_or_else(|p| p.into_inner());
                let prev_mtime = *last_mtime.lock().unwrap_or_else(|p| p.into_inner());
                let prev_exists = *last_exists.lock().unwrap_or_else(|p| p.into_inner());
                let changed = exists != prev_exists || size != prev_size || mtime != prev_mtime;
                if !changed {
                    continue;
                }
                enqueue_watch_file(PendingWatchFileTick {
                    watch_id,
                    path: path.clone(),
                    prev_size,
                    prev_mtime,
                    curr_size: size,
                    curr_mtime: mtime,
                    curr_exists: exists,
                });
                *last_size.lock().unwrap_or_else(|p| p.into_inner()) = size;
                *last_mtime.lock().unwrap_or_else(|p| p.into_inner()) = mtime;
                *last_exists.lock().unwrap_or_else(|p| p.into_inner()) = exists;
            }
        })
        .ok();
}

/// Install `fs.watch` / `fs.watchFile` / `fs.unwatchFile` + FSWatcher EventEmitter bootstrap.
/// Must run after `events::setup_events_api` so `EventEmitter` exists.
pub fn install_fs_watch_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    ensure_channels();

    let global = context.global(scope);
    let fs_key = v8::String::new(scope, "fs").unwrap();
    let Some(fs_val) = global.get(scope, fs_key.into()) else {
        return Ok(());
    };
    if !fs_val.is_object() {
        return Ok(());
    }
    let Ok(fs_obj) = v8::Local::<v8::Object>::try_from(fs_val) else {
        return Ok(());
    };

    let watch_fn = v8::FunctionTemplate::new(scope, fs_watch_callback);
    let watch_inst = watch_fn.get_function(scope).unwrap();
    let watch_key = v8::String::new(scope, "watch").unwrap();
    fs_obj.set(scope, watch_key.into(), watch_inst.into());

    let watch_file_fn = v8::FunctionTemplate::new(scope, fs_watch_file_callback);
    let watch_file_inst = watch_file_fn.get_function(scope).unwrap();
    let watch_file_key = v8::String::new(scope, "watchFile").unwrap();
    fs_obj.set(scope, watch_file_key.into(), watch_file_inst.into());

    let unwatch_fn = v8::FunctionTemplate::new(scope, fs_unwatch_file_callback);
    let unwatch_inst = unwatch_fn.get_function(scope).unwrap();
    let unwatch_key = v8::String::new(scope, "unwatchFile").unwrap();
    fs_obj.set(scope, unwatch_key.into(), unwatch_inst.into());

    // EventEmitter-backed FSWatcher + watchFile listener registry.
    let bootstrap = r#"
        (function() {
            const fs = globalThis.fs;
            if (!fs || typeof fs.watch !== 'function') return;
            const EE = globalThis.EventEmitter || function() {};
            const proto = typeof EE === 'function' ? EE.prototype : Object.prototype;
            if (!globalThis._fsWatchers) {
                globalThis._fsWatchers = Object.create(null);
            }
            if (!globalThis._fsWatchFiles) {
                globalThis._fsWatchFiles = Object.create(null);
            }
            function FSWatcher() {
                this._events = Object.create(null);
            }
            FSWatcher.prototype = Object.create(proto);
            FSWatcher.prototype.constructor = FSWatcher;
            const _nativeWatch = fs.watch;
            fs.watch = function(filename, options, listener) {
                let lis = listener;
                if (typeof options === 'function') {
                    lis = options;
                }
                const watcher = _nativeWatch.call(this, filename, options, listener);
                if (!watcher || typeof watcher !== 'object') return watcher;
                Object.setPrototypeOf(watcher, FSWatcher.prototype);
                watcher._events = Object.create(null);
                const id = watcher._fsWatcherId;
                if (id != null) {
                    globalThis._fsWatchers[String(id)] = watcher;
                }
                if (typeof lis === 'function') {
                    watcher.on('change', lis);
                }
                return watcher;
            };
            const _nativeWatchFile = fs.watchFile;
            fs.watchFile = function(filename, options, listener) {
                let opts = options;
                let lis = listener;
                if (typeof options === 'function') {
                    lis = options;
                    opts = undefined;
                }
                const result = _nativeWatchFile.call(this, filename, opts, lis);
                const id = result && result._fsWatchFileId;
                if (id != null && typeof lis === 'function') {
                    globalThis._fsWatchFiles[String(id)] = { listener: lis, path: String(filename) };
                }
                return undefined;
            };
        })();
    "#;
    let code = v8::String::new(scope, bootstrap).unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let _ = script.run(scope);
    Ok(())
}

fn fs_watch_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    ensure_channels();
    let path = arg_path(scope, args.get(0));
    if path.is_empty() {
        throw_enoent(scope, "watch", &path);
        return;
    }
    if !check_fs_read(scope, &path) {
        return;
    }
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        throw_enoent(scope, "watch", &path);
        return;
    }

    let arg1 = args.get(1);
    let arg2 = args.get(2);
    let (options, _listener) = if arg1.is_function() {
        (v8::undefined(scope).into(), arg1)
    } else {
        (arg1, arg2)
    };
    let recursive = option_bool(scope, options, "recursive");

    let watcher_id = WATCHER_ID.fetch_add(1, Ordering::Relaxed);
    let closed = Arc::new(AtomicBool::new(false));
    if let Err(msg) = spawn_notify_watcher(watcher_id, path_buf, recursive, closed.clone()) {
        let message = v8::String::new(scope, &msg).unwrap();
        let exception = v8::Exception::error(scope, message);
        scope.throw_exception(exception);
        return;
    }

    ACTIVE_FS_WATCHERS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(
            watcher_id,
            ActiveFsWatcher {
                id: watcher_id,
                closed: closed.clone(),
            },
        );

    let watcher_obj = v8::Object::new(scope);
    let id_key = v8::String::new(scope, "_fsWatcherId").unwrap();
    let id_val = v8::Number::new(scope, watcher_id as f64);
    watcher_obj.set(scope, id_key.into(), id_val.into());

    let close_fn = v8::FunctionTemplate::new(scope, fs_watcher_close_callback);
    let close_inst = close_fn.get_function(scope).unwrap();
    let close_key = v8::String::new(scope, "close").unwrap();
    watcher_obj.set(scope, close_key.into(), close_inst.into());

    // Register early so pump finds the object after JS prototype wrap.
    let context = scope.get_current_context();
    let global = context.global(scope);
    let registry_key = v8::String::new(scope, "_fsWatchers").unwrap();
    let registry = if let Some(existing) = global.get(scope, registry_key.into()) {
        if existing.is_object() {
            v8::Local::<v8::Object>::try_from(existing).unwrap_or_else(|_| v8::Object::new(scope))
        } else {
            v8::Object::new(scope)
        }
    } else {
        v8::Object::new(scope)
    };
    let id_str = v8::String::new(scope, &watcher_id.to_string()).unwrap();
    registry.set(scope, id_str.into(), watcher_obj.into());
    global.set(scope, registry_key.into(), registry.into());

    retval.set(watcher_obj.into());
}

fn fs_watcher_close_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let id_key = v8::String::new(scope, "_fsWatcherId").unwrap();
    let watcher_id = this
        .get(scope, id_key.into())
        .and_then(|v| v.to_number(scope))
        .map(|n| n.value() as u64)
        .unwrap_or(0);
    if watcher_id != 0 {
        if let Ok(mut map) = ACTIVE_FS_WATCHERS.lock() {
            if let Some(w) = map.remove(&watcher_id) {
                w.closed.store(true, Ordering::SeqCst);
            }
        }
        let context = scope.get_current_context();
        let global = context.global(scope);
        let registry_key = v8::String::new(scope, "_fsWatchers").unwrap();
        if let Some(reg_val) = global.get(scope, registry_key.into()) {
            if let Ok(reg) = v8::Local::<v8::Object>::try_from(reg_val) {
                let id_str = v8::String::new(scope, &watcher_id.to_string()).unwrap();
                let _ = reg.delete(scope, id_str.into());
            }
        }
    }
    retval.set(v8::undefined(scope).into());
}

fn fs_watch_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    ensure_channels();
    let path = arg_path(scope, args.get(0));
    if path.is_empty() {
        throw_enoent(scope, "watchFile", &path);
        return;
    }
    if !check_fs_read(scope, &path) {
        return;
    }

    let arg1 = args.get(1);
    let arg2 = args.get(2);
    let (options, listener) = if arg1.is_function() {
        (v8::undefined(scope).into(), arg1)
    } else {
        (arg1, arg2)
    };
    if !listener.is_function() {
        let msg =
            v8::String::new(scope, "The \"listener\" argument must be of type function").unwrap();
        let err = v8::Exception::type_error(scope, msg);
        scope.throw_exception(err);
        return;
    }

    let interval_ms = option_u64(scope, options, "interval", 100);
    let path_buf = PathBuf::from(&path);
    let (exists, size, mtime) = metadata_snapshot(&path_buf);

    let watch_id = WATCH_FILE_ID.fetch_add(1, Ordering::Relaxed);
    let stopped = Arc::new(AtomicBool::new(false));
    let last_size = Arc::new(Mutex::new(size));
    let last_mtime = Arc::new(Mutex::new(mtime));
    let last_exists = Arc::new(Mutex::new(exists));

    spawn_watch_file_poller(
        watch_id,
        path_buf.clone(),
        interval_ms,
        stopped.clone(),
        last_size.clone(),
        last_mtime.clone(),
        last_exists.clone(),
    );

    ACTIVE_WATCH_FILES
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(
            watch_id,
            ActiveWatchFile {
                id: watch_id,
                path: path_buf.clone(),
                stopped,
                last_size,
                last_mtime,
                last_exists,
            },
        );
    WATCH_FILE_BY_PATH
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .entry(path_buf)
        .or_default()
        .push(watch_id);

    // Return a tiny handle so the JS wrap can register the listener by id.
    // Node returns undefined; bootstrap discards this and returns undefined.
    let handle = v8::Object::new(scope);
    let id_key = v8::String::new(scope, "_fsWatchFileId").unwrap();
    handle.set(
        scope,
        id_key.into(),
        v8::Number::new(scope, watch_id as f64).into(),
    );
    retval.set(handle.into());
}

fn fs_unwatch_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_path(scope, args.get(0));
    let path_buf = PathBuf::from(&path);
    let ids = {
        let mut by_path = WATCH_FILE_BY_PATH.lock().unwrap_or_else(|p| p.into_inner());
        by_path.remove(&path_buf).unwrap_or_default()
    };
    if let Ok(mut map) = ACTIVE_WATCH_FILES.lock() {
        for id in &ids {
            if let Some(w) = map.remove(id) {
                w.stopped.store(true, Ordering::SeqCst);
            }
        }
    }
    let context = scope.get_current_context();
    let global = context.global(scope);
    let registry_key = v8::String::new(scope, "_fsWatchFiles").unwrap();
    if let Some(reg_val) = global.get(scope, registry_key.into()) {
        if let Ok(reg) = v8::Local::<v8::Object>::try_from(reg_val) {
            for id in ids {
                let id_str = v8::String::new(scope, &id.to_string()).unwrap();
                let _ = reg.delete(scope, id_str.into());
            }
        }
    }
    retval.set(v8::undefined(scope).into());
}

fn stats_shaped<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    path: &Path,
    exists: bool,
    size: u64,
    mtime: Option<SystemTime>,
) -> v8::Local<'a, v8::Object> {
    if exists {
        if let Ok(meta) = std::fs::metadata(path) {
            return super::fs::create_stats_object(scope, &meta);
        }
    }
    // Missing / race: Stats-shaped object with observed size + mtimeMs.
    let obj = v8::Object::new(scope);
    let size_key = v8::String::new(scope, "size").unwrap();
    obj.set(
        scope,
        size_key.into(),
        v8::Number::new(scope, size as f64).into(),
    );
    let mtime_ms = mtime
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0);
    let mtime_ms_key = v8::String::new(scope, "mtimeMs").unwrap();
    obj.set(
        scope,
        mtime_ms_key.into(),
        v8::Number::new(scope, mtime_ms).into(),
    );
    let is_file_key = v8::String::new(scope, "__isFile").unwrap();
    obj.set(
        scope,
        is_file_key.into(),
        v8::Boolean::new(scope, exists).into(),
    );
    obj
}

/// Drain queued watch / watchFile events into JS listeners.
pub fn pump_pending_fs_watch_events_in_scope(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> usize {
    let mut processed = 0usize;
    processed += pump_watch_events(scope, context);
    processed += pump_watch_file_events(scope, context);
    processed
}

fn pump_watch_events(scope: &mut v8::PinScope, context: &v8::Local<v8::Context>) -> usize {
    let rx = if let Ok(guard) = WATCH_RX.read() {
        guard.clone()
    } else {
        None
    };
    let Some(rx) = rx else {
        return 0;
    };

    let global = context.global(scope);
    let registry_key = v8::String::new(scope, "_fsWatchers").unwrap();
    let registry = match global.get(scope, registry_key.into()) {
        Some(val) if val.is_object() => match v8::Local::<v8::Object>::try_from(val) {
            Ok(obj) => obj,
            Err(_) => return 0,
        },
        _ => return 0,
    };

    let mut processed = 0usize;
    while let Ok(pending) = rx.try_recv() {
        // Skip closed watchers.
        let still_active = ACTIVE_FS_WATCHERS
            .lock()
            .map(|m| {
                m.get(&pending.watcher_id)
                    .map(|w| !w.closed.load(Ordering::SeqCst))
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if !still_active {
            continue;
        }

        let id_key = v8::String::new(scope, &pending.watcher_id.to_string()).unwrap();
        let Some(watcher_val) = registry.get(scope, id_key.into()) else {
            continue;
        };
        if !watcher_val.is_object() {
            continue;
        }
        let Ok(watcher_obj) = v8::Local::<v8::Object>::try_from(watcher_val) else {
            continue;
        };

        let emit_key = v8::String::new(scope, "emit").unwrap();
        if let Some(emit_val) = watcher_obj.get(scope, emit_key.into()) {
            if let Ok(emit_func) = v8::Local::<v8::Function>::try_from(emit_val) {
                let event = v8::String::new(scope, "change").unwrap();
                let event_type = v8::String::new(scope, pending.event_type).unwrap();
                let filename = match &pending.filename {
                    Some(name) => v8::String::new(scope, name).unwrap().into(),
                    None => v8::null(scope).into(),
                };
                let _ = emit_func.call(
                    scope,
                    watcher_obj.into(),
                    &[event.into(), event_type.into(), filename],
                );
            }
        }
        processed += 1;
        if processed >= 256 {
            break;
        }
    }
    processed
}

fn pump_watch_file_events(scope: &mut v8::PinScope, context: &v8::Local<v8::Context>) -> usize {
    let rx = if let Ok(guard) = WATCH_FILE_RX.read() {
        guard.clone()
    } else {
        None
    };
    let Some(rx) = rx else {
        return 0;
    };

    let global = context.global(scope);
    let registry_key = v8::String::new(scope, "_fsWatchFiles").unwrap();
    let registry = match global.get(scope, registry_key.into()) {
        Some(val) if val.is_object() => match v8::Local::<v8::Object>::try_from(val) {
            Ok(obj) => obj,
            Err(_) => return 0,
        },
        _ => return 0,
    };

    let mut processed = 0usize;
    while let Ok(pending) = rx.try_recv() {
        let still_active = ACTIVE_WATCH_FILES
            .lock()
            .map(|m| {
                m.get(&pending.watch_id)
                    .map(|w| !w.stopped.load(Ordering::SeqCst))
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if !still_active {
            continue;
        }

        let id_key = v8::String::new(scope, &pending.watch_id.to_string()).unwrap();
        let Some(entry_val) = registry.get(scope, id_key.into()) else {
            continue;
        };
        if !entry_val.is_object() {
            continue;
        }
        let Ok(entry) = v8::Local::<v8::Object>::try_from(entry_val) else {
            continue;
        };
        let listener_key = v8::String::new(scope, "listener").unwrap();
        let Some(listener_val) = entry.get(scope, listener_key.into()) else {
            continue;
        };
        let Ok(listener) = v8::Local::<v8::Function>::try_from(listener_val) else {
            continue;
        };

        let curr = stats_shaped(
            scope,
            &pending.path,
            pending.curr_exists,
            pending.curr_size,
            pending.curr_mtime,
        );
        let prev = stats_shaped(
            scope,
            &pending.path,
            pending.prev_mtime.is_some() || pending.prev_size > 0,
            pending.prev_size,
            pending.prev_mtime,
        );
        // Prefer exact prev size/mtimeMs from the poll snapshot (not re-stat).
        {
            let size_key = v8::String::new(scope, "size").unwrap();
            prev.set(
                scope,
                size_key.into(),
                v8::Number::new(scope, pending.prev_size as f64).into(),
            );
            let mtime_ms = pending
                .prev_mtime
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64() * 1000.0)
                .unwrap_or(0.0);
            let mtime_ms_key = v8::String::new(scope, "mtimeMs").unwrap();
            prev.set(
                scope,
                mtime_ms_key.into(),
                v8::Number::new(scope, mtime_ms).into(),
            );
        }

        let undef = v8::undefined(scope).into();
        let _ = listener.call(scope, undef, &[curr.into(), prev.into()]);
        processed += 1;
        if processed >= 256 {
            break;
        }
    }
    processed
}
