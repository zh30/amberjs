// Performance analyzer for measuring and analyzing Amber runtime performance
// This module provides tools to measure execution time, cache hit rates,
// and other performance metrics.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Performance metrics for a single execution
#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    pub execution_time_ms: f64,
    pub cache_hit: bool,
    pub code_length: usize,
}
/// Performance analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub total_executions: usize,
    pub average_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub cache_hit_rate: f64,
    pub total_code_executed: usize,
}
/// Performance analyzer for measuring runtime performance
pub struct PerformanceAnalyzer {
    metrics: Vec<ExecutionMetrics>,
    start_time: Instant,
}
impl PerformanceAnalyzer {
    /// Create a new performance analyzer
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
            start_time: Instant::now(),
        }
    }
    /// Measure execution time of a function and record metrics
    pub fn measure_execution<F, R>(&mut self, code: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start: _ = Instant::now();
        let result: _ = f();
        let duration: _ = start.elapsed();
        // Estimate cache hit based on execution time (faster = likely cache hit)
        let execution_time_ms: _ = duration.as_secs_f64() * 1000.0;
        let cache_hit: _ = execution_time_ms < 10.0; // Assume < 10ms is cache hit
        self.metrics.push(ExecutionMetrics {
            execution_time_ms,
            cache_hit,
            code_length: code.len(),
        });
        result
    }
    /// Generate a performance report
    pub fn generate_report(&self) -> PerformanceReport {
        if self.metrics.is_empty() {
            return PerformanceReport {
                total_executions: 0,
                average_time_ms: 0.0,
                min_time_ms: 0.0,
                max_time_ms: 0.0,
                cache_hit_rate: 0.0,
                total_code_executed: 0,
            };
        }
        let total_executions: _ = self.metrics.len();
        let total_time: f64 = self.metrics.iter().map(|m| m.execution_time_ms).sum();
        let average_time_ms: _ = total_time / total_executions as f64;
        let min_time_ms: _ = self
            .metrics
            .iter()
            .map(|m| m.execution_time_ms)
            .fold(f64::INFINITY, f64::min);
        let max_time_ms: _ = self
            .metrics
            .iter()
            .map(|m| m.execution_time_ms)
            .fold(f64::NEG_INFINITY, f64::max);
        let cache_hits: _ = self.metrics.iter().filter(|m| m.cache_hit).count();
        let cache_hit_rate: _ = cache_hits as f64 / total_executions as f64 * 100.0;
        let total_code_executed: usize = self.metrics.iter().map(|m| m.code_length).sum();
        PerformanceReport {
            total_executions,
            average_time_ms,
            min_time_ms,
            max_time_ms,
            cache_hit_rate,
            total_code_executed,
        }
    }
    /// Print a formatted performance report
    pub fn print_report(&self) {
        let report: _ = self.generate_report();
        println!("\n=== Amber Performance Analysis Report ===");
        println!("Total executions: {}", report.total_executions);
        println!("Average execution time: {:.3}ms", report.average_time_ms);
        println!("Min execution time: {:.3}ms", report.min_time_ms);
        println!("Max execution time: {:.3}ms", report.max_time_ms);
        println!("Cache hit rate: {:.1}%", report.cache_hit_rate);
        println!("Total code executed: {} bytes", report.total_code_executed);
        // Performance insights
        if report.cache_hit_rate > 50.0 {
            println!("✅ Good cache hit rate! Script caching is effective.");
        } else {
            println!("⚠️  Low cache hit rate. Consider reusing code patterns.");
        }
        if report.average_time_ms < 10.0 {
            println!("🚀 Excellent performance! Average < 10ms.");
        } else if report.average_time_ms < 50.0 {
            println!("✅ Good performance! Average < 50ms.");
        } else {
            println!("⚠️  Performance could be improved. Average > 50ms.");
        }
        println!("=========================================\n");
    }
    /// Clear all recorded metrics
    pub fn reset(&mut self) {
        self.metrics.clear();
        self.start_time = Instant::now();
    }
    /// Get the number of recorded metrics
    pub fn metrics_count(&self) -> usize {
        self.metrics.len()
    }
    /// Get a reference to the metrics (for testing purposes)
    pub fn get_metrics(&self) -> &Vec<ExecutionMetrics> {
        &self.metrics
    }
    /// Get a reference to the metrics (alias for get_metrics)
    pub fn metrics(&self) -> &Vec<ExecutionMetrics> {
        &self.metrics
    }
    /// Get the start time
    pub fn start_time(&self) -> Instant {
        self.start_time
    }
    /// Add a metric directly (for testing purposes)
    pub fn add_metric(&mut self, metric: ExecutionMetrics) {
        self.metrics.push(metric);
    }
}
impl Default for PerformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
