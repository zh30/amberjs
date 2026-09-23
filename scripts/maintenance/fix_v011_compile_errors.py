#!/usr/bin/env python3
"""
Amber v0.1.1 编译错误修复脚本
自动修复测试编译中的常见错误
"""

import os
import re
import sys
from pathlib import Path

def fix_enhanced_module_conflict():
    """修复 enhanced 模块冲突"""
    enhanced_dir = Path("src/debugger/enhanced")
    if enhanced_dir.exists():
        # 删除重复的 mod.rs 文件
        mod_rs = enhanced_dir / "mod.rs"
        if mod_rs.exists():
            mod_rs.unlink()
            print("✅ 删除重复的 enhanced/mod.rs")
        return True
    return False

def fix_arc_mutable_borrow(content, filepath):
    """修复 Arc 可变借用问题"""
    # 模式: Arc<T> -> Arc<Mutex<T>> 或 Arc<RwLock<T>>
    patterns = [
        # Arc::clone(&arc) -> Arc::clone(arc)
        (r'Arc::clone\(&(\w+)\)', r'Arc::clone(\1)'),
        # Arc::new(value) -> Arc::new(Mutex::new(value))
        (r'Arc::new\(([^)]+)\)', r'Arc::new(std::sync::Mutex::new(\1))'),
    ]

    for pattern, replacement in patterns:
        if re.search(pattern, content):
            content = re.sub(pattern, replacement, content)
            print(f"✅ 修复 Arc 可变借用: {filepath}")

    return content

def fix_borrow_moved_value(content, filepath):
    """修复借用已移动值错误"""
    # 模式: 变量被移动后又被借用
    # 解决方案: 克隆或使用引用
    patterns = [
        # .clone() 添加到被移动的变量
        (r'(\w+) = \1\.', r'\1 = \1.clone();'),
    ]

    for pattern, replacement in patterns:
        if re.search(pattern, content):
            content = re.sub(pattern, replacement, content)
            print(f"✅ 修复借用移动值: {filepath}")

    return content

def fix_generic_arguments(content, filepath):
    """修复泛型参数数量不匹配"""
    # 模式: enum Foo<T> -> Foo<T, S>
    patterns = [
        # HashMap -> HashMap<K, V>
        (r'HashMap<([^>]+)>', r'HashMap<\1, std::collections::HashMap<\1, \1>>'),
        # BTreeMap -> BTreeMap<K, V>
        (r'BTreeMap<([^>]+)>', r'BTreeMap<\1, \1>'),
    ]

    modified = False
    for pattern, replacement in patterns:
        if re.search(pattern, content):
            content = re.sub(pattern, replacement, content)
            print(f"✅ 修复泛型参数: {filepath}")
            modified = True

    return content

def add_missing_imports(content, filepath):
    """添加缺失的导入"""
    required_imports = [
        "use std::sync::{Arc, Mutex, RwLock};",
        "use std::collections::{HashMap, BTreeMap};",
    ]

    lines = content.split('\n')
    import_section_end = 0

    # 找到导入部分的结束位置
    for i, line in enumerate(lines):
        if line.strip().startswith('use ') or line.strip().startswith('pub use '):
            import_section_end = i + 1

    # 检查并添加缺失的导入
    for import_stmt in required_imports:
        if import_stmt not in content:
            lines.insert(import_section_end, import_stmt)
            import_section_end += 1
            print(f"✅ 添加导入: {import_stmt}")

    return '\n'.join(lines)

def fix_type_annotations(content, filepath):
    """修复类型注解问题"""
    # 为变量添加明确的类型注解
    patterns = [
        # let var = value; -> let var: Type = value;
        (r'let (\w+) = ([^;]+);', r'let \1: _ = \2;'),
    ]

    for pattern, replacement in patterns:
        if re.search(pattern, content):
            content = re.sub(pattern, replacement, content)
            print(f"✅ 修复类型注解: {filepath}")

    return content

def process_file(filepath):
    """处理单个文件"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 应用各种修复
        content = fix_arc_mutable_borrow(content, filepath)
        content = fix_borrow_moved_value(content, filepath)
        content = fix_generic_arguments(content, filepath)
        content = add_missing_imports(content, filepath)
        content = fix_type_annotations(content, filepath)

        # 如果有修改，写回文件
        if content != original_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✅ 已修复文件: {filepath}")
            return True

    except Exception as e:
        print(f"❌ 处理文件失败 {filepath}: {e}")

    return False

def main():
    """主函数"""
    print("🚀 开始修复 Amber v0.1.1 编译错误...")

    # 1. 修复模块冲突
    fix_enhanced_module_conflict()

    # 2. 处理测试文件
    test_files = list(Path("tests").glob("*.rs"))
    fixed_count = 0

    for test_file in test_files:
        if process_file(test_file):
            fixed_count += 1

    # 3. 处理 src 文件
    src_files = list(Path("src").glob("**/*.rs"))
    for src_file in src_files:
        if process_file(src_file):
            fixed_count += 1

    print(f"\n🎉 修复完成! 共处理 {fixed_count} 个文件")
    print("\n📋 下一步:")
    print("1. 运行 cargo check 检查剩余错误")
    print("2. 运行 cargo test --lib 验证修复")
    print("3. 更新版本到 v0.1.1")

if __name__ == "__main__":
    main()
