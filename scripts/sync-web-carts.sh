#!/bin/bash
# Copy every cart (scripts/cart-dirs.sh) into dist/web/carts and rewrite the picker list + pages.
# No wasm rebuild: a web player served from dist/web picks new carts up on reload.
set -euo pipefail
cd "$(dirname "$0")/.."
out=dist/web
rm -rf $out/carts && mkdir -p $out/carts
scripts/cart-dirs.sh | while read -r c; do
  n=$(basename "$c"); mkdir -p $out/carts/$n && cp "$c"/assets.cw "$c"/main.lua $out/carts/$n/
done
python3 scripts/bundle_web.py
