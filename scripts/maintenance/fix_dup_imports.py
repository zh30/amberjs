#!/usr/bin/env python3
"""Fix duplicate imports v3."""
import re
from pathlib import Path

def fix_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    orig = content

    # Fix Hash/Hasher duplicates
    if 'use std::hash::{Hash, Hasher};' in content and 'use std::hash::{Hash, Hasher, DefaultHasher};' in content:
        content = content.replace('use std::hash::{Hash, Hasher};\n', '')

    # Fix tokio Mutex/RwLock conflicts
    if ('use std::sync::{Mutex' in content or 'use std::sync::Mutex' in content) and 'use tokio::sync::{Mutex, RwLock};' in content:
        content = content.replace('use tokio::sync::{Mutex, RwLock};', 'use tokio::sync::{Mutex as AsyncMutex, RwLock as AsyncRwLock};')

    # Fix TcpListener duplicate
    if 'use tokio::net::{TcpListener' in content and 'use std::net::TcpListener;' in content:
        content = content.replace('use std::net::TcpListener;', 'use std::net::TcpListener as StdTcpListener;')

    # Fix grouped TcpListener
    if 'use tokio::net::{TcpListener' in content:
        content = re.sub(r'use std::net::\{([^}]*?)TcpListener([^}]*?)\};',
                        lambda m: f'use std::net::{{{m.group(1)}TcpListener as StdTcpListener{m.group(2)}}};', content)

    if content != orig:
        with open(filepath, 'w') as f:
            f.write(content)
        return True
    return False

src = Path('/Users/henry/code/amberjs/src')
fixed = sum(1 for f in src.rglob('*.rs') if fix_file(f))
print(f"Fixed {fixed} files")
