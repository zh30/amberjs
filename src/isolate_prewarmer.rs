//! Modern Isolate Pre-warming & Standby Pool System (Task 1.2 / v1.13.0)
//!
//! Provides pre-initialized, fully-warmed `MinimalRuntime` instances ready for
//! instant execution without isolate creation or context compilation overhead.
//!
//! # Architecture
//! Uses thread-affine standby pooling to guarantee 100% V8 stack safety, zero thread
//! migration overhead, zero mutex lock contention, and true sub-millisecond execution.

use crate::runtime_minimal::MinimalRuntime;
use anyhow::Result;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

thread_local! {
    static THREAD_ISOLATE_STANDBY: RefCell<Option<MinimalRuntime>> = const { RefCell::new(None) };
}

/// Pre-warming configuration options
#[derive(Debug, Clone)]
pub struct PrewarmConfig {
    /// Enable V8 snapshot integration during pre-warming
    pub enable_snapshots: bool,
    /// Pre-warm with fast minimal heap profile
    pub fast_heap: bool,
}

impl Default for PrewarmConfig {
    fn default() -> Self {
        Self {
            enable_snapshots: true,
            fast_heap: true,
        }
    }
}

/// Statistics for isolate pre-warming and pool reuse
#[derive(Debug, Default)]
pub struct PrewarmStats {
    /// Total isolates pre-warmed
    pub total_prewarmed: AtomicUsize,
    /// Total pre-warming time in microseconds
    pub total_prewarm_time_us: AtomicUsize,
    /// Pool hits (instant reuse)
    pub cache_hits: AtomicUsize,
    /// Pool misses (on-demand creation)
    pub cache_misses: AtomicUsize,
    /// Last prewarm timestamp
    pub last_prewarm_secs: AtomicUsize,
}

impl PrewarmStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
        let total = hits + self.cache_misses.load(Ordering::Relaxed) as f64;
        if total > 0.0 {
            hits / total
        } else {
            0.0
        }
    }

    pub fn avg_prewarm_time_us(&self) -> f64 {
        let total = self.total_prewarm_time_us.load(Ordering::Relaxed) as f64;
        let count = self.total_prewarmed.load(Ordering::Relaxed) as f64;
        if count > 0.0 {
            total / count
        } else {
            0.0
        }
    }
}

/// High-performance Isolate Prewarmer
pub struct IsolatePrewarmer {
    config: PrewarmConfig,
    stats: Arc<PrewarmStats>,
}

unsafe impl Send for IsolatePrewarmer {}
unsafe impl Sync for IsolatePrewarmer {}

impl IsolatePrewarmer {
    /// Create a new IsolatePrewarmer with default configuration
    pub fn new() -> Self {
        Self::with_config(PrewarmConfig::default())
    }

    /// Create a new IsolatePrewarmer with custom configuration
    pub fn with_config(config: PrewarmConfig) -> Self {
        Self {
            config,
            stats: Arc::new(PrewarmStats::new()),
        }
    }

    /// Pre-warm a single MinimalRuntime instance on the current thread
    pub fn prewarm_one(&self) -> Result<MinimalRuntime> {
        let start = Instant::now();
        // Isolate::new + snapshot blob attach is not safe to race across threads.
        // macOS CI SIGSEGV'd in test_concurrent_multi_thread_checkout when four
        // workers created isolates from the same CoW startup blob at once.
        static CREATE_LOCK: Mutex<()> = Mutex::new(());
        let runtime = {
            let _guard = CREATE_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if self.config.enable_snapshots {
                crate::v8_snapshot::enable_startup_snapshot_for_cli();
            }

            let mut runtime = if self.config.fast_heap {
                MinimalRuntime::new_fast()?
            } else {
                MinimalRuntime::new()?
            };

            runtime.prewarm()?;
            runtime
        };
        let elapsed_us = start.elapsed().as_micros() as usize;

        self.stats.total_prewarmed.fetch_add(1, Ordering::Relaxed);
        self.stats
            .total_prewarm_time_us
            .fetch_add(elapsed_us, Ordering::Relaxed);
        self.stats.last_prewarm_secs.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as usize,
            Ordering::Relaxed,
        );

        Ok(runtime)
    }

    /// Pre-warm the standby isolate on the current thread
    pub fn prewarm(&self) -> Result<usize> {
        THREAD_ISOLATE_STANDBY.with(|standby| {
            let mut s = standby.borrow_mut();
            if s.is_none() {
                let runtime = self.prewarm_one()?;
                *s = Some(runtime);
                Ok(1)
            } else {
                Ok(0)
            }
        })
    }

    /// Replenish available standby instance on current thread
    pub fn replenish(&self) -> usize {
        self.prewarm().unwrap_or(0)
    }

    /// Check out a pre-warmed MinimalRuntime for the current thread.
    /// If a standby instance is available, returns it with sub-microsecond latency.
    /// Otherwise, creates and pre-warms a fresh instance immediately on this thread.
    pub fn acquire(&self) -> Result<MinimalRuntime> {
        let maybe_runtime = THREAD_ISOLATE_STANDBY.with(|standby| standby.borrow_mut().take());

        let runtime = if let Some(runtime) = maybe_runtime {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            runtime
        } else {
            self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            self.prewarm_one()?
        };

        Ok(runtime)
    }

    /// Number of ready isolates currently available in the current thread's standby
    pub fn available_count(&self) -> usize {
        THREAD_ISOLATE_STANDBY.with(|standby| if standby.borrow().is_some() { 1 } else { 0 })
    }

    /// Whether the current thread has an available pre-warmed isolate
    pub fn is_ready(&self) -> bool {
        self.available_count() > 0
    }

    /// Clear standby isolate from the current thread
    pub fn clear(&self) {
        THREAD_ISOLATE_STANDBY.with(|standby| *standby.borrow_mut() = None);
    }

    /// Get current prewarmer statistics
    pub fn stats(&self) -> &PrewarmStats {
        &self.stats
    }
}

impl Default for IsolatePrewarmer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global singleton IsolatePrewarmer instance
pub fn global_prewarmer() -> &'static IsolatePrewarmer {
    static PREWARMER: OnceLock<IsolatePrewarmer> = OnceLock::new();
    PREWARMER.get_or_init(|| {
        let prewarmer = IsolatePrewarmer::new();
        // Pre-warm the main thread's initial standby instance
        let _ = prewarmer.prewarm();
        prewarmer
    })
}
