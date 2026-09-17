# Amber API 文档

> 当前状态：此文件保留为历史 API 设计草案，不代表 Amber v0.1 当前公开用户合同。
> v0.1 的公开入口以 `Cargo.toml`、`src/main.rs`、`src/lib.rs`、`docs/CLI_USAGE_GUIDE.md`
> 和可执行测试为准。下面提到的 JS `Runtime` 构造器、覆盖率、benchmark、heap/profile
> 等接口需要重新验证或重新设计后才能作为公开 API 发布。

欢迎使用 Amber API 文档！这里记录历史阶段的 API 设计意图。

## 📚 目录

- [核心运行时 API](#核心运行时-api)
- [测试框架 API](#测试框架-api)
- [调试器 API](#调试器-api)
- [包管理器 API](#包管理器-api)
- [性能监控 API](#性能监控-api)

## 核心运行时 API

### Runtime

JavaScript 执行引擎的主入口。

#### 初始化

```javascript
// 获取默认运行时实例
const runtime = new Runtime();

// 带配置初始化
const runtime = new Runtime({
    heapSize: "512MB",
    optimizationLevel: "max"
});
```

#### 方法

##### `execute(code, options)`

执行 JavaScript 代码

**参数:**
- `code` (string): 要执行的 JavaScript 代码
- `options` (object, optional): 执行选项
  - `filename` (string): 文件名 (用于错误报告)
  - `lineOffset` (number): 行号偏移
  - `columnOffset` (number): 列号偏移
  - `preview` (boolean): 是否返回预览结果

**返回值:**
- `ExecutionResult`: 执行结果对象

**示例:**

```javascript
const result = runtime.execute(`
    const x = 10;
    const y = 20;
    x + y;
`, { filename: "test.js" });

console.log(result.value); // 30
console.log(result.output); // 控制台输出
console.log(result.error); // 错误 (如果有)
```

##### `compile(code, options)`

预编译 JavaScript 代码

**参数:**
- `code` (string): 要编译的 JavaScript 代码
- `options` (object, optional): 编译选项
  - `mode` (string): "eval" | "function" | "module"
  - `filename` (string): 文件名

**返回值:**
- `CompiledCode`: 编译后的代码对象

**示例:**

```javascript
const compiled = runtime.compile(`
    function add(a, b) {
        return a + b;
    }
`, { mode: "function" });

const result = runtime.runCompiled(compiled, { a: 5, b: 10 });
console.log(result); // 15
```

##### `createContext(options)`

创建新的 V8 上下文

**参数:**
- `options` (object, optional): 上下文选项
  - `timeout` (number): 执行超时时间 (毫秒)
  - `memoryLimit` (number): 内存限制 (字节)
  - `enableDebug` (boolean): 启用调试支持

**返回值:**
- `Context`: 隔离的执行上下文

**示例:**

```javascript
const context = runtime.createContext({
    timeout: 5000,
    enableDebug: true
});

context.execute("console.log('Hello from context!')");
```

### Context

隔离的执行上下文，每个上下文有自己的全局对象。

#### 方法

##### `execute(code)`

在上下文中执行代码

```javascript
const context = runtime.createContext();
context.execute(`
    globalVar = 42;
    console.log(globalVar); // 42
`);

// 全局变量在上下文中
console.log(context.getGlobal("globalVar")); // 42
```

##### `setGlobal(name, value)`

设置全局变量

```javascript
context.setGlobal("PI", 3.14159);
context.execute("console.log(PI);"); // 3.14159
```

##### `getGlobal(name)`

获取全局变量

```javascript
context.setGlobal("answer", 42);
const value = context.getGlobal("answer"); // 42
```

##### `dispose()`

释放上下文资源

```javascript
context.dispose();
```

## 测试框架 API

### Test Runner

测试执行引擎，支持并行执行、快照测试、性能基准等。

#### 创建测试

```javascript
describe("Math Utils", () => {
    test("加法测试", () => {
        expect(2 + 2).toBe(4);
    });

    test("数组测试", () => {
        const arr = [1, 2, 3];
        expect(arr).toHaveLength(3);
    });
});
```

#### 断言方法

##### 基础断言

```javascript
expect(value).toBe(expected)          // 严格相等 (===)
expect(value).toEqual(expected)       // 深度相等
expect(value).not.toBe(expected)      // 不等
```

##### 数字断言

```javascript
expect(number).toBeGreaterThan(n)     // 大于
expect(number).toBeLessThan(n)        // 小于
expect(number).toBeGreaterThanOrEqual(n) // 大于等于
expect(number).toBeLessThanOrEqual(n)    // 小于等于
```

##### 字符串断言

```javascript
expect(str).toContain(substr)         // 包含子串
expect(str).toMatch(regex)            // 匹配正则
expect(str).toHaveLength(n)           // 长度等于
```

##### 数组断言

```javascript
expect(arr).toContain(item)           // 包含元素
expect(arr).toHaveLength(n)           // 长度等于
expect(arr).toEqual(expected)         // 数组相等
```

##### 真值断言

```javascript
expect(value).toBeTruthy()            // 真值
expect(value).toBeFalsy()             // 假值
expect(value).toBeNull()              // null
expect(value).toBeUndefined()         // undefined
expect(value).toBeDefined()           // 已定义
```

##### 对象断言

```javascript
expect(obj).toHaveProperty(key)       // 包含属性
expect(obj).toMatchObject(pattern)    // 匹配对象
```

#### 快照测试

```javascript
test("快照测试", () => {
    const data = {
        id: 1,
        name: "Alice",
        profile: {
            age: 30,
            city: "Beijing"
        }
    };
    expect(data).toMatchSnapshot();
});
```

运行快照测试:

```bash
amber test --update-snapshot  # 更新快照
```

#### 性能测试

```javascript
describe("性能测试", () => {
    benchmark("快速排序", () => {
        const arr = Array.from({ length: 1000 }, () => Math.random());
        return arr.sort((a, b) => a - b);
    });
});
```

历史草案中的性能测试命令:

```bash
# 当前 v0.1 CLI 尚未提供专用 benchmark 子命令。
# 请使用外部 benchmark harness 或脚本内 console.time()。
```

#### 并行测试

```javascript
describe("并行测试", () => {
    test("测试 1", async () => {
        // 并行执行
    });

    test("测试 2", async () => {
        // 并行执行
    });
});
```

启用并行执行:

```bash
amber test --parallel
```

### 测试选项

```javascript
// test.config.js
module.exports = {
    timeout: 30000,              // 测试超时 (毫秒)
    parallel: true,              // 启用并行
    workers: 4,                  // 并行工作进程数
    coverage: true,              // 生成覆盖率报告
    reporter: "spec",            // 报告格式: spec|json|dot|tap
    bail: false,                 // 首次失败后停止
    retry: 0,                    // 失败重试次数
    snapshotDir: "__snapshots__" // 快照目录
};
```

## 调试器 API

### Debugger

高级调试器，支持断点、异步栈追踪、远程调试等。

#### 启动调试

```bash
amber debug script.js
```

#### 断点类型

##### 1. 普通断点

在代码行号处设置断点，程序执行到该行时暂停。

##### 2. 条件断点

只有当条件为真时才暂停。

```javascript
// 在调试器中设置条件: i > 10
for (let i = 0; i < 100; i++) {
    console.log(i); // 断点设置在这一行
}
```

##### 3. 命中次数断点

执行指定次数后暂停。

```javascript
// 设置每 3 次命中暂停一次
for (let i = 0; i < 100; i++) {
    console.log(i);
}
```

##### 4. 日志断点 (Logpoints)

在不断停执行的情况下记录日志。

```javascript
// 转换为日志断点: "i = {i}"
for (let i = 0; i < 100; i++) {
    console.log(i);
}
```

##### 5. 异常断点

在异常抛出时自动暂停。

```javascript
try {
    throw new Error("测试异常");
} catch (e) {
    console.log(e.message);
}
```

#### 调试命令

```javascript
continue (c)      // 继续执行到下一个断点
next (n)          // 执行下一行
step (s)          // 进入函数内部
stepout (so)      // 跳出当前函数
finish (f)        // 完成当前函数
backtrace (bt)    // 显示调用栈
print <var>       // 打印变量值
watch <var>       // 监视变量
quit (q)          // 退出调试器
```

#### 异步栈追踪

自动追踪 `async/await` 调用链。

```javascript
async function asyncFunction() {
    await step1();  // 断点: 完整异步调用栈
    await step2();
    await step3();
}
```

#### 远程调试

##### Chrome DevTools 协议

```javascript
// 启动时启用远程调试
amber debug --remote --port 9229 script.js
```

在 Chrome 中打开: `chrome://inspect`

##### VS Code 集成

创建 `.vscode/launch.json`:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "amberjs",
            "request": "launch",
            "name": "Debug Amber Script",
            "program": "${workspaceFolder}/script.js",
            "port": 9229
        }
    ]
}
```

## 包管理器 API

### Package Manager

包管理功能，支持 npm/yarn/pnpm 兼容。

#### 安装包

```javascript
const { PackageManager } = require('amberjs/package');

const pm = new PackageManager();

// 安装依赖
await pm.install('lodash');

// 从 package.json 安装
await pm.installFromPackageJson();

// 安装开发依赖
await pm.install('jest', { dev: true });
```

#### 解析包规范

```javascript
// 解析包名
const spec = pm.parseSpec('lodash@^4.0.0');
console.log(spec.name);     // "lodash"
console.log(spec.version);  // "^4.0.0"

// Git 包
const gitSpec = pm.parseSpec('git+https://github.com/user/repo.git#v1.0.0');
console.log(gitSpec.type);  // "git"
console.log(gitSpec.url);   // "https://github.com/user/repo.git"
```

#### 版本锁定

```javascript
// 生成 lockfile
await pm.generateLockfile('package-lock.json');

// 从 lockfile 安装
await pm.installFromLockfile('package-lock.json');
```

## 性能监控 API

### Performance Monitor

实时性能追踪和分析。

#### 基础性能监控

```javascript
const { performance } = require('perf_hooks');

// 开始计时
console.time('operation');

// 执行操作
for (let i = 0; i < 1000000; i++) {
    // ...
}

// 结束计时
console.timeEnd('operation');
```

#### 详细性能分析

```javascript
const { PerformanceObserver } = require('perf_hooks');

const obs = new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) {
        console.log(`${entry.name}: ${entry.duration}ms`);
    }
});

obs.observe({ entryTypes: ['measure'] });

performance.mark('start');
// 执行操作
performance.mark('end');
performance.measure('operation', 'start', 'end');
```

#### 内存使用监控

```javascript
const usage = process.memoryUsage();
console.log({
    rss: Math.round(usage.rss / 1024 / 1024) + ' MB',
    heapTotal: Math.round(usage.heapTotal / 1024 / 1024) + ' MB',
    heapUsed: Math.round(usage.heapUsed / 1024 / 1024) + ' MB',
    external: Math.round(usage.external / 1024 / 1024) + ' MB'
});
```

#### 性能基准

```javascript
function benchmark(name, fn, iterations = 10000) {
    const start = performance.now();

    for (let i = 0; i < iterations; i++) {
        fn();
    }

    const end = performance.now();
    const duration = end - start;
    const opsPerSec = (iterations / duration * 1000).toFixed(0);

    console.log(`${name}:`);
    console.log(`  ${duration.toFixed(2)}ms`);
    console.log(`  ${opsPerSec} ops/sec`);
}
```

## 配置选项

### 全局配置

创建 `amberjs.config.json`:

```json
{
    "runtime": {
        "heap": {
            "initial": "256MB",
            "max": "1GB"
        },
        "optimization": {
            "jit": true,
            "inline_cache": true,
            "fast_paths": true
        }
    },
    "testing": {
        "parallel": true,
        "workers": 4,
        "timeout": 30000,
        "coverage": true
    },
    "debugging": {
        "break_on_exception": true,
        "async_stack_trace": true
    },
    "monitoring": {
        "performance": true,
        "memory": true
    }
}
```

### 命令行选项

```bash
amber run script.js
amber test examples/testing/math.test.js --parallel
amber debug script.js
```

## 错误处理

### Error 对象

```javascript
try {
    runtime.execute('invalid code');
} catch (error) {
    console.log(error.name);      // "SyntaxError"
    console.log(error.message);   // 错误消息
    console.log(error.stack);     // 堆栈跟踪
    console.log(error.start);     // 错误开始位置 { line, column }
    console.log(error.end);       // 错误结束位置 { line, column }
}
```

### 自定义错误

```javascript
class ValidationError extends Error {
    constructor(message, field) {
        super(message);
        this.name = 'ValidationError';
        this.field = field;
    }
}
```

## 最佳实践

### 1. 性能优化

```javascript
// ✅ 好的实践: 预热 JIT
for (let i = 0; i < 100; i++) {
    hotFunction(); // 让 JIT 优化函数
}

// ✅ 好的实践: 避免动态类型
let count = 0;     // 始终是数字
count++;
```

### 2. 内存管理

```javascript
// ✅ 及时释放大对象
const largeData = loadLargeData();
processData(largeData);
largeData = null; // 允许 GC

// ✅ 避免内存泄漏
const cache = new Map();
function getCached(key) {
    if (cache.has(key)) return cache.get(key);
    const value = compute(key);
    cache.set(key, value);
    if (cache.size > 1000) cache.clear(); // 限制缓存大小
    return value;
}
```

### 3. 异步处理

```javascript
// ✅ 并发执行
const [data1, data2, data3] = await Promise.all([
    fetch('/api/1'),
    fetch('/api/2'),
    fetch('/api/3')
]);

// ✅ 错误处理
try {
    await riskyOperation();
} catch (error) {
    console.error('Operation failed:', error);
    // 恢复或回退
}
```

---

**更多资源:**
- [快速开始指南](../guides/QUICK_START.md)
- [示例代码](../../examples/)
- [性能优化指南](../performance/)
