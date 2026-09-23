#!/usr/bin/env python3
"""
仅添加缺失的导入，不修改现有的导入
"""

import os
import re
from pathlib import Path

# 定义需要检查的类型和对应的导入
TYPE_IMPORTS = {
    'Arc': 'std::sync::Arc',
    'Mutex': 'std::sync::Mutex',
    'RwLock': 'std::sync::RwLock',
    'AtomicBool': 'std::sync::atomic::AtomicBool',
    'AtomicUsize': 'std::sync::atomic::AtomicUsize',
    'Ordering': 'std::sync::atomic::Ordering',
    'HashMap': 'std::collections::HashMap',
    'HashSet': 'std::collections::HashSet',
    'VecDeque': 'std::collections::VecDeque',
    'Duration': 'std::time::Duration',
    'Instant': 'std::time::Instant',
    'SystemTime': 'std::time::SystemTime',
    'BTreeMap': 'std::collections::BTreeMap',
    'BTreeSet': 'std::collections::BTreeSet',
}

def file_needs_type(file_path, type_name):
    """检查文件是否使用了某个类型"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        # 检查是否已经导入了该类型
        import_pattern = rf'use\s+{re.escape(TYPE_IMPORTS[type_name])}'
        if re.search(import_pattern, content):
            return False

        # 检查是否在代码中使用了该类型
        # 简单的模式匹配:type_name
        # 但要排除注释中的使用
        lines = content.split('\n')
        for line in lines:
            line_stripped = line.strip()
            # 跳过注释行
            if line_stripped.startswith('//') or line_stripped.startswith('/*') or line_stripped.startswith('*'):
                continue

            # 检查是否在代码中使用
            if re.search(rf'\b{re.escape(type_name)}\b', line):
                # 排除导入语句
                if not line_stripped.startswith('use '):
                    return True

        return False

    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False

def add_missing_imports(file_path):
    """为文件添加缺失的导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        lines = content.split('\n')

        # 找出需要添加的类型
        types_to_add = []
        for type_name in TYPE_IMPORTS.keys():
            if file_needs_type(file_path, type_name):
                types_to_add.append(type_name)

        if not types_to_add:
            return False

        # 按模块分组
        imports_by_module = {}
        for type_name in types_to_add:
            module = TYPE_IMPORTS[type_name].split('::')[1]  # 获取模块名
            if module not in imports_by_module:
                imports_by_module[module] = []
            imports_by_module[module].append(type_name)

        # 生成导入语句
        import_lines = []
        for module in sorted(imports_by_module.keys()):
            types = sorted(imports_by_module[module])
            if types:
                # 合并同一模块的导入
                if len(types) > 1:
                    types_str = '{' + ', '.join(types) + '}'
                else:
                    types_str = types[0]
                import_lines.append(f'use std::{module}::{types_str};')

        # 在适当位置插入导入（use 语句区域）
        insert_index = 0
        found_use = False

        for i, line in enumerate(lines):
            line_stripped = line.strip()
            if line_stripped.startswith('use ') or line_stripped.startswith('pub use '):
                insert_index = i
                found_use = True
                break

        if found_use:
            # 在 use 区域插入
            lines[insert_index:insert_index] = [''] + import_lines + ['']
        else:
            # 在文档注释和 attrs 之后插入
            insert_index = 0
            for i, line in enumerate(lines):
                if (line.strip().startswith('//!') or
                    line.strip().startswith('///') or
                    line.strip().startswith('#[')):
                    insert_index = i + 1
                else:
                    break
            lines[insert_index:insert_index] = [''] + import_lines + ['']

        # 重新组合
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
        if add_missing_imports(rust_file):
            fixed_files.append(rust_file.relative_to(src_dir))

    print(f"\n✅ 缺失导入添加完成!")
    print(f"📊 统计信息:")
    print(f"   - 处理文件总数: {total_files}")
    print(f"   - 修复文件数: {len(fixed_files)}")
    print(f"   - 修复率: {len(fixed_files)/total_files*100:.1f}%")

    if fixed_files:
        print(f"\n📝 修复的文件:")
        for file_path in sorted(fixed_files)[:20]:
            print(f"   - {file_path}")
        if len(fixed_files) > 20:
            print(f"   ... 还有 {len(fixed_files) - 20} 个文件")

if __name__ == '__main__':
    main()
