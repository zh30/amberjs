#!/usr/bin/env python3
"""
Fix malformed use statements with extra :: before {
Example: use crate::module::::{Item1, Item2} -> use crate::module::{Item1, Item2}
"""

import os
import re
import sys

def fix_use_statements_in_file(filepath):
    """Fix malformed use statements in a single file"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        original_content = content

        # Pattern to match malformed use statements with :: before {
        # Matches: use module::::{item1, item2}
        # Replaces with: use module::{item1, item2}
        pattern = r'use ([a-zA-Z0-9_:]+)::\{'
        replacement = r'use \1{'

        content = re.sub(pattern, replacement, content)

        # Also fix cases like: use super::::{Item}
        pattern2 = r'use super::\{::'
        replacement2 = r'use super::{'
        content = re.sub(pattern2, replacement2, content)

        # Write back if changed
        if content != original_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            return True, 1
        return False, 0

    except Exception as e:
        print(f"Error processing {filepath}: {e}", file=sys.stderr)
        return False, 0

def find_rust_files(directory):
    """Find all .rs files in directory"""
    rust_files = []
    for root, dirs, files in os.walk(directory):
        # Skip target directory
        if 'target' in root:
            continue
        for file in files:
            if file.endswith('.rs') and not file.endswith('.bak'):
                rust_files.append(os.path.join(root, file))
    return rust_files

def main():
    src_dir = '/Users/henry/code/amberjs/src'
    test_dir = '/Users/henry/code/amberjs/tests'

    all_files = find_rust_files(src_dir) + find_rust_files(test_dir)

    fixed_files = 0
    total_fixes = 0

    print("Fixing malformed use statements...")
    for filepath in all_files:
        fixed, num_fixes = fix_use_statements_in_file(filepath)
        if fixed:
            fixed_files += 1
            total_fixes += num_fixes
            print(f"Fixed: {filepath}")

    print(f"\nSummary:")
    print(f"Files fixed: {fixed_files}")
    print(f"Total fixes: {total_fixes}")

if __name__ == '__main__':
    main()
