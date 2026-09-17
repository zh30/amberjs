#!/usr/bin/env python3
"""
Amber Stage 61 - 编译警告清理工具
清理未使用的导入、未使用的变量和可变绑定
"""

import re
import subprocess
import sys
from pathlib import Path
from typing import List, Tuple, Set

def run_cargo_check() -> str:
    """运行 cargo check 获取警告列表"""
    result = subprocess.run(
        ["cargo", "check"],
        capture_output=True,
        text=True,
        cwd="/Users/henry/code/amberjs"
    )
    return result.stderr

def parse_warnings(output: str) -> List[Tuple[str, int, str]]:
    """解析警告输出，提取文件、行号、警告类型和消息"""
    warnings = []
    # 匹配格式: warning: unused import: `XXX` --> file:line:col
    pattern = r'warning:\s+(.+?)\s+-->\s+([^:]+):(\d+):\d+'
    for match in re.finditer(pattern, output):
        warning_type = match.group(1)
        file_path = match.group(2)
        line_num = int(match.group(3))
        warnings.append((file_path, line_num, warning_type))
    return warnings

def fix_unused_import(file_path: Path, line_num: int) -> bool:
    """修复未使用的导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        if line_num - 1 >= len(lines):
            return False

        line = lines[line_num - 1]

        # 匹配 use 语句
        use_pattern = r'^\s*use\s+(.+?);'
        match = re.match(use_pattern, line.strip())

        if not match:
            return False

        # 检查是否是复杂导入（多项目）
        import_content = match.group(1)

        # 如果是单项目导入，删除整行
        if ',' not in import_content and '::{' not in import_content:
            lines[line_num - 1] = ''
            with open(file_path, 'w', encoding='utf-8') as f:
                f.writelines(lines)
            return True

        # 如果是多项目导入，只删除未使用的项目
        # 这里需要更复杂的解析，暂时跳过
        return False

    except Exception as e:
        print(f"Error fixing {file_path}:{line_num}: {e}")
        return False

def fix_unused_variable(file_path: Path, line_num: int) -> bool:
    """修复未使用的变量（添加下划线前缀）"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        if line_num - 1 >= len(lines):
            return False

        line = lines[line_num - 1]

        # 匹配 let 语句
        # 处理: let var_name =
        # 处理: let mut var_name =
        pattern = r'(\s*)let(\s+mut)?\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*='
        match = re.search(pattern, line)

        if not match:
            return False

        indent = match.group(1)
        is_mut = match.group(2)
        var_name = match.group(3)

        # 如果变量名以下划线开头，说明已经被处理过
        if var_name.startswith('_'):
            return False

        # 添加下划线前缀
        new_line = f"{indent}let{is_mut} _{var_name} ="
        lines[line_num - 1] = line[:match.start()] + new_line + line[match.end():]

        with open(file_path, 'w', encoding='utf-8') as f:
            f.writelines(lines)
        return True

    except Exception as e:
        print(f"Error fixing {file_path}:{line_num}: {e}")
        return False

def fix_unused_mut(file_path: Path, line_num: int) -> bool:
    """修复不需要的可变绑定"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        if line_num - 1 >= len(lines):
            return False

        line = lines[line_num - 1]

        # 匹配 let mut 并移除 mut
        pattern = r'(\s*)let\s+mut\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*='
        match = re.search(pattern, line)

        if not match:
            return False

        indent = match.group(1)
        var_name = match.group(2)

        new_line = f"{indent}let {var_name} ="
        lines[line_num - 1] = line[:match.start()] + new_line + line[match.end():]

        with open(file_path, 'w', encoding='utf-8') as f:
            f.writelines(lines)
        return True

    except Exception as e:
        print(f"Error fixing {file_path}:{line_num}: {e}")
        return False

def main():
    """主函数"""
    print("🚀 Amber Stage 61 - 编译警告清理工具")
    print("=" * 60)

    # 运行 cargo check 获取警告
    print("\n📊 步骤 1: 分析编译警告...")
    output = run_cargo_check()
    warnings = parse_warnings(output)

    print(f"找到 {len(warnings)} 个警告")

    # 分类警告
    unused_imports = [(f, l, w) for f, l, w in warnings if 'unused import' in w]
    unused_variables = [(f, l, w) for f, l, w in warnings if 'unused variable' in w]
    unused_mut = [(f, l, w) for f, l, w in warnings if 'does not need to be mutable' in w]

    print(f"  - 未使用导入: {len(unused_imports)}")
    print(f"  - 未使用变量: {len(unused_variables)}")
    print(f"  - 不需要的可变: {len(unused_mut)}")

    # 修复警告
    fixed_count = 0

    # 1. 修复未使用的变量（最安全）
    print("\n🔧 步骤 2: 修复未使用的变量...")
    for file_path, line_num, _ in unused_variables:
        if fix_unused_variable(Path(file_path), line_num):
            fixed_count += 1

    print(f"  修复了 {fixed_count} 个未使用变量")

    # 2. 修复不需要的可变
    print("\n🔧 步骤 3: 修复不需要的可变绑定...")
    mut_fixed = 0
    for file_path, line_num, _ in unused_mut:
        if fix_unused_mut(Path(file_path), line_num):
            mut_fixed += 1

    print(f"  修复了 {mut_fixed} 个不需要的可变绑定")

    # 3. 对于未使用的导入，使用更保守的方法
    # 只处理单项目的导入
    print("\n🔧 步骤 4: 清理未使用的单项目导入...")
    import_fixed = 0
    for file_path, line_num, _ in unused_imports:
        if fix_unused_import(Path(file_path), line_num):
            import_fixed += 1

    print(f"  修复了 {import_fixed} 个未使用导入")

    # 运行 cargo check 验证结果
    print("\n✅ 步骤 5: 验证修复结果...")
    output = run_cargo_check()
    new_warnings = parse_warnings(output)

    print(f"\n📊 清理结果:")
    print(f"  修复前: {len(warnings)} 个警告")
    print(f"  修复后: {len(new_warnings)} 个警告")
    print(f"  减少: {len(warnings) - len(new_warnings)} 个警告")
    print(f"  成功率: {((len(warnings) - len(new_warnings)) / len(warnings) * 100):.1f}%")

    if len(new_warnings) < len(warnings):
        print("\n✅ 清理成功！请运行 'cargo check' 查看剩余警告")
    else:
        print("\n⚠️  没有清理任何警告，可能需要手动处理")

if __name__ == "__main__":
    main()
