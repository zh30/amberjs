#!/usr/bin/env python3
"""
Stage 46: Fix remaining V8 API compatibility errors
- Debug trait issues with closures
- prototype_or_null API removal
- Scope borrowing issues
- _rv mutability issues
- Temporary value dropped issues
"""

import re
import os

def fix_websocket_rs():
    """Fix websocket.rs errors"""
    file_path = "/Users/henry/code/amberjs/src/web_api/websocket.rs"

    with open(file_path, 'r') as f:
        content = f.read()

    # Fix 1: Remove Debug derive from WebSocket struct (line 36)
    # The event_handlers field contains closures that don't implement Debug
    content = re.sub(
        r'#\[derive\(Debug, Clone\)\]\s*pub struct WebSocket \{',
        '#[derive(Clone)]\npub struct WebSocket {',
        content
    )

    # Fix 2: Replace prototype_or_null with context.global() approach
    # Old: let proto = websocket_constructor.prototype_or_null(scope);
    # New: Use constructor function's prototype from context
    old_prototype = r'let proto = websocket_constructor\.prototype_or_null\(scope\);\s*if let Some\(proto\) = proto \{'
    new_prototype = '''let global = context.global(scope);
    let proto_key = v8::String::new(scope, "WebSocket").unwrap();
    let proto = global.get(scope, proto_key.into()).and_then(|p| p.to_object(scope));
    if let Some(proto) = proto {'''

    content = re.sub(old_prototype, new_prototype, content, flags=re.DOTALL)

    with open(file_path, 'w') as f:
        f.write(content)

    print(f"✓ Fixed {file_path}")

def fix_events_rs():
    """Fix events.rs errors"""
    file_path = "/Users/henry/code/amberjs/src/web_api/events.rs"

    with open(file_path, 'r') as f:
        content = f.read()

    # Fix: Remove Debug derive from EventTarget struct (line 47)
    # The listeners field contains closures that don't implement Debug
    content = re.sub(
        r'#\[derive\(Debug, Clone\)\]\s*pub struct EventTarget \{',
        '#[derive(Clone)]\npub struct EventTarget {',
        content
    )

    with open(file_path, 'w') as f:
        f.write(content)

    print(f"✓ Fixed {file_path}")

def fix_scope_borrowing():
    """Fix scope borrowing issues across multiple files"""

    # Fix events.rs line 166
    file_path = "/Users/henry/code/amberjs/src/nodejs_core/events.rs"
    with open(file_path, 'r') as f:
        content = f.read()

    # Split the line to avoid double borrowing
    # OLD: this.set(scope, prop_key.into(), v8::Boolean::new(scope, true).into());
    # NEW: let val = v8::Boolean::new(scope, true).into(); this.set(scope, prop_key.into(), val);
    content = re.sub(
        r'this\.set\(scope, prop_key\.into\(\), v8::Boolean::new\(scope, true\)\.into\(\)\);',
        'let val = v8::Boolean::new(scope, true).into();\n    this.set(scope, prop_key.into(), val);',
        content
    )

    with open(file_path, 'w') as f:
        f.write(content)
    print(f"✓ Fixed scope borrowing in {file_path}")

    # Fix buffer.rs line 170
    file_path = "/Users/henry/code/amberjs/src/nodejs_core/buffer.rs"
    with open(file_path, 'r') as f:
        content = f.read()

    # OLD: buffer.set(scope, length_key.into(), v8::Integer::new(scope, bytes.len() as i32).into());
    # NEW: let len_val = v8::Integer::new(scope, bytes.len() as i32).into(); buffer.set(scope, length_key.into(), len_val);
    content = re.sub(
        r'buffer\.set\(scope, length_key\.into\(\), v8::Integer::new\(scope, bytes\.len\(\) as i32\)\.into\(\)\);',
        'let len_val = v8::Integer::new(scope, bytes.len() as i32).into();\n    buffer.set(scope, length_key.into(), len_val);',
        content
    )

    with open(file_path, 'w') as f:
        f.write(content)
    print(f"✓ Fixed scope borrowing in {file_path}")

    # Fix os.rs line 406
    file_path = "/Users/henry/code/amberjs/src/nodejs_core/os.rs"
    with open(file_path, 'r') as f:
        content = f.read()

    # OLD: cpu_obj.set(scope, _key_0.into(), v8::String::new(scope, "...").unwrap().into());
    # NEW: let val = v8::String::new(scope, "...").unwrap().into(); cpu_obj.set(scope, _key_0.into(), val);
    content = re.sub(
        r'cpu_obj\.set\(scope, _key_0\.into\(\), v8::String::new\(scope, "Intel\(R\) Core\(TM\) i7-9700K CPU @ 3\.60GHz"\)\.unwrap\(\)\.into\(\)\);',
        '''let val = v8::String::new(scope, "Intel(R) Core(TM) i7-9700K CPU @ 3.60GHz").unwrap().into();
        cpu_obj.set(scope, _key_0.into(), val);''',
        content
    )

    with open(file_path, 'w') as f:
        f.write(content)
    print(f"✓ Fixed scope borrowing in {file_path}")

    # Fix fetch.rs lines 217 and 227
    file_path = "/Users/henry/code/amberjs/src/web_api/fetch.rs"
    with open(file_path, 'r') as f:
        content = f.read()

    # Line 217
    content = re.sub(
        r'response_obj\.set\(scope, status_key\.into\(\), v8::Integer::new_from_unsigned\(scope, status\)\.into\(\)\);',
        '''let status_val = v8::Integer::new_from_unsigned(scope, status).into();
    response_obj.set(scope, status_key.into(), status_val);''',
        content
    )

    # Line 227
    content = re.sub(
        r'response_obj\.set\(scope, body_key\.into\(\), v8::String::new\(scope, &body_text\)\.unwrap\(\)\.into\(\)\);',
        '''let body_val = v8::String::new(scope, &body_text).unwrap().into();
        response_obj.set(scope, body_key.into(), body_val);''',
        content
    )

    with open(file_path, 'w') as f:
        f.write(content)
    print(f"✓ Fixed scope borrowing in {file_path}")

def fix_rv_mutability():
    """Fix _rv mutability issues"""
    files = [
        ("/Users/henry/code/amberjs/src/nodejs_core/buffer.rs", 91),
        ("/Users/henry/code/amberjs/src/web_api/fetch.rs", 246),
        ("/Users/henry/code/amberjs/src/web_api/url.rs", 302),
    ]

    for file_path, line_num in files:
        with open(file_path, 'r') as f:
            lines = f.readlines()

        # Find the callback function and make _rv mutable
        # Look for patterns like: |_scope, _args, _rv: v8::ReturnValue|
        for i, line in enumerate(lines):
            if '|_rv: v8::ReturnValue|' in line and '_rv: v8::ReturnValue' in line:
                # Change to: |_scope, _args, mut _rv: v8::ReturnValue|
                lines[i] = line.replace('_rv: v8::ReturnValue', 'mut _rv: v8::ReturnValue')
                print(f"✓ Made _rv mutable in {file_path} at line {i+1}")
                break

        with open(file_path, 'w') as f:
            f.writelines(lines)

if __name__ == "__main__":
    print("Stage 46: Fixing remaining V8 API compatibility errors...\n")

    fix_websocket_rs()
    fix_events_rs()
    fix_scope_borrowing()
    fix_rv_mutability()

    print("\n✅ All automated fixes applied!")
    print("\nNext: Manual fixes needed for:")
    print("  - path.rs temporary value dropped issues (3 errors)")
    print("  - url.rs borrow of moved value issue (1 error)")
    print("  - plugin/system.rs Box<dyn Plugin> trait bound (1 error)")
