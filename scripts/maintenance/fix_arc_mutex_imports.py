#!/usr/bin/env python3
"""
修复 Arc/Mutex 导入路径错误
Arc 和 Mutex 应该在 std::sync 中，不在 std::sync::atomic 中
"""

import os
import re
from pathlib import Path

def fix_arc_mutex_imports(file_path):
    """修复 Arc/Mutex 的导入路径"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        changes = []

        # 修复 use std::sync::atomic::{Arc, Mutex}; -> use std::sync::{Arc, Mutex};
        pattern = r'use std::sync::atomic::\{([^}]+)\};'
        matches = re.findall(pattern, content)

        for match in matches:
            items = [item.strip() for item in match.split(',')]
            # 检查是否包含 Arc 或 Mutex
            has_arc_mutex = any(item in ['Arc', 'Mutex'] for item in items)

            if has_arc_mutex:
                # 分离 atomic 特有的项和其他项
                atomic_items = []
                sync_items = []

                for item in items:
                    if item in ['Arc', 'Mutex', 'RwLock']:
                        sync_items.append(item)
                    else:
                        atomic_items.append(item)

                # 构建新的导入语句
                new_imports = []
                if sync_items:
                    new_imports.append(f"use std::sync::{{{', '.join(sync_items)}}};")

                if atomic_items:
                    new_imports.append(f"use std::sync::atomic::{{{', '.join(atomic_items)}}};")

                old_import = f"use std::sync::atomic::{{{match}}};"
                new_import = '\n'.join(new_imports)

                if old_import in content:
                    content = content.replace(old_import, new_import)
                    changes.append(f"  {old_import} -> {new_import}")

        # 修复混合的导入：use std::sync::atomic::{Arc, Ordering};
        # 分成两个导入
        pattern2 = r'use std::sync::atomic::\{([^}]*Arc[^}]*)\};'
        matches2 = re.findall(pattern2, content)

        for match in matches2:
            items = [item.strip() for item in match.split(',')]
            atomic_items = []
            sync_items = []

            for item in items:
                if item in ['Arc', 'Mutex', 'RwLock']:
                    sync_items.append(item)
                else:
                    atomic_items.append(item)

            if sync_items and atomic_items:
                old_import = f"use std::sync::atomic::{{{match}}};"
                new_import = f"use std::sync::{{{', '.join(sync_items)}}};\nuse std::sync::atomic::{{{', '.join(atomic_items)}}};"

                if old_import in content:
                    content = content.replace(old_import, new_import)
                    changes.append(f"  {old_import} -> {new_import}")

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

    # 手动修复关键文件
    key_files = [
        "src/benchmarks/concurrent.rs",
        "src/runtime_lite.rs",
        "src/process_pool.rs",
        "src/concurrent_execution.rs",
        "src/shared_object_cache.rs",
        "src/network/memory_mapper.rs",
        "src/zero_copy.rs",
        "src/isolate_prewarmer.rs",
        "src/io/dma_engine.rs",
        "src/io/memory_mapper.rs",
        "src/memory/zero_copy.rs",
        "src/memory/gc_optimizer.rs",
        "src/lib_v8_simple.rs",
        "src/lib_minimal.rs",
        "src/async_io.rs",
        "src/isolate_pool.rs",
        "src/lock_free.rs",
        "src/hot_path_tracker.rs",
        "src/memory_optimizer/adaptive_gc.rs",
        "src/memory/zero_copy_allocator.rs",
        "src/memory/phase2_memory_engine.rs",
        "src/optimization/zero_copy_io.rs",
        "src/optimization/high_performance_core.rs",
        "src/network/stage93_batch_io_enhanced.rs",
        "src/network/buffer_pool.rs",
        "src/network/stage93_zero_copy_enhanced.rs",
        "src/wasm/memory_manager.rs",
        "src/wasm/js_interop.rs",
    ]

    total_files = 0
    fixed_files = 0
    total_changes = 0

    print("=" * 80)
    print("修复 Arc/Mutex 导入路径错误")
    print("=" * 80)

    for key_file in key_files:
        file_path = project_root / key_file
        if file_path.exists():
            total_files += 1
            fixed, changes = fix_arc_mutex_imports(file_path)
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
