#!/usr/bin/env python3
"""
修复编译错误脚本 - 修复变量命名问题
"""

import re
import sys

def fix_file(filepath):
    """修复单个文件中的变量命名问题"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 修复 auto_optimizer.rs 中的 _optimizer 问题
        content = re.sub(
            r'let _optimizer = AutoOptimizer::new\(\);\s*\n\s*let report = optimizer\.analyze_performance',
            'let optimizer = AutoOptimizer::new();\n        let report = optimizer.analyze_performance',
            content
        )
        content = re.sub(
            r'let _optimizer = AutoOptimizer::new\(\);\s*\n\s*let hotspots = optimizer\.detect_hotspots',
            'let optimizer = AutoOptimizer::new();\n        let hotspots = optimizer.detect_hotspots',
            content
        )
        content = re.sub(
            r'let _optimizer = AutoOptimizer::new\(\);\s*\n\s*let suggestions = optimizer\.suggest_optimizations',
            'let optimizer = AutoOptimizer::new();\n        let suggestions = optimizer.suggest_optimizations',
            content
        )
        content = re.sub(
            r'let _optimizer = AutoOptimizer::new\(\);\s*\n\s*let optimizations = optimizer\.suggest_memory_optimizations',
            'let optimizer = AutoOptimizer::new();\n        let optimizations = optimizer.suggest_memory_optimizations',
            content
        )
        content = re.sub(
            r'let _optimizer = AutoOptimizer::new\(\);\s*\n\s*let suggestions = optimizer\.suggest_parallelization',
            'let optimizer = AutoOptimizer::new();\n        let suggestions = optimizer.suggest_parallelization',
            content
        )
        content = re.sub(
            r'let _optimizer = AutoOptimizer::new\(\);\s*\n\s*let result = optimizer\.apply_optimization',
            'let optimizer = AutoOptimizer::new();\n        let result = optimizer.apply_optimization',
            content
        )
        content = re.sub(
            r'let _optimizer = AutoOptimizer::new\(\);\s*\n\s*let gain = optimizer\.calculate_performance_gain',
            'let optimizer = AutoOptimizer::new();\n        let gain = optimizer.calculate_performance_gain',
            content
        )

        # 修复 predictive_scaler.rs 中的 _scaler 问题
        content = re.sub(
            r'let _scaler = PredictiveScaler::new\(\);\s*\n\s*let predictor = scaler\.predictor\.read\(\)\.await',
            'let scaler = PredictiveScaler::new();\n            let predictor = scaler.predictor.read().await',
            content
        )
        content = re.sub(
            r'let _scaler = PredictiveScaler::new\(\);\s*\n\s*let prediction = scaler\.predict_resource_usage',
            'let scaler = PredictiveScaler::new();\n        let prediction = scaler.predict_resource_usage',
            content
        )
        content = re.sub(
            r'let _scaler = PredictiveScaler::new\(\);\s*\n\s*let analysis = scaler\.analyze_trends',
            'let scaler = PredictiveScaler::new();\n        let analysis = scaler.analyze_trends',
            content
        )
        content = re.sub(
            r'let _scaler = PredictiveScaler::new\(\);\s*\n\s*let schedule = scaler\.optimize_schedule',
            'let scaler = PredictiveScaler::new();\n        let schedule = scaler.optimize_schedule',
            content
        )

        # 修复 dma_engine.rs 中的 _buffer 问题
        content = re.sub(
            r'let _buffer = engine\.allocate_buffer\(1024\)\.await\.unwrap\(\);\s*\n\s*assert!\(buffer\.size\(\) >= 1024\);',
            'let buffer = engine.allocate_buffer(1024).await.unwrap();\n        assert!(buffer.size() >= 1024);',
            content
        )
        content = re.sub(
            r'let _buffer = engine\.allocate_buffer\(1024\)\.await\.unwrap\(\);\s*\n\s*buffer\.write\(&data\).await\.unwrap\(\);',
            'let buffer = engine.allocate_buffer(1024).await.unwrap();\n        buffer.write(&data).await.unwrap();',
            content
        )

        if content != original_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✅ 修复了文件: {filepath}")
            return True
        else:
            print(f"⚪ 无需修改: {filepath}")
            return False

    except Exception as e:
        print(f"❌ 错误处理文件 {filepath}: {e}")
        return False

def main():
    """主函数"""
    files_to_fix = [
        '/Users/henry/code/amberjs/src/ai/auto_optimizer.rs',
        '/Users/henry/code/amberjs/src/ai/predictive_scaler.rs',
        '/Users/henry/code/amberjs/src/io/dma_engine.rs',
    ]

    print("🔧 开始修复编译错误...\n")

    fixed_count = 0
    for filepath in files_to_fix:
        if fix_file(filepath):
            fixed_count += 1

    print(f"\n✨ 完成！共修复了 {fixed_count} 个文件")

    if fixed_count > 0:
        print("\n请重新运行测试:")
        print("  export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1")
        print("  cargo test --lib")
    else:
        print("\n✅ 所有文件已经是最新的！")

if __name__ == '__main__':
    main()
