#!/usr/bin/env python3
"""
Stage 61: 修复复合导入中的未使用项目
处理诸如 use anyhow::{Result, Context} 这样的导入
"""

import re
from pathlib import Path

def fix_complex_unused_imports(file_path):
    """修复单个文件中的复合未使用导入"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content
    changes = []

    # 1. 修复 anyhow 导入中的 Context
    if 'use anyhow::{' in content and 'Context' in content:
        # 检查是否真的使用了 Context
        if not re.search(r'\bcontext!\(|\.context\(', content):
            content = re.sub(
                r'use anyhow::\{([^}]*?)Context([^}]*?)\}',
                r'use anyhow::{\1\2}',
                content
            )
            if content != original_content:
                changes.append("Removed unused 'Context' from anyhow imports")
                original_content = content

    # 2. 修复 tokio::time 导入中的 Duration 和 Instant
    if 'use tokio::time::{' in content:
        # 移除未使用的 Duration
        if re.search(r'Duration[,\s]*\}', content) and not re.search(r'\bDuration::', content):
            content = re.sub(
                r'Duration[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'Duration' from tokio::time imports")
            original_content = content

        # 移除未使用的 Instant
        if re.search(r'Instant[,\s]*\}', content) and not re.search(r'\bInstant::', content):
            content = re.sub(
                r'Instant[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'Instant' from tokio::time imports")
            original_content = content

        # 清理多余的逗号
        content = re.sub(r'\{,\s*', '{', content)
        content = re.sub(r',\s*,\s*', ', ', content)
        content = re.sub(r',\s*\}', '}', content)

    # 3. 修复 std::time 导入
    if 'use std::time::{' in content:
        # 移除未使用的 Duration
        if 'Duration' in content and not re.search(r'\bDuration::|\.duration_', content):
            content = re.sub(
                r'Duration[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'Duration' from std::time imports")
            original_content = content

        # 移除未使用的 SystemTime
        if 'SystemTime' in content and not re.search(r'\bSystemTime::', content):
            content = re.sub(
                r'SystemTime[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'SystemTime' from std::time imports")
            original_content = content

        # 移除未使用的 UNIX_EPOCH
        if 'UNIX_EPOCH' in content and not re.search(r'\bUNIX_EPOCH\b', content):
            content = re.sub(
                r'UNIX_EPOCH[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'UNIX_EPOCH' from std::time imports")
            original_content = content

        # 清理多余的逗号
        content = re.sub(r'\{,\s*', '{', content)
        content = re.sub(r',\s*,\s*', ', ', content)
        content = re.sub(r',\s*\}', '}', content)

    # 4. 修复 tracing 导入
    if 'use tracing::{' in content:
        # 移除未使用的 instrument
        if 'instrument' in content and not re.search(r'#\[instrument\]', content):
            content = re.sub(
                r'instrument[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'instrument' from tracing imports")
            original_content = content

        # 清理多余的逗号
        content = re.sub(r'\{,\s*', '{', content)
        content = re.sub(r',\s*,\s*', ', ', content)
        content = re.sub(r',\s*\}', '}', content)

    # 5. 修复 tracing_subscriber 导入
    if 'use tracing_subscriber::{' in content:
        # 移除未使用的 layer::SubscriberExt
        if 'layer::SubscriberExt' in content and not re.search(r'\bSubscriberExt\b', content):
            content = re.sub(
                r'layer::SubscriberExt[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'layer::SubscriberExt' from tracing_subscriber imports")
            original_content = content

        # 移除未使用的 util::SubscriberInitExt
        if 'util::SubscriberInitExt' in content and not re.search(r'\bSubscriberInitExt\b', content):
            content = re.sub(
                r'util::SubscriberInitExt[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'util::SubscriberInitExt' from tracing_subscriber imports")
            original_content = content

        # 清理多余的逗号
        content = re.sub(r'\{,\s*', '{', content)
        content = re.sub(r',\s*,\s*', ', ', content)
        content = re.sub(r',\s*\}', '}', content)

    # 6. 修复 prometheus 导入
    if 'use prometheus::{' in content:
        # 移除未使用的 GaugeVec
        if 'GaugeVec' in content and not re.search(r'\bGaugeVec::', content):
            content = re.sub(
                r'GaugeVec[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'GaugeVec' from prometheus imports")
            original_content = content

        # 移除未使用的 Histogram
        if 'Histogram' in content and not re.search(r'\bHistogram::', content):
            content = re.sub(
                r'Histogram[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'Histogram' from prometheus imports")
            original_content = content

        # 清理多余的逗号
        content = re.sub(r'\{,\s*', '{', content)
        content = re.sub(r',\s*,\s*', ', ', content)
        content = re.sub(r',\s*\}', '}', content)

    # 7. 修复 edge cdn_provider 导入
    if 'use super::cdn_provider::' in content:
        # 移除未使用的 CdnEndpoint
        if 'CdnEndpoint' in content and not re.search(r'\bCdnEndpoint\b', content):
            content = re.sub(
                r'CdnEndpoint[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'CdnEndpoint' from cdn_provider imports")
            original_content = content

        # 移除未使用的 CdnProvider
        if 'CdnProvider' in content and not re.search(r'\bCdnProvider\b', content):
            content = re.sub(
                r'CdnProvider[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'CdnProvider' from cdn_provider imports")
            original_content = content

        # 移除未使用的 DeploymentStatus
        if 'DeploymentStatus' in content and not re.search(r'\bDeploymentStatus\b', content):
            content = re.sub(
                r'DeploymentStatus[,\s]*',
                '',
                content
            )
            changes.append("Removed unused 'DeploymentStatus' from cdn_provider imports")
            original_content = content

        # 清理多余的逗号
        content = re.sub(r'\{,\s*', '{', content)
        content = re.sub(r',\s*,\s*', ', ', content)
        content = re.sub(r',\s*\}', '}', content)

    # 如果有修改，写回文件
    if content != original_content:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        return changes

    return []

def main():
    """主函数：扫描并修复复合未使用导入"""
    src_dir = Path('/Users/henry/code/amberjs/src')
    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("🔧 Stage 61: 修复复合导入中的未使用项目")
    print("=" * 60)

    # 递归查找所有 .rs 文件
    for rs_file in src_dir.rglob('*.rs'):
        total_files += 1
        changes = fix_complex_unused_imports(rs_file)

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

if __name__ == '__main__':
    main()
