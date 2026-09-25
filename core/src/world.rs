//! Core 1.1: the world API every top-down / platform cart was writing by hand in the probe.
//! Tile flags + collision (`move`, `solid`, `touching`, `overlap`, `hitbox`), reading maps
//! back (`bg.get`, `bg.flag`, `bg.size`, `spawns`), `camera`, `rect` and `text` options.
//!
//! World coordinates are layer pixels: tile (tx, ty) covers x in [16·tx, 16·tx+16). The
//! camera maps world to screen; collision never looks at the camera.

use crate::assets::{Assets, Ref};
use crate::gfx::{self, Draw, SpriteCmd, TextCmd, LAYERS, LINE_H};
use crate::{check_layer, Console, State};
use mlua::{Lua, Table, Value};

fn rt<T>(m: impl Into<String>) -> mlua::Result<T> {
    Err(mlua::Error::runtime(m.into()))
}

/// Colour argument → CRAM index (sprite bank, so `pal()` recolours apply):
/// a number = entry of the first palette (as before), `"k"` = that char in the first
/// palette, `"hero:k"` = char `k` of palette `hero`.
pub(crate) fn colour(a: &Assets, v: &Value, what: &str) -> mlua::Result<u16> {
    let (pi, idx) = match v {
        Value::Integer(n) => (0, (*n).clamp(0, 15) as usize),
        Value::Number(n) => (0, (n.floor() as i64).clamp(0, 15) as usize),
        Value::String(s) => {
            let s = s.to_str()?.to_string();
            let (pname, ch) = match s.rsplit_once(':') {
                Some((p, c)) if !p.is_empty() => (p.to_string(), c.to_string()),
                _ => (a.palettes.first().map(|p| p.name.clone()).unwrap_or_default(), s.clone()),
            };
            let Some(&pi) = a.pal_names.get(&pname) else {
                return rt(format!("{what}: unknown palette `{pname}` in colour \"{s}\""));
            };
            let pal = &a.palettes[pi as usize];
            let mut cs = ch.chars();
            let (Some(c), None) = (cs.next(), cs.next()) else {
                return rt(format!("{what}: colour \"{s}\" should be a palette char like \"k\" or \"{pname}:k\""));
            };
            let Some(idx) = pal.chars.iter().position(|&x| x == c) else {
                return rt(format!("{what}: `{c}` is not in palette `{pname}` ({})", pal.chars.iter().collect::<String>()));
            };
            (pi as usize, idx)
        }
        _ => return rt(format!("{what}: colour is a palette char (\"k\", \"hero:k\") or entry number")),
    };
    Ok(gfx::SPR_BANK + (pi * 16 + idx) as u16)
}

/// World-space collision box of a Lua object, whole pixels: (left, top, w, h).
///   o.hit = {x, y, w, h}   box relative to (o.x, o.y)
///   o.spr = "name"         the sprite's `hit=` box, else its full size (clips: first frame)
///   o.w, o.h               box at (o.x, o.y)
pub(crate) fn hitbox(a: &Assets, o: &Table) -> mlua::Result<(i32, i32, i32, i32)> {
    let x: f64 = o.get::<Option<f64>>("x")?.unwrap_or(0.0);
    let y: f64 = o.get::<Option<f64>>("y")?.unwrap_or(0.0);
    let b = if let Some(h) = o.get::<Option<Table>>("hit")? {
        let n = |k: &str, i: i64| -> mlua::Result<f64> { Ok(h.get::<Option<f64>>(k)?.or(h.get::<Option<f64>>(i)?).unwrap_or(0.0)) };
        [n("x", 1)? as i32, n("y", 2)? as i32, n("w", 3)? as i32, n("h", 4)? as i32]
    } else if let Some(name) = o.get::<Option<String>>("spr")? {
        let img = match a.names.get(&name) {
            Some(Ref::Sprite(i)) => &a.sprites[*i as usize],
            Some(Ref::Clip(c)) => &a.sprites[a.clips[*c as usize].frames[0] as usize],
            _ => return rt(format!("hitbox: no sprite or clip called `{name}` (o.spr)")),
        };
        img.hit.unwrap_or([0, 0, img.w as i32, img.h as i32])
    } else if let (Some(w), Some(h)) = (o.get::<Option<f64>>("w")?, o.get::<Option<f64>>("h")?) {
        [0, 0, w as i32, h as i32]
    } else {
        return rt("this object has no size: give it spr=\"name\", hit={x,y,w,h} or w=,h=");
    };
    Ok(((x + b[0] as f64).floor() as i32, (y + b[1] as f64).floor() as i32, b[2].max(1), b[3].max(1)))
}

/// Flag name → bit. An explicit unknown flag is an error (a typo would silently never
/// collide); the default "solid" just matches nothing when no tile has it.
fn flag(a: &Assets, f: Option<String>) -> mlua::Result<u16> {
    match f {
        None => Ok(a.flag_bit("solid").unwrap_or(0)),
        Some(f) => match a.flag_bit(&f) {
            Some(b) => Ok(b),
            None if f == "solid" => Ok(0),
            None => rt(format!("no tile has flag `{f}` (flags in assets.cw: {})", if a.flag_names.is_empty() { "none".into() } else { a.flag_names.join(", ") })),
        },
    }
}

/// First cell with `bit` under the pixel box, over every layer that has a map, as
/// (layer, tx, ty). Cells outside a layer's map never match.
fn find(s: &State, x: i32, y: i32, w: i32, h: i32, bit: u16) -> Option<(usize, i32, i32)> {
    if bit == 0 {
        return None;
    }
    for (li, l) in s.gfx.layers.iter().enumerate() {
        if l.cells.is_empty() {
            continue;
        }
        for ty in y.div_euclid(16)..=(y + h - 1).div_euclid(16) {
            if ty < 0 || ty >= l.h as i32 {
                continue;
            }
            for tx in x.div_euclid(16)..=(x + w - 1).div_euclid(16) {
                if tx < 0 || tx >= l.w as i32 {
                    continue;
                }
                let c = l.cells[ty as usize * l.w + tx as usize];
                if c != 0 && s.assets.tiles[c as usize - 1].flags & bit != 0 {
                    return Some((li, tx, ty));
                }
            }
        }
    }
    None
}

/// Move one axis by `d`, pixel column by pixel column; stop flush against the first
/// blocking cell. Returns (new position, blocked).
#[allow(clippy::too_many_arguments)]
fn sweep(s: &State, pos: f64, d: f64, off: i32, fixed: i32, len: i32, cross: i32, horiz: bool, bit: u16) -> (f64, bool) {
    let target = pos + d;
    let from = (pos + off as f64).floor() as i32;
    let to = (target + off as f64).floor() as i32;
    let hits = |p: i32| if horiz { find(s, p, fixed, len, cross, bit).is_some() } else { find(s, fixed, p, cross, len, bit).is_some() };
    if hits(from) {
        // already inside something (spawned in a wall): let it walk out rather than stick
        return (target, false);
    }
    let step = if to > from { 1 } else { -1 };
    let mut p = from;
    while p != to {
        let n = p + step;
        if hits(n) {
            return ((p - off) as f64, true);
        }
        p = n;
    }
    (target, false)
}

impl Console {
    pub(crate) fn install_world(&mut self) -> mlua::Result<()> {
        let lua: &Lua = &self.lua;
        let g = lua.globals();
        let bg: Table = g.get("bg")?;

        // hitbox(o) -> x, y, w, h (world pixels)
        let st = self.st.clone();
        g.set(
            "hitbox",
            lua.create_function(move |_, o: Table| {
                let s = st.borrow();
                hitbox(&s.assets, &o)
            })?,
        )?;

        // overlap(a, b) -> bool: do the two objects' boxes touch?
        let st = self.st.clone();
        g.set(
            "overlap",
            lua.create_function(move |_, (a, b): (Table, Table)| {
                let s = st.borrow();
                let (ax, ay, aw, ah) = hitbox(&s.assets, &a)?;
                let (bx, by, bw, bh) = hitbox(&s.assets, &b)?;
                Ok(ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah)
            })?,
        )?;

        // move(o, dx, dy [, flag="solid"]) -> blocked_x, blocked_y. x first, then y, so
        // it slides along walls. Writes o.x / o.y.
        let st = self.st.clone();
        g.set(
            "move",
            lua.create_function(move |_, (o, dx, dy, f): (Table, f64, f64, Option<String>)| {
                let s = st.borrow();
                let bit = flag(&s.assets, f)?;
                let (x, y): (f64, f64) = (o.get("x")?, o.get("y")?);
                let (bx, by, bw, bh) = hitbox(&s.assets, &o)?;
                let (offx, offy) = (bx - x.floor() as i32, by - y.floor() as i32);
                // hitbox floors (x + off); off is whole, so floor(x) + off is the same thing
                let (nx, hx) = if dx != 0.0 { sweep(&s, x, dx, offx, by, bw, bh, true, bit) } else { (x, false) };
                let bx2 = (nx + offx as f64).floor() as i32;
                let (ny, hy) = if dy != 0.0 { sweep(&s, y, dy, offy, bx2, bh, bw, false, bit) } else { (y, false) };
                o.set("x", nx)?;
                o.set("y", ny)?;
                Ok((hx, hy))
            })?,
        )?;

        // solid(o [, flag]) or solid(x, y [, w, h] [, flag]) -> bool
        let st = self.st.clone();
        g.set(
            "solid",
            lua.create_function(move |_, args: mlua::Variadic<Value>| {
                let s = st.borrow();
                let (bx, f) = boxed_args(&s.assets, &args, "solid")?;
                let bit = flag(&s.assets, f)?;
                Ok(find(&s, bx.0, bx.1, bx.2, bx.3, bit).is_some())
            })?,
        )?;

        // touching(o, flag) -> tx, ty, layer of the first flagged cell under o, or nil
        let st = self.st.clone();
        g.set(
            "touching",
            lua.create_function(move |_, args: mlua::Variadic<Value>| {
                let s = st.borrow();
                let (bx, f) = boxed_args(&s.assets, &args, "touching")?;
                let bit = flag(&s.assets, f)?;
                Ok(match find(&s, bx.0, bx.1, bx.2, bx.3, bit) {
                    Some((l, tx, ty)) => (Some(tx), Some(ty), Some(l)),
                    None => (None, None, None),
                })
            })?,
        )?;

        // spawns(map [, kind]) -> { {kind=, x=, y=, tx=, ty=}, ... } in reading order
        let st = self.st.clone();
        g.set(
            "spawns",
            lua.create_function(move |lua, (m, kind): (String, Option<String>)| {
                let s = st.borrow();
                let Some(Ref::Map(mi)) = s.assets.names.get(&m).copied() else {
                    return rt(format!("spawns: no map called `{m}`"));
                };
                let out = lua.create_table()?;
                for sp in &s.assets.maps[mi as usize].spawns {
                    if kind.as_ref().is_some_and(|k| *k != sp.kind) {
                        continue;
                    }
                    let t = lua.create_table()?;
                    t.set("kind", sp.kind.as_str())?;
                    t.set("tx", sp.tx)?;
                    t.set("ty", sp.ty)?;
                    t.set("x", sp.tx * 16)?;
                    t.set("y", sp.ty * 16)?;
                    out.push(t)?;
                }
                Ok(out)
            })?,
        )?;

        // camera(x, y [, {clamp=layer}]) -> x, y actually used; camera() -> current
        let st = self.st.clone();
        g.set(
            "camera",
            lua.create_function(move |_, (x, y, opt): (Option<f64>, Option<f64>, Option<Table>)| {
                let mut s = st.borrow_mut();
                let (Some(x), Some(y)) = (x, y) else {
                    return Ok(s.cam);
                };
                let (mut cx, mut cy) = (x.floor() as i32, y.floor() as i32);
                if let Some(l) = opt.map(|o| o.get::<Option<usize>>("clamp")).transpose()?.flatten() {
                    let l = &s.gfx.layers[check_layer(l)?];
                    cx = cx.clamp(0, (l.w as i32 * 16 - gfx::W as i32).max(0));
                    cy = cy.clamp(0, (l.h as i32 * 16 - gfx::H as i32).max(0));
                }
                s.cam = (cx, cy);
                for l in s.gfx.layers.iter_mut().filter(|l| !l.fixed) {
                    l.sx = cx;
                    l.sy = cy;
                }
                Ok((cx, cy))
            })?,
        )?;

        // rect(x, y, w, h, col [, {line=bool, layer=0..3, screen=bool}])
        let st = self.st.clone();
        g.set(
            "rect",
            lua.create_function(move |_, (x, y, w, h, col, opt): (f64, f64, f64, f64, Value, Option<Table>)| {
                let mut s = st.borrow_mut();
                let col = colour(&s.assets, &col, "rect")?;
                let (mut layer, mut fill, mut screen) = (3u8, true, false);
                if let Some(o) = opt {
                    fill = !o.get::<Option<bool>>("line")?.unwrap_or(false);
                    screen = o.get::<Option<bool>>("screen")?.unwrap_or(false);
                    if let Some(l) = o.get::<Option<u8>>("layer")? {
                        layer = l.min(LAYERS as u8 - 1);
                    }
                }
                let (cx, cy) = if screen { (0, 0) } else { s.cam };
                let (w, h) = (w.floor() as i32, h.floor() as i32);
                if w > 0 && h > 0 {
                    s.gfx.push_sprite(SpriteCmd { draw: Draw::Rect { w, h, col, fill }, x: x.floor() as i32 - cx, y: y.floor() as i32 - cy, layer });
                }
                Ok(())
            })?,
        )?;

        // text(str, x, y [, col | {col=, align="left|center|right", wrap=px, bg=, shadow=, scale=1..4, pad=}])
        //   -> w, h of the block. Always screen space, always on top.
        let st = self.st.clone();
        g.set(
            "text",
            lua.create_function(move |_, (t, x, y, opt): (String, f64, f64, Option<Value>)| {
                let mut s = st.borrow_mut();
                let (mut col, mut align, mut wrap, mut bg, mut shadow, mut scale, mut pad) = (gfx::SPR_BANK + 1, "left".to_string(), 0, None, None, 1, 2);
                match opt {
                    None | Some(Value::Nil) => {}
                    Some(Value::Table(o)) => {
                        if let Some(c) = o.get::<Option<Value>>("col")? {
                            col = colour(&s.assets, &c, "text")?;
                        }
                        if let Some(a) = o.get::<Option<String>>("align")? {
                            if !["left", "center", "right"].contains(&a.as_str()) {
                                return rt("text: align is \"left\", \"center\" or \"right\"");
                            }
                            align = a;
                        }
                        scale = o.get::<Option<i32>>("scale")?.unwrap_or(1).clamp(1, 4);
                        wrap = o.get::<Option<i32>>("wrap")?.unwrap_or(0);
                        pad = o.get::<Option<i32>>("pad")?.unwrap_or(2);
                        if let Some(c) = o.get::<Option<Value>>("bg")? {
                            bg = Some(colour(&s.assets, &c, "text")?);
                        }
                        if let Some(c) = o.get::<Option<Value>>("shadow")? {
                            shadow = Some(colour(&s.assets, &c, "text")?);
                        }
                    }
                    Some(c) => col = colour(&s.assets, &c, "text")?,
                }
                let lines = layout(&t, wrap, scale);
                let w = lines.iter().map(|l| l.chars().count() as i32 * 8 * scale).max().unwrap_or(0);
                let h = lines.len() as i32 * LINE_H * scale - 2 * scale;
                let (x, y) = (x.floor() as i32, y.floor() as i32);
                let left = match align.as_str() {
                    "center" => x - w / 2,
                    "right" => x - w,
                    _ => x,
                };
                let placed = lines
                    .into_iter()
                    .map(|l| {
                        let lw = l.chars().count() as i32 * 8 * scale;
                        let lx = match align.as_str() {
                            "center" => x - lw / 2,
                            "right" => x - lw,
                            _ => x,
                        };
                        (l, lx)
                    })
                    .collect();
                let bgbox = bg.map(|c| (left - pad, y - pad, w + 2 * pad, h + 2 * pad, c));
                s.gfx.texts.push(TextCmd { lines: placed, y, col, scale, shadow, bg: bgbox });
                Ok((w, h))
            })?,
        )?;

        // textw(str [, scale]) -> width in px of the widest line
        g.set(
            "textw",
            lua.create_function(|_, (t, scale): (String, Option<i32>)| {
                let k = scale.unwrap_or(1).clamp(1, 4);
                Ok(t.split('\n').map(|l| l.chars().count() as i32 * 8 * k).max().unwrap_or(0))
            })?,
        )?;

        // palette([name]) -> { {ch="k", hex="#1a1c2c"}, ... } for entries 1..n (current colours,
        // after any pal() changes). Fades and UI colour picks start here.
        let st = self.st.clone();
        g.set(
            "palette",
            lua.create_function(move |lua, name: Option<String>| {
                let s = st.borrow();
                // no name = the first palette declared (the one bare colour chars like "w" use)
                let pi = match &name {
                    None if !s.assets.palettes.is_empty() => 0,
                    None => return rt("palette: assets.cw has no palettes"),
                    Some(n) => match s.assets.pal_names.get(n) {
                        Some(&pi) => pi,
                        None => return rt(format!("palette: no palette called `{n}`")),
                    },
                };
                let p = &s.assets.palettes[pi as usize];
                let out = lua.create_table()?;
                for i in 1..p.chars.len() {
                    let [r, g, b] = s.gfx.cram[gfx::SPR_BANK as usize + pi as usize * 16 + i];
                    let e = lua.create_table()?;
                    e.set("ch", p.chars[i].to_string())?;
                    e.set("hex", format!("#{r:02x}{g:02x}{b:02x}"))?;
                    out.push(e)?;
                }
                Ok(out)
            })?,
        )?;

        // sprsize(name) -> w, h of a sprite or clip (first frame)
        let st = self.st.clone();
        g.set(
            "sprsize",
            lua.create_function(move |_, name: String| {
                let s = st.borrow();
                let img = match s.assets.names.get(&name) {
                    Some(Ref::Sprite(i)) => &s.assets.sprites[*i as usize],
                    Some(Ref::Clip(c)) => &s.assets.sprites[s.assets.clips[*c as usize].frames[0] as usize],
                    _ => return rt(format!("sprsize: no sprite or clip called `{name}`")),
                };
                Ok((img.w, img.h))
            })?,
        )?;

        // has(name) -> "sprite" | "clip" | "tile" | "map" | "palette" | nil: lets kits make art optional
        let st = self.st.clone();
        g.set(
            "has",
            lua.create_function(move |_, (name, kind): (String, Option<String>)| {
                let s = st.borrow();
                // has(name, "sfx"|"song"|"instrument"): sounds live in their own namespaces
                if let Some(k) = kind {
                    let b = &s.audio.bank;
                    let yes = match k.as_str() {
                        "sfx" => b.sfx.contains_key(&name),
                        "song" => b.songs.contains_key(&name),
                        "instrument" => b.inst_names.contains_key(&name),
                        _ => return Err(mlua::Error::runtime("has(name, kind): kind is sfx, song or instrument")),
                    };
                    return Ok(if yes { Some("yes") } else { None });
                }
                Ok(match s.assets.names.get(&name) {
                    Some(Ref::Sprite(_)) => Some("sprite"),
                    Some(Ref::Clip(_)) => Some("clip"),
                    Some(Ref::Tile(_)) => Some("tile"),
                    Some(Ref::Map(_)) => Some("map"),
                    None if s.assets.pal_names.contains_key(&name) => Some("palette"),
                    None => None,
                })
            })?,
        )?;

        // kit(name) -> the kit's table. Kits are Lua libraries built into the console; they run
        // in the cart's globals and count against its instruction budget like any cart code.
        let cache = lua.create_table()?;
        g.set(
            "kit",
            lua.create_function(move |lua, name: String| {
                if let Some(t) = cache.get::<Option<Value>>(name.as_str())? {
                    return Ok(t);
                }
                let src = match name.as_str() {
                    "topdown" => include_str!("../kits/topdown.lua"),
                    _ => return rt(format!("kit: no kit called `{name}` (kits: topdown)")),
                };
                let v: Value = lua.load(src).set_name(format!("@kit/{name}.lua")).call(())?;
                cache.set(name.as_str(), v.clone())?;
                Ok(v)
            })?,
        )?;

        // bg.get(layer, tx, ty) -> tile name or nil (also nil outside the map)
        let st = self.st.clone();
        bg.set(
            "get",
            lua.create_function(move |_, (layer, tx, ty): (usize, i64, i64)| {
                let s = st.borrow();
                let l = &s.gfx.layers[check_layer(layer)?];
                if tx < 0 || ty < 0 || tx >= l.w as i64 || ty >= l.h as i64 {
                    return Ok(None);
                }
                let c = l.cells[ty as usize * l.w + tx as usize];
                Ok((c != 0).then(|| s.assets.tiles[c as usize - 1].name.clone()))
            })?,
        )?;

        // bg.flag(layer, tx, ty, flag) -> bool
        let st = self.st.clone();
        bg.set(
            "flag",
            lua.create_function(move |_, (layer, tx, ty, f): (usize, i64, i64, String)| {
                let s = st.borrow();
                let bit = flag(&s.assets, Some(f))?;
                let l = &s.gfx.layers[check_layer(layer)?];
                if tx < 0 || ty < 0 || tx >= l.w as i64 || ty >= l.h as i64 {
                    return Ok(false);
                }
                let c = l.cells[ty as usize * l.w + tx as usize];
                Ok(c != 0 && s.assets.tiles[c as usize - 1].flags & bit != 0)
            })?,
        )?;

        // bg.size(layer) -> w, h in tiles (0, 0 before bg.map)
        let st = self.st.clone();
        bg.set(
            "size",
            lua.create_function(move |_, layer: usize| {
                let s = st.borrow();
                let l = &s.gfx.layers[check_layer(layer)?];
                Ok((l.w, l.h))
            })?,
        )?;

        // bg.fixed(layer, bool): pin a layer to the screen so camera() doesn't move it (HUD)
        let st = self.st.clone();
        bg.set(
            "fixed",
            lua.create_function(move |_, (layer, on): (usize, bool)| {
                st.borrow_mut().gfx.layers[check_layer(layer)?].fixed = on;
                Ok(())
            })?,
        )?;
        Ok(())
    }
}

type Boxed = ((i32, i32, i32, i32), Option<String>);

/// `(o [, flag])` or `(x, y [, w, h] [, flag])`.
fn boxed_args(a: &Assets, args: &[Value], what: &str) -> mlua::Result<Boxed> {
    let flag = match args.last() {
        Some(Value::String(s)) => Some(s.to_str()?.to_string()),
        _ => None,
    };
    let nums: Vec<f64> = args.iter().filter_map(|v| match v {
        Value::Integer(i) => Some(*i as f64),
        Value::Number(n) => Some(*n),
        _ => None,
    }).collect();
    let b = match (args.first(), nums.len()) {
        (Some(Value::Table(o)), _) => hitbox(a, o)?,
        (_, 2) => (nums[0].floor() as i32, nums[1].floor() as i32, 1, 1),
        (_, 4) => (nums[0].floor() as i32, nums[1].floor() as i32, nums[2].max(1.0) as i32, nums[3].max(1.0) as i32),
        _ => return rt(format!("{what}: use {what}(obj [, flag]) or {what}(x, y [, w, h] [, flag])")),
    };
    Ok((b, flag))
}

/// Split on `\n`, then word-wrap to `wrap` px (0 = no wrap). Long words are cut.
fn layout(t: &str, wrap: i32, scale: i32) -> Vec<String> {
    let cols = if wrap > 0 { (wrap / (8 * scale)).max(1) as usize } else { usize::MAX };
    let mut out = vec![];
    for para in t.split('\n') {
        let mut line = String::new();
        for w in para.split(' ') {
            let lc = line.chars().count();
            if lc > 0 && lc + 1 + w.chars().count() > cols {
                out.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(w);
            while line.chars().count() > cols {
                let cut: String = line.chars().take(cols).collect();
                let rest: String = line.chars().skip(cols).collect();
                out.push(cut);
                line = rest;
            }
        }
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::Console;

    /// 5x4 room: walls round the edge, one pillar, a door tile, spawn markers.
    fn room() -> String {
        let mut a = String::from("palette p\n . clear\n k #000000\n w #ffffff\n");
        for (name, ch, fl) in [("wall", 'k', " flags=solid"), ("floor", 'w', ""), ("door", 'k', " flags=door")] {
            a.push_str(&format!("tile {name} 16x16 pal=p{fl}\n"));
            for _ in 0..16 {
                a.push_str(&ch.to_string().repeat(16));
                a.push('\n');
            }
        }
        a.push_str("sprite hero 16x16 pal=p hit=4,8,8,8\n");
        for _ in 0..16 {
            a.push_str(&"w".repeat(16));
            a.push('\n');
        }
        a.push_str("sprite hero_l from=hero fx\n");
        a.push_str("map room 5x4\nlegend # wall , floor D door @ @hero:floor b @bat\n##D##\n#@,##\n#,,b#\n#####\n");
        a
    }

    fn run(lua: &str) -> Vec<String> {
        let mut c = Console::new(&room(), lua, 1).unwrap_or_else(|e| panic!("{e}"));
        c.step(0);
        assert!(c.error.is_none(), "{:?}", c.error);
        c.take_logs()
    }

    #[test]
    fn move_stops_flush_and_slides() {
        let l = run(r#"
            function init() bg.map(0, "room") end
            function update()
              local h = {x = 16, y = 16, spr = "hero"}      -- box (20,24) 8x8
              log(move(h, -10, 0), h.x)                      -- wall at x<16: box left stops at 16 → x=12
              local bx, by = move(h, 5.5, 20)               -- floor below is row 2; row 3 is wall (y>=48)
              log(bx, by, h.x, h.y)                         -- box bottom flush at 47 → y = 32
              local g = {x = 16, y = 16, spr = "hero"}
              log(move(g, 30, 0), g.x)                      -- pillar at tile (3,1) x>=48 → box right 47 → x=36
            end"#);
        // (Lua keeps only the first result of a call that isn't last in an argument list)
        assert_eq!(l, ["true 12", "false true 17.5 32", "true 36"]);
    }

    #[test]
    fn map_queries_and_spawns() {
        let l = run(r#"
            function init() bg.map(1, "room") end
            function update()
              log(bg.get(1, 2, 0), bg.get(1, 1, 1), bg.get(1, 0, 0), bg.get(1, 9, 9), bg.size(1))
              log(bg.flag(1, 2, 0, "door"), solid(20, 20), solid(2, 2), solid(2, 2, "door"))
              local sp = spawns("room")
              log(#sp, sp[1].kind, sp[1].x, sp[1].y, sp[2].kind, sp[2].tx, sp[2].ty, #spawns("room", "bat"))
              local h = {x = 32, y = -4, w = 8, h = 8}
              log(touching(h, "door"))
              log(overlap({x=0,y=0,w=10,h=10}, {x=9,y=9,w=4,h=4}), overlap({x=0,y=0,w=10,h=10}, {x=10,y=0,w=4,h=4}))
              log(hitbox({x=10.7, y=0, spr="hero"}), hitbox({x=0, y=0, spr="hero_l"}))
            end"#);
        assert_eq!(l[0], "door floor wall nil 5 4");
        assert_eq!(l[1], "true false true false");
        assert_eq!(l[2], "2 hero 16 16 bat 3 2 1");
        assert_eq!(l[3], "2 0 1");
        assert_eq!(l[4], "true false");
        assert_eq!(l[5], "14 4 8 8 8"); // hero_l = hero flipped: box x 16-4-8 = 4
    }

    #[test]
    fn camera_text_rect() {
        let l = run(r#"
            function init() bg.map(0, "room") bg.map(3, "room") bg.fixed(3, true) end
            function update()
              log(camera(500, -9, {clamp = 0}))            -- 5x4 map is smaller than the screen → 0,0
              log(camera(30, 12), camera())
              log(text("hello\nworld!", 160, 10, {align = "center", bg = "k", col = "p:w"}))
              log(text("one two three four", 0, 0, {wrap = 64, scale = 2}), textw("abc"), textw("ab\nabcd", 2))
              rect(0, 0, 10, 10, "w", {line = true})
            end"#);
        assert_eq!(l, ["0 0", "30 30 12", "48 18", "64 24 64"]);
    }

    #[test]
    fn unknown_flag_is_an_error() {
        let mut c = Console::new(&room(), "function update() solid(0, 0, 'sold') end", 1).unwrap();
        c.step(0);
        assert!(c.error.as_deref().unwrap_or("").contains("no tile has flag `sold`"), "{:?}", c.error);
    }
}
