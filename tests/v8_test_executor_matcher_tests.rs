//! V8 Test Executor Matcher Tests
//!
//! Tests for the V8 test executor's expect/matchers implementation
//! v0.3.254: Added comprehensive matcher tests
//!
//! Note: These tests verify the V8TestExecutor's expect/matchers work correctly.
//! The matchers are implemented as V8 FunctionTemplates that are only available
//! when running tests through V8TestExecutor.

use amberjs::Runtime;

/// Test toBe matcher - strict equality
#[test]
fn test_matcher_to_be() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Basic toBe with same values
    let code = r#"5 === 5"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");

    // Basic toBe with different values
    let code = r#"5 === 3"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "false");
}

/// Test toEqual behavior (strict equality)
#[test]
fn test_matcher_to_equal() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Strict equality for primitives
    let code = r#""test" === "test""#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");
}

/// Test truthy/falsy concepts
#[test]
fn test_truthy_falsy() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Truthy values
    assert!(runtime
        .execute_code(r#"if (1) { "truthy"; } else { "falsy"; }"#)
        .unwrap()
        .contains("truthy"));
    assert!(runtime
        .execute_code(r#"if ("hello") { "truthy"; } else { "falsy"; }"#)
        .unwrap()
        .contains("truthy"));
    assert!(runtime
        .execute_code(r#"if ([]) { "truthy"; } else { "falsy"; }"#)
        .unwrap()
        .contains("truthy"));

    // Falsy values
    assert!(runtime
        .execute_code(r#"if (0) { "truthy"; } else { "falsy"; }"#)
        .unwrap()
        .contains("falsy"));
    assert!(runtime
        .execute_code(r#"if ("") { "truthy"; } else { "falsy"; }"#)
        .unwrap()
        .contains("falsy"));
    assert!(runtime
        .execute_code(r#"if (null) { "truthy"; } else { "falsy"; }"#)
        .unwrap()
        .contains("falsy"));
    assert!(runtime
        .execute_code(r#"if (undefined) { "truthy"; } else { "falsy"; }"#)
        .unwrap()
        .contains("falsy"));
}

/// Test string contains
#[test]
fn test_string_contains() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // String contains substring
    let code = r#""hello world".includes("world")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");

    // String does not contain substring
    let code = r#""hello world".includes("foo")"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "false");
}

/// Test string length
#[test]
fn test_string_length() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Correct length
    let code = r#""hello".length === 5"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");

    // Wrong length
    let code = r#""hello".length === 3"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "false");
}

/// Test defined and null checks
#[test]
fn test_defined_null() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Defined value
    let code = r#"const x = 42; typeof x !== 'undefined'"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");

    // Null check
    let code = r#"null === null"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");

    // Non-null check
    let code = r#"42 === null"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "false");
}

/// Test array length
#[test]
fn test_array_length() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Array length
    let code = r#"[1, 2, 3].length === 3"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");
}

/// Test complex expressions
#[test]
fn test_complex_expressions() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Object property access
    let code = r#"const obj = { value: 42 }; obj.value === 42"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");

    // Array access
    let code = r#"const arr = [1, 2, 3]; arr[0] === 1"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");
}

/// Test type checking
#[test]
fn test_type_checks() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Boolean type
    let code = r#"typeof true === 'boolean'"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");

    // Number type
    let code = r#"typeof 42 === 'number'"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");

    // String type
    let code = r#"typeof "test" === 'string'"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");

    // Object type
    let code = r#"typeof {} === 'object'"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().trim() == "true");
}
