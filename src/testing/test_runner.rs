// Test Runner
// Executes test suites and collects results
//
// v0.3.251: Enhanced to use V8TestExecutor for actual JS test execution

use crate::testing::test_context::{TestCase, TestResult, TestSuite};
use crate::testing::v8_test_executor::V8TestExecutor;
use rusty_v8 as v8;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Test runner configuration
#[derive(Debug, Clone)]
pub struct TestRunnerConfig {
    pub parallel: bool,
    pub timeout: Duration,
    pub bail: bool, // Stop on first failure
}
impl Default for TestRunnerConfig {
    fn default() -> Self {
        TestRunnerConfig {
            parallel: false,
            timeout: Duration::from_secs(5),
            bail: false,
        }
    }
}
/// Test runner statistics
#[derive(Debug, Clone, Default)]
pub struct TestRunnerStats {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub total_duration: Duration,
}
impl TestRunnerStats {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn add_result(&mut self, result: &TestResult) {
        self.total_tests += 1;
        if result.passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }
        self.total_duration += result.duration;
    }
    pub fn add_skipped(&mut self) {
        self.total_tests += 1;
        self.skipped_tests += 1;
    }
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }
}
/// Test runner
/// v0.3.251: Uses V8TestExecutor for actual JS test execution
pub struct TestRunner {
    pub config: TestRunnerConfig,
    executor: V8TestExecutor, // v0.3.251: V8 executor for running tests
}

impl TestRunner {
    pub fn new(config: TestRunnerConfig) -> Self {
        TestRunner {
            config,
            executor: V8TestExecutor::new(),
        }
    }
    /// Run a single test case using V8 executor
    /// v0.3.251: Now actually executes the JS test function
    pub fn run_test(
        &mut self,
        suite_name: &str,
        test: &TestCase,
        before_each: Option<&[v8::Global<v8::Function>]>,
        after_each: Option<&[v8::Global<v8::Function>]>,
    ) -> TestResult {
        let start: _ = Instant::now();
        let mut result = TestResult::new(suite_name.to_string(), test.name.clone());
        if test.skip {
            // Return a passed result for skipped tests
            let duration: _ = start.elapsed();
            result.duration = duration;
            return result;
        }
        // v0.3.251: Execute test using V8
        result = self
            .executor
            .execute_test(suite_name, test, before_each, after_each);
        result
    }
    /// Run a test suite
    pub fn run_suite(
        &mut self,
        suite: &TestSuite,
        stats: Arc<Mutex<TestRunnerStats>>,
    ) -> Vec<TestResult> {
        let mut results = Vec::new();
        // Run beforeAll hook if present (using V8 executor)
        if let Some(before_all) = &suite.before_all {
            // Execute beforeAll in V8
            let mut isolate = v8::Isolate::new(Default::default());
            v8::scope!(let scope, &mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);

            let hook_fn = v8::Local::new(scope, before_all);
            let undefined = v8::undefined(scope);
            let _ = hook_fn.call(scope, undefined.into(), &[]);
        }
        // Run tests in the suite
        for test in &suite.tests {
            let result = self.run_test(
                &suite.name,
                test,
                Some(&suite.before_each),
                Some(&suite.after_each),
            );
            {
                let mut locked_stats = stats.lock().unwrap();
                locked_stats.add_result(&result);
            }
            // Bail out on first failure if configured
            if self.config.bail && !result.passed {
                results.push(result);
                break;
            }
            results.push(result);
        }
        // Run afterAll hook if present (using V8 executor)
        if let Some(after_all) = &suite.after_all {
            let mut isolate = v8::Isolate::new(Default::default());
            v8::scope!(let scope, &mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);

            let hook_fn = v8::Local::new(scope, after_all);
            let undefined = v8::undefined(scope);
            let _ = hook_fn.call(scope, undefined.into(), &[]);
        }
        results
    }
    /// Run multiple test suites
    pub fn run_suites(&mut self, suites: Vec<TestSuite>) -> (Vec<TestResult>, TestRunnerStats) {
        let stats = Arc::new(Mutex::new(TestRunnerStats::new()));
        let mut all_results = Vec::new();
        for suite in suites {
            // If any test is marked as only, skip tests without only
            let has_only: _ = suite.has_only();
            let suite_to_run: _ = if has_only {
                // Filter tests to only those marked as only
                suite.tests.iter().filter(|t| t.only).cloned().collect()
            } else {
                suite.tests.clone()
            };
            // Create a filtered suite for execution
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
}
/// Test reporter trait
pub trait TestReporter {
    fn report_results(&self, results: &[TestResult], stats: &TestRunnerStats);
}
/// Basic console reporter
pub struct ConsoleReporter {
    pub verbose: bool,
}
impl ConsoleReporter {
    pub fn new(verbose: bool) -> Self {
        ConsoleReporter { verbose }
    }
}
impl TestReporter for ConsoleReporter {
    fn report_results(&self, results: &[TestResult], stats: &TestRunnerStats) {
        println!("\n=== Test Results ===");
        println!("Total: {} tests", stats.total_tests);
        println!("Passed: {}", stats.passed_tests);
        println!("Failed: {}", stats.failed_tests);
        println!("Skipped: {}", stats.skipped_tests);
        println!("Success Rate: {:.2}%", stats.success_rate());
        println!("Duration: {:?}", stats.total_duration);
        if self.verbose {
            println!("\n=== Test Details ===");
            for result in results {
                let status: _ = if result.passed { "✓" } else { "✗" };
                println!("{} {} - {:?}", status, result.test_name, result.duration);
                if !result.passed {
                    if let Some(ref error) = result.error {
                        println!("  Error: {}", error);
                    }
                }
            }
        }
        if stats.failed_tests > 0 {
            println!("\n❌ Some tests failed");
        } else {
            println!("\n✅ All tests passed");
        }
    }
}
