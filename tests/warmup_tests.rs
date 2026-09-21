// V8 快照预热功能测试
// v0.3.234: 测试 MinimalRuntime::warmup() 功能

use serial_test::serial;

#[test]
#[serial]
fn test_warmup_basic() {
    let mut runtime = amberjs::MinimalRuntime::new().unwrap();

    // 执行预热
    let result = runtime.warmup();
    assert!(result.is_ok(), "预热应该成功");

    // 预热后应该仍然可以正常执行代码
    let result = runtime.execute_code("1 + 1");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "2");
}

#[test]
#[serial]
fn test_warmup_string_operations() {
    let mut runtime = amberjs::MinimalRuntime::new().unwrap();
    runtime.warmup().unwrap();

    // 测试字符串操作（应该使用预热后的优化）
    let result = runtime.execute_code(
        r#"
        const str = "hello world";
        str.length + "," + str.toUpperCase() + "," + str.split(' ')[0]
    "#,
    );
    assert!(result.is_ok());
    let output = result.unwrap().trim().to_string();
    assert!(output.contains("HELLO"));
}

#[test]
#[serial]
fn test_warmup_array_operations() {
    let mut runtime = amberjs::MinimalRuntime::new().unwrap();
    runtime.warmup().unwrap();

    // 测试数组操作
    let result = runtime.execute_code(
        r#"
        const arr = [1, 2, 3, 4, 5];
        arr.map(x => x * 2).filter(x => x > 4).reduce((a, b) => a + b, 0)
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "24"); // filter > 4 gives [6, 8, 10], sum is 24
}

#[test]
#[serial]
fn test_warmup_promise() {
    let mut runtime = amberjs::MinimalRuntime::new().unwrap();
    runtime.warmup().unwrap();

    // 测试 Promise
    let result = runtime.execute_code(
        r#"
        Promise.resolve(42).then(v => v * 2)
    "#,
    );
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_warmup_map_set() {
    let mut runtime = amberjs::MinimalRuntime::new().unwrap();
    runtime.warmup().unwrap();

    // 测试 Map 和 Set
    let result = runtime.execute_code(
        r#"
        const map = new Map([['a', 1]]);
        map.set('b', 2);
        map.get('a') + map.get('b')
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "3");
}

#[test]
#[serial]
fn test_warmup_fast_mode() {
    // 快速模式也应该支持预热
    let mut runtime = amberjs::MinimalRuntime::new_fast().unwrap();
    let result = runtime.warmup();
    assert!(result.is_ok(), "快速模式预热应该成功");

    // 验证功能正常
    let result = runtime.execute_code("'test'.toUpperCase()");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "TEST");
}

#[test]
#[serial]
fn test_warmup_multiple_times() {
    let mut runtime = amberjs::MinimalRuntime::new().unwrap();

    // 多次预热应该没问题
    runtime.warmup().unwrap();
    runtime.warmup().unwrap();
    runtime.warmup().unwrap();

    // 仍然可以正常执行
    let result = runtime.execute_code("({}).toString()");
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_warmup_with_existing_context() {
    let mut runtime = amberjs::MinimalRuntime::new().unwrap();

    // 先执行一些代码创建 context
    runtime.execute_code("let x = 1;").unwrap();

    // 然后执行预热
    let result = runtime.warmup();
    assert!(result.is_ok(), "有 context 时预热应该成功");

    // 验证状态保持
    let result = runtime.execute_code("x");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "1");
}

#[test]
#[serial]
fn test_warmup_after_execute() {
    let mut runtime = amberjs::MinimalRuntime::new().unwrap();

    // 先执行代码
    runtime.execute_code("let y = 2;").unwrap();

    // 再预热
    runtime.warmup().unwrap();

    // 验证变量仍然存在
    let result = runtime.execute_code("y");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "2");
}
