#!/usr/bin/env python3
"""
Fix import name conflicts by renaming duplicates with 'as' keyword.
Example: Both std::sync::RwLock and tokio::sync::RwLock -> rename tokio to TokioRwLock
"""

import os
import re
import sys

def fix_name_conflicts_in_file(filepath):
    """Fix name conflicts in a single file"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        original_lines = lines.copy()
        modified = False

        # Track which imports we've seen
        seen_imports = {}
        new_lines = []

        for line in lines:
            # Check if this is an import line
            if line.strip().startswith('use ') and line.strip().endswith(';'):
                import_line = line.strip()

                # Handle RwLock conflicts
                if 'tokio::sync::RwLock' in import_line and 'std::sync::{Arc, Mutex, RwLock}' in ''.join(new_lines[-5:]):
                    # Replace tokio::sync::RwLock with tokio::sync::RwLock as TokioRwLock
                    line = import_line.replace('tokio::sync::RwLock', 'tokio::sync::RwLock as TokioRwLock') + '\n'
                    modified = True
                    print(f"  Fixed RwLock conflict in {filepath}")

                # Handle Duration conflicts
                elif 'tokio::time::Duration' in import_line and 'std::time::{Duration, Instant}' in ''.join(new_lines[-5:]):
                    line = import_line.replace('tokio::time::Duration', 'tokio::time::Duration as TokioDuration') + '\n'
                    modified = True
                    print(f"  Fixed Duration conflict in {filepath}")

                # Handle Instant conflicts
                elif 'tokio::time::Instant' in import_line and 'std::time::{Duration, Instant}' in ''.join(new_lines[-5:]):
                    line = import_line.replace('tokio::time::Instant', 'tokio::time::Instant as TokioInstant') + '\n'
                    modified = True
                    print(f"  Fixed Instant conflict in {filepath}")

                # Handle Mutex conflicts (less common, but possible)
                elif 'tokio::sync::Mutex' in import_line and 'std::sync::{Arc, Mutex}' in ''.join(new_lines[-5:]):
                    line = import_line.replace('tokio::sync::Mutex', 'tokio::sync::Mutex as TokioMutex') + '\n'
                    modified = True
                    print(f"  Fixed Mutex conflict in {filepath}")

                # Handle HashMap conflicts
                elif 'tokio::sync::RwLock<HashMap' in import_line:
                    # This is a complex case, might need manual review
                    pass

            new_lines.append(line)

        # Write back if modified
        if modified:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.writelines(new_lines)
            return True

        return False

    except Exception as e:
        print(f"Error processing {filepath}: {e}", file=sys.stderr)
        return False

def find_rust_files_with_conflicts(directory):
    """Find files that likely have name conflicts"""
    files_with_conflicts = []

    for root, dirs, files in os.walk(directory):
        if 'target' in root:
            continue
        for file in files:
            if file.endswith('.rs') and not file.endswith('.bak'):
                filepath = os.path.join(root, file)
                try:
                    with open(filepath, 'r', encoding='utf-8') as f:
                        content = f.read()

                    # Check for both std and tokio versions
                    has_std_sync = 'std::sync::RwLock' in content or 'std::sync::Mutex' in content
                    has_tokio_sync = 'tokio::sync::RwLock' in content or 'tokio::sync::Mutex' in content
                    has_std_time = 'std::time::Duration' in content or 'std::time::Instant' in content
                    has_tokio_time = 'tokio::time::Duration' in content or 'tokio::time::Instant' in content

                    if (has_std_sync and has_tokio_sync) or (has_std_time and has_tokio_time):
                        files_with_conflicts.append(filepath)

                except Exception as e:
                    print(f"Error reading {filepath}: {e}", file=sys.stderr)

    return files_with_conflicts

def main():
    src_dir = '/Users/henry/code/amberjs/src'

    print("Finding files with import name conflicts...")
    files_with_conflicts = find_rust_files_with_conflicts(src_dir)

    print(f"Found {len(files_with_conflicts)} files with potential conflicts")

    fixed_files = 0
    for filepath in files_with_conflicts:
        print(f"Processing: {filepath}")
        if fix_name_conflicts_in_file(filepath):
            fixed_files += 1

    print(f"\nSummary:")
    print(f"Files with conflicts: {len(files_with_conflicts)}")
    print(f"Files fixed: {fixed_files}")

if __name__ == '__main__':
    main()
