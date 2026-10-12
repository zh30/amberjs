---
title: "打包与编译"
nav: "打包与编译"
subtitle: "amber bundle 把模块图打成一个文件，amber compile 把它变成当前主机的单文件可执行程序。"
status: "stable"
id: "bundling-compilation"
---

## amber bundle

`amber bundle` 是 **Stable**（G1），契约见 [`docs/BUNDLE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/BUNDLE_CONTRACT.md)。

```bash
amber bundle src/index.ts -o dist/app.js
amber bundle src/index.ts -o dist/app.js --minify --sourcemap
```

它读取本地 JS、TS 或 JSON 模块图中的静态 `import` 和 `require`，用 oxc 输出一个 IIFE。

### 限制

- 只跟随静态 `import` 和 `require`。
- 无法解析的裸模块名会保留为运行时 `require()` 调用。
- 缺失的相对导入或非 JS/JSON 资源会以 `error: amber bundle:` 失败。
- `--sourcemap` 输出的 SourceMap v3 只列出源文件，`mappings` 为空。
- `--target` 只写一行头部注释，`--tree-shake` 会被接受但不生效。
- 只使用 `package.json` 的 `"main"`，不使用 `"exports"`。

这不是 webpack、Rollup 或 esbuild。

## amber compile

`amber compile` 是 **Stable**（G2），契约见 [`docs/COMPILE_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/COMPILE_CONTRACT.md)。

```bash
amber compile src/cli.ts -o mytool
./mytool --help
```

它复制当前运行的 `amber` 可执行文件，再追加打包后的脚本和 `AMBER_STANDALONE` trailer。在 macOS 上，载荷放在 `__AMBER` Mach-O 段中，并做 ad-hoc 签名。

### 限制

- 仅支持 Linux、macOS 和 Windows 主机，不支持交叉编译。
- 产物与构建它的 `amber` 使用同一个动态链接器。
- 拒绝原生 `.node` 插件。
- 动态 `import()` 和计算型 `require()` 会让编译失败。
- 没有内嵌虚拟文件系统。`--help` 和 `--version` 会透传给你的脚本。

这不是 pkg、nexe 或 `bun build --compile`。
