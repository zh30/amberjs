#!/usr/bin/env python3
"""
Stage 73 Phase 2: 智能清理编译警告
使用安全的模式匹配，只修复明确的警告
"""

import os
import re
import sys

def smart_fix_warnings(file_path: str) -> int:
    """智能修复警告"""
    changes = 0
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original = content

        # 只处理明确标记的未使用导入
        # 模式: // use xxx; - unused (auto-removed)
        content = re.sub(
            r'^\s*//\s*use\s+[^;]+;\s*-\s*unused\s*\(auto-removed\)\s*$',
            '',
            content,
            flags=re.MULTILINE
        )

        # 处理 verbose_logging cfg 条件
        content = content.replace(
            'if cfg!(feature = "verbose_logging") {',
            '// if cfg!(feature = "verbose_logging") { // disabled - no such feature'
        )

        if content != original:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            changes = 1

    except Exception as e:
        print(f"Error: {file_path}: {e}")

    return changes

def main():
    print("🎯 Stage 73 Phase 2: 智能清理编译警告")
    print("=" * 60)

    # 只处理几个关键文件
    key_files = [
        'src/testing/v8_bindings.rs',
    ]

    for file in key_files:
        path = f'/Users/henry/code/amberjs/{file}'
        if os.path.exists(path):
            print(f"📝 处理: {file}")
            changes = smart_fix_warnings(path)
            print(f"   {'✅ 已修复' if changes else '⚠️  无需修改'}")

    print("\n🔍 运行 clippy 检查:")
    print("   cargo clippy --all-targets --all-features -- -D warnings 2>&1 | head -30")

if __name__ == '__main__':
    main()
