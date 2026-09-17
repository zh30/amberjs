// Amber 性能分析器
// Stage 80 Phase 3 - 开发者工具链
// 支持火焰图生成、内存分析、性能瓶颈检测



use std::collections::{BTreeMap};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use std::time::SystemTime;
/// 函数调用节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallNode {
    pub function_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub call_count: u64,
    pub total_time_ns: u64,
    pub self_time_ns: u64,
    pub children: Vec<CallNode>,
}
/// 调用图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub root: CallNode,
    pub total_calls: u64,
    pub total_time_ns: u64,
}
/// 火焰图数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameGraphNode {
    pub name: String,
    pub value: u64,
    pub level: u32,
    pub children: Vec<FlameGraphNode>,
}
/// 火焰图
#[derive(Debug, Clone)]
pub struct FlameGraph {
    pub root: FlameGraphNode,
    pub total_time_ns: u64,
    pub max_depth: u32,
}
/// 性能采样器
#[derive(Debug)]
pub struct CallGraphSampler {
    samples: Arc<Mutex<Vec<CallSample>>>,
    start_time: Instant,
}
impl Clone for CallGraphSampler {
    fn clone(&self) -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new()))
            start_time: Instant::now(),
        }
    }
}
/// 调用样本
#[derive(Debug, Clone)]
struct CallSample {
    function_name: String,
    file_path: String,
    line_number: u32,
    start_time: Instant,
    end_time: Option<Instant>,
}
/// 性能报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub total_time_ns: u64,
    pub hot_functions: Vec<HotFunction>,
    pub call_graph: CallGraph,
    pub recommendations: Vec<String>,
}
/// 热点函数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotFunction {
    pub function_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub total_time_ns: u64,
    pub call_count: u64,
    pub percentage: f64,
}
/// 堆快照
#[derive(Debug, Clone)]
pub struct HeapSnapshot {
    pub timestamp: u64, // 使用 u64 而不是 Instant，便于序列化
    pub total_size_bytes: usize,
    pub objects: Vec<HeapObject>,
}
/// 堆对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeapObject {
    pub object_type: String,
    pub size_bytes: usize,
    pub retainers: Vec<String>,
}
/// 内存报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryReport {
    pub total_size_bytes: usize,
    pub object_distribution: HashMap<String, usize>,
    pub potential_leaks: Vec<MemoryLeak>,
    pub recommendations: Vec<String>,
}
/// 内存泄漏
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeak {
    pub object_type: String,
    pub count: usize,
    pub size_bytes: usize,
    pub description: String,
}
/// 性能分析器
#[derive(Debug)]
pub struct Profiler {
    pub sampler: Arc<CallGraphSampler>,
    pub call_graph_analyzer: Arc<CallGraphAnalyzer>,
}
impl Profiler {
    /// 创建新的性能分析器
    pub fn new() -> Self {
        Self {
            sampler: Arc::new(Mutex::new(CallGraphSampler::new()))
            call_graph_analyzer: Arc::new(Mutex::new(CallGraphAnalyzer::new()))
        }
    }
    /// 开始采样
    pub fn start_sampling(&self) {
        self.sampler.start();
    }
    /// 停止采样
    pub fn stop_sampling(&self) {
        self.sampler.stop();
    }
    /// 记录函数调用
    pub fn record_call(
        &self,
        function_name: &str,
        file_path: &str,
        line_number: u32,
    ) -> CallHandle {
        self.sampler.record_call_start(function_name, file_path, line_number)
    }
    /// 生成火焰图
    pub async fn generate_flamegraph(
        &self,
        duration: Duration,
    ) -> Result<FlameGraph, Box<dyn std::error::Error + Send + Sync>> {
        self.start_sampling();
        // 等待指定时间
        tokio::time::sleep(duration).await;
        self.stop_sampling();
        // 获取调用样本
        let samples: _ = self.sampler.get_samples();
        let call_graph: _ = self.call_graph_analyzer.analyze_samples(&samples)?;
        // 转换为火焰图
        let flame_graph: _ = self.convert_to_flamegraph(&call_graph);
        Ok(flame_graph)
    }
    /// 分析性能
    pub async fn analyze_performance(
        &self,
        duration: Duration,
    ) -> Result<PerformanceReport, Box<dyn std::error::Error + Send + Sync>> {
        self.start_sampling();
        tokio::time::sleep(duration).await;
        self.stop_sampling();
        let samples: _ = self.sampler.get_samples();
        let call_graph: _ = self.call_graph_analyzer.analyze_samples(&samples)?;
        // 生成热点函数
        let hot_functions: _ = self.call_graph_analyzer.find_hot_functions(&call_graph);
        // 生成建议
        let recommendations: _ = self.generate_recommendations(&hot_functions);
        Ok(PerformanceReport {
            total_time_ns: call_graph.total_time_ns,
            hot_functions,
            call_graph,
            recommendations,
        })
    }
    /// 分析内存
    pub async fn analyze_memory(
        &self,
        heap_snapshot: &HeapSnapshot,
    ) -> Result<MemoryReport, Box<dyn std::error::Error + Send + Sync>> {
        // 分析对象分布
        let mut object_distribution = HashMap::new();
        for obj in &heap_snapshot.objects {
            *object_distribution
                .entry(obj.object_type.clone())
                .or_insert(0) += obj.size_bytes;
        }
        // 检测潜在泄漏
        let potential_leaks: _ = self.detect_memory_leaks(heap_snapshot);
        // 生成建议
        let recommendations: _ = self.generate_memory_recommendations(&potential_leaks);
        Ok(MemoryReport {
            total_size_bytes: heap_snapshot.total_size_bytes,
            object_distribution,
            potential_leaks,
            recommendations,
        })
    }
    /// 转换调用图为火焰图
    fn convert_to_flamegraph(&self, call_graph: &CallGraph) -> FlameGraph {
        let root: _ = self.call_node_to_flamegraph_node(&call_graph.root, 0);
        let max_depth: _ = self.calculate_max_depth(&root);
        FlameGraph {
            root,
            total_time_ns: call_graph.total_time_ns,
            max_depth,
        }
    }
    /// 将调用节点转换为火焰图节点
    fn call_node_to_flamegraph_node(&self, node: &CallNode, level: u32) -> FlameGraphNode {
        FlameGraphNode {
            name: format!("{} ({}ms)", node.function_name, node.total_time_ns / 1_000_000),
            value: node.total_time_ns,
            level,
            children: node
                .children
                .iter()
                .map(|child| self.call_node_to_flamegraph_node(child, level + 1))
                .collect(),
        }
    }
    /// 计算最大深度
    fn calculate_max_depth(&self, node: &FlameGraphNode) -> u32 {
        if node.children.is_empty() {
            node.level
        } else {
            node.children
                .iter()
                .map(|child| self.calculate_max_depth(child))
                .max()
                .unwrap_or(node.level)
        }
    }
    /// 检测内存泄漏
    fn detect_memory_leaks(&self, heap_snapshot: &HeapSnapshot) -> Vec<MemoryLeak> {
        let mut leaks = Vec::new();
        // 简单的泄漏检测：找出大量重复的大对象
        let mut type_counts: HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize), String, (usize, usize), std::collections::HashMap<String, (usize, usize), String, (usize, usize)>> = HashMap::new(); // (count, total_size)
        for obj in &heap_snapshot.objects {
            let entry: _ = type_counts
                .entry(obj.object_type.clone())
                .or_insert((0, 0));
            entry.0 += 1;
            entry.1 += obj.size_bytes;
        }
        for (object_type, (count, size_bytes)) in type_counts {
            // 如果某个类型的对象数量过多或总大小过大，可能是泄漏
            if count > 1000 || size_bytes > 10 * 1024 * 1024 {
                leaks.push(MemoryLeak {
                    object_type: object_type.clone(),
                    count,
                    size_bytes,
                    description: format!(
                        "对象类型 '{}' 数量过多 ({} 个) 或大小过大 ({} MB)",
                        object_type,
                        count,
                        size_bytes / (1024 * 1024)
                    ),
                });
            }
        }
        leaks
    }
    /// 生成性能优化建议
    fn generate_recommendations(&self, hot_functions: &[HotFunction]) -> Vec<String> {
        let mut recommendations = Vec::new();
        for hot_func in hot_functions.iter().take(5) {
            if hot_func.percentage > 20.0 {
                recommendations.push(format!(
                    "函数 '{}' 占用 {}% 的执行时间，建议优化 ({}:{})",
                    hot_func.function_name,
                    hot_func.percentage,
                    hot_func.file_path,
                    hot_func.line_number
                ));
            }
        }
        if recommendations.is_empty() {
            recommendations.push("性能表现良好，未发现明显瓶颈".to_string());
        }
        recommendations
    }
    /// 生成内存优化建议
    fn generate_memory_recommendations(&self, leaks: &[MemoryLeak]) -> Vec<String> {
        let mut recommendations = Vec::new();
        for leak in leaks {
            recommendations.push(format!(
                "检测到潜在内存泄漏: {} - {}",
                leak.description,
                if leak.size_bytes > 50 * 1024 * 1024 {
                    "建议立即修复"
                } else {
                    "建议监控"
                }
            ));
        }
        if recommendations.is_empty() {
            recommendations.push("内存使用正常，未检测到明显泄漏".to_string());
        }
        recommendations
    }
}
/// 调用句柄
pub struct CallHandle {
    sampler: Arc<CallGraphSampler>,
    function_name: String,
    file_path: String,
    line_number: u32,
    start_time: Instant,
}
impl CallHandle {
    /// 结束调用
    pub fn end(self) {
        self.sampler.record_call_end(
            &self.function_name,
            &self.file_path,
            self.line_number,
            self.start_time,
        );
    }
}
impl Drop for CallHandle {
    fn drop(&mut self) {
        self.sampler.record_call_end(
            &self.function_name,
            &self.file_path,
            self.line_number,
            self.start_time,
        );
    }
}
/// 调用图分析器
#[derive(Debug)]
pub struct CallGraphAnalyzer {
    // 分析器状态
}
impl CallGraphAnalyzer {
    /// 创建新的分析器
    pub fn new() -> Self {
        Self {}
    }
    /// 分析样本并构建调用图
    pub fn analyze_samples(&self, samples: &[CallSample]) -> Result<CallGraph, Box<dyn std::error::Error + Send + Sync>> {
        let mut call_tree: HashMap<String, CallNode> = HashMap::new();
        // 聚合调用数据
        for sample in samples {
            if let Some(end_time) = sample.end_time {
                let duration: _ = end_time.duration_since(sample.start_time).as_nanos() as u64;
                let node: _ = call_tree
                    .entry(sample.function_name.clone())
                    .or_insert(CallNode {
                        function_name: sample.function_name.clone(),
                        file_path: sample.file_path.clone(),
                        line_number: sample.line_number,
                        call_count: 0,
                        total_time_ns: 0,
                        self_time_ns: 0,
                        children: Vec::new(),
                    });
                node.call_count += 1;
                node.total_time_ns += duration;
            }
        }
        // 计算自调用时间
        for node in call_tree.values_mut() {
            let mut child_time = 0;
            // 这里简化处理，实际应该根据调用栈计算
            node.self_time_ns = node.total_time_ns.saturating_sub(child_time);
        }
        // 构建根节点
        let root: _ = CallNode {
            function_name: "<root>".to_string(),
            file_path: "".to_string(),
            line_number: 0,
            call_count: call_tree.values().map(|n| n.call_count).sum(),
            total_time_ns: call_tree.values().map(|n| n.total_time_ns).sum(),
            self_time_ns: 0,
            children: call_tree.values().cloned().collect(),
        };
        let root_call_count: _ = root.call_count;
        let root_total_time: _ = root.total_time_ns;
        Ok(CallGraph {
            root,
            total_calls: root_call_count,
            total_time_ns: root_total_time,
        })
    }
    /// 查找热点函数
    pub fn find_hot_functions(&self, call_graph: &CallGraph) -> Vec<HotFunction> {
        let mut hot_functions = Vec::new();
        let total_time: _ = call_graph.total_time_ns;
        for child in &call_graph.root.children {
            let percentage: _ = (child.total_time_ns as f64 / total_time as f64) * 100.0;
            hot_functions.push(HotFunction {
                function_name: child.function_name.clone(),
                file_path: child.file_path.clone(),
                line_number: child.line_number,
                total_time_ns: child.total_time_ns,
                call_count: child.call_count,
                percentage,
            });
        }
        // 按占用时间排序
        hot_functions.sort_by(|a, b| b.total_time_ns.cmp(&a.total_time_ns));
        hot_functions
    }
}
impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}
impl CallGraphSampler {
    /// 创建新的采样器
    fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new()))
            start_time: Instant::now(),
        }
    }
    /// 开始采样
    fn start(&self) {
        let mut samples = self.samples.lock().unwrap();
        samples.clear();
    }
    /// 停止采样
    fn stop(&self) {
        // 采样已在后台进行，无需特殊操作
    }
    /// 记录调用开始
    fn record_call_start(
        &self,
        function_name: &str,
        file_path: &str,
        line_number: u32,
    ) -> CallHandle {
        CallHandle {
            sampler: Arc::new(Mutex::new(self.clone()))
            function_name: function_name.to_string(),
            file_path: file_path.to_string(),
            line_number,
            start_time: Instant::now(),
        }
    }
    /// 记录调用结束
    fn record_call_end(
        &self,
        function_name: &str,
        file_path: &str,
        line_number: u32,
        start_time: Instant,
    ) {
        let mut samples = self.samples.lock().unwrap();
        samples.push(CallSample {
            function_name: function_name.to_string(),
            file_path: file_path.to_string(),
            line_number,
            start_time,
            end_time: Some(Instant::now()),
        });
    }
    /// 获取所有样本
    fn get_samples(&self) -> Vec<CallSample> {
        let samples: _ = self.samples.lock().unwrap();
        samples.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
use std::time::{Duration, Instant};
    #[tokio::test]
    async fn test_flamegraph_generation() {
        let profiler: _ = Profiler::new();
        // 模拟一些函数调用
        {
            let _handle1: _ = profiler.record_call("func_a", "test.js", 10);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        {
            let _handle2: _ = profiler.record_call("func_b", "test.js", 20);
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        // 生成火焰图
        let flamegraph: _ = profiler.generate_flamegraph(Duration::from_millis(100)).await.unwrap();
        assert!(flamegraph.total_time_ns > 0);
        assert!(flamegraph.max_depth >= 0);
    }
    #[tokio::test]
    async fn test_memory_leak_detection() {
        let profiler: _ = Profiler::new();
        // 创建堆快照
        let mut objects = Vec::new();
        for i in 0..1500 {
            objects.push(HeapObject {
                object_type: "LargeArray".to_string(),
                size_bytes: 1024,
                retainers: vec!["global".to_string()],
            });
        }
        let heap_snapshot: _ = HeapSnapshot {
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            total_size_bytes: objects.len() * 1024,
            objects,
        };
        let memory_report: _ = profiler.analyze_memory(&heap_snapshot).await.unwrap();
        // 应该检测到潜在泄漏
        assert!(!memory_report.potential_leaks.is_empty());
        assert!(memory_report.recommendations.len() > 0);
    }
    #[tokio::test]
    async fn test_performance_analysis() {
        let profiler: _ = Profiler::new();
        // 执行一些性能测试
        {
            let _handle: _ = profiler.record_call("test_function", "test.js", 1);
            for i in 0..1000 {
                let _: _ = i * i;
            }
        }
        let report: _ = profiler.analyze_performance(Duration::from_millis(50)).await.unwrap();
        assert!(report.total_time_ns > 0);
        assert!(!report.hot_functions.is_empty());
        assert!(!report.recommendations.is_empty());
    }
}