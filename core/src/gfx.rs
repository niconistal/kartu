//! The Kartu picture unit: 320×240, 4 tile layers, up to 128 sprites, 512 colours
//! of CRAM (16 BG palettes + 16 sprite palettes × 16). The framebuffer holds CRAM indices,
//! not RGB, so a frame hash is identical on every target no matter the output format.

use crate::assets::{Assets, Image};
use crate::font::FONT8;

pub const W: usize = 320;
pub const H: usize = 240;
pub const LAYERS: usize = 4;
pub const MAX_SPRITES: usize = 128;
/// CRAM index base of the sprite palettes.
pub const SPR_BANK: u16 = 256;

pub const FLIP_X: u8 = 1;
pub const FLIP_Y: u8 = 2;

#[derive(Clone, Default)]
pub struct Layer {
    /// size in tiles; the layer wraps in both directions
    pub w: usize,
    pub h: usize,
    /// tile id + 1; 0 = empty
    pub cells: Vec<u16>,
    pub sx: i32,
    pub sy: i32,
    pub visible: bool,
    /// pinned to the screen: `camera()` leaves its scroll alone (HUD layers)
    pub fixed: bool,
}

#[derive(Clone, Copy)]
pub enum Draw {
    Img { img: u16, flags: u8, pal: u8 },
    /// `col` is a CRAM index; `fill` false = 1 px outline
    Rect { w: i32, h: i32, col: u16, fill: bool },
}

#[derive(Clone, Copy)]
pub struct SpriteCmd {
    pub draw: Draw,
    pub x: i32,
    pub y: i32,
    /// drawn right after BG layer `layer` (3 = in front of everything)
    pub layer: u8,
}

/// Text line height at scale 1 (8 px glyph + 2 px gap).
pub const LINE_H: i32 = 10;

pub struct TextCmd {
    /// (line, x) — already wrapped and aligned
    pub lines: Vec<(String, i32)>,
    pub y: i32,
    pub col: u16,
    pub scale: i32,
    pub shadow: Option<u16>,
    /// box behind the whole block: x, y, w, h, colour
    pub bg: Option<(i32, i32, i32, i32, u16)>,
}

pub struct Gfx {
    pub cram: [[u8; 3]; 512],
    pub backdrop: [u8; 3],
    pub layers: [Layer; LAYERS],
    pub sprites: Vec<SpriteCmd>,
    pub texts: Vec<TextCmd>,
    /// sprites asked for beyond MAX_SPRITES this frame (dropped, reported to the cart dev)
    pub dropped: u32,
    pub fb: Vec<u16>,
}

/// CRAM index reserved for the backdrop (BG palette 0, entry 0 is transparent anyway).
const BACKDROP: u16 = 0;

impl Gfx {
    pub fn new(assets: &Assets) -> Gfx {
        let mut g = Gfx {
            cram: [[0; 3]; 512],
            backdrop: [0, 0, 0],
            layers: Default::default(),
            sprites: Vec::with_capacity(MAX_SPRITES),
            texts: vec![],
            dropped: 0,
            fb: vec![0; W * H],
        };
        for (pi, p) in assets.palettes.iter().enumerate() {
            for (ci, c) in p.colors.iter().enumerate() {
                g.cram[pi * 16 + ci] = *c;
                g.cram[SPR_BANK as usize + pi * 16 + ci] = *c;
            }
        }
        g
    }

    pub fn push_sprite(&mut self, s: SpriteCmd) {
        if self.sprites.len() < MAX_SPRITES {
            self.sprites.push(s);
        } else {
            self.dropped += 1;
        }
    }

    /// Compose the frame into `fb`, then clear the per-frame sprite and text lists.
    pub fn render(&mut self, a: &Assets) {
        self.cram[BACKDROP as usize] = self.backdrop;
        self.fb.fill(BACKDROP);
        for l in 0..LAYERS {
            if self.layers[l].visible && !self.layers[l].cells.is_empty() {
                draw_layer(&mut self.fb, &self.layers[l], &a.tiles);
            }
            for i in 0..self.sprites.len() {
                let s = self.sprites[i];
                if s.layer as usize != l {
                    continue;
                }
                match s.draw {
                    Draw::Img { img, flags, pal } => draw_image(&mut self.fb, &a.sprites[img as usize], s.x, s.y, flags, SPR_BANK + pal as u16 * 16),
                    Draw::Rect { w, h, col, fill: true } => self.fill_rect(s.x, s.y, w, h, col),
                    Draw::Rect { w, h, col, fill: false } => {
                        self.fill_rect(s.x, s.y, w, 1, col);
                        self.fill_rect(s.x, s.y + h - 1, w, 1, col);
                        self.fill_rect(s.x, s.y, 1, h, col);
                        self.fill_rect(s.x + w - 1, s.y, 1, h, col);
                    }
                }
            }
        }
        for t in std::mem::take(&mut self.texts) {
            if let Some((x, y, w, h, c)) = t.bg {
                self.fill_rect(x, y, w, h, c);
            }
            for (i, (line, x)) in t.lines.iter().enumerate() {
                let y = t.y + i as i32 * LINE_H * t.scale;
                if let Some(sc) = t.shadow {
                    self.text_scaled(line, x + t.scale, y + t.scale, sc, t.scale);
                }
                self.text_scaled(line, *x, y, t.col, t.scale);
            }
        }
        self.sprites.clear();
    }

    /// Draw 8×8 text straight into the framebuffer (also used by hosts for HUDs).
    pub fn text(&mut self, s: &str, x: i32, y: i32, col: u16) {
        self.text_scaled(s, x, y, col, 1);
    }

    /// Text with each font pixel drawn `k`×`k`.
    pub fn text_scaled(&mut self, s: &str, x: i32, y: i32, col: u16, k: i32) {
        let (mut cx, mut y) = (x, y);
        for ch in s.chars() {
            if ch == '\n' {
                cx = x;
                y += LINE_H * k;
                continue;
            }
            let c = ch as u32;
            let g = if (32..127).contains(&c) { &FONT8[(c - 32) as usize] } else { &FONT8[('?' as u32 - 32) as usize] };
            for (ry, bits) in g.iter().enumerate() {
                for rx in 0..8 {
                    if bits & (0x80 >> rx) == 0 {
                        continue;
                    }
                    let (px, py) = (cx + rx * k, y + ry as i32 * k);
                    if k == 1 {
                        if (0..W as i32).contains(&px) && (0..H as i32).contains(&py) {
                            self.fb[py as usize * W + px as usize] = col;
                        }
                    } else {
                        self.fill_rect(px, py, k, k, col);
                    }
                }
            }
            cx += 8 * k;
        }
    }

    /// Box behind text so HUDs stay readable over any scene.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, col: u16) {
        for py in y.max(0)..(y + h).min(H as i32) {
            for px in x.max(0)..(x + w).min(W as i32) {
                self.fb[py as usize * W + px as usize] = col;
            }
        }
    }

    /// 512-entry lookup from CRAM index to a host pixel, built once per frame.
    pub fn lut(&self, pack: impl Fn([u8; 3]) -> u32) -> [u32; 512] {
        let mut l = [0u32; 512];
        for (i, c) in self.cram.iter().enumerate() {
            l[i] = pack(*c);
        }
        l
    }

    /// FNV-1a over the CRAM-index framebuffer plus the CRAM itself: equal hashes mean
    /// equal pictures, independent of the host's pixel format.
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let mut eat = |b: u8| {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        };
        for v in &self.fb {
            eat(*v as u8);
            eat((*v >> 8) as u8);
        }
        for c in &self.cram {
            c.iter().for_each(|b| eat(*b));
        }
        h
    }
}

fn draw_layer(fb: &mut [u16], l: &Layer, tiles: &[Image]) {
    let pw = (l.w * 16) as i32;
    let ph = (l.h * 16) as i32;
    let x0 = l.sx.rem_euclid(pw) as usize;
    for y in 0..H {
        let my = (y as i32 + l.sy).rem_euclid(ph) as usize;
        let row = &l.cells[(my / 16) * l.w..(my / 16 + 1) * l.w];
        let py = my % 16;
        let out = &mut fb[y * W..(y + 1) * W];
        let mut x = 0;
        let mut mx = x0;
        while x < W {
            let px = mx % 16;
            let span = (16 - px).min(W - x);
            let c = row[mx / 16];
            if c != 0 {
                let t = &tiles[c as usize - 1];
                let base = t.pal as u16 * 16;
                let src = &t.pix[py * 16 + px..py * 16 + px + span];
                for (o, &v) in out[x..x + span].iter_mut().zip(src) {
                    if v != 0 {
                        *o = base + v as u16;
                    }
                }
            }
            x += span;
            mx += span;
            if mx >= pw as usize {
                mx -= pw as usize;
            }
        }
    }
}

fn draw_image(fb: &mut [u16], img: &Image, x: i32, y: i32, flags: u8, base: u16) {
    let (w, h) = (img.w as i32, img.h as i32);
    let x0 = x.max(0);
    let x1 = (x + w).min(W as i32);
    let y0 = y.max(0);
    let y1 = (y + h).min(H as i32);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for py in y0..y1 {
        let mut sy = py - y;
        if flags & FLIP_Y != 0 {
            sy = h - 1 - sy;
        }
        let src = &img.pix[(sy * w) as usize..((sy + 1) * w) as usize];
        let out = &mut fb[py as usize * W..(py as usize + 1) * W];
        if flags & FLIP_X == 0 {
            let s = &src[(x0 - x) as usize..(x1 - x) as usize];
            for (o, &v) in out[x0 as usize..x1 as usize].iter_mut().zip(s) {
                if v != 0 {
                    *o = base + v as u16;
                }
            }
        } else {
            for px in x0..x1 {
                let v = src[(w - 1 - (px - x)) as usize];
                if v != 0 {
                    out[px as usize] = base + v as u16;
                }
            }
        }
    }
}
