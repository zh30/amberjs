#!/usr/bin/env python3
"""
修复复杂的多项目导入
"""

import re
import subprocess
from pathlib import Path

def fix_complex_imports():
    """修复复杂的多项目导入"""
    # 手动列出一些已知可以安全删除的导入
    safe_removals = [
        # debugger 相关
        ("src/debugger/engine.rs", "DebugCommand"),
        ("src/debugger/session.rs", "DebugCliCommand"),
        ("src/debugger/stack_trace.rs", "HashMap"),
        ("src/debugger/variable_scope.rs", "TryFrom"),
        ("src/debugger/session.rs", "rusty_v8 as v8"),
        ("src/debugger/mod.rs", "HashMap"),
        ("src/debugger/mod.rs", "Arc"),
        ("src/debugger/mod.rs", "rusty_v8 as v8"),

        # observability 相关
        ("src/observability/metrics.rs", "Instant"),
        ("src/observability/metrics.rs", "error"),
        ("src/observability/alerting.rs", "warn"),
        ("src/observability/jaeger_tracer.rs", "Context"),
        ("src/observability/jaeger_tracer.rs", "Duration, SystemTime, and UNIX_EPOCH"),
        ("src/observability/jaeger_tracer.rs", "instrument"),
        ("src/observability/mod.rs", "error and warn"),
        ("src/observability/mod.rs", "layer::SubscriberExt and util::SubscriberInitExt"),

        # ai_inference 相关
        ("src/ai_inference/tensor_ops.rs", "Context"),
        ("src/ai_inference/onnx_runtime.rs", "Context"),
        ("src/ai_inference/batch_optimizer.rs", "Deserialize and Serialize"),
        ("src/ai_inference/pytorch_engine.rs", "Context"),
        ("src/ai_inference/pytorch_engine.rs", "Mutex"),
    ]

    fixed_count = 0
    for file_path, item_to_remove in safe_removals:
        full_path = Path("/Users/henry/code/amberjs") / file_path
        if not full_path.exists():
            continue

        try:
            with open(full_path, 'r', encoding='utf-8') as f:
                content = f.read()

            # 查找并删除特定项目
            lines = content.split('\n')
            modified = False

            for i, line in enumerate(lines):
                if line.strip().startswith('use '):
                    # 处理复杂导入
                    if '::{' in line and item_to_remove in line:
                        # 解析并删除特定项目
                        use_match = re.match(r'(\s*use\s+[^;]+)\{([^}]+)\}(.*)', line)
                        if use_match:
                            prefix = use_match.group(1) + '{'
                            items = use_match.group(2)
                            suffix = use_match.group(3)

                            # 解析项目列表
                            item_list = [item.strip() for item in items.split(',')]
                            item_list = [item for item in item_list if item and item != item_to_remove]

                            if item_list:
                                lines[i] = prefix + ', '.join(item_list) + '}' + suffix
                            else:
                                lines[i] = ''  # 删除整行
                            modified = True
                            break

            if modified:
                with open(full_path, 'w', encoding='utf-8') as f:
                    f.write('\n'.join(lines))
                fixed_count += 1
                print(f"✅ Fixed: {file_path} - removed {item_to_remove}")

        except Exception as e:
            print(f"❌ Error processing {file_path}: {e}")

    print(f"\n📊 Total fixed: {fixed_count} imports")
    return fixed_count

if __name__ == "__main__":
    print("🔧 Fixing complex imports...")
    fixed = fix_complex_imports()

    # 验证
    print("\n📊 Running cargo check...")
    result = subprocess.run(
        ["cargo", "check"],
        capture_output=True,
        text=True,
        cwd="/Users/henry/code/amberjs"
    )
    warning_count = result.stderr.count('warning:')
    print(f"Warnings after fix: {warning_count}")
