//! V8 兼容性检查工具
//!
//! Stage 96 Phase 1: V8 API 兼容性完善
//! 提供命令行工具来检查 V8 API 兼容性

use amberjs::v8_engine::{
    V8CompatibilityChecker,
    V8APIAdapter,
    AdapterConfig,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "check" => {
            println!("🔍 检查 V8 API 兼容性...\n");
            run_compatibility_check().await?;
        }
        "adapt" => {
            println!("🔧 运行 API 适配器...\n");
            run_api_adapter().await?;
        }
        "report" => {
            println!("📊 生成完整兼容性报告...\n");
            generate_full_report().await?;
        }
        "verify" => {
            println!("✅ 验证适配器...\n");
            verify_adapters().await?;
        }
        "help" | "-h" | "--help" => {
            print_usage();
        }
        _ => {
            eprintln!("❌ 未知命令: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Amber V8 兼容性检查工具");
    println!("\n用法:");
    println!("  cargo run --bin v8_compatibility_check <命令>");
    println!("\n命令:");
    println!("  check    - 检查 V8 API 兼容性");
    println!("  adapt    - 运行 API 适配器");
    println!("  report   - 生成完整的兼容性报告");
    println!("  verify   - 验证所有适配器");
    println!("  help     - 显示此帮助信息");
    println!("\n示例:");
    println!("  cargo run --bin v8_compatibility_check check");
    println!("  cargo run --bin v8_compatibility_check report");
}

async fn run_compatibility_check() -> Result<(), anyhow::Error> {
    let checker = V8CompatibilityChecker::new();

    println!("正在检查 V8 API 兼容性...\n");

    // 获取 V8 信息
    let v8_info = checker.get_v8_info().await?;
    println!("📋 V8 信息:");
    println!("  V8 版本: {}", v8_info.v8_version);
    println!("  rusty_v8 版本: {}", v8_info.rusty_v8_version);
    println!("  调试模式: {}", v8_info.build_config.debug);
    println!("  优化模式: {}", v8_info.build_config.optimize);
    println!("  SIMD 支持: {}", v8_info.build_config.simd);
    println!("  并行支持: {}", v8_info.build_config.parallel);
    println!("  启用的特性: {}", v8_info.features.join(", "));
    println!();

    // 检查兼容性
    let report = checker.check_compatibility().await?;
    println!("📊 兼容性报告:");
    println!("  总 API 数量: {}", report.summary.total_apis);
    println!("  稳定 API: {}", report.summary.compatible_apis);
    println!("  已弃用 API: {}", report.summary.deprecated_apis);
    println!("  实验性 API: {}", report.summary.experimental_apis);
    println!("  兼容性百分比: {:.2}%", report.summary.compatibility_percentage);
    println!();

    // 计算兼容性评分
    let score = checker.calculate_compatibility_score(&report);
    println!("🎯 兼容性评分: {:.2}/100", score);

    if score >= 90.0 {
        println!("✅ 优秀！兼容性非常好");
    } else if score >= 70.0 {
        println!("⚠️  良好，但有改进空间");
    } else {
        println!("❌ 需要改进兼容性");
    }
    println!();

    // 显示已弃用的 API
    if !report.deprecated_apis.is_empty() {
        println!("⚠️  已弃用 API:");
        for api in &report.deprecated_apis {
            println!("  - {} (替代: {})", api.api_name, api.replacement);
            if let Some(removal) = &api.removal_version {
                println!("    将于版本 {} 移除", removal);
            }
        }
        println!();
    }

    // 生成迁移指南
    let guide = checker.generate_migration_guide().await?;
    if !guide.migration_steps.is_empty() {
        println!("📋 迁移指南:");
        for step in &guide.migration_steps {
            println!("  {}: {} -> {} ({})", step.priority, step.api_name, step.action, step.estimated_effort);
        }
        println!();
    }

    // 显示建议
    if !guide.recommendations.is_empty() {
        println!("💡 建议:");
        for rec in &guide.recommendations {
            println!("  {}", rec);
        }
        println!();
    }

    Ok(())
}

async fn run_api_adapter() -> Result<(), anyhow::Error> {
    let config = AdapterConfig {
        auto_adapt: true,
        mode: "hybrid".to_string(),
        verbose_logging: true,
        timeout_ms: 5000,
        max_retries: 3,
    };

    let adapter = V8APIAdapter::new(config);

    println!("🔧 V8 API 适配器已启动");
    println!("  适配模式: {}", adapter.config.mode);
    println!("  自动适配: {}", adapter.config.auto_adapt);
    println!();

    // 测试适配一些 API
    let test_apis = vec![
        "OldContext",
        "HandleScope::Empty",
        "V8::Initialize",
        "String::New",
        "Object::New",
    ];

    println!("🧪 测试 API 适配:");
    for api in &test_apis {
        let result = adapter.adapt_api_call(api, serde_json::json!({})).await;
        if result.success {
            println!("  ✅ {} -> {}", api, result.adapted_name);
        } else {
            println!("  ❌ {}: {}", api, result.error_message.unwrap_or_default());
        }
    }
    println!();

    // 显示统计信息
    let stats = adapter.get_stats().await;
    println!("📊 适配统计:");
    println!("  总适配器数量: {}", stats.total_adapters);
    println!("  成功率: {:.2}%", if stats.total_adapters > 0 {
        (stats.successful_adapters as f64 / stats.total_adapters as f64) * 100.0
    } else { 0.0 });
    println!();

    Ok(())
}

async fn generate_full_report() -> Result<(), anyhow::Error> {
    println!("📊 生成完整的 V8 兼容性报告...\n");

    // 兼容性检查
    let checker = V8CompatibilityChecker::new();
    let compatibility_report = checker.check_compatibility().await?;

    println!("=== V8 兼容性检查 ===");
    println!("版本信息:");
    println!("  V8: {}", compatibility_report.v8_version);
    println!("  rusty_v8: {}", compatibility_report.rusty_v8_version);
    println!("兼容性统计:");
    println!("  稳定 API: {}/{}", compatibility_report.summary.compatible_apis, compatibility_report.summary.total_apis);
    println!("  已弃用 API: {}", compatibility_report.summary.deprecated_apis);
    println!("  实验性 API: {}", compatibility_report.summary.experimental_apis);
    println!("  兼容性评分: {:.2}/100", checker.calculate_compatibility_score(&compatibility_report));
    println!();

    // 适配器报告
    let adapter = V8APIAdapter::new_with_default_config();
    let adapter_report = adapter.generate_report().await?;

    println!("=== API 适配器报告 ===");
    println!("适配器统计:");
    println!("  总数量: {}", adapter_report.total_adapters);
    println!("  已验证: {}", adapter_report.verified_adapters);
    println!();

    println!("适配器列表:");
    for item in &adapter_report.adapter_list {
        let status = if item.verified { "✅" } else { "⚠️" };
        println!("  {} {} -> {} ({:?})", status, item.name, item.target, item.adapter_type);
    }
    println!();

    if !adapter_report.recommendations.is_empty() {
        println!("建议:");
        for rec in &adapter_report.recommendations {
            println!("  {}", rec);
        }
        println!();
    }

    println!("=== 总结 ===");
    let score = checker.calculate_compatibility_score(&compatibility_report);
    if score >= 90.0 {
        println!("🎉 V8 兼容性优秀！系统运行正常。");
    } else if score >= 70.0 {
        println!("⚠️  V8 兼容性良好，但建议处理已弃用 API。");
    } else {
        println!("❌ V8 兼容性需要改进。");
    }

    Ok(())
}

async fn verify_adapters() -> Result<(), anyhow::Error> {
    let adapter = V8APIAdapter::new_with_default_config();

    println!("✅ 验证所有适配器...");

    let verified = adapter.verify_all_adapters().await?;
    println!("成功验证了 {} 个适配器:", verified.len());

    for api_name in verified {
        println!("  ✅ {}", api_name);
    }

    if verified.is_empty() {
        println!("没有需要验证的适配器。");
    }

    Ok(())
}
