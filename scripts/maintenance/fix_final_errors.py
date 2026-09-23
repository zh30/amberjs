#!/usr/bin/env python3
"""
修复最后的编译错误
"""

import re

def fix_ai_performance_engine_final():
    """修复 ai_performance_engine.rs 的最终错误"""
    filepath = "/Users/henry/code/amberjs/src/ai/ai_performance_engine.rs"

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    # 1. 修复 history 类型
    content = re.sub(
        r'let prediction = predictor\.predict\(&\*history\)\?;',
        'let prediction = predictor.predict(history.as_slice())?;',
        content
    )

    # 2. 修复 optimizer.lock() 问题
    content = re.sub(
        r'let mut optimizer_guard = &mut \*optimizer\.lock\(\)\.unwrap\(\);',
        '''{
            let optimizer_clone = Arc::clone(&optimizer);
            let mut optimizer_guard = optimizer_clone.lock().unwrap();
            optimizer_guard.optimize(&history_data).await;
        }''',
        content
    )

    # 3. 修复 TensorOptimizer::new() 参数
    content = re.sub(
        r'tensor_optimizer: Arc::new\(Mutex::new\(TensorOptimizer::new\(config\.clone\(\)\)\)\),',
        'tensor_optimizer: Arc::new(Mutex::new(TensorOptimizer::new())),',
        content
    )

    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath}")
        return 1
    return 0

def fix_intelligent_scheduler_final():
    """修复 intelligent_scheduler.rs 的最终错误"""
    filepath = "/Users/henry/code/amberjs/src/ai/intelligent_scheduler.rs"

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    # 修复 Duration 运算
    content = re.sub(
        r'let estimated_completion_time = now \+ Duration::from_millis\(task\.estimated_duration\);',
        'let estimated_completion_time = now + std::time::Duration::from_millis(task.estimated_duration);',
        content
    )

    # 修复 self 移动问题
    content = re.sub(
        r'pub fn start_background_tasks\(self: Arc<Self>\) \{',
        'pub fn start_background_tasks(self: &Arc<Self>) {',
        content
    )

    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath}")
        return 1
    return 0

def fix_zero_copy_final():
    """修复 zero_copy_enhanced.rs 的最终错误"""
    filepath = "/Users/henry/code/amberjs/src/memory/zero_copy_enhanced.rs"

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content

    # 修复缓存借用问题 - 使用 clone
    content = re.sub(
        r'if let Some\(mmap\) = cache\.get_mut\(path\) \{',
        'if let Some(mmap) = cache.get(path).cloned() {',
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
    print("🔧 开始修复最后的编译错误...\n")

    total_fixed = 0

    total_fixed += fix_ai_performance_engine_final()
    total_fixed += fix_intelligent_scheduler_final()
    total_fixed += fix_zero_copy_final()

    print(f"\n✨ 完成！共修复了 {total_fixed} 个文件")

    if total_fixed > 0:
        print("\n请重新运行编译:")
        print("  export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1")
        print("  cargo build --release")
    else:
        print("\n✅ 无需修复！")

if __name__ == '__main__':
    main()
