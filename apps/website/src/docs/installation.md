---
title: "Installation"
subtitle: "Install a v1.16.0 binary, or build from source"
group: "Getting Started"
id: "installation"
---

## Quick Install (Recommended)

On macOS or Linux, run the official one-line install script in your terminal:

```bash
curl -fsSL https://get.amberjs.com/install.sh | sh

# pin a release
curl -fsSL https://get.amberjs.com/install.sh | AMBER_VERSION=v1.16.0 sh
```

On Windows (PowerShell):

```powershell
irm https://get.amberjs.com/install.ps1 | iex
```

Homebrew (formula in the Amber repo; tap hashes are filled after each GitHub Release):

```bash
brew install zh30/tap/amber
```

### What the Install Script Does

1. **Detects Environment**: Automatically identifies your OS (macOS / Linux) and CPU architecture (Apple Silicon `arm64` or Intel `x86_64`).
2. **Fetches Prebuilt Archive**: Downloads the release archive optimized with `-O3` and verifies binary integrity.
3. **Deploys Binary**: Unpacks the `amber` executable into `~/.amber/bin/amber`.
4. **Configures PATH**: Updates your active shell profile (`~/.zshrc`, `~/.bashrc`, etc.) with `export PATH="$HOME/.amber/bin:$PATH"`.

After installation finishes, either restart your terminal or run `source ~/.zshrc` (or `source ~/.bashrc`) to start using `amber`.

---

## Verifying Installation

Verify that `amber` is properly installed and accessible:

```bash
# Print version info
amber --version

# Evaluate a quick JavaScript snippet
amber eval "1 + 1"
```

You should see output similar to:

```text
amber 1.16.0
2
```

---

## Supported Platforms & Requirements

| Operating System | Architecture | Minimum Requirements | Prebuilt Release |
| :--- | :---: | :---: | :---: |
| **macOS (Apple Silicon)** | `arm64` (M1–M4) | macOS 12.0+ (Monterey or later) | ✅ Yes |
| **macOS (Intel)** | `x86_64` | macOS 12.0+ | ✅ Yes |
| **Linux** | `x86_64` | Kernel 4.18+, glibc 2.28+ | ✅ Yes |
| **Linux (ARM64)** | `aarch64` | Kernel 4.18+, glibc 2.28+ | ✅ Yes |
| **Windows** | `x86_64` | Windows 10+ | ✅ `install.ps1` zip |

> [!NOTE]
> Windows also works under **WSL 2** with the Unix `install.sh` script.

---

## Building from Source

To customize Amber, debug internal subsystems, or build for non-standard architectures, compile directly using the Rust toolchain.

### 1. Prerequisites

Ensure you have the following installed:

- **Rust** **1.97.1** (pinned in this repo; [rustup.rs](https://rustup.rs))
- **Clang / LLVM** (required for V8 C++ bindings)
- **Python 3** (build helper scripts)

On Ubuntu / Debian:

```bash
sudo apt update && sudo apt install -y build-essential clang llvm git curl cmake python3
```

On macOS:

```bash
xcode-select --install
```

### 2. Clone and Build

```bash
git clone https://github.com/zh30/amberjs.git
cd amberjs

# Release build with full optimizations
cargo build --release

# The compiled binary is output to:
./target/release/amber --version
```

### 3. Install to Global PATH

```bash
sudo cp ./target/release/amber /usr/local/bin/
amber --version
```

---

## Environment Variables

| Variable | Default | Purpose |
| :--- | :---: | :--- |
| `BEE_WORKERS` | `1` | Default number of worker threads for parallel HTTP execution |
| `BEE_HOME` | `~/.amber` | Base directory for package caches and downloads |
| `BEE_AUDIT_LOG` | None | File path for writing JSONL security sandbox audit records |
| `BEE_LOG` | `info` | Log verbosity (`error`, `warn`, `info`, `debug`, `trace`) |

Example for `~/.zshrc` or Dockerfile:

```bash
export BEE_WORKERS=8
export BEE_LOG=warn
```

---

## Uninstallation

To uninstall Amber, delete the binary directory and remove the PATH entry from your shell configuration:

```bash
# 1. Remove binary and caches
rm -rf ~/.amber

# 2. Remove the PATH export line from ~/.zshrc or ~/.bashrc
```
