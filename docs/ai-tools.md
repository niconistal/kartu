# AI tools

Kartu meets agents at three levels. The lowest is never broken; the higher ones are
conveniences built on it.

1. **The CLI.** `kartu check`, `run`, `playtest` print plain text with stable exit codes. Any
   agent that can run a shell command can make Kartu games.
2. **`AGENTS.md`.** `kartu new` writes a guide into every cart: the workflow, the tools, the rules
   of the console. `CLAUDE.md` contains `@AGENTS.md`, so Claude Code reads it too; other agents
   that look for `AGENTS.md` find it directly.
3. **MCP.** `kartu mcp` serves the same functions over the Model Context Protocol, with
   screenshots returned as images. `kartu setup` connects Claude Code.

## What `kartu new` writes

```text
frog-quest/
├── main.lua      a working one-room game (the topdown template)
├── assets.cw     its art, palettes and sounds
├── bot.txt       the goal bot's plan = proof it can be won
├── kartu.toml    title, format version
├── AGENTS.md     how to make a Kartu game (below)
├── CLAUDE.md     @AGENTS.md
├── .mcp.json     {"mcpServers": {"kartu": {"command": "kartu", "args": ["mcp", "--cart", "."]}}}
└── .gitignore    .cw/
```

Then `cd frog-quest && claude` (or `codex`, `gemini`, an editor agent…) and describe the game.

## `AGENTS.md`

The guide is short and lives in the repo at [`maker/skill/AGENTS.md`](../maker/skill/AGENTS.md).
The heart of it:

1. **Read before writing** — `docs` with no topic lists the topics; `kits` gives a whole game from one table.
2. **Start from the working starter** and change it toward the brief. Small steps, `check` after every edit.
3. **Look at it** — `run` with screenshots at a few frames, and actually look.
4. **Make `log()` lines say what happens** — `got key`, `room 2`, `hurt`, and exactly `WIN`.
5. **Prove it can be won** — update `bot.txt`, `playtest` until it passes on every seed.
6. **You're done when playtest passes and the screenshots match the brief.** Small and polished beats big and broken.

If you improve the guide, everyone's agents get better: it is the single most leveraged file
in the repository.

## `kartu setup`

Writes a Claude Code **skill** (`~/.claude/skills/kartu/SKILL.md`) so that "make me a game"
in any session knows to reach for `kartu new`. With `--mcp` it also registers the MCP server
for every session (`claude mcp add -s user kartu -- kartu mcp --root .`); without it, carts
made by `kartu new` bring their own `.mcp.json`, which is usually enough.

Other agents: point them at the `kartu` binary and `AGENTS.md`. For MCP clients, the server
command is `kartu mcp --cart <dir>` (one cart) or `kartu mcp --root <dir>` (a folder of carts).

## The MCP server

`kartu-mcp` (the `maker/` crate, AGPL-3.0) speaks MCP over stdio; `kartu mcp …` runs it.

```
kartu mcp --cart .                 this folder is the cart
kartu mcp --root DIR               a folder of carts; tools take a `cart` argument
          --library DIR            more art to search (repeatable; adds to config + the starter library)
          --log FILE               append one JSON line per tool call
```

One workspace per cart: the cart folder plus a private `.cw/` scratch dir for shots and runner
output. Tools shell out to the `kartu` runner next to the binary (90 s cap per run).

| tool | what |
|---|---|
| `docs` | the API reference by topic (`kits`, `assets`, `bots`, `sound`, …), `sound_songs` (built-ins as text), `kit_source`, `all` |
| `cart_new` | a new cart from the `topdown` template (a working one-room game + `bot.txt`) or blank |
| `cart_list` / `cart_read` | files |
| `cart_write` / `cart_patch` | write a file, or apply exact-once replacements (all or nothing). Writes of `main.lua` / `assets.cw` return the `check` result at once |
| `check` | the runner's check |
| `run` | frames, seed, script; returns logs, watches, the hash and up to 6 screenshots **as images**; `bot=true` plays the plan |
| `state` | the cart's state as JSON at given frames (`--dump`) |
| `playtest` | `bot.txt` on seeds 1–3 until WIN; fairness; trace and end shot; PASS / FAIL |
| `asset_search` / `asset_copy` | search the art library (your `library` dirs + the starter carts) and copy sprites, tiles, maps or sounds into the cart, pulling palettes, `from=` sources, clip frames and map tiles along; name clashes are renamed (`dungeon_main`, `key_dungeon`) |

The starter carts double as the art library, so a new game can begin from a knight, a bat, a
key and a dungeon tileset that already fit together.

## Configuration

`~/.config/kartu/config.toml` (never touched by updates):

```toml
library = ["~/kartu-art", "~/my-carts"]   # more carts/packs the AI can search and copy from
judge = "~/bin/my-judge"                  # optional, below
[device.miyoo]
ip = "192.168.1.50"
```

`kartu config` prints the effective settings and paths; `kartu doctor` checks the install.

## The judge hook

`playtest` decides the mechanical facts: won, time, stuck, unfair hits. Opinions — is it too
easy, is the layout confusing, does it match the brief — are yours to plug in. Set `judge` to a
command; `scripts/judge.py` shows the contract: it receives JSON (`{cart, facts, trace}`) on
stdin and prints its opinions, which `playtest` appends to its report. Anything can sit behind
it: a script, another model, a person.

## Cost and quality, honestly

In our runs, a capable model given `kartu new` + `AGENTS.md` + the MCP tools builds a small,
provably winnable top-down game from a one-line brief in one to two attempts. The games are
short and plain unless asked for more; the art is only as good as the starter library plus
whatever you import. The parts that most improve results are, in order: the kit's defaults,
the guide, and the art available to copy. All three are in this repository and all three take
pull requests.
