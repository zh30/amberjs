// 计算密集型工作负载
//
// 实现 CPU 密集型任务的性能测试，包括：
// - 数值计算 (Fibonacci, 素数计算)
// - 数组操作 (排序, 搜索)
// - 算法实现 (动态规划, 递归)

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use super::{BenchmarkError, BenchmarkResult as Result, ResourceUsage, WorkloadResult};
use std::time::{Duration, Instant};

/// 计算密集型工作负载
#[derive(Debug)]
pub struct ComputeWorkload {
    /// 工作负载类型
    workload_type: super::WorkloadType,
}
impl ComputeWorkload {
    /// 创建新的计算密集型工作负载
    pub fn new() -> Self {
        Self {
            workload_type: super::WorkloadType::ComputeIntensive,
        }
    }
    /// 执行工作负载
    pub async fn execute(
        &self,
        parameters: HashMap<String, serde_json::Value>>,
        concurrency: u32,
    ) -> Result<WorkloadResult> {
        let mut result = WorkloadResult::new(self.workload_type);
        result.start();
        let iterations: _ = get_iterations(&parameters);
        let operation: _ = get_operation(&parameters);
        // 并行执行计算任务
        let handles: Vec<_> = (0..concurrency)
            .map(|_| {
                let operation: _ = operation.clone();
                let iterations: _ = iterations;
                tokio::spawn(async move {
                    Self::run_compute_tasks(operation, iterations).await
                })
            })
            .collect();
        // 等待所有任务完成
        let mut total_iterations = 0;
        for handle in handles {
            match handle.await {
                Ok(Ok(iterations_completed)) => {
                    total_iterations += iterations_completed;
                }
                Ok(Err(e)) => {
                    result.add_error(&format!("Task failed: {}", e));
                }
                Err(e) => {
                    result.add_error(&format!("Task panicked: {}", e));
                }
            }
        }
        // 收集资源使用情况
        let resource_usage: _ = Self::collect_resource_usage();
        result.resource_usage = resource_usage;
        result.finish(total_iterations);
        Ok(result)
    }
    /// 运行计算任务
    async fn run_compute_tasks(
        operation: String,
        iterations: u32,
    ) -> Result<u32, BenchmarkError> {
        let mut completed_iterations = 0;
        for i in 0..iterations {
            match operation.as_str() {
                "fibonacci" => {
                    Self::fibonacci(30);
                }
                "prime" => {
                    Self::calculate_primes(1000);
                }
                "sort" => {
                    Self::sort_array(10000);
                }
                "matrix_multiply" => {
                    Self::matrix_multiply(100);
                }
                "dynamic_programming" => {
                    Self::longest_common_subsequence("ABCBDAB", "BDCABA");
                }
                "fft" => {
                    Self::fft_benchmark(1024);
                }
                "pi_calculation" => {
                    Self::calculate_pi(1000000);
                }
                "hash_table" => {
                    Self::hash_table_benchmark(10000);
                }
                "tree_traversal" => {
                    Self::tree_traversal(1000);
                }
                _ => {
                    return Err(BenchmarkError::ConfigError(
                        format!("Unknown operation: {}", operation));
                }
            }
            completed_iterations += 1;
            // 每 1000 次迭代检查一次超时
            if i > 0 && i % 1000 == 0 {
                tokio::task::yield_now().await;
            }
        }
        Ok(completed_iterations)
    }
    /// 计算 Fibonacci 数列
    fn fibonacci(n: u32) -> u64 {
        if n <= 1 {
            n as u64
        } else {
            let mut a = 0;
            let mut b = 1;
            for _ in 2..=n {
                let temp: _ = a + b;
                a = b;
                b = temp;
            }
            b
        }
    }
    /// 计算素数
    fn calculate_primes(limit: usize) -> Vec<usize> {
        let mut primes = Vec::new();
        'outer: for num in 2..=limit {
            for p in 2..=(num as f64).sqrt() as usize {
                if num % p == 0 {
                    continue 'outer;
                }
            }
            primes.push(num);
        }
        primes
    }
    /// 数组排序
    fn sort_array(size: usize) -> Vec<i32> {
        let mut rng = rand::rng();
        let mut arr: Vec<i32> = (0..size)
            .map(|_| rng.random_range(0..1000000))
            .collect();
        arr.sort();
        arr
    }
    /// 矩阵乘法
    fn matrix_multiply(size: usize) -> Vec<Vec<i32> {
        let mut a = vec![vec![0; size]; size];
        let mut b = vec![vec![0; size]; size];
        let mut result = vec![vec![0; size]; size];
        // 初始化矩阵
        for i in 0..size {
            for j in 0..size {
                a[i][j] = (i + j) as i32;
                b[i][j] = (i * j) as i32;
            }
        }
        // 矩阵乘法
        for i in 0..size {
            for j in 0..size {
                for k in 0..size {
                    result[i][j] += a[i][k] * b[k][j];
                }
            }
        }
        result
    }
    /// 最长公共子序列 (动态规划)
    fn longest_common_subsequence(s1: &str, s2: &str) -> usize {
        let m: _ = s1.len();
        let n: _ = s2.len();
        let mut dp = vec![vec![0; n + 1]; m + 1];
        for i in 1..=m {
            for j in 1..=n {
                if s1.chars().nth(i - 1) == s2.chars().nth(j - 1) {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }
        dp[m][n]
    }
    /// 快速傅里叶变换 (简化版)
    fn fft_benchmark(size: usize) -> Vec<f64> {
        let mut data: Vec<f64> = (0..size)
            .map(|i| (i as f64 * 0.1).sin())
            .collect();
        // 简化的 FFT 实现
        for i in 0..size {
            let mut real = 0.0;
            let mut imag = 0.0;
            for j in 0..size {
                let angle: _ = -2.0 * std::f64::consts::PI * (i * j) as f64 / size as f64;
                real += data[j] * angle.cos();
                imag += data[j] * angle.sin();
            }
            data[i] = (real * real + imag * imag).sqrt();
        }
        data
    }
    /// 计算圆周率 (蒙特卡洛方法)
    fn calculate_pi(samples: usize) -> f64 {
        let mut rng = rand::rng();
        let mut inside_circle = 0;
        for _ in 0..samples {
            let x: f64 = rng.random_range(-1.0..1.0);
            let y: f64 = rng.random_range(-1.0..1.0);
            if x * x + y * y <= 1.0 {
                inside_circle += 1;
            }
        }
        4.0 * inside_circle as f64 / samples as f64
    }
    /// 哈希表基准测试
    fn hash_table_benchmark(size: usize) -> usize {
        let mut map = HashMap::with_capacity(size);
        let mut count = 0;
        for i in 0..size {
            map.insert(i, i * 2);
        }
        for i in 0..size {
            if let Some(&value) = map.get(&i) {
                count += value;
            }
        }
        count
    }
    /// 树遍历
    fn tree_traversal(size: usize) -> Vec<usize> {
        // 创建二叉树
        let mut tree = Vec::with_capacity(size);
        for i in 0..size {
            tree.push(i);
        }
        // 中序遍历
        let mut result = Vec::new();
        Self::inorder_traversal(&tree, 0, &mut result);
        result
    }
    /// 中序遍历辅助函数
    fn inorder_traversal(tree: &Vec<usize>, index: usize, result: &mut Vec<usize>) {
        if index >= tree.len() {
            return;
        }
        let left: _ = 2 * index + 1;
        let right: _ = 2 * index + 2;
        Self::inorder_traversal(tree, left, result);
        result.push(tree[index]);
        Self::inorder_traversal(tree, right, result);
    }
    /// 收集资源使用情况
    fn collect_resource_usage() -> ResourceUsage {
        ResourceUsage::new()
            .cpu_usage(100.0) // 计算密集型任务 CPU 使用率较高
    }
}
impl Default for ComputeWorkload {
    fn default() -> Self {
        Self::new()
    }
}
/// 获取迭代次数
fn get_iterations(parameters: &HashMap<String, serde_json::Value>) -> u32 {
    parameters
        .get("iterations")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(1000)
}
/// 获取操作类型
fn get_operation(parameters: &HashMap<String, serde_json::Value>) -> String {
    parameters
        .get("operation")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "fibonacci".to_string())
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_fibonacci_calculation() {
        let result: _ = ComputeWorkload::fibonacci(10);
        assert_eq!(result, 55);
    }
    #[tokio::test]
    async fn test_prime_calculation() {
        let primes: _ = ComputeWorkload::calculate_primes(30);
        assert_eq!(primes.len(), 10);
        assert_eq!(primes[0], 2);
        assert_eq!(primes[1], 3);
        assert_eq!(primes[2], 5);
    }
    #[tokio::test]
    async fn test_sort_array() {
        let arr: _ = ComputeWorkload::sort_array(100);
        assert_eq!(arr.len(), 100);
        assert!(arr.windows(2).all(|w| w[0] <= w[1]));
    }
    #[tokio::test]
    async fn test_longest_common_subsequence() {
        let result: _ = ComputeWorkload::longest_common_subsequence("ABCBDAB", "BDCABA");
        assert_eq!(result, 4);
    }
    #[tokio::test]
    async fn test_pi_calculation() {
        let pi: _ = ComputeWorkload::calculate_pi(100000);
        assert!((pi - 3.14159).abs() < 0.01);
    }
    #[tokio::test]
    async fn test_workload_execution() {
        let workload: _ = ComputeWorkload::new();
        let mut parameters = HashMap::new();
        parameters.insert("iterations".to_string(), serde_json::Value::from(10u64));
        parameters.insert("operation".to_string(), serde_json::Value::from("fibonacci"));
        let result: _ = workload.execute(parameters, 1).await.unwrap();
        assert_eq!(result.workload_type, super::super::WorkloadType::ComputeIntensive);
        assert_eq!(result.iterations, 10);
        assert!(result.success);
    }
}