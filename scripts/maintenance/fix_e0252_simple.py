#!/usr/bin/env python3
"""
简单修复 E0252 错误：删除重复的单独导入行
"""

import re
from pathlib import Path

def fix_file(file_path):
    """修复单个文件的重复导入"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    lines = content.split('\n')
    new_lines = []
    i = 0
    changes = 0

    while i < len(lines):
        line = lines[i]

        # 检查是否包含 atomic::Ordering 的行
        if 'atomic::Ordering' in line and 'use std::sync::' in line:
            # 检查下一行
            if i + 1 < len(lines):
                next_line = lines[i + 1]
                # 如果下一行是单独的 Ordering 导入，删除它
                if next_line.strip() == 'use std::sync::atomic::Ordering;':
                    print(f"  删除: {file_path.name}:{i+2} {next_line}")
                    changes += 1
                    i += 2
                    continue

        new_lines.append(line)
        i += 1

    if changes > 0:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write('\n'.join(new_lines))
        return changes
    return 0

def main():
    src_dir = Path('/Users/henry/code/amberjs/src')

    # 获取所有有 E0252 错误的文件
    result_files = [
        'monitor/performance_monitor.rs',
        'web_api/websocket.rs',
        'web_api/timers.rs',
        'ai_inference/model_cache.rs',
        'network/buffer_pool.rs',
        'network/zero_copy/batch_processor.rs',
        'network/zero_copy/receiver.rs',
        'memory/zero_copy_enhanced.rs',
    ]

    print("🔧 修复 E0252 重复导入错误...")
    total = 0

    for file_name in result_files:
        file_path = src_dir / file_name
        if file_path.exists():
            changes = fix_file(file_path)
            if changes > 0:
                print(f"✅ {file_name}: {changes} 处修改")
                total += changes
        else:
            print(f"⚠️  文件不存在: {file_name}")

    print(f"\n🎉 总共修复了 {total} 个重复导入")

if __name__ == '__main__':
    main()
