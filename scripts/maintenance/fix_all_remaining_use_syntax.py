#!/usr/bin/env python3
"""
Fix all remaining use statement syntax errors (::< to ::{)
"""
import re
import os
from pathlib import Path

def fix_use_statements(content):
    """Fix use statement syntax errors"""
    # Pattern 1: use path::<Item>;
    # Change to: use path::{Item};
    content = re.sub(
        r'use\s+([a-zA-Z0-9_:]+)<([^>]+)>;',
        r'use \1::{\2};',
        content
    )

    # Pattern 2: use path::<Item1, Item2, Item3>;
    # Change to: use path::{Item1, Item2, Item3};
    content = re.sub(
        r'use\s+([a-zA-Z0-9_:]+)<([^>]+)>;',
        r'use \1::{\2};',
        content
    )

    # Pattern 3: Multi-line use with trailing comma and semicolon on wrong line
    # Handle cases like:
    # use crate::module::{
    #     Item1,
    #     Item2,
    # };
    # This is already correct, so we don't need to change it

    # Pattern 4: Fix use path::< \n Item, \n>;
    # Change to: use path::{ \n Item, \n};
    content = re.sub(
        r'use\s+([a-zA-Z0-9_:]+)<\s*\n(.*?)\n\s*>;',
        r'use \1::{\2};',
        content,
        flags=re.DOTALL
    )

    # Pattern 5: Fix consecutive use statements on same line
    # use path::<Item>;use path2::<Item2>;
    # Change to:
    # use path::{Item};
    # use path2::{Item2};
    content = re.sub(
        r'(use\s+[a-zA-Z0-9_:]+<[^>]+>;)use\s+',
        r'\1\nuse ',
        content
    )

    return content

def process_file(filepath):
    """Process a single Rust file"""
    print(f"Processing: {filepath}")

    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content
    fixed_content = fix_use_statements(content)

    if fixed_content != original_content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(fixed_content)
        print(f"  ✓ Fixed use statements")
        return True
    else:
        print(f"  - No changes needed")
        return False

def main():
    """Main function"""
    src_dir = Path('/Users/henry/code/amberjs/src')
    rust_files = list(src_dir.rglob('*.rs'))

    fixed_count = 0
    for filepath in rust_files:
        if process_file(filepath):
            fixed_count += 1

    print(f"\nFixed {fixed_count} files")

if __name__ == '__main__':
    main()
