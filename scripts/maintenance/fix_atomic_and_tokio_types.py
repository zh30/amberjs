#!/usr/bin/env python3
"""
修复原子类型导入错误 (E0432)
- 修复错误的 std::sync::atomic::{Arc, Mutex, RwLock}
- 修复不存在的 TokioInstant, TokioDuration 类型
"""

import re
from pathlib import Path

def fix_atomic_imports(file_path):
    """修复文件中的原子类型导入错误"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 修复错误的原子类型导入
        # std::sync::atomic::Arc -> std::sync::Arc
        content = re.sub(r'std::sync::atomic::Arc', 'std::sync::Arc', content)

        # std::sync::atomic::Mutex -> std::sync::Mutex
        content = re.sub(r'std::sync::atomic::Mutex', 'std::sync::Mutex', content)

        # std::sync::atomic::RwLock -> std::sync::RwLock
        content = re.sub(r'std::sync::atomic::RwLock', 'std::sync::RwLock', content)

        # std::sync::atomic::Weak -> std::sync::Weak
        content = re.sub(r'std::sync::atomic::Weak', 'std::sync::Weak', content)

        # std::sync::atomic::Ordering -> std::sync::atomic::Ordering (这个是对的，保持不变)

        # 修复不存在的 Tokio 类型
        # TokioInstant -> std::time::Instant
        content = re.sub(r'tokio::time::TokioInstant', 'std::time::Instant', content)
        content = re.sub(r'std::time::TokioInstant', 'std::time::Instant', content)

        # TokioDuration -> std::time::Duration
        content = re.sub(r'tokio::time::TokioDuration', 'std::time::Duration', content)
        content = re.sub(r'std::time::TokioDuration', 'std::time::Duration', content)

        # 写回文件
        if content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"  修复原子类型和Tokio类型: {file_path}")
            return True

    except Exception as e:
        print(f"  错误处理文件 {file_path}: {e}")

    return False

def main():
    """主函数：扫描并修复所有 Rust 源文件"""
    src_dir = Path('/Users/henry/code/amberjs/src')
    fixed_count = 0
    total_files = 0

    print("=== 修复原子类型导入错误 (E0432) ===\n")
    print("修复:")
    print("  - std::sync::atomic::{Arc, Mutex, RwLock} -> std::sync::{Arc, Mutex, RwLock}")
    print("  - tokio::time::TokioInstant -> std::time::Instant")
    print("  - tokio::time::TokioDuration -> std::time::Duration")
    print()

    # 扫描所有 .rs 文件
    for rust_file in src_dir.rglob('*.rs'):
        total_files += 1
        if fix_atomic_imports(rust_file):
            fixed_count += 1

    print(f"\n=== 修复完成 ===")
    print(f"处理文件数: {total_files}")
    print(f"修复文件数: {fixed_count}")

if __name__ == '__main__':
    main()
