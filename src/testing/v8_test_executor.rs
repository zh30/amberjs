//! V8 Test Executor - Executes JavaScript test functions in V8 isolate
//!
//! This module provides the ability to run test cases (stored as V8 `Global<Function>`)
//! within a V8 isolate context. It handles:
//! - V8 isolate creation and lifecycle
//! - JS function execution with error handling
//! - Assertion result collection
//! - Timeout management

use crate::testing::test_context::{TestCase, TestResult, TestSuite};
use rusty_v8 as v8;
use std::time::Instant;

/// Executor for running tests in V8 isolate
pub struct V8TestExecutor {
    /// Isolates created for this executor
    isolates: Vec<v8::OwnedIsolate>,
}

impl V8TestExecutor {
    /// Create a new V8 test executor
    pub fn new() -> Self {
        V8TestExecutor {
            isolates: Vec::new(),
        }
    }

    /// Get the number of isolates created (for testing)
    pub fn isolate_count(&self) -> usize {
        self.isolates.len()
    }

    /// Execute a single test case in V8
    ///
    /// # Arguments
    /// * `suite_name` - Name of the test suite
    /// * `test` - Test case to execute
    /// * `before_each` - Optional beforeEach hooks from the suite
    /// * `after_each` - Optional afterEach hooks from the suite
    ///
    /// # Returns
    /// TestResult with pass/fail status and any error message
    pub fn execute_test(
        &mut self,
        suite_name: &str,
        test: &TestCase,
        before_each: Option<&[v8::Global<v8::Function>]>,
        after_each: Option<&[v8::Global<v8::Function>]>,
    ) -> TestResult {
        let start = Instant::now();
        let mut result = TestResult::new(suite_name.to_string(), test.name.clone());

        if test.skip {
            result.duration = start.elapsed();
            return result;
        }

        // Create a new isolate for this test (simpler than reusing)
        let mut isolate = v8::Isolate::new(Default::default());
        v8::scope!(let scope, &mut isolate);

        // Create context
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // Set up testing APIs (expect, matchers, etc.)
        let global = context.global(scope);
        setup_testing_apis(scope, global);

        // Execute beforeEach hooks
        if let Some(hooks) = before_each {
            for hook in hooks {
                let hook_fn = v8::Local::new(scope, hook);
                let undefined = v8::undefined(scope);
                let _ = hook_fn.call(scope, undefined.into(), &[]);
            }
        }

        // Execute the test function
        let test_fn = v8::Local::new(scope, &test.function);
        let undefined = v8::undefined(scope);

        // Execute the test function with TryCatch for error handling
        let _test_passed: bool;
        let _duration = start.elapsed();
        let mut error_message: Option<String> = None;

        {
            v8::tc_scope!(let tc, scope);

            // Execute test function - TryCatch derefs to HandleScope
            let _call_result = test_fn.call(tc, undefined.into(), &[]);

            // Check for errors during test execution
            if tc.has_caught() {
                let exception = tc.exception();
                error_message = if let Some(exc) = exception {
                    // Use tc (TryCatch) for operations since it derefs to HandleScope
                    let exc_local = v8::Local::new(tc, exc);
                    let exc_str = exc_local.to_string(tc);
                    exc_str
                        .map(|s| s.to_rust_string_lossy(tc))
                        .or_else(|| Some("Unknown error".to_string()))
                } else {
                    Some("Unknown error".to_string())
                };
                _test_passed = false;
            } else {
                _test_passed = true;
            }
        } // TryCatch is dropped here, scope is available again

        if let Some(msg) = error_message {
            result.passed = false;
            result.error = Some(format!("Test threw: {}", msg));
        } else {
            result.passed = true;
        }

        // Execute afterEach hooks (even if test failed)
        if let Some(hooks) = after_each {
            for hook in hooks {
                let hook_fn = v8::Local::new(scope, hook);
                let undefined = v8::undefined(scope);
                let _ = hook_fn.call(scope, undefined.into(), &[]);
            }
        }

        result
    }

    /// Execute a test suite with all its tests
    pub fn execute_suite(&mut self, suite: &TestSuite) -> Vec<TestResult> {
        let mut results = Vec::new();

        // Run beforeAll hook if present
        if let Some(before_all) = &suite.before_all {
            let mut isolate = v8::Isolate::new(Default::default());
            v8::scope!(let scope, &mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);

            let hook_fn = v8::Local::new(scope, before_all);
            let undefined = v8::undefined(scope);
            let _ = hook_fn.call(scope, undefined.into(), &[]);
        }

        // Run all tests
        for test in &suite.tests {
            let result = self.execute_test(
                &suite.name,
                test,
                Some(&suite.before_each),
                Some(&suite.after_each),
            );
            results.push(result);
        }

        // Run afterAll hook if present
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
}

impl Default for V8TestExecutor {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! expectation_actual {
    ($scope:expr, $args:expr) => {{
        let this_obj = $args.this();
        let actual_key = v8::String::new($scope, "_actual").unwrap();
        this_obj
            .get($scope, actual_key.into())
            .unwrap_or_else(|| v8::undefined($scope).into())
    }};
}

macro_rules! value_to_display {
    ($scope:expr, $value:expr) => {{
        let value = $value;
        if value.is_undefined() {
            "undefined".to_string()
        } else if value.is_null() {
            "null".to_string()
        } else if value.is_string() {
            let text = value
                .to_string($scope)
                .map(|value| value.to_rust_string_lossy($scope))
                .unwrap_or_default();
            format!("\"{}\"", text)
        } else {
            value
                .to_string($scope)
                .map(|value| value.to_rust_string_lossy($scope))
                .unwrap_or_else(|| "<unprintable>".to_string())
        }
    }};
}

macro_rules! throw_matcher_error {
    ($scope:expr, $matcher:expr, $actual:expr, $expected:expr $(,)?) => {{
        let actual = value_to_display!($scope, $actual);
        let message = format!(
            "{} assertion failed: expected {} {}",
            $matcher, actual, $expected
        );
        let message = v8::String::new($scope, &message).unwrap();
        let exception = v8::Exception::error($scope, message);
        $scope.throw_exception(exception);
    }};
}

/// Set up testing APIs in V8 context (simplified version)
fn setup_testing_apis(scope: &mut v8::PinScope, global: v8::Local<v8::Object>) {
    // Create expect function - simplified to not use closures
    let expect_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let actual = args.get(0);

            // Create expectation object
            let expect_obj = v8::Object::new(scope);

            // Store actual in a property named "_actual" for matchers to access
            let actual_key = v8::String::new(scope, "_actual").unwrap();
            expect_obj.set(scope, actual_key.into(), actual);

            // Add toBe matcher
            let to_be_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let expected = args.get(0);
                    let actual_val = expectation_actual!(scope, args);
                    let result = actual_val.strict_equals(expected);
                    if result {
                        retval.set(v8::Boolean::new(scope, true).into());
                    } else {
                        let expected = format!("toBe {}", value_to_display!(scope, expected));
                        throw_matcher_error!(scope, "toBe", actual_val, expected);
                    }
                },
            )
            .unwrap();
            let to_be_key = v8::String::new(scope, "toBe").unwrap();
            expect_obj.set(scope, to_be_key.into(), to_be_fn.into());

            // Add toEqual matcher - simplified to not use closures (V8 Function constraints)
            let to_equal_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let expected = args.get(0);
                    let actual_val = expectation_actual!(scope, args);
                    // Use strict equality for comparison
                    let result = actual_val.strict_equals(expected);
                    if result {
                        retval.set(v8::Boolean::new(scope, true).into());
                    } else {
                        let expected = format!("toEqual {}", value_to_display!(scope, expected));
                        throw_matcher_error!(scope, "toEqual", actual_val, expected);
                    }
                },
            )
            .unwrap();
            let to_equal_key = v8::String::new(scope, "toEqual").unwrap();
            expect_obj.set(scope, to_equal_key.into(), to_equal_fn.into());

            // Add toBeTruthy matcher - v0.3.254: Check if actual value is truthy
            let to_truthy_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let actual_val = expectation_actual!(scope, args);
                    // Truthy check: not null, not undefined, not 0, not false, not empty string, not NaN
                    let is_truthy = !actual_val.is_null_or_undefined()
                        && !actual_val.is_false()
                        && !actual_val.is_undefined()
                        && !actual_val.is_null();
                    // Additional check for 0 and empty string
                    let result = if actual_val.is_number() {
                        let num = actual_val.to_number(scope).unwrap();
                        let value = num.value();
                        value != 0.0 && !value.is_nan()
                    } else if actual_val.is_string() {
                        let str_val = actual_val.to_string(scope).unwrap();
                        str_val.length() > 0
                    } else {
                        is_truthy
                    };
                    if result {
                        retval.set(v8::Boolean::new(scope, true).into());
                    } else {
                        throw_matcher_error!(
                            scope,
                            "toBeTruthy",
                            actual_val,
                            "to be truthy".to_string(),
                        );
                    }
                },
            )
            .unwrap();
            let to_truthy_key = v8::String::new(scope, "toBeTruthy").unwrap();
            expect_obj.set(scope, to_truthy_key.into(), to_truthy_fn.into());

            // Add toBeFalsy matcher - v0.3.254: Check if actual value is falsy
            let to_falsy_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let actual_val = expectation_actual!(scope, args);
                    // Falsy check: null, undefined, 0, false, empty string, NaN
                    let is_falsy = actual_val.is_null_or_undefined()
                        || actual_val.is_false()
                        || actual_val.is_undefined()
                        || actual_val.is_null();
                    // Additional check for 0 and empty string
                    let result = if actual_val.is_number() {
                        let num = actual_val.to_number(scope).unwrap();
                        let value = num.value();
                        value == 0.0 || value.is_nan()
                    } else if actual_val.is_string() {
                        let str_val = actual_val.to_string(scope).unwrap();
                        str_val.length() == 0
                    } else {
                        is_falsy
                    };
                    if result {
                        retval.set(v8::Boolean::new(scope, true).into());
                    } else {
                        throw_matcher_error!(
                            scope,
                            "toBeFalsy",
                            actual_val,
                            "to be falsy".to_string(),
                        );
                    }
                },
            )
            .unwrap();
            let to_falsy_key = v8::String::new(scope, "toBeFalsy").unwrap();
            expect_obj.set(scope, to_falsy_key.into(), to_falsy_fn.into());

            // Add toContain matcher - v0.3.254: Check if string/array contains value
            let to_contain_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let actual_val = expectation_actual!(scope, args);
                    let expected = args.get(0);

                    let result = if actual_val.is_string() && expected.is_string() {
                        // String contains
                        let actual_str = actual_val
                            .to_string(scope)
                            .unwrap()
                            .to_rust_string_lossy(scope);
                        let expected_str = expected
                            .to_string(scope)
                            .unwrap()
                            .to_rust_string_lossy(scope);
                        actual_str.contains(&expected_str)
                    } else if actual_val.is_object() {
                        // Check if it's an array - borrow scope once
                        let length_key = v8::String::new(scope, "length").unwrap();
                        if let Some(obj) = actual_val.to_object(scope) {
                            if let Some(length_val) = obj.get(scope, length_key.into()) {
                                if length_val.is_number() {
                                    // Array contains (using strict equality)
                                    let arr = v8::Local::<v8::Array>::try_from(actual_val).ok();
                                    if let Some(arr) = arr {
                                        let len = arr.length();
                                        let mut found = false;
                                        for i in 0..len {
                                            if let Some(item) = arr.get_index(scope, i as u32) {
                                                if item.strict_equals(expected) {
                                                    found = true;
                                                    break;
                                                }
                                            }
                                        }
                                        found
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if result {
                        retval.set(v8::Boolean::new(scope, true).into());
                    } else {
                        let expected = format!("toContain {}", value_to_display!(scope, expected));
                        throw_matcher_error!(scope, "toContain", actual_val, expected);
                    }
                },
            )
            .unwrap();
            let to_contain_key = v8::String::new(scope, "toContain").unwrap();
            expect_obj.set(scope, to_contain_key.into(), to_contain_fn.into());

            // Add toThrow matcher - v0.3.254: Check if function throws (not applicable in this context)
            let to_throw_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let actual_val = expectation_actual!(scope, args);
                    let Ok(function) = v8::Local::<v8::Function>::try_from(actual_val) else {
                        throw_matcher_error!(
                            scope,
                            "toThrow",
                            actual_val,
                            "to be a function that throws".to_string(),
                        );
                        return;
                    };

                    let did_throw = {
                        v8::tc_scope!(let try_catch, scope);
                        let undefined = v8::undefined(try_catch);
                        let _ = function.call(try_catch, undefined.into(), &[]);
                        try_catch.has_caught()
                    };

                    if did_throw {
                        retval.set(v8::Boolean::new(scope, true).into());
                    } else {
                        throw_matcher_error!(scope, "toThrow", actual_val, "to throw".to_string(),);
                    }
                },
            )
            .unwrap();
            let to_throw_key = v8::String::new(scope, "toThrow").unwrap();
            expect_obj.set(scope, to_throw_key.into(), to_throw_fn.into());

            // Add toHaveLength matcher - v0.3.254: Check if string/array has expected length
            let to_length_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let actual_val = expectation_actual!(scope, args);
                    let expected = args.get(0);

                    let expected_len = if expected.is_number() {
                        let value = expected.to_number(scope).unwrap().value();
                        if value >= 0.0 && value.fract() == 0.0 {
                            Some(value as usize)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let actual_len: usize = if actual_val.is_string() {
                        actual_val.to_string(scope).unwrap().length()
                    } else if actual_val.is_object() {
                        // Try to cast to array
                        if let Ok(arr) = v8::Local::<v8::Array>::try_from(actual_val) {
                            arr.length() as usize
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    if expected_len == Some(actual_len) {
                        retval.set(v8::Boolean::new(scope, true).into());
                    } else {
                        let expected =
                            format!("toHaveLength {}", value_to_display!(scope, expected));
                        throw_matcher_error!(scope, "toHaveLength", actual_val, expected);
                    }
                },
            )
            .unwrap();
            let to_length_key = v8::String::new(scope, "toHaveLength").unwrap();
            expect_obj.set(scope, to_length_key.into(), to_length_fn.into());

            // Add toBeDefined matcher - v0.3.254: Check if value is not undefined
            let to_defined_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let actual_val = expectation_actual!(scope, args);
                    let result = !actual_val.is_undefined();
                    if result {
                        retval.set(v8::Boolean::new(scope, true).into());
                    } else {
                        throw_matcher_error!(
                            scope,
                            "toBeDefined",
                            actual_val,
                            "to be defined".to_string(),
                        );
                    }
                },
            )
            .unwrap();
            let to_defined_key = v8::String::new(scope, "toBeDefined").unwrap();
            expect_obj.set(scope, to_defined_key.into(), to_defined_fn.into());

            // Add toBeNull matcher - v0.3.254: Check if value is null
            let to_null_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let actual_val = expectation_actual!(scope, args);
                    let result = actual_val.is_null();
                    if result {
                        retval.set(v8::Boolean::new(scope, true).into());
                    } else {
                        throw_matcher_error!(
                            scope,
                            "toBeNull",
                            actual_val,
                            "to be null".to_string(),
                        );
                    }
                },
            )
            .unwrap();
            let to_null_key = v8::String::new(scope, "toBeNull").unwrap();
            expect_obj.set(scope, to_null_key.into(), to_null_fn.into());

            retval.set(expect_obj.into());
        },
    )
    .unwrap();
    let expect_key = v8::String::new(scope, "expect").unwrap();
    global.set(scope, expect_key.into(), expect_fn.into());

    // Add test/it functions
    let test_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let _name: String = args.get(0).to_rust_string_lossy(scope);
            retval.set(v8::undefined(scope).into());
        },
    )
    .unwrap();
    let test_key = v8::String::new(scope, "test").unwrap();
    global.set(scope, test_key.into(), test_fn.into());

    // Add describe function
    let describe_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let _name: String = args.get(0).to_rust_string_lossy(scope);
            retval.set(v8::undefined(scope).into());
        },
    )
    .unwrap();
    let describe_key = v8::String::new(scope, "describe").unwrap();
    global.set(scope, describe_key.into(), describe_fn.into());

    // Add beforeEach/afterEach/beforeAll/afterAll
    let before_each_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::undefined(scope).into());
        },
    )
    .unwrap();
    let before_each_key = v8::String::new(scope, "beforeEach").unwrap();
    global.set(scope, before_each_key.into(), before_each_fn.into());

    let after_each_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::undefined(scope).into());
        },
    )
    .unwrap();
    let after_each_key = v8::String::new(scope, "afterEach").unwrap();
    global.set(scope, after_each_key.into(), after_each_fn.into());

    let before_all_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::undefined(scope).into());
        },
    )
    .unwrap();
    let before_all_key = v8::String::new(scope, "beforeAll").unwrap();
    global.set(scope, before_all_key.into(), before_all_fn.into());

    let after_all_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            retval.set(v8::undefined(scope).into());
        },
    )
    .unwrap();
    let after_all_key = v8::String::new(scope, "afterAll").unwrap();
    global.set(scope, after_all_key.into(), after_all_fn.into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn execute_with_testing_apis(source: &str) -> Result<String, String> {
        crate::initialize_v8().map_err(|err| err.to_string())?;

        let mut isolate = v8::Isolate::new(Default::default());
        v8::scope!(let handle_scope, &mut isolate);
        let context = v8::Context::new(handle_scope, Default::default());
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let global = context.global(scope);
        setup_testing_apis(scope, global);

        let code = v8::String::new(scope, source).ok_or("failed to create script source")?;
        let script = v8::Script::compile(scope, code, None).ok_or("failed to compile script")?;

        v8::tc_scope!(let try_catch, scope);
        match script.run(try_catch) {
            Some(value) => Ok(value
                .to_string(try_catch)
                .map(|value| value.to_rust_string_lossy(try_catch))
                .unwrap_or_default()),
            None => {
                let message = try_catch
                    .exception()
                    .and_then(|exception| exception.to_string(try_catch))
                    .map(|message| message.to_rust_string_lossy(try_catch))
                    .unwrap_or_else(|| "Unknown V8 exception".to_string());
                Err(message)
            }
        }
    }

    #[test]
    #[serial]
    fn expect_to_be_returns_success_for_matching_values() {
        let result = execute_with_testing_apis("expect(1).toBe(1); 'ok';");

        assert_eq!(result.as_deref(), Ok("ok"));
    }

    #[test]
    #[serial]
    fn expect_to_be_throws_for_mismatched_values() {
        let error = execute_with_testing_apis("expect(1).toBe(2);").unwrap_err();

        assert!(
            error.contains("toBe"),
            "matcher error should name toBe, got: {error}"
        );
    }

    #[test]
    #[serial]
    fn core_matchers_throw_for_mismatches() {
        let cases = [
            ("expect('amber').toEqual('wasp');", "toEqual"),
            ("expect(0).toBeTruthy();", "toBeTruthy"),
            ("expect(1).toBeFalsy();", "toBeFalsy"),
            ("expect('amber').toContain('js');", "toContain"),
            ("expect([1, 2]).toHaveLength(3);", "toHaveLength"),
            ("expect(undefined).toBeDefined();", "toBeDefined"),
            ("expect(1).toBeNull();", "toBeNull"),
            ("expect(() => 1).toThrow();", "toThrow"),
        ];

        for (source, matcher) in cases {
            let error = match execute_with_testing_apis(source) {
                Ok(value) => panic!("{matcher} should throw, got value {value:?}"),
                Err(error) => error,
            };
            assert!(
                error.contains(matcher),
                "{matcher} error should name matcher, got: {error}"
            );
        }
    }

    #[test]
    #[serial]
    fn core_matchers_return_success_for_matching_values() {
        let result = execute_with_testing_apis(
            r#"
            expect('amber').toEqual('amber');
            expect(1).toBeTruthy();
            expect(0).toBeFalsy();
            expect('amberjs').toContain('js');
            expect([1, 2]).toContain(2);
            expect([1, 2]).toHaveLength(2);
            expect('amber').toHaveLength(5);
            expect(null).toBeNull();
            expect(1).toBeDefined();
            expect(() => { throw new Error('boom'); }).toThrow();
            'ok';
            "#,
        );

        assert_eq!(result.as_deref(), Ok("ok"));
    }
}
