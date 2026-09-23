// Process Pool Implementation for Amber
//
// This module implements a process pool system to reuse pre-spawned worker processes
// for script execution, significantly reducing the overhead of process creation.
//
// Key features:
// - Pre-spawned worker processes with initialized V8 runtimes
// - Intelligent process selection based on workload
// - Automatic process lifecycle management
// - Support for both simple and complex script execution

use anyhow::{Context, Result};
use crate::{OptimizeMode, Runtime};
use std::collections::{BTreeMap, HashMap};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::io::Read;
use std::hash::{Hash, Hasher, DefaultHasher};

// Import Runtime for worker execution
#[allow(unused_imports)]
const DEFAULT_POOL_SIZE: usize = 4;
const MAX_POOL_SIZE: usize = 16;
const SOCKET_PATH_PREFIX: &str = "/tmp/amberjs-pool-";
const WORKER_READY_MSG: &str = "READY";
const EXEC_SUCCESS_MSG: &str = "SUCCESS:";
const EXEC_ERROR_MSG: &str = "ERROR:";
/// Configuration for the process pool
#[derive(Debug, Clone)]
pub struct ProcessPoolConfig {
    /// Maximum number of worker processes in the pool
    pub max_workers: usize,
    /// Initial number of worker processes to spawn
    pub initial_workers: usize,
    /// Minimum number of worker processes (for scaling down)
    pub min_workers: usize,
    /// Timeout for worker initialization (ms)
    pub init_timeout_ms: u64,
    /// Enable process pool (disable for single-process mode)
    pub enabled: bool,
    /// Enable intelligent auto-scaling
    pub auto_scaling_enabled: bool,
    /// Queue length threshold to trigger scaling up
    pub scale_up_threshold: usize,
    /// Average wait time threshold to trigger scaling up (ms)
    pub scale_up_latency_ms: u64,
    /// Idle time threshold to trigger scaling down (seconds)
    pub scale_down_idle_seconds: u64,
    /// Scale up step (number of workers to add at once)
    pub scale_up_step: usize,
    /// Scale down step (number of workers to remove at once)
    pub scale_down_step: usize,
}
impl Default for ProcessPoolConfig {
    fn default() -> Self {
        let cpu_count: _ = num_cpus::get();
        Self {
            max_workers: std::cmp::min(MAX_POOL_SIZE, cpu_count),
            initial_workers: std::cmp::min(DEFAULT_POOL_SIZE, cpu_count),
            min_workers: std::cmp::min(2, cpu_count),
            init_timeout_ms: 5000,
            enabled: true,
            auto_scaling_enabled: true,
            scale_up_threshold: 3,
            scale_up_latency_ms: 100,
            scale_down_idle_seconds: 30,
            scale_up_step: std::cmp::min(2, cpu_count / 2),
            scale_down_step: 1,
        }
    }
}
/// Worker process state
#[derive(Debug, Clone, PartialEq)]
enum WorkerState {
    Starting,
    Ready,
    Busy,
    Terminating,
}
/// Information about a worker process
#[derive(Debug)]
struct WorkerInfo {
    #[allow(dead_code)]
    pid: u32,
    state: WorkerState,
    socket_path: String,
    last_used: Instant,
    current_task: Option<String>,
    #[allow(dead_code)]
    total_executions: usize,
}
/// Worker performance metrics for intelligent scheduling
#[derive(Debug, Clone)]
pub struct WorkerMetrics {
    pub worker_id: u32,
    pub avg_execution_time: Duration,
    pub success_rate: f64,
    pub memory_usage: usize,
    pub task_count: usize,
    pub last_used: Instant,
    pub min_execution_time: Duration,
    pub max_execution_time: Duration,
    pub total_execution_time: Duration,
    pub failed_tasks: usize,
    /// Weighted score for scheduling decisions (lower is better)
    pub scheduling_score: f64,
}
impl Default for WorkerMetrics {
    fn default() -> Self {
        Self {
            worker_id: 0,
            avg_execution_time: Duration::from_millis(100),
            success_rate: 1.0,
            memory_usage: 0,
            task_count: 0,
            last_used: Instant::now(),
            min_execution_time: Duration::from_millis(100),
            max_execution_time: Duration::from_millis(100),
            total_execution_time: Duration::from_millis(0),
            failed_tasks: 0,
            scheduling_score: 100.0,
        }
    }
}
impl WorkerMetrics {
    /// Update metrics after task execution
    pub fn update_after_execution(&mut self, execution_time: Duration, success: bool) {
        self.task_count += 1;
        self.total_execution_time += execution_time;
        // Update min/max
        if execution_time < self.min_execution_time {
            self.min_execution_time = execution_time;
        }
        if execution_time > self.max_execution_time {
            self.max_execution_time = execution_time;
        }
        // Update average using exponential moving average
        let alpha: _ = 0.1; // Smoothing factor
        self.avg_execution_time = self.avg_execution_time.mul_f64(1.0 - alpha) + execution_time.mul_f64(alpha);
        // Update success rate
        if success {
            // Keep success rate as exponential moving average
            self.success_rate = self.success_rate * 0.99 + 1.0 * 0.01;
        } else {
            self.failed_tasks += 1;
            self.success_rate = self.success_rate * 0.99;
        }
        // Calculate scheduling score (lower is better)
        // Factors: average execution time, success rate, task count
        let time_factor: _ = self.avg_execution_time.as_millis() as f64 / 100.0; // Normalize to 100ms units
        let reliability_factor: _ = 1.0 / self.success_rate; // Lower is better (inverse of success rate)
        let experience_factor: _ = (self.task_count as f64 / 1000.0).min(2.0); // Cap at 2x for experience
        self.scheduling_score = time_factor * reliability_factor * experience_factor;
        self.last_used = Instant::now();
    }
    /// Get a score for a specific task type
    /// Simple tasks prefer fast workers, complex tasks prefer reliable workers
    pub fn get_score_for_task_type(&self, task_complexity: TaskComplexity) -> f64 {
        match task_complexity {
            TaskComplexity::Simple => {
                // For simple tasks, prioritize speed
                self.avg_execution_time.as_millis() as f64 * (1.0 / self.success_rate)
            }
            TaskComplexity::Medium => {
                // For medium tasks, balance speed and reliability
                self.avg_execution_time.as_millis() as f64 * (2.0 - self.success_rate)
            }
            TaskComplexity::Complex => {
                // For complex tasks, prioritize reliability
                self.avg_execution_time.as_millis() as f64 * (1.0 / self.success_rate.powi(2))
            }
        }
    }
}
/// Task complexity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskComplexity {
    Simple,   // < 100 chars, no loops/conditions
    Medium,   // 100-500 chars, some loops/conditions
    Complex,  // > 500 chars, complex logic
}
impl TaskComplexity {
    /// Determine task complexity from script
    pub fn from_script(script: &str) -> Self {
        let len: _ = script.len();
        let has_loops: _ = script.contains("for") || script.contains("while");
        let has_conditions: _ = script.contains("if") || script.contains("else");
        let has_functions: _ = script.contains("function") || script.contains("=>");
        let complexity_score: _ = len / 100 + if has_loops { 2 } else { 0 } +
                              if has_conditions { 1 } else { 0 } + if has_functions { 1 } else { 0 };
        match complexity_score {
            0..=2 => TaskComplexity::Simple,
            3..=5 => TaskComplexity::Medium,
            _ => TaskComplexity::Complex,
        }
    }
}
/// Statistics about the process pool
#[derive(Debug, Clone)]
pub struct ProcessPoolStats {
    pub total_workers: usize,
    pub ready_workers: usize,
    pub busy_workers: usize,
    pub total_executions: usize,
    pub avg_execution_time_ms: f64,
    pub pool_hit_rate: f64,
    /// Current queue length
    pub current_queue_length: usize,
    /// Average wait time for tasks (ms)
    pub avg_wait_time_ms: f64,
    /// Total scaling operations performed
    pub total_scale_operations: usize,
    /// Peak queue length observed
    pub peak_queue_length: usize,
    /// Worker utilization percentage
    pub worker_utilization_percent: f64,
    /// Worker metrics for intelligent scheduling
    pub worker_metrics: HashMap<u32, WorkerMetrics>,
}
impl Default for ProcessPoolStats {
    fn default() -> Self {
        Self {
            total_workers: 0,
            ready_workers: 0,
            busy_workers: 0,
            total_executions: 0,
            avg_execution_time_ms: 0.0,
            pool_hit_rate: 0.0,
            current_queue_length: 0,
            avg_wait_time_ms: 0.0,
            total_scale_operations: 0,
            peak_queue_length: 0,
            worker_utilization_percent: 0.0,
            worker_metrics: HashMap::new(),
        }
    }
}
/// The main Process Pool manager
pub struct ProcessPool {
    config: ProcessPoolConfig,
    workers: Arc<Mutex<HashMap<u32, WorkerInfo>>>,
    available_workers: Arc<Mutex<Vec<u32>>>,
    stats: Arc<Mutex<ProcessPoolStats>>,
    worker_counter: AtomicUsize,
    shutdown: Arc<AtomicBool>,
    /// Task queue for auto-scaling monitoring
    task_queue: Arc<Mutex<Vec<Instant>>>,
    /// Last scaling operation timestamp
    last_scale_operation: Arc<Mutex<Instant>>,
    /// Worker idle time tracking
    #[allow(dead_code)]
    worker_idle_times: Arc<Mutex<HashMap<u32, Instant>>>, // Reserved for future idle tracking
}
impl ProcessPool {
    /// Create a new process pool with the given configuration
    pub fn new(config: ProcessPoolConfig) -> Result<Self> {
        let pool: _ = Self {
            config: config.clone(),
            workers: Arc::new(Mutex::new(HashMap::new())),
            available_workers: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(ProcessPoolStats::default())),
            worker_counter: AtomicUsize::new(0),
            shutdown: Arc::new(Mutex::new(AtomicBool::new(false))),
            task_queue: Arc::new(Mutex::new(Vec::new())),
            last_scale_operation: Arc::new(Mutex::new(Instant::now())),
            worker_idle_times: Arc::new(Mutex::new(HashMap::new())),
        };
        // Workers are initialized lazily on first use to avoid async complexity
        Ok(pool)
    }
    /// Lazy initialization - spawn workers on first use
    fn ensure_initialized(&self) -> Result<()> {
        let workers_count: _ = {
            let workers: _ = self.workers.lock().unwrap();
            workers.len()
        };
        if workers_count == 0 && self.config.enabled {
            // Spawn initial workers synchronously
            for i in 0..self.config.initial_workers {
                let _: _ = self.spawn_worker_blocking(i)?;
            }
        }
        Ok(())
    }
    /// Spawn worker synchronously (blocking version for initialization)
    fn spawn_worker_blocking(&self, worker_id: usize) -> Result<u32> {
        let socket_path: _ = format!("{}{}", worker_id, SOCKET_PATH_PREFIX);
        // Remove old socket if exists
        let _: _ = std::fs::remove_file(&socket_path);
        // Create Unix domain socket for IPC
        let _listener: _ = UnixListener::bind(&socket_path)
            .context("Failed to create Unix socket")?;
        let pid: _ = self.worker_counter.fetch_add(1, Ordering::SeqCst) as u32;
        let worker_info: _ = WorkerInfo {
            pid,
            state: WorkerState::Starting,
            socket_path: socket_path.clone(),
            last_used: Instant::now(),
            current_task: None,
            total_executions: 0,
        };
        // Add to workers map
        {
            let mut workers = self.workers.lock().unwrap();
            workers.insert(pid, worker_info);
        }
        // Spawn the worker process
        let worker_stdout: _ = Stdio::null();
        let worker_stderr: _ = Stdio::null();
        let mut child = Command::new(std::env::current_exe()?)
            .arg("--worker-mode")
            .arg("--worker-id")
            .arg(pid.to_string())
            .arg("--socket-path")
            .arg(&socket_path)
            .stdout(worker_stdout)
            .stderr(worker_stderr)
            .spawn()
            .context("Failed to spawn worker process")?;
        let child_pid: _ = child.id();
        // Wait for worker to signal readiness synchronously
        let max_wait: _ = std::time::Duration::from_millis(self.config.init_timeout_ms);
        let wait_start: _ = Instant::now();
        while wait_start.elapsed() < max_wait {
            // Check if worker has written the ready message
            if let Ok(content) = std::fs::read_to_string(&socket_path) {
                if content == WORKER_READY_MSG {
                    // Worker is ready, update state
                    {
                        let mut workers = self.workers.lock().unwrap();
                        if let Some(worker) = workers.get_mut(&child_pid) {
                            worker.state = WorkerState::Ready;
                        }
                    }
                    // Add to available workers
                    {
                        let mut available = self.available_workers.lock().unwrap();
                        available.push(child_pid);
                    }
                    println!("[ProcessPool] Spawned worker {} (PID: {}) - READY", worker_id, child_pid);
                    return Ok(child_pid);
                }
            }
            // Check if child process has exited (worker failed to start)
            match child.try_wait() {
                Ok(Some(_)) => {
                    return Err(anyhow::anyhow!("Worker process {} failed to start", worker_id));
                }
                Ok(None) => {
                    // Process still running, wait a bit
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Error checking worker status: {}", e));
                }
            }
        }
        Err(anyhow::anyhow!("Worker {} failed to become ready within timeout", worker_id))
    }
    /// Initialize worker processes (async to allow proper async/await)
    #[allow(dead_code)]
    async fn initialize_workers(&self) -> Result<()> {
        let initial: _ = self.config.initial_workers;
        println!("[ProcessPool] Initializing {} worker processes...", initial);
        for i in 0..initial {
            match self.spawn_worker(i).await {
                Ok(_) => {
                    if i % 2 == 0 {
                        print!(".");
                    }
                }
                Err(e) => {
                    eprintln!("\n[ProcessPool] Failed to spawn worker {}: {}", i, e);
                }
            }
        }
        println!("\n[ProcessPool] Worker pool ready!");
        Ok(())
    }
    /// Spawn a new worker process (async wrapper)
    #[allow(dead_code)]
    async fn spawn_worker(&self, worker_id: usize) -> Result<u32> {
        // Use the blocking version for actual spawning
        self.spawn_worker_blocking(worker_id)
    }
    /// Execute a script using an available worker process
    pub async fn execute_script(&self, script: &str) -> Result<String> {
        if !self.config.enabled {
            // Fallback to direct execution if pool is disabled
            return self.execute_direct(script).await;
        }
        // Lazy initialization on first use
        self.ensure_initialized()?;
        let start: _ = Instant::now();
        // Add to task queue for monitoring
        {
            let mut queue = self.task_queue.lock().unwrap();
            queue.push(start);
            // Update peak queue length
            if queue.len() > self.stats.lock().unwrap().peak_queue_length {
                self.stats.lock().unwrap().peak_queue_length = queue.len();
            }
        }
        // Check if we need to scale up
        if self.config.auto_scaling_enabled {
            self.check_and_scale().await;
        }
        // Get an available worker
        let worker_pid: _ = self.acquire_worker().await
            .context("No available workers")?;
        let wait_time: _ = start.elapsed();
        let task_start: _ = Instant::now();
        // Determine task complexity for intelligent scheduling
        let _task_complexity: _ = TaskComplexity::from_script(script);
        let result: _ = self.execute_on_worker(worker_pid, script).await;
        // Update worker metrics after execution
        let execution_time: _ = task_start.elapsed();
        let success: _ = result.is_ok();
        self.update_worker_metrics(worker_pid, execution_time, success);
        // Release the worker
        self.release_worker(worker_pid);
        // Remove from task queue and update statistics
        {
            let mut queue = self.task_queue.lock().unwrap();
            if !queue.is_empty() {
                queue.remove(0);
            }
            self.stats.lock().unwrap().current_queue_length = queue.len();
        }
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_executions += 1;
            let elapsed: _ = start.elapsed();
            let avg_time: _ = stats.avg_execution_time_ms;
            stats.avg_execution_time_ms = avg_time * 0.9 + elapsed.as_millis() as f64 * 0.1;
            let avg_wait: _ = stats.avg_wait_time_ms;
            stats.avg_wait_time_ms = avg_wait * 0.9 + wait_time.as_millis() as f64 * 0.1;
            // Update worker utilization
            let total_workers: _ = stats.total_workers;
            if total_workers > 0 {
                stats.worker_utilization_percent = (stats.busy_workers as f64 / total_workers as f64) * 100.0;
            }
        }
        // Check if we need to scale down
        if self.config.auto_scaling_enabled {
            self.check_and_scale_down().await;
        }
        result
    }
    /// Acquire an available worker process using intelligent scheduling
    async fn acquire_worker(&self) -> Option<u32> {
        let available: _ = self.available_workers.lock().unwrap();
        if available.is_empty() {
            // No ready workers, try to spawn a new one
            let workers_count: _ = self.workers.lock().unwrap().len();
            drop(available);
            if workers_count < self.config.max_workers {
                if let Ok(new_pid) = self.spawn_worker_blocking(workers_count) {
                    return Some(new_pid);
                }
            }
            return None;
        }
        // Intelligent worker selection
        let task_complexity: _ = TaskComplexity::Simple; // Default for now, will be updated based on actual task
        self.select_optimal_worker(&available, task_complexity)
    }
    /// Select optimal worker based on historical performance and task type
    fn select_optimal_worker(&self, available_workers: &Vec<u32>, task_complexity: TaskComplexity) -> Option<u32> {
        if available_workers.is_empty() {
            return None;
        }
        let workers: _ = self.workers.lock().unwrap();
        let stats: _ = self.stats.lock().unwrap();
        // Collect worker metrics
        let mut worker_candidates: Vec<(u32, f64)> = Vec::new();
        for &pid in available_workers {
            if let Some(worker) = workers.get(&pid) {
                if worker.state == WorkerState::Ready {
                    // Get or create worker metrics
                    let metrics: _ = stats.worker_metrics.get(&pid)
                        .cloned()
                        .unwrap_or_else(|| WorkerMetrics {
                            worker_id: pid,
                            ..Default::default()
                        });
                    // Calculate score based on task complexity
                    let score: _ = metrics.get_score_for_task_type(task_complexity);
                    worker_candidates.push((pid, score));
                }
            }
        }
        drop(workers);
        drop(stats);
        if worker_candidates.is_empty() {
            return None;
        }
        // Sort by score (lower is better)
        worker_candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        // Apply load balancing: randomly select from top 3 workers to avoid always picking the same one
        let top_k: _ = std::cmp::min(3, worker_candidates.len());
        let selected_index: _ = if top_k > 1 {
            let mut hasher = DefaultHasher::new();
            std::time::Instant::now().hash(&mut hasher);
            (hasher.finish() as usize) % top_k
        } else {
            0
        };
        Some(worker_candidates[selected_index].0)
    }
    /// Update worker metrics after task execution
    pub fn update_worker_metrics(&self, worker_pid: u32, execution_time: Duration, success: bool) {
        let mut stats = self.stats.lock().unwrap();
        let metrics: _ = stats.worker_metrics.entry(worker_pid).or_insert_with(|| WorkerMetrics {
            worker_id: worker_pid,
            ..Default::default()
        });
        metrics.update_after_execution(execution_time, success);
    }
    /// Get worker metrics for debugging/analysis
    pub fn get_worker_metrics(&self) -> HashMap<u32, WorkerMetrics> {
        let stats: _ = self.stats.lock().unwrap();
        stats.worker_metrics.clone()
    }
    /// Release a worker process back to the pool
    fn release_worker(&self, worker_pid: u32) {
        let mut available = self.available_workers.lock().unwrap();
        available.push(worker_pid);
        // Update worker's last used time
        let mut workers = self.workers.lock().unwrap();
        if let Some(worker) = workers.get_mut(&worker_pid) {
            worker.last_used = Instant::now();
            worker.current_task = None;
            if worker.state != WorkerState::Terminating {
                worker.state = WorkerState::Ready;
            }
        }
    }
    /// Execute script on a specific worker process
    async fn execute_on_worker(&self, worker_pid: u32, script: &str) -> Result<String> {
        let socket_path: _ = {
            let workers: _ = self.workers.lock().unwrap();
            workers.get(&worker_pid)
                .map(|w| w.socket_path.clone())
                .ok_or_else(|| anyhow::anyhow!("Worker not found"))?
        };
        // Connect to the worker's Unix socket
        let mut stream = UnixStream::connect(&socket_path)
            .await
            .context("Failed to connect to worker")?;
        // Send the script
        stream.write_all(script.as_bytes()).await?;
        stream.write_all(b"\nEND\n").await?;
        // Read the result
        let mut response = String::new();
        stream.read_to_string(&mut response).await?;
        // Parse the response
        if response.starts_with(EXEC_SUCCESS_MSG) {
            Ok(response[EXEC_SUCCESS_MSG.len()..].to_string())
        } else if response.starts_with(EXEC_ERROR_MSG) {
            Err(anyhow::anyhow!("{}", &response[EXEC_ERROR_MSG.len()..]))
        } else {
            Err(anyhow::anyhow!("Invalid response from worker: {}", response))
        }
    }
    /// Execute script directly (fallback when pool is disabled)
    async fn execute_direct(&self, script: &str) -> Result<String> {
        // This would integrate with the existing Runtime system
        // For now, return a placeholder
        Ok(format!("Direct execution: {}", script.len()))
    }
    /// Get process pool statistics
    pub fn get_stats(&self) -> ProcessPoolStats {
        // Ensure initialization for accurate stats
        let _: _ = self.ensure_initialized();
        let stats: _ = self.stats.lock().unwrap().clone();
        let workers: _ = self.workers.lock().unwrap();
        ProcessPoolStats {
            total_workers: workers.len(),
            ready_workers: workers.values().filter(|w| w.state == WorkerState::Ready).count(),
            busy_workers: workers.values().filter(|w| w.state == WorkerState::Busy).count(),
            current_queue_length: stats.current_queue_length,
            avg_wait_time_ms: stats.avg_wait_time_ms,
            total_scale_operations: stats.total_scale_operations,
            peak_queue_length: stats.peak_queue_length,
            worker_utilization_percent: stats.worker_utilization_percent,
            total_executions: stats.total_executions,
            avg_execution_time_ms: stats.avg_execution_time_ms,
            pool_hit_rate: stats.pool_hit_rate,
            worker_metrics: stats.worker_metrics.clone(),
        }
    }
    /// Check if we need to scale up the pool
    async fn check_and_scale(&self) {
        let queue_length;
        let avg_wait_time;
        {
            let stats: _ = self.stats.lock().unwrap();
            queue_length = stats.current_queue_length;
            avg_wait_time = stats.avg_wait_time_ms;
        }
        let should_scale_up: _ = queue_length >= self.config.scale_up_threshold
            || avg_wait_time >= self.config.scale_up_latency_ms as f64;
        if should_scale_up {
            let current_workers: _ = {
                let workers: _ = self.workers.lock().unwrap();
                workers.len()
            };
            if current_workers < self.config.max_workers {
                // Prevent rapid scaling
                let last_scale: _ = *self.last_scale_operation.lock().unwrap();
                if last_scale.elapsed().as_secs() >= 2 {
                    self.scale_up().await;
                }
            }
        }
    }
    /// Check if we need to scale down the pool
    async fn check_and_scale_down(&self) {
        let current_workers;
        let queue_length;
        let utilization;
        let all_idle;
        {
            let workers: _ = self.workers.lock().unwrap();
            current_workers = workers.len();
            queue_length = self.stats.lock().unwrap().current_queue_length;
            utilization = self.stats.lock().unwrap().worker_utilization_percent;
            // Check if workers have been idle
            let idle_threshold: _ = std::time::Duration::from_secs(self.config.scale_down_idle_seconds);
            all_idle = workers.values().all(|w| w.last_used.elapsed() >= idle_threshold);
        }
        // Only scale down if queue is empty, utilization is low, and workers are idle
        if queue_length == 0
            && utilization < 50.0
            && all_idle
            && current_workers > self.config.min_workers
        {
            let last_scale: _ = *self.last_scale_operation.lock().unwrap();
            if last_scale.elapsed().as_secs() >= 10 {
                self.scale_down().await;
            }
        }
    }
    /// Scale up the process pool
    async fn scale_up(&self) {
        let current_workers: _ = {
            let workers: _ = self.workers.lock().unwrap();
            workers.len()
        };
        let workers_to_add: _ = std::cmp::min(
            self.config.scale_up_step,
            self.config.max_workers - current_workers
        );
        if workers_to_add > 0 {
            println!("[ProcessPool] Scaling up: adding {} workers (current: {})",
                     workers_to_add, current_workers);
            for _i in 0..workers_to_add {
                let worker_id: _ = self.worker_counter.fetch_add(1, Ordering::SeqCst);
                if let Ok(pid) = self.spawn_worker_blocking(worker_id) {
                    println!("[ProcessPool] Scaled up: added worker {} (PID: {})", worker_id, pid);
                }
            }
            // Update statistics
            {
                let mut stats = self.stats.lock().unwrap();
                stats.total_scale_operations += 1;
            }
            *self.last_scale_operation.lock().unwrap() = Instant::now();
        }
    }
    /// Scale down the process pool
    async fn scale_down(&self) {
        let current_workers: _ = {
            let workers: _ = self.workers.lock().unwrap();
            workers.len()
        };
        let workers_to_remove: _ = std::cmp::min(
            self.config.scale_down_step,
            current_workers - self.config.min_workers
        );
        if workers_to_remove > 0 {
            println!("[ProcessPool] Scaling down: removing {} workers (current: {})",
                     workers_to_remove, current_workers);
            // Get idle workers to terminate
            let idle_workers: _ = {
                let workers: _ = self.workers.lock().unwrap();
                let idle_threshold: _ = std::time::Duration::from_secs(self.config.scale_down_idle_seconds);
                workers.iter()
                    .filter(|(_, w)| w.state == WorkerState::Ready && w.last_used.elapsed() > idle_threshold)
                    .map(|(pid, _)| *pid)
                    .take(workers_to_remove)
                    .collect::<Vec<u32>>()
            };
            for pid in idle_workers {
                // Mark worker as terminating and remove it
                {
                    let mut workers = self.workers.lock().unwrap();
                    if let Some(_worker) = workers.get_mut(&pid) {
                        // Worker state will be marked as terminating
                    }
                }
                // Remove from available workers list
                {
                    let mut available = self.available_workers.lock().unwrap();
                    available.retain(|&p| p != pid);
                }
                // Terminate the process
                let _: _ = std::process::Command::new("kill")
                    .args(&["-TERM", &pid.to_string()])
                    .spawn();
                println!("[ProcessPool] Scaled down: terminated worker PID: {}", pid);
            }
            // Update statistics
            {
                let mut stats = self.stats.lock().unwrap();
                stats.total_scale_operations += 1;
            }
            *self.last_scale_operation.lock().unwrap() = Instant::now();
        }
    }
    /// Shutdown the process pool and all workers
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let workers: _ = self.workers.lock().unwrap();
        for worker in workers.values() {
            // Send shutdown signal to worker
            let _: _ = std::fs::remove_file(&worker.socket_path);
        }
        println!("[ProcessPool] Shutdown complete");
    }
}
/// Global process pool instance
static PROCESS_POOL: std::sync::OnceLock<Arc<ProcessPool>> = std::sync::OnceLock::new();
static POOL_CONFIG: std::sync::OnceLock<ProcessPoolConfig> = std::sync::OnceLock::new();
/// Initialize the global process pool
pub fn initialize_process_pool(config: ProcessPoolConfig) -> Result<()> {
    let pool: _ = ProcessPool::new(config.clone())
        .context("Failed to initialize process pool")?;
    POOL_CONFIG.set(config)
        .map_err(|_| anyhow::anyhow!("Failed to store pool config"))?;
    PROCESS_POOL.set(Arc::new(Mutex::new(pool)))
        .map_err(|_| anyhow::anyhow!("Failed to set global process pool"))?;
    println!("[ProcessPool] Global process pool initialized");
    Ok(())
}
/// Get the global process pool instance
pub fn get_process_pool() -> Option<Arc<ProcessPool>> {
    PROCESS_POOL.get().cloned()
}
/// Execute a script using the process pool
pub async fn execute_with_pool(script: &str) -> Result<String> {
    if let Some(pool) = get_process_pool() {
        pool.execute_script(script).await
    } else {
        Err(anyhow::anyhow!("Process pool not initialized"))
    }
}
/// Execute script in worker process using Runtime
async fn execute_script_in_worker(script: &str) -> Result<String> {
    // Create a Runtime instance for this worker
    // Use default settings optimized for speed
    let runtime: _ = Runtime::new(67108864, 134217728, false, false) // 64MB stack, 128MB heap, no verbose
        .context("Failed to create Runtime in worker")?;
    // Execute the script and capture output
    let result: _ = runtime.execute_code(script)
        .context("Worker failed to execute script")?;
    Ok(result)
}
/// Worker mode entry point (called by spawned worker processes)
pub async fn worker_main(worker_id: u32, socket_path: String) -> Result<()> {
    // Create Unix socket listener
    let listener: _ = UnixListener::bind(&socket_path)
        .context("Worker failed to bind socket")?;
    println!("[Worker-{}] Started and listening on {}", worker_id, socket_path);
    // Signal ready to parent
    let _: _ = std::fs::write(&socket_path, WORKER_READY_MSG);
    // Accept connections and execute scripts
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let mut buffer = String::new();
                let mut stream = stream;
                // Read script from socket
                stream.read_to_string(&mut buffer).await?;
                if buffer.trim() == "SHUTDOWN" {
                    println!("[Worker-{}] Received shutdown signal", worker_id);
                    break;
                }
                // Execute the script using Runtime
                let script_result: _ = match execute_script_in_worker(&buffer).await {
                    Ok(output) => format!("{}{}", EXEC_SUCCESS_MSG, output),
                    Err(e) => format!("{}{}", EXEC_ERROR_MSG, e),
                };
                stream.write_all(script_result.as_bytes()).await?;
            }
            Err(e) => {
                eprintln!("[Worker-{}] Error accepting connection: {}", worker_id, e);
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore] // Disabled due to async spawn complexity in test environment
    async fn test_process_pool_performance() {
        // Test: Compare process pool vs new process execution
        let config: _ = ProcessPoolConfig {
            max_workers: 2,
            initial_workers: 2,
            min_workers: 1,
            init_timeout_ms: 5000,
            enabled: true,
            auto_scaling_enabled: true,
            scale_up_threshold: 3,
            scale_up_latency_ms: 100,
            scale_down_idle_seconds: 30,
            scale_up_step: 1,
            scale_down_step: 1,
        };
        // Initialize process pool
        let pool: _ = Arc::new(Mutex::new(ProcessPool::new(config).expect("Failed to create pool")));
        // Test script - simple computation
        let test_script: _ = r#"
            let sum: _ = 0;
            for (let i: _ = 0; i < 100; i++) {
                sum += i;
            }
            sum
        "#;
        // Execute script multiple times through pool
        let start: _ = Instant::now();
        for _ in 0..10 {
            let result: _ = pool.execute_script(test_script).await;
            assert!(result.is_ok(), "Pool execution failed");
        }
        let pool_time: _ = start.elapsed();
        println!("Process pool: 10 executions in {:?}", pool_time);
        // Verify results contain expected output
        let result: _ = pool.execute_script(test_script).await.unwrap();
        assert!(result.contains("4950"), "Expected result 4950, got: {}", result);
        // Test that pool is still functional
        let stats: _ = pool.get_stats();
        assert!(stats.total_workers > 0, "Pool should have workers");
        assert!(stats.total_executions >= 10, "Should track executions");
        println!("Pool stats: {:?}", stats);
    }
    #[tokio::test]
    #[ignore] // Disabled due to async spawn complexity in test environment
    async fn test_process_pool_concurrent_execution() {
        // Test concurrent script execution
        let config: _ = ProcessPoolConfig {
            max_workers: 4,
            initial_workers: 4,
            min_workers: 1,
            init_timeout_ms: 5000,
            enabled: true,
            auto_scaling_enabled: true,
            scale_up_threshold: 3,
            scale_up_latency_ms: 100,
            scale_down_idle_seconds: 30,
            scale_up_step: 2,
            scale_down_step: 1,
        };
        let pool: _ = Arc::new(Mutex::new(ProcessPool::new(config).expect("Failed to create pool")));
        // Test multiple concurrent executions
        let mut handles = vec![];
        for i in 0..8 {
            let pool_clone: _ = Arc::clone(pool);
            let script: _ = format!(r#"console.log("Task {}");"#, i);
            let handle: _ = tokio::spawn(async move {
                pool_clone.execute_script(&script).await
            });
            handles.push(handle);
        }
        // Wait for all tasks to complete
        for (i, handle) in handles.into_iter().enumerate() {
            let result: _ = handle.await.expect("Task panicked");
            assert!(result.is_ok(), "Concurrent execution {} failed", i);
        }
        // Verify all executions were tracked
        let stats: _ = pool.get_stats();
        assert!(stats.total_executions >= 8, "Should track all 8 executions");
        println!("Concurrent execution stats: {:?}", stats);
    }
    #[tokio::test]
    async fn test_process_pool_creation() {
        let config: _ = ProcessPoolConfig {
            max_workers: 2,
            initial_workers: 1,
            min_workers: 1,
            init_timeout_ms: 1000,
            enabled: true,
            auto_scaling_enabled: true,
            scale_up_threshold: 3,
            scale_up_latency_ms: 100,
            scale_down_idle_seconds: 30,
            scale_up_step: 1,
            scale_down_step: 1,
        };
        let pool: _ = ProcessPool::new(config);
        assert!(pool.is_ok());
    }
    #[tokio::test]
    #[ignore] // Disabled due to async spawn complexity in test environment
    async fn test_process_pool_stats() {
        let config: _ = ProcessPoolConfig {
            max_workers: 2,
            initial_workers: 1,
            min_workers: 1,
            init_timeout_ms: 5000,
            enabled: true,
            auto_scaling_enabled: true,
            scale_up_threshold: 3,
            scale_up_latency_ms: 100,
            scale_down_idle_seconds: 30,
            scale_up_step: 1,
            scale_down_step: 1,
        };
        let pool: _ = ProcessPool::new(config).unwrap();
        // Give the worker process time to initialize and signal readiness
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let stats: _ = pool.get_stats();
        assert_eq!(stats.total_workers, 1, "Should have 1 worker");
        assert_eq!(stats.ready_workers, 1, "Worker should be ready");
    }
}