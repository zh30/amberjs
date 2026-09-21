//! Stage 93 Phase 5 演示程序
//!
//! 展示基准测试系统的完整功能

use amberjs::benchmark::{
    BenchmarkEngine, BenchmarkConfig, TestSuite, BenchmarkTest, WorkloadProfile,
    RuntimeComparison, WorkloadExecutor, WorkloadType,
    RuntimeDetector, ProcessLauncher, ProcessConfig,
    RegressionDetector, RealTimeMonitor, MonitorConfig,
    TestLanguage, OutputFormat, Verbosity, Runtime,
};
use std::collections::HashMap;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎉 Amber Stage 93 Phase 5: 性能基准测试套件演示\n");

    // 1. 演示基准测试配置
    println!("1️⃣ 基准测试配置演示");
    let config = BenchmarkConfig::new()
        .name("demo_benchmark")
        .iterations(10)
        .warmup_iterations(2)
        .timeout(Duration::from_secs(30))
        .output_format(OutputFormat::Json)
        .enable_profiling(true)
        .workers(2)
        .verbosity(Verbosity::Info)
        .add_tag("demo")
        .category("performance");

    println!("   配置名称: {}", config.name);
    println!("   迭代次数: {}", config.iterations);
    println!("   预热迭代: {}", config.warmup_iterations);
    println!("   超时时间: {:?}", config.timeout);
    println!("   并行度: {}", config.workers);
    println!("   标签: {:?}", config.tags);

    // 2. 演示工作负载执行器
    println!("\n2️⃣ 工作负载执行器演示");

    // 计算密集型工作负载
    println!("   🔢 计算密集型工作负载");
    let compute_executor = WorkloadExecutor::new(WorkloadType::ComputeIntensive);
    let mut compute_params = HashMap::new();
    compute_params.insert("iterations".to_string(), serde_json::Value::from(5u64));
    compute_params.insert("operation".to_string(), serde_json::Value::from("fibonacci"));

    let compute_result = compute_executor
        .parameters(compute_params)
        .concurrency(2)
        .execute()
        .await?;

    println!("      工作负载类型: {:?}", compute_result.workload_type);
    println!("      执行时间: {:?}", compute_result.total_duration);
    println!("      迭代次数: {}", compute_result.iterations);
    println!("      吞吐量: {:.2} ops/sec", compute_result.throughput);
    println!("      成功: {}", compute_result.success);

    // AI 工作负载
    println!("\n   🤖 AI 工作负载");
    let ai_executor = WorkloadExecutor::new(WorkloadType::AiWorkload);
    let mut ai_params = HashMap::new();
    ai_params.insert("iterations".to_string(), serde_json::Value::from(3u64));

    let ai_result = ai_executor
        .parameters(ai_params)
        .concurrency(1)
        .execute()
        .await?;

    println!("      工作负载类型: {:?}", ai_result.workload_type);
    println!("      执行时间: {:?}", ai_result.total_duration);
    println!("      吞吐量: {:.2} ops/sec", ai_result.throughput);

    // 3. 演示运行时检测
    println!("\n3️⃣ 运行时检测演示");
    let detector = RuntimeDetector::new();
    let available_runtimes = detector.get_available_runtimes();

    println!("   可用运行时: {:?}", available_runtimes);

    for runtime in &available_runtimes {
        if let Some(version) = detector.get_version(*runtime) {
            println!("   {}: {}", runtime, version);
        }
    }

    // 4. 演示实时监控
    println!("\n4️⃣ 实时性能监控演示");
    let monitor_config = MonitorConfig::new()
        .collection_interval(Duration::from_millis(100))
        .enable_detailed_metrics(true);

    let monitor = RealTimeMonitor::new(monitor_config);
    let current_metrics = monitor.get_current_metrics().await;

    println!("   CPU 使用率: {:.2}%", current_metrics.system.cpu_usage);
    println!("   内存使用: {} / {}",
        format_bytes(current_metrics.system.memory_usage.used),
        format_bytes(current_metrics.system.memory_usage.total)
    );
    println!("   收集时间: {:?}", current_metrics.collection_time);

    // 5. 演示回归检测
    println!("\n5️⃣ 性能回归检测演示");
    let regression_detector = RegressionDetector::new(std::path::PathBuf::from("/tmp/amberjs_benchmark_history"));

    // 创建模拟的历史数据
    let mut current_results = amberjs::benchmark::result::BenchmarkResultSet::new("current");
    let mut baseline_results = amberjs::benchmark::result::BenchmarkResultSet::new("baseline");

    // 模拟基线结果 (性能更好)
    let mut baseline_result = amberjs::benchmark::result::BenchmarkResult::new(
        "demo_test",
        Runtime::Amber,
    );
    baseline_result.add_iteration(Duration::from_millis(100));
    baseline_result.finish();

    // 模拟当前结果 (性能稍差)
    let mut current_result = amberjs::benchmark::result::BenchmarkResult::new(
        "demo_test",
        Runtime::Amber,
    );
    current_result.add_iteration(Duration::from_millis(120));
    current_result.finish();

    baseline_results.add_result(baseline_result);
    current_results.add_result(current_result);

    let regression_report = regression_detector
        .detect_regressions(&current_results, &baseline_results)
        .await?;

    println!("   回归检测报告:");
    println!("      总测试数: {}", regression_report.summary.total_tests);
    println!("      回归数: {}", regression_report.summary.regressions);
    println!("      显著回归数: {}", regression_report.summary.significant_regressions);
    println!("      回归率: {:.2}%", regression_report.summary.regression_rate);
    println!("      状态: {}", regression_report.summary.get_status());

    // 6. 演示测试套件
    println!("\n6️⃣ 测试套件演示");
    let mut suite = TestSuite::new(
        "demo_test_suite",
        "演示用的测试套件"
    );

    // 添加基准测试
    let benchmark = BenchmarkTest::new(
        "fibonacci_demo",
        "Fibonacci 计算性能测试",
        "console.log('Fibonacci test');",
        TestLanguage::JavaScript,
    )
    .iterations(10)
    .add_tag("compute")
    .category("mathematics");

    suite = suite.add_benchmark(benchmark);

    // 添加工作负载
    let workload = WorkloadProfile::new(
        "compute_demo",
        WorkloadType::ComputeIntensive,
        "计算密集型演示工作负载",
    )
    .add_parameter("operation", serde_json::Value::from("fibonacci"))
    .concurrency(1);

    suite = suite.add_workload(workload);

    println!("   套件名称: {}", suite.name);
    println!("   基准测试数: {}", suite.benchmarks.len());
    println!("   工作负载数: {}", suite.workloads.len());
    println!("   运行时数: {}", suite.runtimes.len());

    // 7. 演示所有工作负载类型
    println!("\n7️⃣ 所有工作负载类型演示");
    let workload_types = vec![
        WorkloadType::ComputeIntensive,
        WorkloadType::IoIntensive,
        WorkloadType::MemoryIntensive,
        WorkloadType::Concurrent,
        WorkloadType::AiWorkload,
    ];

    for workload_type in workload_types {
        let executor = WorkloadExecutor::new(workload_type)
            .concurrency(1);

        let result = executor.execute().await?;

        println!("   ✓ {:?}: 成功={}, 迭代={}, 吞吐量={:.2}",
            workload_type,
            result.success,
            result.iterations,
            result.throughput
        );
    }

    println!("\n🎉 Stage 93 Phase 5 演示完成！");
    println!("\n📊 总结:");
    println!("   ✅ 基准测试引擎: 配置灵活，执行高效");
    println!("   ✅ 工作负载执行器: 6 种工作负载类型全支持");
    println!("   ✅ 运行时对比: 自动检测，多运行时并行对比");
    println!("   ✅ 回归检测: 智能分析，性能变化追踪");
    println!("   ✅ 实时监控: 系统指标收集，可视化报告");
    println!("   ✅ 测试套件: 模块化管理，易于扩展");

    Ok(())
}

/// 格式化字节数
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}
