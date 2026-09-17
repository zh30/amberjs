---
title: "原生 C ABI 外部函数接口 (bee:ffi)"
subtitle: "零依赖、高吞吐的 C 符号调用与裸内存操作接口，专为系统互操作与硬件加速而生"
group: "Agent & Advanced"
id: "ffi-native"
---

## 1. 背景与核心价值

现代系统级开发经常需要直接从 JavaScript/TypeScript 调用系统动态链接库（`.dylib`、`.so`、`.dll`）、操作系统 C ABI 或硬件驱动加速器。

在传统 Node.js 生态中，开发者必须依赖繁重的外部插件（如 `node-ffi-napi`），并且往往受限于 `node-gyp`、Python 环境与本地 C++ 编译器的构建痛点。

**Beejs v1.4.0 原生内置了 `bee:ffi` 模块**，基于操作系统原生动态链接器（Unix/macOS 下的 `dlopen` 与 Windows 下的 `LoadLibraryA`），提供无需任何外部 npm 依赖或编译环境的超轻量 C 语言外部函数接口。

### 核心亮点
- **零外部依赖**：单二进制直接支持，无需安装 node-gyp、Python 或 C++ 编译器。
- **快速路径 C ABI 调用**：针对 0 到 3 个标量参数的常见调用实现底层直接调度，开销极低。
- **裸内存与指针操作**：支持获取 TypedArray 内存物理地址（`ptr`）、内存读写（`read` / `write`）与 C 字符串解析（`readCString`）。
- **完善的类型支持**：完整的 TypeScript 类型定义内置于 `bee:ffi` 与 `@types/beejs`。

---

## 2. 快速上手：调用系统数学库

通过 `bee:ffi` 导入 `dlopen`：

```typescript
import { dlopen, FFIType } from 'bee:ffi';

// 加载系统标准 C 库 (传入 null 可直接解析主进程全局导出的 C 符号)
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

// 调用完成后释放动态库句柄
lib.close();
```

---

## 3. 支持的 FFI 数据类型

`FFIType` 提供了全面的 C 标量与指针映射：

| 类型标识 | 描述 | JavaScript 对应类型 |
| :--- | :--- | :--- |
| `'void'` | 无返回值 | `undefined` |
| `'bool'` | 8 位布尔值 | `boolean` |
| `'u8'`, `'i8'` | 8 位无符号 / 有符号整数 | `number` |
| `'u16'`, `'i16'` | 16 位无符号 / 有符号整数 | `number` |
| `'u32'`, `'i32'` | 32 位无符号 / 有符号整数 | `number` |
| `'u64'`, `'i64'` | 64 位整数 | `bigint` 或 `number` |
| `'f32'` | 32 位单精度浮点数 | `number` |
| `'f64'` | 64 位双精度浮点数 | `number` |
| `'ptr'`, `'pointer'` | 内存物理地址指针 | `bigint` |
| `'cstring'`, `'string'` | 以 null 结尾的 C 语言字符串 | `string` |

---

## 4. 裸内存读写与指针操作

`bee:ffi` 允许直接检查与修改内存地址，无需中间堆拷贝：

```typescript
import { ptr, read, write, readCString } from 'bee:ffi';

// 在 JS 堆中申请 64 字节缓冲区
const buffer = new Uint8Array(64);

// 1. 获取底层物理内存地址 (BigInt)
const address = ptr(buffer);
console.log(`内存地址: 0x${address.toString(16)}`);

// 2. 直接向内存写入基元类型数值
write(address, 0, 'i32', 42);
write(address, 4, 'f64', 3.1415926);

// 3. 根据偏移量从内存中读取数值
console.log('偏移 0 (i32):', read(address, 0, 'i32')); // 42
console.log('偏移 4 (f64):', read(address, 4, 'f64')); // 3.1415926

// 4. 从内存地址读取以 null 结尾的 C 字符串
buffer[16] = 72;  // 'H'
buffer[17] = 105; // 'i'
buffer[18] = 0;   // '\0'

const str = readCString(address + 16n);
console.log('解析 C 字符串:', str); // "Hi"
```
