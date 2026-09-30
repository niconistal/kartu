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

# Allium: the same player and carts as Apps/Kartu.pak, plus a Games-list folder Roms/Kartu where
# each cart is a <Title>.port folder. Allium runs .port folders with its built-in "Native" core, so
# nothing in Allium's own config has to change. Box art is the cart's own frame 90.
al=dist/allium
rm -rf $al && mkdir -p $al/Apps $al/Roms/Kartu/Imgs
cp -r $app $al/Apps/Kartu.pak
rm -f $al/Apps/Kartu.pak/launch.sh
cp miyoo/allium/launch.sh miyoo/allium/play.sh $al/Apps/Kartu.pak/
cw=target/release/kartu; [ -x $cw ] || cw=kartu
while read -r c; do
  n=$(basename "$c"); [ "$n" = bench ] && continue
  t=$(sed -n '1s/^-- title: *//p' "$c"/main.lua | sed 's/ *([^)]*) *$//; s#[/:]# #g')
  t=${t:-$n}
  mkdir -p "$al/Roms/Kartu/$t.port"
  printf '#!/bin/sh\nexec /mnt/SDCARD/Apps/Kartu.pak/play.sh %s\n' "$n" > "$al/Roms/Kartu/$t.port/launch.sh"
  chmod +x "$al/Roms/Kartu/$t.port/launch.sh"
  $cw run "$c" --frames 90 --shot "$al/Roms/Kartu/Imgs/$t.png" --scale 1 >/dev/null 2>&1 ||
    echo "no box art for $n"
done < dist/miyoo/carts.txt
ls $al/Roms/Kartu
file $app/kartu; ls -la $app
