use beejs::testing::{
    EnhancedRunner, EnhancedRunnerConfig, ParallelConfig, ParallelExecutor, TestCase,
};
use rusty_v8 as v8;
use serial_test::serial;
use std::time::Duration;

fn create_noop_test_case(scope: &mut v8::PinScope, name: &str, timeout: Duration) -> TestCase {
    let function = v8::Function::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         _retval: v8::ReturnValue| {},
    )
    .expect("test function should be created");
    TestCase::new(name.to_string(), v8::Global::new(scope, function), timeout)
}

#[test]
#[serial]
fn parallel_executor_without_actual_v8_execution_fails_closed() {
    beejs::initialize_v8().expect("V8 should initialize");
    let mut isolate = v8::Isolate::new(Default::default());
    v8::scope!(let scope, &mut isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let test_case = create_noop_test_case(
        scope,
        "must not be reported as passed without execution",
        Duration::from_millis(25),
    );
    let executor = ParallelExecutor::new(ParallelConfig {
        num_threads: Some(1),
        preserve_order: true,
        chunk_size: 1,
    });

    let results = executor.run_tests_parallel(
        "fail-closed parallel suite",
        &[test_case],
        Duration::from_secs(1),
    );

    assert_eq!(results.len(), 1);
    assert!(
        !results[0].passed,
        "parallel executor must fail closed when it cannot execute the V8 test function"
    );
    assert!(
        results[0]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("not implemented"),
        "fail-closed result should explain that parallel V8 execution is not implemented, got: {:?}",
        results[0].error
    );
}

#[test]
#[serial]
fn enhanced_runner_without_actual_v8_execution_fails_closed() {
    beejs::initialize_v8().expect("V8 should initialize");
    let mut isolate = v8::Isolate::new(Default::default());
    v8::scope!(let scope, &mut isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let test_case = create_noop_test_case(
        scope,
        "must not be reported as passed without execution",
        Duration::from_millis(25),
    );
    let config = EnhancedRunnerConfig {
        parallel: false,
        retry_count: 0,
        ..EnhancedRunnerConfig::default()
    };
    let runner = EnhancedRunner::new(config);

    let result = runner.run_test_with_retry("fail-closed enhanced suite", &test_case);

    assert!(
        !result.passed,
        "enhanced runner must fail closed when it cannot execute the V8 test function"
    );
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("not implemented"),
        "fail-closed result should explain that enhanced V8 execution is not implemented, got: {:?}",
        result.error
    );
}
