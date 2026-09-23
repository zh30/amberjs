#!/usr/bin/env python3
"""
自动添加缺少的标准库导入
"""

import re
from pathlib import Path

def add_missing_imports(file_path):
    """为单个文件添加缺少的导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        changes = []

        # 标准库导入映射
        std_imports = {
            'Duration': 'use std::time::Duration;',
            'Instant': 'use std::time::Instant;',
            'AtomicUsize': 'use std::sync::atomic::AtomicUsize;',
            'AtomicBool': 'use std::sync::atomic::AtomicBool;',
            'AtomicU64': 'use std::sync::atomic::AtomicU64;',
            'HashMap': 'use std::collections::HashMap;',
            'HashSet': 'use std::collections::HashSet;',
            'BTreeMap': 'use std::collections::BTreeMap;',
            'BTreeSet': 'use std::collections::BTreeSet;',
            'VecDeque': 'use std::collections::VecDeque;',
            'PathBuf': 'use std::path::PathBuf;',
            'Arc': 'use std::sync::Arc;',
            'Mutex': 'use std::sync::Mutex;',
            'RwLock': 'use std::sync::RwLock;',
        }

        # 收集现有的导入
        existing_imports = set()
        use_statements = []
        
        for match in re.finditer(r'use ([^;]+);', content):
            import_path = match.group(1)
            use_statements.append((match.start(), match.end(), import_path))
            
            # 提取导入的项目
            if '::' in import_path:
                parts = import_path.split('::')
                for part in parts:
                    if part and not part.startswith('{') and not part == '*':
                        existing_imports.add(part.split(' as ')[0].strip())

        # 查找缺少的类型并添加导入
        lines = content.split('\n')
        for typename, import_stmt in std_imports.items():
            if typename not in existing_imports:
                # 检查代码中是否使用了这个类型
                if any(typename in line and not line.strip().startswith('use ') for line in lines):
                    # 添加导入（在最后一个 use 语句之后）
                    if use_statements:
                        last_use_end = use_statements[-1][1]
                        content = content[:last_use_end] + '\n' + import_stmt + ';' + content[last_use_end:]
                        changes.append(f"  Added: {import_stmt}")

        if changes:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            return True, changes
        return False, []

    except Exception as e:
        print(f"Error processing {file_path}: {e}", file=sys.stderr)
        return False, [f"ERROR: {e}"]

def main():
    """主函数"""
    project_root = Path("/Users/henry/code/amberjs")

    # 处理核心源文件
    key_files = [
        "src/lib.rs",
        "src/runtime_core.rs",
        "src/runtime_lite.rs",
    ]

    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("=" * 80)
    print("添加缺少的标准库导入")
    print("=" * 80)

    for key_file in key_files:
        file_path = project_root / key_file
        if file_path.exists():
            total_files += 1
            fixed, changes = add_missing_imports(file_path)
            if fixed:
                fixed_files += 1
                total_changes += len(changes)
                print(f"\n✅ Fixed: {key_file}")
                for change in changes:
                    print(change)

    print("\n" + "=" * 80)
    print(f"修复完成！")
    print(f"总文件数: {total_files}")
    print(f"修复文件数: {fixed_files}")
    print(f"总修改数: {total_changes}")
    print("=" * 80)

if __name__ == "__main__":
    main()
