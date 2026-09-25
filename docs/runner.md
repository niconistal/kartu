# The runner

`kartu` runs carts without a window: it steps the console, prints what the cart logs, saves
screenshots, dumps state, replays inputs and lets bots play. This is the test loop that makes
Kartu carts checkable — by you, by CI and by an agent. `kartu help` prints every flag.

## `check`

```sh
kartu check <cart> [--frames 120] [--script F] [--press S]
```

Everything wrong, at once: every `assets.cw` error, `main.lua` syntax, unknown names in
literal `spr("…")` / `bg.map` / `bg.tile` / `pal` / `spawns` arguments and unknown flag names
(with a did-you-mean), `move()` in a cart with no solid tiles, then a smoke run of N frames for
runtime errors and dropped sprites. Exit 1 on any error.

```
$ kartu check carts/crypt
carts/crypt: 2 palettes, 9 sprites, 3 clips, 5 tiles, 1 maps
check: 0 error(s), 0 warning(s), ran 120 frames clean
```

## `run`

```sh
kartu run <cart> [--frames N] [--seed S] [--script FILE] [--press "S,S"] [--until TEXT]
                 [--shot out.png] [--shot-at 70,190] [--shot-every N] [--scale 2]
                 [--record FILE] [--watch "a;b"] [--watch-every N] [--dump F,F|end]
                 [--save FILE] [--load FILE] [--persist FILE] [--bot PLAN] [--sounds] [--wav FILE]
```

Runs N frames (default 600) and prints `[log fN] …` lines and the final frame hash. The run
stops at a cart error (exit 2) and at `--until`.

| | |
|---|---|
| `--until WIN` | stop after the first frame whose log contains the text; exit 3 if it never appears. **This is the win check:** `kartu run cart --frames 5000 --script win.txt --until WIN` |
| `--shot F` | screenshot of the last frame. `--shot-at 70,190` saves `F-f70.png`, `F-f190.png`; `--shot-every 60` every 60th frame. A shot of frame F is the picture after that frame's update + draw |
| `--scale K` | screenshot scale (2) |
| `--seed S` | the `rnd()` seed (1 — the web player uses 1 too) |
| `--watch "hero.x;hero.y;state"` | print Lua expressions every `--watch-every N` frames (60). They see globals **and file-level locals**, so no `log()` calls needed |
| `--dump F,F\|end` | the cart's whole state (globals + file-level locals, no functions) as JSON at those frames |
| `--record FILE` | write the buttons actually used as an input script (merges `--script` and `--press`) |
| `--save FILE` / `--load FILE` | a save is the input so far plus the frame hash; loading replays it from boot and refuses if the hash no longer matches (the cart changed) |
| `--persist FILE` | the cart's own `save()`/`load()` string: read into `load()` at boot, the last `save()` written at the end (`[save fN] N bytes` lines). Without it a run is unsaved: `load()` is nil and saves are dropped, so runs stay reproducible |
| `--bot PLAN` | let a goal bot play (below); exit 4 if it fails or gets stuck |
| `--sounds` / `--wav F` | print `[sound fN]` lines / write the run's audio |

Exit codes: 0 ok · 1 check failed · 2 cart error · 3 `--until` never appeared · 4 bot failed.

## Input scripts

`--script FILE` or `--press "…"`. One step per line, or separated by commas/spaces:

```text
# comments start with # or --; blank lines are fine
30:right+a    from frame 30 on, hold exactly these buttons (replaces what was held)
40:           release everything
+12:up        frame relative to the previous step (here 52)
60:a!         tap: add `a` on top of whatever is held, for 1 frame
70:b!5        add `b` for 5 frames
```

Frame F is the frame whose `update()` sees those buttons — the one logged `[log fF]`. `btnp`
fires in frame F only if the button was up in frame F−1: to press A twice, release it in between
(`100:a!`, `110:a!`).

The web player's **● Record** button produces the same format from a human playthrough,
together with the hash to check it against.

## Goal bots

A plan says *what* to do; the bot works out the buttons each frame. It reads the hero's box,
path-finds over the live tile map (`solid` tiles; a door that opened no longer blocks) and
steers. Knocked off course, it replans next frame. It doesn't time jumps.

```text
# one step per line; any step may end with max=N (frames before it fails)
hero hero                   # expression for the player object (default `hero`); any Lua
                            #   expression, e.g. {x = p.x + p.hx, y = p.y + p.hy, w = p.hw, h = p.hh}
solid wall,block            # also treat these tiles as solid (carts without flags)
avoid bats 24               # route around these objects (a list or one; alive=false/dead=true skipped)
fight enemies a 30          # or fight them: face and press a within 30 px
abort state == "over"       # fail at once if this becomes true
tap a                       # press a for one frame
hold right 12               # hold buttons for N frames (walk off a screen edge)
wait 30                     # N frames of nothing
wait until state == "play"  # until a Lua expression is true
goto key                    # walk until the hero overlaps it; done at once if it's nil (picked up)
goto flag door              # nearest tile with that flag; a solid one is pushed into (a locked door)
goto tile 19,8              # walk into that tile
collect pickups             # every object in the list, nearest first
```

`[bot fN]` lines report each step (`reached`, `gone`, `pushed into it`) or why it failed.
`no path from tile (3,2) to (35,6)` is a **softlock or a map bug**; `stuck at …` means something
blocks the way. Examples: `carts/crypt/bot.txt` (a hand-written cart), `carts/dungeon/bot.txt`
(the kit).

## `playtest`

```sh
kartu playtest <cart> [--seeds 1,2] [--plan F]
```

Runs `bot.txt` on seeds 1–3 and decides for itself: won or not, win time, stuck, errors, and
**unfair hits** (hurt within one second of entering a room). PASS only if the bot wins on every
seed. Exit 0 on PASS. The seed-1 trace and an end-of-run screenshot land in `<cart>/.cw/`.

```
$ kartu playtest carts/dungeon
playtest PASS: the bot won on every seed, no unfair hits
seed 1: WON at f1409 (23.5 s)
seed 2: WON at f1426 (23.8 s)
seed 3: WON at f1412 (23.5 s)
```

Make the game's `log()` lines say what happens (`ouch 2`, `got key`, `room 3`, `WIN`): they are
what the playtest reads.

## Regression for a whole folder of carts

`scripts/verify-carts.sh` runs `check` on every cart in `carts/`, replays `win.txt` with
`--until WIN` where there is one, and runs `bot.txt` with `--until WIN` where there is one. CI
runs it on every push.

## `bench`, `sound`, `play`

- `kartu bench <cart>` — headless timing (ms per frame, fps).
- `kartu sound <cart> --song NAME | --sfx NAME [--secs N] [--wav F]` — render one sound.
- `kartu play <cart>` — the Linux framebuffer player for handhelds ([Handhelds](handhelds.md)).
