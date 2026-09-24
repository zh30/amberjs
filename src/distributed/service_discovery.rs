// 服务发现模块
// 实现基于 Gossip 协议的集群节点自动发现和注册

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use super::node_manager::{NodeMetadata, NodeStatus};
use tokio::time::interval;
use tracing::{debug, info, warn};
use std::time::{Duration, Instant};
use std::time::SystemTime;
use std::hash::Hash;

/// 服务发现配置
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub cluster_name: String,
    pub gossip_interval: Duration,
    pub node_timeout: Duration,
}
/// 节点信息
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeInfo {
    pub id: String,
    pub address: String,
    pub cpu_cores: usize,
    pub memory_gb: usize,
    pub location: String,
    pub capabilities: Vec<String>,
}
/// Gossip 消息
#[derive(Debug, Clone)]
pub struct GossipMessage {
    pub cluster_name: String,
    pub node_id: String,
    pub node_info: NodeInfo,
    pub timestamp: u64, // 使用 u64 而不是 Instant，便于序列化
}
/// 服务发现实现
#[derive(Debug, Clone)]
pub struct ServiceDiscovery {
    config: DiscoveryConfig,
    nodes: Arc<RwLock<HashMap<String, NodeMetadata>>>,
    gossip_history: Arc<RwLock<Vec<GossipMessage>>>,
}
impl ServiceDiscovery {
    /// 创建新的服务发现实例
    pub fn new(config: DiscoveryConfig) -> Self {
        let discovery: _ = Self {
            config: config.clone(),
            nodes: Arc::new(Mutex::new(HashMap::new())),
            gossip_history: Arc::new(Mutex::new(Vec::new())),
        };
        // 启动后台 gossip 任务
        let config_clone: _ = config.clone();
        let nodes_clone: _ = discovery.nodes.clone();
        let gossip_history_clone: _ = discovery.gossip_history.clone();
        tokio::spawn(async move {
            let mut interval = interval(config_clone.gossip_interval);
            loop {
                interval.tick().await;
                Self::gossip_protocol(
                    nodes_clone.clone(),
                    gossip_history_clone.clone(),
                    &config_clone,
                ).await;
            }
        });
        discovery
    }
    /// 注册当前节点
    pub async fn register_self(&self, node_info: NodeInfo) {
        let mut nodes = self.nodes.write().await;
        let metadata: _ = NodeMetadata {
            cpu_cores: node_info.cpu_cores,
            memory_gb: node_info.memory_gb,
            location: node_info.location.clone(),
            capabilities: node_info.capabilities.clone(),
            status: NodeStatus::Online,
            registered_at: Instant::now(),
            last_heartbeat: Instant::now(),
            version: 1,
        };
        nodes.insert(node_info.id.clone(), metadata);
        info!("Registered self node: {}", node_info.id);
        // 广播注册消息
        self.broadcast_gossip(&node_info).await;
    }
    /// 更新节点信息
    pub async fn update_node(&self, node_info: NodeInfo) -> Result<(), String> {
        let mut nodes = self.nodes.write().await;
        if let Some(metadata) = nodes.get_mut(&node_info.id) {
            metadata.last_heartbeat = Instant::now();
            metadata.version += 1;
            info!("Updated node: {}", node_info.id);
        } else {
            // 新节点注册
            let metadata: _ = NodeMetadata {
                cpu_cores: node_info.cpu_cores,
                memory_gb: node_info.memory_gb,
                location: node_info.location.clone(),
                capabilities: node_info.capabilities.clone(),
                status: NodeStatus::Online,
                registered_at: Instant::now(),
                last_heartbeat: Instant::now(),
                version: 1,
            };
            nodes.insert(node_info.id.clone(), metadata);
            info!("Registered new node: {}", node_info.id);
        }
        self.broadcast_gossip(&node_info).await;
        Ok(())
    }
    /// 获取已知节点列表
    pub async fn get_known_nodes(&self) -> Vec<NodeInfo> {
        let nodes: _ = self.nodes.read().await;
        let known_nodes: _ = Vec::new();
        // 清理超时节点
        let now: _ = Instant::now();
        let timeout_nodes: Vec<String> = nodes
            .iter()
            .filter(|(_, metadata)| now.duration_since(metadata.last_heartbeat) > self.config.node_timeout)
            .map(|(id, _)| id.clone())
            .collect();
        for timeout_node in timeout_nodes {
            warn!("Node timeout: {}", timeout_node);
        }
        // 提取活跃节点信息
        for (id, metadata) in nodes.iter() {
            if now.duration_since(metadata.last_heartbeat) <= self.config.node_timeout {
                // 注意：这里需要从 gossip_history 中重建 node_info
                // 简化实现，实际应该维护完整的节点信息
                debug!("Active node: {}", id);
            }
        }
        known_nodes
    }
    /// Gossip 协议实现
    async fn gossip_protocol(
        nodes: Arc<RwLock<HashMap<String, NodeMetadata>>>,
        _gossip_history: Arc<RwLock<Vec<GossipMessage>>>,
        _config: &DiscoveryConfig,
    ) {
        let known_nodes: _ = {
            let nodes_guard: _ = nodes.read().await;
            nodes_guard.keys().cloned().collect::<Vec<_>>()
        };
        if known_nodes.len() < 2 {
            return; // 需要至少 2 个节点才能进行 gossip
        }
        // 随机选择 gossip 目标
        
        let mut rng = rand::rng();
        let target_count: _ = (known_nodes.len() / 3).max(1);
        for _ in 0..target_count {
            if let Some(target) = known_nodes.iter().choose(&mut rng) {
                debug!("Gossiping with node: {}", target);
                // 实际实现中这里会发送网络消息
                // 简化实现
            }
        }
    }
    /// 广播 Gossip 消息
    async fn broadcast_gossip(&self, node_info: &NodeInfo) {
        let message: _ = GossipMessage {
            cluster_name: self.config.cluster_name.clone(),
            node_id: node_info.id.clone(),
            node_info: node_info.clone(),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        };
        let mut history = self.gossip_history.write().await;
        history.push(message);
        // 保持历史记录在合理大小
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }
    /// 接收 Gossip 消息
    pub async fn receive_gossip(&self, message: GossipMessage) {
        if message.cluster_name != self.config.cluster_name {
            return; // 忽略不同集群的消息
        }
        let node_id: _ = message.node_id.clone();
        let mut nodes = self.nodes.write().await;
        let metadata: _ = NodeMetadata {
            cpu_cores: message.node_info.cpu_cores,
            memory_gb: message.node_info.memory_gb,
            location: message.node_info.location.clone(),
            capabilities: message.node_info.capabilities.clone(),
            status: NodeStatus::Online,
            registered_at: Instant::now(),
            last_heartbeat: Instant::now(),
            version: 1,
        };
        nodes.insert(node_id.clone(), metadata);
        debug!("Received gossip from: {}", node_id);
    }
    /// 清理离线节点
    pub async fn cleanup_offline_nodes(&self) -> usize {
        let mut nodes = self.nodes.write().await;
        let now: _ = Instant::now();
        let offline_nodes: Vec<String> = nodes
            .iter()
            .filter(|(_, metadata)| now.duration_since(metadata.last_heartbeat) > self.config.node_timeout)
            .map(|(id, _)| id.clone())
            .collect();
        for node_id in &offline_nodes {
            nodes.remove(node_id);
            warn!("Removed offline node: {}", node_id);
        }
        offline_nodes.len()
    }
    /// 获取集群统计信息
    pub async fn get_cluster_stats(&self) -> ClusterStats {
        let nodes: _ = self.nodes.read().await;
        let now: _ = Instant::now();
        let (online, offline, total) = {
            let mut online_count = 0;
            let mut offline_count = 0;
            for (_, metadata) in nodes.iter() {
                if now.duration_since(metadata.last_heartbeat) <= self.config.node_timeout {
                    online_count += 1;
                } else {
                    offline_count += 1;
                }
            }
            (online_count, offline_count, nodes.len())
        };
        ClusterStats {
            total_nodes: total,
            online_nodes: online,
            offline_nodes: offline,
            cluster_name: self.config.cluster_name.clone(),
        }
    }
}
/// 集群统计信息
#[derive(Debug, Clone)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub offline_nodes: usize,
    pub cluster_name: String,
}
impl ClusterStats {
    /// 检查集群是否健康
    pub fn is_healthy(&self) -> bool {
        self.online_nodes > 0 && (self.offline_nodes as f64 / self.total_nodes as f64) < 0.5
    }
    /// 获取集群可用性百分比
    pub fn availability_percentage(&self) -> f64 {
        if self.total_nodes == 0 {
            return 0.0;
        }
        (self.online_nodes as f64 / self.total_nodes as f64) * 100.0
    }
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_service_discovery_registration() {
        let config: _ = DiscoveryConfig {
            cluster_name: "test-cluster".to_string(),
            gossip_interval: Duration::from_millis(100),
            node_timeout: Duration::from_secs(5),
        };
        let discovery: _ = ServiceDiscovery::new(config);
        let node: _ = NodeInfo {
            id: "test-node".to_string(),
            address: "192.168.1.1:8080".to_string(),
            cpu_cores: 4,
            memory_gb: 8,
            location: "us-west".to_string(),
            capabilities: vec!["js-execution".to_string()],
        };
        discovery.register_self(node.clone()).await;
        // 等待 gossip 传播
        tokio::time::sleep(Duration::from_millis(200)).await;
        let stats: _ = discovery.get_cluster_stats().await;
        assert_eq!(stats.total_nodes, 1);
        assert_eq!(stats.online_nodes, 1);
    }
    #[tokio::test]
    async fn test_node_cleanup() {
        let config: _ = DiscoveryConfig {
            cluster_name: "test-cluster".to_string(),
            gossip_interval: Duration::from_millis(100),
            node_timeout: Duration::from_millis(300),
        };
        let discovery: _ = ServiceDiscovery::new(config);
        let node: _ = NodeInfo {
            id: "test-node".to_string(),
            address: "192.168.1.1:8080".to_string(),
            cpu_cores: 4,
            memory_gb: 8,
            location: "us-west".to_string(),
            capabilities: vec!["js-execution".to_string()],
        };
        discovery.register_self(node).await;
        // 等待超时
        tokio::time::sleep(Duration::from_millis(400)).await;
        let cleaned_count: _ = discovery.cleanup_offline_nodes().await;
        assert_eq!(cleaned_count, 1);
        let stats: _ = discovery.get_cluster_stats().await;
        assert_eq!(stats.total_nodes, 0);
    }
}