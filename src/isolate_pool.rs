

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use rusty_v8 as v8;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::VecDeque;
use std::time::SystemTime;

/// V8 Isolate Pool - 高性能Isolate复用池（优化版）
/// 通过复用预创建的V8 Isolates来减少启动时间
/// 注意：V8 Isolate不是线程安全的，这个池只能在单线程中使用
pub struct IsolatePool {
    /// 可用的Isolate池
    available: Mutex<VecDeque<v8::OwnedIsolate>>,
    /// 正在使用的Isolate数量
    in_use: Mutex<usize>,
    /// 池的最大容量
    max_size: usize,
    /// 是否已初始化
    initialized: bool,
    /// 池统计信息
    stats: PoolStatistics,
}
/// 池统计信息（增强版）
#[derive(Debug, Default)]
pub struct PoolStatistics {
    /// 总获取次数
    pub total_acquires: Arc<AtomicUsize>,
    /// 总释放次数
    pub total_releases: Arc<AtomicUsize>,
    /// 缓存命中次数（从池中获取）
    pub cache_hits: Arc<AtomicUsize>,
    /// 缓存未命中次数（创建新Isolate）
    pub cache_misses: Arc<AtomicUsize>,
    /// 预热次数
    pub warmup_count: Arc<AtomicUsize>,
    /// 总创建时间（纳秒）
    pub total_creation_time_ns: Arc<AtomicUsize>,
    /// 最后预热时间
    pub last_warmup: Arc<AtomicUsize>,
}
impl PoolStatistics {
    pub fn new() -> Self {
        Self::default()
    }
    /// 获取缓存命中率
    #[allow(dead_code)]
    pub fn hit_rate(&self) -> f64 {
        let hits: _ = self.cache_hits.load(Ordering::Relaxed) as f64;
        let total: _ = hits + self.cache_misses.load(Ordering::Relaxed) as f64;
        if total > 0.0 { hits / total } else { 0.0 }
    }
    /// 获取平均创建时间（微秒）
    #[allow(dead_code)]
    pub fn avg_creation_time_us(&self) -> f64 {
        let total_time: _ = self.total_creation_time_ns.load(Ordering::Relaxed) as f64;
        let count: _ = self.cache_misses.load(Ordering::Relaxed) as f64;
        if count > 0.0 { total_time / count / 1000.0 } else { 0.0 }
    }
}
// 确保IsolatePool只在单线程中使用（V8 Isolate的线程限制）
unsafe impl Sync for IsolatePool {}
unsafe impl Send for IsolatePool {}
impl IsolatePool {
    /// 创建新的Isolate池（优化版）
    pub fn new(max_size: usize) -> Self {
        Self {
            available: Mutex::new(VecDeque::new()),
            in_use: Mutex::new(0),
            max_size,
            initialized: false,
            stats: PoolStatistics::new(),
        }
    }
    /// 预热池 - 预先创建指定数量的Isolates（优化版）
    pub fn pre_warm(&mut self, count: usize) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }
        let actual_count: _ = count.min(self.max_size);
        let mut pool = self.available.lock().map_err(|e| e.to_string())?;
        // 智能预热策略：根据池大小自适应
        let warmup_count: _ = if self.max_size >= 16 {
            (actual_count * 3 / 2).min(self.max_size)
        } else {
            actual_count
        };
        for i in 0..warmup_count {
            let isolate_start: _ = Instant::now();
            let isolate: _ = v8::Isolate::new(Default::default());
            let creation_time: _ = isolate_start.elapsed();
            pool.push_back(isolate);
            // 记录创建时间
            self.stats.total_creation_time_ns.fetch_add(
                creation_time.as_nanos() as usize,
                Ordering::Relaxed
            );
            // 渐进式延迟：根据池大小调整延迟
            if self.max_size >= 16 && i % 4 == 0 {
                thread::sleep(Duration::from_millis(1));
            }
        }
        self.initialized = true;
        self.stats.warmup_count.fetch_add(1, Ordering::Relaxed);
        self.stats.last_warmup.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as usize,
            Ordering::Relaxed
        );
        Ok(())
    }
    /// 获取一个Isolate（从池中借用）- 优化版
    pub fn acquire(&self) -> Option<v8::OwnedIsolate> {
        self.stats.total_acquires.fetch_add(1, Ordering::Relaxed);
        let mut pool = self.available.lock().unwrap();
        let mut in_use = self.in_use.lock().unwrap();
        // 尝试从池中获取
        if let Some(isolate) = pool.pop_front() {
            *in_use += 1;
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            // 智能预热：如果池快空了，在同一线程快速预创建
            if pool.len() < self.max_size / 4 && pool.len() + *in_use < self.max_size {
                let needed: _ = (self.max_size / 4).min(self.max_size - pool.len() - *in_use);
                // 快速创建一些Isolates以补充池
                for _ in 0..needed {
                    if pool.len() + *in_use >= self.max_size {
                        break;
                    }
                    let isolate: _ = v8::Isolate::new(Default::default());
                    pool.push_back(isolate);
                }
            }
            Some(isolate)
        } else {
            // 池为空，创建一个新的（不超过最大容量）
            let total_in_use: _ = *in_use;
            if total_in_use < self.max_size {
                *in_use += 1;
                self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
                Some(v8::Isolate::new(Default::default())
            } else {
                None
            }
        }
    }
    /// 归还一个Isolate到池中（优化版）
    pub fn release(&self, mut isolate: v8::OwnedIsolate) {
        self.stats.total_releases.fetch_add(1, Ordering::Relaxed);
        let mut pool = self.available.lock().unwrap();
        let mut in_use = self.in_use.lock().unwrap();
        // 重置Isolate状态以准备重用
        // 使用更安全的方法清理Isolate状态
        isolate.low_memory_notification();
        // 添加回池中（如果池未满）
        if pool.len() < self.max_size {
            pool.push_back(isolate);
        }
        *in_use = in_use.saturating_sub(1);
    }
    /// 获取池的统计信息（增强版）
    #[allow(dead_code)]
    pub fn stats(&self) -> PoolStats {
        let pool: _ = self.available.lock().unwrap();
        let in_use: _ = self.in_use.lock().unwrap();
        PoolStats {
            available: pool.len(),
            in_use: *in_use,
            max_size: self.max_size,
        }
    }
    /// 获取详细统计信息
    #[allow(dead_code)]
    pub fn detailed_stats(&self) -> PoolStatistics {
        PoolStatistics {
            total_acquires: Arc::clone(&self.stats.total_acquires),
            total_releases: Arc::clone(&self.stats.total_releases),
            cache_hits: Arc::clone(&self.stats.cache_hits),
            cache_misses: Arc::clone(&self.stats.cache_misses),
            warmup_count: Arc::clone(&self.stats.warmup_count),
            total_creation_time_ns: Arc::clone(&self.stats.total_creation_time_ns),
            last_warmup: Arc::clone(&self.stats.last_warmup),
        }
    }
    /// 清理池中未使用的Isolates
    #[allow(dead_code)]
    pub fn shrink_to_fit(&self) -> usize {
        let mut pool = self.available.lock().unwrap();
        let before_len: _ = pool.len();
        // 保留至少一个Isolate以避免完全耗尽
        while pool.len() > 1 {
            pool.pop_back();
        }
        before_len - pool.len()
    }
}
/// 池统计信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PoolStats {
    pub available: usize,
    pub in_use: usize,
    pub max_size: usize,
}
impl PoolStats {
    /// 检查池是否接近满载
    #[allow(dead_code)]
    pub fn is_near_full(&self) -> bool {
        self.in_use as f64 / self.max_size as f64 > 0.8
    }
    /// 获取利用率百分比
    #[allow(dead_code)]
    pub fn utilization_percent(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.in_use as f64 / self.max_size as f64) * 100.0
        }
    }
}
/// 全局Isolate池实例
/// 在测试环境中禁用全局池，避免生命周期问题
static POOL: once_cell::sync::OnceCell<Box<IsolatePool>> = once_cell::sync::OnceCell::new();
/// 检测是否在测试环境中运行
/// 集成测试不会设置 cfg(test)，所以需要额外检测
fn is_test_environment() -> bool {
    // 1. 编译时检测 - 仅在 cargo test 时为 true
    #[cfg(test)]
    return true;
    // 2. 环境变量检测 - 仅在明确设置时为 true
    #[allow(unreachable_code)]
    let env_check: _ = std::env::var("AMBER_TEST_MODE").is_ok();
    env_check
    // 注意：不再检查二进制路径，因为 target/release/amber 会被误判为测试环境
    // 这会导致生产构建的 amberjs 无法使用 Isolate 池，严重影响性能
}
/// 初始化全局Isolate池（超级激进版）
#[allow(dead_code)]
pub fn initialize_pool(max_size: usize) -> Result<(), String> {
    // 在测试环境中不初始化全局池
    if is_test_environment() {
        return Ok(());
    }
    let mut pool = IsolatePool::new(max_size);
    // 超级激进的预热策略：预创建75%容量的Isolates
    let warmup_count: _ = (max_size * 3 / 4).max(4);
    pool.pre_warm(warmup_count)?;
    let pool_box: _ = Box::new(pool);
    POOL.set(pool_box)
        .map_err(|_| "Pool already initialized".to_string())
}
/// 获取全局Isolate池
#[allow(dead_code)]
pub fn get_pool() -> Option<&'static IsolatePool> {
    // 在测试环境中返回 None
    if is_test_environment() {
        return None;
    }
    POOL.get().map(|p| p.as_ref())
}
/// 测试专用的 V8 Isolate 管理器（简化版本）
/// 在测试环境中禁用全局 Isolate 池，每个 Runtime 使用独立的 Isolate
/// 关键原则：确保 Isolate 在创建它的线程上被销毁
/// 测试环境安全的 Isolate 获取（简化版本）
/// 返回一个新创建的 Isolate，不进行复用
#[allow(dead_code)]
pub fn get_test_isolate() -> Option<v8::OwnedIsolate> {
    #[cfg(not(test))]
    return None;
    #[cfg(test)]
    crate::is_v8_available().then(|| v8::Isolate::new(Default::default())
}
/// 测试环境安全的 Isolate 归还（简化版本）
/// 直接丢弃 Isolate，确保在正确的线程上触发 Drop
#[allow(dead_code)]
pub fn return_test_isolate(isolate: v8::OwnedIsolate) {
    // 直接丢弃 Isolate，它会在当前线程的当前作用域结束时被正确销毁
    drop(isolate);
}
/// 测试环境安全检查：清理测试 Isolate（空实现）
/// 不需要全局清理，因为每个 Isolate 都有自己的生命周期
#[allow(dead_code)]
pub fn cleanup_test_isolate() {
    // 空实现：不需要全局清理
}
/// 从池中获取Isolate
pub fn acquire_isolate() -> Option<v8::OwnedIsolate> {
    // 在测试环境中不使用池
    if is_test_environment() {
        return None;
    }
    POOL.get().and_then(|pool| pool.acquire())
}
/// 将Isolate归还给池
#[allow(dead_code)]
pub fn release_isolate(isolate: v8::OwnedIsolate) {
    // 在测试环境中不归还，直接丢弃
    if is_test_environment() {
        drop(isolate);
        return;
    }
    if let Some(pool) = POOL.get() {
        pool.release(isolate);
    }
}
#[cfg(test)]
mod tests {
    // 初始化V8以供测试使用
    fn init_v8_for_tests() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            let platform: _ = v8::new_default_platform().unwrap();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
    }
    #[test]
    fn test_isolate_pool_creation() {
        init_v8_for_tests();
        let pool: _ = IsolatePool::new(4);
        let stats: _ = pool.stats();
        assert_eq!(stats.available, 0);
        assert_eq!(stats.in_use, 0);
        assert_eq!(stats.max_size, 4);
    }
    #[test]
    #[ignore]
    fn test_isolate_pool_pre_warm() {
        init_v8_for_tests();
        let mut pool = IsolatePool::new(4);
        pool.pre_warm(2).unwrap();
        let stats: _ = pool.stats();
        assert_eq!(stats.available, 2);
        assert_eq!(stats.in_use, 0);
    }
    #[test]
    #[ignore = "V8 Isolate生命周期限制 - 需要在创建线程销毁"]
    fn test_isolate_acquire_release() {
        init_v8_for_tests();
        let mut pool = IsolatePool::new(4);
        pool.pre_warm(2).unwrap();
        // 获取一个Isolate
        let isolate: _ = pool.acquire().unwrap();
        let stats: _ = pool.stats();
        assert_eq!(stats.available, 1);
        assert_eq!(stats.in_use, 1);
        // 归还Isolate
        pool.release(isolate);
        let stats: _ = pool.stats();
        assert_eq!(stats.available, 2);
        assert_eq!(stats.in_use, 0);
    }
    #[test]
    #[ignore]
    fn test_pool_stats() {
        init_v8_for_tests();
        let mut pool = IsolatePool::new(10);
        pool.pre_warm(5).unwrap();
        let stats: _ = pool.stats();
        assert_eq!(stats.utilization_percent(), 0.0);
        assert!(!stats.is_near_full());
        // 使用8个Isolate (80%利用率)
        let _isolate1: _ = pool.acquire().unwrap();
        let _isolate2: _ = pool.acquire().unwrap();
        let _isolate3: _ = pool.acquire().unwrap();
        let _isolate4: _ = pool.acquire().unwrap();
        let _isolate5: _ = pool.acquire().unwrap();
        let _isolate6: _ = pool.acquire().unwrap();
        let _isolate7: _ = pool.acquire().unwrap();
        let _isolate8: _ = pool.acquire().unwrap();
        let stats: _ = pool.stats();
        assert_eq!(stats.utilization_percent(), 80.0);
        assert!(stats.is_near_full());
    }
}
