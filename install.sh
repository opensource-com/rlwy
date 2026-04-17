#!/usr/bin/env bash
# rlwy installer for Linux and macOS.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/rlwy-dev/rlwy/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/rlwy-dev/rlwy/main/install.sh | RLWY_VERSION=v0.1.0 bash
#
# Env:
#   RLWY_VERSION      tag to install (default: latest)
#   RLWY_INSTALL_DIR  install directory (default: $HOME/.local/bin)
#   RLWY_REPO         override repo (default: rlwy-dev/rlwy)
set -euo pipefail

REPO="${RLWY_REPO:-rlwy-dev/rlwy}"
VERSION="${RLWY_VERSION:-latest}"
INSTALL_DIR="${RLWY_INSTALL_DIR:-$HOME/.local/bin}"

msg()  { printf '\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m==>\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

if command -v curl >/dev/null 2>&1; then
  dl() { curl -fsSL "$1" -o "$2"; }
  head_location() { curl -fsSLI "$1" | tr -d '\r' | awk 'tolower($1)=="location:"{print $2}' | tail -n1; }
elif command -v wget >/dev/null 2>&1; then
  dl() { wget -qO "$2" "$1"; }
  head_location() { wget -qS --max-redirect=0 "$1" 2>&1 | awk '/^[[:space:]]*Location:/{print $2}' | tail -n1; }
else
  die "need curl or wget"
fi

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Linux)
    case "$arch" in
      x86_64|amd64)   triple="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64)  triple="aarch64-unknown-linux-gnu" ;;
      *) die "unsupported linux arch: $arch" ;;
    esac ;;
  Darwin)
    case "$arch" in
      x86_64)         triple="x86_64-apple-darwin" ;;
      arm64|aarch64)  triple="aarch64-apple-darwin" ;;
      *) die "unsupported macos arch: $arch" ;;
    esac ;;
  *) die "unsupported OS: $os (Windows users: use install.ps1)" ;;
esac

if [ "$VERSION" = "latest" ]; then
  loc="$(head_location "https://github.com/${REPO}/releases/latest" || true)"
  VERSION="${loc##*/}"
  [ -n "$VERSION" ] || die "could not resolve latest release for ${REPO}"
fi

ver="${VERSION#v}"
asset="rlwy-v${ver}-${triple}"
url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"

msg "installing rlwy ${VERSION} (${triple})"
msg "↓ ${url}"

tmp="$(mktemp -t rlwy.XXXXXX)"
trap 'rm -f "$tmp"' EXIT
dl "$url" "$tmp" || die "download failed: $url"
[ -s "$tmp" ] || die "downloaded file is empty"

mkdir -p "$INSTALL_DIR"
chmod +x "$tmp"
mv "$tmp" "$INSTALL_DIR/rlwy"
trap - EXIT

msg "installed → $INSTALL_DIR/rlwy"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    warn "$INSTALL_DIR is not on your PATH. Add it:"
    warn "  bash/zsh: export PATH=\"$INSTALL_DIR:\$PATH\""
    warn "  fish:     fish_add_path $INSTALL_DIR"
    ;;
esac

"$INSTALL_DIR/rlwy" --version 2>/dev/null || true
msg "done. try: rlwy --help"
