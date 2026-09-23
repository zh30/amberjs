#!/usr/bin/env python3
"""
修复所有空导入语法错误
"""

import re
from pathlib import Path

def fix_all_empty_imports(file_path):
    """修复单个文件中的所有空导入"""
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    # 修复模式 1: use module::{, };
    content = re.sub(r'use (\w+)::\{\s*,\s*\};', r'use \1::{};', content)

    # 修复模式 2: use module::{item, , };
    content = re.sub(r'use (\w+)::\{\s*([^,}]+?)\s*,\s*\};', r'use \1::{\2};', content)

    # 修复模式 3: use module::{, item};
    content = re.sub(r'use (\w+)::\{\s*,\s*([^,}]+?)\s*\};', r'use \1::{\2};', content)

    # 修复模式 4: use module::{item1, , item2};
    content = re.sub(r'use (\w+)::\{\s*([^,}]+?)\s*,\s*,\s*([^,}]+?)\s*\};', r'use \1::{\2, \3};', content)

    # 修复模式 5: use module::{item1, item2, , };
    content = re.sub(r'use (\w+)::\{\s*([^,}]+?)\s*,\s*([^,}]+?)\s*,\s*\};', r'use \1::{\2, \3};', content)

    # 修复模式 6: use module::{, item1, item2};
    content = re.sub(r'use (\w+)::\{\s*,\s*([^,}]+?)\s*,\s*([^,}]+?)\s*\};', r'use \1::{\2, \3};', content)

    # 修复模式 7: 多个连续逗号
    content = re.sub(r'use (\w+)::\{\s*,+\s*\};', r'use \1::{};', content)

    # 写入文件
    if content != original_content:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        return True

    return False

def main():
    """主函数"""
    src_dir = Path('/Users/henry/code/amberjs/src')
    fixed_count = 0

    print("🔧 修复所有空导入语法错误")
    print("=" * 60)

    for rs_file in src_dir.rglob('*.rs'):
        if fix_all_empty_imports(rs_file):
            fixed_count += 1
            print(f"✅ 修复: {rs_file.relative_to(src_dir)}")

    print("\n" + "=" * 60)
    print(f"✅ 完成！修复了 {fixed_count} 个文件")

if __name__ == '__main__':
    main()
