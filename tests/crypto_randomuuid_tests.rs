// Tests for crypto.randomUUID module (v0.3.29)
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_random_uuid_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof crypto.randomUUID");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_random_uuid_returns_string() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const uuid = crypto.randomUUID();
        typeof uuid;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "string");
}

#[test]
#[serial]
fn test_random_uuid_valid_format() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const uuid = crypto.randomUUID();
        // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
        const regex = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
        regex.test(uuid);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_random_uuid_length() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const uuid = crypto.randomUUID();
        uuid.length;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // UUID v4 is 36 characters (including hyphens)
    assert_eq!(result.unwrap().trim(), "36");
}

#[test]
#[serial]
fn test_random_uuid_different_each_time() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const uuid1 = crypto.randomUUID();
        const uuid2 = crypto.randomUUID();
        uuid1 !== uuid2;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_random_uuid_multiple_calls() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const uuids = [];
        for (let i = 0; i < 10; i++) {
            uuids.push(crypto.randomUUID());
        }
        // All should be unique
        new Set(uuids).size === 10;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_random_uuid_no_arguments() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const uuid = crypto.randomUUID();
        uuid !== undefined && uuid.length === 36;
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_random_uuid_consistent_format() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let code = r#"
        const uuid1 = crypto.randomUUID();
        const uuid2 = crypto.randomUUID();
        // Both should have the same format pattern
        const pattern = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
        pattern.test(uuid1) && pattern.test(uuid2);
    "#;
    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}
