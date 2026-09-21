#!/usr/bin/env python3
"""
最终警告清理 - 处理剩余的复杂导入和歧义重导出
"""

import re
import subprocess
from pathlib import Path

def fix_remaining_warnings():
    """修复剩余警告"""
    # 定义修复规则
    fixes = [
        # 1. 分析/优化器模块
        {
            "file": "src/analysis/optimizer.rs",
            "target": "BottleneckSeverity",
            "type": "remove_from_use"
        },

        # 2. 监控模块
        {
            "file": "src/monitor/alerts.rs",
            "target": "ThresholdSeverity",
            "type": "remove_from_use"
        },

        # 3. 观测模块
        {
            "file": "src/observability/metrics.rs",
            "target": "Instant",
            "type": "remove_from_use"
        },
        {
            "file": "src/observability/metrics.rs",
            "target": "error",
            "type": "remove_from_use"
        },
        {
            "file": "src/observability/alerting.rs",
            "target": "warn",
            "type": "remove_from_use"
        },
        {
            "file": "src/observability/jaeger_tracer.rs",
            "target": "Duration, SystemTime, and UNIX_EPOCH",
            "type": "remove_from_use"
        },
        {
            "file": "src/observability/mod.rs",
            "target": "error and warn",
            "type": "remove_from_use"
        },
        {
            "file": "src/observability/mod.rs",
            "target": "layer::SubscriberExt and util::SubscriberInitExt",
            "type": "remove_from_use"
        },

        # 4. AI推理模块
        {
            "file": "src/ai_inference/batch_optimizer.rs",
            "target": "Deserialize and Serialize",
            "type": "remove_from_use"
        },

        # 5. 网络模块
        {
            "file": "src/network/zero_copy/sender.rs",
            "target": "SeekFrom and Seek",
            "type": "remove_from_use"
        },
        {
            "file": "src/network/zero_copy/sender.rs",
            "target": "RawFd",
            "type": "remove_from_use"
        },
        {
            "file": "src/network/zero_copy/receiver.rs",
            "target": "RawFd",
            "type": "remove_from_use"
        },
        {
            "file": "src/network/zero_copy/async_impl.rs",
            "target": "AsyncRead, AsyncWrite, and ReadBuf",
            "type": "remove_from_use"
        },
        {
            "file": "src/network/zero_copy/async_impl.rs",
            "target": "tokio::io::AsyncWriteExt",
            "type": "remove_from_use"
        },
        {
            "file": "src/network/zero_copy/async_impl.rs",
            "target": "std::pin::Pin",
            "type": "remove_from_use"
        },
        {
            "file": "src/network/zero_copy/async_impl.rs",
            "target": "Mutex",
            "type": "remove_from_use"
        },
        {
            "file": "src/network/zero_copy/async_impl.rs",
            "target": "super::super::sendfile::SendFile",
            "type": "remove_from_use"
        },
        {
            "file": "src/network/zero_copy/async_impl.rs",
            "target": "super::super::splice::Splice",
            "type": "remove_from_use"
        },
        {
            "file": "src/network/zero_copy/async_impl.rs",
            "target": "ZeroCopyMetrics",
            "type": "remove_from_use"
        },
    ]

    fixed_count = 0

    for fix in fixes:
        file_path = Path("/Users/henry/code/amberjs") / fix["file"]
        if not file_path.exists():
            continue

        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                lines = f.readlines()

            modified = False
            target = fix["target"]

            for i, line in enumerate(lines):
                if line.strip().startswith('use '):
                    # 尝试匹配并删除目标项目
                    if '::' in line and '{' in line:
                        # 多项目导入
                        use_match = re.match(r'(\s*use\s+[^;]+)\{([^}]+)\}(.*)', line)
                        if use_match:
                            prefix = use_match.group(1) + '{'
                            items_text = use_match.group(2)
                            suffix = use_match.group(3)

                            # 解析项目列表
                            item_list = []
                            for item in items_text.split(','):
                                item = item.strip()
                                if item and item != target and not item.startswith(target.split(' ')[0]):
                                    item_list.append(item)

                            if item_list:
                                lines[i] = prefix + ', '.join(item_list) + '}' + suffix
                            else:
                                # 如果删除后为空，删除整行
                                lines[i] = ''
                            modified = True
                            break

            if modified:
                with open(file_path, 'w', encoding='utf-8') as f:
                    f.write(''.join(lines))
                fixed_count += 1
                print(f"✅ Fixed: {fix['file']} - removed {target}")

        except Exception as e:
            print(f"❌ Error processing {fix['file']}: {e}")

    # 修复歧义重导出
    print("\n🔧 Fixing ambiguous re-exports...")
    ai_inference_mod = Path("/Users/henry/code/amberjs/src/ai_inference/mod.rs")
    if ai_inference_mod.exists():
        try:
            with open(ai_inference_mod, 'r') as f:
                content = f.read()

            # 修复歧义重导出
            content = content.replace(
                "pub use engine_interface::*;\npub use model_loader::*;",
                """pub use engine_interface::{EngineInterface, ModelFormat};
pub use model_loader::{ModelLoader, ModelFormat as LoaderModelFormat};"""
            )

            with open(ai_inference_mod, 'w') as f:
                f.write(content)

            fixed_count += 1
            print("✅ Fixed: src/ai_inference/mod.rs - ambiguous re-exports")

        except Exception as e:
            print(f"❌ Error fixing mod.rs: {e}")

    print(f"\n📊 Total fixed: {fixed_count} items")
    return fixed_count

if __name__ == "__main__":
    print("🔧 Final warning cleanup...")
    fixed = fix_remaining_warnings()

    # 验证
    print("\n📊 Running cargo check...")
    result = subprocess.run(
        ["cargo", "check"],
        capture_output=True,
        text=True,
        cwd="/Users/henry/code/amberjs"
    )
    warning_count = result.stderr.count('warning:')
    print(f"Warnings after final fix: {warning_count}")

    if warning_count < 130:
        print(f"✅ Cleaned {130 - warning_count} more warnings!")
