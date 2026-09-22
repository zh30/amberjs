---
title: "打包与编译"
subtitle: "Stable amber bundle（本地图）；Preview SEA amber compile"
group: "开发者工具"
id: "bundling-compilation"
---

两个打包工具：

1. **`amber bundle`** — **Stable**。把本地 JS/TS/JSON 图打成一个 IIFE
2. **`amber compile`** — **Preview**。复制 `amber` 二进制并追加脚本 payload（SEA）

`amber bundle` 不是 webpack / rollup / esbuild / pkg 的对等实现。完整契约：[BUNDLE_CONTRACT.md](https://github.com/zh30/amberjs/blob/main/docs/BUNDLE_CONTRACT.md)。

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

## `amber compile`

```bash
amber compile app.ts -o myapp
./myapp
```

输出二进制布局：

```text
+----------------------------------------------------------+
|  Amber 运行时（宿主 `amber` 的一份拷贝）                    |
+----------------------------------------------------------+
|  打包后的用户脚本                                        |
+----------------------------------------------------------+
|  payload 长度 (u64)  |  魔数 AMBER_STANDALONE（16 字节）   |
+----------------------------------------------------------+
```

启动时 `amber` 检查自己的 trailer。如果有 `AMBER_STANDALONE`，就直接跑内嵌脚本，跳过普通 CLI 解析。

限制：产物体积大约是 `amber` 本身加上脚本；原生 addon 和完整 Node 模块图不在范围内。

---

## 状态

| | 状态 |
| :--- | :--- |
| `amber bundle` | **Stable**（限制见上方契约） |
| `amber compile` | Preview |
| 测试 | `tests/bundle_contract_tests.rs`、`tests/bundler_integration_tests.rs` |
