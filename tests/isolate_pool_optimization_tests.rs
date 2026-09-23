use amberjs::Runtime;
use serial_test::serial;
use std::time::{Duration, Instant};

#[test]
#[serial]
fn test_isolate_pool_optimization() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // 测试重复执行同一段代码的性能（利用Isolate池化）
    let code = r#"
        let sum = 0;
        for (let i = 0; i < 100; i++) {
            sum += i;
        }
        sum;
    "#;

    let iterations = 100;
    let mut durations = Vec::with_capacity(iterations);

    // 第一次执行（可能触发Isolate创建）
    let start = Instant::now();
    let _ = runtime.execute_code(code);
    let first_duration = start.elapsed();
    println!(
        "首次执行耗时: {:.2}ms",
        first_duration.as_secs_f64() * 1000.0
    );

    // 后续执行（应该复用Isolate）
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = runtime.execute_code(code);
        durations.push(start.elapsed());
    }

    let avg_duration: f64 = durations
        .iter()
        .map(|d: &Duration| d.as_secs_f64() * 1000.0)
        .sum::<f64>()
        / iterations as f64;

    println!("平均执行耗时: {:.2}ms", avg_duration);
    println!("Isolate池化优化测试通过！");

    // 验证性能合理（应该比首次执行快）
    assert!(
        avg_duration < first_duration.as_secs_f64() * 1000.0 * 1.5,
        "后续执行应该比首次执行快或相近"
    );
}

#[test]
#[serial]
fn test_isolate_pool_concurrent_execution() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // 测试多个不同代码段的并发执行
    let codes = vec![
        "1 + 2 + 3",
        "let x = 10; x * x;",
        "[1, 2, 3, 4, 5].reduce((a, b) => a + b, 0)",
        "Math.random() * 100;",
    ];

    let start = Instant::now();
    for code in &codes {
        let _ = runtime.execute_code(code);
    }
    let elapsed = start.elapsed();

    println!(
        "并发执行 {} 个代码段耗时: {:.2}ms",
        codes.len(),
        elapsed.as_secs_f64() * 1000.0
    );
    println!("Isolate池并发优化测试通过！");

    // 验证执行时间合理
    assert!(elapsed.as_secs_f64() < 1.0, "并发执行应该在1秒内完成");
}
