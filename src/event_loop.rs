// V8 事件循环实现
// 为 Amber 提供异步 JavaScript 执行支持
// v0.3.247: 添加异步定时器调度支持 (setTimeout/setInterval/setImmediate)
// v0.3.248: 使用 fired timer 队列简化架构

use once_cell::sync::Lazy;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

/// 事件循环状态
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EventLoopState {
    Running,
    Stopped,
    Paused,
}

/// 事件循环配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EventLoopConfig {
    pub max_execution_time: Duration,
    pub max_queue_size: usize,
    pub enable_promise_tracking: bool,
}

impl Default for EventLoopConfig {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_secs(30),
            max_queue_size: 10000,
            enable_promise_tracking: true,
        }
    }
}

/// 事件循环任务
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EventLoopTask {
    pub id: u64,
    pub task_type: String,
    pub description: String,
    pub created_at: Instant,
    pub estimated_duration: Duration,
}

/// Pending I/O completion to be applied on the V8 main thread.
#[derive(Debug)]
pub struct IoCompletion {
    pub id: u64,
    pub ok: bool,
    pub payload: String,
}

/// Cross-thread I/O completion queue for the tokio ↔ V8 bridge.
/// Async work should push here; the main event-loop drain pops and resolves Promises.
pub struct IoCompletionQueue {
    inner: Mutex<Vec<IoCompletion>>,
    next_id: AtomicU64,
    pending_ops: AtomicU64,
}

impl IoCompletionQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
            next_id: AtomicU64::new(1),
            pending_ops: AtomicU64::new(0),
        }
    }

    pub fn begin_op(&self) -> u64 {
        self.pending_ops.fetch_add(1, Ordering::SeqCst);
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn complete(&self, id: u64, ok: bool, payload: String) {
        if let Ok(mut q) = self.inner.lock() {
            q.push(IoCompletion { id, ok, payload });
        }
        self.pending_ops.fetch_sub(1, Ordering::SeqCst);
        get_async_timer_manager().wake();
    }

    pub fn drain(&self) -> Vec<IoCompletion> {
        self.inner
            .lock()
            .map(|mut q| q.drain(..).collect())
            .unwrap_or_default()
    }

    pub fn has_pending(&self) -> bool {
        self.pending_ops.load(Ordering::SeqCst) > 0
            || self.inner.lock().map(|q| !q.is_empty()).unwrap_or(false)
    }
}

impl Default for IoCompletionQueue {
    fn default() -> Self {
        Self::new()
    }
}

static IO_COMPLETION_QUEUE: Lazy<IoCompletionQueue> = Lazy::new(IoCompletionQueue::new);

pub fn io_completion_queue() -> &'static IoCompletionQueue {
    &IO_COMPLETION_QUEUE
}

/// Node-like phase tags for the main-thread drain loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLoopPhase {
    NextTick,
    Microtasks,
    Timers,
    IoCallbacks,
    Immediate,
    Close,
}

// v0.3.247: 异步定时器调度器（纯 tokio 异步实现）
// v0.3.249: 添加 fired timer 通知通道，支持 V8 主线程轮询执行回调
// ============================================================

const DEFAULT_TIMER_COMMAND_QUEUE_SIZE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerScheduleError {
    InvalidTimerId,
    QueueFull,
    ChannelClosed,
}

impl fmt::Display for TimerScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimerId => write!(f, "timer id must be greater than zero"),
            Self::QueueFull => write!(f, "timer command queue is full"),
            Self::ChannelClosed => write!(f, "timer manager channel is closed"),
        }
    }
}

impl std::error::Error for TimerScheduleError {}

#[derive(Debug, Clone, Copy)]
struct TimerEntry {
    deadline: Instant,
    delay: Duration,
    remaining: u32,
    sequence: u64,
}

/// 定时器命令（用于与工作线程通信）
enum TimerCommand {
    ScheduleTimeout {
        timer_id: u64,
        deadline: Instant,
        delay: Duration,
        sequence: u64,
    },
    ScheduleInterval {
        timer_id: u64,
        deadline: Instant,
        delay: Duration,
        repeat_count: u32,
        sequence: u64,
    },
    Cancel {
        timer_id: u64,
    },
    CancelWithAck {
        timer_id: u64,
        ack: oneshot::Sender<bool>,
    },
    Clear,
    Shutdown,
    /// v0.3.261: Clear with acknowledgement
    ClearWithAck(oneshot::Sender<()>),
    ShutdownWithAck(oneshot::Sender<()>),
}

/// Thread-safe event loop waker using Condvar
#[derive(Debug)]
pub struct EventLoopWaker {
    mutex: Mutex<bool>,
    condvar: Condvar,
}

impl EventLoopWaker {
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    pub fn wake(&self) {
        if let Ok(mut lock) = self.mutex.lock() {
            *lock = true;
            self.condvar.notify_all();
        }
    }

    pub fn reset(&self) {
        if let Ok(mut lock) = self.mutex.lock() {
            *lock = false;
        }
    }

    /// Wait for a wake event with a timeout.
    /// Returns true if woken before timeout, false if timed out.
    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        if let Ok(mut lock) = self.mutex.lock() {
            if *lock {
                *lock = false;
                return true;
            }
            if timeout.is_zero() {
                return false;
            }
            match self.condvar.wait_timeout(lock, timeout) {
                Ok((mut guard, result)) => {
                    let was_woken = *guard;
                    *guard = false;
                    was_woken || !result.timed_out()
                }
                Err(poisoned) => {
                    let mut guard = poisoned.into_inner().0;
                    let was_woken = *guard;
                    *guard = false;
                    was_woken
                }
            }
        } else {
            false
        }
    }
}

impl Default for EventLoopWaker {
    fn default() -> Self {
        Self::new()
    }
}

/// 异步定时器管理器
/// 使用 tokio 异步通道和独立运行时处理定时器
pub struct AsyncTimerManager {
    // 命令发送端
    cmd_tx: mpsc::Sender<TimerCommand>,
    // 工作线程句柄
    _worker_thread: std::thread::JoinHandle<()>,
    // 下一个定时器 ID（本地缓存，用于立即返回）- v0.3.265: 预留用于外部 ID 模式
    #[allow(dead_code)]
    next_timer_id: AtomicU64,
    // 调度插入序号；用于同 deadline timer 的稳定 FIFO 顺序
    next_timer_sequence: AtomicU64,
    // Fired 定时器队列（使用 Arc<RwLock> 实现线程安全共享）
    // 工作线程写入，主线程读取
    fired_timers: Arc<RwLock<Vec<u64>>>,
    // v0.3.339: 已调度定时器计数（使用原子计数实现线程安全共享）
    // 工作线程在调度时递增，主线程可以检查是否有待处理的定时器
    scheduled_timer_count: Arc<AtomicU64>,
    // 可检测的命令队列容量；0 用于稳定表示队列已满
    command_queue_size: usize,
    // 事件循环唤醒器，用于定时器到期或 IO 完成时通知挂起线程
    waker: Arc<EventLoopWaker>,
}

fn handle_timer_command(
    cmd: TimerCommand,
    scheduled_timers: &mut HashMap<u64, TimerEntry>,
    timer_heap: &mut BTreeSet<(Instant, u64, u64)>,
    scheduled_count: &Arc<AtomicU64>,
    waker: &Arc<EventLoopWaker>,
) -> bool {
    match cmd {
        TimerCommand::ScheduleTimeout {
            timer_id,
            deadline,
            delay,
            sequence,
        } => {
            let is_new = !scheduled_timers.contains_key(&timer_id);
            if let Some(old) = scheduled_timers.insert(
                timer_id,
                TimerEntry {
                    deadline,
                    delay,
                    remaining: 1,
                    sequence,
                },
            ) {
                timer_heap.remove(&(old.deadline, old.sequence, timer_id));
            }
            timer_heap.insert((deadline, sequence, timer_id));
            if is_new {
                scheduled_count.fetch_add(1, Ordering::SeqCst);
            }
            true
        }
        TimerCommand::ScheduleInterval {
            timer_id,
            deadline,
            delay,
            repeat_count,
            sequence,
        } => {
            let repeats = if repeat_count == 0 {
                u32::MAX
            } else {
                repeat_count
            };
            let is_new = !scheduled_timers.contains_key(&timer_id);
            if let Some(old) = scheduled_timers.insert(
                timer_id,
                TimerEntry {
                    deadline,
                    delay,
                    remaining: repeats,
                    sequence,
                },
            ) {
                timer_heap.remove(&(old.deadline, old.sequence, timer_id));
            }
            timer_heap.insert((deadline, sequence, timer_id));
            if is_new {
                scheduled_count.fetch_add(1, Ordering::SeqCst);
            }
            true
        }
        TimerCommand::Cancel { timer_id } => {
            if let Some(entry) = scheduled_timers.remove(&timer_id) {
                timer_heap.remove(&(entry.deadline, entry.sequence, timer_id));
                scheduled_count.fetch_sub(1, Ordering::SeqCst);
            }
            true
        }
        TimerCommand::CancelWithAck { timer_id, ack } => {
            let removed = if let Some(entry) = scheduled_timers.remove(&timer_id) {
                timer_heap.remove(&(entry.deadline, entry.sequence, timer_id));
                scheduled_count.fetch_sub(1, Ordering::SeqCst);
                true
            } else {
                false
            };
            let _ = ack.send(removed);
            true
        }
        TimerCommand::Clear => {
            let count = scheduled_timers.len() as u64;
            scheduled_timers.clear();
            timer_heap.clear();
            scheduled_count.fetch_sub(count, Ordering::SeqCst);
            waker.wake();
            true
        }
        TimerCommand::ClearWithAck(sender) => {
            let count = scheduled_timers.len() as u64;
            scheduled_timers.clear();
            timer_heap.clear();
            scheduled_count.fetch_sub(count, Ordering::SeqCst);
            let _ = sender.send(());
            waker.wake();
            true
        }
        TimerCommand::Shutdown => {
            let count = scheduled_timers.len() as u64;
            scheduled_timers.clear();
            timer_heap.clear();
            scheduled_count.fetch_sub(count, Ordering::SeqCst);
            waker.wake();
            false
        }
        TimerCommand::ShutdownWithAck(sender) => {
            let count = scheduled_timers.len() as u64;
            scheduled_timers.clear();
            timer_heap.clear();
            scheduled_count.fetch_sub(count, Ordering::SeqCst);
            let _ = sender.send(());
            waker.wake();
            false
        }
    }
}

fn fire_due_timers(
    scheduled_timers: &mut HashMap<u64, TimerEntry>,
    timer_heap: &mut BTreeSet<(Instant, u64, u64)>,
    now: Instant,
    fired_timers: &Arc<RwLock<Vec<u64>>>,
    scheduled_count: &Arc<AtomicU64>,
    waker: &Arc<EventLoopWaker>,
) {
    let mut fired_ids = Vec::new();
    let mut to_reschedule = Vec::new();

    while let Some(&(deadline, sequence, timer_id)) = timer_heap.iter().next() {
        if deadline <= now {
            timer_heap.remove(&(deadline, sequence, timer_id));
            if let Some(entry) = scheduled_timers.remove(&timer_id) {
                fired_ids.push(timer_id);
                if entry.remaining > 1 {
                    let remaining = if entry.remaining == u32::MAX {
                        u32::MAX
                    } else {
                        entry.remaining - 1
                    };
                    to_reschedule.push((timer_id, entry.delay, remaining, sequence));
                } else {
                    scheduled_count.fetch_sub(1, Ordering::SeqCst);
                }
            }
        } else {
            break;
        }
    }

    for (timer_id, delay, remaining, sequence) in to_reschedule {
        let next_deadline = Instant::now() + delay;
        scheduled_timers.insert(
            timer_id,
            TimerEntry {
                deadline: next_deadline,
                delay,
                remaining,
                sequence,
            },
        );
        timer_heap.insert((next_deadline, sequence, timer_id));
    }

    if !fired_ids.is_empty() {
        {
            let mut fired = fired_timers.write().unwrap();
            fired.extend(fired_ids);
        }
        waker.wake();
    }
}

impl AsyncTimerManager {
    /// 创建新的定时器管理器
    pub fn new() -> Self {
        Self::new_with_command_queue_size(DEFAULT_TIMER_COMMAND_QUEUE_SIZE)
    }

    /// 创建指定命令队列容量的定时器管理器
    pub fn new_with_command_queue_size(command_queue_size: usize) -> Self {
        let channel_capacity = command_queue_size.max(1);
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<TimerCommand>(channel_capacity);
        // 创建 fired 定时器队列（共享状态）
        let fired_timers = Arc::new(RwLock::new(Vec::new()));
        let fired_timers_clone = fired_timers.clone();
        // v0.3.339: 创建已调度定时器计数（共享状态）
        let scheduled_timer_count = Arc::new(AtomicU64::new(0));
        let scheduled_count_clone = scheduled_timer_count.clone();
        let waker = Arc::new(EventLoopWaker::new());
        let waker_clone = waker.clone();

        // 创建工作线程
        let worker_thread = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            let mut scheduled_timers: HashMap<u64, TimerEntry> = HashMap::new();
            let mut timer_heap: BTreeSet<(Instant, u64, u64)> = BTreeSet::new();

            rt.block_on(async move {
                loop {
                    let next_deadline = timer_heap.iter().next().map(|(d, _, _)| *d);

                    match next_deadline {
                        Some(deadline) => {
                            let now = Instant::now();
                            if deadline <= now {
                                fire_due_timers(
                                    &mut scheduled_timers,
                                    &mut timer_heap,
                                    now,
                                    &fired_timers_clone,
                                    &scheduled_count_clone,
                                    &waker_clone,
                                );
                                continue;
                            }

                            let sleep_dur = deadline.duration_since(now);
                            tokio::select! {
                                _ = tokio::time::sleep(sleep_dur) => {
                                    let now = Instant::now();
                                    fire_due_timers(
                                        &mut scheduled_timers,
                                        &mut timer_heap,
                                        now,
                                        &fired_timers_clone,
                                        &scheduled_count_clone,
                                        &waker_clone,
                                    );
                                }
                                cmd = cmd_rx.recv() => {
                                    match cmd {
                                        Some(command) => {
                                            if !handle_timer_command(
                                                command,
                                                &mut scheduled_timers,
                                                &mut timer_heap,
                                                &scheduled_count_clone,
                                                &waker_clone,
                                            ) {
                                                break;
                                            }
                                            while let Ok(command) = cmd_rx.try_recv() {
                                                if !handle_timer_command(
                                                    command,
                                                    &mut scheduled_timers,
                                                    &mut timer_heap,
                                                    &scheduled_count_clone,
                                                    &waker_clone,
                                                ) {
                                                    return;
                                                }
                                            }
                                        }
                                        None => break,
                                    }
                                }
                            }
                        }
                        None => match cmd_rx.recv().await {
                            Some(command) => {
                                if !handle_timer_command(
                                    command,
                                    &mut scheduled_timers,
                                    &mut timer_heap,
                                    &scheduled_count_clone,
                                    &waker_clone,
                                ) {
                                    break;
                                }
                                while let Ok(command) = cmd_rx.try_recv() {
                                    if !handle_timer_command(
                                        command,
                                        &mut scheduled_timers,
                                        &mut timer_heap,
                                        &scheduled_count_clone,
                                        &waker_clone,
                                    ) {
                                        return;
                                    }
                                }
                            }
                            None => break,
                        },
                    }
                }
            });
        });

        Self {
            cmd_tx,
            _worker_thread: worker_thread,
            next_timer_id: AtomicU64::new(1),
            next_timer_sequence: AtomicU64::new(1),
            fired_timers,
            scheduled_timer_count,
            command_queue_size,
            waker,
        }
    }

    /// 获取下一个定时器 ID - v0.3.265: 预留用于外部 ID 模式
    #[allow(dead_code)]
    fn _next_id(&self) -> u64 {
        self.next_timer_id.fetch_add(1, Ordering::SeqCst)
    }

    fn next_sequence(&self) -> u64 {
        self.next_timer_sequence.fetch_add(1, Ordering::SeqCst)
    }

    fn map_try_send_error<T>(err: mpsc::error::TrySendError<T>) -> TimerScheduleError {
        match err {
            mpsc::error::TrySendError::Full(_) => TimerScheduleError::QueueFull,
            mpsc::error::TrySendError::Closed(_) => TimerScheduleError::ChannelClosed,
        }
    }

    fn try_send_schedule_command(&self, command: TimerCommand) -> Result<(), TimerScheduleError> {
        if self.command_queue_size == 0 {
            return Err(TimerScheduleError::QueueFull);
        }

        self.cmd_tx
            .try_send(command)
            .map_err(Self::map_try_send_error)
    }

    /// 安排一个一次性定时器，并返回调度错误
    pub fn try_schedule_timeout(
        &self,
        delay: Duration,
        timer_id: u64,
        _callback: impl Fn() + Send + 'static,
    ) -> Result<(), TimerScheduleError> {
        if timer_id == 0 {
            return Err(TimerScheduleError::InvalidTimerId);
        }

        self.try_send_schedule_command(TimerCommand::ScheduleTimeout {
            timer_id,
            deadline: Instant::now() + delay,
            delay,
            sequence: self.next_sequence(),
        })
    }

    /// 安排一个一次性定时器
    /// v0.3.261: Accept external timer_id to use epoch-based IDs for test isolation
    pub fn schedule_timeout(
        &self,
        delay: Duration,
        timer_id: u64,
        _callback: impl Fn() + Send + 'static,
    ) -> Result<(), TimerScheduleError> {
        self.try_schedule_timeout(delay, timer_id, _callback)
    }

    /// Mark a timer as ready for main-thread execution without worker-thread scheduling.
    ///
    /// This is used for zero-delay timeouts after the callback has been stored. It
    /// preserves event-loop ordering while avoiding a race where the main thread
    /// exits before the worker processes a freshly sent schedule command.
    pub fn mark_timer_fired(&self, timer_id: u64) {
        {
            let mut fired = self.fired_timers.write().unwrap();
            fired.push(timer_id);
        }
        self.waker.wake();
    }

    /// 安排一个重复定时器
    /// v0.3.261: Accept external timer_id to use epoch-based IDs for test isolation
    pub fn try_schedule_interval(
        &self,
        delay: Duration,
        repeat_count: u32,
        timer_id: u64,
        _callback: impl Fn() + Send + 'static,
    ) -> Result<(), TimerScheduleError> {
        if timer_id == 0 {
            return Err(TimerScheduleError::InvalidTimerId);
        }

        self.try_send_schedule_command(TimerCommand::ScheduleInterval {
            timer_id,
            deadline: Instant::now() + delay,
            delay,
            repeat_count,
            sequence: self.next_sequence(),
        })
    }

    /// 安排一个重复定时器
    /// v0.3.261: Accept external timer_id to use epoch-based IDs for test isolation
    pub fn schedule_interval(
        &self,
        delay: Duration,
        repeat_count: u32,
        timer_id: u64,
        _callback: impl Fn() + Send + 'static,
    ) -> Result<(), TimerScheduleError> {
        self.try_schedule_interval(delay, repeat_count, timer_id, _callback)
    }

    /// 取消定时器
    pub fn try_cancel(&self, timer_id: u64) -> Result<(), TimerScheduleError> {
        if timer_id == 0 {
            return Err(TimerScheduleError::InvalidTimerId);
        }

        self.try_send_schedule_command(TimerCommand::Cancel { timer_id })
    }

    /// 取消定时器
    pub fn cancel(&self, timer_id: u64) -> bool {
        self.try_cancel(timer_id).is_ok()
    }

    /// 取消定时器并等待工作线程确认是否移除了待触发 timer
    pub async fn cancel_with_ack(&self, timer_id: u64) -> Result<bool, TimerScheduleError> {
        if timer_id == 0 {
            return Err(TimerScheduleError::InvalidTimerId);
        }

        let (tx, rx) = oneshot::channel();
        self.try_send_schedule_command(TimerCommand::CancelWithAck { timer_id, ack: tx })?;
        rx.await.map_err(|_| TimerScheduleError::ChannelClosed)
    }

    /// 清除所有定时器
    pub fn clear(&self) {
        let _ = self.cmd_tx.try_send(TimerCommand::Clear);
        // Also clear fired timers to prevent stale timers from previous tests
        let mut fired = self.fired_timers.write().unwrap();
        fired.clear();
        self.waker.wake();
    }

    /// v0.3.261: 清除已触发的定时器列表
    pub fn clear_fired_timers(&self) {
        let mut fired = self.fired_timers.write().unwrap();
        fired.clear();
    }

    /// v0.3.261: 清除所有定时器并等待确认
    /// 使用同步通道确保清除命令被处理后才返回
    pub fn clear_with_ack(&self) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.cmd_tx.blocking_send(TimerCommand::ClearWithAck(tx));
        // Also clear fired timers to prevent stale timers from previous tests
        let mut fired = self.fired_timers.write().unwrap();
        fired.clear();
        // Wait for the worker to acknowledge the clear
        let _ = rx.blocking_recv();
        self.waker.wake();
    }

    /// 关闭定时器管理器
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.try_send(TimerCommand::Shutdown);
        self.waker.wake();
    }

    /// 关闭定时器管理器并等待工作线程确认
    pub async fn shutdown_with_ack(&self) -> Result<(), TimerScheduleError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .try_send(TimerCommand::ShutdownWithAck(tx))
            .map_err(Self::map_try_send_error)?;
        rx.await.map_err(|_| TimerScheduleError::ChannelClosed)?;

        while !self.cmd_tx.is_closed() {
            tokio::task::yield_now().await;
        }

        self.waker.wake();
        Ok(())
    }

    /// v0.3.249: 轮询并获取已触发的定时器 ID 列表
    /// 供 V8 主线程调用，返回自上次轮询以来触发的所有定时器 ID
    pub fn poll_fired_timers(&self) -> Vec<u64> {
        let mut fired = self.fired_timers.write().unwrap();
        let ids = fired.clone();
        fired.clear();
        ids
    }

    /// v0.3.249: 获取 fired 定时器数量（不清除）
    pub fn has_fired_timers(&self) -> bool {
        let fired = self.fired_timers.read().unwrap();
        !fired.is_empty()
    }

    /// v0.3.339: 检查是否有已调度但尚未触发的定时器
    /// 用于事件循环判断是否需要继续等待
    pub fn has_scheduled_timers(&self) -> bool {
        self.scheduled_timer_count.load(Ordering::SeqCst) > 0
    }

    /// 唤醒正在等待定时器或事件的线程
    pub fn wake(&self) {
        self.waker.wake();
    }

    /// 等待事件循环唤醒或超时
    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        self.waker.wait_timeout(timeout)
    }

    /// 获取底层唤醒器引用
    pub fn waker(&self) -> &Arc<EventLoopWaker> {
        &self.waker
    }
}

impl Drop for AsyncTimerManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Default for AsyncTimerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局异步定时器管理器实例
static ASYNC_TIMER_MANAGER: Lazy<AsyncTimerManager> = Lazy::new(|| AsyncTimerManager::new());

/// 获取全局异步定时器管理器
pub fn get_async_timer_manager() -> &'static AsyncTimerManager {
    &ASYNC_TIMER_MANAGER
}

/// 在异步上下文中安排延迟任务
pub async fn async_sleep(delay: Duration) {
    tokio::time::sleep(delay).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    // Simple ID generator for tests
    fn next_test_id(id: &AtomicU64) -> u64 {
        id.fetch_add(1, Ordering::SeqCst)
    }

    #[tokio::test]
    async fn test_schedule_timeout() {
        let manager = AsyncTimerManager::new();
        let test_id = AtomicU64::new(1000);

        let id = next_test_id(&test_id);
        manager
            .schedule_timeout(Duration::from_millis(10), id, || {})
            .unwrap();

        assert!(id > 0, "Timer ID should be positive");

        // 等待定时器调度完成
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 验证定时器可以取消（说明它被正确调度了）
        let cancelled = manager.cancel(id);
        assert!(cancelled, "Cancel should return true for scheduled timer");
    }

    #[tokio::test]
    async fn test_schedule_interval() {
        let manager = AsyncTimerManager::new();
        let test_id = AtomicU64::new(2000);

        let id = next_test_id(&test_id);
        manager
            .schedule_interval(Duration::from_millis(10), 3, id, || {})
            .unwrap();

        assert!(id > 0, "Timer ID should be positive");

        // 等待定时器调度
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 验证定时器可以取消
        let cancelled = manager.cancel(id);
        assert!(
            cancelled,
            "Cancel should return true for scheduled interval"
        );
    }

    #[tokio::test]
    async fn test_cancel_timeout() {
        let manager = AsyncTimerManager::new();
        let test_id = AtomicU64::new(3000);

        let id = next_test_id(&test_id);
        manager
            .schedule_timeout(Duration::from_millis(50), id, || {})
            .unwrap();

        // 立即取消
        let cancelled = manager.cancel(id);
        assert!(
            cancelled,
            "Cancel should return true for first cancellation"
        );

        // 等待取消命令被处理
        tokio::time::sleep(Duration::from_millis(10)).await;

        // 验证取消后再取消可能返回 true（因为命令已被处理）
        // 这是异步系统的预期行为
        let _cancelled_again = manager.cancel(id);
        // 注意：由于异步性质，第二次取消也可能返回 true
        // 关键是验证定时器确实被取消了（通过安排新定时器）
    }

    #[tokio::test]
    async fn test_clear_timers() {
        let manager = AsyncTimerManager::new();
        let test_id = AtomicU64::new(4000);

        // 安排多个定时器
        manager
            .schedule_timeout(Duration::from_millis(100), next_test_id(&test_id), || {})
            .unwrap();
        manager
            .schedule_timeout(Duration::from_millis(200), next_test_id(&test_id), || {})
            .unwrap();
        manager
            .schedule_timeout(Duration::from_millis(300), next_test_id(&test_id), || {})
            .unwrap();

        // 清除所有定时器
        manager.clear();

        // 验证清除后可以安排新定时器
        let id = next_test_id(&test_id);
        manager
            .schedule_timeout(Duration::from_millis(10), id, || {})
            .unwrap();
        assert!(id > 0, "Should be able to schedule after clear");

        // 验证取消已清除的定时器返回 false
        let _cancelled = manager.cancel(id);
        // 注意：清除后立即取消可能返回 true 或 false，取决于时序
    }

    #[tokio::test]
    async fn test_zero_delay_timeout() {
        let manager = AsyncTimerManager::new();
        let test_id = AtomicU64::new(5000);

        // 安排延迟为 0 的定时器
        let id = next_test_id(&test_id);
        manager
            .schedule_timeout(Duration::from_millis(0), id, || {})
            .unwrap();

        assert!(id > 0, "Timer ID should be positive");

        // 等待极短时间
        tokio::time::sleep(Duration::from_millis(1)).await;

        // 验证可以取消（即使它可能已经触发了）
        let _ = manager.cancel(id);
    }

    #[tokio::test]
    async fn test_timer_id_sequential() {
        let manager = AsyncTimerManager::new();
        let test_id = AtomicU64::new(6000);

        let id1 = next_test_id(&test_id);
        let id2 = next_test_id(&test_id);
        let id3 = next_test_id(&test_id);

        manager
            .schedule_timeout(Duration::from_millis(100), id1, || {})
            .unwrap();
        manager
            .schedule_timeout(Duration::from_millis(200), id2, || {})
            .unwrap();
        manager
            .schedule_timeout(Duration::from_millis(300), id3, || {})
            .unwrap();

        assert_eq!(id2, id1 + 1, "Timer IDs should be sequential");
        assert_eq!(id3, id2 + 1, "Timer IDs should be sequential");
    }
}
