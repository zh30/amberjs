#!/usr/bin/env python3
"""
Fix remaining use syntax errors: ::< should be ::{
Pattern: use module::<Item> -> use module::{Item}
"""

import os
import re
from pathlib import Path

def fix_use_syntax_errors(file_path):
    """Fix use statements with ::< syntax error."""
    try:
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
    except:
        return False

    original = content

    # Pattern: use something::<...> -> use something::{...}
    # This handles multi-line use statements too

    # Fix single-line: use path::<Item1, Item2>;
    content = re.sub(
        r'(use\s+[\w:]+)::(<[^;]+);',
        lambda m: m.group(1) + '::{' + m.group(2)[1:].rstrip('>') + '};',
        content
    )

    # Fix: pub use path::<...>;
    content = re.sub(
        r'(pub\s+use\s+[\w:]+)::(<[^;]+);',
        lambda m: m.group(1) + '::{' + m.group(2)[1:].rstrip('>') + '};',
        content
    )

    # Fix multi-line use statements that start with ::<
    # Pattern: use path::<\n    Item1,\n    Item2,\n>;
    def fix_multiline_use(match):
        prefix = match.group(1)
        items = match.group(2)
        # Remove leading < and trailing >
        items = items.strip()
        if items.startswith('<'):
            items = items[1:]
        if items.endswith('>'):
            items = items[:-1]
        return prefix + '::{\n' + items + '\n}'

    content = re.sub(
        r'((?:pub\s+)?use\s+[\w:]+)::<\s*\n([\s\S]*?)\n\s*>;',
        fix_multiline_use,
        content
    )

    # Simpler approach: just replace ::< with ::{ and >; with };
    # But only in use statements context
    lines = content.split('\n')
    fixed_lines = []
    in_use_block = False

    for i, line in enumerate(lines):
        # Check if this line starts a use statement with ::<
        if re.match(r'\s*(pub\s+)?use\s+.*::<', line):
            in_use_block = True

        if in_use_block or '::<' in line:
            # Replace ::< with ::{
            line = re.sub(r'::(<)(\w)', r'::{\2', line)
            line = re.sub(r'::(<)\s*$', '::{', line)
            line = re.sub(r'::(<)(\s+\w)', r'::{\2', line)

        # Replace >; with }; only in use context
        if in_use_block and re.match(r'\s*>;', line):
            line = line.replace('>;', '};')
            in_use_block = False

        # Single line fix: >; at end of use statement
        if 'use ' in line and '>;' in line:
            line = line.replace('>;', '};')

        fixed_lines.append(line)

    content = '\n'.join(fixed_lines)

    if content != original:
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        return True
    return False

def main():
    src_dir = Path('/Users/henry/code/amberjs/src')
    fixed_count = 0

    for rs_file in src_dir.rglob('*.rs'):
        if fix_use_syntax_errors(rs_file):
            print(f"Fixed: {rs_file}")
            fixed_count += 1

    # Also check tests and tools directories
    for extra_dir in ['tests', 'tools', 'benches']:
        extra_path = Path(f'/Users/henry/code/amberjs/{extra_dir}')
        if extra_path.exists():
            for rs_file in extra_path.rglob('*.rs'):
                if fix_use_syntax_errors(rs_file):
                    print(f"Fixed: {rs_file}")
                    fixed_count += 1

    print(f"\nTotal files fixed: {fixed_count}")

if __name__ == '__main__':
    main()
