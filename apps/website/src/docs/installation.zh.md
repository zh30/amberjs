---
title: "安装"
subtitle: "安装 v1.16.0 预编译包，或从源码构建"
group: "开始"
id: "installation"
---

## 快速安装 (推荐)

在 macOS 或 Linux 系统上，你可以直接在终端中运行以下一键安装脚本：

```bash
curl -fsSL https://get.amberjs.com/install.sh | sh

# 固定版本
curl -fsSL https://get.amberjs.com/install.sh | AMBER_VERSION=v1.16.0 sh
```

Windows（PowerShell）：

```powershell
irm https://get.amberjs.com/install.ps1 | iex
```

Homebrew（配方在 Amber 仓库内；每次 GitHub Release 后填写 sha256）：

```bash
brew install zh30/tap/amber
```

crates.io（GitHub `v*` 标签通过 Release Assets + `CARGO_REGISTRY_TOKEN` 按 `amber_transpile` → `amber_sandbox` → `amberjs` 发布）：

```bash
cargo install amberjs
```

会安装 `amber` 二进制。需要 Rust **1.97.1** 以及编译 V8 的 C++ 工具链。

### 安装脚本执行过程说明

1. **自动识别硬件与系统**：自动检测你的系统（macOS / Linux）与 CPU 架构（Apple Silicon `arm64`、Intel `x86_64`）；
2. **下载预编译产物**：从官方发布源下载经过 `-O3` 生产优化的二进制压缩包并校验完整性；
3. **部署到用户主目录**：将可执行文件 `amber` 解压部署至 `~/.amber/bin/amber`；
4. **自动配置环境变量**：自动检测当前 Shell（`~/.zshrc`、`~/.bashrc` 等），在文件末尾注入 `export PATH="$HOME/.amber/bin:$PATH"`。

安装完成后，打开一个新的终端窗口或执行 `source ~/.zshrc`（或 `source ~/.bashrc`），即可直接使用 `amber` 命令。

---

## 验证安装

运行以下命令，验证 Amber 是否正确安装并就绪：

```bash
# 查看版本号
amber --version

# 快速运行 JavaScript 代码片段
amber eval "1 + 1"
```

看到类似输出即可：

```text
amber 1.16.0
2
```

---

## 支持平台与硬件要求

| 操作系统 | CPU 架构 | 运行环境要求 | 预编译包支持 |
| :--- | :---: | :---: | :---: |
| **macOS (Apple Silicon)** | `arm64` (M1/M2/M3/M4) | macOS 12.0+ (Monterey 及以上) | ✅ 官方支持 |
| **macOS (Intel)** | `x86_64` | macOS 12.0+ | ✅ 官方支持 |
| **Linux** | `x86_64` | Linux Kernel 4.18+, glibc 2.28+ | ✅ 官方支持 |
| **Linux (ARM64)** | `aarch64` | Linux Kernel 4.18+, glibc 2.28+ | ✅ 官方支持 |
| **Windows** | `x86_64` | Windows 10+ | ✅ `install.ps1` zip |

> [!NOTE]
> Windows 也可在 **WSL 2** 下使用 Unix 的 `install.sh`。

---

## 从源码编译构建

如果你需要对 Amber 进行定制化开发、本地调试或在未提供预编译产物的操作系统上运行，可以通过 Rust 工具链从源码编译。

### 1. 安装编译依赖

确保本地已安装：

- **Rust** **1.97.1**（仓库已锁定；用 [rustup.rs](https://rustup.rs) 安装）
- **Clang / LLVM**（V8 编译与 C++ 绑定必需）
- **Python 3**（构建辅助脚本）

在 Ubuntu / Debian 上安装基础工具：

```bash
sudo apt update && sudo apt install -y build-essential clang llvm git curl cmake python3
```

在 macOS 上：

```bash
xcode-select --install
```

### 2. 克隆仓库并构建

```bash
git clone https://github.com/zh30/amberjs.git
cd amberjs

# 生产级优化编译 (耗时约 5~15 分钟，视机器性能而定)
cargo build --release

# 编译生成的可执行文件位于：
./target/release/amber --version
```

### 3. 安装到全局 PATH

```bash
sudo cp ./target/release/amber /usr/local/bin/
amber --version
```

---

## 环境变量配置

Amber 支持通过环境变量调整全局运行时行为：

| 环境变量 | 默认值 | 作用说明 |
| :--- | :---: | :--- |
| `AMBER_WORKERS` | `1` | 设置 HTTP 服务或并发任务的默认 Worker 线程池并发数 |
| `AMBER_HOME` | `~/.amber` | 指定 Amber 的缓存、下载与全局配置目录 |
| `AMBER_AUDIT_LOG` | 无 | 指定沙箱全局安全审计日志 JSONL 输出文件路径 |
| `AMBER_LOG` | `info` | 设置日志级别（`error`、`warn`、`info`、`debug`、`trace`） |

示例：在 `~/.zshrc` 或生产环境 Dockerfile 中设置：

```bash
export AMBER_WORKERS=8
export AMBER_LOG=warn
```

---

## 卸载与清理

如果需要卸载 Amber，只需删除安装目录并移除 PATH 配置：

```bash
# 1. 移除二进制文件与缓存
rm -rf ~/.amber

# 2. 从 Shell 配置文件中移除 PATH (编辑 ~/.zshrc 或 ~/.bashrc 删除对应 export 行)
```
