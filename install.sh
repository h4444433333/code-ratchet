#!/usr/bin/env bash
# code-ratchet — one-line installer.
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/h4444433333/code-ratchet/main/install.sh | bash
#   curl -sSf https://raw.githubusercontent.com/h4444433333/code-ratchet/main/install.sh | bash -s -- --no-setup
#
# What it does:
#   1. Detects OS + arch (darwin/linux × arm64/x86_64).
#   2. Downloads the matching binary from the latest GitHub release.
#   3. Installs to ~/.local/bin (creates if missing); falls back to sudo
#      /usr/local/bin if ~/.local/bin is not on PATH.
#   4. Runs `code-ratchet setup` in the current directory to wire up the
#      project — unless --no-setup is passed.

set -euo pipefail

REPO="${CODE_RATCHET_REPO:-h4444433333/code-ratchet}"
VERSION="${CODE_RATCHET_VERSION:-latest}"

RUN_SETUP=1
for arg in "$@"; do
  case "$arg" in
    --no-setup) RUN_SETUP=0 ;;
    -h|--help)
      cat <<EOF
Usage: install.sh [--no-setup]
  --no-setup   Install the binary only; do not run setup in the cwd.
Environment:
  CODE_RATCHET_REPO     GitHub repo (default: h4444433333/code-ratchet)
  CODE_RATCHET_VERSION  Release tag (default: latest)
EOF
      exit 0
      ;;
  esac
done

# --- Detect platform ---
os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch_raw="$(uname -m)"
case "$arch_raw" in
  x86_64|amd64)  arch="x86_64"  ;;
  arm64|aarch64) arch="arm64"   ;;
  *) echo "unsupported arch: $arch_raw" >&2; exit 1 ;;
esac
case "$os" in
  darwin|linux) ;;
  *) echo "unsupported OS: $os (only darwin/linux supported in v0.3)" >&2; exit 1 ;;
esac

asset="code-ratchet-${os}-${arch}"

# --- Resolve download URL ---
if [ "$VERSION" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

# --- Pick install dir ---
if echo ":${PATH}:" | grep -q ":${HOME}/.local/bin:"; then
  install_dir="${HOME}/.local/bin"
  use_sudo=0
elif [ -w /usr/local/bin ]; then
  install_dir="/usr/local/bin"
  use_sudo=0
else
  install_dir="/usr/local/bin"
  use_sudo=1
fi
mkdir -p "$install_dir" 2>/dev/null || true

dest="${install_dir}/code-ratchet"

# --- Download ---
echo "→ downloading code-ratchet (${os}-${arch}) from ${url}"
tmp="$(mktemp)"
if command -v curl >/dev/null 2>&1; then
  curl -sSfL "$url" -o "$tmp"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$url" -O "$tmp"
else
  echo "need curl or wget" >&2
  exit 1
fi
chmod +x "$tmp"

if [ "$use_sudo" -eq 1 ]; then
  echo "→ installing to ${dest} (sudo)"
  sudo mv "$tmp" "$dest"
else
  echo "→ installing to ${dest}"
  mv "$tmp" "$dest"
fi

# --- PATH hint ---
if ! command -v code-ratchet >/dev/null 2>&1; then
  echo
  echo "code-ratchet is installed at ${dest} but not on PATH."
  echo "Add this to your shell rc and reopen:"
  echo "  export PATH=\"${install_dir}:\$PATH\""
  echo
fi

echo "✓ installed: $("$dest" --version)"

# --- Auto-setup ---
if [ "$RUN_SETUP" -eq 1 ]; then
  echo
  echo "→ running \`code-ratchet setup\` in $(pwd)"
  echo
  "$dest" setup -y || true
fi
