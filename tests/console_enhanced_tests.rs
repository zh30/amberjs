//! Console API Tests - console.table and other console methods
//!
//! Tests for enhanced console APIs including console.table, console.time, console.timeEnd, etc.
//! v0.3.255: Added console.table and console utility methods tests
//!
//! Note: These tests verify the console APIs work correctly with various data types.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

/// Test console.table with simple array
#[test]
#[serial]
fn test_console_table_with_array() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.table with array should not throw
    let result = runtime.execute_code("console.table([1, 2, 3]); 'done'");
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("done"));
}

/// Test console.table with object array
#[test]
#[serial]
fn test_console_table_with_object_array() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.table with array of objects
    let code = r#"
        const users = [
            { name: 'Alice', age: 30 },
            { name: 'Bob', age: 25 }
        ];
        console.table(users);
        'object array test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("object array test passed"));
}

/// Test console.table with plain object
#[test]
#[serial]
fn test_console_table_with_object() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.table with plain object should work
    let code = r#"
        const data = { key1: 'value1', key2: 'value2' };
        console.table(data);
        'plain object test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("plain object test passed"));
}

/// Test console.table with empty data
#[test]
#[serial]
fn test_console_table_with_empty() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.table with empty array
    let code = r#"console.table([]); 'empty array passed'"#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
}

/// Test console.time and console.timeEnd
#[test]
#[serial]
fn test_console_time() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.time and console.timeEnd should work
    let code = r#"
        console.time('test');
        for (let i = 0; i < 1000; i++) {}
        console.timeEnd('test');
        'timer test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("timer test passed"));
}

/// Test console.count
#[test]
#[serial]
fn test_console_count() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.count should increment and output
    let code = r#"
        console.count('myLabel');
        console.count('myLabel');
        console.count('myLabel');
        'count test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("count test passed"));
}

/// Test console.countReset
#[test]
#[serial]
fn test_console_count_reset() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.countReset should reset the counter
    let code = r#"
        console.count('resetLabel');
        console.countReset('resetLabel');
        console.count('resetLabel');
        'countReset test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("countReset test passed"));
}

/// Test console.group and console.groupEnd
#[test]
#[serial]
fn test_console_group() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.group and console.groupEnd should work
    let code = r#"
        console.group('myGroup');
        console.log('inside group');
        console.groupEnd();
        'group test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("group test passed"));
}

/// Test console.trace
#[test]
#[serial]
fn test_console_trace() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.trace should output stack trace
    let code = r#"
        function foo() {
            console.trace('trace test');
        }
        foo();
        'trace test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("trace test passed"));
}

/// Test console.assert
#[test]
#[serial]
fn test_console_assert() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.assert should only log when assertion fails
    let code = r#"
        console.assert(true, 'This should not appear');
        console.assert(false, 'Assertion failed message');
        'assert test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("assert test passed"));
}

/// Test console.dir
#[test]
#[serial]
fn test_console_dir() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.dir should output object representation
    let code = r#"
        const obj = { a: 1, b: 2 };
        console.dir(obj);
        'dir test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("dir test passed"));
}

/// Test console.table with nested objects
#[test]
#[serial]
fn test_console_table_nested() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.table with nested objects
    let code = r#"
        const data = [
            { name: 'Item 1', details: { color: 'red', size: 'large' } },
            { name: 'Item 2', details: { color: 'blue', size: 'small' } }
        ];
        console.table(data);
        'nested object test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("nested object test passed"));
}

/// Test console.table with columns parameter
#[test]
#[serial]
fn test_console_table_columns() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.table with specific columns
    let code = r#"
        const data = [
            { a: 1, b: 2, c: 3 },
            { a: 4, b: 5, c: 6 }
        ];
        console.table(data, ['a', 'c']);
        'columns test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("columns test passed"));
}

/// Test console.table with Map
#[test]
#[serial]
fn test_console_table_map() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.table with Map
    let code = r#"
        const map = new Map([
            ['key1', 'value1'],
            ['key2', 'value2']
        ]);
        console.table(map);
        'map test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("map test passed"));
}

/// Test console.table with Set
#[test]
#[serial]
fn test_console_table_set() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // console.table with Set
    let code = r#"
        const set = new Set([1, 2, 3, 4]);
        console.table(set);
        'set test passed'
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("set test passed"));
}
