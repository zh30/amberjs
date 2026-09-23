#!/usr/bin/env python3
"""
修复特定的 use 语句错误
"""

import os
import re
from pathlib import Path

def fix_use_statements(file_path):
    """修复单个文件的 use 语句"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 修复模式1: use serde::::{Deserialize, Serialize} -> use serde::{Deserialize, Serialize}
        content = re.sub(r'use\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*::\s*\{', r'use \1::{', content)

        # 修复模式2: use serde{Deserialize, Serialize} -> use serde::{Deserialize, Serialize}
        content = re.sub(r'use\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\{', r'use \1::{', content)

        # 修复模式3: use std::collections::::{BTreeMap, HashMap} -> use std::collections::{BTreeMap, HashMap}
        content = re.sub(r'use\s+std::collections\s*::\s*\{', r'use std::collections::{', content)

        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            return True

        return False

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

    # 只修复有特定错误的文件
    error_files = [
        'benchmarks/mod.rs',
        'performance_analyzer.rs',
        'performance_regression.rs',
        'performance_comparison/mod.rs',
        'automation/mod.rs',
        'analysis/mod.rs',
    ]

    for error_file in error_files:
        file_path = src_dir / error_file
        if file_path.exists():
            if fix_use_statements(file_path):
                fixed_files.append(error_file)

    print(f"\n✅ Use 语句修复完成!")
    print(f"📊 修复文件数: {len(fixed_files)}")

    if fixed_files:
        print(f"\n📝 修复的文件:")
        for file_path in fixed_files:
            print(f"   - {file_path}")

if __name__ == '__main__':
    main()
