---
title: "动态包运行器 (bee x / bee dlx)"
subtitle: "免安装即时运行远程 npm 包与 CLI 工具，带内容寻址全局缓存与 Agent 安全沙箱"
group: "生态与扩展"
id: "package-manager-dlx"
---

## 1. 什么是 `bee x` / `bee dlx`？

类似于 `npx` 或 `bunx`，`bee x`（别名 `bee dlx`）允许开发者在**无需预先运行 `npm install`**、不污染本地 `node_modules` 与 `package.json` 的情况下，即时从 npm registry 下载并执行远程包的 CLI 二进制文件。

传统 `npx` 往往速度缓慢，且会重复下载依赖。`bee x` 在 Rust 原生层进行了三项核心优化：
1. **并发流式下载与解压**：直接使用 Rust 异步 HTTP 流下载 tarball 并以管道形式解包至内容寻址缓存目录；
2. **全局缓存去重**：默认缓存在 `~/.beejs/x_cache`，同版本包仅下载一次，二次执行达到亚毫秒级启动；
3. **原生 V8 执行与沙箱保护**：自动识别 `package.json` 的 `bin` 字段，直接由 `bee run` 驱动，并可选配超时与内存配额保护。

---

## 2. 基础语法与命令示例

```bash
# 基础调用语法
bee x <package-specifier> [arguments...]

# 别名同样支持
bee dlx cowsay "Hello Beejs!"
```

### 包版本与作用域指定

```bash
# 1. 运行最新版包
bee x cowsay "Hello Beejs!"

# 2. 指定精准版本号
bee x prettier@3.2.5 --write "src/**/*.ts"

# 3. 运行带 npm 作用域（Scope）的包
bee x @biomejs/biome check ./src

# 4. 指定入口文件或子包
bee x typescript tsc --init
```

---

## 3. 高级选项与安全沙箱

`bee x` 继承了 Beejs 的安全执行哲学，支持对未知的第三方脚本施加确定性安全边界：

```bash
# 限制最大运行时间为 10 秒（防止远程脚本死循环）
bee x --timeout 10000 untrusted-script

# 限制最大堆内存占用为 128MB（防止内存溢出与恶意 DoS）
bee x --max-memory 134217728 heavy-cli-tool

# 强制跳过本地缓存，重新拉取最新版本
bee x --no-cache cowsay "Always Fresh"
```

### 选项参考

| 标志 | 简写 | 描述 | 默认值 |
| :--- | :--- | :--- | :--- |
| `--timeout <ms>` | `-t` | 设置最大执行超时毫秒数 | 无限制 |
| `--max-memory <bytes>` | `-m` | 设置 V8 堆内存物理硬上限（字节） | 无限制 |
| `--no-cache` | | 跳过 `~/.beejs/x_cache` 强制在线下载 | `false` |
| `--registry <url>` | | 自定义 npm registry 镜像地址 | 官方 npm 镜像 |

---

## 4. 在 CI/CD 与自动化脚本中的优势

在 GitHub Actions 或轻量级 Docker 构建流水线中，使用 `bee x` 可以免除繁琐的 `npm install` 步骤，将脚手架生成、代码检查、以及临时工具执行的耗时由原本的十余秒降低至百毫秒内：

```bash
# 在 CI 中直接调用最新检查工具
bee x eslint src/
bee x rimraf dist/
```
