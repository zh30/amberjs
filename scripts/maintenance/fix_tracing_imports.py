#!/usr/bin/env python3
"""
修复 tracing 导入问题
将未使用的 tracing 导入注释替换为实际使用的导入
"""

import os
import re

def fix_tracing_imports(file_path):
    """修复单个文件的 tracing 导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        # 查找 TODO 注释的 tracing 导入
        pattern = r'// TODO: Remove unused import: use tracing::\{([^}]+)\};'
        matches = re.findall(pattern, content)

        if matches:
            # 合并所有需要的 tracing 宏
            all_items = set()
            for match in matches:
                items = [item.strip() for item in match.split(',')]
                all_items.update(items)

            # 移除 TODO 注释
            content = re.sub(r'// TODO: Remove unused import: use tracing::\{[^}]+\};\n', '', content)

            # 在文件开头添加新的导入
            # 找到第一个 use 语句或模块注释之后
            insertion_point = 0
            lines = content.split('\n')
            for i, line in enumerate(lines):
                if line.startswith('//!') or line.startswith('//'):
                    insertion_point = i + 1
                elif line.startswith('use '):
                    insertion_point = i
                    break

            # 插入导入语句
            items_str = ', '.join(sorted(all_items))
            import_line = "use tracing::{" + items_str + "};\n"
            lines.insert(insertion_point, import_line)

            # 写回文件
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write('\n'.join(lines))

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
                fix_tracing_imports(file_path)

if __name__ == '__main__':
    main()
