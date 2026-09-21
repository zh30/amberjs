#!/usr/bin/env python3
"""
Fix all turbofish syntax errors.
Pattern: ::{Type} -> ::<Type>
But NOT in use statements: use module::{Item} is correct

This script needs to be careful to only fix turbofish (method generic) syntax,
not use statement syntax.
"""

import re
import os
from pathlib import Path

def is_turbofish_context(line, pos):
    """Check if the ::{ at position is in turbofish context (not use statement)."""
    # If line starts with 'use ' or 'pub use ', it's a use statement
    stripped = line.strip()
    if stripped.startswith('use ') or stripped.startswith('pub use '):
        return False
    return True

def fix_turbofish(content):
    """Fix turbofish syntax errors."""
    lines = content.split('\n')
    fixed_lines = []

    for line in lines:
        # Skip use statements
        stripped = line.strip()
        if stripped.startswith('use ') or stripped.startswith('pub use '):
            fixed_lines.append(line)
            continue

        # In non-use contexts, fix ::{Type> to ::<Type>
        # Pattern matches: ::{SomeType> or ::{some_type>
        # But we need to be careful: use module::{Item} is valid

        # Fix patterns like .method::{Type>() or Type::{T>
        # These are turbofish and should use ::<Type>

        # Match pattern: something::{Type}> where } is wrong
        # Actually the pattern is ::{Type> which should be ::<Type>

        # Fix: somefunc::{Type>(...) -> somefunc::<Type>(...)
        # But avoid: use mod::{Item} which is correct

        new_line = line

        # Simple fix: in non-use context, ::{X> should be ::<X>
        # But we need to be careful not to break use statements

        # Fix turbofish with common patterns
        patterns = [
            (r'(\.\w+)::\{(\w+)>\(', r'\1::<\2>('),  # .method::{Type>( -> .method::<Type>(
            (r'(\w+)::\{(\w+)>\(\)', r'\1::<\2>()'),  # func::{Type>() -> func::<Type>()
            (r'(\w+)::\{(\w+)>\s*\)', r'\1::<\2>)'),  # func::{Type>) -> func::<Type>)
            (r'v8::Local::\{v8::(\w+)>', r'v8::Local::<v8::\1>'),  # v8::Local
            (r'Api::\{(\w+)>', r'Api::<\1>'),  # Api::{Type>
            (r'serde_json::from_str::\{([^}]+)>', r'serde_json::from_str::<\1>'),  # serde_json
            (r'unbounded_channel::\{([^}]+)>', r'unbounded_channel::<\1>'),
            (r'unbounded::\{([^}]+)>', r'unbounded::<\1>'),
            (r'downcast_ref::\{(\w+)>', r'downcast_ref::<\1>'),
            (r'parse::\{(\w+)>', r'parse::<\1>'),
            (r'gen::\{(\w+)>', r'gen::<\1>'),
            (r'mpsc::unbounded_channel::\{([^}]+)>', r'mpsc::unbounded_channel::<\1>'),
        ]

        for pattern, replacement in patterns:
            new_line = re.sub(pattern, replacement, new_line)

        fixed_lines.append(new_line)

    return '\n'.join(fixed_lines)

def fix_file(filepath):
    """Fix a single file."""
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()

        original = content
        fixed = fix_turbofish(content)

        if fixed != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(fixed)
            return True
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
    return False

def main():
    base = Path('/Users/henry/code/amberjs')
    fixed = 0

    for rs_file in base.rglob('*.rs'):
        if fix_file(rs_file):
            print(f"Fixed: {rs_file}")
            fixed += 1

    print(f"\nTotal files fixed: {fixed}")

if __name__ == '__main__':
    main()
