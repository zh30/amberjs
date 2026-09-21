#!/usr/bin/env python3
"""
修复 Amber 项目中的所有括号不匹配错误
"""

import os
import re
import sys

def fix_box_new_prometheus(content):
    """修复 Box::new(PrometheusCollector::new( 缺少闭合括号的问题"""
    # 修复 Box::new(PrometheusCollector::new(... 的模式
    # 匹配类似 Box::new(PrometheusCollector::new(...) 这样的情况，添加缺失的 ))
    pattern = r'(Box::new\(\s*PrometheusCollector::new\([^)]*\))(\s*,?\s*\))'
    fixed = re.sub(pattern, r'\1))\2', content)

    # 另一种情况：直接在 ) 后添加 )
    pattern2 = r'(Box::new\(\s*PrometheusCollector::new\([^)]*\))(\s*,?\s*)\)'
    fixed = re.sub(pattern2, r'\1))\2', fixed)

    return fixed

def fix_assert_eq_string(content):
    """修复 assert_eq 中的 .to_string() 缺少 ) 问题"""
    # 修复 assert_eq!(..., Some(&"...".to_string(); 为 assert_eq!(..., Some(&"...".to_string()));
    pattern = r'assert_eq!\([^;]*?Some\(&"[^"]*?"\.to_string\(\);'
    def add_closing_paren(match):
        content = match.group(0)
        if not content.endswith('))'):
            content = content.rstrip(';') + '));'
        return content

    fixed = re.sub(pattern, add_closing_paren, content, flags=re.MULTILINE | re.DOTALL)

    return fixed

def process_file(file_path):
    """处理单个文件"""
    print(f"处理文件: {file_path}")

    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    # 应用修复
    content = fix_box_new_prometheus(content)
    content = fix_assert_eq_string(content)

    if content != original_content:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  ✓ 已修复: {file_path}")
        return True
    else:
        print(f"  - 无需修复: {file_path}")
        return False

def main():
    """主函数"""
    print("开始修复所有括号不匹配错误...")

    # 查找所有 Rust 文件
    files_to_fix = []
    for root, dirs, files in os.walk('src'):
        for file in files:
            if file.endswith('.rs'):
                files_to_fix.append(os.path.join(root, file))

    fixed_count = 0
    for file_path in files_to_fix:
        if process_file(file_path):
            fixed_count += 1

    print(f"\n修复完成！共修复了 {fixed_count} 个文件")

if __name__ == '__main__':
    main()
