#!/usr/bin/env python3
"""Fix missing standard library imports systematically."""
import os
import re
from pathlib import Path

# Type to import mapping
IMPORTS = {
    # std::sync
    'Arc': 'use std::sync::Arc;',
    'Mutex': 'use std::sync::Mutex;',
    'RwLock': 'use std::sync::RwLock;',
    'Weak': 'use std::sync::Weak;',
    'Condvar': 'use std::sync::Condvar;',
    'MutexGuard': 'use std::sync::MutexGuard;',

    # std::sync::atomic
    'AtomicU64': 'use std::sync::atomic::AtomicU64;',
    'AtomicUsize': 'use std::sync::atomic::AtomicUsize;',
    'AtomicBool': 'use std::sync::atomic::AtomicBool;',
    'AtomicU32': 'use std::sync::atomic::AtomicU32;',
    'Ordering': 'use std::sync::atomic::Ordering;',

    # std::collections
    'HashMap': 'use std::collections::HashMap;',
    'HashSet': 'use std::collections::HashSet;',
    'BTreeMap': 'use std::collections::BTreeMap;',
    'BTreeSet': 'use std::collections::BTreeSet;',
    'VecDeque': 'use std::collections::VecDeque;',
    'BinaryHeap': 'use std::collections::BinaryHeap;',

    # std::time
    'Duration': 'use std::time::Duration;',
    'Instant': 'use std::time::Instant;',
    'SystemTime': 'use std::time::SystemTime;',

    # std::path
    'Path': 'use std::path::Path;',
    'PathBuf': 'use std::path::PathBuf;',

    # std::io
    'BufReader': 'use std::io::BufReader;',
    'BufWriter': 'use std::io::BufWriter;',
    'Write': 'use std::io::Write;',
    'Read': 'use std::io::Read;',

    # std::hash
    'DefaultHasher': 'use std::hash::{Hash, Hasher, DefaultHasher};',
    'Hasher': 'use std::hash::Hasher;',
    'Hash': 'use std::hash::Hash;',

    # std::task
    'Poll': 'use std::task::Poll;',
    'Context': 'use std::task::Context;',

    # std::num
    'NonZero': 'use std::num::NonZeroUsize;',
    'NonZeroUsize': 'use std::num::NonZeroUsize;',

    # std::cell
    'RefCell': 'use std::cell::RefCell;',
    'Cell': 'use std::cell::Cell;',

    # std::marker
    'PhantomData': 'use std::marker::PhantomData;',

    # std::pin
    'Pin': 'use std::pin::Pin;',
}

def check_missing_imports(content, filename):
    """Check which types are used but not imported."""
    missing = []
    for type_name, import_stmt in IMPORTS.items():
        # Check if type is used
        pattern = rf'\b{type_name}\b'
        if re.search(pattern, content):
            # Check if already imported
            import_base = import_stmt.replace('use ', '').replace(';', '')
            # Check various import patterns
            if type_name not in content.split('use ')[0]:  # Before first use statement
                if not re.search(rf'use\s+[^;]*\b{type_name}\b', content):
                    missing.append((type_name, import_stmt))
    return missing

def add_imports(content, imports):
    """Add missing imports to the file."""
    if not imports:
        return content

    # Find position to insert imports (after existing use statements or at start)
    lines = content.split('\n')
    insert_pos = 0

    for i, line in enumerate(lines):
        if line.strip().startswith('use '):
            insert_pos = i + 1
        elif line.strip().startswith('mod ') or line.strip().startswith('pub mod'):
            if insert_pos == 0:
                insert_pos = i
            break
        elif line.strip().startswith('fn ') or line.strip().startswith('pub fn'):
            if insert_pos == 0:
                insert_pos = i
            break
        elif line.strip().startswith('struct ') or line.strip().startswith('pub struct'):
            if insert_pos == 0:
                insert_pos = i
            break

    # Insert imports
    import_lines = [stmt for _, stmt in imports]
    new_lines = lines[:insert_pos] + import_lines + lines[insert_pos:]
    return '\n'.join(new_lines)

def process_file(filepath):
    """Process a single Rust file."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()

        missing = check_missing_imports(content, filepath)
        if missing:
            new_content = add_imports(content, missing)
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(new_content)
            return len(missing)
        return 0
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return 0

def main():
    src_dir = Path('/Users/henry/code/amberjs/src')
    total_fixed = 0
    files_modified = 0

    for rs_file in src_dir.rglob('*.rs'):
        fixed = process_file(rs_file)
        if fixed > 0:
            total_fixed += fixed
            files_modified += 1
            print(f"Fixed {fixed} imports in {rs_file}")

    print(f"\n=== Summary ===")
    print(f"Files modified: {files_modified}")
    print(f"Total imports added: {total_fixed}")

if __name__ == '__main__':
    main()
