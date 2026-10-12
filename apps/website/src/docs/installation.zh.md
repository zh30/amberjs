---
title: "安装"
nav: "安装"
subtitle: "从发布包安装 amber，或用 Cargo 从源码构建。"
status: "stable"
id: "installation"
---

## macOS 与 Linux

```bash
curl -fsSL https://get.amberjs.com/install.sh | sh
```

脚本会从 GitHub Releases 下载适合当前系统和 CPU 的发布包，并安装为 `~/.amber/bin/amber`。如果脚本提示，请把该目录加入 `PATH`。

可以用环境变量调整安装行为：

| 变量 | 默认值 | 作用 |
| :--- | :--- | :--- |
| `AMBER_VERSION` | 最新版本 | 要安装的标签，例如 `v1.17.0` |
| `AMBER_INSTALL_DIR` | `~/.amber/bin` | `amber` 的安装目录 |
| `AMBER_REPO` | `zh30/amberjs` | 下载所用的 GitHub 仓库 |

```bash
AMBER_VERSION=v1.17.0 AMBER_INSTALL_DIR=~/.local/bin sh -c "$(curl -fsSL https://get.amberjs.com/install.sh)"
```

## Windows

```powershell
irm https://get.amberjs.com/install.ps1 | iex
```

PowerShell 脚本会把 `amber.exe` 安装到 `%LOCALAPPDATA%\amber\bin`，同样支持 `AMBER_VERSION`、`AMBER_INSTALL_DIR` 和 `AMBER_REPO`。

## 从源码构建

从源码构建是最直接的方式，也属于 Stable 范围。

```bash
git clone https://github.com/zh30/amberjs
cd amberjs
cargo build --release
./target/release/amber --version
```

仓库在 `rust-toolchain.toml` 中固定了 Rust 工具链。首次构建会通过 `v8` crate 下载预编译的 V8 库，因此需要联网。

## 发布产物

每个 GitHub Release 都会发布 Linux、macOS、Windows 的压缩包，以及校验和、SBOM 和 cosign 签名。压缩包命名为 `amber-v<version>-<target>`。

## 其他渠道

仓库中还有 Homebrew formula、GHCR 容器镜像（`ghcr.io/zh30/amberjs`，仅 linux/amd64）和 winget 清单。它们属于 **Experimental** 分发渠道，本手册暂不推荐。

## 验证安装

```bash
amber --version
amber eval "1 + 1"
```
