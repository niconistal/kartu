# Building

## Native

Rust 1.85 or newer. The only C is Lua, which `mlua` builds for you.

```sh
cargo build --release              # target/release/kartu, kartu-mcp
cargo test -p kartu-core
./target/release/kartu bench carts/bench
scripts/verify-carts.sh            # check + a proven win for every cart (what CI runs)
```

## The web player

The player is the same `kartu` binary compiled for `wasm32-unknown-emscripten`, plus
`web/index.html`. You need [emsdk](https://emscripten.org/) (the scripts expect it at
`/opt/emsdk`; edit the path at the top of `scripts/build-web.sh` if yours differs).

```sh
scripts/build-web.sh          # dist/web/ (kartu.js + kartu.wasm + index.html + carts/)
                              # and dist/kartu-player.html (one self-contained file to share or embed)
scripts/sync-web-carts.sh     # re-bundle carts only (no wasm rebuild)
```

`bundle_web.py` computes the native reference hash with the x86 runner, so the page's
**Verify** button checks the browser against a real native run rather than a number typed in
by hand. Carts in `dist/web/carts/` come from `scripts/cart-dirs.sh`: `carts/*` first, then
any carts in your config's `library` dirs.

`kartu web <cart>` serves the player from `~/.local/share/kartu/web` with that cart.

## Handheld

zig 0.16 + `cargo-zigbuild` as the static cross-linker; `qemu-arm` to run the result locally.

```sh
scripts/build-miyoo.sh        # dist/miyoo/Kartu/ (an Onion OS App folder)
scripts/deliver-miyoo.sh      # install over ssh (DEVICE_IP or [device.miyoo] ip in config)
```

See [Handhelds](handhelds.md).

## Packaging a release

```sh
scripts/package.sh            # dist/pkg/kartu-linux-x86_64.tar.gz (static musl) for install.sh
gh release create vX.Y.Z dist/pkg/*.tar.gz
```

Bump the versions in the three `Cargo.toml` files and `core/CHANGELOG.md` first. `install.sh`
fetches `releases/latest`.

## Gotchas we hit so you don't have to

- **emscripten + mlua:** Lua is compiled **as C++** (errors are exceptions). Rust links with
  `-fwasm-exceptions`, so Lua must too (`CXXFLAGS_wasm32_unknown_emscripten`), plus
  `-lc++ -lc++abi`. After changing those flags, `cargo clean -p mlua-sys --target …`: its build
  script caches.
- **Single-file player:** needs `-sINCOMING_MODULE_JS_API=wasmBinary`, or emscripten ignores the
  inlined bytes and tries to fetch `kartu.wasm`.
- **fbdev:** with two pages, `FBIOPAN_DISPLAY` is the flip; `FBIO_WAITFORVSYNC` on top halves
  the frame rate, so it's only used single-buffered (`CW_VSYNC=1` forces it).
- **fbdev without a screen:** `modprobe vfb vfb_enable=1 videomemorysize=4000000`,
  `fbset -depth 32 -vyres 960`; fbcon grabs it, so `echo 0 > /sys/class/vtconsole/vtcon1/bind`
  before `rmmod vfb`.
- **Determinism:** never use `f32::sin` or the like in the core; use the console's own
  polynomials. `verify-carts.sh` compares frame hashes, so a non-deterministic change shows up
  as a failing replay.

## The website

`site/` holds the landing page and the docs renderer (Node, `marked` + `highlight.js`).
`scripts/build-site.sh` renders `docs/*.md` and `API.md`, copies the web player from `dist/web`
into `play/`, and writes `dist/site/`. `scripts/deploy-site.sh` pushes that folder to the
`gh-pages` branch, which GitHub Pages serves at <https://niconistal.github.io/kartu/>.
