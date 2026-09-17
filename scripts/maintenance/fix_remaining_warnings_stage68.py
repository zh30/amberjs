#!/usr/bin/env python3
"""
Stage 68 Phase 2: 清理剩余的编译警告
处理剩余的 37 个警告，主要包括:
1. ambiguous glob re-exports (模糊的 glob 重导出)
2. 未使用的导入
3. 未使用的变量
"""

import subprocess
import re
from pathlib import Path

def run_command(cmd):
    """执行命令并返回输出"""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    return result.stdout, result.stderr, result.returncode

def fix_glob_reexports():
    """修复模糊的 glob 重导出"""
    print("修复 ambiguous glob re-exports...")

    # 查找有问题的 mod.rs 文件
    src_dir = Path("src")
    mod_files = list(src_dir.rglob("mod.rs"))

    fixed = 0
    for mod_file in mod_files:
        with open(mod_file, 'r', encoding='utf-8') as f:
            content = f.read()

        original = content

        # 移除空的重导出行
        lines = content.split('\n')
        new_lines = []
        skip_next = False

        for i, line in enumerate(lines):
            # 检测空的重导出: pub use foo::bar;
            if 'pub use ' in line and line.strip().endswith(';'):
                # 检查下一行是否也是重导出
                if i + 1 < len(lines) and 'pub use ' in lines[i + 1]:
                    # 检查是否有实际导出
                    if '::' in line and not any(x in line for x in ['{', '}']):
                        # 这是空的重导出，跳过
                        continue

            new_lines.append(line)

        content = '\n'.join(new_lines)

        if content != original:
            with open(mod_file, 'w', encoding='utf-8') as f:
                f.write(content)
            fixed += 1
            print(f"  ✓ 修复: {mod_file}")

    print(f"修复了 {fixed} 个 glob re-export 问题\n")
    return fixed

def fix_specific_unused_imports():
    """修复特定的未使用导入"""
    print("修复特定的未使用导入...")

    files_to_fix = [
        ("src/runtime_lite/cache/mod.rs", [
            "pub use l1_zero_copy::L1ZeroCopyCache;",
            "pub use l2_smart::L2SmartCache;",
            "pub use l3_mmap::L3MmapCache;",
            "pub use prefetcher::PatternAnalyzer;",
        ]),
    ]

    fixed = 0
    for file_path, imports in files_to_fix:
        if not Path(file_path).exists():
            continue

        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original = content
        for imp in imports:
            if imp in content:
                content = content.replace(imp, f"// TODO: Remove unused export: {imp}")

        if content != original:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            fixed += 1
            print(f"  ✓ 修复: {file_path}")

    print(f"修复了 {fixed} 个文件\n")
    return fixed

def add_allow_warnings():
    """为无法修复的警告添加 allow 属性"""
    print("为无法自动修复的警告添加 allow 属性...")

    # 对于特定的警告，添加 allow 属性
    code_snippets = [
        ("src/runtime_lite/cache/mod.rs", [
            (r'pub\s+use\s+l1_zero_copy::L1ZeroCopyCache;', '#[allow(dead_code)]'),
            (r'pub\s+use\s+l2_smart::L2SmartCache;', '#[allow(dead_code)]'),
            (r'pub\s+use\s+l3_mmap::L3MmapCache;', '#[allow(dead_code)]'),
        ]),
    ]

    fixed = 0
    for file_path, patterns in code_to_fix:
        if not Path(file_path).exists():
            continue

        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        original = content
        for pattern, attribute in patterns:
            content = re.sub(pattern, f'{attribute}\n$1', content)

        if content != original:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(content)
            fixed += 1
            print(f"  ✓ 修复: {file_path}")

    print(f"修复了 {fixed} 个文件\n")
    return fixed

def main():
    print("=== Stage 68 Phase 2: 清理剩余警告 ===\n")

    # 步骤 1: 修复 glob re-exports
    print("步骤 1: 修复 ambiguous glob re-exports...")
    fix_glob_reexports()

    # 步骤 2: 修复特定导入
    print("步骤 2: 修复特定的未使用导入...")
    fix_specific_unused_imports()

    # 步骤 3: 验证修复
    print("步骤 3: 验证修复...")
    stdout, stderr, _ = run_command("cargo check 2>&1")
    remaining = [line for line in stdout.split('\n') if 'warning:' in line]
    print(f"剩余警告: {len(remaining)} 个")

    if remaining:
        print("\n剩余警告详情:")
        for warning in remaining[:10]:  # 只显示前 10 个
            print(f"  {warning}")

    # 步骤 4: 测试功能
    print("\n步骤 4: 测试基本功能...")
    stdout, stderr, code = run_command("./amber test_basic_functionality.js")
    if code == 0:
        print("✓ 基本功能测试通过")
    else:
        print(f"✗ 基本功能测试失败: {stderr}")

    print("\n=== Stage 68 Phase 2 完成 ===")
    print(f"剩余警告: {len(remaining)} 个")

if __name__ == "__main__":
    main()
