# Changelog

## 0.1.0 (unreleased)

First public release of the Kartu core.

- Picture unit: 320×240, 4 wrapping tile layers (16×16 tiles), 128 sprites, 512-colour CRAM
  (16 BG + 16 sprite palettes × 16), 8×8 text.
- Text-native assets (`assets.cw`): palettes, sprites, tiles, clips, maps with legends, tile
  flags, hit boxes, spawn markers, derived sprites (flip/rotate/recolour), instruments, sfx, songs.
- Sandboxed Lua 5.4 console API: sprites, layers, camera, collision (`move`, `overlap`,
  `solid`), text and rects, input, seeded random, deterministic trig, sound.
- Synth: 8 voices at 24 kHz, square/triangle/saw/sine/noise/pluck/FM, MML songs, built-in
  instrument, sfx and song presets.
- Built-in `topdown` kit (`kit("topdown")`): a whole top-down adventure from one config table.
- Determinism: frame and audio hashes match across x86, ARM and WASM.
