# Kartu — API reference

A cart is a directory with two text files: `main.lua` (Lua 5.4) and `assets.cw` (art + maps).
Screen **320×240**, 60 frames/s, top-left is (0,0), y grows **down**. Nothing else is loaded:
no `require`, no files, no clock. Sound: see "Sound" (built-in music and effects work with no setup).

Example carts (the starter library: `asset_search` over MCP, or `~/.local/share/kartu/library/`):
`crypt` (the core API by hand, 99 lines of Lua); `bakehouse` and `dungeon` (the **top-down kit**,
~40 lines each: see "Kits" at the end — start there for any top-down game).

Workflow: `kartu new <folder>` (a working starter) → edit both files → `kartu check <cart>` (every
error at once) → `kartu run <cart> --shot-at …` to look → write a **bot plan** (`bot.txt`, below)
and `kartu playtest <cart>` to prove it's winnable and fair. Log `WIN` when the player wins.

## Lifecycle
```lua
function init()   end   -- once, after assets load
function update() end   -- every frame: read buttons, move things
function draw()   end   -- every frame: queue sprites/text, set scroll
```
- A frame that runs > ~4M Lua instructions is stopped and the error shown on screen.
- Any runtime error freezes the cart on a red error screen (message + line).
- **What resets each frame:** only the sprite and text queues. Everything else **persists until
  you change it**: layer maps and edits (`bg.map`, `bg.tile`), `bg.scroll`, `bg.show`,
  `pal` recolours, `backdrop`, `camera`. So set maps once (e.g. in `init` or on a room change), not every frame.

**Coordinates.** World = layer pixels: tile (tx, ty) covers x 16·tx … 16·tx+15. `camera(x, y)` says
which world point is at the screen's top-left; `spr` and `rect` take **world** coordinates and the
camera shifts them (pass `screen=true` for HUD things). `text` is always in screen coordinates.
With the camera at (0,0), which it is until you call it, world = screen.

## Drawing
| call | what |
|---|---|
| `spr(name, x, y [, {fx=bool, fy=bool, pal="palname", layer=0..3, screen=bool}])` | queue a sprite or clip by name at its **top-left** corner (world coords; `screen=true` ignores the camera). Coords are floored; sprites partly or fully off screen are clipped (fine to draw at negative x). `layer` = drawn just after BG layer N (default 3 = in front of every BG layer). Within one layer, later calls draw on top. Max 128 sprites/frame; extras are dropped (`check` warns). |
| `text(str, x, y [, col or opts])` → `w, h` | 8×8 monospace ASCII in **screen** coords, drawn on top of **everything**. Lines are 10 px apart; `\n` starts a new line. Returns the block's size in px. opts: `col=`, `align="left"/"center"/"right"` (x is the left edge / centre / right edge; each line aligned), `wrap=px` (word-wrap to that width), `bg=col` (box behind the block), `pad=2` (box margin), `shadow=col` (1 px drop shadow), `scale=1..4` (big text: 2 = 16 px glyphs). |
| `textw(str [, scale])` | width in px of the widest line (`#str*8` at scale 1). |
| `rect(x, y, w, h, col [, {line=bool, layer=0..3, screen=bool}])` | filled box (`line=true`: 1 px outline). World coords, layered and ordered like sprites (default layer 3, counts toward the 128). |
| `backdrop("#rrggbb")` | colour behind all layers. |
| `pal(palname, entry, "#rrggbb")` | recolour one palette entry (its char, or 0..15) for every sprite and tile using that palette, from now on. There is no way to read a colour back: keep the originals in Lua if you plan to fade. |

**Colours** (`text`, `rect`): a palette char — `"w"` from the **first palette declared**, or
`"hero:w"` from palette `hero` — or an entry number 1..15 of the first palette (the old way).
Chars are the robust choice: reordering a palette doesn't change them.

There are no line/circle/pixel primitives.

**Clips** animate from the global frame counter: all copies of a clip are in step, frame
`floor(frame() * fps / 60) % #frames`.

## Background layers (4, drawn 0 = back … 3 = front)
| call | what |
|---|---|
| `bg.map(layer, "mapname")` | put a copy of a map from assets.cw on a layer (and show it). Later `bg.tile` edits change the copy, not the asset; calling `bg.map` again restores the original. Copying is cheap but pointless every frame. |
| `bg.scroll(layer, x, y)` | the layer works like a **camera**: the layer pixel (x, y) lands at screen (0, 0). `bg.scroll(0, 100, 0)` shows the map from x=100 on, so a world point wx appears at screen `wx - 100`. Layers **wrap** in both directions. |
| `bg.show(layer, bool)` | hide/show a layer. |
| `bg.fixed(layer, bool)` | pin a layer to the screen: `camera()` no longer scrolls it (HUD / dialogue layers). |
| `bg.get(layer, tx, ty)` | name of the tile in that cell, or nil (empty or outside the map). |
| `bg.flag(layer, tx, ty, "flag")` | does that cell's tile have the flag? |
| `bg.size(layer)` | `w, h` of the layer's map in tiles (0, 0 before `bg.map`). |
| `bg.tile(layer, tx, ty, "tilename" or nil)` | change one cell of the layer's copy (nil = empty). tx,ty are 0-based tile coords inside the layer's map. A layer has **no cells until `bg.map`**: to paint freely, map a blank map first (`map blank 20x15` of `.` rows). |

Tiles are 16×16. A 320×240 screen is 20×15 tiles. `bg.scroll` is the manual, per-layer
version of the camera (parallax: call it after `camera`); prefer `camera`, which also moves sprites.

## Camera
| call | what |
|---|---|
| `camera(x, y [, {clamp=layer}])` → `x, y` | world point at the screen's top-left. Scrolls every layer not `bg.fixed` and offsets later `spr`/`rect` calls, so call it at the **top of `draw()`**. `clamp=0` keeps the view inside layer 0's map (no wrap-around at the edges). Returns what it used. Follow a hero: `camera(hero.x - 152, hero.y - 112, {clamp=0})`. |
| `camera()` | current `x, y`. |

## World: collision, map markers
Objects are plain Lua tables with `x, y` (top-left, world px, fractions fine) and a size, from the
first of these that's present:
- `hit = {x, y, w, h}` — box relative to (x, y)
- `spr = "name"` — the sprite's `hit=` box from assets.cw, else its full size (clip → first frame)
- `w = , h =` — box at (x, y)

The box doesn't flip with `fx`, so keep hit boxes centred (or derive a flipped sprite, whose
`hit=` is mirrored). Collision checks **every layer that has a map**, visible or not (a hidden
layer makes a fine collision mask). Cells outside a map never collide. The camera doesn't matter.

| call | what |
|---|---|
| `move(o, dx, dy [, "flag"])` → `blocked_x, blocked_y` | move `o` (writes `o.x`, `o.y`), x then y, stopping flush against tiles with the flag (default `"solid"`). Sliding along walls comes free. An object already inside a wall may walk out. Bounce: `local bx, by = move(b, b.vx, b.vy) if bx then b.vx = -b.vx end`. |
| `overlap(a, b)` | do two objects' boxes overlap? (touching edges don't count) |
| `hitbox(o)` → `x, y, w, h` | the world box used by all of these. |
| `solid(o [, "flag"])`, `solid(x, y [, w, h] [, "flag"])` | is any flagged tile under the object / point / box? |
| `touching(o, "flag")` → `tx, ty, layer` or nil | first flagged cell under the box (doors, spikes, exits). To detect a **solid** door you're pushing against, test a box nudged 1–2 px that way. |
| `spawns("map" [, "kind"])` | markers from the map's legend (below): list of `{kind=, x=, y=, tx=, ty=}` in reading order (x, y = the cell's top-left px). |

An unknown flag name is an error (a typo would otherwise never collide); `"solid"` with no
solid tiles just matches nothing (`check` warns).

## Input
`btn(b)` = held now, `btnp(b)` = pressed this frame (down now, up last frame). Buttons are
strings: `"left" "right" "up" "down" "a" "b" "x" "y" "l" "r" "start" "select"`.
A button held across a state change stays held: `btnp` will not fire again until it is
released and pressed. Keyboard in the web player: arrows, Z = a, X = b, A = y, S = x, Q/W = l/r,
Enter = start, Shift = select.

## Other
| call | what |
|---|---|
| `rnd([n])` | float in [0, n) (n defaults to 1). |
| `irnd(a, b)` | integer in [a, b]. |
| `frame()` | frames since boot (0 during `init` and the first update). |
| `sin(t)`, `cos(t)` | t in **turns** (1.0 = full circle), not radians; standard sign (`sin(0.25)` ≈ 1). A deterministic polynomial, accurate to ~1e-11: `sin(0.25)` is 0.99999999999, so never compare with `==`. |
| `palette([name])` | current colours of a palette (default: the first declared) as `{ {ch="k", hex="#1a1c2c"}, … }`: fades start here. |
| `sprsize(name)` | `w, h` of a sprite or clip. |
| `has(name)` | `"sprite"`, `"clip"`, `"tile"`, `"map"`, `"palette"` or nil: make art optional. |
| `kit(name)` | load a built-in kit (Lua library) — see "Kits". |
| `log(...)` / `print(...)` | message to the runner's console as `[log fN] …` (N = the frame it was logged in). |

**Randomness is seeded.** Same seed + same inputs ⇒ same game, bit for bit. The runner and the
web player both use seed 1 unless told otherwise (`--seed`). Any change to how often you call
`rnd` changes everything after it, so replay scripts break when you retune; that's expected.

Standard Lua available: `string`, `table`, `math`, `utf8`, `coroutine` (no `io`, `os`, `load`,
`require`). Lua 5.4 gotchas: `string.format("%d", 1.5)` is an **error** (floats need
`math.floor` first, or `%.0f`); `7 // 2` is integer division (3), `7 / 2` is 3.5.

## Sound
8 voices, synthesised by the console (24 kHz mono, the same samples on every target). Songs use
up to 6 voices; sound effects get the rest. **Everything below works with no setup**: the
console has built-in instruments, sound effects and songs. The top-down kit already plays
effects for its events (see Kits: `sounds`, `music`).

| call | what |
|---|---|
| `sfx(name [, {vol=0..2, pitch=semitones}])` | play a sound effect. Several can overlap; the oldest is cut if all voices are busy. |
| `music(name [, {vol=, loop=bool}])` | start a song (loops unless the song says `once`). Calling it again with the song that's already playing does nothing, so it's safe every frame. |
| `music(nil [, {fade=secs}])` | stop the music (optionally fading out). |
| `music_playing()` | name of the song playing, or nil. |
| `play(mml [, {inst="bell", vol=0.7}])` | play a short tune written in MML (below) as a sound effect, e.g. `play("o5 l16 c e g >c4", {inst="bell"})`. |
| `has(name, "sfx"|"song"|"instrument")` | does it exist? |

**Built in.** Sound effects: `coin pickup key heal powerup jump hit hurt swing laser explosion blip
select door locked talk step magic win lose start`. Songs: `adventure` (bright, loops), `village`
(gentle), `cave` (slow, minor), `boss` (fast, tense), `victory` (short fanfare, plays once).
Instruments: `piano epiano bell marimba organ flute lead softlead strings pad brass pluck harp
bass synbass pluckbass square pulse triangle saw sine noise` and drums `kick snare hat openhat
clap tom crash`. Hear them all in the `jukebox` cart.

### Your own sounds (assets.cw)
Three more keywords. Sounds have their own names (a `coin` sfx and a `coin` sprite are fine).
Declare instruments before the songs that use them.
```text
instrument buzz from=lead duty=0.25 vibrato=0.3,6    -- start from a built-in, change a few things
instrument twang wave=pluck decay=0.6 lowpass=0.5
sfx zap wave=saw from=C7 to=C4 len=0.2 vol=0.6       -- a pitch sweep
sfx chime notes="o6 l16 c e g >c4" inst=bell         -- or a little tune
song theme bpm=120                                   -- then one line per channel (max 6), indented
  lead  @lead o5 l8 [e d c d e e e4]2 | d d d4 e g g4
  bass  @bass o3 l4 c g c g c g c g
  drums @drums [k8 h8 s8 h8]4
```
- **Instrument settings**: `wave=square|triangle|saw|sine|noise|pluck|fm`, `duty=0.5` (square),
  `attack decay sustain release` (seconds, sustain 0..1), `vol`, `vibrato=depth,rate[,delay]`
  (semitones, Hz, s), `slide=semitones,secs` (start off-pitch and glide in), `lowpass=0..1`
  (1 = bright, 0.2 = soft), `detune=semitones` (a second, detuned oscillator: fuller),
  `fm=ratio,index[,decay]` (bells, e-pianos), `noise=0..1` (mix noise in), `transpose`, `note=C2`
  (fixed pitch: drums), `from=name`.
- **Sweep sfx**: `wave from to len vol duty vibrato noise lowpass env=decay|flat|swell`; `from`/`to`
  are notes (`C5`, `F#3`) or Hz. **Tune sfx**: `notes="MML" inst= bpm=150 vol=`.
- **Songs**: `song name bpm=N [once] [vol=0.8]`. Channel lines with the same name join up, so a
  long part can span lines. The song loops at the end of its **longest** channel: make every
  channel the same length (`check` warns if not, and prints each song's bars).

**MML** (the notes): `c d e f g a b` with `#`/`+` sharp, `-` flat, then an optional length
(`4` quarter, `8` eighth, `2` half, `1` whole, `16`, `3`/`6`/`12` triplets; `.` dotted) ·
`r` rest · `o4` octave (o4 c = middle C), `<` `>` down/up an octave · `l8` default length ·
`v0`–`v15` volume (12) · `q1`–`q8` how much of each note sounds (7 = slightly detached, 8 = legato) ·
`^8` tie (extend the last note) · `@name` instrument · `[ ... ]3` repeat (default 2) · `|`
ignored (bar lines for reading). In a `@drums` channel the letters are drums: `k` kick, `s` snare,
`h` hat, `o` open hat, `c` clap, `t` tom, `x` crash, `r` rest, with lengths as usual (`k8 h8 s8 h8`).

Writing music that sounds good: keep melodies in o4–o6 and bass in o2–o3; give each channel a
role (melody, chords/arpeggio, bass, drums); make every bar add up (4 quarters = `l8` × 8);
build from a 4-chord loop (C–G–Am–F, or Am–F–C–G for sad) with the bass playing the roots; lower
the accompaniment (`v8`). Copy a built-in song as a starting point: its MML is in the
`sound_songs` docs topic.

## assets.cw
Line based. `--` starts a comment on keyword, palette and legend lines (at line start or after a
space); **grid rows are read raw**, so no comments there. Rows are trimmed, so indenting is fine.
```text
palette hero            -- ≤16 entries: <one char> <#rrggbb | clear> [label]; first entry must be clear
  . clear
  k #1a1c2c ink         -- label is optional, just for you
  O #ef7d57 orange
sprite cat 8x8 pal=hero -- 1..64 px each side; then exactly H rows of W palette chars
..k..k..
.kOkkOk.
...
sprite cat_l from=cat fx             -- derived: no rows. fx, fy, rot=90|180|270 (clockwise), pal=other
sprite cat_red from=cat pal=villain  -- palette swap: pixels keep their entry number (0..15), so a
                                     --   palette with the same order recolours the sprite
sprite hero 16x16 pal=hero hit=4,6,8,9  -- collision box inside the sprite: x,y,w,h (derived copies inherit it, flipped)
clip cat_run fps=8 cat.1 cat.2       -- animation: sprite names, in order; spr("cat_run", ...) plays it
tile wall 16x16 pal=room             -- tiles are always 16x16; 16 rows of 16 chars
tile wall_r from=wall fx             -- tiles derive the same way (from a tile or a 16x16 sprite)
tile door 16x16 pal=room flags=solid,door   -- flags: any words (max 16 distinct); `solid` is what move() stops at
map level1 20x15                     -- W x H tiles (max 256x256)
legend # wall  , floor               -- <char> <tilename> pairs, any spacing; several legend lines OK
legend @ @hero:floor  b @bat:floor   -- spawn markers: @kind (empty cell) or @kind:tile (that tile under it)
####################                 -- then exactly H rows of W legend chars
```
Markers put monsters, pickups and the start point in the map picture itself; read them with
`spawns("level1")`, e.g. `local h = spawns("level1", "hero")[1]; hero = {x=h.x, y=h.y, spr="hero"}`.
- `.` means an empty cell unless the legend redefines it; `<char> empty` makes another empty char.
- Max 16 palettes, 16 entries each. A palette can be used by both sprites and tiles.
- **Names:** palettes have their own namespace (`palette car` + `sprite car` is fine).
  Sprites, clips, tiles and maps share one (a sprite and a tile can't both be `grass`). No spaces in names.
- Declare palettes before the sprites that use them, and sprites before a `from=` that copies them.
  Clips and maps may name things declared later.
- Every error names the line. `kartu check` lists all of them in one go.

### Importing art
Bring your own image generator: `kartu art import` turns any PNG into assets.cw text.
```sh
kartu art import hero.png --name hero --size 32 --outline            # print palette + sprite
kartu art import boss.png --name boss --size 128 --append mycart     # add to mycart/assets.cw
kartu art import coin.png --name coin --size 12 --pal main --append mycart   # reuse a palette
```
It removes the background where it touches the border (a flood fill, so the same colour inside
the subject survives; transparent pixels always go), crops to the subject, area-averages it
down, and quantises to ≤15 colours, darkest first (entry chars `1`…`f`, `.` clear).
- `--size N` = longest side N, aspect kept; `--size WxH` = exactly that. Default 32, max 256.
  Over 64 px the result is cut into `NAME_0_0`, `NAME_1_0`… (column_row) 64 px pieces, empty
  ones skipped; a comment above them gives each piece's offset.
- `--bg auto` (default) keys out the border's main colour if there is one; `black`, `none`,
  or `#rrggbb`; `--tol N` (34) = how far per channel still counts as background.
- `--colors N` (15), `--outline` (1 px, darkest colour), `--dither 0.3` (ordered, 0..1).
- `--out FILE`, or `--append CART`: refuses name clashes and re-checks the whole assets.cw
  before writing. `--pal NAME` (needs `--append`) maps onto one of the cart's palettes instead
  of making a new one: handy when you are near the 16-palette limit.
- PNG only (any colour type); convert other formats first. Same input, same output.
- Generate on a flat background (black or one solid colour), subject centred, not touching the
  edges. Small sprites read best from simple, high-contrast images.

## Runner
`kartu help` prints all of this.
```sh
kartu check <cart> [--frames 120] [--script F]
kartu run   <cart> [--frames N] [--seed S] [--script FILE] [--press "S,S"]
                        [--until TEXT] [--shot out.png] [--shot-at 70,190] [--shot-every N]
                        [--scale 2] [--record FILE]
```
- `check`: all assets.cw errors, main.lua syntax, unknown names in literal
  `spr("…")`/`bg.map`/`bg.tile`/`pal`/`spawns` arguments and unknown flag names (with a
  did-you-mean), `move()` with no solid tiles, then a smoke run of
  N frames for runtime errors and dropped sprites. Exit 1 on any error.
- `run`: runs N frames (default 600) headless and prints `[log fN]` lines and the final
  frame hash. The run **stops at a cart error** (exit 2) and at `--until`.
- `--until WIN`: stop after the first frame whose log contains `WIN`; exit 3 if it never
  shows up. This is the win check: `kartu run cart --frames 5000 --script win.txt --until WIN`.
- `--shot` = the last frame run. `--shot-at 70,190` saves `out-f70.png`, `out-f190.png`
  (named after `--shot`, default `shot.png`). `--shot-every 60` does every 60th frame.
  A shot of frame F is the picture after update+draw of frame F.
- `--record FILE`: writes the buttons actually used as a script (merges `--script` + `--press`).
  The **web player** has a ● Record button that does the same for a human playthrough: it
  restarts the cart at seed 1 and gives you the script plus the hash to check it against.

- `--watch "hero.x;hero.y;state"`: print Lua expressions every `--watch-every N` frames (60).
  They see the cart's globals **and its file-level `local`s**, so no `log()` calls needed.
- `--dump F,F|end`: the cart's whole state (its globals + file-level locals, no functions) as JSON.
- `--save FILE` / `--load FILE`: a save is the input that got there plus the frame hash; loading
  replays it from boot (fast) and refuses if the hash no longer matches (cart changed).
- `--bot PLAN`: let a goal bot play (below). Exit 4 if it fails.
- `--sounds` prints `[sound fN] sfx coin` / `music theme` lines; `--wav out.wav` saves the sound;
  the summary gets an `audio <hash> …` line (peak level, clipped samples) when anything played.
  `kartu sound <cart> --song NAME|--sfx NAME [--wav F]` renders one sound on its own;
  with no name it lists everything the cart can play.

### Bots (`--bot plan.txt`)
A plan says *what* to do; the bot works out the buttons each frame: it reads the hero's box,
path-finds over the live tile map (solid = tiles with the `solid` flag, doors that opened count)
and steers. Knocked off course? It replans next frame. It doesn't fight or time jumps.
```text
# one step per line; any step may end with max=N (frames before it fails)
hero hero                   # expression for the player object (default `hero`); any Lua
                            #   expression, e.g. {x = p.x + p.hx, y = p.y + p.hy, w = p.hw, h = p.hh}
solid wall,block            # also treat these tiles as solid (carts without flags)
avoid bats 24               # route around these objects (a list or one; alive=false/dead=true skipped)
tap a                       # press a for one frame
hold right 12               # hold buttons for N frames (walk off a screen edge)
wait 30                     # N frames of nothing
wait until state == "play"  # until a Lua expression is true
goto key                    # walk until the hero overlaps it; done at once if it's nil (picked up)
goto flag door              # nearest tile with that flag; a solid one is pushed into (locked door)
goto tile 19,8              # walk into that tile
```
`[bot fN]` lines report each step (`reached`, `gone`, `pushed into it`) or why it failed: `no path
from tile (3,2) to (35,6)` is a **softlock or a map bug**, `stuck at …` means something blocks it.
Examples: `crypt/bot.txt`, `keyquest/bot.txt` in the starter library (a cart without flags or `hero`).

**Playtest:** `kartu playtest <cart>` runs `bot.txt` on seeds 1-3 and decides won / win time /
stuck / errors / unfair hits (hurt within 1 s of arriving) itself: PASS only if the bot wins on
every seed. Exit 0 on PASS. Make the game's `log()` lines say what happens (`ouch 2`, `got key`,
`room 3`, `WIN`): they're what the playtest reads.

### Input scripts (`--script FILE`, `--press`)
One step per line, or separated by commas/spaces:
```text
# comments start with # or --, blank lines are fine
30:right+a    from frame 30 on, hold exactly these buttons (replaces what was held)
40:           release everything
+12:up        frame relative to the previous step (here 52)
60:a!         tap: add `a` on top of whatever is held, for 1 frame
70:b!5        add `b` for 5 frames
```
Frame F = the frame whose `update()` sees those buttons, the one logged `[log fF]`.
`btnp` fires in frame F only if the button was up in frame F-1: to press A twice, release
it in between (`100:a!`, `110:a!`). A tap on a button that is already held does nothing.

## Kits
A kit is a Lua library built into the console: `local td = kit("topdown")`. It runs as cart
code (same budget), so anything it does you could do by hand, and you can mix both.

### Top-down kit (`kit("topdown")`)
Zelda-like adventures: hero, rooms, enemies, pickups, doors, dialogue, HUD, title/win/lose
screens, y-sorted drawing. A whole cart:
```lua
local td = kit("topdown")
td.setup {
  title = "DUNGEON", intro = "Find the key. A swings your sword.",
  rooms = { { "hall", "gallery", "vault" } },            -- or map = "one_big_map"
  hero = { spr = "knight1", walk = "knight_walk", hp = 3 },
  attack = { btn = "a", spr = "sword", spr_v = "swordv" },
  enemies = { bat = { spr = "bat_fly", move = "chase", speed = 0.8, hp = 1, drop = "heart" } },
  pickups = { key = { spr = "key", toast = "You found the key!" }, heart = { spr = "heart", heal = 1 } },
  doors = { door = { needs = "key", opens = "dooropen" } },   -- tile names
  win = "flag:exit",                                          -- dooropen has flags=exit
  hud = { hearts = { "heart", "heart0" }, show = { "key" } },
}
function update() td.update() end
function draw() td.draw() end
```
and in assets.cw, place things with markers, one kind per `@name`:
`legend P @hero:floor  B @bat:floor  K @key:floor`.

Rules of thumb:
- Room maps are 20x15; the top 14 px are under the HUD, so make row 0 wall (or a copy of row 1
  where a doorway leads up). Leave gaps in the outer wall where rooms connect (grid neighbours).
- Give solid tiles `flags=solid`; a door tile `flags=solid,door`; its open version `flags=exit`
  if walking through it wins.
- Sprites face **right**; the kit flips them for left. Without `hit=`, the hero collides with its
  lower body and enemies with a slightly smaller box (fair, fits 1-tile gaps).
- `ui` colours default to the darkest/lightest entries of the first palette; set them if that's wrong.
- Don't put an enemy within ~6 tiles of a room entrance or the start: playtest flags hits within
  1 s of arriving as unfair.

Config reference (from the kit's source):
```text

  local td = kit("topdown")
  td.setup { title = "CRYPT", map = "crypt", hero = { spr = "knight" }, ... }
  function update() td.update() end
  function draw() td.draw() end

Everything lives in the global `world` (hero, enemies, pickups, npcs, room, state,
items), so bots, playtest and your own code can read it. Things are placed with
map markers: `legend @ @hero:floor  b @bat:floor  k @key:floor`, and each marker kind
must be a key of cfg.enemies, cfg.pickups or cfg.npcs (or "hero").

cfg fields (all optional except map/rooms and hero):
  map = "name"                 one map (scrolls if bigger than the screen)
  rooms = {{"a","b"},{"c","d"}}  a grid of maps; walking off an edge enters the neighbour
  names = { a = "KITCHEN" }    room names for the HUD (default: map name in capitals)
  hero = { spr=, hit={x,y,w,h}, walk=, up=, down=, walk_up=, walk_down=, left=, walk_left=,
           speed=1.5, hp=3, diag=true }
           spr/walk face right (flipped for left unless left= is given); up/down optional
           hit: default = the sprite's hit=, else a lower-body box (fits 1-tile gaps)
  attack = { btn="a", spr=, spr_v=, reach=14, time=12, damage=1 }   (no attack if absent)
  enemies = { bat = { spr=, hit=, hp=1, speed=1, move="still|bounce_x|bounce_y|wander|chase",
                      range=80, damage=1, flying=false, drop="heart" } }
  pickups = { key = { spr=, item="key" (default: the kind), heal=0, score=0,
                      goal=false, say=, toast= } }
  npcs = { owl = { spr=, name="OWL", say = {"line", "line"} or function(world) ... end } }
  doors = { door = { needs="key", consume=true, opens="dooropen", locked="Locked." } }
  win = "flag:exit" | "collect" (every goal pickup) | "collect:cookie" | function(world)
  title=, intro=, win_text="YOU WIN!", lose_text="GAME OVER"
  hud = { hearts = {"heart", "heart0"}, show = {"key", "cookie"} } or false
  sounds = { pickup="coin", hurt="hurt", ... } override the default sfx per event, or
           sounds = false for silence. Events and defaults: start=start pickup=pickup
           goal=coin key=key heal=heal hurt=hurt swing=swing hit=hit defeat=explosion
           door=door locked=locked talk=talk win=win lose=lose (a pickup's own sfx= wins)
  music = "song"  or { title=, play=, win=, lose=, rooms = { cellar = "cave" } }
           songs from assets.cw or built in (adventure cave village boss victory)
  ui = { ink="k", paper="w", accent="y" }  (default: darkest / lightest of the 1st palette)
  on = { start=fn(w), hurt=fn(w, n), defeat=fn(w, e), pickup=fn(w, p), room=fn(w, name),
         update=fn(w), draw=fn(w) (world space, after actors), hud=fn(w) }

Logs (what bots and playtest read): state <s>, room <name>, got <kind>, hurt hp=<n>,
defeated <kind>, door open, locked, talk <npc>, WIN, LOSE.
```
Other calls: `td.say(lines [, who])` dialogue, `td.toast(msg)`, `td.box(x, y, w, h)` framed panel,
`td.spawn(kind, x, y)`, `td.hurt(n)`, `td.win()`, `td.lose()`, `td.goals_left([kind])`.

**World for bots:** everything is in the global `world`: `world.hero`, `world.enemies`,
`world.pickups`, `world.npcs` (current room), `world.room`, `world.state`
(`title|play|win|over|pause`), `world.items` (inventory counts), `world.talk`. A kit bot plan:
```text
hero world.hero
avoid world.enemies 20          # or: fight world.enemies a 30  (if the hero has an attack)
abort world.state == "over"
tap a
wait until world.state == "play"
collect world.pickups           # everything in this room
goto tile 19,7                  # walk to the east doorway…
hold right 16                   # …and through it
wait until world.room == "pantry"
```
