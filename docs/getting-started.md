# Getting started

## Install

**Linux x86_64** (prebuilt):

```sh
curl -fsSL https://raw.githubusercontent.com/niconistal/kartu/main/install.sh | sh
```

The installer puts `kartu` and `kartu-mcp` in `~/.local/bin`, and the shipped defaults (the
browser player, the starter library) in `~/.local/share/kartu`. Re-run it to update. It never
touches your settings in `~/.config/kartu/config.toml`. If Claude Code is on your PATH it also
runs `kartu setup` (see [AI tools](ai-tools.md)).

**Everywhere else** (macOS, Windows, ARM Linux), build from source with Rust 1.85 or newer:

```sh
cargo install --git https://github.com/niconistal/kartu kartu kartu-maker
```

This gives you the runner and the maker commands. The browser player (`kartu web`) needs the
WebAssembly build in `~/.local/share/kartu/web`; until there are prebuilt packages for your
platform, either copy `dist/web` from a Linux install or [build it](building.md).

Check the install:

```
$ kartu doctor
```

## Your first cart

```sh
kartu new hello --template blank
cd hello
```

A cart is a folder with two text files. `assets.cw` holds the art:

```text
palette p
  . clear
  w #f4f4f4
  y #ffcd75
sprite star 8x8 pal=p
...yy...
...yy...
yyyyyyyy
.yyyyyy.
..yyyy..
.yy..yy.
yy....yy
........
```

and `main.lua` holds the game. Three functions run it: `init` once, then `update` and `draw`
every frame, sixty times a second.

```lua
x = 156
function update()
  if btn("left")  then x = x - 2 end
  if btn("right") then x = x + 2 end
end
function draw()
  text("HELLO, KARTU", 112, 40, "w")
  spr("star", x, 116)
end
```

Then:

```sh
kartu check .        # every problem at once: art, Lua syntax, unknown names, a smoke run
kartu web .          # play it in the browser (http://localhost:7070, or the next free port) — reload after edits
```

Keyboard in the web player: arrows move, **Z** = A, **X** = B, **A** = Y, **S** = X, **Q/W** = L/R,
Enter = Start, Shift = Select.

The default template is `topdown`: a small, working, winnable game built on the
[top-down kit](kits.md), which is the better starting point for an adventure or a collect-a-thon.

## The commands

| | |
|---|---|
| `kartu new <folder>` | a working starter cart + `AGENTS.md` + `.mcp.json` (`--title T`, `--template topdown|blank`) |
| `kartu check <cart>` | every problem at once: art, Lua syntax, unknown names, a smoke run. Exit 1 on any error |
| `kartu run <cart> …` | play headless: log lines, watched values, screenshots, bots, saves |
| `kartu playtest <cart>` | the goal bot plays `bot.txt` on seeds 1–3; PASS = provably winnable and fair |
| `kartu web <cart>` | the browser player on localhost (`--port N`) |
| `kartu play <cart>` | the Linux framebuffer player (handhelds) |
| `kartu sound <cart>` | render one song or effect to WAV, or list what the cart can play |
| `kartu art import pic.png --name x` | any PNG → palette + sprites (bring your own image generator) |
| `kartu docs [topic]` | the API reference, by topic |
| `kartu mcp` | the maker tools as an MCP server |
| `kartu setup` / `doctor` / `config` | connect Claude Code · check the install · show your settings |
| `kartu help` | all of the above with every flag |

The runner's flags are explained in [The runner](runner.md).

## Where things live

| | |
|---|---|
| `~/.local/share/kartu/` | shipped defaults: the web player, the starter library. Replaced on every install — don't edit |
| `~/.config/kartu/config.toml` | yours; never touched by updates (below) |
| `<cart>/kartu.toml` | the game's title and format version; travels with the cart |
| `<cart>/.cw/` | the runner's scratch space for that cart (screenshots, playtest output). Safe to delete, ignored by git |

`config.toml` is optional:

```toml
library = ["~/kartu-art"]         # more carts or packs whose art the AI can search and copy
judge = "~/bin/my-judge"          # optional: an opinion on playtests (see AI tools → the judge hook)
[device.miyoo]
ip = "192.168.1.50"               # scripts/deliver-miyoo.sh
```

`KARTU_CONFIG` overrides the path; `XDG_CONFIG_HOME` and `XDG_DATA_HOME` are respected.

## Next

- [Making a game](making-a-game.md) — the loop that makes carts good, with an AI or by hand
- [Cart format](cart-format.md) — everything `assets.cw` can say
- [API.md](../API.md) — the reference
