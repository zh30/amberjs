//! Disk + memory cache for TypeScript transpile output (content-hash keyed).
//!
//! The memory tier is capped. A project-sized transpile used to retain every
//! JavaScript output and source map until process exit (hundreds of MiB on a
//! VS Code-sized tree). Disk entries stay so a later process can reuse them.

use crate::compiler::{CompilationOutput, TypeScriptError};
use crate::oxc_backend::BACKEND_ID;
use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Resident JavaScript + source map bytes kept in this process.
/// One oversized entry is allowed so a single large file still hits memory.
const MEMORY_CACHE_BUDGET: usize = 32 * 1024 * 1024;

static MEMORY_CACHE: Lazy<Mutex<MemoryCache>> = Lazy::new(|| Mutex::new(MemoryCache::new()));

/// Test-only override. `0` means [`MEMORY_CACHE_BUDGET`].
static BUDGET_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

/// Serializes process-global cache ops under `cfg(test)`.
#[cfg(test)]
static CACHE_TEST: Mutex<()> = Mutex::new(());

#[cfg(test)]
thread_local! {
    static IN_CACHE_CRITICAL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn with_cache_test_lock<R>(f: impl FnOnce() -> R) -> R {
    if IN_CACHE_CRITICAL.with(|c| c.get()) {
        return f();
    }
    let _guard = CACHE_TEST
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    IN_CACHE_CRITICAL.with(|c| c.set(true));
    let result = f();
    IN_CACHE_CRITICAL.with(|c| c.set(false));
    result
}

struct MemoryCache {
    map: HashMap<u64, CompilationOutput>,
    order: VecDeque<u64>,
    bytes: usize,
}

impl MemoryCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
        }
    }

    fn insert(&mut self, key: u64, output: CompilationOutput) {
        if self.map.contains_key(&key) {
            return;
        }
        let size = output_bytes(&output);
        self.map.insert(key, output);
        self.order.push_back(key);
        self.bytes += size;
        let budget = memory_budget();
        while self.map.len() > 1 && self.bytes > budget {
            let Some(old) = self.order.pop_front() else {
                break;
            };
            if let Some(removed) = self.map.remove(&old) {
                self.bytes = self.bytes.saturating_sub(output_bytes(&removed));
            }
        }
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.bytes = 0;
    }
}

fn memory_budget() -> usize {
    match BUDGET_OVERRIDE.load(Ordering::Relaxed) {
        0 => MEMORY_CACHE_BUDGET,
        budget => budget,
    }
}

fn output_bytes(output: &CompilationOutput) -> usize {
    output.js_code.len() + output.source_map.as_ref().map_or(0, String::len)
}

fn hash_source(source: &str, file_name: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    BACKEND_ID.hash(&mut hasher);
    source.hash(&mut hasher);
    file_name.hash(&mut hasher);
    hasher.finish()
}

fn cache_dir() -> PathBuf {
    std::env::temp_dir().join("amberjs-ts-cache")
}

pub fn get_cached(source: &str, file_name: &str) -> Option<CompilationOutput> {
    #[cfg(test)]
    {
        return with_cache_test_lock(|| get_cached_inner(source, file_name));
    }
    #[cfg(not(test))]
    {
        get_cached_inner(source, file_name)
    }
}

fn get_cached_inner(source: &str, file_name: &str) -> Option<CompilationOutput> {
    let key = hash_source(source, file_name);
    if let Ok(cache) = MEMORY_CACHE.lock() {
        if let Some(hit) = cache.map.get(&key) {
            return Some(hit.clone());
        }
    }
    let path = cache_dir().join(format!("{:016x}.js", key));
    if let Ok(js) = fs::read_to_string(&path) {
        let map_path = cache_dir().join(format!("{:016x}.map", key));
        let source_map = fs::read_to_string(&map_path).ok();
        let output = CompilationOutput {
            js_code: js,
            source_map,
            diagnostics: Vec::<TypeScriptError>::new(),
        };
        if let Ok(mut cache) = MEMORY_CACHE.lock() {
            cache.insert(key, output.clone());
        }
        return Some(output);
    }
    None
}

pub fn put_cached(source: &str, file_name: &str, output: &CompilationOutput) {
    #[cfg(test)]
    {
        return with_cache_test_lock(|| put_cached_inner(source, file_name, output));
    }
    #[cfg(not(test))]
    {
        put_cached_inner(source, file_name, output)
    }
}

fn put_cached_inner(source: &str, file_name: &str, output: &CompilationOutput) {
    let key = hash_source(source, file_name);
    if let Ok(mut cache) = MEMORY_CACHE.lock() {
        cache.insert(key, output.clone());
    }
    let dir = cache_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(format!("{:016x}.js", key));
    let _ = fs::write(path, &output.js_code);
    let map_path = dir.join(format!("{:016x}.map", key));
    if let Some(ref map) = output.source_map {
        let _ = fs::write(map_path, map);
    } else {
        let _ = fs::remove_file(map_path);
    }
}

/// Clear both memory and disk transpile caches.
pub fn clear_cache() {
    #[cfg(test)]
    {
        return with_cache_test_lock(|| clear_cache_inner());
    }
    #[cfg(not(test))]
    {
        clear_cache_inner()
    }
}

fn clear_cache_inner() {
    if let Ok(mut cache) = MEMORY_CACHE.lock() {
        cache.clear();
    }
    let dir = cache_dir();
    if dir.exists() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
struct BudgetGuard;

#[cfg(test)]
impl Drop for BudgetGuard {
    fn drop(&mut self) {
        BUDGET_OVERRIDE.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
fn set_test_budget(budget: usize) -> BudgetGuard {
    BUDGET_OVERRIDE.store(budget, Ordering::Relaxed);
    BudgetGuard
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Holds [`CACHE_TEST`] for the duration of a cache unit test.
    struct IsolatedCacheTest {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl IsolatedCacheTest {
        fn enter() -> Self {
            let lock = CACHE_TEST
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            IN_CACHE_CRITICAL.with(|c| c.set(true));
            Self { _lock: lock }
        }
    }

    impl Drop for IsolatedCacheTest {
        fn drop(&mut self) {
            clear_cache_inner();
            IN_CACHE_CRITICAL.with(|c| c.set(false));
        }
    }

    #[test]
    fn test_cache_preserves_source_map() {
        let _guard = IsolatedCacheTest::enter();
        clear_cache();
        let source = "const x: number = 42;";
        let file_name = "test_cache_map.ts";
        let output = CompilationOutput {
            js_code: "const x = 42;".to_string(),
            source_map: Some("{\"version\":3,\"mappings\":\"AAAA\"}".to_string()),
            diagnostics: Vec::new(),
        };

        put_cached(source, file_name, &output);

        // Verify memory cache hit preserves source_map
        let hit = get_cached(source, file_name).expect("should hit cache");
        assert_eq!(hit.js_code, "const x = 42;");
        assert_eq!(
            hit.source_map,
            Some("{\"version\":3,\"mappings\":\"AAAA\"}".to_string())
        );

        // Clear only memory cache to test disk cache read
        if let Ok(mut cache) = MEMORY_CACHE.lock() {
            cache.clear();
        }

        let disk_hit = get_cached(source, file_name).expect("should hit disk cache");
        assert_eq!(disk_hit.js_code, "const x = 42;");
        assert_eq!(
            disk_hit.source_map,
            Some("{\"version\":3,\"mappings\":\"AAAA\"}".to_string())
        );

        clear_cache();
    }

    #[test]
    fn memory_cache_evicts_oldest_and_disk_still_hits() {
        let _guard = IsolatedCacheTest::enter();
        let _budget = set_test_budget(64);
        clear_cache();
        let older = CompilationOutput {
            js_code: "a".repeat(40),
            source_map: Some("m".repeat(20)),
            diagnostics: Vec::new(),
        };
        let newer = CompilationOutput {
            js_code: "b".repeat(40),
            source_map: Some("n".repeat(20)),
            diagnostics: Vec::new(),
        };
        put_cached("source-older", "older.ts", &older);
        put_cached("source-newer", "newer.ts", &newer);

        {
            let cache = MEMORY_CACHE.lock().expect("memory cache lock");
            assert_eq!(cache.map.len(), 1, "oldest entry should be evicted");
            assert!(
                cache.bytes <= 64,
                "resident bytes {} exceeded test budget after eviction",
                cache.bytes
            );
        }
        let restored = get_cached("source-older", "older.ts").expect("disk hit");
        assert_eq!(restored.js_code, older.js_code);
        assert_eq!(restored.source_map, older.source_map);
        clear_cache();
    }
}
