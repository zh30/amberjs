//! Disk + memory cache for TypeScript transpile output (content-hash keyed).

use crate::typescript::compiler::{CompilationOutput, TypeScriptError};
use crate::typescript::oxc_backend::BACKEND_ID;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;

static MEMORY_CACHE: Lazy<Mutex<HashMap<u64, CompilationOutput>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

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
    let key = hash_source(source, file_name);
    if let Ok(cache) = MEMORY_CACHE.lock() {
        if let Some(hit) = cache.get(&key) {
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
    if let Ok(mut cache) = MEMORY_CACHE.lock() {
        cache.clear();
    }
    let dir = cache_dir();
    if dir.exists() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_preserves_source_map() {
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
}
