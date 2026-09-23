#!/usr/bin/env python3
"""Final comprehensive import fixer."""
import re
import os
from pathlib import Path

def needs_import(content, type_name, module_pattern):
    """Check if type is used but not imported."""
    # Check if type is used (with type context)
    if type_name == 'Duration' or type_name == 'Instant':
        pattern = rf'\b{type_name}\b'
    elif type_name in ['Arc', 'Mutex', 'RwLock', 'HashMap', 'HashSet', 'VecDeque', 'BTreeMap']:
        pattern = rf'\b{type_name}<'
    else:
        pattern = rf'\b{type_name}\b'

    if not re.search(pattern, content):
        return False

    # Check if already imported
    if re.search(module_pattern, content):
        return False

    return True

def add_import_after_header(content, import_stmt):
    """Add import statement after module doc comments."""
    lines = content.split('\n')

    # Find insertion point
    insert_pos = 0
    for i, line in enumerate(lines):
        if line.strip().startswith('//!'):
            insert_pos = i + 1
        elif line.strip().startswith('use '):
            insert_pos = i + 1
        elif line.strip() and not line.strip().startswith('//') and not line.strip().startswith('#'):
            if insert_pos == 0:
                insert_pos = i
            break

    lines.insert(insert_pos, import_stmt)
    return '\n'.join(lines)

def fix_file(filepath):
    """Fix a single file."""
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content
    imports_to_add = []

    # Check each type
    checks = [
        ('Duration', r'use std::time::\{[^}]*Duration', 'use std::time::Duration;'),
        ('Instant', r'use std::time::\{[^}]*Instant', 'use std::time::Instant;'),
        ('Ordering', r'use std::sync::atomic::\{[^}]*Ordering', 'use std::sync::atomic::Ordering;'),
        ('HashMap', r'use std::collections::\{[^}]*HashMap', 'use std::collections::HashMap;'),
        ('Arc', r'use std::sync::\{[^}]*Arc', 'use std::sync::Arc;'),
        ('Mutex', r'use std::sync::\{[^}]*Mutex', 'use std::sync::Mutex;'),
        ('RwLock', r'use std::sync::\{[^}]*RwLock', 'use std::sync::RwLock;'),
        ('AtomicUsize', r'use std::sync::atomic::\{[^}]*AtomicUsize', 'use std::sync::atomic::AtomicUsize;'),
        ('AtomicBool', r'use std::sync::atomic::\{[^}]*AtomicBool', 'use std::sync::atomic::AtomicBool;'),
        ('AtomicU64', r'use std::sync::atomic::\{[^}]*AtomicU64', 'use std::sync::atomic::AtomicU64;'),
        ('VecDeque', r'use std::collections::\{[^}]*VecDeque', 'use std::collections::VecDeque;'),
        ('BTreeMap', r'use std::collections::\{[^}]*BTreeMap', 'use std::collections::BTreeMap;'),
        ('HashSet', r'use std::collections::\{[^}]*HashSet', 'use std::collections::HashSet;'),
        ('SystemTime', r'use std::time::\{[^}]*SystemTime', 'use std::time::SystemTime;'),
    ]

    for type_name, pattern, import_stmt in checks:
        if needs_import(content, type_name, pattern):
            # Also check exact import
            exact = import_stmt.replace(';', '')
            if exact not in content:
                imports_to_add.append(import_stmt)
                content = add_import_after_header(content, import_stmt)

    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        return len(imports_to_add)
    return 0

def main():
    src_dir = Path('/Users/henry/code/amberjs/src')
    total = 0
    files = 0
    for rs_file in src_dir.rglob('*.rs'):
        n = fix_file(rs_file)
        if n > 0:
            print(f"{rs_file.relative_to(src_dir)}: +{n} imports")
            total += n
            files += 1

    print(f"\n=== Summary ===")
    print(f"Files modified: {files}")
    print(f"Imports added: {total}")

if __name__ == '__main__':
    main()
