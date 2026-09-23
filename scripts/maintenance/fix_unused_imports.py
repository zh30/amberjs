#!/usr/bin/env python3
"""
批量修复 TODO 注释中的未使用导入
"""

import os
import re

def fix_unused_imports(file_path):
    """修复单个文件的未使用导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 替换 TODO 注释的导入
        patterns = [
            (r'// TODO: Remove unused import: use (std::[^;]+);', r'use \1;'),
            (r'// TODO: Remove unused import: use (\{[^}]+\});', r'use \1;'),
            (r'// TODO: Remove unused import: use ([^;]+);', r'use \1;'),
        ]

        for pattern, replacement in patterns:
            content = re.sub(pattern, replacement, content)

        # 如果有变化，写回文件
        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"Fixed: {file_path}")
            return True
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False

def main():
    """主函数"""
    src_dir = '/Users/henry/code/amberjs/src'

    # 查找所有 Rust 文件
    for root, dirs, files in os.walk(src_dir):
        for file in files:
            if file.endswith('.rs'):
                file_path = os.path.join(root, file)
                fix_unused_imports(file_path)

if __name__ == '__main__':
    main()
