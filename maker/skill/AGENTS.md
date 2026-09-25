# Making a Kartu game

This folder is a **Kartu cart**: a small retro game for a 320×240 fantasy console.
`main.lua` is the game (Lua 5.4), `assets.cw` is its art, maps and sounds as plain text,
`bot.txt` is a plan a goal bot follows to prove the game can be won, `kartu.toml` names it.
The person you're working with describes the game they want; you build it here.

## Tools

If you have the `kartu` MCP tools, use them. Otherwise run the same things in a shell:

| MCP tool | shell | what |
|---|---|---|
| `docs` | `kartu docs [topic]` | the API reference, by topic. Start with `kits`, then `assets`, `bots` |
| `cart_read` / `cart_write` / `cart_patch` | read / edit files | every edit of main.lua or assets.cw should be followed by `check` |
| `check` | `kartu check .` | every problem at once: art errors, Lua syntax, unknown names, a smoke run |
| `run` | `kartu run . --frames 600 --press "60:a!" --shot-at 120,400 --shot s.png` | play it headless: its `log()` lines, watched values, screenshots — look at them |
| `state` | `kartu run . --dump 300` | the game's variables as JSON at a frame |
| `playtest` | `kartu playtest .` | the goal bot plays `bot.txt` on seeds 1-3 and must reach `WIN`; PASS or FAIL |
| `asset_search` / `asset_copy` | — | reuse art and sounds from the starter library |
|  | `kartu art import pic.png --name x --append .` | turn any PNG into sprites + a palette |
|  | `kartu web .` | play it for real in the browser (`kartu play .` on Linux handhelds) |

## How to work

1. **Read before writing.** `docs` with no topic lists the topics. For a top-down game (adventure,
   collect-a-thon, dungeon) the `kits` topic gives you a whole game from one `td.setup{...}` table: use it.
2. **Start from the working starter** in this folder and change it toward the brief. Keep it running:
   small steps, `check` after every edit.
3. **Look at it.** `run` with screenshots at a few frames and actually look. Fix what looks wrong
   (sprites too small to read, text off screen, everything the same colour).
4. **Make `log()` lines say what happens** — `got key`, `room 2`, `hurt`, `WIN`. The bot, the runner
   and you all read them. Print exactly `WIN` when the player wins.
5. **Prove it can be won.** Update `bot.txt` for your game (`docs bots`) and `playtest` until it
   PASSES on every seed. A hit within 1 s of entering a room is reported as unfair: move the enemy.
6. **You're done when playtest passes and the screenshots match what they asked for.**
   Small and polished beats big and broken. Then tell them in 2-3 friendly sentences what the game
   is and how to play it (arrows move, Z = the A button; `kartu web .` plays it in the browser).

When they ask for a change: read the cart first, change only what they asked for, and make sure
playtest still passes (update `bot.txt` if the change needs it).

## Rules of the console

- 320×240, 16×16 tiles, sprites up to 64 px, palettes of 15 colours + clear. Text is 8×8.
- No files, clock, network or `require`: the game is `main.lua` + `assets.cw`. `rnd()` is seeded,
  so a run with the same inputs and seed is identical everywhere.
- A runaway loop becomes a readable error on screen, not a hang. Every error names its line.
- Buttons: `left right up down a b x y l r start select`; keyboard arrows, Z = a, X = b.
