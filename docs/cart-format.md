# Cart format: `assets.cw`

All of a cart's art, maps and sounds live in one plain-text file, `assets.cw`. It is line
based and meant to be read: a sprite is a grid of palette characters, a map is a picture made
of legend characters, a song is notation. Every error names its line, and `kartu check` lists
all of them in one go.

`--` starts a comment on keyword, palette and legend lines (at the start of a line or after a
space). **Grid rows are read raw**, so no comments inside a sprite or map. Rows are trimmed,
so indenting is fine.

## Palettes

```text
palette hero             -- up to 16 entries: <one char> <#rrggbb | clear> [label]
  . clear                -- the first entry must be clear
  k #1a1c2c ink          -- the label is optional, just for you
  O #ef7d57 orange
```

- Up to **16 palettes** of 16 entries (15 colours + clear). A palette can be used by both
  sprites and tiles.
- Palettes have their own namespace: `palette cat` and `sprite cat` can coexist.
- Declare a palette before the sprites that use it.
- Colours are 24-bit in the file and 15-bit on the console (5 bits per channel).
- `pal(palname, entry, "#rrggbb")` in Lua recolours one entry for everything that uses that
  palette, from then on — the tool for fades, damage flashes and day/night.

The **first palette declared** is special: its characters are what `text` and `rect` accept
as colours (`"w"`), and it supplies the kit's default UI colours. `"hero:w"` names an entry in
another palette.

## Sprites

```text
sprite cat 8x8 pal=hero        -- 1..64 px each side; then exactly H rows of W palette chars
..k..k..
.kOkkOk.
.kOOOOk.
.kOkOOk.
..kkkk..
.kOOOOk.
..k..k..
........
```

Derived sprites have no rows:

```text
sprite cat_l   from=cat fx             -- flipped: fx, fy, rot=90|180|270 (clockwise)
sprite cat_red from=cat pal=villain    -- palette swap: pixels keep their entry number, so a palette
                                       --   with the same order recolours the sprite
sprite hero 16x16 pal=hero hit=4,6,8,9 -- a collision box inside the sprite: x,y,w,h
```

A `hit=` box is what `move`, `overlap` and the bots use for that sprite; derived copies inherit
it, mirrored when flipped. Without one, an object's box is the whole sprite (the kit chooses a
lower-body box for the hero so it fits through one-tile gaps).

## Clips

```text
clip cat_run fps=8 cat.1 cat.2 cat.3   -- sprite names, in order
```

`spr("cat_run", x, y)` plays the animation. Clips run from the global frame counter, so every
copy of a clip is in step: frame `floor(frame() * fps / 60) % #frames`.

## Tiles

```text
tile wall 16x16 pal=room                    -- tiles are always 16x16: 16 rows of 16 chars
tile wall_r from=wall fx                    -- tiles derive like sprites (from a tile or a 16x16 sprite)
tile door 16x16 pal=room flags=solid,door   -- flags: any words, up to 16 distinct per cart
```

**Flags** are how the game reads the map. `solid` is what `move()` stops at; a `door` flag
lets you find doors with `touching(o, "door")`; an `exit` flag can be the win condition. An
unknown flag name in Lua is an error (a typo would otherwise never collide).

## Maps and legends

```text
map hall 20x15                              -- W x H tiles, up to 256 x 256
legend # wall  T torch  o block  E door  . floor
legend P @hero:floor  B @bat:floor  K @key:floor
####T####E####T#####
#..................#
#...o..........o...#
#.....P.............
#..........B.......#
####################
```

- A legend line is `<char> <tilename>` pairs, any spacing; several legend lines are fine.
- `.` is an empty cell unless the legend redefines it; `<char> empty` makes another empty char.
- **Markers** put the hero, enemies and pickups in the map picture itself: `@kind` on an empty
  cell, or `@kind:tile` with that tile underneath. Read them with `spawns("hall")` →
  `{ {kind="hero", x=, y=, tx=, ty=}, … }`, or let the [top-down kit](kits.md) do it.
- A map is 20 × 15 tiles for one screen; bigger maps scroll with `camera()`.

Sprites, clips, tiles and maps share one namespace (a sprite and a tile can't both be called
`grass`). No spaces in names. Clips and maps may name things declared later; `from=` may not.

## Sounds

Three more keywords — `instrument`, `sfx` and `song` — with their own namespace. They are
covered in [Sound](sound.md); the short version:

```text
instrument twang wave=pluck decay=0.6 lowpass=0.5
sfx zap wave=saw from=C7 to=C4 len=0.2 vol=0.6
song theme bpm=120
  lead  @lead o5 l8 [e d c d e e e4]2 | d d d4 e g g4
  bass  @bass o3 l4 c g c g c g c g
  drums @drums [k8 h8 s8 h8]4
```

## Importing art

Bring your own image generator or paint program: `kartu art import` turns any PNG into
`assets.cw` text.

```sh
kartu art import hero.png --name hero --size 32 --outline          # print palette + sprite
kartu art import boss.png --name boss --size 128 --append mycart   # add to mycart/assets.cw
kartu art import coin.png --name coin --size 12 --pal main --append mycart   # reuse a palette
```

It keys out the background where it touches the border (a flood fill, so the same colour
inside the subject survives; transparent pixels always go), crops to the subject, area-averages
it down and quantises to at most 15 colours, darkest first.

- `--size N` = longest side N, aspect kept; `--size WxH` = exactly that. Default 32, max 256.
  Over 64 px the result is cut into `NAME_0_0`, `NAME_1_0`… (column_row) pieces of 64 px, with
  a comment giving each piece's offset.
- `--bg auto` (default) keys out the border's main colour; also `black`, `none` or `#rrggbb`.
  `--tol N` (34) is how far per channel still counts as background.
- `--colors N` (15), `--outline` (1 px, darkest colour), `--dither 0.3` (ordered, 0..1).
- `--append CART` refuses name clashes and re-checks the whole file before writing.
  `--pal NAME` maps onto one of the cart's existing palettes instead of making a new one.
- Same input, same output. PNG only.

Generate on a flat background (black or one solid colour), subject centred and not touching
the edges. Small sprites read best from simple, high-contrast images.

## Limits

| | |
|---|---|
| palettes | 16 × 16 entries (first entry clear) |
| sprites | 1–64 px per side; 128 drawn per frame (extras dropped, `check` warns) |
| tiles | 16 × 16; 16 distinct flag names |
| maps | up to 256 × 256 tiles; 4 layers on screen |
| names | one namespace for sprites/clips/tiles/maps; palettes and sounds have their own |

Reference: [API.md → assets.cw](../API.md#assetscw)
