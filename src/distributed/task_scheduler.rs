// 分布式任务调度模块
// 提供任务分发、优先级队列、结果聚合等功能

use std::collections::{HashMap, BinaryHeap};
use std::cmp::Reverse;
use std::hash::Hash;
/// 任务类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskType {
    JavaScriptExecution,
    TypeScriptCompilation,
    AIInference,
    DataProcessing,
}
/// 任务状态枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
/// 任务定义
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub payload: Vec<u8>,
    pub priority: u8,
    pub created_at: Instant,
    pub timeout: Duration,
    pub metadata: HashMap<String, String>,
}
/// 任务结果
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub result_data: Option<Vec<u8>>,
    pub error_message: Option<String>,
    pub execution_time: Duration,
    pub node_id: Option<String>,
}
/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub max_concurrent_tasks: usize,
    pub task_timeout: Duration,
    pub retry_attempts: u32,
    pub enable_priority_queue: bool,
}
impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 100,
            task_timeout: Duration::from_secs(30),
            retry_attempts: 3,
            enable_priority_queue: true,
        }
    }
}
/// 分发器配置
#[derive(Debug, Clone)]
pub struct DistributorConfig {
    pub max_tasks_per_node: usize,
    pub load_balancing_strategy: String,
    pub enable_locality: bool,
}
impl Default for DistributorConfig {
    fn default() -> Self {
        Self {
            max_tasks_per_node: 50,
            load_balancing_strategy: "least_loaded".to_string(),
            enable_locality: true,
        }
    }
}
/// 聚合器配置
#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    pub aggregation_strategy: String,
    pub timeout: Duration,
    pub min_results: usize,
}
impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            aggregation_strategy: "collect_all".to_string(),
            timeout: Duration::from_secs(30),
            min_results: 1,
        }
    }
}
/// 节点信息（简化版）
#[derive(Debug, Clone)]
pub struct SchedulerNodeInfo {
    pub id: String,
    pub cpu_cores: usize,
    pub memory_gb: usize,
    pub current_load: u8,
    pub capabilities: Vec<TaskType>,
    pub region: String,
}
/// 调度统计信息
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub total_tasks: u64,
    pub pending_tasks: u64,
    pub running_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub average_execution_time: Duration,
    pub throughput_per_second: f64,
}
// ============================================================================
// 任务调度器 (Task Scheduler)
// ============================================================================
/// 任务调度器 - 负责任务的接收、排队和管理
#[derive(Debug)]
pub struct TaskScheduler {
    config: SchedulerConfig,
    pending_tasks: BinaryHeap<Reverse<TaskWrapper>>,
    running_tasks: HashMap<String, Task>,
    completed_tasks: HashMap<String, TaskResult>,
    failed_tasks: HashMap<String, Task>,
    stats: SchedulerStats,
}
/// 任务包装器，用于优先级队列排序
#[derive(Debug, Clone)]
struct TaskWrapper {
    task: Task,
}
impl PartialEq for TaskWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.task.priority == other.task.priority
    }
}
impl Eq for TaskWrapper {}
impl PartialOrd for TaskWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // 反转比较，高优先级任务在前面
        other.task.priority.partial_cmp(&self.task.priority)
    }
}
impl Ord for TaskWrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}
impl TaskScheduler {
    /// 创建新的任务调度器
    pub fn new(config: SchedulerConfig) -> Result<Self, String> {
        if config.max_concurrent_tasks == 0 {
            return Err("max_concurrent_tasks must be greater than 0".to_string());
        }
        Ok(Self {
            config,
            pending_tasks: BinaryHeap::new(),
            running_tasks: HashMap::new(),
            completed_tasks: HashMap::new(),
            failed_tasks: HashMap::new(),
            stats: SchedulerStats {
                total_tasks: 0,
                pending_tasks: 0,
                running_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
                average_execution_time: Duration::from_millis(0),
                throughput_per_second: 0.0,
            },
        })
    }
    /// 提交任务
    pub fn submit_task(&mut self, task: Task) -> Result<(), String> {
        if self.running_tasks.len() >= self.config.max_concurrent_tasks {
            return Err("Maximum concurrent tasks reached".to_string());
        }
        // 检查是否已存在相同 ID 的任务
        if self.running_tasks.contains_key(&task.id)
            || self.completed_tasks.contains_key(&task.id)
            || self.failed_tasks.contains_key(&task.id)
        {
            return Err(format!("Task with id '{}' already exists", task.id));
        }
        self.pending_tasks.push(Reverse(TaskWrapper { task }));
        self.stats.total_tasks += 1;
        self.stats.pending_tasks += 1;
        Ok(())
    }
    /// 获取下一个待执行的任务
    pub fn get_next_task(&mut self) -> Option<Task> {
        if let Some(Reverse(TaskWrapper { task })) = self.pending_tasks.pop() {
            self.stats.pending_tasks -= 1;
            self.stats.running_tasks += 1;
            self.running_tasks.insert(task.id.clone(), task.clone());
            Some(task)
        } else {
            None
        }
    }
    /// 标记任务完成
    pub fn mark_task_completed(&mut self, task_id: &str) -> Option<TaskResult> {
        if let Some(task) = self.running_tasks.remove(task_id) {
            let result: _ = TaskResult {
                task_id: task.id.clone(),
                status: TaskStatus::Completed,
                result_data: Some(Vec::new()),
                error_message: None,
                execution_time: Duration::from_millis(0),
                node_id: None,
            };
            self.completed_tasks.insert(task.id.clone(), result.clone());
            self.stats.running_tasks -= 1;
            self.stats.completed_tasks += 1;
            Some(result)
        } else {
            None
        }
    }
    /// 清理超时任务
    pub fn cleanup_timed_out_tasks(&mut self) -> usize {
        let now: _ = Instant::now();
        let mut timed_out_count = 0;
        // 清理 pending 队列中的超时任务
        let mut remaining_tasks = BinaryHeap::new();
        while let Some(Reverse(wrapper)) = self.pending_tasks.pop() {
            if now.duration_since(wrapper.task.created_at) > wrapper.task.timeout {
                timed_out_count += 1;
                self.stats.pending_tasks -= 1;
                self.stats.failed_tasks += 1;
                self.failed_tasks.insert(wrapper.task.id.clone(), wrapper.task);
            } else {
                remaining_tasks.push(Reverse(wrapper));
            }
        }
        self.pending_tasks = remaining_tasks;
        // 清理 running 队列中的超时任务
        let mut timed_out_tasks = Vec::new();
        for (task_id, task) in &self.running_tasks {
            if now.duration_since(task.created_at) > task.timeout {
                timed_out_tasks.push(task_id.clone());
            }
        }
        for task_id in timed_out_tasks {
            if let Some(task) = self.running_tasks.remove(&task_id) {
                timed_out_count += 1;
                self.stats.running_tasks -= 1;
                self.stats.failed_tasks += 1;
                self.failed_tasks.insert(task_id, task);
            }
        }
        timed_out_count
    }
    /// 获取待处理任务数量
    pub fn get_pending_task_count(&self) -> usize {
        self.pending_tasks.len()
    }
    /// 获取调度统计信息
    pub fn get_stats(&self) -> &SchedulerStats {
        &self.stats
    }
}
// ============================================================================
// 任务分发器 (Task Distributor)
// ============================================================================
/// 任务分发器 - 负责任务到节点的智能分发
#[derive(Debug)]
pub struct TaskDistributor {
    config: DistributorConfig,
    nodes: HashMap<String, SchedulerNodeInfo>,
}
impl TaskDistributor {
    /// 创建新的任务分发器
    pub fn new(config: DistributorConfig) -> Result<Self, String> {
        Ok(Self {
            config,
            nodes: HashMap::new(),
        })
    }
    /// 注册节点
    pub fn register_node(&mut self, node: SchedulerNodeInfo) -> Result<(), String> {
        if node.cpu_cores == 0 {
            return Err("CPU cores must be greater than 0".to_string());
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }
    /// 注销节点
    pub fn unregister_node(&mut self, node_id: &str) -> Option<SchedulerNodeInfo> {
        self.nodes.remove(node_id)
    }
    /// 分发任务到节点
    pub fn distribute_task(&self, task: &Task) -> Option<String> {
        // 找到支持该任务类型的节点
        let mut compatible_nodes: Vec<&SchedulerNodeInfo> = self.nodes
            .values()
            .filter(|node| node.capabilities.contains(&task.task_type))
            .collect();
        if compatible_nodes.is_empty() {
            return None;
        }
        // 根据负载均衡策略选择节点
        let node_id: _ = match self.config.load_balancing_strategy.as_str() {
            "least_loaded" => {
                compatible_nodes.sort_by_key(|node| node.current_load);
                compatible_nodes.first().map(|n| &n.id).cloned()
            }
            "random" => {
                use rand::Rng;
use std::collections::{BTreeMap};
use std::time::{Duration, Instant};
use std::sync::atomic::Ordering;

                let mut rng = rand::rng();
                if compatible_nodes.is_empty() {
                    None
                } else {
                    let index: _ = rng.random_range(0..compatible_nodes.len());
                    Some(compatible_nodes[index].id.clone())
                }
            }
            _ => {
                // 默认使用最少加载
                compatible_nodes.sort_by_key(|node| node.current_load);
                compatible_nodes.first().map(|n| &n.id).cloned()
            }
        };
        node_id
    }
    /// 更新节点负载
    pub fn update_node_load(&mut self, node_id: &str, new_load: u8) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.current_load = new_load;
            Ok(())
        } else {
            Err(format!("Node '{}' not found", node_id))
        }
    }
    /// 获取节点信息
    pub fn get_node_info(&self, node_id: &str) -> Option<&SchedulerNodeInfo> {
        self.nodes.get(node_id)
    }
    /// 获取已注册节点数量
    pub fn get_registered_node_count(&self) -> usize {
        self.nodes.len()
    }
}
// ============================================================================
// 结果聚合器 (Result Aggregator)
// ============================================================================
/// 结果聚合器 - 负责收集和聚合任务结果
#[derive(Debug)]
pub struct ResultAggregator {
    config: AggregatorConfig,
    batches: HashMap<String, BatchResults>,
}
/// 批量结果
#[derive(Debug, Clone)]
struct BatchResults {
    results: Vec<TaskResult>,
    start_time: Instant,
    is_complete: bool,
}
impl ResultAggregator {
    /// 创建新的结果聚合器
    pub fn new(config: AggregatorConfig) -> Result<Self, String> {
        if config.min_results == 0 {
            return Err("min_results must be greater than 0".to_string());
        }
        Ok(Self {
            config,
            batches: HashMap::new(),
        })
    }
    /// 收集任务结果
    pub fn collect_result(&mut self, result: TaskResult, batch_id: &str) -> Result<(), String> {
        let batch: _ = self.batches.entry(batch_id.to_string()).or_insert_with(|| BatchResults {
            results: Vec::new(),
            start_time: Instant::now(),
            is_complete: false,
        });
        // 检查是否已存在相同任务 ID 的结果
        if batch.results.iter().any(|r| r.task_id == result.task_id) {
            return Err(format!("Result for task '{}' already collected", result.task_id));
        }
        batch.results.push(result);
        // 检查是否达到最小结果数
        if batch.results.len() >= self.config.min_results {
            batch.is_complete = true;
        }
        Ok(())
    }
    /// 检查批量是否完成
    pub fn is_batch_complete(&self, batch_id: &str) -> bool {
        self.batches.get(batch_id)
            .map(|batch| batch.is_complete || batch.results.len() >= self.config.min_results)
            .unwrap_or(false)
    }
    /// 检查批量是否超时
    pub fn check_timeout(&self, batch_id: &str) -> bool {
        if let Some(batch) = self.batches.get(batch_id) {
            batch.start_time.elapsed() > self.config.timeout
        } else {
            false
        }
    }
    /// 获取聚合结果
    pub fn get_aggregated_results(&self, batch_id: &str) -> Option<Vec<TaskResult>> {
        self.batches.get(batch_id).map(|batch| batch.results.clone())
    }
    /// 获取已收集结果数量
    pub fn get_collected_count(&self, batch_id: &str) -> usize {
        self.batches.get(batch_id)
            .map(|batch| batch.results.len())
            .unwrap_or(0)
    }
}