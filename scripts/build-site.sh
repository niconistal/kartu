#!/bin/bash
# Build the website into dist/site: landing page + rendered docs (site/build.mjs) + the web
# player with the public carts at play/. Needs node and a web build (scripts/build-web.sh) in
# dist/web. Deploy with scripts/deploy-site.sh.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f dist/web/kartu.wasm ] || { echo "no dist/web/kartu.wasm: run scripts/build-web.sh first" >&2; exit 1; }
(cd site && { [ -d node_modules ] && npm ls marked >/dev/null 2>&1 || npm install --no-audit --no-fund --silent; } && node build.mjs)

# The player, with the carts from this repo only (not the ones in your config's library).
out=dist/site/play
mkdir -p $out/carts
cp dist/web/kartu.js dist/web/kartu.wasm dist/web/index.html $out/
order="froggy dungeon bakehouse keyquest crypt shanty jukebox bench"
for n in $order; do
  [ -f carts/$n/main.lua ] || continue
  mkdir -p $out/carts/$n && cp carts/$n/main.lua carts/$n/assets.cw $out/carts/$n/
done
for c in carts/*/; do n=$(basename $c); [ -d $out/carts/$n ] || { mkdir -p $out/carts/$n && cp $c/main.lua $c/assets.cw $out/carts/$n/; }; done
python3 - "$out" $order <<'EOF'
import json, os, re, sys
out, order = sys.argv[1], sys.argv[2:]
names = order + sorted(n for n in os.listdir(f"{out}/carts") if n not in order)
carts = []
for n in names:
    p = f"{out}/carts/{n}/main.lua"
    if not os.path.isfile(p): continue
    m = re.search(r"^--\s*title:\s*(.+)$", open(p).read(), re.M)
    carts.append({"name": n, "title": m.group(1).strip() if m else n})
json.dump(carts, open(f"{out}/carts/index.json", "w"), indent=1)
print(f"play/: {len(carts)} carts")
EOF
du -sh dist/site | cut -f1 | xargs echo "dist/site:"
