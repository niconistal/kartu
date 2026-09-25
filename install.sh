#!/bin/sh
# Kartu installer:  curl -fsSL https://raw.githubusercontent.com/niconistal/kartu/main/install.sh | sh
#
# Puts kartu + kartu-mcp in ~/.local/bin and the shipped defaults (browser player, starter
# library) in ~/.local/share/kartu, which every install replaces. Your settings live in
# ~/.config/kartu/config.toml and are never touched. Re-run it to update.
# KARTU_URL overrides where the release tarball comes from (a URL or a local .tar.gz).
set -eu
os=$(uname -s | tr A-Z a-z); arch=$(uname -m)
case "$os-$arch" in
  linux-x86_64|linux-amd64) pkg=kartu-linux-x86_64 ;;
  *) echo "kartu: no prebuilt binary for $os-$arch yet; build from source: cargo install --git https://github.com/niconistal/kartu kartu kartu-maker" >&2; exit 1 ;;
esac
url=${KARTU_URL:-https://github.com/niconistal/kartu/releases/latest/download/$pkg.tar.gz}
bin=${KARTU_BIN_DIR:-$HOME/.local/bin}
share=${XDG_DATA_HOME:-$HOME/.local/share}/kartu
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
echo "kartu: fetching $url"
case "$url" in
  http*) curl -fsSL "$url" -o "$tmp/k.tgz" ;;
  *) cp "$url" "$tmp/k.tgz" ;;
esac
tar -xzf "$tmp/k.tgz" -C "$tmp"
mkdir -p "$bin" "$(dirname "$share")"
install -m 755 "$tmp/$pkg/bin/kartu" "$tmp/$pkg/bin/kartu-mcp" "$bin/"
rm -rf "$share" && cp -r "$tmp/$pkg/share/kartu" "$share"
echo "kartu: installed $("$bin/kartu" version) → $bin, defaults → $share"
case ":$PATH:" in *":$bin:"*) ;; *) echo "kartu: add $bin to your PATH:  export PATH=\"$bin:\$PATH\"" ;; esac
if command -v claude >/dev/null 2>&1; then "$bin/kartu" setup; else
  echo "kartu: Claude Code not found — install it (https://claude.com/claude-code), then run: kartu setup"; fi
echo
echo "make a game:  kartu new my-game && cd my-game && claude"
