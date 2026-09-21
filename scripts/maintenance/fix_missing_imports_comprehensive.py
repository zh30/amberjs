#!/usr/bin/env python3
"""
批量修复 Rust 源代码中缺失的导入问题
修复 Arc, Mutex, RwLock, HashMap, AtomicUsize 等同步原语和标准库类型
"""

import os
import re
from pathlib import Path

# 定义需要添加的导入
IMPORTS_TO_ADD = {
    'Arc': 'use std::sync::Arc;',
    'Mutex': 'use std::sync::Mutex;',
    'RwLock': 'use std::sync::RwLock;',
    'AtomicBool': 'use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};',
    'AtomicUsize': 'use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};',
    'Ordering': 'use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};',
    'HashMap': 'use std::collections::{HashMap, HashSet};',
    'HashSet': 'use std::collections::{HashMap, HashSet};',
    'VecDeque': 'use std::collections::VecDeque;',
    'Duration': 'use std::time::{Duration, Instant, SystemTime};',
    'Instant': 'use std::time::{Duration, Instant, SystemTime};',
    'SystemTime': 'use std::time::{Duration, Instant, SystemTime};',
}

def fix_file_imports(file_path):
    """修复单个文件的导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        lines = content.split('\n')

        # 检查是否已经是 Rust 文件
        if not any(line.strip().startswith('use ') or line.strip().startswith('pub ') for line in lines[:50]):
            return False

        # 收集需要添加的导入
        needed_imports = []

        # 检查每个缺失的类型
        for type_name, import_line in IMPORTS_TO_ADD.items():
            # 检查是否已经导入
            import_already_exists = False
            for line in lines:
                if import_line in line or (type_name in line and 'use std::' in line):
                    # 检查是否已经包含了这个类型
                    if type_name in line:
                        import_already_exists = True
                        break

            if not import_already_exists:
                needed_imports.append(import_line)

        # 如果没有需要的导入，跳过
        if not needed_imports:
            return False

        # 去重导入
        needed_imports = list(set(needed_imports))

        # 找到合适的位置插入导入（use 语句区域）
        insert_index = 0
        found_use_section = False

        for i, line in enumerate(lines):
            line_stripped = line.strip()

            # 找到第一个 use 语句的位置
            if line_stripped.startswith('use ') or line_stripped.startswith('pub use '):
                found_use_section = True
                insert_index = i
                break

        if found_use_section:
            # 在 use 语句区域插入新导入
            # 先按模块分组
            imports_by_module = {}
            for imp in needed_imports:
                module = imp.split('::')[0].replace('use ', '')
                if module not in imports_by_module:
                    imports_by_module[module] = []
                imports_by_module[module].append(imp)

            # 在 use 区域按模块顺序插入
            lines_to_insert = []
            for module in sorted(imports_by_module.keys()):
                # 合并同一模块的导入
                imports = imports_by_module[module]
                # 去重
                imports = list(set(imports))
                lines_to_insert.extend(sorted(imports))

            # 在找到的位置插入
            lines[insert_index:insert_index] = [''] + lines_to_insert + ['']
        else:
            # 如果没有 use 语句，在顶部插入（跳过文档注释和 attrs）
            insert_index = 0
            for i, line in enumerate(lines):
                if line.strip().startswith('//!') or line.strip().startswith('///') or line.strip().startswith('#['):
                    insert_index = i + 1
                else:
                    break

            lines_to_insert = [''] + list(set(needed_imports)) + ['']
            lines[insert_index:insert_index] = lines_to_insert

        # 重新组合内容
        content = '\n'.join(lines)

        # 写入文件
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)

        return True

    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False

def main():
    """主函数"""
    src_dir = Path('/Users/henry/code/amberjs/src')

    if not src_dir.exists():
        print(f"Source directory not found: {src_dir}")
        return

    fixed_files = []
    total_files = 0

    # 遍历所有 Rust 文件
    for rust_file in src_dir.rglob('*.rs'):
        total_files += 1
        if fix_file_imports(rust_file):
            fixed_files.append(rust_file.relative_to(src_dir))

    print(f"\n✅ 批量导入修复完成!")
    print(f"📊 统计信息:")
    print(f"   - 处理文件总数: {total_files}")
    print(f"   - 修复文件数: {len(fixed_files)}")
    print(f"   - 修复率: {len(fixed_files)/total_files*100:.1f}%")

    if fixed_files:
        print(f"\n📝 修复的文件:")
        for file_path in sorted(fixed_files)[:20]:  # 只显示前 20 个
            print(f"   - {file_path}")
        if len(fixed_files) > 20:
            print(f"   ... 还有 {len(fixed_files) - 20} 个文件")

if __name__ == '__main__':
    main()
