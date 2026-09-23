#!/usr/bin/env python3
"""
修复 Stage 92 剩余编译错误
"""

import re

def fix_ai_performance_engine():
    """修复 ai_performance_engine.rs 中的错误"""
    filepath = "/Users/henry/code/amberjs/src/ai/ai_performance_engine.rs"

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    # 1. 修复 predict 方法调用 - 添加 deref
    content = re.sub(
        r'let prediction = predictor\.predict\(&history\)\?;',
        'let prediction = predictor.predict(&*history)?;',
        content
    )

    # 2. 修复 Duration 运算 - 转换为 u64
    content = re.sub(
        r'let cutoff = chrono::Utc::now\(\)\.timestamp\(\) as u64 - duration;',
        'let cutoff = chrono::Utc::now().timestamp() as u64 - duration.as_secs();',
        content
    )

    # 3. 修复 lock 方法调用 - lock 是字段不是方法
    content = re.sub(
        r'let mut optimizer_guard = optimizer\.lock\(\)\.unwrap\(\);',
        'let mut optimizer_guard = &mut *optimizer.lock().unwrap();',
        content
    )

    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath}")
        return 1
    return 0

def fix_intelligent_scheduler():
    """修复 intelligent_scheduler.rs 中的错误"""
    filepath = "/Users/henry/code/amberjs/src/ai/intelligent_scheduler.rs"

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    # 1. 修复 task 移动问题 - 使用 clone
    content = re.sub(
        r'queue\.push_back\(task\);',
        'queue.push_back(task.clone());',
        content
    )

    # 2. 修复 last_active 类型不匹配
    content = re.sub(
        r'workers\[worker_idx\]\.last_active = chrono::Utc::now\(\)\.timestamp\(\) as u64;',
        'workers[worker_idx].last_active = std::time::Instant::now();',
        content
    )

    # 3. 修复 workers 迭代器问题 - 使用 *
    content = re.sub(
        r'for worker in &mut workers \{',
        'for worker in &mut *workers {',
        content
    )

    # 4. 修复构造中的 last_active
    content = re.sub(
        r'last_active: chrono::Utc::now\(\)\.timestamp\(\) as u64,',
        'last_active: std::time::Instant::now(),',
        content
    )

    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath}")
        return 1
    return 0

def fix_zero_copy_enhanced():
    """修复 zero_copy_enhanced.rs 中的错误"""
    filepath = "/Users/henry/code/amberjs/src/memory/zero_copy_enhanced.rs"

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    # 1. 修复缓存借用问题 - 使用 get_mut 或克隆
    content = re.sub(
        r'if let Some\(mmap\) = cache\.get\(path\) \{',
        'if let Some(mmap) = cache.get_mut(path) {',
        content
    )

    # 2. 修复 madvise 参数 - 移除 as *const
    content = re.sub(
        r'addr\.as_ptr\(\) as \*const c_void,',
        'addr.as_ptr(),',
        content
    )

    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath}")
        return 1
    return 0

def fix_gc_optimizer_enhanced():
    """修复 gc_optimizer_enhanced.rs 中的 match 错误"""
    filepath = "/Users/henry/code/amberjs/src/memory/gc_optimizer_enhanced.rs"

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    # 修复 match 分支类型不匹配 - 添加分号
    content = re.sub(
        r'(GcStrategy::Emergency => self\.metrics\.emergency_collections\.fetch_add\(1, Ordering::Relaxed\),)',
        r'\1;',
        content
    )
    content = re.sub(
        r'(GcStrategy::Predictive => self\.metrics\.predictive_collections\.fetch_add\(1, Ordering::Relaxed\),)',
        r'\1;',
        content
    )

    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath}")
        return 1
    return 0

def main():
    """主函数"""
    print("🔧 开始修复 Stage 92 编译错误...\n")

    total_fixed = 0

    total_fixed += fix_ai_performance_engine()
    total_fixed += fix_intelligent_scheduler()
    total_fixed += fix_zero_copy_enhanced()
    total_fixed += fix_gc_optimizer_enhanced()

    print(f"\n✨ 完成！共修复了 {total_fixed} 个文件")

    if total_fixed > 0:
        print("\n请重新运行编译:")
        print("  export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1")
        print("  cargo build --release")
    else:
        print("\n✅ 无需修复！")

if __name__ == '__main__':
    main()
