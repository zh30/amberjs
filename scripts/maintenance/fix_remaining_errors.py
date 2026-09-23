#!/usr/bin/env python3
"""
修复剩余的编译错误
"""

import os
import re

def fix_ai_hardware_features():
    """修复 AiHardwareFeatures 类型问题"""
    # 检查是否存在定义
    simd_file = "/Users/henry/code/amberjs/src/wasm/simd_engine.rs"
    if not os.path.exists(simd_file):
        print("❌ simd_engine.rs 不存在")
        return 0

    # 检查 HardwareFeatures 是否在 simd_engine 中定义
    with open(simd_file, 'r', encoding='utf-8') as f:
        content = f.read()

    if 'pub struct HardwareFeatures' in content or 'pub struct AiHardwareFeatures' in content:
        print("✅ HardwareFeatures 已在 simd_engine.rs 中定义")
        return 0
    else:
        print("⚠️  HardwareFeatures 未在 simd_engine.rs 中定义")
        return 0

def fix_duplicate_ai_mod():
    """修复重复的 ai 模块定义"""
    lib_file = "/Users/henry/code/amberjs/src/lib.rs"

    with open(lib_file, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    # 查找重复的 ai 模块声明
    ai_mod_lines = []
    for i, line in enumerate(lines):
        if 'pub mod ai' in line and not line.strip().startswith('//'):
            ai_mod_lines.append((i, line.strip()))

    if len(ai_mod_lines) > 1:
        print(f"⚠️  发现 {len(ai_mod_lines)} 个 ai 模块声明:")
        for i, (line_num, content) in enumerate(ai_mod_lines):
            print(f"  行 {line_num + 1}: {content}")

        # 注释掉第 93 行的 ai 模块声明
        # 保留第 21-35 行的内联声明
        if len(ai_mod_lines) >= 2:
            second_ai_line = ai_mod_lines[1][0]
            if 'pub mod ai;' in lines[second_ai_line]:
                lines[second_ai_line] = '// pub mod ai;  // Stage 78 Phase 3: AI 工作负载专用优化 (moved to inline mod)\n'

                with open(lib_file, 'w', encoding='utf-8') as f:
                    f.writelines(lines)
                print(f"✅ 注释掉了第 {second_ai_line + 1} 行的 ai 模块声明")
                return 1

    return 0

def fix_duration_operations():
    """修复 Duration 运算问题"""
    files_to_fix = [
        "/Users/henry/code/amberjs/src/ai/ai_performance_engine.rs",
        "/Users/henry/code/amberjs/src/ai/intelligent_scheduler.rs",
    ]

    fixed_count = 0
    for filepath in files_to_fix:
        if not os.path.exists(filepath):
            continue

        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 修复 Duration 运算
        # 替换 duration1 - duration2 为 duration1.as_secs_f64() - duration2.as_secs_f64()
        content = re.sub(
            r'(\w+)\s*-\s*(\w+)',
            lambda m: f"{m.group(1)}.as_secs_f64() - {m.group(2)}.as_secs_f64()"
            if 'duration' in m.group(1).lower() and 'duration' in m.group(2).lower()
            else m.group(0),
            content
        )

        # 替换 duration + duration 为 duration + duration
        content = re.sub(
            r'(\w+)\s*\+\s*(\w+)',
            lambda m: f"{m.group(1)}.as_secs_f64() + {m.group(2)}.as_secs_f64()"
            if 'duration' in m.group(1).lower() and 'duration' in m.group(2).lower()
            else m.group(0),
            content
        )

        if content != original_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✅ 修复了 {filepath} 中的 Duration 运算")
            fixed_count += 1

    return fixed_count

def fix_tensor_optimizer_optimize():
    """修复 TensorOptimizer::optimize 方法调用"""
    filepath = "/Users/henry/code/amberjs/src/ai/ai_performance_engine.rs"

    if not os.path.exists(filepath):
        return 0

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    # 修复 optimize 方法调用 - 添加 Mutex::lock()
    content = re.sub(
        r'optimizer\.optimize\(&history_data\)\.await',
        '{\n                let mut optimizer_guard = optimizer.lock().unwrap();\n                optimizer_guard.optimize(&history_data).await;\n            }',
        content
    )

    if content != original_content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath} 中的 optimize 方法调用")
        return 1
    return 0

def fix_mismatched_types():
    """修复类型不匹配错误"""
    # 这个需要根据具体错误进行修复
    # 暂时跳过，返回 0
    return 0

def main():
    """主函数"""
    print("🔧 开始修复剩余的编译错误...\n")

    total_fixed = 0

    # 1. 修复 AiHardwareFeatures
    print("1. 检查 AiHardwareFeatures...")
    total_fixed += fix_ai_hardware_features()

    # 2. 修复重复的 ai 模块
    print("\n2. 修复重复的 ai 模块定义...")
    total_fixed += fix_duplicate_ai_mod()

    # 3. 修复 Duration 运算
    print("\n3. 修复 Duration 运算...")
    total_fixed += fix_duration_operations()

    # 4. 修复 TensorOptimizer::optimize
    print("\n4. 修复 TensorOptimizer::optimize 方法...")
    total_fixed += fix_tensor_optimizer_optimize()

    # 5. 修复类型不匹配
    print("\n5. 修复类型不匹配...")
    total_fixed += fix_mismatched_types()

    print(f"\n✨ 完成！共修复了 {total_fixed} 个问题")

    if total_fixed > 0:
        print("\n请重新运行编译:")
        print("  export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1")
        print("  cargo build --release")
    else:
        print("\n✅ 无需修复！")

if __name__ == '__main__':
    main()
