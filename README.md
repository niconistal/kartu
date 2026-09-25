# Kartu

**A fantasy console made for AI.** Describe a game to your AI; it builds a small retro game
(320×240, Lua + text-native art and sound), plays it with a bot to prove it can be won, and you
play it in the browser or on a handheld (Miyoo Mini, Anbernic).

```sh
curl -fsSL https://raw.githubusercontent.com/niconistal/kartu/main/install.sh | sh
kartu new frog-quest && cd frog-quest
claude                      # "make this a game where a frog hops across lily pads to eat 5 flies"
kartu web .                 # play it
```

That's all. `kartu new` gives you a small working game plus everything an AI needs to change it:
`AGENTS.md` (how to make a Kartu game), `CLAUDE.md`, and `.mcp.json` (the maker tools). Any agent with
a shell can drive the `kartu` commands; Claude Code gets the MCP tools and a skill (`kartu setup`).

## Commands

| | |
|---|---|
| `kartu new <folder>` | a working starter cart + AGENTS.md + .mcp.json |
| `kartu check <cart>` | every problem at once: art, Lua syntax, unknown names, a smoke run |
| `kartu run <cart> …` | headless: log lines, watched values, screenshots, bots, saves |
| `kartu playtest <cart>` | the goal bot plays `bot.txt` on seeds 1-3; PASS = provably winnable |
| `kartu web <cart>` | the browser player on localhost (reload after edits) |
| `kartu play <cart>` | Linux framebuffer player (handhelds) |
| `kartu art import pic.png --name x` | any PNG → palette + sprites (bring your own image AI) |
| `kartu docs [topic]` | the API reference ([API.md](API.md)) |
| `kartu mcp` | the maker tools as an MCP server |
| `kartu setup` / `doctor` / `config` | connect Claude Code · check the install · your settings |

## Where things live

| | |
|---|---|
| `~/.local/share/kartu/` | shipped defaults: browser player, starter library. Replaced on every install — don't edit |
| `~/.config/kartu/config.toml` | yours, never touched by updates (below) |
| `<cart>/kartu.toml` | the game's title and format version; travels with the cart |

```toml
library = ["~/kartu-art"]         # more carts/packs whose art the AI can search and copy
judge = "~/bin/my-judge"          # optional: reads {cart, facts, trace} JSON, prints opinions (scripts/judge.py)
[device.miyoo]
ip = "192.168.1.50"               # scripts/deliver-miyoo.sh
```

## Repo

| path | what |
|---|---|
| `core/` | `kartu-core` (MIT): picture unit, text-native assets, synth, Lua VM + console API, kits |
| `player/` | `kartu` (MIT): runner, checker, goal bot, art import, fbdev + OSS player; the web player on emscripten |
| `maker/` | `kartu-maker` (AGPL-3.0): `kartu-mcp`, the MCP server + `new/setup/web/…`, templates, the AGENTS.md skill |
| `carts/` | example carts (also the starter art library) |
| `web/` | browser player page · `miyoo/`: Onion OS app + Games-tab console |
| `scripts/` | builds, packaging, `verify-carts.sh`, `judge.py` |

Licences: MIT for the console, player and carts; AGPL-3.0 for the maker (`maker/LICENSE`).

## The console

- 320×240, CRAM of 512 colours (16 BG + 16 sprite palettes × 16), 15-bit master colours.
- 4 tile layers (16×16 tiles, wrap + per-layer scroll), 128 sprites (≤64 px, flip, palette
  override, per-sprite layer), 8×8 text.
- Lua 5.4 sandbox: no io/os/load/require, 32 MB memory cap, instruction budget per frame
  (a runaway loop becomes a readable on-screen error, not a hang).
- Determinism: seeded `rnd`/`irnd`, no clock, `sin`/`cos` in turns from a polynomial
  (the C libm differs across targets). The framebuffer holds CRAM indices, so `hash()` is
  target-independent.
- API: `init/update/draw`, `spr(name|clip, x, y, {fx,fy,pal,layer})`, `bg.map/scroll/show/tile`,
  `text`, `backdrop`, `pal`, `btn/btnp`, `rnd/irnd`, `frame`, `sin/cos`, `log`.

Not yet: per-line scroll, 8×8 tiles, autotiles, `.cart.png`.

## Sound (P1.4)

`core/src/audio.rs`: 8 voices synthesised in the core, 24 kHz mono, 400 samples per frame; plain
IEEE ops + own exp2/sin polynomials so the samples (and the `audio` hash) match on x86, wasm and armv7.
Waves square/triangle/saw/sine/noise/pluck (Karplus-Strong)/2-op FM, ADSR, vibrato, slide, lowpass,
detune, DC blocker + soft knee. assets.cw keywords `instrument`, `sfx`, `song` (MML channels, `@drums`
letters). 29 instruments, 21 sfx and 5 songs built in. Lua `sfx music play music_playing has(n,kind)`;
the top-down kit plays event sounds and `music=` per state/room. Runner `--sounds --wav`, `kartu
sound`. Web player: ScriptProcessor fed from a ring (≤150 ms), 🔊 toggle, starts on first input.
**Handheld (fbdev) player:** OSS `/dev/dsp` (`player/src/oss.rs`, `--audio DEV`, `--mute`); on the Miyoo, launch.sh frees the device from Onion's audioserver first. `carts/jukebox` plays everything.

## Build

Toolchains: rustup, zig 0.16 + `cargo-zigbuild` (static Linux + ARM cross-link), emsdk at
`/opt/emsdk` (web player), `qemu-arm` to run the ARM binary locally.

```sh
cargo test -p kartu-core
cargo build --release && ./target/release/kartu bench carts/bench
./target/release/kartu run carts/bench --frames 600 --press 200:a,201: --shot /tmp/x.png
scripts/build-web.sh      # dist/web/ + dist/kartu-player.html (single file, wasm inlined)
scripts/build-miyoo.sh    # dist/miyoo/Kartu/ (Onion App folder)
scripts/deliver-miyoo.sh  # install on a Miyoo over ssh (DEVICE_IP or [device.miyoo] ip in config)
scripts/package.sh        # dist/pkg/kartu-linux-x86_64.tar.gz for install.sh
scripts/verify-carts.sh   # regression: check + a proven win for every cart
```

Gotchas found on the way:
- emscripten: mlua builds Lua **as C++** (errors are exceptions). Rust links with
  `-fwasm-exceptions`, so Lua must too (`CXXFLAGS_wasm32_unknown_emscripten`), plus
  `-lc++ -lc++abi`. After changing those flags, `cargo clean -p mlua-sys --target …`: its
  build script caches.
- The single-file player needs `-sINCOMING_MODULE_JS_API=wasmBinary` or emscripten
  ignores the inlined bytes and tries to fetch `kartu.wasm`.
- fbdev: with two pages, `FBIOPAN_DISPLAY` is the flip; `FBIO_WAITFORVSYNC` on top would
  halve the frame rate, so it's only used single-buffered (`CW_VSYNC=1` forces it).
- Testing fbdev without a screen: `modprobe vfb vfb_enable=1 videomemorysize=4000000`,
  `fbset -depth 32 -vyres 960`. fbcon grabs it: `echo 0 > /sys/class/vtconsole/vtcon1/bind`
  before `rmmod vfb`.

## Miyoo Mini

Onion App at `/mnt/SDCARD/App/Kartu` (Apps → Kartu), plus a **Kartu console in the Games tab**:
`Emu/KARTU` (`miyoo/emu`) + `Roms/KARTU/<Title>.kartu` (one line: the cart folder), so each cart
lists like a game (recents, favourites). Both use the player + carts in the App folder. Opens a menu of every cart
(`miyoo/menu`, list generated by `build-miyoo.sh` from each `-- title:` line): A plays,
MENU in a game goes back to the menu, MENU in the menu quits. The menu's `log("PICK dir")`
+ `kartu play --pick-out F` hand the choice to `launch.sh`. In game: L2 perf HUD · R2 flips
the picture (panel is mounted 180°; `CW_ROTATE=0|180` overrides). The last entry is the P0
bench (20 s gate + stress, writes `bench.txt`, then keeps playing).
Sound: `kartu` is static, so Onion's `libpadsp.so` preload can't route it through `audioserver`;
launch.sh runs Onion's `stop_audioserver.sh` (keeps the volume) and the player writes 24 kHz mono
straight to `/dev/dsp` (the SigmaStar driver takes it as is). Onion restarts audioserver before MainUI.
The driver returns its own error codes (0xA005xxxx) from write(), hence raw `libc::write` +
an error counter in the exit report; `KARTU_OSS_DEBUG=N` logs the first N writes. `kartu.log`
shows the audio line and dropped/error counts.

## Maker tools (MCP)

`kartu-mcp` (crate `maker/`, AGPL) speaks MCP over stdio; `kartu mcp …` runs it.

`kartu new` writes `.mcp.json` with `kartu mcp --cart .` (the folder is the cart); `kartu mcp --root DIR`
serves a folder of carts instead. The art library is your `library` dirs + the starter library, or
`--library DIR` (repeatable).

One workspace per cart: `<root>/<cart>/` (main.lua, assets.cw, bot.txt, notes) + a private
`.cw/` scratch dir for shots and runner output. `--cart NAME` pins a server to one cart (the
a hosted maker runs one server per cart). `--log FILE` appends one JSON line
per tool call. Tools shell out to the `kartu` runner next to the binary (90 s cap per run).

| tool | what |
|---|---|
| `docs` | API.md by section (`kits`, `bots`, …), `kit_source`, `all` |
| `cart_new` | from `maker/templates/topdown` (a working 1-room game + bot.txt) or blank |
| `cart_list` / `cart_read` / `cart_write` / `cart_patch` | files; writes of main.lua/assets.cw return `check` at once; patch = exact-once replacements, all or nothing |
| `check` | the runner's check |
| `run` | logs, watches, hash + up to 6 screenshots as images; `bot=true` |
| `state` | `--dump` JSON at frames |
| `playtest` | bot.txt on seeds 1-3 until WIN, fairness (early hits), trace + end shot; PASS/FAIL |
| `asset_search` / `asset_copy` | library = `--library` dirs (carts or packs); copy pulls palettes, `from=`, clip frames, map tiles; name clashes are renamed (`dungeon_main`, `key_dungeon`) |
