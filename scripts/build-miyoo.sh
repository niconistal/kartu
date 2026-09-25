#!/bin/bash
# Build the static armv7 player and lay out an Onion OS app folder in dist/miyoo/Kartu.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH=$HOME/.cargo/bin:$PATH
cargo zigbuild --release --target armv7-unknown-linux-musleabihf -p kartu
app=dist/miyoo/Kartu
rm -rf dist/miyoo/Kartu dist/miyoo/Emu dist/miyoo/Roms && mkdir -p $app/carts
cp target/armv7-unknown-linux-musleabihf/release/kartu $app/
cp miyoo/launch.sh miyoo/config.json miyoo/icon.png $app/ 2>/dev/null || cp miyoo/launch.sh miyoo/config.json $app/
scripts/cart-dirs.sh > dist/miyoo/carts.txt
while read -r c; do n=$(basename "$c"); mkdir -p $app/carts/$n; cp "$c"/assets.cw "$c"/main.lua $app/carts/$n/; done < dist/miyoo/carts.txt
# The menu cart: every cart's `-- title:` line (36 chars max, fits 320 px), bench last.
mkdir -p $app/menu && cp miyoo/menu/assets.cw $app/menu/
{
  echo "CARTS = {"
  while read -r c; do
    n=$(basename "$c"); [ "$n" = bench ] && continue
    t=$(sed -n '1s/^-- title: *//p' "$c"/main.lua | cut -c1-36 | sed 's/[\\"]/\\&/g')
    echo "  {dir=\"$n\", title=\"${t:-$n}\"},"
  done < dist/miyoo/carts.txt
  echo '  {dir="bench", title="P0 bench (20 s, writes bench.txt)"},'
  echo "}"
  cat miyoo/menu/menu.lua
} > $app/menu/main.lua
# The Games-tab console: Emu/KARTU (launcher) + Roms/KARTU/<Title>.kartu, one per cart (not bench).
# The title is the `-- title:` line without a trailing "(...)" note and without / : characters.
mkdir -p dist/miyoo/Emu/KARTU dist/miyoo/Roms/KARTU
cp miyoo/emu/config.json miyoo/emu/launch.sh dist/miyoo/Emu/KARTU/
cp miyoo/icon.png dist/miyoo/Emu/KARTU/ 2>/dev/null || true
while read -r c; do
  n=$(basename "$c"); [ "$n" = bench ] && continue
  t=$(sed -n '1s/^-- title: *//p' "$c"/main.lua | sed 's/ *([^)]*) *$//; s#[/:]# #g')
  echo "$n" > "dist/miyoo/Roms/KARTU/${t:-$n}.kartu"
done < dist/miyoo/carts.txt
ls dist/miyoo/Roms/KARTU
file $app/kartu; ls -la $app
