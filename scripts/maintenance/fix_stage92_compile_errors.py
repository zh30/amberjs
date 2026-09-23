#!/usr/bin/env python3
"""
修复 Stage 92 编译错误脚本
"""

import os
import re
import shutil

def move_ai_modules():
    """移动 AI 模块到正确的位置"""
    src_dir = "/Users/henry/code/amberjs/src"
    ai_dir = "/Users/henry/code/amberjs/src/ai"

    modules = [
        "ai_memory_pool.rs",
        "ai_batch_processor.rs",
        "ai_async_queue.rs",
    ]

    moved_count = 0
    for module in modules:
        src_file = os.path.join(src_dir, module)
        dst_file = os.path.join(ai_dir, module)

        if os.path.exists(src_file) and not os.path.exists(dst_file):
            shutil.move(src_file, dst_file)
            print(f"✅ 移动 {module} 到 {ai_dir}")
            moved_count += 1

    return moved_count

def create_model_interface():
    """创建缺失的 model_interface.rs"""
    ai_dir = "/Users/henry/code/amberjs/src/ai"
    interface_file = os.path.join(ai_dir, "model_interface.rs")

    if not os.path.exists(interface_file):
        content = """//! AI 模型接口模块
//! 提供统一的 AI 模型管理接口

use std::sync::Arc;
use std::time::Instant;

/// AI 模型接口特征
pub trait AIModelInterface: Send + Sync {
    /// 加载模型
    fn load_model(&self, model_path: &str) -> Result<(), Box<dyn std::error::Error>>;
    /// 运行推理
    fn infer(&self, input: &[f32]) -> Result<Vec<f32>, Box<dyn std::error::Error>>;
    /// 获取模型信息
    fn get_model_info(&self) -> ModelInfo;
}

/// 模型信息
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub input_shape: Vec<usize>,
    pub output_shape: Vec<usize>,
    pub parameters: usize,
}

/// 模型管理器
pub struct ModelManager {
    models: std::collections::HashMap<String, Arc<dyn AIModelInterface>>,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            models: std::collections::HashMap::new(),
        }
    }

    pub fn register_model(&mut self, name: String, model: Arc<dyn AIModelInterface>) {
        self.models.insert(name, model);
    }

    pub fn get_model(&self, name: &str) -> Option<&Arc<dyn AIModelInterface>> {
        self.models.get(name)
    }
}
"""
        with open(interface_file, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 创建了 {interface_file}")
        return 1
    return 0

def fix_instant_serialization():
    """修复 Instant 类型的序列化问题"""
    files_to_fix = [
        "/Users/henry/code/amberjs/src/ai/ai_performance_engine.rs",
        "/Users/henry/code/amberjs/src/ai/intelligent_scheduler.rs",
    ]

    fixed_count = 0
    for filepath in files_to_fix:
        if not os.path.exists(filepath):
            continue

        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 添加 chrono 依赖或在 derive 中移除 Instant
        # 这里我们选择使用 Option<Duration> 或 u64 来代替 Instant

        # 替换 Instant 类型为 u64 (Unix 时间戳)
        content = re.sub(r'timestamp: Instant,', 'timestamp: u64,', content)
        content = re.sub(r'created_at: Instant,', 'created_at: u64,', content)
        content = re.sub(r'deadline: Option<Instant>,', 'deadline: Option<u64>,', content)
        content = re.sub(r'estimated_start_time: Instant,', 'estimated_start_time: u64,', content)
        content = re.sub(r'estimated_completion_time: Instant,', 'estimated_completion_time: u64,', content)

        # 添加 Instant::now() 的 Unix 时间戳
        content = re.sub(r'Instant::now\(\)', 'chrono::Utc::now().timestamp() as u64', content)

        if content != original_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✅ 修复了 {filepath} 中的 Instant 序列化问题")
            fixed_count += 1

    return fixed_count

def fix_gc_event_type():
    """修复 GcEventType 枚举问题"""
    filepath = "/Users/henry/code/amberjs/src/memory/gc_optimizer_enhanced.rs"

    if not os.path.exists(filepath):
        return 0

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    # 添加缺失的 Standard 变体
    content = re.sub(
        r'(pub enum GcEventType \{)',
        r'''\1
    /// 标准 GC 事件
    Standard,''',
        content
    )

    # 修复 match 表达式
    if 'GcStrategy::Standard' not in content and 'match decision.strategy' in content:
        # 添加 Standard 分支
        content = re.sub(
            r'(GcStrategy::Emergency => self\.metrics\.emergency_collections\.fetch_add\(1, Ordering::Relaxed\),)',
            r'''\1
            GcStrategy::Standard => self.metrics.standard_collections.fetch_add(1, Ordering::Relaxed),''',
            content
        )

    if content != original_content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath} 中的 GcEventType 问题")
        return 1
    return 0

def fix_mmap_options():
    """修复 MmapOptions 构造问题"""
    filepath = "/Users/henry/code/amberjs/src/memory/zero_copy_enhanced.rs"

    if not os.path.exists(filepath):
        return 0

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    # 修复 MmapOptions::new() 调用
    content = re.sub(
        r'MmapOptions::new\(size\)',
        '{\n            let mut opts = MmapOptions::new();\n            opts.len(size)',
        content
    )

    if content != original_content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath} 中的 MmapOptions 问题")
        return 1
    return 0

def fix_ai_hardware_features():
    """修复 AiHardwareFeatures 导入问题"""
    filepath = "/Users/henry/code/amberjs/src/ai/matrix_accelerator.rs"

    if not os.path.exists(filepath):
        return 0

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    # 修复 AiHardwareFeatures 导入
    content = re.sub(
        r'use crate::ai::AiHardwareFeatures;',
        '// AiHardwareFeatures 在 mod.rs 中定义',
        content
    )

    # 注释掉或移除对 AiHardwareFeatures 的使用
    content = re.sub(
        r'pub struct MatrixAccelerator \{[^}]*AiHardwareFeatures[^}]*\}',
        lambda m: re.sub(r'AiHardwareFeatures[^,\n]*', '// AiHardwareFeatures', m.group(0)),
        content,
        flags=re.DOTALL
    )

    if content != original_content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"✅ 修复了 {filepath} 中的 AiHardwareFeatures 问题")
        return 1
    return 0

def main():
    """主函数"""
    print("🔧 开始修复 Stage 92 编译错误...\n")

    total_fixed = 0

    # 1. 移动 AI 模块
    print("1. 移动 AI 模块到正确位置...")
    total_fixed += move_ai_modules()

    # 2. 创建 model_interface
    print("\n2. 创建 model_interface 模块...")
    total_fixed += create_model_interface()

    # 3. 修复 Instant 序列化
    print("\n3. 修复 Instant 序列化问题...")
    total_fixed += fix_instant_serialization()

    # 4. 修复 GcEventType
    print("\n4. 修复 GcEventType 枚举问题...")
    total_fixed += fix_gc_event_type()

    # 5. 修复 MmapOptions
    print("\n5. 修复 MmapOptions 构造问题...")
    total_fixed += fix_mmap_options()

    # 6. 修复 AiHardwareFeatures
    print("\n6. 修复 AiHardwareFeatures 导入问题...")
    total_fixed += fix_ai_hardware_features()

    print(f"\n✨ 完成！共修复了 {total_fixed} 个问题")

    if total_fixed > 0:
        print("\n请重新运行编译:")
        print("  export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1")
        print("  cargo build --release")
    else:
        print("\n✅ 所有文件已经是最新的！")

if __name__ == '__main__':
    main()
