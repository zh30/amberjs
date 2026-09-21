#!/usr/bin/env python3
"""
全面修复编译错误脚本
"""

import re

def fix_file(filepath):
    """修复单个文件中的所有编译错误"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # 修复 module_resolver.rs 中的不可变借用问题
        if 'module_resolver.rs' in filepath:
            # 将 let resolver = 改为 let mut resolver =
            content = re.sub(
                r'let resolver = ModuleResolver::new',
                'let mut resolver = ModuleResolver::new',
                content
            )

        # 修复 repl_enhanced.rs 中的不可变借用问题
        if 'repl_enhanced.rs' in filepath:
            # 将 let temp_file = 改为 let mut temp_file =
            content = re.sub(
                r'let temp_file = NamedTempFile::new',
                'let mut temp_file = NamedTempFile::new',
                content
            )

        # 修复 llm_engine.rs 中的不可变借用问题
        if 'llm_engine.rs' in filepath:
            # 在测试函数中将 &engine 改为 &mut engine
            content = re.sub(
                r'let engine = LlmEngine::new',
                'let mut engine = LlmEngine::new',
                content
            )

        # 修复 inline_strategy.rs 中的不可变借用问题
        if 'inline_strategy.rs' in filepath:
            # 将 let strategy = 改为 let mut strategy =
            content = re.sub(
                r'let strategy = InlineStrategy::new',
                'let mut strategy = InlineStrategy::new',
                content
            )

        if content != original_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"✅ 修复了文件: {filepath}")
            return True
        else:
            print(f"⚪ 无需修改: {filepath}")
            return False

    except Exception as e:
        print(f"❌ 错误处理文件 {filepath}: {e}")
        return False

def main():
    """主函数"""
    files_to_fix = [
        '/Users/henry/code/amberjs/src/cli/module_resolver.rs',
        '/Users/henry/code/amberjs/src/cli/repl_enhanced.rs',
        '/Users/henry/code/amberjs/src/ai/llm_engine.rs',
        '/Users/henry/code/amberjs/src/jit/inline_strategy.rs',
    ]

    print("🔧 开始修复所有编译错误...\n")

    fixed_count = 0
    for filepath in files_to_fix:
        if fix_file(filepath):
            fixed_count += 1

    print(f"\n✨ 完成！共修复了 {fixed_count} 个文件")

    if fixed_count > 0:
        print("\n请重新运行测试:")
        print("  export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1")
        print("  cargo test --lib")
    else:
        print("\n✅ 所有文件已经是最新的！")

if __name__ == '__main__':
    main()
