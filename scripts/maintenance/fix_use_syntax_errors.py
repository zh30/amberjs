#!/usr/bin/env python3
"""
Fix malformed use statements with wrong syntax:
- Missing :: before {
- Curly braces instead of angle brackets for generics
- Other syntax errors
"""

import os
import re
import sys

def fix_use_syntax_in_file(filepath):
    """Fix malformed use statements in a single file"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content
        fixes = 0

        # Fix missing :: before {
        # Pattern: use module{item1, item2}
        # Replace with: use module::{item1, item2}
        pattern1 = r'use ([a-zA-Z0-9_:]+)\{'
        replacement1 = r'use \1::{'
        new_content = re.sub(pattern1, replacement1, content)
        if new_content != content:
            fixes += 1
            content = new_content

        # Fix curly braces in use statements that should be angle brackets
        # Pattern: use module{GenericType}
        # Replace with: use module<GenericType>
        pattern2 = r'use ([a-zA-Z0-9_:]+)\{([a-zA-Z0-9_:,<>\s]+)\}'
        def replace_braces(match):
            module = match.group(1)
            types = match.group(2)
            return f'use {module}::<{types}>'
        new_content = re.sub(pattern2, replace_braces, content)
        if new_content != content:
            fixes += 2
            content = new_content

        # Fix missing braces around multiple items
        # Pattern: use module::item1, item2;
        # Replace with: use module::{item1, item2};
        pattern3 = r'use ([a-zA-Z0-9_:\.]+);'
        def add_braces(match):
            import_path = match.group(1)
            # Only add braces if it doesn't already have them and has commas
            if '{' not in import_path and ',' in import_path:
                return f'use {{{import_path}}};'
            elif '{' not in import_path and '::' in import_path and not import_path.endswith('}'):
                return f'use {import_path};'
            return match.group(0)
        new_content = re.sub(pattern3, add_braces, content)
        if new_content != content:
            fixes += 3
            content = new_content

        # Write back if fixed
        if content != original_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            return True, fixes
        return False, 0

    except Exception as e:
        print(f"Error processing {filepath}: {e}", file=sys.stderr)
        return False, 0

def find_rust_files(directory):
    """Find all .rs files in directory"""
    rust_files = []
    for root, dirs, files in os.walk(directory):
        if 'target' in root:
            continue
        for file in files:
            if file.endswith('.rs') and not file.endswith('.bak'):
                rust_files.append(os.path.join(root, file))
    return rust_files

def main():
    src_dir = '/Users/henry/code/amberjs/src'

    all_files = find_rust_files(src_dir)

    fixed_files = 0
    total_fixes = 0

    print("Fixing use statement syntax errors...")
    for filepath in all_files:
        fixed, num_fixes = fix_use_syntax_in_file(filepath)
        if fixed:
            fixed_files += 1
            total_fixes += num_fixes
            print(f"Fixed: {filepath} ({num_fixes} fixes)")

    print(f"\nSummary:")
    print(f"Files fixed: {fixed_files}")
    print(f"Total fixes: {total_fixes}")

if __name__ == '__main__':
    main()
