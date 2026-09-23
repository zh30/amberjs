#!/usr/bin/env python3
"""
Amber 导入语法错误批量修复脚本 v2.0
修复所有 std::sync::atomic 导入相关的语法错误
"""

import os
import re
import sys
from pathlib import Path

def fix_import_syntax(file_path):
    """修复单个文件的导入语法错误"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        changes = []

        # 模式 1: 修复 use std::sync::atomic::Arc, , Mutex, ; 类型的错误
        # 正确语法: use std::sync::atomic::{Arc, Mutex};
        pattern1 = r'use std::sync::atomic::([A-Za-z_][A-Za-z0-9_]*)(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?(?:,\s*,?\s*([A-Za-z_][A-Za-z0-9_]*))?\s*;'
        # 更简单的方法：直接查找这种模式并提取所有标识符
        matches = re.findall(r'use std::sync::atomic::([A-Za-z_][A-Za-z0-9_]*(?:,\s*,?\s*[A-Za-z_][A-Za-z0-9_]*)*)\s*;', content)
        for match in matches:
            items = [item.strip().strip(',').strip() for item in match.split(',') if item.strip().strip(',').strip()]
            items = [item for item in items if item and not item.isspace()]
            if items:
                new_import = f"use std::sync::atomic::{{{', '.join(items)}}};"
                old_import = f"use std::sync::atomic::{match};"
                if old_import in content:
                    content = content.replace(old_import, new_import)
                    changes.append(f"  {old_import} -> {new_import}")

        # 模式 2: 修复 use std::sync{Arc, Mutex}; 缺少 :: 的错误
        pattern2 = r'use std::sync\{([A-Za-z_][A-Za-z0-9_,\s]*)\};'
        matches = re.findall(pattern2, content)
        for match in matches:
            items = [item.strip() for item in match.split(',') if item.strip()]
            if items:
                new_import = f"use std::sync::{{{', '.join(items)}}};"
                old_import = f"use std::sync{{{match}}};"
                if old_import in content:
                    content = content.replace(old_import, new_import)
                    changes.append(f"  {old_import} -> {new_import}")

        # 模式 3: 修复 use tokio::time::Duration as TokioDuration as TokioDuration; 重复 as 错误
        pattern3 = r'(use [^;]+)\s+as\s+([A-Za-z_][A-Za-z0-9_]*)\s+as\s+([A-Za-z_][A-Za-z0-9_]*);'
        matches = re.findall(pattern3, content)
        for match in matches:
            prefix = match[0]
            old_import = f"{prefix} as {match[1]} as {match[2]};"
            new_import = f"{prefix} as {match[2]};"
            content = content.replace(old_import, new_import)
            changes.append(f"  {old_import} -> {new_import}")

        # 模式 4: 修复 use std::sync::atomic:: ; 空导入
        pattern4 = r'use std::sync::atomic::\s*;'
        matches = re.findall(pattern4, content)
        for match in matches:
            # 删除空导入语句
            content = content.replace(match, '')
            changes.append(f"  Removed empty import: {match}")

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
    rust_files = list(project_root.rglob("*.rs"))

    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("=" * 80)
    print("Amber 导入语法错误批量修复脚本 v2.0")
    print("=" * 80)

    for file_path in rust_files:
        # 跳过备份文件
        if file_path.name.endswith('.bak'):
            continue

        total_files += 1
        fixed, changes = fix_import_syntax(file_path)

        if fixed:
            fixed_files += 1
            total_changes += len(changes)
            rel_path = file_path.relative_to(project_root)
            print(f"\n✅ Fixed: {rel_path}")
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
