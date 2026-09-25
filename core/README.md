# kartu-core

The core of **Kartu**, a 16-bit fantasy console designed to be written by AI as easily as by
people. One deterministic machine, compiled for every screen: desktop, the browser
(WebAssembly) and ARM retro handhelds.

- **320×240**, 4 tile layers, 128 sprites, 512-colour palette RAM, 8×8 text
- **Lua 5.4** carts in a sandbox: no `io`/`os`/`load`/`require`, 32 MB heap, an instruction
  budget per frame (a runaway loop becomes a readable on-screen error, not a hang)
- **Text-native assets**: sprites, maps and music are plain text in one `assets.cw` file,
  so a language model can read, write and patch them. Every error names the line.
- **Sound**: an 8-voice synth with MML songs and parametric sound effects, with built-in presets
- **Deterministic**: same seed + same inputs ⇒ the same frame hash and audio hash on x86,
  ARM and WASM, which makes carts replayable and testable by bots
- **Kits**: `kit("topdown")` gives a whole top-down adventure (rooms, enemies, pickups,
  doors, NPCs, HUD) from one config table

This crate is the engine only: no window, no audio device, no files. A host hands in button
bits, calls `step()` 60 times a second, and shows the framebuffer.

## A cart

`assets.cw`:

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

`main.lua`:

```lua
x = 156
function update()
  if btn("left") then x = x - 2 end
  if btn("right") then x = x + 2 end
end
function draw()
  text("HELLO, KARTU", 112, 40, "w")
  spr("star", x, 116)
end
```

## Hosting it

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

Run the example, which renders a frame to a PPM image:

```sh
cargo run -p kartu-core --example hello -- hello.ppm
```

## Licence

MIT. The built-in 8×8 font is [font8x8](https://github.com/dhepper/font8x8) (public domain).
Lua is bundled through [mlua](https://github.com/mlua-rs/mlua) (MIT).
