# Amber 快速开始指南

本文档描述 Amber v0.1 当前公开 CLI。命令行为以 `Cargo.toml`、
`src/main.rs` 和可执行测试为准。

## 安装

```bash
curl -fsSL https://raw.githubusercontent.com/zh30/amberjs/main/install.sh | sh
amber --version
```

从源码构建：

```bash
git clone https://github.com/zh30/amberjs.git
cd amberjs
cargo build --release
./target/release/amber --version
```

## 常用命令

```bash
amber --help
amber version
amber eval "1 + 1"
amber run examples/basics/hello_world.js
amber run examples/basics/typescript_demo.ts
amber repl
amber test examples/testing/math.test.js
```

`--verbose` 是全局参数，需要放在子命令前：

```bash
amber --verbose run examples/basics/hello_world.js
```

## 第一个脚本

创建 `hello.js`：

```javascript
console.log("Hello from Amber!");
console.log("Rust + V8 runtime");
```

运行：

```bash
amber run hello.js
```

## TypeScript

传入 `.ts` 或 `.tsx` 文件时，CLI 会先调用内置 TypeScript 转译模块，
再交给 V8 执行：

```bash
amber run examples/basics/typescript_demo.ts
```

## 测试

```bash
amber test
amber test examples/testing/math.test.js
amber test examples/testing/math.test.js --test-name-pattern "adds"
amber test examples/testing/math.test.js --bail
amber test examples/testing/math.test.js --timeout 10
```

## Bundle 和 Server

```bash
amber bundle src/index.js --outfile dist/bundle.js
amber bundle src/index.js --outfile dist/bundle.js --minify
amber serve --host localhost --port 3000
```

## 开发验证

```bash
cargo fmt --all -- --check
cargo build
cargo test --lib
cargo test --test timers_enhanced_tests
```

## 版本定位

Amber v0.1 适合运行仓库示例、验证脚本工作流和参与 Node/Web API 兼容层开发。
性能数字必须来自当前可复现的 benchmark 命令；不要把历史阶段报告当作当前事实。
