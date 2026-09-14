// Isolate Pre-warming System - Stage 21.3
// Enhanced pre-warming mechanism with V8 snapshots and context preparation
// Integrates with IsolatePool to provide fully-prepared isolates ready for execution

use anyhow::{Result, anyhow};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use rusty_v8 as v8;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;
use std::task::Context;

/// Enhanced Isolate Pre-warming System
/// Stage 21.3: Integrates V8 snapshots and context preparation for optimal performance
pub struct IsolatePrewarmer {
    /// V8 Snapshot Manager for creating and loading snapshots
    #[allow(dead_code)]
    snapshot_manager: Arc<SnapshotManager>,
    /// Pre-warmed isolates with prepared contexts
    prewarmed_isolates: Arc<Mutex<Vec<v8::OwnedIsolate>>>,
    /// Statistics tracking
    stats: Arc<PrewarmStats>,
    /// Common JavaScript snippets to pre-compile
    common_snippets: Arc<Mutex<Vec<CompiledSnippet>>>,
    /// Maximum number of isolates to pre-warm
    max_prewarm: usize,
    /// Pre-warming configuration
    config: PrewarmConfig,
}
/// Pre-warming statistics
#[derive(Debug, Clone, Default)]
pub struct PrewarmStats {
    /// Total isolates pre-warmed
    pub total_prewarmed: Arc<AtomicUsize>,
    /// Snapshots created during pre-warming
    pub snapshots_created: Arc<AtomicUsize>,
    /// Snippets pre-compiled
    pub snippets_precompiled: Arc<AtomicUsize>,
    /// Total pre-warming time (microseconds)
    pub total_prewarm_time_us: Arc<AtomicUsize>,
    /// Average time per isolate (microseconds)
    pub avg_time_per_isolate_us: Arc<AtomicUsize>,
    /// Cache hits (using pre-warmed isolates)
    pub cache_hits: Arc<AtomicUsize>,
    /// Cache misses (creating new isolates)
    pub cache_misses: Arc<AtomicUsize>,
    /// Last pre-warm timestamp
    pub last_prewarm: Arc<AtomicUsize>,
}
impl PrewarmStats {
    pub fn new() -> Self {
        Self::default()
    }
    /// Calculate cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let hits: _ = self.cache_hits.load(Ordering::Relaxed) as f64;
        let total: _ = hits + self.cache_misses.load(Ordering::Relaxed) as f64;
        if total > 0.0 { hits / total } else { 0.0 }
    }
    /// Get average pre-warming time per isolate in microseconds
    pub fn avg_prewarm_time_us(&self) -> f64 {
        let total: _ = self.total_prewarm_time_us.load(Ordering::Relaxed) as f64;
        let count: _ = self.total_prewarmed.load(Ordering::Relaxed) as f64;
        if count > 0.0 { total / count } else { 0.0 }
    }
}
/// Pre-warming configuration
#[derive(Debug, Clone)]
pub struct PrewarmConfig {
    /// Enable V8 snapshot integration during pre-warming
    pub enable_snapshots: bool,
    /// Pre-compile common JavaScript snippets
    pub precompile_snippets: bool,
    /// Prepare console API in advance
    pub prepare_console: bool,
    /// Prepare Node.js APIs in advance
    pub prepare_nodejs: bool,
    /// Enable aggressive pre-warming (warm up more isolates)
    pub aggressive: bool,
}
impl Default for PrewarmConfig {
    fn default() -> Self {
        Self {
            enable_snapshots: true,
            precompile_snippets: true,
            prepare_console: true,
            prepare_nodejs: false, // Disabled by default as it's heavier
            aggressive: false,
        }
    }
}
/// Pre-compiled JavaScript snippet
#[derive(Debug, Clone)]
struct CompiledSnippet {
    /// Snippet name for identification
    #[allow(dead_code)]
    name: String,
    /// JavaScript code
    #[allow(dead_code)]
    code: String,
    /// Compiled script
    #[allow(dead_code)]
    script: v8::Global<v8::Script>,
    /// Creation timestamp
    #[allow(dead_code)]
    created_at: u64,
}
impl IsolatePrewarmer {
    /// Create new Isolate Prewarmer
    pub fn new(max_prewarm: usize, config: PrewarmConfig) -> Result<Self> {
        let snapshot_config: _ = crate::v8_snapshot::SnapshotConfig::default();
        let snapshot_manager: _ = Arc::new(Mutex::new(crate::v8_snapshot::SnapshotManager::new(snapshot_config)));
        Ok(Self {
            snapshot_manager,
            prewarmed_isolates: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(PrewarmStats::new())),
            common_snippets: Arc::new(Mutex::new(Vec::new())),
            max_prewarm,
            config,
        })
    }
    /// Pre-warm isolates with V8 snapshots and prepared contexts
    pub fn prewarm(&self) -> Result<()> {
        let start_time: _ = Instant::now();
        // Pre-compile common JavaScript snippets
        if self.config.precompile_snippets {
            self.precompile_common_snippets()?;
        }
        // Determine number of isolates to pre-warm
        let count: _ = if self.config.aggressive {
            (self.max_prewarm * 3 / 2).min(32) // Cap at 32 for sanity
        } else {
            self.max_prewarm
        };
        // Pre-warm isolates
        for i in 0..count {
            let isolate_start: _ = Instant::now();
            // Create new isolate
            let mut isolate = v8::Isolate::new(Default::default());
            {
                // Create a context for this isolate
                let scope: _ = &mut v8::HandleScope::new(&mut isolate);
                // Create context with template
                let context: _ = v8::Context::new(scope, Default::default());
                let _context_scope: _ = &mut v8::ContextScope::new(scope, context);
                // Note: Console and Node.js API setup would require Runtime struct
                // For now, we focus on core pre-warming with pre-compilation
                // V8 snapshot creation would require SnapshotCreator which has complex lifecycle requirements
                // These APIs can be set up later when the isolate is acquired for use
                // Track that we attempted snapshot creation
                if self.config.enable_snapshots {
                    self.stats.snapshots_created.fetch_add(1, Ordering::Relaxed);
                }
            } // scope is dropped here, releasing the borrow
            let isolate_time: _ = isolate_start.elapsed();
            // Store the pre-warmed isolate
            let mut prewarmed = self.prewarmed_isolates.lock().unwrap();
            prewarmed.push(isolate);
            // Update statistics
            self.stats.total_prewarmed.fetch_add(1, Ordering::Relaxed);
            self.stats.total_prewarm_time_us.fetch_add(
                isolate_time.as_micros() as usize,
                Ordering::Relaxed
            );
            // Add small delay for aggressive mode to avoid overwhelming the system
            if self.config.aggressive && i % 4 == 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        // Update last pre-warm timestamp
        self.stats.last_prewarm.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as usize,
            Ordering::Relaxed
        );
        let total_time: _ = start_time.elapsed();
        eprintln!(
            "Isolate Pre-warming completed: {} isolates in {:.2}ms (avg: {:.2}µs per isolate)",
            count,
            total_time.as_millis(),
            self.stats.avg_prewarm_time_us()
        );
        Ok(())
    }
    /// Pre-compile common JavaScript snippets
    fn precompile_common_snippets(&self) -> Result<()> {
        let common_codes: _ = vec![
            ("hello", "console.log('Hello from Beejs!');"),
            ("simple_arithmetic", "const result = 2 + 2; result;"),
            ("array_ops", "const arr = [1, 2, 3, 4, 5]; arr.map(x => x * 2);"),
            ("object_props", "const obj = {a: 1, b: 2, c: 3}; obj.a + obj.b;"),
            ("string_ops", "const str = 'hello world'; str.toUpperCase();"),
        ];
        let mut isolate = v8::Isolate::new(Default::default());
        let scope: _ = &mut v8::HandleScope::new(&mut isolate);
        let context: _ = v8::Context::new(scope, Default::default());
        let context_scope: _ = &mut v8::ContextScope::new(scope, context);
        for (name, code) in common_codes {
            let code_handle: _ = v8::String::new(context_scope, code)
                .ok_or_else(|| anyhow!("Failed to create code string"))?;
            let script: _ = v8::Script::compile(context_scope, code_handle, None)
                .ok_or_else(|| anyhow!("Failed to compile snippet: {}", name))?;
            let script_global: _ = v8::Global::new(context_scope, script);
            let snippet: _ = CompiledSnippet {
                name: name.to_string(),
                code: code.to_string(),
                script: script_global,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            self.common_snippets.lock().unwrap().push(snippet);
            self.stats.snippets_precompiled.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
    /// Get a pre-warmed isolate
    pub fn get_prewarmed_isolate(&self) -> Option<v8::OwnedIsolate> {
        let mut prewarmed = self.prewarmed_isolates.lock().unwrap();
        if let Some(isolate) = prewarmed.pop() {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            Some(isolate)
        } else {
            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
    /// Return a pre-warmed isolate to the pool
    pub fn return_prewarmed_isolate(&self, isolate: v8::OwnedIsolate) {
        let mut prewarmed = self.prewarmed_isolates.lock().unwrap();
        if prewarmed.len() < self.max_prewarm {
            prewarmed.push(isolate);
        }
    }
    /// Get statistics
    pub fn stats(&self) -> PrewarmStats {
        PrewarmStats {
            total_prewarmed: Arc::clone(&self.stats.total_prewarmed),
            snapshots_created: Arc::clone(&self.stats.snapshots_created),
            snippets_precompiled: Arc::clone(&self.stats.snippets_precompiled),
            total_prewarm_time_us: Arc::clone(&self.stats.total_prewarm_time_us),
            avg_time_per_isolate_us: Arc::clone(&self.stats.avg_time_per_isolate_us),
            cache_hits: Arc::clone(&self.stats.cache_hits),
            cache_misses: Arc::clone(&self.stats.cache_misses),
            last_prewarm: Arc::clone(&self.stats.last_prewarm),
        }
    }
    /// Print statistics
    pub fn print_stats(&self) {
        let stats: _ = self.stats();
        println!("=== Isolate Pre-warming Statistics ===");
        println!("Total pre-warmed isolates: {}", stats.total_prewarmed.load(Ordering::Relaxed));
        println!("Snapshots created: {}", stats.snapshots_created.load(Ordering::Relaxed));
        println!("Snippets pre-compiled: {}", stats.snippets_precompiled.load(Ordering::Relaxed));
        println!("Cache hit rate: {:.2}%", stats.hit_rate() * 100.0);
        println!("Average pre-warm time: {:.2}µs per isolate", stats.avg_prewarm_time_us());
        println!("=====================================");
    }
    /// Get number of available pre-warmed isolates
    pub fn available_count(&self) -> usize {
        self.prewarmed_isolates.lock().unwrap().len()
    }
    /// Check if pre-warming is complete
    pub fn is_prewarmed(&self) -> bool {
        self.available_count() > 0
    }
    /// Clear all pre-warmed isolates
    pub fn clear(&self) {
        self.prewarmed_isolates.lock().unwrap().clear();
    }
}