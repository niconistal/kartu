//! Text-native assets. One `assets.cw` file per cart, line based, meant to be
//! read and patched by an AI as easily as by a person:
//!
//! ```text
//! palette hero            -- up to 16 entries; the FIRST entry is always transparent
//!   . clear
//!   k #1a1c2c ink
//!   O #f4a261 orange
//! sprite cat 16x16 pal=hero
//! ................        -- exactly H rows of W palette chars
//! tile grass 16x16 pal=field
//! clip cat_walk fps=8 cat_walk.1 cat_walk.2
//! map level 40x15
//! legend # brick  g grass  . empty
//! ####....                -- exactly H rows of W legend chars
//! ```
//!
//! Every error names the line, because the reader of the error is usually a model.

use std::collections::HashMap;

pub const MAX_PALETTES: usize = 16;

#[derive(Clone, Debug)]
pub struct Palette {
    pub name: String,
    pub chars: Vec<char>,
    /// 15-bit master colour, already expanded to 8-bit channels.
    pub colors: Vec<[u8; 3]>,
}

#[derive(Clone, Debug)]
pub struct Image {
    pub name: String,
    pub w: usize,
    pub h: usize,
    pub pal: u8,
    /// 0 = transparent, 1..=15 = palette entry.
    pub pix: Vec<u8>,
    /// sprites: collision box (x, y, w, h) relative to the top-left, from `hit=x,y,w,h`
    pub hit: Option<[i32; 4]>,
    /// tiles: bit i = `Assets::flag_names[i]`, from `flags=solid,door`
    pub flags: u16,
}

#[derive(Clone, Debug)]
pub struct Spawn {
    pub kind: String,
    pub tx: usize,
    pub ty: usize,
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub name: String,
    pub fps: u32,
    pub frames: Vec<u16>,
}

#[derive(Clone, Debug)]
pub struct Map {
    pub name: String,
    pub w: usize,
    pub h: usize,
    /// tile id + 1; 0 = empty.
    pub cells: Vec<u16>,
    /// `@kind` markers from the legend, in reading order
    pub spawns: Vec<Spawn>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ref {
    Sprite(u16),
    Clip(u16),
    Tile(u16),
    Map(u16),
}

#[derive(Default, Debug)]
pub struct Assets {
    pub palettes: Vec<Palette>,
    pub sprites: Vec<Image>,
    pub tiles: Vec<Image>,
    pub clips: Vec<Clip>,
    pub maps: Vec<Map>,
    /// sprites, clips, tiles and maps share one namespace
    pub names: HashMap<String, Ref>,
    /// palettes have their own, so `palette car` + `sprite car` is fine
    pub pal_names: HashMap<String, u8>,
    /// tile flag names in first-use order (max 16)
    pub flag_names: Vec<String>,
    /// instruments, sound effects and songs (built-in presets + the cart's own)
    pub sound: crate::audio::Bank,
}

/// Snap an 8-bit colour to the console's 15-bit master palette.
pub fn to15(c: [u8; 3]) -> [u8; 3] {
    c.map(|v| {
        let v5 = v >> 3;
        (v5 << 3) | (v5 >> 2)
    })
}

pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(to15([(v >> 16) as u8, (v >> 8) as u8, v as u8]))
}

fn msg(line: usize, m: impl std::fmt::Display) -> String {
    format!("assets.cw:{}: {}", line + 1, m)
}

fn strip_comment(l: &str) -> &str {
    // `--` comments (Lua style) on keyword, palette and legend lines. Grid rows are read
    // raw, so any char is fair game there.
    if l.trim_start().starts_with("--") {
        return "";
    }
    match l.find(" --") {
        Some(i) => &l[..i],
        None => l,
    }
}

const KEYWORDS: [&str; 8] = ["palette", "sprite", "tile", "clip", "map", "instrument", "sfx", "song"];

/// A line that starts a new block (`sprite cat 8x8 ...`), used to resync after an error.
pub(crate) fn is_header(l: &str) -> bool {
    let mut w = strip_comment(l).split_whitespace();
    matches!((w.next(), w.next()), (Some(k), Some(_)) if KEYWORDS.contains(&k))
}

fn dims(s: &str, line: usize) -> Result<(usize, usize), String> {
    let (w, h) = s.split_once('x').ok_or_else(|| msg(line, format!("expected WxH, got `{s}`")))?;
    let p = |v: &str| v.parse::<usize>().map_err(|_| msg(line, format!("bad size `{s}`")));
    Ok((p(w)?, p(h)?))
}

fn kv<'a>(words: &[&'a str], key: &str) -> Option<&'a str> {
    words.iter().find_map(|w| w.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
}

/// Flip / rotate a pixel grid. `rot` is clockwise degrees (0, 90, 180, 270).
fn transform(pix: &[u8], w: usize, h: usize, fx: bool, fy: bool, rot: u32) -> (Vec<u8>, usize, usize) {
    let mut p = pix.to_vec();
    if fx {
        p = (0..w * h).map(|i| pix[(i / w) * w + (w - 1 - i % w)]).collect();
    }
    if fy {
        let q = p.clone();
        p = (0..w * h).map(|i| q[(h - 1 - i / w) * w + i % w]).collect();
    }
    let (mut w, mut h) = (w, h);
    for _ in 0..rot / 90 {
        // 90° clockwise: new (x, y) takes old (y, H-1-x)
        let (nw, nh) = (h, w);
        let q = p.clone();
        p = (0..nw * nh).map(|i| q[(h - 1 - i % nw) * w + i / nw]).collect();
        w = nw;
        h = nh;
    }
    (p, w, h)
}

/// Where box `b` of a `w`×`h` image ends up after the same flips/rotation as `transform`.
fn transform_box(b: [i32; 4], w: i32, h: i32, fx: bool, fy: bool, rot: u32) -> [i32; 4] {
    let [mut x, mut y, mut bw, mut bh] = b;
    let (mut w, mut h) = (w, h);
    if fx {
        x = w - x - bw;
    }
    if fy {
        y = h - y - bh;
    }
    for _ in 0..rot / 90 {
        (x, y, bw, bh) = (h - y - bh, x, bh, bw);
        (w, h) = (h, w);
    }
    [x, y, bw, bh]
}

type PendingMap = (usize, String, usize, usize, HashMap<char, String>, Vec<String>);

impl Assets {
    /// Parse, stopping at the first error.
    pub fn parse(src: &str) -> Result<Assets, String> {
        let (a, errs) = Assets::parse_all(src);
        match errs.into_iter().next() {
            Some(e) => Err(e),
            None => Ok(a),
        }
    }

    /// Parse everything, collecting every error instead of stopping at the first
    /// (what a runner's `check` command shows). A broken block is skipped up to the next header.
    pub fn parse_all(src: &str) -> (Assets, Vec<String>) {
        let lines: Vec<&str> = src.lines().collect();
        let mut a = Assets { sound: crate::audio::Bank::new(), ..Default::default() };
        let mut errs = vec![];
        let mut pending_clips: Vec<(usize, String, u32, Vec<String>)> = vec![];
        let mut pending_maps: Vec<PendingMap> = vec![];
        let mut i = 0;
        while i < lines.len() {
            let l = strip_comment(lines[i]).trim();
            if l.is_empty() {
                i += 1;
                continue;
            }
            let start = i;
            if let Err(e) = a.block(&lines, &mut i, &mut errs, &mut pending_clips, &mut pending_maps) {
                errs.push(e);
                i = start + 1;
                while i < lines.len() && !is_header(lines[i]) {
                    i += 1;
                }
            }
        }

        for (line, name, fps, frames) in pending_clips {
            let mut ids = vec![];
            for f in &frames {
                match a.names.get(f) {
                    Some(Ref::Sprite(id)) => ids.push(*id),
                    _ => errs.push(msg(line, format!("clip `{name}`: `{f}` is not a sprite"))),
                }
            }
            if frames.is_empty() {
                errs.push(msg(line, format!("clip `{name}` has no frames")));
            }
            if ids.is_empty() {
                continue;
            }
            if a.names.contains_key(&name) {
                errs.push(msg(line, format!("name `{name}` is already used")));
                continue;
            }
            a.names.insert(name.clone(), Ref::Clip(a.clips.len() as u16));
            a.clips.push(Clip { name, fps, frames: ids });
        }
        for (line, name, w, h, legend, rows) in pending_maps {
            let mut cells = Vec::with_capacity(w * h);
            let mut spawns = vec![];
            let mut bad = std::collections::BTreeSet::new();
            for (r, row) in rows.iter().enumerate() {
                for ch in row.chars() {
                    let t = match legend.get(&ch) {
                        Some(t) => t,
                        None => {
                            if bad.insert(ch) {
                                errs.push(msg(line + r + 1, format!("map `{name}`: char `{ch}` has no legend entry")));
                            }
                            cells.push(0);
                            continue;
                        }
                    };
                    // `@bat` = spawn marker on an empty cell, `@bat:floor` = marker on a floor tile
                    let t = match t.strip_prefix('@') {
                        Some(m) => {
                            let (kind, under) = m.split_once(':').unwrap_or((m, ""));
                            spawns.push(Spawn { kind: kind.to_string(), tx: (cells.len() % w.max(1)), ty: r });
                            under
                        }
                        None => t.as_str(),
                    };
                    if t.is_empty() {
                        cells.push(0);
                    } else {
                        match a.names.get(t) {
                            Some(Ref::Tile(id)) => cells.push(id + 1),
                            _ => {
                                if bad.insert(ch) {
                                    errs.push(msg(line, format!("map `{name}`: legend `{ch}` → `{t}` is not a tile")));
                                }
                                cells.push(0);
                            }
                        }
                    }
                }
            }
            if a.names.contains_key(&name) {
                errs.push(msg(line, format!("name `{name}` is already used")));
                continue;
            }
            a.names.insert(name.clone(), Ref::Map(a.maps.len() as u16));
            a.maps.push(Map { name, w, h, cells, spawns });
        }
        errs.sort_by_key(|e| e.split(':').nth(1).and_then(|n| n.parse::<usize>().ok()).unwrap_or(0));
        (a, errs)
    }

    /// One block starting at `lines[*i]`. `Err` = the header itself is unusable (skip the
    /// block); row-level problems go to `errs` and parsing carries on.
    fn block(
        &mut self,
        lines: &[&str],
        i: &mut usize,
        errs: &mut Vec<String>,
        pending_clips: &mut Vec<(usize, String, u32, Vec<String>)>,
        pending_maps: &mut Vec<PendingMap>,
    ) -> Result<(), String> {
        let a = self;
        let l = strip_comment(lines[*i]).trim();
        let words: Vec<&str> = l.split_whitespace().collect();
        let start = *i;
        match words[0] {
            "palette" => {
                let name = words.get(1).ok_or_else(|| msg(start, "palette needs a name"))?.to_string();
                if a.palettes.len() >= MAX_PALETTES {
                    return Err(msg(start, "more than 16 palettes"));
                }
                if a.pal_names.contains_key(&name) {
                    return Err(msg(start, format!("palette `{name}` is declared twice")));
                }
                let mut p = Palette { name: name.clone(), chars: vec![], colors: vec![] };
                *i += 1;
                while *i < lines.len() {
                    let e = strip_comment(lines[*i]).trim();
                    let ew: Vec<&str> = e.split_whitespace().collect();
                    if ew.len() < 2 || ew[0].chars().count() != 1 || is_header(lines[*i]) {
                        break;
                    }
                    let ch = ew[0].chars().next().unwrap();
                    let col = if ew[1] == "clear" {
                        Some([0, 0, 0])
                    } else {
                        parse_hex(ew[1])
                    };
                    match col {
                        None => errs.push(msg(*i, format!("palette `{name}`: `{}` is not a colour (#rrggbb or clear)", ew[1]))),
                        Some(_) if p.chars.len() >= 16 => errs.push(msg(*i, format!("palette `{name}` has more than 16 colours"))),
                        Some(_) if p.chars.contains(&ch) => errs.push(msg(*i, format!("char `{ch}` used twice in palette `{name}`"))),
                        Some(c) => {
                            if p.chars.is_empty() && ew[1] != "clear" {
                                errs.push(msg(*i, format!("palette `{name}`: the first entry must be `clear` (it is the transparent one)")));
                            }
                            p.chars.push(ch);
                            p.colors.push(c);
                        }
                    }
                    *i += 1;
                }
                if p.chars.len() < 2 {
                    errs.push(msg(start, format!("palette `{name}` needs a transparent entry plus at least one colour")));
                }
                a.pal_names.insert(name, a.palettes.len() as u8);
                a.palettes.push(p);
            }
            kind @ ("sprite" | "tile") => {
                let name = words.get(1).ok_or_else(|| msg(start, format!("{kind} needs a name")))?.to_string();
                *i += 1;
                if let Some(src) = kv(&words, "from") {
                    // derived: `sprite cat_l from=cat fx [fy] [rot=90] [pal=other]`, no rows
                    let base = match (kind, a.names.get(src)) {
                        (_, Some(Ref::Sprite(id))) => a.sprites[*id as usize].clone(),
                        (_, Some(Ref::Tile(id))) => a.tiles[*id as usize].clone(),
                        _ => return Err(msg(start, format!("{kind} `{name}`: from=`{src}` is not a sprite or tile declared above"))),
                    };
                    let rot: u32 = match kv(&words, "rot").unwrap_or("0") {
                        r @ ("0" | "90" | "180" | "270") => r.parse().unwrap(),
                        r => return Err(msg(start, format!("rot=`{r}`: use 90, 180 or 270 (clockwise)"))),
                    };
                    let (fx, fy) = (words.contains(&"fx"), words.contains(&"fy"));
                    let (pix, w, h) = transform(&base.pix, base.w, base.h, fx, fy, rot);
                    let hit = base.hit.map(|b| transform_box(b, base.w as i32, base.h as i32, fx, fy, rot));
                    let (hit, flags) = a.attrs(&words, kind, w, h, start, hit, base.flags)?;
                    if kind == "tile" && (w, h) != (16, 16) {
                        return Err(msg(start, "tiles are 16x16"));
                    }
                    let pal = match kv(&words, "pal") {
                        None => base.pal,
                        Some(pn) => {
                            let pi = *a.pal_names.get(pn).ok_or_else(|| msg(start, format!("unknown palette `{pn}` (declare it above)")))?;
                            let n = a.palettes[pi as usize].chars.len();
                            if let Some(&m) = pix.iter().max().filter(|&&m| m as usize >= n) {
                                errs.push(msg(start, format!("{kind} `{name}`: palette `{pn}` has {n} entries but `{src}` uses entry {m}")));
                            }
                            pi
                        }
                    };
                    return a.add_image(kind, Image { name, w, h, pal, pix, hit, flags }, start);
                }
                let (w, h) = dims(words.get(2).copied().unwrap_or(""), start)?;
                if kind == "tile" && (w, h) != (16, 16) {
                    return Err(msg(start, "tiles are 16x16"));
                }
                if kind == "sprite" && (!(1..=64).contains(&w) || !(1..=64).contains(&h)) {
                    return Err(msg(start, "sprites are 1..64 px on each side"));
                }
                let (hit, flags) = a.attrs(&words, kind, w, h, start, None, 0)?;
                let pname = kv(&words, "pal").ok_or_else(|| msg(start, format!("{kind} `{name}` needs pal=<palette>")))?;
                let pi = *a.pal_names.get(pname).ok_or_else(|| msg(start, format!("unknown palette `{pname}` (declare it above)")))?;
                let pal = &a.palettes[pi as usize];
                let mut pix = Vec::with_capacity(w * h);
                let mut bad = std::collections::BTreeSet::new();
                for r in 0..h {
                    let row = lines.get(*i).map(|s| s.trim()).unwrap_or("");
                    if is_header(row) || *i >= lines.len() {
                        errs.push(msg(*i, format!("{kind} `{name}` has only {r} rows, expected {h}")));
                        pix.resize(w * h, 0);
                        break;
                    }
                    *i += 1;
                    let n = row.chars().count();
                    if n != w {
                        errs.push(msg(*i - 1, format!("{kind} `{name}` row {} is {n} chars wide, expected {w}", r + 1)));
                    }
                    for ch in row.chars().chain(std::iter::repeat('\0')).take(w) {
                        match pal.chars.iter().position(|&c| c == ch) {
                            Some(ix) => pix.push(ix as u8),
                            None => {
                                if ch != '\0' && bad.insert(ch) {
                                    errs.push(msg(*i - 1, format!("char `{ch}` is not in palette `{pname}` ({})", pal.chars.iter().collect::<String>())));
                                }
                                pix.push(0);
                            }
                        }
                    }
                }
                a.add_image(kind, Image { name, w, h, pal: pi, pix, hit, flags }, start)?;
            }
            "clip" => {
                let name = words.get(1).ok_or_else(|| msg(start, "clip needs a name"))?.to_string();
                let fps = kv(&words, "fps").and_then(|v| v.parse().ok()).unwrap_or(8u32).clamp(1, 60);
                let frames = words[2..].iter().filter(|w| !w.contains('=')).map(|s| s.to_string()).collect();
                pending_clips.push((start, name, fps, frames));
                *i += 1;
            }
            "map" => {
                let name = words.get(1).ok_or_else(|| msg(start, "map needs a name"))?.to_string();
                let (w, h) = dims(words.get(2).copied().unwrap_or(""), start)?;
                if w > 256 || h > 256 {
                    return Err(msg(start, "maps are at most 256x256 tiles"));
                }
                *i += 1;
                let mut legend = HashMap::new();
                legend.insert('.', String::new());
                while let Some(l) = lines.get(*i).map(|s| strip_comment(s).trim()) {
                    let Some(rest) = l.strip_prefix("legend") else { break };
                    let toks: Vec<&str> = rest.split_whitespace().collect();
                    for pair in toks.chunks(2) {
                        let [k, v] = pair else {
                            errs.push(msg(*i, "legend is `<char> <tile>` pairs"));
                            break;
                        };
                        let mut kc = k.chars();
                        let (Some(c), None) = (kc.next(), kc.next()) else {
                            errs.push(msg(*i, format!("legend key `{k}` must be one char")));
                            continue;
                        };
                        legend.insert(c, if *v == "empty" { String::new() } else { v.to_string() });
                    }
                    *i += 1;
                }
                let mut rows = vec![];
                for r in 0..h {
                    let row = lines.get(*i).map(|s| s.trim()).unwrap_or("");
                    if is_header(row) || *i >= lines.len() {
                        errs.push(msg(*i, format!("map `{name}` has only {r} rows, expected {h}")));
                        break;
                    }
                    let n = row.chars().count();
                    if n != w {
                        errs.push(msg(*i, format!("map `{name}` row {} is {n} chars wide, expected {w}", r + 1)));
                    }
                    rows.push(row.chars().chain(std::iter::repeat('.')).take(w).collect());
                    *i += 1;
                }
                rows.resize(h, ".".repeat(w));
                pending_maps.push((start, name, w, h, legend, rows));
            }
            "instrument" => {
                let name = words.get(1).ok_or_else(|| msg(start, "instrument needs a name"))?.to_string();
                *i += 1;
                a.sound.instrument(&name, &words[2..]).map_err(|e| msg(start, format!("instrument `{name}`: {e}")))?;
            }
            "sfx" => {
                let name = words.get(1).ok_or_else(|| msg(start, "sfx needs a name"))?.to_string();
                *i += 1;
                // the raw rest of the line: notes="..." may hold spaces (and `--` only
                // starts a comment outside the quotes)
                let raw = lines[start].trim_start();
                let rest = raw.splitn(3, char::is_whitespace).nth(2).unwrap_or("");
                let mut q = false;
                let mut cut = rest.len();
                for (k, ch) in rest.char_indices() {
                    if ch == '"' {
                        q = !q;
                    } else if !q && rest[k..].starts_with(" --") {
                        cut = k;
                        break;
                    }
                }
                let def = &rest[..cut];
                a.sound.sfx_def(&name, def).map_err(|e| msg(start, format!("sfx `{name}`: {e}")))?;
            }
            "song" => {
                a.sound.song_def(lines, i).map_err(|(l, e)| msg(l, e))?;
            }
            other => {
                *i += 1;
                return Err(msg(start, format!("unknown keyword `{other}` (palette, sprite, tile, clip, map, instrument, sfx, song)")));
            }
        }
        Ok(())
    }

    /// `hit=x,y,w,h` (sprites) and `flags=a,b` (tiles) on a sprite/tile header; derived
    /// images inherit both unless they say otherwise.
    #[allow(clippy::too_many_arguments)]
    fn attrs(&mut self, words: &[&str], kind: &str, w: usize, h: usize, line: usize, hit: Option<[i32; 4]>, flags: u16) -> Result<(Option<[i32; 4]>, u16), String> {
        let mut out = (hit, flags);
        if let Some(v) = kv(words, "hit") {
            if kind == "tile" {
                return Err(msg(line, "hit= is for sprites; tiles collide by flags (flags=solid)"));
            }
            let n: Vec<i32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            let [x, y, bw, bh] = n[..] else {
                return Err(msg(line, format!("hit=`{v}`: want hit=x,y,w,h (box inside the sprite)")));
            };
            if bw <= 0 || bh <= 0 || x < 0 || y < 0 || x + bw > w as i32 || y + bh > h as i32 {
                return Err(msg(line, format!("hit=`{v}` must fit inside the {w}x{h} sprite")));
            }
            out.0 = Some([x, y, bw, bh]);
        }
        if let Some(v) = kv(words, "flags") {
            if kind == "sprite" {
                return Err(msg(line, "flags= is for tiles (sprites use hit=)"));
            }
            out.1 = 0;
            for f in v.split(',').filter(|f| !f.is_empty()) {
                let i = match self.flag_names.iter().position(|x| x == f) {
                    Some(i) => i,
                    None if self.flag_names.len() < 16 => {
                        self.flag_names.push(f.to_string());
                        self.flag_names.len() - 1
                    }
                    None => return Err(msg(line, "more than 16 different tile flags")),
                };
                out.1 |= 1 << i;
            }
        }
        Ok(out)
    }

    pub fn flag_bit(&self, name: &str) -> Option<u16> {
        self.flag_names.iter().position(|x| x == name).map(|i| 1 << i)
    }

    fn add_image(&mut self, kind: &str, img: Image, line: usize) -> Result<(), String> {
        if self.names.contains_key(&img.name) {
            return Err(msg(line, format!("name `{}` is already used", img.name)));
        }
        if kind == "tile" {
            self.names.insert(img.name.clone(), Ref::Tile(self.tiles.len() as u16));
            self.tiles.push(img);
        } else {
            self.names.insert(img.name.clone(), Ref::Sprite(self.sprites.len() as u16));
            self.sprites.push(img);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_kinds() {
        let a = Assets::parse(
            "palette p\n . clear\n k #102030 ink\nsprite s 2x2 pal=p\n.k\nk.\ntile t 16x16 pal=p\n".to_string().as_str(),
        );
        assert!(a.is_err(), "tile rows missing must fail");
        let mut src = String::from("palette p\n . clear\n k #102030\nsprite s1 2x1 pal=p\n.k\nsprite s2 2x1 pal=p\nk.\nclip c fps=4 s1 s2\ntile t 16x16 pal=p\n");
        for _ in 0..16 {
            src.push_str("kkkkkkkkkkkkkkkk\n");
        }
        src.push_str("map m 3x1\nlegend # t\n#.#\n");
        let a = Assets::parse(&src).unwrap();
        assert_eq!(a.sprites[0].pix, vec![0, 1]);
        assert_eq!(a.clips[0].frames, vec![0, 1]);
        assert_eq!(a.maps[0].cells, vec![1, 0, 1]);
        assert_eq!(a.palettes[0].colors[1], to15([0x10, 0x20, 0x30]));
    }

    #[test]
    fn palettes_have_their_own_names() {
        let a = Assets::parse("palette car\n . clear\n k #102030\nsprite car 1x1 pal=car\nk\n").unwrap();
        assert_eq!(a.pal_names["car"], 0);
        assert_eq!(a.names["car"], Ref::Sprite(0));
    }

    #[test]
    fn collects_every_error() {
        let src = "palette p\n . clear\n k #000000\nsprite s 2x2 pal=p\n.z\nkkk\nsprite t 2x1 pal=nope\n..\nmap m 2x1\n#.\nwidget w\n";
        let (_, errs) = Assets::parse_all(src);
        let lines: Vec<&str> = errs.iter().map(|e| e.split(": ").next().unwrap()).collect();
        assert_eq!(lines, ["assets.cw:5", "assets.cw:6", "assets.cw:7", "assets.cw:10", "assets.cw:11"], "{errs:#?}");
    }

    #[test]
    fn derived_images() {
        let src = "palette p\n . clear\n k #000000\n r #ff0000\npalette q\n . clear\n k #ffffff\n r #00ff00\nsprite s 3x2 pal=p\nkr.\n..k\nsprite sx from=s fx\nsprite sr from=s rot=90 pal=q\n";
        let a = Assets::parse(src).unwrap();
        assert_eq!(a.sprites[1].pix, vec![0, 2, 1, 1, 0, 0]);
        let r = &a.sprites[2];
        assert_eq!((r.w, r.h, r.pal), (2, 3, 1));
        // rows of the clockwise turn: (.k) (.r) (k.)
        assert_eq!(r.pix, vec![0, 1, 0, 2, 1, 0]);
    }

    #[test]
    fn errors_name_the_line() {
        let e = Assets::parse("palette p\n . clear\n k #000000\nsprite s 2x1 pal=p\n.z\n").unwrap_err();
        assert!(e.starts_with("assets.cw:5:"), "{e}");
    }
}
