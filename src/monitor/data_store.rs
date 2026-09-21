// 数据存储模块
// 负责高效存储和查询时序性能数据

use crate::monitor::performance_monitor::{MetricType, MetricValue};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::time::{Duration, Instant};

/// 导出格式
#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
    Prometheus,
}
/// 查询条件
#[derive(Debug, Clone)]
pub struct QueryCondition {
    /// 指标类型
    pub metric_type: Option<MetricType>,
    /// 开始时间
    pub start_time: Option<u64>,
    /// 结束时间
    pub end_time: Option<u64>,
    /// 标签过滤
    pub tag_filters: HashMap<String, _>,
    /// 限制返回数量
    pub limit: Option<usize>,
}
/// 数据点
#[derive(Debug, Clone)]
pub struct DataPoint {
    /// 指标值
    pub value: MetricValue,
    /// 压缩后的大小 (字节)
    pub compressed_size: Option<usize>,
}
/// 数据存储配置
#[derive(Debug, Clone)]
pub struct DataStoreConfig {
    /// 最大内存使用量 (字节)
    pub max_memory_bytes: usize,
    /// 数据压缩阈值
    pub compression_threshold: usize,
    /// 保留期
    pub retention_period: Duration,
    /// 自动清理间隔
    pub cleanup_interval: Duration,
    /// 压缩级别 (0-9)
    pub compression_level: u32,
}
/// 数据存储管理器
#[derive(Debug)]
pub struct DataStore {
    /// 配置
    config: DataStoreConfig,
    /// 内存数据缓存
    memory_cache: Arc<Mutex<VecDeque<DataPoint>>>,
    /// 压缩数据存储
    compressed_storage: Arc<Mutex<HashMap<String, _>>>,
    /// 查询索引
    query_index: Arc<Mutex<QueryIndex>>,
    /// 统计信息
    stats: Arc<Mutex<DataStoreStats>>,
}
/// 压缩数据块
#[derive(Debug, Clone)]
pub struct CompressedData {
    /// 指标类型
    pub metric_type: MetricType,
    /// 压缩数据
    pub data: Vec<u8>,
    /// 原始大小
    pub original_size: usize,
    /// 压缩后大小
    pub compressed_size: usize,
    /// 时间范围
    pub time_range: (u64, u64),
}
/// 查询索引
#[derive(Debug, Clone)]
pub struct QueryIndex {
    /// 按时间排序的索引
    pub time_index: Vec<(u64, MetricType)>,
    /// 按指标类型分组的索引
    pub type_index: HashMap<String, _>,
    /// 索引最后更新时间
    pub last_update: Instant,
}
/// 数据存储统计
#[derive(Debug, Clone)]
pub struct DataStoreStats {
    /// 总存储数据点数
    pub total_data_points: u64,
    /// 内存缓存大小
    pub memory_cache_size: usize,
    /// 压缩存储大小
    pub compressed_storage_size: usize,
    /// 查询次数
    pub query_count: u64,
    /// 压缩次数
    pub compression_count: u64,
    /// 导出次数
    pub export_count: u64,
    /// 最后清理时间
    pub last_cleanup: Option<Instant>,
    /// 磁盘使用量 (字节)
    pub disk_usage_bytes: u64,
}
impl DataStore {
    /// 创建新的数据存储实例
    pub fn new(config: DataStoreConfig) -> Self {
        Self {
            config,
            memory_cache: Arc::new(Mutex::new(VecDeque::new())),
            compressed_storage: Arc::new(Mutex::new(HashMap::new())),
            query_index: Arc::new(Mutex::new(Mutex::new(std::sync::Mutex::new(Mutex::new(
                QueryIndex {
                    time_index: Vec::new(),
                    type_index: HashMap::new(),
                    last_update: Instant::now(),
                },
            ))))),
            stats: Arc::new(Mutex::new(DataStoreStats {
                total_data_points: 0,
                memory_cache_size: 0,
                compressed_storage_size: 0,
                query_count: 0,
                compression_count: 0,
                export_count: 0,
                last_cleanup: None,
                disk_usage_bytes: 0,
            })),
        }
    }
    /// 创建默认配置的数据存储
    pub fn with_default_config() -> Self {
        let config: _ = DataStoreConfig {
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            compression_threshold: 1000,
            retention_period: Duration::from_secs(86400), // 24小时
            cleanup_interval: Duration::from_secs(3600),  // 1小时
            compression_level: 6,
        };
        Self::new(config)
    }
    /// 存储数据点
    pub fn store(&self, data_point: DataPoint) -> Result<(), String> {
        let mut memory_cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let mut stats = self.stats.lock().map_err(|e| e.to_string())?;
        // 添加到内存缓存
        memory_cache.push_back(data_point.clone());
        stats.total_data_points += 1;
        stats.memory_cache_size += std::mem::size_of::<DataPoint>();
        // 检查是否需要压缩
        if memory_cache.len() >= self.config.compression_threshold {
            self.compress_data()?;
        }
        // 检查内存限制
        self.enforce_memory_limit()?;
        // 更新索引
        self.update_index(&data_point.value)?;
        Ok(())
    }
    /// 批量存储数据点
    pub fn store_batch(&self, data_points: Vec<DataPoint>) -> Result<(), String> {
        for data_point in data_points {
            self.store(data_point)?;
        }
        Ok(())
    }
    /// 查询数据
    pub fn query(&self, condition: QueryCondition) -> Result<Vec<MetricValue>, String> {
        let mut stats = self.stats.lock().map_err(|e| e.to_string())?;
        stats.query_count += 1;
        let memory_cache: _ = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let compressed_storage: _ = self.compressed_storage.lock().map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        // 从内存缓存查询
        for data_point in memory_cache.iter() {
            if self.matches_condition(&data_point.value, &condition) {
                results.push(data_point.value.clone());
            }
        }
        // 从压缩存储查询
        for compressed_data in compressed_storage.values() {
            if self.matches_compressed_condition(compressed_data, &condition) {
                let decompressed: _ = self.decompress_data(compressed_data)?;
                for data_point in decompressed {
                    if self.matches_condition(&data_point.value, &condition) {
                        results.push(data_point.value.clone());
                    }
                }
            }
        }
        // 应用限制
        if let Some(limit) = condition.limit {
            if results.len() > limit {
                results.truncate(limit);
            }
        }
        Ok(results)
    }
    /// 压缩数据
    fn compress_data(&self) -> Result<(), String> {
        let mut memory_cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let mut compressed_storage = self.compressed_storage.lock().map_err(|e| e.to_string())?;
        let mut stats = self.stats.lock().map_err(|e| e.to_string())?;
        if memory_cache.len() < self.config.compression_threshold {
            return Ok(());
        }
        // 按指标类型分组
        let mut grouped_data: HashMap<String, _> = HashMap::new();
        while let Some(data_point) = memory_cache.pop_front() {
            grouped_data
                .entry(data_point.value.metric_type.clone())
                .or_insert_with(Vec::new)
                .push(data_point);
        }
        // 压缩每个组
        for (metric_type, data_points) in grouped_data {
            if data_points.len() < self.config.compression_threshold {
                // 放回内存缓存
                for data_point in data_points {
                    memory_cache.push_back(data_point);
                }
                continue;
            }
            let compressed: _ = self.compress_data_points(&data_points)?;
            let key: _ = Self::generate_storage_key(&metric_type);
            compressed_storage.insert(key, compressed);
            stats.compression_count += 1;
            stats.memory_cache_size -= data_points.len() * std::mem::size_of::<DataPoint>();
        }
        Ok(())
    }
    /// 压缩数据点
    fn compress_data_points(&self, data_points: &[DataPoint]) -> Result<CompressedData, String> {
        if data_points.is_empty() {
            return Err("Empty data points".to_string());
        }
        let metric_type: _ = data_points[0].value.metric_type.clone();
        let time_range: _ = (
            data_points
                .iter()
                .map(|dp| dp.value.timestamp)
                .min()
                .unwrap(),
            data_points
                .iter()
                .map(|dp| dp.value.timestamp)
                .max()
                .unwrap(),
        );
        // 序列化数据
        let mut serialized = Vec::new();
        for data_point in data_points {
            let json: _ = serde_json::to_string(&data_point.value)
                .map_err(|e| format!("JSON serialization failed: {}", e))?;
            serialized.extend_from_slice(json.as_bytes());
            serialized.push(b'\n');
        }
        // 使用简单的 RLE 压缩（实际生产中应使用专业压缩库）
        let compressed: _ = Self::rle_compress(&serialized);
        let compressed_size: _ = compressed.len();
        Ok(CompressedData {
            metric_type,
            data: compressed,
            original_size: serialized.len(),
            compressed_size,
            time_range,
        })
    }
    /// RLE 压缩算法
    fn rle_compress(data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }
        let mut compressed = Vec::new();
        let mut current_byte = data[0];
        let mut count = 1;
        for &byte in &data[1..] {
            if byte == current_byte && count < 255 {
                count += 1;
            } else {
                compressed.push(count);
                compressed.push(current_byte);
                current_byte = byte;
                count = 1;
            }
        }
        compressed.push(count);
        compressed.push(current_byte);
        compressed
    }
    /// 解压数据
    fn decompress_data(&self, compressed_data: &CompressedData) -> Result<Vec<DataPoint>, String> {
        let decompressed: _ = Self::rle_decompress(&compressed_data.data)?;
        let mut data_points = Vec::new();
        for line in decompressed.split(|&b| b == b'\n') {
            if line.is_empty() {
                continue;
            }
            let metric_value: MetricValue = serde_json::from_slice(line)
                .map_err(|e| format!("JSON deserialization failed: {}", e))?;
            data_points.push(DataPoint {
                value: metric_value,
                compressed_size: Some(compressed_data.compressed_size),
            });
        }
        Ok(data_points)
    }
    /// RLE 解压算法
    fn rle_decompress(compressed: &[u8]) -> Result<Vec<u8>, String> {
        if compressed.len() % 2 != 0 {
            return Err("Invalid compressed data length".to_string());
        }
        let mut decompressed = Vec::new();
        let mut i = 0;
        while i < compressed.len() {
            let count: _ = compressed[i] as usize;
            let byte: _ = compressed[i + 1];
            if count == 0 {
                return Err("Invalid RLE count".to_string());
            }
            decompressed.extend(std::iter::repeat(byte).take(count));
            i += 2;
        }
        Ok(decompressed)
    }
    /// 生成存储键
    fn generate_storage_key(metric_type: &MetricType) -> String {
        format!("metric_{:?}", metric_type)
    }
    /// 检查条件匹配
    fn matches_condition(&self, metric: &MetricValue, condition: &QueryCondition) -> bool {
        if let Some(ref metric_type) = condition.metric_type {
            if &metric.metric_type != metric_type {
                return false;
            }
        }
        if let Some(start_time) = condition.start_time {
            if metric.timestamp < start_time {
                return false;
            }
        }
        if let Some(end_time) = condition.end_time {
            if metric.timestamp > end_time {
                return false;
            }
        }
        for (key, value) in &condition.tag_filters {
            if metric.tags.get(key) != Some(value) {
                return false;
            }
        }
        true
    }
    /// 检查压缩数据条件匹配
    fn matches_compressed_condition(
        &self,
        compressed: &CompressedData,
        condition: &QueryCondition,
    ) -> bool {
        if let Some(ref metric_type) = condition.metric_type {
            if &compressed.metric_type != metric_type {
                return false;
            }
        }
        if let Some(start_time) = condition.start_time {
            if compressed.time_range.1 < start_time {
                return false;
            }
        }
        if let Some(end_time) = condition.end_time {
            if compressed.time_range.0 > end_time {
                return false;
            }
        }
        true
    }
    /// 更新查询索引
    fn update_index(&self, metric: &MetricValue) -> Result<(), String> {
        let mut query_index = self.query_index.lock().map_err(|e| e.to_string())?;
        query_index
            .time_index
            .push((metric.timestamp, metric.metric_type.clone()));
        query_index
            .type_index
            .entry(metric.metric_type.clone())
            .or_insert_with(Vec::new)
            .push(metric.timestamp);
        query_index.last_update = Instant::now();
        Ok(())
    }
    /// 强制内存限制
    fn enforce_memory_limit(&self) -> Result<(), String> {
        let mut memory_cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let mut stats = self.stats.lock().map_err(|e| e.to_string())?;
        while stats.memory_cache_size > self.config.max_memory_bytes {
            if let Some(data_point) = memory_cache.pop_front() {
                stats.memory_cache_size -= std::mem::size_of::<DataPoint>();
            } else {
                break;
            }
        }
        Ok(())
    }
    /// 导出数据
    pub fn export(
        &self,
        condition: QueryCondition,
        format: ExportFormat,
    ) -> Result<String, String> {
        let mut stats = self.stats.lock().map_err(|e| e.to_string())?;
        stats.export_count += 1;
        let data: _ = self.query(condition)?;
        match format {
            ExportFormat::Json => self.export_json(&data),
            ExportFormat::Csv => self.export_csv(&data),
            ExportFormat::Prometheus => self.export_prometheus(&data),
        }
    }
    /// 导出为 JSON 格式
    fn export_json(&self, data: &[MetricValue]) -> Result<String, String> {
        let json: _ =
            serde_json::to_string(data).map_err(|e| format!("JSON export failed: {}", e))?;
        Ok(json)
    }
    /// 导出为 CSV 格式
    fn export_csv(&self, data: &[MetricValue]) -> Result<String, String> {
        let mut csv = String::new();
        csv.push_str("timestamp,metric_type,value,tags\n");
        for metric in data {
            let tags_str: _ = metric
                .tags
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(",");
            csv.push_str(&format!(
                "{},{:?},{},{}\n",
                metric.timestamp, metric.metric_type, metric.value, tags_str
            ));
        }
        Ok(csv)
    }
    /// 导出为 Prometheus 格式
    fn export_prometheus(&self, data: &[MetricValue]) -> Result<String, String> {
        let mut prometheus = String::new();
        for metric in data {
            let metric_name: _ = match &metric.metric_type {
                MetricType::CpuUsage => "amberjs_cpu_usage_percent",
                MetricType::MemoryUsage => "amberjs_memory_usage_bytes",
                MetricType::HeapMemory => "amberjs_heap_memory_bytes",
                MetricType::ExecutionTime => "amberjs_execution_time_microseconds",
                MetricType::ConcurrentTasks => "amberjs_concurrent_tasks",
                MetricType::RequestsPerSecond => "amberjs_requests_per_second",
                MetricType::CacheHitRate => "amberjs_cache_hit_rate_percent",
                MetricType::GcTime => "amberjs_gc_time_microseconds",
                MetricType::V8HeapSize => "amberjs_v8_heap_size_bytes",
                MetricType::Custom(name) => name,
            };
            let labels: _ = if metric.tags.is_empty() {
                String::new()
            } else {
                metric
                    .tags
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let line: _ = if labels.is_empty() {
                format!("{} {} {}\n", metric_name, metric.value, metric.timestamp)
            } else {
                format!(
                    "{}{{{}}} {} {}\n",
                    metric_name, labels, metric.value, metric.timestamp
                )
            };
            prometheus.push_str(&line);
        }
        Ok(prometheus)
    }
    /// 获取统计信息
    pub fn get_stats(&self) -> Result<DataStoreStats, String> {
        let stats: _ = self.stats.lock().map_err(|e| e.to_string())?;
        Ok(stats.clone())
    }
    /// 清理过期数据
    pub fn cleanup(&self) -> Result<u64, String> {
        let mut memory_cache = self.memory_cache.lock().map_err(|e| e.to_string())?;
        let mut compressed_storage = self.compressed_storage.lock().map_err(|e| e.to_string())?;
        let mut stats = self.stats.lock().map_err(|e| e.to_string())?;
        let cutoff_time: _ =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap() - self.config.retention_period;
        let mut cleaned_count = 0;
        // 清理内存缓存
        while let Some(data_point) = memory_cache.front() {
            if data_point.value.timestamp < cutoff_time.as_secs() {
                memory_cache.pop_front();
                stats.memory_cache_size -= std::mem::size_of::<DataPoint>();
                cleaned_count += 1;
            } else {
                break;
            }
        }
        // 清理压缩存储
        compressed_storage.retain(|_, compressed_data| {
            if compressed_data.time_range.1 < cutoff_time.as_secs() {
                cleaned_count += 1;
                false
            } else {
                true
            }
        });
        stats.last_cleanup = Some(Instant::now());
        Ok(cleaned_count)
    }
    /// 获取实时指标
    pub fn get_real_time_metrics(&self) -> Result<Vec<MetricValue>, String> {
        let memory_cache: _ = self.memory_cache.lock().map_err(|e| e.to_string())?;
        // 获取最近几分钟的数据作为实时指标
        let now: _ = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let recent_threshold: _ = now - 300; // 最近5分钟
        let mut real_time_metrics = Vec::new();
        // 从内存缓存中提取最近的指标
        for data_point in memory_cache.iter() {
            if data_point.value.timestamp >= recent_threshold {
                real_time_metrics.push(data_point.value.clone());
            }
        }
        Ok(real_time_metrics)
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_data_store_creation() {
        let store: _ = DataStore::with_default_config();
        assert!(store.get_stats().is_ok());
    }
    #[test]
    fn test_store_single_data_point() {
        let store: _ = DataStore::with_default_config();
        let data_point: _ = DataPoint {
            value: MetricValue {
                metric_type: MetricType::CpuUsage,
                value: 50.0,
                timestamp: 1234567890,
                tags: HashMap::new(),
            },
            compressed_size: None,
        };
        assert!(store.store(data_point).is_ok());
        let stats: _ = store.get_stats().unwrap();
        assert_eq!(stats.total_data_points, 1);
    }
    #[test]
    fn test_store_batch_data_points() {
        let store: _ = DataStore::with_default_config();
        let data_points: _ = vec![
            DataPoint {
                value: MetricValue {
                    metric_type: MetricType::CpuUsage,
                    value: 50.0,
                    timestamp: 1234567890,
                    tags: HashMap::new(),
                },
                compressed_size: None,
            },
            DataPoint {
                value: MetricValue {
                    metric_type: MetricType::MemoryUsage,
                    value: 100.0,
                    timestamp: 1234567891,
                    tags: HashMap::new(),
                },
                compressed_size: None,
            },
        ];
        assert!(store.store_batch(data_points).is_ok());
        let stats: _ = store.get_stats().unwrap();
        assert_eq!(stats.total_data_points, 2);
    }
    #[test]
    fn test_query_data() {
        let store: _ = DataStore::with_default_config();
        // 存储数据
        let data_point: _ = DataPoint {
            value: MetricValue {
                metric_type: MetricType::CpuUsage,
                value: 50.0,
                timestamp: 1234567890,
                tags: HashMap::new(),
            },
            compressed_size: None,
        };
        store.store(data_point).unwrap();
        // 查询数据
        let condition: _ = QueryCondition {
            metric_type: Some(MetricType::CpuUsage),
            start_time: None,
            end_time: None,
            tag_filters: HashMap::new(),
            limit: None,
        };
        let results: _ = store.query(condition).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, 50.0);
    }
    #[test]
    fn test_export_json() {
        let store: _ = DataStore::with_default_config();
        let data_point: _ = DataPoint {
            value: MetricValue {
                metric_type: MetricType::CpuUsage,
                value: 50.0,
                timestamp: 1234567890,
                tags: HashMap::new(),
            },
            compressed_size: None,
        };
        store.store(data_point).unwrap();
        let condition: _ = QueryCondition {
            metric_type: None,
            start_time: None,
            end_time: None,
            tag_filters: HashMap::new(),
            limit: None,
        };
        let json: _ = store.export(condition, ExportFormat::Json).unwrap();
        assert!(json.contains("cpu_usage"));
    }
    #[test]
    fn test_export_csv() {
        let store: _ = DataStore::with_default_config();
        let data_point: _ = DataPoint {
            value: MetricValue {
                metric_type: MetricType::CpuUsage,
                value: 50.0,
                timestamp: 1234567890,
                tags: HashMap::new(),
            },
            compressed_size: None,
        };
        store.store(data_point).unwrap();
        let condition: _ = QueryCondition {
            metric_type: None,
            start_time: None,
            end_time: None,
            tag_filters: HashMap::new(),
            limit: None,
        };
        let csv: _ = store.export(condition, ExportFormat::Csv).unwrap();
        assert!(csv.contains("timestamp,metric_type,value,tags"));
    }
    #[test]
    fn test_rle_compression() {
        let data: _ = vec![1, 1, 1, 1, 2, 2, 3, 3, 3];
        let compressed: _ = DataStore::rle_compress(&data);
        let decompressed: _ = DataStore::rle_decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
    #[test]
    fn test_cleanup_expired_data() {
        let store: _ = DataStore::with_default_config();
        // 存储数据
        let data_point: _ = DataPoint {
            value: MetricValue {
                metric_type: MetricType::CpuUsage,
                value: 50.0,
                timestamp: 1000000000, // 很旧的时间
                tags: HashMap::new(),
            },
            compressed_size: None,
        };
        store.store(data_point).unwrap();
        // 清理过期数据
        let cleaned_count: _ = store.cleanup().unwrap();
        assert!(cleaned_count > 0);
    }
}
