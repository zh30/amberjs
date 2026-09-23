#!/usr/bin/env python3
"""
Stage 68: 清理编译警告
清理 Amber 项目中的编译警告，提升代码质量

警告类型:
- unused imports (未使用的导入)
- unused variables (未使用的变量)
- clippy lints (clippy 规范)

策略:
1. 使用 rustc --explain 了解警告
2. 批量修复常见的未使用导入
3. 标记不可移除的代码避免警告
4. 验证修复不影响功能
"""

import subprocess
import re
import os
from pathlib import Path

def run_command(cmd):
    """执行命令并返回输出"""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return result.stdout, result.stderr, result.returncode

def get_compile_warnings():
    """获取编译警告列表"""
    stdout, stderr, _ = run_command("cargo check 2>&1")
    warnings = []
    for line in stdout.split('\n'):
        if 'warning:' in line:
            warnings.append(line.strip())
    return warnings

def categorize_warnings(warnings):
    """分类警告"""
    categories = {
        'unused_import': [],
        'unused_variable': [],
        'clippy_lint': [],
        'dead_code': [],
        'other': []
    }

    for warning in warnings:
        if 'unused import:' in warning:
            categories['unused_import'].append(warning)
        elif 'unused variable:' in warning:
            categories['unused_variable'].append(warning)
        elif 'clippy' in warning.lower():
            categories['clippy_lint'].append(warning)
        elif 'dead_code' in warning:
            categories['dead_code'].append(warning)
        else:
            categories['other'].append(warning)

    return categories

def extract_import_from_warning(warning):
    """从警告中提取导入名称"""
    # 格式: warning: unused import: `SomeImport` ...
    match = re.search(r'`([^`]+)`', warning)
    if match:
        return match.group(1)
    return None

def fix_unused_imports(warnings):
    """修复未使用的导入"""
    imports_to_fix = set()

    for warning in warnings['unused_import']:
        imp = extract_import_from_warning(warning)
        if imp:
            imports_to_fix.add(imp)

    print(f"发现 {len(imports_to_fix)} 个未使用的导入")

    # 读取每个源文件并修复
    src_dir = Path("src")
    rust_files = list(src_dir.rglob("*.rs"))

    fixed_count = 0
    for rust_file in rust_files:
        with open(rust_file, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 修复未使用的导入
        for imp in imports_to_fix:
            # 匹配完整的 use 语句行
            patterns = [
                rf'use\s+[^;]*\b{re.escape(imp)}\b[^;]*;',
                rf'pub\s+use\s+[^;]*\b{re.escape(imp)}\b[^;]*;',
            ]

            for pattern in patterns:
                matches = re.findall(pattern, content, re.MULTILINE)
                for match in matches:
                    # 检查是否是唯一导入
                    if match.count('\n') == 0:  # 单行导入
                        # 如果导入项在多项目导入中，移除该项
                        if '::{' in match or ', ' in match:
                            content = content.replace(match, f"// TODO: Remove unused import: {match}")

        if content != original_content:
            with open(rust_file, 'w', encoding='utf-8') as f:
                f.write(content)
            fixed_count += 1
            print(f"✓ 修复文件: {rust_file}")

    print(f"总共修复了 {fixed_count} 个文件")
    return fixed_count

def add_allow_attributes(warnings):
    """为无法移除的代码添加 allow 属性"""
    # 对于无法自动修复的警告，添加 allow 属性
    pass

def main():
    print("=== Stage 68: 清理编译警告 ===\n")

    # 步骤 1: 获取当前警告
    print("步骤 1: 分析编译警告...")
    warnings = get_compile_warnings()
    print(f"总共发现 {len(warnings)} 个警告\n")

    # 步骤 2: 分类警告
    print("步骤 2: 分类警告...")
    categorized = categorize_warnings(warnings)
    for category, warns in categorized.items():
        if warns:
            print(f"  {category}: {len(warns)} 个")
    print()

    # 步骤 3: 修复未使用的导入
    print("步骤 3: 修复未使用的导入...")
    if categorized['unused_import']:
        fix_unused_imports(categorized)
    else:
        print("  没有发现未使用的导入")
    print()

    # 步骤 4: 验证修复
    print("步骤 4: 验证修复...")
    stdout, stderr, code = run_command("cargo check 2>&1")
    new_warnings = [line for line in stdout.split('\n') if 'warning:' in line]
    print(f"修复后剩余警告: {len(new_warnings)} 个")
    print(f"减少了 {len(warnings) - len(new_warnings)} 个警告")

    # 步骤 5: 测试基本功能
    print("\n步骤 5: 测试基本功能...")
    test_script = "test_basic_functionality.js"
    if os.path.exists(test_script):
        stdout, stderr, code = run_command(f"./amber {test_script}")
        if code == 0:
            print("✓ 基本功能测试通过")
        else:
            print(f"✗ 基本功能测试失败: {stderr}")
    else:
        print("⚠ 测试脚本不存在，跳过功能测试")

    print("\n=== Stage 68 完成 ===")
    print(f"警告数量: {len(warnings)} → {len(new_warnings)}")
    print(f"改进: {((len(warnings) - len(new_warnings)) / len(warnings) * 100):.1f}%")

if __name__ == "__main__":
    main()
