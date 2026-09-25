# Making a game

The same loop works whether the hands on the keyboard belong to a person or a language model.
It is short on purpose:

1. **Start from something that runs.** `kartu new` gives you a small, winnable game. Change it
   toward the game you want, in small steps.
2. **Check after every edit.** `kartu check <cart>` lists every art error, syntax error and
   unknown name at once, then runs a few frames to catch runtime errors.
3. **Look at it.** `kartu run <cart> --shot-at 120,600` saves screenshots. Actually look at
   them: sprites too small to read, text off the screen, everything one colour.
4. **Make the logs say what happens.** `log("got key")`, `log("room 2")`, `log("hurt")`, and
   exactly `log("WIN")` when the player wins. The bot, the runner and you all read them.
5. **Prove it can be won.** Write a `bot.txt` plan and run `kartu playtest <cart>` until it
   passes on every seed. A hit within a second of entering a room is reported as unfair:
   move the enemy.
6. **Then stop.** Small and polished beats big and broken.

## With an AI

```sh
kartu new frog-quest && cd frog-quest
claude    # "make this a game where a frog hops across lily pads to eat 5 flies"
```

`kartu new` writes the cart *and* the instructions an agent needs to change it: `AGENTS.md`
is this loop written for a model, `CLAUDE.md` points at it, and `.mcp.json` gives Claude Code
the maker tools (`check`, `run` with screenshots as images, `playtest`, `asset_search`…). Any
agent with a shell can do the same work with the `kartu` commands — the MCP tools and the CLI
are the same functions.

What makes agents good at Kartu is not a clever prompt. It is that every step above gives them
something to read: an error with a line number, a screenshot, a JSON dump of the game's
state, a PASS or a FAIL with a reason. Put the effort into `log()` lines and `bot.txt` and the
model does the rest.

Details of the tools and how to connect other agents: [AI tools](ai-tools.md).

## By hand

Everything above, plus `kartu web <cart>` to play for real. The web player has two things
worth knowing:

- **● Record** restarts the cart at seed 1 and saves your buttons as a replay script. Save it
  as `win.txt` and `kartu run <cart> --script win.txt --until WIN` becomes a regression test that
  a human once won this game — and because the console is deterministic, the replay produces
  the same frame hash natively as it did in your browser.
- **Verify** replays a fixed script and compares the frame hash with the native build, which is
  how you know the wasm player and the runner agree pixel for pixel.

## Anatomy of a cart

```text
frog-quest/
├── main.lua      the game (Lua 5.4)
├── assets.cw     palettes, sprites, tiles, maps, sounds — plain text
├── bot.txt       the goal bot's plan: proof the game can be won
├── kartu.toml    title + format version
├── AGENTS.md     how to make a Kartu game (for agents; CLAUDE.md points here)
├── .mcp.json     the maker tools for Claude Code
└── .cw/          runner scratch (screenshots, playtest output); ignored by git
```

A cart needs only the first two. A `-- title: Frog Quest` comment on the first line of
`main.lua` names it in the web player's picker and the handheld menu.

## Rules of the console, briefly

- 320 × 240, 16 × 16 tiles (a screen is 20 × 15 tiles), sprites up to 64 px, palettes of
  15 colours + clear, 8 × 8 text.
- No files, clock, network or `require`. `rnd()` is seeded, so a run with the same inputs and
  seed is identical everywhere.
- A frame that runs past its instruction budget, or any Lua error, becomes a readable error on
  screen with the line number. Nothing hangs.
- Buttons: `left right up down a b x y l r start select`.

The complete rules are in [API.md](../API.md).

## Writing a bot plan

A plan says *what* to do; the bot works out *how*, frame by frame, by path-finding over the
live tile map. Plans are short:

```text
# bot.txt for a top-down kit game
hero world.hero
avoid world.enemies 20          # or: fight world.enemies a 30
abort world.state == "over"
tap a                           # leave the title screen
wait until world.state == "play"
collect world.pickups           # everything in this room
goto tile 19,7                  # walk to the east doorway…
hold right 16                   # …and through it
wait until world.room == "pantry"
goto flag exit
```

`goto key` walks until the hero overlaps the object called `key`; `goto flag door` walks to the
nearest tile with that flag; `wait until` takes any Lua expression. When the bot reports `no path
from tile (3,2) to (35,6)`, that is a softlock or a map bug, not a bot bug. The full plan
language is in [The runner](runner.md#goal-bots).

## Making it look good

- Palettes are the rails. Pick 15 colours per palette with real contrast and stick to them;
  derived sprites (`sprite cat_red from=cat pal=villain`) give you variants for free.
- Sprites read best at 16 px or bigger. Hero 16 × 16, bosses 32–48.
- Every room needs a wall row under the HUD (the top 14 px are covered).
- Use the built-in music and sound effects first; the top-down kit already plays them for
  every event. Write your own when the defaults feel wrong ([Sound](sound.md)).
- Bring your own image generator: `kartu art import boss.png --name boss --size 48 --append .`
  turns a picture into palette + sprite text ([Cart format → Importing art](cart-format.md#importing-art)).
