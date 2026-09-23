// Amber 网络 I/O 优化模块
// Stage 30.3: 网络 I/O 零拷贝优化
//
// 提供高性能网络 I/O 功能，包括 epoll 事件驱动、零拷贝传输、批处理等
pub mod epoll_manager;
pub mod zero_copy_io;
pub mod batch_processor;
pub mod buffer_pool;
pub mod statistics;
pub mod sendfile;  // Stage 39.0: sendfile 系统调用
pub mod splice;  // Stage 39.0: splice 系统调用
pub mod tcp_socket;  // TCP 套接字
pub mod udp_socket;  // UDP 套接字
pub mod connection_pool;  // 连接池
pub mod http2_server;  // HTTP/2 服务器
pub mod http3_server;  // HTTP/3 服务器
pub mod zero_copy;  // Stage 39.0: 零拷贝 I/O 优化
pub mod memory_mapper;  // Stage 39.0: 内存映射管理器
// Stage 92 Phase 3: 网络 I/O 极致优化
pub mod zero_copy_network;
pub mod batch_io;
pub mod async_zero_copy;
pub mod network_buffer;
pub mod io_uring;
// Stage 93 Phase 1.3: 网络极致优化
pub mod stage93_intelligent_prefetch;
pub mod stage93_network_topology;
// 重新导出主要类型

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// 扩展类型
// 网络缓冲区和统计类型
// Stage 92 Phase 3: 导出优化模块
pub use zero_copy_network::{
    ZeroCopySocket, ZeroCopyListener, ZeroCopyStream,
    ZeroCopyConfig, NetworkZeroCopyStats
};
pub use batch_io::{
    BatchIoEngine, BatchOperation, BatchConfig,
    BatchStats, BatchPriority
};
pub use async_zero_copy::{
    AsyncZeroCopy, ZeroCopyError, TransferRequest,
    TransferStats, ZeroCopyFuture
};
pub use network_buffer::{
    NetworkBuffer, BufferPool, BufferConfig,
    BufferStats, BufferType
};
pub use io_uring::{
    IoUringEngine, UringSubmission, UringCompletion,
    UringConfig, UringStats
};
// Stage 93 Phase 1.3: 导出优化组件
pub use stage93_intelligent_prefetch::{
    Stage93IntelligentPrefetcher, PrefetchConfig, PrefetchStats,
    AccessPattern, AIPrefetchPredictor
};
pub use stage93_network_topology::{
    Stage93TopologyDiscoverer, TopologyConfig, NetworkTopology,
    NetworkNode, NetworkPath, NodeType
};
/// 网络配置
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// 最大连接数
    pub max_connections: usize,
    /// 批处理大小
    pub batch_size: usize,
    /// 批处理超时
    pub batch_timeout: Duration,
    /// 最大缓冲区大小
    pub max_buffer_size: usize,
    /// 启用 HTTP/2
    pub enable_http2: bool,
    /// 启用 HTTP/3
    pub enable_http3: bool,
    /// 连接池大小
    pub pool_size: usize,
    /// UDP 缓冲区大小
    pub udp_buffer_size: usize,
}
impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_connections: 10000,
            batch_size: 100,
            batch_timeout: Duration::from_millis(10),
            max_buffer_size: 64 * 1024,
            enable_http2: false,
            enable_http3: false,
            pool_size: 100,
            udp_buffer_size: 32 * 1024,
        }
    }
}
/// 网络统计信息
#[derive(Debug, Clone)]
pub struct NetworkStats {
    /// 总连接数
    pub total_connections: usize,
    /// 活跃连接数
    pub active_connections: usize,
    /// 零拷贝操作次数
    pub zero_copy_operations: usize,
    /// 总发送字节数
    pub total_bytes_sent: u64,
    /// 总接收字节数
    pub total_bytes_received: u64,
    /// 批处理次数
    pub batch_operations: usize,
    /// 平均延迟 (微秒)
    pub average_latency_us: u64,
    /// 内存使用 (字节)
    pub memory_usage: usize,
}
/// 网络事件类型
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkEvent {
    /// 新连接
    NewConnection,
    /// 数据接收
    DataReceived,
    /// 连接关闭
    ConnectionClosed,
    /// 错误
    Error(String),
}
/// 网络错误类型
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("超时错误")]
    Timeout,
    #[error("连接错误: {0}")]
    Connection(String),
    #[error("协议错误: {0}")]
    Protocol(String),
    #[error("资源不足")]
    ResourceExhausted,
}
/// 网络事件处理程序
pub trait NetworkEventHandler: Send + Sync {
    /// 处理网络事件
    fn handle_event(&self, event: NetworkEvent) -> Result<(), NetworkError>;
}
/// 网络性能监控器
pub struct NetworkMonitor {
    stats: std::sync::Arc<std::sync::Mutex<NetworkStats>>,
}
impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            stats: std::sync::Arc::new(std::sync::Mutex::new(NetworkStats {
                total_connections: 0,
                active_connections: 0,
                zero_copy_operations: 0,
                total_bytes_sent: 0,
                total_bytes_received: 0,
                batch_operations: 0,
                average_latency_us: 0,
                memory_usage: 0,
            })),
        }
    }
    /// 更新统计信息
    pub fn update_stats(&self, updater: fn(&mut NetworkStats)) {
        if let Ok(mut stats) = self.stats.lock() {
            updater(&mut stats);
        }
    }
    /// 获取统计信息快照
    pub fn get_stats_snapshot(&self) -> NetworkStats {
        self.stats.lock().unwrap().clone()
    }
}
impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}