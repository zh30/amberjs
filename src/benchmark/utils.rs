// 基准测试工具函数
//
// 提供基准测试系统中使用的各种工具函数，包括：
// - 时间格式化
// - 文件操作
// - 系统信息收集
// - 数据处理


use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::AtomicU64;
use std::time::SystemTime;
/// 格式化持续时间
pub fn format_duration(duration: &Duration) -> String {
    let nanos: _ = duration.subsec_nanos();
    let secs: _ = duration.as_secs();
    if secs > 60 {
        format!("{:.2}min {:.2}s", secs as f64 / 60.0, secs % 60)
    } else if secs > 0 {
        format!("{}.{:03}s", secs, nanos / 1_000_000)
    } else {
        let micros: _ = duration.subsec_micros();
        if micros > 0 {
            format!("{}.{:03}ms", micros / 1000, micros % 1000)
        } else {
            format!("{}ns", nanos)
        }
    }
}
/// 格式化字节数
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}
/// 格式化百分比
pub fn format_percentage(value: f64) -> String {
    format!("{:.2}%", value)
}
/// 格式化数字
pub fn format_number<T: std::fmt::Display>(value: T) -> String {
    format!("{}", value)
}
/// 获取当前时间戳
pub fn get_current_timestamp() -> SystemTime {
    SystemTime::now()
}
/// 格式化时间戳
pub fn format_timestamp(time: &SystemTime) -> String {
    let dt: _ = time.duration_since(UNIX_EPOCH).unwrap();
    let datetime: _ = chrono::DateTime::from_timestamp(
        dt.as_secs() as i64,
        dt.subsec_nanos() as u32,
    ).unwrap();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
/// 创建目录 (如果不存在)
pub fn create_dir_if_not_exists(path: &PathBuf) -> Result<(), std::io::Error> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}
/// 读取文件内容
pub fn read_file(path: &PathBuf) -> Result<String, std::io::Error> {
    fs::read_to_string(path)
}
/// 写入文件内容
pub fn write_file(path: &PathBuf, content: &str) -> Result<(), std::io::Error> {
    // 创建父目录
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}
/// 获取系统信息
pub fn get_system_info() -> HashMap<String, String> {
    let mut info = HashMap::new();
    // 操作系统
    if let Ok(os) = env::var("OS") {
        info.insert("os".to_string(), os);
    } else {
        info.insert("os".to_string(), "Unknown".to_string());
    }
    // 架构
    if let Ok(arch) = env::var("TARGET_ARCH") {
        info.insert("architecture".to_string(), arch);
    } else {
        info.insert("architecture".to_string(), "Unknown".to_string());
    }
    // CPU 核心数
    info.insert("cpu_cores".to_string(), num_cpus::get().to_string());
    // 内存大小 (估算)
    if let Ok(mem_info) = fs::read_to_string("/proc/meminfo") {
        for line in mem_info.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        let bytes: _ = kb * 1024;
                        info.insert("memory_size".to_string(), bytes.to_string());
                    }
                }
                break;
            }
        }
    }
    info
}
/// 检查命令是否可用
pub fn check_command_available(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
/// 获取命令版本
pub fn get_command_version(command: &str) -> Option<String> {
    let output: _ = Command::new(command)
        .arg("--version")
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}
/// 获取可用的运行时
pub fn get_available_runtimes() -> HashMap<String, bool> {
    let mut runtimes = HashMap::new();
    runtimes.insert("node".to_string(), check_command_available("node"));
    runtimes.insert("npm".to_string(), check_command_available("npm"));
    runtimes.insert("bun".to_string(), check_command_available("bun"));
    runtimes.insert("deno".to_string(), check_command_available("deno"));
    runtimes
}
/// 计算平均值
pub fn calculate_mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}
/// 计算标准差
pub fn calculate_std_dev(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean: _ = calculate_mean(values)?;
    let variance: _ = values.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / values.len() as f64;
    Some(variance.sqrt())
}
/// 计算百分位数
pub fn calculate_percentile(values: &[f64], percentile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted_values = values.to_vec();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let index: _ = (percentile / 100.0) * (sorted_values.len() - 1) as f64;
    let lower: _ = index.floor() as usize;
    let upper: _ = index.ceil() as usize;
    if lower == upper {
        Some(sorted_values[lower])
    } else {
        let weight: _ = index - lower as f64;
        Some(sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight)
    }
}
/// 计算变异系数
pub fn calculate_coefficient_of_variation(values: &[f64]) -> Option<f64> {
    let mean: _ = calculate_mean(values)?;
    let std_dev: _ = calculate_std_dev(values)?;
    if mean == 0.0 {
        None
    } else {
        Some(std_dev / mean)
    }
}
/// 生成唯一 ID
pub fn generate_unique_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter: _ = COUNTER.fetch_add(1, Ordering::SeqCst);
    let timestamp: _ = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}_{}", timestamp, counter)
}
/// 创建临时目录
pub fn create_temp_dir(prefix: &str) -> Result<tempfile::TempDir, std::io::Error> {
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
}
/// 清理临时文件
pub fn cleanup_temp_dir(temp_dir: tempfile::TempDir) -> Result<(), std::io::Error> {
    drop(temp_dir);
    Ok(())
}
/// 重试操作
pub async fn retry_async<F, Fut, T, E>(
    mut f: F,
    max_retries: u32,
    delay: Duration,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut last_error = None;
    for attempt in 0..=max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    Err(last_error.unwrap())
}
/// 并行执行任务
pub async fn parallel_execute<T, R, F, Fut>(
    items: Vec<T>,
    max_concurrency: usize,
    mut f: F,
) -> Vec<R>
where
    F: FnMut(T) -> Fut + Send + Clone,
    Fut: std::future::Future<Output = R> + Send,
    T: Send,
    R: Send,
{
    let semaphore: _ = Semaphore::new(max_concurrency);
    let mut handles = Vec::new();
    for item in items {
        let semaphore: _ = semaphore.clone();
        let f: _ = f.clone();
        let handle: _ = tokio::spawn(async move {
            let _permit: _ = semaphore.acquire().await.unwrap();
            f(item).await
        });
        handles.push(handle);
    }
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }
    results
}
/// 计算性能提升百分比
pub fn calculate_performance_improvement(
    baseline: f64,
    current: f64,
) -> f64 {
    if baseline == 0.0 {
        0.0
    } else {
        ((baseline - current) / baseline) * 100.0
    }
}
/// 检查是否显著提升
pub fn is_significant_improvement(
    baseline: f64,
    current: f64,
    threshold: f64,
) -> bool {
    if baseline == 0.0 {
        false
    } else {
        let improvement: _ = calculate_performance_improvement(baseline, current);
        improvement.abs() >= threshold
    }
}
/// 生成随机测试数据
pub fn generate_test_data(size: usize) -> Vec<f64> {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..size)
        .map(|_| rng.random_range(0.0..1000.0))
        .collect()
}
/// 验证结果数据
pub fn validate_result_data(values: &[f64]) -> Result<(), String> {
    if values.is_empty() {
        return Err("Empty data set".to_string());
    }
    for (i, &value) in values.iter().enumerate() {
        if value.is_nan() {
            return Err(format!("NaN value at index {}", i));
        }
        if value.is_infinite() {
            return Err(format!("Infinite value at index {}", i));
        }
        if value < 0.0 {
            return Err(format!("Negative value at index {}: {}", i, value));
        }
    }
    Ok(())
}
/// 导出数据到 CSV
pub fn export_to_csv(data: &[HashMap<String, String>], path: &PathBuf) -> Result<(), std::io::Error> {
    if data.is_empty() {
        return Ok(());
    }
    // 获取所有键
    let keys: Vec<String> = data[0].keys().cloned().collect();
    // 创建 CSV 内容
    let mut csv = String::new();
    // 写入标题行
    csv.push_str(&keys.join(","));
    csv.push('\n');
    // 写入数据行
    for row in data {
        let values: Vec<String> = keys.iter()
            .map(|key| row.get(key).unwrap_or(&"".to_string()).clone())
            .collect();
        csv.push_str(&values.join(","));
        csv.push('\n');
    }
    write_file(path, &csv)
}
/// 从 CSV 导入数据
pub fn import_from_csv(path: &PathBuf) -> Result<Vec<HashMap<String, String>, std::io::Error> {
    let content: _ = read_file(path)?;
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    // 解析标题行
    let headers: Vec<String> = lines[0]
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    // 解析数据行
    let mut data = Vec::new();
    for line in &lines[1..] {
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<String> = line
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        if values.len() != headers.len() {
            continue; // 跳过不完整的行
        }
        let mut row = HashMap::new();
        for (i, header) in headers.iter().enumerate() {
            row.insert(header.clone(), values[i].clone());
        }
        data.push(row);
    }
    Ok(data)
}
/// 检查文件是否存在且可读
pub fn check_file_readable(path: &PathBuf) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}
/// 检查目录是否存在且可写
pub fn check_dir_writable(path: &PathBuf) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false) || path.parent().map_or(false, |p| check_dir_writable(p))
}
#[cfg(test)]
mod tests {
    use super::*;
use std::time::Duration;
use std::sync::atomic::Ordering;
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(&Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(&Duration::from_secs(2)), "2.000s");
        assert_eq!(format_duration(&Duration::from_secs(65)), "1.08min 1.00s");
    }
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }
    #[test]
    fn test_calculate_mean() {
        assert_eq!(calculate_mean(&[1.0, 2.0, 3.0]), Some(2.0));
        assert_eq!(calculate_mean(&[]), None);
    }
    #[test]
    fn test_calculate_percentile() {
        let values: _ = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(calculate_percentile(&values, 50.0), Some(3.0));
        assert_eq!(calculate_percentile(&values, 25.0), Some(2.0));
        assert_eq!(calculate_percentile(&values, 75.0), Some(4.0));
    }
    #[test]
    fn test_calculate_performance_improvement() {
        assert_eq!(calculate_performance_improvement(100.0, 80.0), 20.0);
        assert_eq!(calculate_performance_improvement(100.0, 120.0), -20.0);
        assert_eq!(calculate_performance_improvement(0.0, 80.0), 0.0);
    }
    #[test]
    fn test_is_significant_improvement() {
        assert!(is_significant_improvement(100.0, 80.0, 15.0));
        assert!(!is_significant_improvement(100.0, 90.0, 15.0));
    }
    #[test]
    fn test_validate_result_data() {
        assert!(validate_result_data(&[1.0, 2.0, 3.0]).is_ok());
        assert!(validate_result_data(&[]).is_err());
        assert!(validate_result_data(&[1.0, f64::NAN, 3.0]).is_err());
        assert!(validate_result_data(&[1.0, f64::INFINITY, 3.0]).is_err());
        assert!(validate_result_data(&[1.0, -1.0, 3.0]).is_err());
    }
    #[test]
    fn test_generate_unique_id() {
        let id1: _ = generate_unique_id();
        let id2: _ = generate_unique_id();
        assert_ne!(id1, id2);
        assert!(id1.len() > 10);
    }
    #[tokio::test]
    async fn test_retry_async() {
        let result: _ = retry_async(|| async { Ok(42) }, 3, Duration::from_millis(10)).await;
        assert_eq!(result, Ok(42));
        let result: _ = retry_async(|| async { Err("error") }, 1, Duration::from_millis(10)).await;
        assert_eq!(result, Err("error"));
    }
    #[tokio::test]
    async fn test_parallel_execute() {
        let items: _ = vec![1, 2, 3, 4, 5];
        let results: _ = parallel_execute(items, 2, |x| async move { x * 2 }).await;
        assert_eq!(results, vec![2, 4, 6, 8, 10]);
    }
}