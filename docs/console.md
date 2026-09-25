# The console

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  kartu-core  (Rust, MIT)                        one machine  │
│  assets.rs  text-native assets → palettes, sprites, maps     │
│  gfx.rs     picture unit: 4 layers, 128 sprites, CRAM, text  │
│  audio.rs   8-voice synth, MML                               │
│  world.rs   collision, hit boxes, spawn markers              │
│  lib.rs     Lua 5.4 sandbox + the console API; kits          │
│  inspect.rs state dumps and watches (runner only)            │
└──────────────┬───────────────────┬───────────────────┬───────┘
               │                   │                   │
        kartu (player)      web player (wasm)     fbdev + OSS
   check · run · bots       canvas + keyboard      Miyoo Mini,
   playtest · art import    + touch pad            any Linux fb
               │
        kartu-maker (AGPL)
   kartu-mcp · new/setup/web · templates · AGENTS.md
```

`kartu-core` is a library with no window, no audio device and no files. A host does three
things: hands in button bits, calls `step()` sixty times a second, and shows the framebuffer
(and plays the audio buffer if it has speakers). That is why the same core runs natively, in
the browser through Emscripten, and on an ARM handheld's framebuffer.

## Embedding the core

```rust
use kartu_core::{Console, BUTTONS};

let mut c = Console::new(&assets_src, &main_lua, /* seed */ 1)?;
loop {
    c.step(buttons);                       // bit i = BUTTONS[i]: left right up down a b x y l r start select
    let s = c.st.borrow();
    let lut = s.gfx.lut(|[r, g, b]| u32::from_be_bytes([0, r, g, b]));
    for (px, &i) in screen.iter_mut().zip(&s.gfx.fb) {
        *px = lut[i as usize];             // 320×240, row-major
    }
    speaker.push(&s.audio.out);            // 400 samples/frame, 24 kHz mono i16
    if let Some(e) = &c.error { /* the console already drew it on screen */ }
}
```

`cargo run -p kartu-core --example hello -- hello.ppm` renders one frame to a PPM file. A
desktop window player (SDL, winit, …) is a wanted contribution; the example shows everything it
needs.

## The picture unit

- **320 × 240** at a fixed 60 fps. Chosen because it is an exact ×2 on the 640 × 480 panels of
  the handhelds we target, and 4:3 letterboxes gracefully on everything else.
- **Colour RAM**: 16 background palettes + 16 sprite palettes × 16 entries = 512 colours on
  screen from 15-bit master colours. The framebuffer stores palette indices, not colours,
  which is what makes it target-independent.
- **4 tile layers** of 16 × 16 tiles, each with its own scroll, wrapping in both directions.
  `camera()` moves them all (except `bg.fixed` layers, for HUDs); `bg.scroll` moves one, for
  parallax.
- **128 sprites** per frame, up to 64 px, flip X/Y, palette override, drawn just after the
  layer of your choice; later calls draw on top within a layer.
- **Text**: 8 × 8 glyphs from the public-domain font8x8, drawn on top of everything, with
  alignment, wrap, boxes, shadow and integer scale.

There are no line, circle or pixel primitives, no per-line scroll and no 8 × 8 tiles yet.

## The sandbox

Carts run in Lua 5.4 through [mlua](https://github.com/mlua-rs/mlua) with `io`, `os`, `load`
and `require` removed, a 32 MB heap and an instruction budget per frame (about 4 million
instructions for `update` + `draw`). A frame that runs over budget is stopped and the error is
drawn on screen with its line; any runtime error freezes the cart on a readable red error
screen. Nothing hangs, and nothing reaches the file system, the network or the clock.

## Determinism

Same seed + same inputs ⇒ the same frames and the same samples, bit for bit, on x86, ARM and
WebAssembly. This is the property everything else stands on: replay scripts, saves, the bots,
the web player's *Verify* button and the whole playtest story.

How: `rnd` is seeded; there is no clock; `sin`/`cos` are a polynomial rather than the C `libm`
(which differs across targets); the framebuffer holds palette indices; and the synth uses its
own `exp2`/`sin` so audio is deterministic too. `Console::hash()` and `Console::audio_hash()`
expose it.

## Limits, and why

| limit | value | why |
|---|---|---|
| source size | unlimited | golfed code is where models make mistakes; we limit runtime instead |
| instructions / frame | ~4 M | keeps a runaway loop from hanging a handheld; plenty for a 320×240 game |
| Lua heap | 32 MB | fits the smallest handheld we care about with room to spare |
| sprites / frame | 128 | forces scenes to stay readable |
| palettes | 16 × 16 | palettes are the rails that keep art coherent |
| map | 256 × 256 tiles | a big world, still one text file |
| buttons | 12 | anything that plays in the browser plays on every handheld |

## Cart format version

`kartu.toml` carries `format = 1`. A newer Kartu reads older carts; an older Kartu refuses a
newer format with a clear message. Format changes come with a migration and a changelog entry.

## Crates

| crate | licence | |
|---|---|---|
| `kartu-core` | MIT | the console. `mlua` (vendored Lua 5.4) is its only dependency |
| `kartu` | MIT | the runner, web player, framebuffer player, art import |
| `kartu-maker` | AGPL-3.0 | `kartu-mcp`, `new`/`setup`/`web`, templates, the `AGENTS.md` skill |

`kartu-core` builds with `cargo package -p kartu-core`; publishing to crates.io is on the list.
