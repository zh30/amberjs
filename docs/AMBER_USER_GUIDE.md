# Amber 用户指南

> 发布校验说明（2026-05-26）：本文命令以当前 public CLI 为准：`amber run <file>`、`bee eval <code>`、`bee repl`、`bee test <path>`、`bee debug <file>`、`bee serve`。

欢迎使用 Amber - 高性能 JavaScript/TypeScript 运行时！本指南将帮助您快速上手并充分利用 Amber 的强大功能。

## 📚 目录

- [快速开始](#快速开始)
- [安装指南](#安装指南)
- [基础使用](#基础使用)
- [高级功能](#高级功能)
- [调试与性能分析](#调试与性能分析)
- [最佳实践](#最佳实践)
- [常见问题](#常见问题)

## 🚀 快速开始

### Hello World

创建 `hello.js` 文件：

```javascript
console.log("Hello from Amber!");
console.log("高性能 JavaScript/TypeScript 运行时");
```

运行：

```bash
amber run hello.js
```

### TypeScript 支持

Amber 原生支持 TypeScript，无需额外编译步骤：

```typescript
// hello.ts
interface User {
    name: string;
    age: number;
}

const user: User = {
    name: "Amber",
    age: 1
};

console.log(`Hello, ${user.name}!`);
```

直接运行：

```bash
amber run hello.ts
```

## 📦 安装指南

### 系统要求

- Rust 1.70+
- CMake 3.0+
- Python 3.8+ (用于构建 V8)

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/zh30/beejs.git
cd beejs

# 构建发布版本
cargo build --release

# 运行测试
cargo test

# 安装到系统
cargo install --path .
```

### 一键安装 (推荐)

```bash
curl -fsSL https://raw.githubusercontent.com/zh30/beejs/main/install.sh | sh
amber --version
```

### 预编译二进制 (手动)

```bash
# 选择版本和平台
VERSION=v0.1.1
TARGET=x86_64-unknown-linux-gnu

# 下载指定版本
curl -L https://github.com/zh30/beejs/releases/download/${VERSION}/bee-${VERSION}-${TARGET}.tar.gz -o bee.tar.gz

# 解压
tar -xzf bee.tar.gz

# 安装
mkdir -p ~/.beejs/bin
mv bee ~/.beejs/bin/
chmod +x ~/.beejs/bin/bee
export PATH=\"$HOME/.beejs/bin:$PATH\"
```

## 🔧 基础使用

### 命令行选项

```bash
# 查看帮助
bee --help

# 查看版本
amber --version

# 运行脚本
amber run script.js

# 交互式 REPL
bee repl

# 启用详细输出
bee --verbose run script.js

# 当前 public CLI 不支持 --max-heap-size
```

### 配置文件

创建 `beejs.config.json`：

```json
{
    "heap": {
        "initial": "256MB",
        "max": "1GB"
    },
    "optimization": {
        "jit": true,
        "inline_cache": true,
        "fast_paths": true
    },
    "monitoring": {
        "performance": true,
        "memory": true
    }
}
```

### 模块系统

#### CommonJS

```javascript
// math.js
exports.add = (a, b) => a + b;
exports.multiply = (a, b) => a * b;

// main.js
const math = require('./math');
console.log(math.add(2, 3)); // 5
```

#### ES Modules

```javascript
// math.mjs
export const add = (a, b) => a + b;
export const multiply = (a, b) => a * b;

// main.mjs
import { add, multiply } from './math.mjs';
console.log(add(2, 3)); // 5
```

### 内置模块

#### 文件系统

```javascript
import { readFile, writeFile } from 'fs';

const content = await readFile('file.txt', 'utf8');
await writeFile('output.txt', content.toUpperCase());
```

#### 网络

```javascript
import { http } from 'net';

const server = http.createServer((req, res) => {
    res.writeHead(200, { 'Content-Type': 'text/plain' });
    res.end('Hello from Amber!');
});

server.listen(3000, () => {
    console.log('Server running on http://localhost:3000');
});
```

#### 加密

```javascript
import { crypto } from 'crypto';

const hash = crypto.createHash('sha256');
hash.update('Hello, Amber!');
console.log(hash.digest('hex'));
```

## 🎯 高级功能

### 性能优化

Amber 提供了多种性能优化功能：

```javascript
// 启用所有优化
import { optimization } from 'beejs/optimization';

optimization.enableJit(true);
optimization.enableInlineCache(true);
optimization.enableFastPaths(true);

// 性能监控
import { monitor } from 'beejs/monitor';

monitor.startProfiling('my-function');

myFunction();

const profile = monitor.stopProfiling('my-function');
console.log(profile.executionTime);
console.log(profile.memoryUsage);
```

### 并发执行

```javascript
import { concurrent } from 'beejs/concurrent';

// 并发执行多个函数
const results = await concurrent.run([
    () => heavyComputation(1),
    () => heavyComputation(2),
    () => heavyComputation(3)
]);

console.log(results); // [result1, result2, result3]
```

### 智能缓存

```javascript
import { cache } from 'beejs/cache';

// 创建缓存
const myCache = cache.create({
    maxSize: 1000,
    ttl: 60000 // 1分钟
});

// 缓存函数结果
const cachedFunction = myCache.wrap(expensiveFunction);

// 第一次调用会执行函数
const result1 = await cachedFunction('key1');

// 第二次调用会从缓存返回
const result2 = await cachedFunction('key1');
```

### AI 集成

```javascript
import { ai } from 'beejs/ai';

// AI 代码生成
const code = await ai.generateCode('create a function to sort an array');
console.log(code);

// AI 优化建议
const suggestions = await ai.optimizeCode(myFunction);
console.log(suggestions);
```

## 🐛 调试与性能分析

### 调试器使用

```javascript
// 启用调试模式
bee debug script.js

// 在脚本中使用调试器
import { debugger } from 'beejs/debugger';

// 设置断点
debugger.setBreakpoint('script.js', 10);

// 单步执行
debugger.stepInto();

// 查看变量
debugger.printVariable('myVariable');

// 查看调用栈
debugger.backtrace();
```

### 性能分析器

```javascript
import { profiler } from 'beejs/profiler';

// 开始分析
profiler.start();

// 运行代码
myFunction();

// 生成报告
const report = profiler.generateReport();
console.log('火焰图:', report.flameGraph);
console.log('性能瓶颈:', report.bottlenecks);
console.log('优化建议:', report.recommendations);
```

### 内存分析器

```javascript
import { memory } from 'beejs/memory';

// 启用内存监控
memory.startMonitoring();

// 创建堆快照
const snapshot = memory.takeHeapSnapshot();

// 检测内存泄漏
const leaks = memory.detectLeaks();
if (leaks.length > 0) {
    console.log('检测到内存泄漏:', leaks);
}

// 生成内存报告
const report = memory.generateReport();
console.log('内存使用:', report.usage);
console.log('内存效率:', report.efficiency);
```

## 💡 最佳实践

### 性能优化

1. **启用 JIT 优化**
   ```javascript
   import { optimization } from 'beejs/optimization';
   optimization.enableJit(true);
   ```

2. **使用智能缓存**
   ```javascript
   import { cache } from 'beejs/cache';
   const cached = cache.wrap(expensiveFunction);
   ```

3. **避免频繁的内存分配**
   ```javascript
   // 好的做法：复用对象
   const buffer = new ArrayBuffer(1024);
   reuseBuffer(buffer);

   // 避免：频繁创建新对象
   for (let i = 0; i < 1000; i++) {
       process(new ArrayBuffer(1024)); // 不好
   }
   ```

4. **使用并发执行 I/O 密集型任务**
   ```javascript
   import { concurrent } from 'beejs/concurrent';
   const results = await concurrent.run([
       () => readFile('file1.txt'),
       () => readFile('file2.txt'),
       () => readFile('file3.txt')
   ]);
   ```

### 内存管理

1. **及时释放不需要的引用**
   ```javascript
   let largeData = loadLargeData();
   processData(largeData);
   largeData = null; // 及时释放
   ```

2. **监控内存使用**
   ```javascript
   import { memory } from 'beejs/memory';
   memory.startMonitoring();
   // 定期检查内存使用
   setInterval(() => {
       const usage = memory.getUsage();
       console.log(`内存使用: ${usage}MB`);
   }, 5000);
   ```

3. **使用对象池**
   ```javascript
   import { pool } from 'beejs/pool';
   const objectPool = pool.create(ObjectClass, 100);

   // 获取对象
   const obj = objectPool.acquire();
   useObject(obj);
   // 归还对象
   objectPool.release(obj);
   ```

### 错误处理

1. **使用 try-catch**
   ```javascript
   try {
       riskyOperation();
   } catch (error) {
       console.error('操作失败:', error.message);
       // 错误恢复
       fallbackOperation();
   }
   ```

2. **启用详细错误信息**
   ```javascript
   import { error } from 'beejs/error';
   error.setLevel('verbose');
   error.enableStackTrace(true);
   ```

## ❓ 常见问题

### Q: Amber 与 Node.js 有什么区别？

A: Amber 是用 Rust 和 V8 构建的高性能 JavaScript 运行时，专为 AI 时代优化。与 Node.js 相比：
- 启动速度更快 (5ms vs 50ms)
- 内存使用更少 (10MB vs 30MB)
- 执行性能更高 (100M+ ops/sec)
- 原生 TypeScript 支持
- 内置 AI 集成

### Q: 如何提升性能？

A: 启用 Amber 的优化功能：
```javascript
import { optimization } from 'beejs/optimization';
optimization.enableAll();
```

### Q: 支持哪些 TypeScript 功能？

A: Amber 用 oxc 转译 TypeScript 6.0 语法（与 TypeScript 7.0 语言面兼容），再交给 V8 执行。这是 transpile-only，不做完整 `tsc` 类型检查。已覆盖的语法包括：
- 类型注解、泛型、接口、枚举、命名空间
- `satisfies`、`import type`、`as const`
- `const` 类型参数、Stage 3 装饰器、`using` / `await using`
- TSX（classic JSX，`React.createElement`）

### Q: 如何调试内存泄漏？

A: 使用 Amber 的内存分析器：
```javascript
import { memory } from 'beejs/memory';
memory.startMonitoring();
// 运行你的应用
const leaks = memory.detectLeaks();
console.log('泄漏:', leaks);
```

### Q: 如何报告 bug？

A: 请在 GitHub 上提交 issue：
1. 访问 https://github.com/zh30/beejs/issues
2. 点击 "New issue"
3. 选择 "Bug report"
4. 填写详细信息

## 📖 更多资源

- [API 文档](api/)
- [性能基准测试](BENCHMARK_RESULTS.md)
- [开发指南](DEVELOPMENT_SUMMARY.md)
- [故障排除指南](TROUBLESHOOTING.md)

## 🤝 社区与支持

- [GitHub](https://github.com/zh30/beejs)
- [讨论区](https://github.com/zh30/beejs/discussions)
- [问题反馈](https://github.com/zh30/beejs/issues)

---

感谢使用 Amber！如果您有任何问题或建议，欢迎随时联系我们。
