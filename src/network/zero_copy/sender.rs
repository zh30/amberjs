// 零拷贝发送器 - sendfile/splice 系统调用封装
//
// Stage 39.0: 网络零拷贝优化
//
// 该模块提供了高性能的零拷贝数据传输功能，通过 sendfile 和 splice 系统调用
// 实现文件到网络套接字的零拷贝传输，最小化数据在内核空间和用户空间之间的拷贝。

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use std::fs::File;
use std::time::{Duration, Instant};
use std::io::Write;

/// 零拷贝发送方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZeroCopyDirection {
    /// 文件到网络套接字
    FileToSocket,
    /// 管道到网络套接字
    PipeToSocket,
    /// 网络套接字到文件
    SocketToFile,
}
/// 零拷贝传输配置
#[derive(Debug, Clone)]
pub struct ZeroCopySenderConfig {
    /// 块大小 (默认 64KB)
    pub chunk_size: usize,
    /// 超时时间
    pub timeout: Duration,
    /// 最大重试次数
    pub max_retries: u32,
    /// 启用压缩
    pub enable_compression: bool,
    /// 压缩级别 (0-9)
    pub compression_level: u8,
}
impl Default for ZeroCopySenderConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            enable_compression: false,
            compression_level: 6,
        }
    }
}
/// 零拷贝传输统计信息
#[derive(Debug, Clone, Default)]
pub struct ZeroCopySenderStats {
    /// 总传输字节数
    pub total_bytes: u64,
    /// 传输成功次数
    pub success_count: u64,
    /// 传输失败次数
    pub error_count: u64,
    /// 平均传输速度 (bytes/sec)
    pub avg_speed: f64,
    /// 峰值传输速度 (bytes/sec)
    pub peak_speed: f64,
    /// sendfile 调用次数
    pub sendfile_count: u64,
    /// splice 调用次数
    pub splice_count: u64,
    /// 总传输时间
    pub total_duration: Duration,
    /// 最后传输时间
    pub last_transfer: Option<Instant>,
}
impl std::fmt::Display for ZeroCopySenderStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "零拷贝传输统计:\n  总传输字节数: {} bytes ({:.2} MB)\n  成功次数: {}\n  失败次数: {}\n  平均速度: {:.2} bytes/sec\n  峰值速度: {:.2} bytes/sec\n  sendfile 调用: {}\n  splice 调用: {}\n  总耗时: {:?}",
            self.total_bytes,
            self.total_bytes as f64 / 1024.0 / 1024.0,
            self.success_count,
            self.error_count,
            self.avg_speed,
            self.peak_speed,
            self.sendfile_count,
            self.splice_count,
            self.total_duration
        )
    }
}
/// 零拷贝发送器
///
/// 该结构体统一了 sendfile 和 splice 的接口，提供：
/// - 零拷贝文件传输
/// - 智能传输模式选择
/// - 性能统计和监控
/// - 错误处理和重试机制
#[derive(Debug)]
pub struct ZeroCopySender {
    /// 发送配置
    config: ZeroCopySenderConfig,
    /// 传输统计
    stats: Arc<Mutex<ZeroCopySenderStats>>,
    /// sendfile 传输器
    sendfile: Option<SendFile>,
    /// splice 传输器
    splice: Splice,
    /// 当前传输位置
    current_pos: u64,
}
impl ZeroCopySender {
    /// 创建新的零拷贝发送器
    ///
    /// # 参数
    /// - `config`: 发送配置，如果为 None 则使用默认配置
    ///
    /// # 返回值
    /// 返回创建结果
    pub fn new(config: Option<ZeroCopySenderConfig>) -> io::Result<Self> {
        let config: _ = config.unwrap_or_default();
        let splice: _ = Splice::new();
        Ok(Self {
            config,
            stats: Arc::new(Mutex::new(ZeroCopySenderStats::default())),
            sendfile: None,
            splice,
            current_pos: 0,
        })
    }
    /// 从文件路径创建零拷贝发送器
    ///
    /// # 参数
    /// - `file_path`: 文件路径
    /// - `config`: 发送配置
    ///
    /// # 返回值
    /// 返回创建结果
    pub fn from_file(file_path: &str, config: Option<ZeroCopySenderConfig>) -> io::Result<Self> {
        let mut sender = Self::new(config)?;
        let file: _ = File::open(file_path)?;
        sender.sendfile = Some(SendFile::new(file)?);
        Ok(sender)
    }
    /// 零拷贝传输文件到网络套接字
    ///
    /// 使用 sendfile 系统调用实现真正的零拷贝传输
    ///
    /// # 参数
    /// - `socket`: 网络套接字（实现 Write + AsRawFd）
    /// - `max_bytes`: 最大传输字节数（0 表示传输整个文件）
    ///
    /// # 返回值
    /// 返回传输的字节数
    pub fn send_to_socket<W: Write + AsRawFd>(
        &mut self,
        socket: &mut W,
        max_bytes: usize,
    ) -> io::Result<u64> {
        let start_time: _ = Instant::now();
        let mut total_sent = 0u64;
        // 确保 sendfile 已初始化
        if self.sendfile.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "sendfile not initialized",
            ));
        }
        let sendfile: _ = self.sendfile.as_mut().unwrap();
        // 设置起始位置
        sendfile.current_pos = self.current_pos;
        // 执行零拷贝传输
        match sendfile.send_to(socket, max_bytes) {
            Ok(bytes_sent) => {
                total_sent = bytes_sent;
                self.current_pos += bytes_sent;
                // 更新统计信息
                self.update_stats_on_success(bytes_sent, &start_time);
                println!("✅ 零拷贝传输成功: {} bytes", bytes_sent);
                Ok(bytes_sent)
            }
            Err(e) => {
                self.update_stats_on_error(&start_time);
                println!("❌ 零拷贝传输失败: {}", e);
                Err(e)
            }
        }
    }
    /// 零拷贝传输管道数据到网络套接字
    ///
    /// 使用 splice 系统调用实现管道到套接字的零拷贝传输
    ///
    /// # 参数
    /// - `pipe`: Unix 管道
    /// - `socket`: 网络套接字（实现 AsRawFd）
    /// - `max_bytes`: 最大传输字节数
    ///
    /// # 返回值
    /// 返回传输的字节数
    pub fn send_pipe_to_socket(
        &self,
        pipe: &std::os::unix::net::UnixStream,
        socket: &impl AsRawFd,
        max_bytes: usize,
    ) -> io::Result<u64> {
        let start_time: _ = Instant::now();
        let out_fd: _ = socket.as_raw_fd();
        // 使用 splice 进行零拷贝传输
        match self.splice.pipe_to_fd(pipe, out_fd, max_bytes) {
            Ok(bytes_sent) => {
                self.update_stats_on_success(bytes_sent, &start_time);
                println!("✅ 管道到套接字零拷贝传输成功: {} bytes", bytes_sent);
                Ok(bytes_sent)
            }
            Err(e) => {
                self.update_stats_on_error(&start_time);
                println!("❌ 管道到套接字零拷贝传输失败: {}", e);
                Err(e)
            }
        }
    }
    /// 设置文件位置
    ///
    /// # 参数
    /// - `pos`: 新的文件位置
    pub fn set_position(&mut self, pos: u64) {
        self.current_pos = pos;
        if let Some(sendfile) = &mut self.sendfile {
            sendfile.current_pos = pos;
        }
    }
    /// 获取当前文件位置
    ///
    /// # 返回值
    /// 返回当前文件位置
    pub fn position(&self) -> u64 {
        self.current_pos
    }
    /// 重置传输状态
    pub fn reset(&mut self) {
        self.current_pos = 0;
        if let Some(sendfile) = &mut self.sendfile {
            sendfile.reset();
        }
        // 重置统计信息
        let mut stats = self.stats.lock().unwrap();
        *stats = ZeroCopySenderStats::default();
    }
    /// 获取传输统计信息
    ///
    /// # 返回值
    /// 返回统计信息副本
    pub fn get_stats(&self) -> ZeroCopySenderStats {
        self.stats.lock().unwrap().clone()
    }
    /// 计算传输进度百分比
    ///
    /// # 返回值
    /// 返回传输进度 (0.0-100.0)
    pub fn progress(&self) -> f64 {
        if let Some(sendfile) = &self.sendfile {
            let file_size: _ = sendfile.file_size();
            if file_size > 0 {
                return (self.current_pos as f64 / file_size as f64) * 100.0;
            }
        }
        0.0
    }
    /// 获取传输速度 (bytes/sec)
    ///
    /// # 返回值
    /// 返回当前传输速度
    pub fn speed(&self) -> f64 {
        let stats: _ = self.stats.lock().unwrap();
        stats.avg_speed
    }
    /// 检查是否可以继续传输
    ///
    /// # 返回值
    /// 返回是否可以继续传输
    pub fn can_continue(&self) -> bool {
        if let Some(sendfile) = &self.sendfile {
            self.current_pos < sendfile.file_size()
        } else {
            false
        }
    }
    /// 更新成功传输的统计信息
    fn update_stats_on_success(&self, bytes: u64, start_time: &Instant) {
        let mut stats = self.stats.lock().unwrap();
        stats.total_bytes += bytes;
        stats.success_count += 1;
        stats.last_transfer = Some(Instant::now());
        let elapsed: _ = start_time.elapsed();
        stats.total_duration += elapsed;
        if elapsed.as_secs_f64() > 0.0 {
            let current_speed: _ = bytes as f64 / elapsed.as_secs_f64();
            stats.avg_speed = (stats.avg_speed * (stats.success_count - 1) as f64 + current_speed)
                / stats.success_count as f64;
            if current_speed > stats.peak_speed {
                stats.peak_speed = current_speed;
            }
        }
        // 增加 sendfile 或 splice 调用计数
        if self.sendfile.is_some() {
            stats.sendfile_count += 1;
        } else {
            stats.splice_count += 1;
        }
    }
    /// 更新失败传输的统计信息
    fn update_stats_on_error(&self, start_time: &Instant) {
        let mut stats = self.stats.lock().unwrap();
        stats.error_count += 1;
        stats.total_duration += start_time.elapsed();
    }
}
impl Default for ZeroCopySender {
    fn default() -> Self {
        Self::new(None).expect("Failed to create default ZeroCopySender")
    }
}
#[cfg(test)]
mod tests {
    /// 测试创建零拷贝发送器
    #[test]
    fn test_zero_copy_sender_creation() {
        let config: _ = ZeroCopySenderConfig::default();
        let sender: _ = ZeroCopySender::new(Some(config)).expect("创建发送器失败");
        let stats: _ = sender.get_stats();
        assert_eq!(stats.total_bytes, 0);
        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.error_count, 0);
        println!("✅ 测试通过: 零拷贝发送器创建");
    }
    /// 测试文件位置设置
    #[test]
    fn test_position_setting() {
        let mut sender = ZeroCopySender::new(None).expect("创建发送器失败");
        sender.set_position(1024);
        assert_eq!(sender.position(), 1024);
        println!("✅ 测试通过: 文件位置设置");
    }
    /// 测试进度计算
    #[test]
    fn test_progress_calculation() {
        // 创建临时测试文件
        let test_file_path: _ = "/tmp/amberjs_zero_copy_test.bin";
        let test_data: _ = vec![42u8; 1024];
        std::fs::write(test_file_path, &test_data).expect("写入测试文件失败");
        let mut sender =
            ZeroCopySender::from_file(test_file_path, None).expect("创建发送器失败");
        // 设置位置为文件中间
        sender.set_position(512);
        let progress: _ = sender.progress();
        assert!(progress > 0.0 && progress < 100.0);
        println!("进度: {:.1}%", progress);
        // 清理
        std::fs::remove_file(test_file_path).ok();
        println!("✅ 测试通过: 进度计算");
    }
    /// 测试传输统计
    #[test]
    fn test_transfer_stats() {
        let sender: _ = ZeroCopySender::new(None).expect("创建发送器失败");
        // 模拟成功传输
        let start_time: _ = Instant::now();
        sender.update_stats_on_success(1024, &start_time);
        let stats: _ = sender.get_stats();
        assert_eq!(stats.total_bytes, 1024);
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.error_count, 0);
        println!("✅ 测试通过: 传输统计");
    }
}