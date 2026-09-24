// Enhanced Test Runner
// Provides advanced test execution features including parallel execution,
// timeout control, test filtering, and retry mechanisms

use crate::testing::parallel_executor::{ParallelConfig, ParallelExecutor};
use crate::testing::test_context::{TestCase, TestResult, TestSuite};
use crate::testing::test_timeout::{TestTimeout, TimeoutConfig};
use rand::rng;
use rand::seq::SliceRandom;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Enhanced test runner configuration
#[derive(Debug, Clone)]
pub struct EnhancedRunnerConfig {
    pub parallel: bool,
    pub parallel_config: ParallelConfig,
    pub timeout_config: TimeoutConfig,
    pub bail: bool,
    pub verbose: bool,
    pub retry_count: usize,
    pub test_filter: Option<TestFilter>,
    pub test_sorter: Option<TestSorter>,
}
impl Default for EnhancedRunnerConfig {
    fn default() -> Self {
        EnhancedRunnerConfig {
            parallel: true,
            parallel_config: ParallelConfig::default(),
            timeout_config: TimeoutConfig::default(),
            bail: false,
            verbose: false,
            retry_count: 1,
            test_filter: None,
            test_sorter: None,
        }
    }
}
/// Test filtering options
#[derive(Debug, Clone)]
pub struct TestFilter {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub only_tests: bool,
    pub skip_tests: bool,
}
impl TestFilter {
    pub fn new() -> Self {
        TestFilter {
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            only_tests: false,
            skip_tests: false,
        }
    }
    pub fn include(&mut self, pattern: String) -> &mut Self {
        self.include_patterns.push(pattern);
        self
    }
    pub fn exclude(&mut self, pattern: String) -> &mut Self {
        self.exclude_patterns.push(pattern);
        self
    }

    fn pattern_matches(pattern: &str, test_name: &str, suite_name: &str) -> bool {
        match regex::Regex::new(pattern) {
            Ok(regex) => regex.is_match(test_name) || regex.is_match(suite_name),
            Err(_) => test_name.contains(pattern) || suite_name.contains(pattern),
        }
    }

    /// Check if a test matches this filter
    pub fn matches(&self, test_name: &str, suite_name: &str) -> bool {
        // If only_tests is set, only run tests that match include_patterns
        if self.only_tests && !self.include_patterns.is_empty() {
            return self
                .include_patterns
                .iter()
                .any(|p| Self::pattern_matches(p, test_name, suite_name));
        }
        // If skip_tests is set, exclude tests that match exclude_patterns
        if self.skip_tests && !self.exclude_patterns.is_empty() {
            return !self
                .exclude_patterns
                .iter()
                .any(|p| Self::pattern_matches(p, test_name, suite_name));
        }
        // If include_patterns is set, only run tests that match
        if !self.include_patterns.is_empty() {
            return self
                .include_patterns
                .iter()
                .any(|p| Self::pattern_matches(p, test_name, suite_name));
        }
        // If exclude_patterns is set, exclude matching tests
        if !self.exclude_patterns.is_empty() {
            return !self
                .exclude_patterns
                .iter()
                .any(|p| Self::pattern_matches(p, test_name, suite_name));
        }
        true
    }
}
/// Test sorting options
#[derive(Debug, Clone)]
pub enum TestSorter {
    ByName,
    ByDuration,
    Random,
}
impl TestSorter {
    pub fn sort(&self, tests: &mut [TestCase], _suite_name: &str) {
        match self {
            TestSorter::ByName => {
                tests.sort_by(|a, b| a.name.cmp(&b.name));
            }
            TestSorter::ByDuration => {
                // Sort by timeout (assuming longer timeout = more complex test)
                tests.sort_by(|a, b| b.timeout.cmp(&a.timeout));
            }
            TestSorter::Random => {
                // Fisher-Yates shuffle
                tests.shuffle(&mut rng());
            }
        }
    }
}
/// Enhanced test runner statistics
#[derive(Debug, Clone, Default)]
pub struct EnhancedRunnerStats {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub retried_tests: usize,
    pub timed_out_tests: usize,
    pub total_duration: Duration,
    pub min_duration: Option<Duration>,
    pub max_duration: Option<Duration>,
    pub avg_duration: Duration,
}
impl EnhancedRunnerStats {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn add_result(&mut self, result: &TestResult, was_retried: bool) {
        self.total_tests += 1;
        if was_retried {
            self.retried_tests += 1;
        }
        if result.passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }
        self.total_duration += result.duration;
        // Update min/max duration
        if self.min_duration.is_none() || result.duration < self.min_duration.unwrap() {
            self.min_duration = Some(result.duration);
        }
        if self.max_duration.is_none() || result.duration > self.max_duration.unwrap() {
            self.max_duration = Some(result.duration);
        }
        // Update average duration
        self.avg_duration = Duration::from_nanos(
            (self.total_duration.as_nanos() / self.total_tests as u128) as u64,
        );
    }
    pub fn add_skipped(&mut self) {
        self.total_tests += 1;
        self.skipped_tests += 1;
    }
    pub fn add_timeout(&mut self) {
        self.total_tests += 1;
        self.timed_out_tests += 1;
    }
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }
    pub fn failure_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.failed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }
    pub fn timeout_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.timed_out_tests as f64 / self.total_tests as f64) * 100.0
        }
    }
}
/// Enhanced test runner
pub struct EnhancedRunner {
    pub config: EnhancedRunnerConfig,
    pub parallel_executor: ParallelExecutor,
    pub timeout_handler: TestTimeout,
}
impl EnhancedRunner {
    pub fn new(config: EnhancedRunnerConfig) -> Self {
        let parallel_executor: _ = ParallelExecutor::new(config.parallel_config.clone());
        let timeout_handler: _ = TestTimeout::new(config.timeout_config.clone());
        EnhancedRunner {
            config,
            parallel_executor,
            timeout_handler,
        }
    }
    /// Run a single test with retry logic
    pub fn run_test_with_retry(&self, suite_name: &str, test: &TestCase) -> TestResult {
        let mut last_error = None;
        let mut result = None;
        for attempt in 0..=self.config.retry_count {
            let test_result: _ = self.run_single_test(suite_name, test);
            if test_result.passed {
                return test_result;
            }
            last_error = test_result.error.clone();
            // If this is not the last attempt, wait before retry
            if attempt < self.config.retry_count {
                std::thread::sleep(Duration::from_millis(100));
            }
            result = Some(test_result);
        }
        // Return the last result (which includes the error)
        result.unwrap_or_else(|| {
            TestResult::new(suite_name.to_string(), test.name.clone())
                .with_error(last_error.unwrap_or_else(|| "Unknown error".to_string()))
        })
    }
    /// Run a test suite with enhanced features
    pub fn run_suite(
        &self,
        suite: &TestSuite,
        stats: Arc<Mutex<EnhancedRunnerStats>>,
    ) -> Vec<TestResult> {
        let mut results = Vec::new();
        // Filter and sort tests
        let mut tests = suite.tests.clone();
        // Apply filter
        if let Some(ref filter) = self.config.test_filter {
            tests.retain(|t| filter.matches(&t.name, &suite.name));
        }
        // Apply sorter
        if let Some(ref sorter) = self.config.test_sorter {
            sorter.sort(&mut tests, &suite.name);
        }
        if tests.is_empty() {
            return results;
        }
        // Run beforeAll hook
        if let Some(ref _before_all) = suite.before_all {
            // TODO: Execute beforeAll hook
        }
        // Run tests
        if self.config.parallel && tests.len() > 1 {
            // Run tests in parallel
            let parallel_results: _ = self.parallel_executor.run_tests_parallel(
                &suite.name,
                &tests,
                self.config.timeout_config.default_timeout,
            );
            for result in parallel_results {
                let was_retried: _ = false; // Parallel tests don't retry
                {
                    let mut locked_stats = stats.lock().unwrap();
                    if result.passed {
                        locked_stats.add_result(&result, was_retried);
                    } else {
                        locked_stats.add_result(&result, was_retried);
                    }
                }
                results.push(result.clone());
                // Bail out on first failure if configured
                if self.config.bail && !result.passed {
                    break;
                }
            }
        } else {
            // Run tests sequentially
            for test in tests {
                let result: _ = self.run_test_with_retry(&suite.name, &test);
                {
                    let mut locked_stats = stats.lock().unwrap();
                    if result.passed {
                        locked_stats.add_result(&result, false);
                    } else {
                        locked_stats.add_result(&result, false);
                    }
                }
                results.push(result.clone());
                // Bail out on first failure if configured
                if self.config.bail && !result.passed {
                    break;
                }
            }
        }
        // Run afterAll hook
        if let Some(ref _after_all) = suite.after_all {
            // TODO: Execute afterAll hook
        }
        results
    }
    /// Run multiple test suites
    pub fn run_suites(&self, suites: Vec<TestSuite>) -> (Vec<TestResult>, EnhancedRunnerStats) {
        let stats = Arc::new(Mutex::new(EnhancedRunnerStats::new()));
        let mut all_results = Vec::new();
        for suite in suites {
            // Filter suites that have 'only' tests
            let has_only: _ = suite.tests.iter().any(|t| t.only);
            let suite_to_run: _ = if has_only {
                suite.tests.iter().filter(|t| t.only).cloned().collect()
            } else {
                suite.tests.clone()
            };
            // Create filtered suite
            let mut filtered_suite = suite;
            filtered_suite.tests = suite_to_run;
            let results: _ = self.run_suite(&filtered_suite, Arc::clone(&stats));
            all_results.extend(results);
        }
        let final_stats: _ = Arc::try_unwrap(stats)
            .ok()
            .map(|m| m.into_inner().unwrap())
            .unwrap_or_default();
        (all_results, final_stats)
    }
    /// Run a single test
    fn run_single_test(&self, suite_name: &str, test: &TestCase) -> TestResult {
        let start: _ = Instant::now();
        let mut result = TestResult::new(suite_name.to_string(), test.name.clone());
        if test.skip {
            let duration: _ = start.elapsed();
            result.duration = duration;
            return result;
        }
        // Determine timeout for this test
        let timeout: _ = if test.timeout > Duration::from_secs(0) {
            test.timeout
        } else {
            self.config.timeout_config.default_timeout
        };
        let timeout_result = self.timeout_handler.run_async_with_timeout(timeout, || {});
        if let Err(err) = timeout_result {
            result.passed = false;
            result.error = Some(err.to_string());
        } else {
            result.passed = false;
            result.error = Some(
                "enhanced V8 test execution is not implemented; refusing to report an unexecuted test as passed"
                    .to_string(),
            );
        }
        let duration: _ = start.elapsed();
        result.duration = duration;
        result
    }
}

// Tests for enhanced runner are in tests/ directory to avoid V8 API complexity
