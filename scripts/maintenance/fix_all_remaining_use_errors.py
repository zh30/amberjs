#!/usr/bin/env python3
"""
Comprehensive fix for ALL malformed use statements in the entire codebase
"""

import os
import re
import sys

def fix_use_statements_comprehensive(content):
    """Comprehensively fix all malformed use statements"""
    original = content

    # Fix all instances of use module::::{item1, item2} -> use module::{item1, item2}
    # This handles nested modules like crate::benchmarks:: etc.
    pattern1 = r'use ([a-zA-Z0-9_:]+)::\{'
    content = re.sub(pattern1, r'use \1::{', content)

    # Fix use module::{::{item1, item2}} pattern
    pattern2 = r'use ([a-zA-Z0-9_:\.]+)::\{::\{([^{}]+)\}\}'
    def replace_double_brace(match):
        module = match.group(1)
        items = match.group(2)
        return f'use {module}::{{{items}}}'
    content = re.sub(pattern2, replace_double_brace, content)

    # Fix use crate::module::::{item1, item2}
    pattern3 = r'use crate::([a-zA-Z0-9_:/]+)::\{::'
    content = re.sub(pattern3, r'use crate::\1::{', content)

    # Fix use std::collections::::<BTreeMap> -> use std::collections::{BTreeMap}
    pattern4 = r'use std::([a-zA-Z0-9_:/]+)::::<([a-zA-Z0-9_,\s]+)>'
    def fix_std_pattern(match):
        module = match.group(1)
        items = match.group(2)
        return f'use std::{module}::<{items}>'
    content = re.sub(pattern4, fix_std_pattern, content)

    # Fix use serde::::<Deserialize, Serialize> -> use serde::{Deserialize, Serialize}
    pattern5 = r'use ([a-zA-Z0-9_:\.]+)::::<([a-zA-Z0-9_,\s]+)>'
    def fix_generic_pattern(match):
        module = match.group(1)
        items = match.group(2)
        return f'use {module}::<{items}>'
    content = re.sub(pattern5, fix_generic_pattern, content)

    # Fix direct module::::{item} patterns
    pattern6 = r'([a-zA-Z0-9_]+)::\{::([a-zA-Z0-9_,\s]+)\}'
    def fix_direct_pattern(match):
        module = match.group(1)
        items = match.group(2)
        return f'{module}::<{items}>'
    content = re.sub(pattern6, fix_direct_pattern, content)

    return content if content != original else None

def fix_file(filepath):
    """Fix a single file"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        new_content = fix_use_statements_comprehensive(content)

        if new_content:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(new_content)
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}", file=sys.stderr)
        return False

def find_all_rust_files(directory):
    """Find all .rs files"""
    files = []
    for root, dirs, files_list in os.walk(directory):
        if 'target' in root:
            continue
        for file in files_list:
            if file.endswith('.rs') and not file.endswith('.bak'):
                files.append(os.path.join(root, file))
    return files

def main():
    src_dir = '/Users/henry/code/amberjs/src'

    print("Finding all Rust files...")
    all_files = find_all_rust_files(src_dir)

    print(f"Processing {len(all_files)} files...")
    fixed_count = 0

    for filepath in all_files:
        if fix_file(filepath):
            fixed_count += 1
            print(f"Fixed: {filepath}")

    print(f"\nTotal files fixed: {fixed_count} out of {len(all_files)}")

if __name__ == '__main__':
    main()
