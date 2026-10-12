---
title: "Installation"
nav: "Installation"
subtitle: "Install the amber binary from a release archive, or build it from source with Cargo."
status: "stable"
id: "installation"
---

## macOS and Linux

```bash
curl -fsSL https://get.amberjs.com/install.sh | sh
```

The script downloads the release archive for your OS and CPU from GitHub Releases and installs it as `~/.amber/bin/amber`. Add that directory to your `PATH` if the script asks you to.

You can change what it installs with environment variables:

| Variable | Default | Purpose |
| :--- | :--- | :--- |
| `AMBER_VERSION` | latest release | Tag to install, for example `v1.17.0` |
| `AMBER_INSTALL_DIR` | `~/.amber/bin` | Where the `amber` binary goes |
| `AMBER_REPO` | `zh30/amberjs` | GitHub repository to download from |

```bash
AMBER_VERSION=v1.17.0 AMBER_INSTALL_DIR=~/.local/bin sh -c "$(curl -fsSL https://get.amberjs.com/install.sh)"
```

## Windows

```powershell
irm https://get.amberjs.com/install.ps1 | iex
```

The PowerShell script installs `amber.exe` under `%LOCALAPPDATA%\amber\bin`. It honors the same `AMBER_VERSION`, `AMBER_INSTALL_DIR`, and `AMBER_REPO` variables.

## Build from source

Building from source is the most direct path and is part of the Stable scope.

```bash
git clone https://github.com/zh30/amberjs
cd amberjs
cargo build --release
./target/release/amber --version
```

The repository pins its Rust toolchain in `rust-toolchain.toml`. The first build downloads a prebuilt V8 library through the `v8` crate, so it needs network access.

## Release assets

Each GitHub release publishes archives for Linux, macOS, and Windows, plus checksums, an SBOM, and cosign signatures. The archives are named `amber-v<version>-<target>`.

## Other channels

A Homebrew formula, a GHCR container image (`ghcr.io/zh30/amberjs`, linux/amd64 only), and a winget manifest exist in the repository. They are **Experimental** distribution channels, so this manual does not recommend them yet.

## Verify the install

```bash
amber --version
amber eval "1 + 1"
```
