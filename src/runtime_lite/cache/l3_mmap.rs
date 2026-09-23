// L3 Memory-Mapped Cache - Large files and cold data
//
// This module provides L3 cache using memory mapping for efficient handling
// of large script files and infrequently accessed cold data.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, RwLock};
use std::fs::File;
use std::time::{Duration, Instant};
use std::path::PathBuf;
use std::hash::{Hash, Hasher, DefaultHasher};

/// L3 Cache entry for memory-mapped files
#[derive(Debug, Clone)]
struct L3Entry {
    file_path: PathBuf,
    file_size: usize,
    last_accessed: Instant,
    access_count: u32,
    is_mapped: bool,
}
/// L3 Cache statistics
#[derive(Debug, Clone, Default)]
pub struct L3Stats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub memory_mapped_files: usize,
    pub disk_files: usize,
    pub total_memory_bytes: usize,
}
impl L3Stats {
    pub fn hit_rate(&self) -> f64 {
        let total: _ = self.hits + self.misses;
        if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        }
    }
    pub fn memory_usage_mb(&self) -> f64 {
        self.total_memory_bytes as f64 / (1024.0 * 1024.0)
    }
}
/// L3 Memory-Mapped Cache
pub struct L3MmapCache {
    /// Cache storage: key -> entry
    entries: Arc<RwLock<HashMap<String, L3Entry>>>,
    /// Statistics
    stats: Arc<RwLock<L3Stats>>,
    /// Configuration
    cache_dir: PathBuf,
    max_memory_mapped: usize,
    max_disk_files: usize,
    /// Cleanup task handle
    cleanup_interval: Duration,
}
impl L3MmapCache {
    /// Create a new L3 cache
    pub fn new() -> Self {
        let cache_dir: _ = PathBuf::from("/tmp/amberjs_l3_cache");
        std::fs::create_dir_all(&cache_dir).unwrap_or(());
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(L3Stats::default())),
            cache_dir,
            max_memory_mapped: 1024 * 1024 * 1024, // 1 GB
            max_disk_files: 10000,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
    /// Put data into L3 cache
    pub async fn put(&self, key: &str, data: &[u8]) {
        let file_name: _ = format!("{}.cache", self.hash_key(key));
        let file_path: _ = self.cache_dir.join(&file_name);
        // Write to disk
        if let Ok(mut file) = File::create(&file_path) {
            let _: _ = file.write_all(data);
        }
        // Create cache entry
        let entry: _ = L3Entry {
            file_path,
            file_size: data.len(),
            last_accessed: Instant::now(),
            access_count: 0,
            is_mapped: false,
        };
        // Store entry
        {
            let mut entries = self.entries.write().unwrap();
            entries.insert(key.to_string(), entry);
        }
        // Update statistics
        let mut stats = self.stats.write().unwrap();
        stats.disk_files += 1;
        // Perform cleanup if needed
        self.cleanup_if_needed().await;
    }
    /// Get data from L3 cache
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut stats = self.stats.write().unwrap();
        if let Some(entry) = self.entries.read().unwrap().get(key).cloned() {
            // Update access statistics
            {
                let mut entries = self.entries.write().unwrap();
                if let Some(entry_mut) = entries.get_mut(key) {
                    entry_mut.access_count += 1;
                    entry_mut.last_accessed = Instant::now();
                }
            }
            stats.hits += 1;
            // Read from file
            self.read_from_disk(key).await
        } else {
            stats.misses += 1;
            None
        }
    }
    /// Invalidate a cached entry
    pub async fn invalidate(&self, key: &str) {
        if let Some(entry) = self.entries.write().unwrap().remove(key) {
            // Delete file
            let _: _ = std::fs::remove_file(&entry.file_path);
            // Update statistics
            let mut stats = self.stats.write().unwrap();
            stats.disk_files = stats.disk_files.saturating_sub(1);
            if entry.is_mapped {
                stats.memory_mapped_files = stats.memory_mapped_files.saturating_sub(1);
            }
        }
    }
    /// Perform garbage collection
    pub async fn gc(&self) {
        let mut entries = self.entries.write().unwrap();
        // Remove old entries with low access count
        let cutoff: _ = Instant::now() - Duration::from_secs(7200); // 2 hours
        let min_access_count: _ = 3;
        let keys_to_remove: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| entry.last_accessed < cutoff && entry.access_count < min_access_count)
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys_to_remove {
            if let Some(entry) = entries.remove(&key) {
                let _: _ = std::fs::remove_file(&entry.file_path);
                drop(entry); // Explicit drop
            }
        }
        // Update statistics
        let mut stats = self.stats.write().unwrap();
        stats.disk_files = entries.len();
    }
    /// Get L3 cache statistics
    pub async fn get_stats(&self) -> L3Stats {
        self.stats.read().unwrap().clone()
    }
    /// Read data from disk
    async fn read_from_disk(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.entries.read().unwrap().get(key).cloned() {
            match File::open(&entry.file_path) {
                Ok(mut file) => {
                    let mut data = Vec::new();
                    if file.read_to_end(&mut data).is_ok() {
                        Some(data)
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        }
    }
    /// Perform cleanup if cache is over capacity
    async fn cleanup_if_needed(&self) {
        let stats: _ = self.stats.read().unwrap();
        if stats.disk_files > self.max_disk_files {
            self.evict_old_entries().await;
        }
    }
    /// Evict old entries
    async fn evict_old_entries(&self) {
        let mut entries = self.entries.write().unwrap();
        // Sort by last accessed time and collect owned keys
        let keys_to_remove: Vec<String> = {
            let mut sorted_entries: Vec<_> = entries.iter().collect();
            sorted_entries.sort_by(|a, b| a.1.last_accessed.cmp(&b.1.last_accessed));
            let to_remove: _ = sorted_entries.len() / 4;
            sorted_entries.into_iter().take(to_remove).map(|(k, _)| k.clone()).collect()
        };
        // Remove collected keys
        for key in keys_to_remove {
            if let Some(entry) = entries.remove(&key) {
                let _: _ = std::fs::remove_file(&entry.file_path);
            }
        }
    }
    /// Hash a string key
    fn hash_key(&self, key: &str) -> String {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
    /// Get current size
    pub fn size(&self) -> usize {
        self.entries.read().unwrap().len()
    }
    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().unwrap().is_empty()
    }
}
impl Default for L3MmapCache {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_l3_basic_operations() {
        let cache: _ = L3MmapCache::new();
        cache.put("test.js", b"console.log('test');").await;
        let result: _ = cache.get("test.js").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), b"console.log('test');");
    }
    #[tokio::test]
    async fn test_l3_file_storage() {
        let cache: _ = L3MmapCache::new();
        let large_data: _ = b"x".repeat(10000);
        cache.put("large.js", &large_data).await;
        let result: _ = cache.get("large.js").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 10000);
    }
    #[tokio::test]
    async fn test_l3_invalidation() {
        let cache: _ = L3MmapCache::new();
        cache.put("test.js", b"test").await;
        assert!(cache.get("test.js").await.is_some());
        cache.invalidate("test.js").await;
        assert!(cache.get("test.js").await.is_none());
    }
    #[tokio::test]
    async fn test_l3_stats() {
        let cache: _ = L3MmapCache::new();
        cache.put("test.js", b"test").await;
        cache.get("test.js").await;
        let stats: _ = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert!(stats.disk_files > 0);
    }
}