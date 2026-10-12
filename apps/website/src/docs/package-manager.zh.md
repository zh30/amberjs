---
title: "包安装器"
nav: "amber install"
subtitle: "从 package.json 安装直接依赖，支持 lockfile 固定和完整性校验。不是 npm 的替代品。"
status: "stable"
id: "package-manager"
---

`amber install` 是 **Stable**（G3），范围以 [`docs/INSTALL_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/INSTALL_CONTRACT.md) 为准。

## 用法

```bash
amber install
amber install --frozen-lockfile
```

## 它会做什么

- 读取 `./package.json` 中的 `dependencies` 和 `devDependencies`。
- 从 `https://registry.npmjs.org` 下载 tarball 到 `node_modules`。
- 如果 `./package-lock.json` 的顶层 `dependencies` 中有该包，会校验版本请求、`resolved` URL 和 `integrity`。
- 解包前对每个 tarball 做哈希校验（SRI `sha512`、`sha384`、`sha256` 或 `sha1`，否则使用 SHA-1 `shasum`）。
- `--frozen-lockfile` 在 lock 缺失或直接依赖与 lock 不一致时失败，并且不会改写 lock。

契约内的失败会打印 `error: amber install:` 并以非零码退出。

## 它不做什么

`amber install` 不是 npm、yarn 或 pnpm。以下内容在契约之外：

- workspaces、生命周期脚本（`postinstall` 等）和 `.bin` 链接。
- 自定义 registry 和 `.npmrc`。
- 安装 `peerDependencies`。
- lockfile 的 `packages` 映射、嵌套 lock 条目、`yarn.lock` 和 `pnpm-lock.yaml`。
- 传递依赖的 lock 固定。未锁定的范围按 registry 返回顺序选择，而不是最高的匹配 semver。

`optionalDependencies` 下载出错不会让命令失败。

## Experimental 包命令

`amber add`、`amber remove`、`amber prune`、`amber upgrade`、`amber init`、`amber create` 和 `amber x`（也叫 `dlx`、`bunx`）存在，但属于 **Experimental**，不在安装契约内。

```bash
amber add lodash@4.17.21   # Experimental
amber x cowsay hello       # Experimental
```
