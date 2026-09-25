#!/bin/bash
# Release tarball for install.sh: dist/pkg/kartu-linux-x86_64.tar.gz
#   bin/kartu bin/kartu-mcp (static musl) · share/kartu/web (browser player) ·
#   share/kartu/library (starter carts = examples + art to reuse) · licences
# Needs the web player built once (scripts/build-web.sh) and cargo-zigbuild.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH=$HOME/.cargo/bin:$PATH
TARGET=${TARGET:-x86_64-unknown-linux-musl}
NAME=${NAME:-kartu-linux-x86_64}
STARTERS=${STARTERS:-"crypt dungeon bakehouse froggy keyquest jukebox shanty"}
[ -f dist/web/kartu.wasm ] || { echo "no dist/web: run scripts/build-web.sh first" >&2; exit 1; }
cargo zigbuild --release --target "$TARGET" -p kartu -p kartu-maker
out=dist/pkg/$NAME
rm -rf "$out" && mkdir -p "$out/bin" "$out/share/kartu/web" "$out/share/kartu/library"
cp target/$TARGET/release/kartu target/$TARGET/release/kartu-mcp "$out/bin/"
cp dist/web/index.html dist/web/kartu.js dist/web/kartu.wasm "$out/share/kartu/web/"
for c in $STARTERS; do
  mkdir -p "$out/share/kartu/library/$c"
  cp carts/$c/main.lua carts/$c/assets.cw "$out/share/kartu/library/$c/"
  for f in bot.txt win.txt; do [ -f carts/$c/$f ] && cp carts/$c/$f "$out/share/kartu/library/$c/"; done
done
cp LICENSE "$out/LICENSE"; cp maker/LICENSE "$out/LICENSE-AGPL"; cp README.md "$out/"
"$out/bin/kartu" version > "$out/VERSION"
tar -C dist/pkg -czf "dist/pkg/$NAME.tar.gz" "$NAME"
ls -la "dist/pkg/$NAME.tar.gz"
