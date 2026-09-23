// Enhanced Test Command Tests
// Tests for --test-name-pattern, --test-only, --test-skip CLI options
//
// v0.3.263: TDD tests for enhanced test command filtering

/// Test that test-name-pattern filter works correctly
#[test]
fn test_test_name_pattern_filter() {
    // This test validates the TestFilter pattern matching
    // The actual integration with CLI is tested in CLI integration tests

    // Test include pattern matching
    let mut filter = amberjs::testing::enhanced_runner::TestFilter::new();
    filter.include("timeout".to_string());

    assert!(filter.matches("test_timeout_basic", "suite1"));
    assert!(!filter.matches("test_basic", "suite1"));
    assert!(filter.matches("handle_timeout_error", "suite2"));
}

#[test]
fn test_test_exclude_pattern_filter() {
    // Test exclude pattern matching
    let mut filter = amberjs::testing::enhanced_runner::TestFilter::new();
    filter.exclude("slow".to_string());

    assert!(!filter.matches("test_slow_operation", "suite1"));
    assert!(filter.matches("test_fast_operation", "suite1"));
}

#[test]
fn test_test_only_flag() {
    // Test only_tests flag - only runs tests that match include patterns
    let mut filter = amberjs::testing::enhanced_runner::TestFilter::new();
    filter.only_tests = true;
    filter.include("critical".to_string());

    assert!(filter.matches("test_critical_path", "suite1"));
    assert!(!filter.matches("test_edge_case", "suite1"));
}

#[test]
fn test_test_skip_flag() {
    // Test skip_tests flag - skips tests that match exclude patterns
    let mut filter = amberjs::testing::enhanced_runner::TestFilter::new();
    filter.skip_tests = true;
    filter.exclude("wip".to_string());

    assert!(!filter.matches("test_feature_wip", "suite1"));
    assert!(filter.matches("test_feature_done", "suite1"));
}

#[test]
fn test_multiple_include_patterns() {
    // Test multiple include patterns - OR semantics
    let mut filter = amberjs::testing::enhanced_runner::TestFilter::new();
    filter.include("auth".to_string());
    filter.include("user".to_string());

    assert!(filter.matches("test_auth_login", "suite1"));
    assert!(filter.matches("test_user_profile", "suite1"));
    assert!(!filter.matches("test_payment", "suite1"));
}

#[test]
fn test_multiple_exclude_patterns() {
    // Test multiple exclude patterns - OR semantics for exclusion
    let mut filter = amberjs::testing::enhanced_runner::TestFilter::new();
    filter.exclude("debug".to_string());
    filter.exclude("temp".to_string());

    assert!(!filter.matches("test_debug_log", "suite1"));
    assert!(!filter.matches("test_temp_feature", "suite1"));
    assert!(filter.matches("test_production", "suite1"));
}

#[test]
fn test_empty_filter_matches_all() {
    // Test that an empty filter matches all tests
    let filter = amberjs::testing::enhanced_runner::TestFilter::new();

    assert!(filter.matches("any_test", "any_suite"));
    assert!(filter.matches("another_test", "another_suite"));
}

#[test]
fn test_suite_name_also_matched() {
    // Test that suite name is also considered in pattern matching
    let mut filter = amberjs::testing::enhanced_runner::TestFilter::new();
    filter.include("auth".to_string());

    assert!(filter.matches("test_login", "auth_suite"));
    assert!(filter.matches("test_login", "suite_auth_tests"));
    assert!(!filter.matches("test_login", "user_suite"));
}

#[test]
fn test_filter_patterns_are_regexes() {
    let mut filter = amberjs::testing::enhanced_runner::TestFilter::new();
    filter.include(r"^test_(auth|user)_\d+$".to_string());

    assert!(filter.matches("test_auth_42", "suite1"));
    assert!(filter.matches("test_user_7", "suite1"));
    assert!(!filter.matches("test_auth_alpha", "suite1"));
    assert!(!filter.matches("prefix_test_auth_42", "suite1"));
}

#[test]
fn test_exclude_patterns_are_regexes() {
    let mut filter = amberjs::testing::enhanced_runner::TestFilter::new();
    filter.exclude(r"^test_slow_\d+$".to_string());

    assert!(!filter.matches("test_slow_42", "suite1"));
    assert!(filter.matches("test_slow_alpha", "suite1"));
    assert!(filter.matches("prefix_test_slow_42", "suite1"));
}
