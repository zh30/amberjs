#!/usr/bin/env python3
"""
Comprehensive fix for duplicate import errors.
"""

import re
from pathlib import Path

def fix_duplicate_imports(content):
    """Remove duplicate imports and fix conflicts."""
    lines = content.split('\n')

    new_lines = []
    for line in lines:
        stripped = line.strip()

        skip = False

        # List of patterns to check for duplicates
        standalone_to_group = [
            # (standalone import, group pattern to check)
            ('use std::sync::Arc;', r'use std::sync::\{[^}]*Arc'),
            ('use std::sync::Mutex;', r'use std::sync::\{[^}]*Mutex'),
            ('use std::sync::RwLock;', r'use std::sync::\{[^}]*RwLock'),
            ('use std::collections::HashMap;', r'use std::collections::\{[^}]*HashMap'),
            ('use std::collections::BTreeMap;', r'use std::collections::\{[^}]*BTreeMap'),
            ('use std::collections::HashSet;', r'use std::collections::\{[^}]*HashSet'),
            ('use std::time::Duration;', r'use std::time::\{[^}]*Duration'),
            ('use std::time::Instant;', r'use std::time::\{[^}]*Instant'),
            ('use std::io::Read;', r'use std::io::\{[^}]*Read'),
            ('use std::io::Write;', r'use std::io::\{[^}]*Write'),
            ('use std::path::Path;', r'use std::path::\{[^}]*Path'),
            ('use std::path::PathBuf;', r'use std::path::\{[^}]*PathBuf'),
        ]

        # Check standalone imports against group imports
        for standalone, group_pattern in standalone_to_group:
            if stripped == standalone:
                if any(re.search(group_pattern, l) for l in new_lines):
                    skip = True
                    break

        # Check tokio vs std conflicts
        tokio_conflicts = [
            ('use tokio::sync::RwLock;', 'std::sync', 'RwLock'),
            ('use tokio::sync::Mutex;', 'std::sync', 'Mutex'),
            ('use tokio::time::Duration;', 'std::time', 'Duration'),
            ('use tokio::time::Instant;', 'std::time', 'Instant'),
        ]

        for tokio_import, std_module, item in tokio_conflicts:
            if stripped == tokio_import:
                if any(std_module in l and item in l for l in new_lines):
                    skip = True
                    break

        # Skip exact duplicate lines
        if stripped.startswith('use ') and stripped in [l.strip() for l in new_lines]:
            skip = True

        if not skip:
            new_lines.append(line)

    return '\n'.join(new_lines)

def fix_file(filepath):
    """Fix a single file."""
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()

        original = content
        fixed = fix_duplicate_imports(content)

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
