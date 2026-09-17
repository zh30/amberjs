---
title: "Native C ABI Foreign Function Interface (bee:ffi)"
subtitle: "Zero-dependency, high-speed C symbol dispatch and raw memory manipulation for systems interoperability"
group: "Agent & Advanced"
id: "ffi-native"
---

## 1. Overview & Motivation

Modern systems programming often requires invoking native shared libraries (`.dylib`, `.so`, `.dll`), system C ABIs, hardware drivers, or specialized native accelerators directly from JavaScript and TypeScript.

Historically, Node.js developers had to rely on heavy third-party native addons like `node-ffi-napi`, requiring `node-gyp`, Python, and local C++ compiler toolchains.

**Beejs v1.4.0 introduces `bee:ffi`**, a zero-dependency, ultra-fast C ABI Foreign Function Interface built directly into the Beejs runtime binary via OS-native dynamic linkers (`dlopen` on Unix/macOS, `LoadLibraryA` on Windows).

### Key Features
- **Zero External Dependencies**: Zero npm packages or compiler toolchains required at runtime.
- **Fast-Path ABI Dispatch**: Directly mapped scalar calls (0 to 3 arguments) execute with near-native invocation overhead.
- **Raw Memory Operations**: High-performance pointer inspection (`ptr`), primitive read/write (`read`, `write`), and null-terminated C string utilities (`readCString`).
- **Complete Type Safety**: TypeScript declarations included in `bee:ffi` and `@types/beejs`.

---

## 2. Quick Start: Invoking Native Math Functions

Import `dlopen` from `bee:ffi`:

```typescript
import { dlopen, FFIType } from 'bee:ffi';

// Load system dynamic library (or null to resolve globally loaded symbols)
const libPath = process.platform === 'darwin'
  ? '/usr/lib/libSystem.B.dylib'
  : (process.platform === 'win32' ? 'msvcrt.dll' : 'libc.so.6');

const lib = dlopen(libPath, {
  symbols: {
    cos: { args: ['f64'], returns: 'f64' },
    sin: { args: ['f64'], returns: 'f64' },
    abs: { args: ['i32'], returns: 'i32' },
  }
});

console.log('cos(0):', lib.symbols.cos(0.0)); // 1.0
console.log('sin(0):', lib.symbols.sin(0.0)); // 0.0
console.log('abs(-42):', lib.symbols.abs(-42)); // 42

// Always close the library handle when finished
lib.close();
```

---

## 3. Supported FFI Types

The `FFIType` enum defines scalar and pointer representations:

| Type | Description | JavaScript Equivalent |
| :--- | :--- | :--- |
| `'void'` | No return value | `undefined` |
| `'bool'` | 8-bit boolean | `boolean` |
| `'u8'`, `'i8'` | 8-bit unsigned / signed integer | `number` |
| `'u16'`, `'i16'` | 16-bit unsigned / signed integer | `number` |
| `'u32'`, `'i32'` | 32-bit unsigned / signed integer | `number` |
| `'u64'`, `'i64'` | 64-bit integer | `bigint` or `number` |
| `'f32'` | 32-bit single-precision float | `number` |
| `'f64'` | 64-bit double-precision float | `number` |
| `'ptr'`, `'pointer'` | Raw memory address pointer | `bigint` |
| `'cstring'`, `'string'` | Null-terminated C string | `string` |

---

## 4. Direct Memory Pointer Operations

`bee:ffi` provides low-level pointer inspection and mutation without heap allocation:

```typescript
import { ptr, read, write, readCString } from 'bee:ffi';

// Allocate 64 bytes in JavaScript
const buffer = new Uint8Array(64);

// 1. Get raw memory pointer as BigInt
const address = ptr(buffer);
console.log(`Memory allocated at address: 0x${address.toString(16)}`);

// 2. Write primitive values directly to memory
write(address, 0, 'i32', 42);
write(address, 4, 'f64', 3.1415926);

// 3. Read back primitive values from memory offsets
console.log('Offset 0 (i32):', read(address, 0, 'i32')); // 42
console.log('Offset 4 (f64):', read(address, 4, 'f64')); // 3.1415926

// 4. Read C strings directly from native memory
buffer[16] = 72;  // 'H'
buffer[17] = 105; // 'i'
buffer[18] = 0;   // '\0'

const str = readCString(address + 16n);
console.log('Read C string:', str); // "Hi"
```
