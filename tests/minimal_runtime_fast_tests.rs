// MinimalRuntime 快速启动模式测试
// v0.3.231: 测试 new_fast() 构造函数

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_fast_runtime_creation() {
    // 测试快速启动模式创建
    let runtime = MinimalRuntime::new_fast();

    assert!(runtime.is_ok(), "Fast runtime creation should succeed");
    println!("✅ MinimalRuntime::new_fast() 创建成功");
}

#[test]
#[serial]
fn test_fast_runtime_execution() {
    // 测试快速启动模式的运行时可以执行代码
    let mut runtime = MinimalRuntime::new_fast().expect("Failed to create fast runtime");

    let result = runtime.execute_code("1 + 1");
    assert!(result.is_ok(), "Execution should succeed");

    let output = result.unwrap();
    assert_eq!(output.trim(), "2", "1 + 1 should equal 2");
    println!("✅ 快速模式执行 1+1 = {}", output.trim());
}

#[test]
#[serial]
fn test_fast_runtime_string_operations() {
    // 测试快速模式下的字符串操作
    let mut runtime = MinimalRuntime::new_fast().expect("Failed to create fast runtime");

    let code = r#""hello" + " " + "world""#;
    let result = runtime.execute_code(code);

    assert!(result.is_ok(), "String concatenation should succeed");
    assert_eq!(result.unwrap().trim(), "hello world");
    println!("✅ 快速模式字符串拼接正常");
}

#[test]
#[serial]
fn test_fast_runtime_object_creation() {
    // 测试快速模式下的对象创建
    let mut runtime = MinimalRuntime::new_fast().expect("Failed to create fast runtime");

    let code = r#"const obj = { name: "test", value: 42 }; obj.name"#;
    let result = runtime.execute_code(code);

    assert!(result.is_ok(), "Object creation should succeed");
    assert_eq!(result.unwrap().trim(), "test");
    println!("✅ 快速模式对象创建正常");
}

#[test]
#[serial]
fn test_fast_runtime_array_operations() {
    // 测试快速模式下的数组操作
    let mut runtime = MinimalRuntime::new_fast().expect("Failed to create fast runtime");

    let code = r#"[1, 2, 3, 4, 5].reduce((a, b) => a + b, 0)"#;
    let result = runtime.execute_code(code);

    assert!(result.is_ok(), "Array operations should succeed");
    assert_eq!(result.unwrap().trim(), "15");
    println!("✅ 快速模式数组操作正常");
}

#[test]
#[serial]
fn test_fast_runtime_timers() {
    // 测试快速模式下的定时器
    let mut runtime = MinimalRuntime::new_fast().expect("Failed to create fast runtime");

    let code = r#"typeof setTimeout"#;
    let result = runtime.execute_code(code);

    assert!(result.is_ok(), "Timer check should succeed");
    assert_eq!(result.unwrap().trim(), "function");
    println!("✅ 快速模式定时器可用");
}
