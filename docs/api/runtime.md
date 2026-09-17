# 运行时 API

`amberjs::RuntimeLite` 是 Amber 运行时系统的核心入口点。

## 概述

RuntimeLite 提供了高性能的 JavaScript/TypeScript 执行环境，集成了 V8 引擎、智能缓存、并发执行等特性。

## 核心结构

### RuntimeLite

```rust
pub struct RuntimeLite {
    // 内部字段
    v8_isolate: Option<*mut v8::OwnedIsolate>,
    smart_cache: SmartCache,
    performance_monitor: PerformanceMonitor,
    // ...
}
```

## 构造函数

### new()

创建新的运行时实例。

```rust
impl RuntimeLite {
    pub fn new() -> Result<Self> {
        // 初始化 V8 引擎
        // 设置智能缓存
        // 配置性能监控
        // ...
    }
}
```

**示例**:
```rust
use amberjs::RuntimeLite;

let runtime = match RuntimeLite::new() {
    Ok(r) => {
        println!("✅ Runtime initialized successfully");
        r
    },
    Err(e) => {
        eprintln!("❌ Failed to initialize runtime: {}", e);
        std::process::exit(1);
    }
};
```

## 文件执行

### execute_file()

执行指定的 JavaScript/TypeScript 文件。

```rust
impl RuntimeLite {
    pub fn execute_file<P: AsRef<Path>>(&self, path: P) -> Result<Value> {
        let path = path.as_ref();
        // 1. 检查缓存
        // 2. 加载文件
        // 3. 解析和编译
        // 4. 执行
        // ...
    }
}
```

**参数**:
- `path`: 要执行的文件路径

**返回值**:
- `Result<Value>`: 执行结果或错误

**示例**:
```rust
use amberjs::RuntimeLite;
use std::path::Path;

let runtime = RuntimeLite::new()?;

// 执行 JavaScript 文件
let result = runtime.execute_file("examples/hello.js")?;
println!("Result: {:?}", result);

// 执行 TypeScript 文件 (自动编译)
let result = runtime.execute_file("examples/basics/typescript_demo.ts")?;
```

### execute_file_with_options()

使用指定选项执行文件。

```rust
pub fn execute_file_with_options<P: AsRef<Path>>(
    &self,
    path: P,
    options: ExecutionOptions
) -> Result<Value>
```

**示例**:
```rust
use amberjs::cli::ExecutionOptions;

let options = ExecutionOptions {
    enable_debugger: true,
    enable_profiling: true,
    cache_enabled: true,
    ..Default::default()
};

let result = runtime.execute_file_with_options("script.js", options)?;
```

## 代码执行

### execute_string()

直接执行 JavaScript 代码字符串。

```rust
impl RuntimeLite {
    pub fn execute_string(&self, code: &str) -> Result<Value> {
        // 1. 创建 V8 上下文
        // 2. 编译代码
        // 3. 执行并返回结果
        // ...
    }
}
```

**参数**:
- `code`: 要执行的 JavaScript 代码

**返回值**:
- `Result<Value>`: 执行结果

**示例**:
```rust
let runtime = RuntimeLite::new()?;

// 执行简单表达式
let result = runtime.execute_string("1 + 1")?;
assert_eq!(result.as_i32(), Some(2));

// 执行函数
let result = runtime.execute_string("Math.sqrt(16)")?;
assert_eq!(result.as_f64(), Some(4.0));

// 执行复杂逻辑
let code = r#"
    function fibonacci(n) {
        if (n <= 1) return n;
        return fibonacci(n - 1) + fibonacci(n - 2);
    }
    fibonacci(10)
"#;
let result = runtime.execute_string(code)?;
```

### execute_string_with_context()

使用自定义上下文执行代码。

```rust
pub fn execute_string_with_context(
    &self,
    code: &str,
    context: &mut v8::Context
) -> Result<Value>
```

## 异步执行

### execute_file_async()

异步执行文件。

```rust
pub async fn execute_file_async<P: AsRef<Path>>(&self, path: P) -> Result<Value> {
    // 异步加载和执行
    // ...
}
```

**示例**:
```rust
use amberjs::RuntimeLite;

#[tokio::main]
async fn main() -> Result<()> {
    let runtime = RuntimeLite::new()?;

    // 异步执行文件
    let result = runtime.execute_file_async("async_script.js").await?;
    println!("Async result: {:?}", result);

    Ok(())
}
```

### execute_string_async()

异步执行代码字符串。

```rust
pub async fn execute_string_async(&self, code: &str) -> Result<Value>
```

## 配置选项

### ExecutionOptions

```rust
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    pub enable_debugger: bool,
    pub enable_profiling: bool,
    pub cache_enabled: bool,
    pub timeout_ms: Option<u64>,
    pub memory_limit_mb: Option<u64>,
}
```

**示例**:
```rust
use amberjs::cli::ExecutionOptions;

let options = ExecutionOptions {
    enable_debugger: true,
    enable_profiling: false,
    cache_enabled: true,
    timeout_ms: Some(5000), // 5 秒超时
    memory_limit_mb: Some(256), // 256MB 内存限制
    ..Default::default()
};
```

## 性能监控

### get_performance_metrics()

获取性能指标。

```rust
impl RuntimeLite {
    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.performance_monitor.get_metrics()
    }
}
```

**返回值**:
```rust
pub struct PerformanceMetrics {
    pub execution_time_ms: u64,
    pub memory_usage_bytes: u64,
    pub gc_pause_time_ms: u64,
    pub cache_hit_rate: f64,
    // ...
}
```

**示例**:
```rust
let runtime = RuntimeLite::new()?;

// 执行代码
runtime.execute_file("script.js")?;

// 获取性能指标
let metrics = runtime.get_performance_metrics();
println!("执行时间: {}ms", metrics.execution_time_ms);
println!("内存使用: {} bytes", metrics.memory_usage_bytes);
println!("缓存命中率: {:.2}%", metrics.cache_hit_rate * 100.0);
```

## 错误处理

运行时使用 `anyhow::Result` 进行错误处理：

```rust
pub type Result<T> = std::result::Result<T, anyhow::Error>;
```

**常见错误**:
- `ScriptNotFound`: 脚本文件不存在
- `CompilationError`: 代码编译错误
- `ExecutionError`: 代码执行错误
- `TimeoutError`: 执行超时

**示例**:
```rust
use amberjs::RuntimeLite;
use anyhow::Result;

fn run_script() -> Result<()> {
    let runtime = RuntimeLite::new()?;

    match runtime.execute_file("script.js") {
        Ok(result) => {
            println!("✅ Script executed successfully: {:?}", result);
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            // 处理错误或返回
            return Err(e);
        }
    }

    Ok(())
}
```

## 最佳实践

### 1. 重用运行时实例

```rust
// ❌ 不推荐：每次执行都创建新实例
fn bad_example() {
    for i in 0..100 {
        let runtime = RuntimeLite::new().unwrap();
        runtime.execute_file(&format!("script{}.js", i)).unwrap();
    }
}

// ✅ 推荐：重用实例
fn good_example() {
    let runtime = RuntimeLite::new().unwrap();
    for i in 0..100 {
        runtime.execute_file(&format!("script{}.js", i)).unwrap();
    }
}
```

### 2. 启用缓存

```rust
let runtime = RuntimeLite::new()?;

// 启用缓存以提高重复执行性能
let options = ExecutionOptions {
    cache_enabled: true,
    ..Default::default()
};
runtime.execute_file_with_options("script.js", options)?;
```

### 3. 监控性能

```rust
let runtime = RuntimeLite::new()?;

// 执行前记录开始时间
let start = std::time::Instant::now();

// 执行代码
runtime.execute_file("script.js")?;

// 执行后检查性能
let elapsed = start.elapsed();
println!("执行耗时: {:?}", elapsed);
```

## 相关资源

- [V8 引擎 API](v8_engine.md) - V8 引擎详细文档
- [模块系统 API](module_system.md) - 模块加载和解析
- [性能监控 API](../monitoring.md) - 详细性能指标
- [示例代码](../../examples/) - 更多用法示例

## 变更日志

### v0.1.0
- 初始版本发布
- 支持文件执行和字符串执行
- 集成智能缓存
- 性能监控支持
