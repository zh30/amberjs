#!/usr/bin/env python3
"""Smarter import fixer that checks actual usage in struct definitions."""
import re
from pathlib import Path

def fix_file(filepath):
    """Fix imports for a single file based on actual usage."""
    with open(filepath, 'r') as f:
        content = f.read()

    changes = []

    # Check which types are used but not properly imported
    type_imports = {
        r'\bArc<': ('Arc', 'std::sync'),
        r'\bMutex<': ('Mutex', 'std::sync'),
        r'\bRwLock<': ('RwLock', 'std::sync'),
        r'\bWeak<': ('Weak', 'std::sync'),
        r'\bHashMap<': ('HashMap', 'std::collections'),
        r'\bHashSet<': ('HashSet', 'std::collections'),
        r'\bBTreeMap<': ('BTreeMap', 'std::collections'),
        r'\bVecDeque<': ('VecDeque', 'std::collections'),
        r'\bDuration\b': ('Duration', 'std::time'),
        r'\bInstant\b': ('Instant', 'std::time'),
        r'\bSystemTime\b': ('SystemTime', 'std::time'),
        r'\bPathBuf\b': ('PathBuf', 'std::path'),
        r'\bPath\b': ('Path', 'std::path'),
        r'\bAtomicUsize\b': ('AtomicUsize', 'std::sync::atomic'),
        r'\bAtomicU64\b': ('AtomicU64', 'std::sync::atomic'),
        r'\bAtomicBool\b': ('AtomicBool', 'std::sync::atomic'),
        r'\bOrdering\b': ('Ordering', 'std::sync::atomic'),
        r'\bDefaultHasher\b': ('DefaultHasher', 'std::hash'),
        r'\bPoll::': ('Poll', 'std::task'),
        r'\bIpAddr\b': ('IpAddr', 'std::net'),
        r'\bSocketAddr\b': ('SocketAddr', 'std::net'),
        r'\bTcpListener\b': ('TcpListener', 'std::net'),
    }

    needed_imports = {}
    for pattern, (type_name, module) in type_imports.items():
        if re.search(pattern, content):
            # Check if already imported
            import_patterns = [
                rf'use\s+{module.replace(".", r"\.")}::\{{\s*[^}}]*\b{type_name}\b[^}}]*\}};',
                rf'use\s+{module.replace(".", r"\.")}::{type_name};',
            ]
            found = False
            for ip in import_patterns:
                if re.search(ip, content):
                    found = True
                    break
            if not found:
                if module not in needed_imports:
                    needed_imports[module] = []
                if type_name not in needed_imports[module]:
                    needed_imports[module].append(type_name)

    if not needed_imports:
        return False

    # Generate import statements
    new_imports = []
    for module, types in sorted(needed_imports.items()):
        if len(types) == 1:
            new_imports.append(f'use {module}::{types[0]};')
        else:
            new_imports.append(f'use {module}::{{{", ".join(sorted(types))}}};')

    # Find where to insert
    lines = content.split('\n')
    insert_pos = 0
    for i, line in enumerate(lines):
        if line.strip().startswith('use '):
            insert_pos = i + 1
        elif insert_pos > 0 and not line.strip().startswith('use ') and line.strip():
            break

    if insert_pos == 0:
        for i, line in enumerate(lines):
            if line.strip() and not line.strip().startswith('//') and not line.strip().startswith('#'):
                insert_pos = i
                break

    # Insert new imports
    for imp in reversed(new_imports):
        lines.insert(insert_pos, imp)
        changes.append(imp)

    with open(filepath, 'w') as f:
        f.write('\n'.join(lines))

    return changes

def main():
    src_dir = Path('/Users/henry/code/amberjs/src')
    total = 0
    for rs_file in src_dir.rglob('*.rs'):
        changes = fix_file(rs_file)
        if changes:
            print(f"{rs_file.relative_to(src_dir)}: {len(changes)} imports")
            total += len(changes)
    print(f"\nTotal imports added: {total}")

if __name__ == '__main__':
    main()
