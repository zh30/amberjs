#!/bin/sh
set -e

AMBER_REPO_DEFAULT="zh30/amberjs"
AMBER_INSTALL_DIR_DEFAULT="${HOME}/.amber/bin"

AMBER_REPO="${AMBER_REPO:-$AMBER_REPO_DEFAULT}"
AMBER_INSTALL_DIR="${AMBER_INSTALL_DIR:-$AMBER_INSTALL_DIR_DEFAULT}"

usage() {
  cat <<'USAGE'
Amber installer

Usage:
  curl -fsSL https://get.amberjs.com/install.sh | sh

Environment variables:
  AMBER_VERSION     Version tag to install (example: v1.16.0 or 1.16.0)
  AMBER_INSTALL_DIR Install directory (default: ~/.amber/bin)
  AMBER_REPO        GitHub repo (default: zh30/amberjs)

The installer tries amber-<tag>-<target>.tar.gz first, then the
legacy bee-<tag>-<target>.tar.gz asset from older releases. The
archive may contain a binary named amber or bee; it is always
installed as $AMBER_INSTALL_DIR/amber.

Examples:
  AMBER_VERSION=v1.16.0 sh install.sh
  AMBER_INSTALL_DIR=~/.local/bin sh install.sh
USAGE
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

fail() {
  echo "amber install: $1" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

if need_cmd curl; then
  http_get() { curl -fsSL "$1"; }
  http_download() { curl -fsSL "$1" -o "$2"; }
  http_try_download() {
    code=$(curl -sSL -o "$2" -w "%{http_code}" "$1") || return 1
    [ "$code" = "200" ]
  }
elif need_cmd wget; then
  http_get() { wget -qO- "$1"; }
  http_download() { wget -qO "$2" "$1"; }
  http_try_download() { wget -qO "$2" "$1"; }
else
  fail "curl or wget is required"
fi

resolve_platform() {
  raw_os="${AMBER_UNAME_S:-$(uname -s)}"
  raw_arch="${AMBER_UNAME_M:-$(uname -m)}"

  case "$raw_os" in
    Darwin) os="apple-darwin" ;;
    Linux) os="unknown-linux-gnu" ;;
    *) fail "unsupported OS: $raw_os" ;;
  esac

  case "$raw_arch" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *) fail "unsupported architecture: $raw_arch" ;;
  esac

  echo "${arch}-${os}"
}

if [ "${1:-}" = "--print-platform" ]; then
  resolve_platform
  exit 0
fi

release_asset_url() {
  version_tag="$1"
  filename="$2"
  if [ -n "${AMBER_RELEASE_BASE:-}" ]; then
    echo "${AMBER_RELEASE_BASE%/}/${version_tag}/${filename}"
  else
    echo "https://github.com/${AMBER_REPO}/releases/download/${version_tag}/${filename}"
  fi
}

candidate_archive_urls() {
  version_tag="$1"
  target="$2"
  release_asset_url "$version_tag" "amber-${version_tag}-${target}.tar.gz"
  release_asset_url "$version_tag" "bee-${version_tag}-${target}.tar.gz"
}

resolve_version() {
  if [ -n "${AMBER_VERSION:-}" ]; then
    version="${AMBER_VERSION}"
  else
    api_url="https://api.github.com/repos/${AMBER_REPO}/releases/latest"
    json=$(http_get "$api_url") || fail "unable to fetch latest release"
    version=$(printf "%s" "$json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)
    [ -n "$version" ] || fail "unable to resolve latest version"
  fi

  case "$version" in
    v*) echo "$version" ;;
    *) echo "v$version" ;;
  esac
}

if [ "${1:-}" = "--print-asset-urls" ]; then
  print_target=$(resolve_platform)
  print_tag=$(resolve_version)
  candidate_archive_urls "$print_tag" "$print_target"
  exit 0
fi

find_extracted_binary() {
  root="$1"
  if [ -f "$root/amber" ]; then
    echo "$root/amber"
    return 0
  fi
  if [ -f "$root/bee" ]; then
    echo "$root/bee"
    return 0
  fi
  find "$root" -type f \( -name amber -o -name bee \) | head -n 1
}

install_binary() {
  target="$1"
  version_tag="$2"

  tmpdir=$(mktemp -d 2>/dev/null || mktemp -d -t amber)
  archive="$tmpdir/amber.tar.gz"

  trap 'rm -rf "$tmpdir"' EXIT INT TERM

  amber_url=$(release_asset_url "$version_tag" "amber-${version_tag}-${target}.tar.gz")
  bee_url=$(release_asset_url "$version_tag" "bee-${version_tag}-${target}.tar.gz")
  tried="${amber_url} ${bee_url}"
  downloaded=""

  # Prefer amber- assets; fall back to bee- leftovers from v1.16.0 and earlier.
  echo "Downloading ${amber_url}"
  if http_try_download "$amber_url" "$archive"; then
    downloaded="$amber_url"
  else
    echo "Downloading ${bee_url}"
    if http_try_download "$bee_url" "$archive"; then
      downloaded="$bee_url"
    fi
  fi

  [ -n "$downloaded" ] || fail "download failed (tried: ${tried})"

  tar -xzf "$archive" -C "$tmpdir" || fail "failed to extract archive"

  src=$(find_extracted_binary "$tmpdir")
  [ -n "${src:-}" ] || fail "amber or bee binary not found in archive"

  mkdir -p "$AMBER_INSTALL_DIR"
  cp "$src" "$AMBER_INSTALL_DIR/amber"
  chmod +x "$AMBER_INSTALL_DIR/amber"
}

ensure_path() {
  install_dir="$1"

  case ":$PATH:" in
    *":$install_dir:"*) return 0 ;;
  esac

  shell_name=$(basename "${SHELL:-sh}")
  profile=""

  case "$shell_name" in
    zsh) profile="$HOME/.zshrc" ;;
    bash)
      if [ -f "$HOME/.bashrc" ]; then
        profile="$HOME/.bashrc"
      elif [ -f "$HOME/.bash_profile" ]; then
        profile="$HOME/.bash_profile"
      else
        profile="$HOME/.bashrc"
      fi
      ;;
    fish) profile="$HOME/.config/fish/config.fish" ;;
    *) profile="$HOME/.profile" ;;
  esac

  if [ -f "$profile" ] && grep -qs "$install_dir" "$profile"; then
    return 0
  fi

  mkdir -p "$(dirname "$profile")" 2>/dev/null || true

  if [ "$shell_name" = "fish" ]; then
    line="set -gx PATH \"$install_dir\" \$PATH"
  else
    line="export PATH=\"$install_dir:\$PATH\""
  fi

  printf "\n# Amber\n%s\n" "$line" >> "$profile"
}

main() {
  target=$(resolve_platform)
  version_tag=$(resolve_version)

  install_binary "$target" "$version_tag"
  ensure_path "$AMBER_INSTALL_DIR"

  echo "Amber ${version_tag} installed to ${AMBER_INSTALL_DIR}/amber"
  echo "Restart your shell or run:"
  echo "  export PATH=\"${AMBER_INSTALL_DIR}:\$PATH\""
  echo "Verify with: amber --version"
}

main
