#!/usr/bin/env python3
"""Finish the web player: dist/web/index.html (loads kartu.js + carts/) and
dist/kartu-player.html (one self-contained file: wasm + cart inlined, to share or embed).

The native hash is computed here by the x86 runner, so the page's Verify button checks
the browser against a real native run, not a number typed in by hand."""
import base64, os, re, subprocess, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
os.chdir(ROOT)
out = subprocess.run(["target/release/kartu", "run", "carts/bench", "--frames", "600",
                      "--press", "200:a,201:,400:right,450:"], capture_output=True, text=True, check=True).stdout
native = re.search(r"hash ([0-9a-f]{16})", out).group(1)
page = open("web/index.html").read().replace("__NATIVE_HASH__", native)
js = open("dist/web/kartu.js").read()
assert "</script" not in js

# Cart list for the web player's picker: every dist/web/carts/<name>/ (sync-web-carts.sh copies
# them), newest first, bench last. Title = a `-- title: ...` line in main.lua, else the dir name.
carts = []
for n in os.listdir("dist/web/carts"):
    d = os.path.join("dist/web/carts", n)
    if not all(os.path.isfile(os.path.join(d, f)) for f in ("main.lua", "assets.cw")):
        continue
    m = re.search(r"^--\s*title:\s*(.+)$", open(os.path.join(d, "main.lua")).read(), re.M)
    carts.append({"name": n, "title": m.group(1).strip() if m else n,
                  "t": max(os.path.getmtime(os.path.join(d, f)) for f in ("main.lua", "assets.cw"))})
carts.sort(key=lambda c: (c["name"] == "bench", -c["t"]))
os.makedirs("dist/web/carts", exist_ok=True)
json_list = [{"name": c["name"], "title": c["title"]} for c in carts]
open("dist/web/carts/index.json", "w").write(__import__("json").dumps(json_list, indent=1))

open("dist/web/index.html", "w").write(page.replace("<script>__PLAYER_JS__</script>", '<script src="kartu.js"></script>'))

import json
cart = {"assets": open("carts/bench/assets.cw").read(), "main": open("carts/bench/main.lua").read()}
wasm = base64.b64encode(open("dist/web/kartu.wasm", "rb").read()).decode()
inline = "window.CW_INLINE = " + json.dumps({"wasm": wasm, "cart": cart}).replace("</", "<\\/") + ";\n" + js
open("dist/kartu-player.html", "w").write(page.replace("__PLAYER_JS__", inline))
print(f"native hash {native}; dist/web/index.html + dist/kartu-player.html ({os.path.getsize('dist/kartu-player.html')//1024} KB)")
