---
title: "打包与编译"
subtitle: "v1.16.0 Preview：oxc bee bundle，SEA bee compile"
group: "开发者工具"
id: "bundling-compilation"
---

两个打包工具，都是 **Preview**：

1. **`bee bundle`** — 把本地模块图打成一个 JS 文件
2. **`bee compile`** — 复制 `bee` 二进制并追加脚本 payload（SEA）

契约还在收紧。不要当成 webpack / esbuild / pkg 的对等实现。

---

## `bee bundle`

```bash
amber bundle src/index.ts -o dist/bundle.js
amber bundle src/index.ts -o dist/bundle.min.js --minify
```

当前会做的事：

- 递归跟随静态本地 import
- 用 oxc 擦掉 `.ts` / `.tsx` 类型
- 每个模块包进隔离的 registry（`__beejs_require__`）
- 可选 minify

明确不做的承诺：完整 `node_modules` 生态打包、code splitting、浏览器应用工具链。

---

## `bee compile`

```bash
amber compile app.ts -o myapp
./myapp
```

输出二进制布局：

```text
+----------------------------------------------------------+
|  Amber 运行时（宿主 `bee` 的一份拷贝）                    |
+----------------------------------------------------------+
|  打包后的用户脚本                                        |
+----------------------------------------------------------+
|  payload 长度 (u64)  |  魔数 BEE_STANDALONE（16 字节）   |
+----------------------------------------------------------+
```

启动时 `bee` 检查自己的 trailer。如果有 `BEE_STANDALONE`，就直接跑内嵌脚本，跳过普通 CLI 解析。

限制：产物体积大约是 `bee` 本身加上脚本；原生 addon 和完整 Node 模块图不在范围内。

---

## 状态

| | 状态 |
| :--- | :--- |
| `bee bundle` | Preview |
| `bee compile` | Preview |
| 测试 | `tests/bundle_compile_tests.rs` |
