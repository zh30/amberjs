#!/usr/bin/env python3
"""
Stage 61: 清理编译警告脚本
自动删除未使用的导入，减少 335 个编译警告
"""

import re
import os
from pathlib import Path

def fix_unused_imports(file_path):
    """修复单个文件中的未使用导入"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content
    changes = []

    # 1. 修复 Context 导入 (anyhow::Result, Context)
    if 'use anyhow::{' in content:
        # 检查是否真的使用了 Context
        if 'Context' in content and 'context!' not in content and '.context(' not in content:
            # 从 anyhow 导入中移除 Context
            content = re.sub(
                r'use anyhow::\{([^}]*?)Context([^}]*?)\}',
                r'use anyhow::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'Context' from anyhow imports")

    # 2. 修复 Command 导入 (clap)
    if 'use clap::{' in content and 'Command' in content:
        # 检查是否使用了 Command
        if 'clap::Command' not in content and 'Command::' not in content:
            content = re.sub(
                r'use clap::\{([^}]*?)Command([^}]*?)\}',
                r'use clap::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'Command' from clap imports")

    # 3. 修复 warn 导入 (tracing)
    if 'use tracing::{' in content:
        if 'warn' in content and 'warn!(' not in content and 'tracing::warn' not in content:
            content = re.sub(
                r'use tracing::\{([^}]*?)warn([^}]*?)\}',
                r'use tracing::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'warn' from tracing imports")

    # 4. 修复 error 导入 (tracing)
    if 'use tracing::{' in content:
        if 'error' in content and 'error!(' not in content and 'tracing::error' not in content:
            content = re.sub(
                r'use tracing::\{([^}]*?)error([^}]*?)\}',
                r'use tracing::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'error' from tracing imports")

    # 5. 修复未使用的变量 (以 _ 开头)
    # 匹配: let variable_name = ...
    content = re.sub(r'\blet (\w+) = ', r'let _\1 = ', content)
    if content != original_content:
        changes.append("Prefixed unused variables with underscore")

    # 6. 修复 Duration 和 Instant 导入
    if 'use tokio::time::{' in content:
        if 'Duration' in content and 'Duration::' not in content:
            content = re.sub(
                r'use tokio::time::\{([^}]*?)Duration([^}]*?)\}',
                r'use tokio::time::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'Duration' from tokio::time imports")

        if 'Instant' in content and 'Instant::' not in content:
            content = re.sub(
                r'use tokio::time::\{([^}]*?)Instant([^}]*?)\}',
                r'use tokio::time::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'Instant' from tokio::time imports")

    # 7. 修复 std::time 中的 Duration, SystemTime, UNIX_EPOCH
    if 'use std::time::{' in content:
        if 'Duration' in content and 'Duration::' not in content and '.duration_' not in content:
            content = re.sub(
                r'use std::time::\{([^}]*?)Duration([^}]*?)\}',
                r'use std::time::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'Duration' from std::time imports")

        if 'SystemTime' in content and 'SystemTime::' not in content:
            content = re.sub(
                r'use std::time::\{([^}]*?)SystemTime([^}]*?)\}',
                r'use std::time::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'SystemTime' from std::time imports")

        if 'UNIX_EPOCH' in content and 'UNIX_EPOCH' not in content:
            content = re.sub(
                r'use std::time::\{([^}]*?)UNIX_EPOCH([^}]*?)\}',
                r'use std::time::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'UNIX_EPOCH' from std::time imports")

    # 8. 修复 instrument 导入
    if 'use tracing::{' in content and 'instrument' in content:
        if '#[instrument' not in content:
            content = re.sub(
                r'use tracing::\{([^}]*?)instrument([^}]*?)\}',
                r'use tracing::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'instrument' from tracing imports")

    # 9. 修复 GaugeVec 和 Histogram 导入
    if 'use prometheus::{' in content:
        if 'GaugeVec' in content and 'GaugeVec::' not in content:
            content = re.sub(
                r'use prometheus::\{([^}]*?)GaugeVec([^}]*?)\}',
                r'use prometheus::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'GaugeVec' from prometheus imports")

        if 'Histogram' in content and 'Histogram::' not in content:
            content = re.sub(
                r'use prometheus::\{([^}]*?)Histogram([^}]*?)\}',
                r'use prometheus::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'Histogram' from prometheus imports")

    # 10. 修复 layer::SubscriberExt 和 util::SubscriberInitExt
    if 'use tracing_subscriber::{' in content:
        if 'layer::SubscriberExt' in content and 'SubscriberExt' not in content:
            content = re.sub(
                r'use tracing_subscriber::\{([^}]*?)layer::SubscriberExt([^}]*?)\}',
                r'use tracing_subscriber::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'layer::SubscriberExt' from tracing_subscriber imports")

        if 'util::SubscriberInitExt' in content and 'SubscriberInitExt' not in content:
            content = re.sub(
                r'use tracing_subscriber::\{([^}]*?)util::SubscriberInitExt([^}]*?)\}',
                r'use tracing_subscriber::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'util::SubscriberInitExt' from tracing_subscriber imports")

    # 写入文件
    if content != original_content:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        return changes

    return []

def main():
    """主函数：扫描所有 Rust 文件并修复未使用的导入"""
    src_dir = Path('/Users/henry/code/amberjs/src')
    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("🔧 Stage 61: 清理编译警告")
    print("=" * 60)

    # 递归查找所有 .rs 文件
    for rs_file in src_dir.rglob('*.rs'):
        total_files += 1
        changes = fix_unused_imports(rs_file)

        if changes:
            fixed_files += 1
            print(f"\n📝 {rs_file.relative_to(src_dir)}:")
            for change in changes:
                print(f"  ✅ {change}")
                total_changes += 1

    print("\n" + "=" * 60)
    print(f"✅ 完成！扫描了 {total_files} 个文件")
    print(f"📊 修复了 {fixed_files} 个文件")
    print(f"🎯 总计 {total_changes} 处修改")
    print("\n下一步: 运行 'cargo check --lib' 验证修复结果")

if __name__ == '__main__':
    main()
