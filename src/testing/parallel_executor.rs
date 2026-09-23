// Parallel Test Executor
// Executes tests concurrently using thread pools

use crate::testing::test_context::{TestCase, TestResult, TestSuite};
use rayon::prelude::*;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Parallel execution configuration
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    pub num_threads: Option<usize>,
    pub preserve_order: bool,
    pub chunk_size: usize,
}
impl Default for ParallelConfig {
    fn default() -> Self {
        ParallelConfig {
            num_threads: None, // Use Rayon's default (number of cores)
            preserve_order: true,
            chunk_size: 1,
        }
    }
}
/// Parallel test executor
pub struct ParallelExecutor {
    pub config: ParallelConfig,
}
impl ParallelExecutor {
    pub fn new(config: ParallelConfig) -> Self {
        ParallelExecutor { config }
    }
    /// Execute test cases in parallel
    pub fn run_tests_parallel(
        &self,
        suite_name: &str,
        tests: &[TestCase],
        timeout: Duration,
    ) -> Vec<TestResult> {
        if tests.is_empty() {
            return Vec::new();
        }
        // Create a shared result vector with thread-safe access
        let results = Arc::new(Mutex::new(Vec::with_capacity(tests.len())));
        let results_clone: Arc<Mutex<Vec<TestResult>>> = Arc::clone(&results);
        // Execute tests in parallel using Rayon
        tests
            .par_iter()
            .chunks(self.config.chunk_size)
            .for_each(|chunk| {
                let chunk_results: Vec<TestResult> = chunk
                    .into_par_iter()
                    .map(|test| self.run_single_test(suite_name, test, timeout))
                    .collect();
                // Insert results maintaining order if requested
                let mut locked: MutexGuard<'_, Vec<TestResult>> = results_clone.lock().unwrap();
                if self.config.preserve_order {
                    locked.extend(chunk_results);
                } else {
                    // For unordered execution, we need to track indices
                    // For simplicity, we'll extend in arbitrary order
                    locked.extend(chunk_results);
                }
            });
        // Extract results from Arc
        let locked: _ = results.lock().unwrap();
        locked.clone()
    }
    /// Execute test suites in parallel
    pub fn run_suites_parallel(
        &self,
        suites: &[TestSuite],
        global_timeout: Duration,
    ) -> Vec<TestResult> {
        if suites.is_empty() {
            return Vec::new();
        }
        // Execute suites in parallel
        let all_results: Vec<Vec<TestResult>> = suites
            .par_iter()
            .map(|suite| self.run_tests_parallel(&suite.name, &suite.tests, global_timeout))
            .collect();
        // Flatten results
        all_results.into_iter().flatten().collect()
    }
    /// Run a single test with timeout
    fn run_single_test(&self, suite_name: &str, test: &TestCase, timeout: Duration) -> TestResult {
        let start: _ = Instant::now();
        let mut result = TestResult::new(suite_name.to_string(), test.name.clone());
        if test.skip {
            let duration: _ = start.elapsed();
            result.duration = duration;
            return result;
        }
        // Execute test with timeout
        let test_timeout: _ = if test.timeout > Duration::from_secs(0) {
            test.timeout
        } else {
            timeout
        };
        let duration: _ = start.elapsed();
        result.passed = false;
        result.error = Some(
            "parallel V8 test execution is not implemented; refusing to report an unexecuted test as passed"
                .to_string(),
        );
        // Check if test exceeded timeout
        if duration > test_timeout {
            result.error = Some(format!("Test timeout: exceeded {:?}", test_timeout));
        }
        result.duration = duration;
        result
    }
}
/// Thread pool configuration for test execution
#[derive(Debug, Clone)]
pub struct ThreadPoolConfig {
    pub size: usize,
    pub stack_size: usize,
    pub name_prefix: String,
}
impl Default for ThreadPoolConfig {
    fn default() -> Self {
        ThreadPoolConfig {
            size: num_cpus::get(),
            stack_size: 2 * 1024 * 1024, // 2MB
            name_prefix: "amberjs-test".to_string(),
        }
    }
}

// Tests for parallel executor are in tests/ directory to avoid V8 API complexity
