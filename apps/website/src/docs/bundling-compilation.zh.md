---
title: "打包与编译"
subtitle: "Stable amber bundle 与 Stable SEA amber compile"
group: "开发者工具"
id: "bundling-compilation"
---

两个打包工具：

1. **`amber bundle`** — **Stable**。把本地 JS/TS/JSON 图打成一个 IIFE
2. **`amber compile`** — **Stable**。复制宿主 `amber` 并嵌入脚本 payload（SEA）

`amber bundle` 不是 webpack / rollup / esbuild 的对等实现。完整契约：[BUNDLE_CONTRACT.md](https://github.com/zh30/amberjs/blob/main/docs/BUNDLE_CONTRACT.md)。`amber compile` 不是 pkg、nexe 或 Bun compile。SEA 契约见 [`docs/COMPILE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/COMPILE_CONTRACT.md)。`amber install` 仍是 Preview。

---

## `amber bundle`

```bash
amber bundle src/index.ts -o dist/bundle.js
amber bundle src/index.ts -o dist/bundle.min.js --minify
amber bundle src/index.ts -o dist/bundle.js --sourcemap
```

契约（由 `tests/bundle_contract_tests.rs` 钉住）：

- 单文件入口。只跟随静态 `import` / `export from` / `require("...")`
- 内联 `.js` / `.mjs` / `.cjs` / `.jsx` / `.ts` / `.tsx` / `.mts` / `.cts` / `.json`
- 解析不到的 bare specifier 保留为运行时 `require()`（Node 内建、未找到的包）
- 解析不到的 `./` / `../` 以及非 JS/JSON 资源以 `error: amber bundle:` 失败，且不写 outfile
- `--sourcemap` 写出 SourceMap v3 `<name>.map`：`sources` 列出模块，`mappings` 为空
- `--target` 只写进文件头注释；`--tree-shake` 接受但不生效
- `node_modules` 只读 `package.json` 的 `"main"`（不读 `"exports"`）

明确不做：code splitting、CSS/图片/Wasm 管线、动态 `import()`、完整 Node 生态打包。

---

## `amber compile`（Stable）

```bash
amber compile app.ts -o myapp
./myapp
```

只支持 Linux、macOS、Windows 本机。产物是**这份** `amber` 的拷贝再加 trailer，不交叉编译，也不是独立于动态链接器的 freestanding 二进制。其他操作系统会以 `unsupported host OS` 失败，并且不写文件。

Linux 与 Windows 的 trailer 是文件最后 32 字节。macOS 把同样的 32 字节放在 `__LINKEDIT` 之前的 `__AMBER` 段末尾，再做 ad-hoc `codesign`；签名才是文件末尾。

布局：

```text
+---------------------------------------------------------------+
|  宿主 amber 可执行文件（原样拷贝）                              |
+---------------------------------------------------------------+
|  UTF-8 打包脚本                                                |
+---------------------------------------------------------------+
|  payload 长度 (u64 LE)  |  flags (u64 LE，必须为 0)            |
+---------------------------------------------------------------+
|  魔数 AMBER_STANDALONE（16 字节）                               |
+---------------------------------------------------------------+
```

`AMBER_STANDALONE` 是这段魔数，不是环境变量。trailer 有效时直接跑 payload，跳过 CLI，所以 `--help` 和 `--version` 是脚本参数。`process.argv[0]` 和 `process.argv[1]` 都是可执行文件路径。

契约内的失败会打印以 `error: amber compile:` 开头的一行（二进制跑起来之后是 `error: amber standalone:`），并且不会留下半成品。

明确限制：

- 动态 `import()` 和计算出来的 `require(expr)` 会使编译失败
- `.node` 原生插件会被拒绝，不会被嵌进去
- 没有虚拟文件系统；worker 和 `fs` 路径都是真实磁盘
- macOS 先插入 `__AMBER` 段，再 ad-hoc `codesign`；任一步失败则编译失败
- 不是 pkg / nexe / Bun compile 的对等实现

---

## 状态

| | 状态 |
| :--- | :--- |
| `amber bundle` | **Stable**（限制见上方契约） |
| `amber compile` | **Stable** |
| 测试 | `tests/bundle_contract_tests.rs`、`tests/compile_contract_tests.rs`、`tests/bundle_compile_tests.rs` |
