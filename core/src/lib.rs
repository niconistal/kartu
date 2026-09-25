//! # kartu-core
//!
//! The core of **Kartu**, a fantasy console built to be written by AI and by people: one
//! deterministic 320×240 machine compiled for every screen (desktop, WebAssembly, ARM
//! handhelds). Carts are a Lua 5.4 `main.lua` plus a text-native `assets.cw`.
//!
//! A host only does three things: hand in button bits, call [`Console::step`] 60 times a
//! second, and show `gfx.fb` through `gfx.lut()` (and play [`Console::audio_out`] if it
//! has speakers).
//!
//! ```
//! use kartu_core::{Console, BUTTONS};
//!
//! let assets = "palette p\n . clear\n w #ffffff\n";
//! let main = r#"
//!   x = 0
//!   function update() if btn("right") then x = x + 1 end end
//!   function draw() text("HELLO " .. x, 8, 8, "w") end
//! "#;
//! let mut c = Console::new(assets, main, 1).unwrap();
//! let right = 1 << BUTTONS.iter().position(|b| *b == "right").unwrap();
//! for _ in 0..60 {
//!     c.step(right);
//! }
//! assert!(c.error.is_none());
//! assert_eq!(c.frame(), 60);
//! // same seed + same inputs => same hash, on every target
//! println!("frame hash {:016x}", c.hash());
//! ```
//!
//! Determinism: `rnd` is seeded, there is no clock, `sin`/`cos` come from a polynomial
//! (the C libm differs across targets), the framebuffer holds palette (CRAM) indices and
//! the synth uses its own math, so [`Console::hash`] and [`Console::audio_hash`] match
//! bit for bit on x86, ARM and WASM.
//!
//! Sandbox: no `io`/`os`/`require` (and no Lua `load`: that name is the save-game call), a
//! 32 MB Lua heap and an instruction budget per frame, so a runaway loop becomes an on-screen
//! error instead of a hang.
//!
//! Persistence: a cart may keep one string of up to [`SAVE_MAX`] bytes between sessions with
//! `save(str)` / `load()`. The host owns the storage: it hands the string in at boot
//! ([`Console::new_saved`], or [`Console::set_saved`] before the first step) and collects
//! pending writes with [`Console::take_save`]. Saves never touch the frame hash or the RNG.

pub mod assets;
pub mod audio;
pub mod font;
pub mod gfx;
pub mod inspect;
mod world;

use assets::{Assets, Ref};
use gfx::{Draw, Gfx, SpriteCmd, FLIP_X, FLIP_Y, LAYERS};
use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Value, VmState};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub use gfx::{H, W};

/// Button bits, in the order hosts pack them.
pub const BUTTONS: [&str; 12] = ["left", "right", "up", "down", "a", "b", "x", "y", "l", "r", "start", "select"];

/// The budget is counted in blocks of `HOOK_EVERY` instructions.
const HOOK_EVERY: u32 = 1000;
/// Lua instructions a cart may run per frame (`update` + `draw`) before it is stopped.
pub const DEFAULT_BUDGET: u32 = 4_000_000;
/// Lua heap limit per cart.
pub const LUA_MEMORY: usize = 32 * 1024 * 1024;
/// Longest string `save()` accepts, in bytes.
pub const SAVE_MAX: usize = 4096;

/// Everything the console API reads and writes; hosts read `gfx` and `audio` from here.
pub struct State {
    /// the picture unit: `fb` (CRAM indices) and the colour table
    pub gfx: Gfx,
    pub assets: Assets,
    /// this frame's and last frame's button bits (bit i = `BUTTONS[i]`)
    pub btn: u16,
    pub prev: u16,
    /// frames stepped so far
    pub frame: u64,
    rng: u64,
    pub logs: Vec<String>,
    /// world offset set by `camera()`: subtracted from spr/rect, applied to unpinned layers
    pub cam: (i32, i32),
    /// the synth: `audio.out` holds this frame's samples (24 kHz mono)
    pub audio: audio::Mixer,
    /// what `load()` returns: the string the host stored last session, if any
    pub saved: Option<String>,
    /// the latest `save()` not yet collected by the host (`take_save`)
    pub save_out: Option<String>,
}

/// One running cart.
pub struct Console {
    lua: Lua,
    pub st: Rc<RefCell<State>>,
    ticks: Rc<Cell<u32>>,
    /// Set when the cart fails at runtime; the console then draws the error and stops calling Lua.
    pub error: Option<String>,
    inspect: Option<inspect::Inspector>,
}

fn btn_bit(name: &str) -> mlua::Result<u16> {
    BUTTONS
        .iter()
        .position(|b| *b == name)
        .map(|i| 1 << i)
        .ok_or_else(|| mlua::Error::runtime(format!("unknown button `{name}` (use {})", BUTTONS.join(", "))))
}

impl Console {
    /// Parse the assets, load `main.lua` and call its `init()`.
    /// `seed` feeds `rnd()`: same seed + same inputs ⇒ same frames, on every target.
    /// `load()` returns nil in a console booted this way; see [`Console::new_saved`].
    pub fn new(assets_src: &str, main_lua: &str, seed: u64) -> Result<Console, String> {
        Console::new_saved(assets_src, main_lua, seed, None)
    }

    /// [`Console::new`] with the cart's saved string (from a previous session) already in
    /// place, so `load()` works from `init()` on. A host that only learns it later can still
    /// call [`Console::set_saved`] before the first step.
    pub fn new_saved(assets_src: &str, main_lua: &str, seed: u64, saved: Option<String>) -> Result<Console, String> {
        let mut assets = Assets::parse(assets_src)?;
        let gfx = Gfx::new(&assets);
        let audio = audio::Mixer::new(std::mem::take(&mut assets.sound));
        let st = Rc::new(RefCell::new(State {
            gfx,
            assets,
            btn: 0,
            prev: 0,
            frame: 0,
            rng: seed.wrapping_mul(0x9E3779B97F4A7C15) | 1,
            logs: vec![],
            cam: (0, 0),
            audio,
            saved,
            save_out: None,
        }));
        // `debug` is loaded only for the runner's inspector, which takes it out of the
        // globals before any cart code runs (see inspect.rs).
        let libs = StdLib::STRING | StdLib::TABLE | StdLib::MATH | StdLib::UTF8 | StdLib::COROUTINE | StdLib::DEBUG;
        let lua = unsafe { Lua::unsafe_new_with(libs, LuaOptions::default()) };
        lua.set_memory_limit(LUA_MEMORY).map_err(|e| e.to_string())?;
        let ticks = Rc::new(Cell::new(0u32));
        let mut c = Console { lua, st, ticks, error: None, inspect: None };
        c.inspect = Some(inspect::Inspector::new(&c.lua).map_err(|e| format!("inspector: {e}"))?);
        c.install_api().map_err(|e| format!("api: {e}"))?;
        c.install_world().map_err(|e| format!("api: {e}"))?;
        if let Some(i) = &c.inspect {
            i.mark_builtin.call::<()>(()).map_err(|e| format!("inspector: {e}"))?;
        }
        let t = c.ticks.clone();
        let limit = DEFAULT_BUDGET / HOOK_EVERY;
        c.lua
            .set_hook(HookTriggers::new().every_nth_instruction(HOOK_EVERY), move |_, _| {
                t.set(t.get() + 1);
                if t.get() > limit {
                    Err(mlua::Error::runtime("this frame ran too long (instruction budget); is there an endless loop?"))
                } else {
                    Ok(VmState::Continue)
                }
            })
            .map_err(|e| e.to_string())?;
        c.ticks.set(0);
        c.lua.load(main_lua).set_name("@main.lua").exec().map_err(|e| clean(&e))?;
        c.call("init")?;
        Ok(c)
    }

    fn call(&mut self, f: &str) -> Result<(), String> {
        self.ticks.set(0);
        if let Ok(Value::Function(func)) = self.lua.globals().get::<Value>(f) {
            func.call::<()>(()).map_err(|e| clean(&e))?;
        }
        Ok(())
    }

    /// Run one 1/60 s frame with the given button bits, then render it.
    pub fn step(&mut self, buttons: u16) {
        {
            let mut s = self.st.borrow_mut();
            s.prev = s.btn;
            s.btn = buttons;
            s.gfx.dropped = 0;
        }
        if self.error.is_none() {
            let r = self.call("update").and_then(|_| self.call("draw"));
            if let Err(e) = r {
                self.error = Some(e);
            }
        }
        let mut s = self.st.borrow_mut();
        let State { gfx, assets, .. } = &mut *s;
        gfx.render(assets);
        if let Some(e) = &self.error {
            gfx.fill_rect(0, 0, W as i32, H as i32, 0);
            gfx.backdrop = [0x20, 0x08, 0x10];
            gfx.cram[0] = gfx.backdrop;
            gfx.cram[511] = [0xff, 0xd0, 0xd0];
            let mut y = 8;
            gfx.text("CART ERROR", 8, y, 511);
            y += 16;
            for line in wrap(e, 38) {
                gfx.text(&line, 8, y, 511);
                y += 10;
            }
        }
        s.audio.frame();
        s.frame += 1;
    }

    /// This frame's sound: `audio::PER_FRAME` samples, 24 kHz mono.
    pub fn audio_out(&self) -> Vec<i16> {
        self.st.borrow().audio.out.clone()
    }

    /// Running hash of every sample produced so far.
    pub fn audio_hash(&self) -> u64 {
        self.st.borrow().audio.hash
    }

    /// `sfx coin`, `music theme`, `music stop` since the last call (for the runner).
    pub fn take_sound_events(&self) -> Vec<String> {
        std::mem::take(&mut self.st.borrow_mut().audio.events)
    }

    /// Frames stepped so far.
    pub fn frame(&self) -> u64 {
        self.st.borrow().frame
    }

    /// Hash of the current framebuffer (CRAM indices + colours), identical on every target.
    pub fn hash(&self) -> u64 {
        self.st.borrow().gfx.hash()
    }

    /// Lines the cart printed with `log`/`print` since the last call.
    pub fn take_logs(&self) -> Vec<String> {
        std::mem::take(&mut self.st.borrow_mut().logs)
    }

    /// What `load()` returns from now on. Call it before the first `step` (a `load()` inside
    /// `init()` has already run by then: give the string to [`Console::new_saved`] for that).
    pub fn set_saved(&mut self, s: Option<String>) {
        self.st.borrow_mut().saved = s;
    }

    /// The latest `save()` since the last call, for the host to store; `None` if there was
    /// none. Only the last value of a frame counts.
    pub fn take_save(&mut self) -> Option<String> {
        self.st.borrow_mut().save_out.take()
    }

    fn install_api(&mut self) -> mlua::Result<()> {
        let lua = &self.lua;
        let g = lua.globals();
        for f in ["dofile", "loadfile", "require"] {
            g.set(f, Value::Nil)?;
        }

        // save(str) / load(): one string per cart that outlives the session (the host stores
        // it). Lua's own `load` is replaced, which keeps the sandbox closed.
        let st = self.st.clone();
        g.set(
            "save",
            lua.create_function(move |_, v: Value| {
                let s = match &v {
                    Value::String(s) => s.to_str().map_err(|_| mlua::Error::runtime("save: the string must be valid UTF-8"))?.to_string(),
                    Value::Integer(i) => i.to_string(),
                    Value::Number(n) => n.to_string(),
                    _ => return Err(mlua::Error::runtime("save: expects a string")),
                };
                if s.len() > SAVE_MAX {
                    return Err(mlua::Error::runtime(format!("save: too big ({} bytes, max {SAVE_MAX})", s.len())));
                }
                st.borrow_mut().save_out = Some(s);
                Ok(())
            })?,
        )?;
        let st = self.st.clone();
        g.set("load", lua.create_function(move |_, ()| Ok(st.borrow().saved.clone()))?)?;

        let st = self.st.clone();
        g.set(
            "log",
            lua.create_function(move |_, args: mlua::Variadic<Value>| {
                let s: Vec<String> = args.iter().map(|v| v.to_string().unwrap_or_else(|_| "?".into())).collect();
                st.borrow_mut().logs.push(s.join(" "));
                Ok(())
            })?,
        )?;
        g.set("print", g.get::<Value>("log")?)?;

        // spr(name|id, x, y [, {fx=, fy=, pal=, layer=}])
        let st = self.st.clone();
        g.set(
            "spr",
            lua.create_function(move |_, (who, x, y, opt): (Value, f64, f64, Option<mlua::Table>)| {
                let mut s = st.borrow_mut();
                let img = match &who {
                    Value::String(n) => {
                        let n = n.to_str()?;
                        match s.assets.names.get(&*n) {
                            Some(Ref::Sprite(id)) => *id,
                            Some(Ref::Clip(c)) => {
                                let clip = &s.assets.clips[*c as usize];
                                let i = (s.frame * clip.fps as u64 / 60) as usize % clip.frames.len();
                                clip.frames[i]
                            }
                            _ => return Err(mlua::Error::runtime(format!("spr: no sprite or clip called `{}`", &*n))),
                        }
                    }
                    Value::Integer(i) if (*i as usize) < s.assets.sprites.len() => *i as u16,
                    _ => return Err(mlua::Error::runtime("spr: first argument must be a sprite name")),
                };
                let mut flags = 0;
                let mut pal = s.assets.sprites[img as usize].pal;
                let mut layer = 3u8;
                let mut screen = false;
                if let Some(o) = opt {
                    screen = o.get::<Option<bool>>("screen")?.unwrap_or(false);
                    if o.get::<Option<bool>>("fx")?.unwrap_or(false) {
                        flags |= FLIP_X;
                    }
                    if o.get::<Option<bool>>("fy")?.unwrap_or(false) {
                        flags |= FLIP_Y;
                    }
                    if let Some(p) = o.get::<Option<String>>("pal")? {
                        match s.assets.pal_names.get(&p) {
                            Some(pi) => pal = *pi,
                            None => return Err(mlua::Error::runtime(format!("spr: unknown palette `{p}`"))),
                        }
                    }
                    if let Some(l) = o.get::<Option<u8>>("layer")? {
                        layer = l.min(LAYERS as u8 - 1);
                    }
                }
                let (cx, cy) = if screen { (0, 0) } else { s.cam };
                s.gfx.push_sprite(SpriteCmd { draw: Draw::Img { img, flags, pal }, x: x.floor() as i32 - cx, y: y.floor() as i32 - cy, layer });
                Ok(())
            })?,
        )?;

        let st = self.st.clone();
        g.set(
            "backdrop",
            lua.create_function(move |_, hex: String| {
                let c = assets::parse_hex(&hex).ok_or_else(|| mlua::Error::runtime("backdrop: use \"#rrggbb\""))?;
                st.borrow_mut().gfx.backdrop = c;
                Ok(())
            })?,
        )?;

        // pal(palette, entry char or 0-15, "#rrggbb"): recolour both banks (fades, swaps)
        let st = self.st.clone();
        g.set(
            "pal",
            lua.create_function(move |_, (p, entry, hex): (String, Value, String)| {
                let mut s = st.borrow_mut();
                let Some(pi) = s.assets.pal_names.get(&p).copied() else {
                    return Err(mlua::Error::runtime(format!("pal: unknown palette `{p}`")));
                };
                let idx = match entry {
                    Value::Integer(i) if (0..16).contains(&i) => i as usize,
                    Value::String(c) => {
                        let c = c.to_str()?.chars().next().unwrap_or(' ');
                        s.assets.palettes[pi as usize].chars.iter().position(|&x| x == c).ok_or_else(|| mlua::Error::runtime("pal: char not in palette"))?
                    }
                    _ => return Err(mlua::Error::runtime("pal: entry is a palette char or 0-15")),
                };
                let c = assets::parse_hex(&hex).ok_or_else(|| mlua::Error::runtime("pal: use \"#rrggbb\""))?;
                s.gfx.cram[pi as usize * 16 + idx] = c;
                s.gfx.cram[gfx::SPR_BANK as usize + pi as usize * 16 + idx] = c;
                Ok(())
            })?,
        )?;

        let st = self.st.clone();
        g.set("btn", lua.create_function(move |_, b: String| Ok(st.borrow().btn & btn_bit(&b)? != 0))?)?;
        let st = self.st.clone();
        g.set(
            "btnp",
            lua.create_function(move |_, b: String| {
                let s = st.borrow();
                let bit = btn_bit(&b)?;
                Ok(s.btn & bit != 0 && s.prev & bit == 0)
            })?,
        )?;

        // rnd(n): float in [0, n); irnd(a, b): integer in [a, b]. Seeded, never wall-clock.
        let st = self.st.clone();
        g.set(
            "rnd",
            lua.create_function(move |_, n: Option<f64>| {
                let r = next_rng(&mut st.borrow_mut().rng);
                Ok((r >> 11) as f64 / (1u64 << 53) as f64 * n.unwrap_or(1.0))
            })?,
        )?;
        let st = self.st.clone();
        g.set(
            "irnd",
            lua.create_function(move |_, (a, b): (i64, i64)| {
                if b < a {
                    return Err(mlua::Error::runtime("irnd: b < a"));
                }
                let r = next_rng(&mut st.borrow_mut().rng);
                Ok(a + (r % (b - a + 1) as u64) as i64)
            })?,
        )?;
        let st = self.st.clone();
        g.set("frame", lua.create_function(move |_, ()| Ok(st.borrow().frame))?)?;

        // Deterministic trig (the C libm's sin/cos can differ by an ulp across targets).
        // Angles are in turns like PICO-8: 1.0 = full circle.
        g.set("sin", lua.create_function(|_, t: f64| Ok(dsin(t)))?)?;
        g.set("cos", lua.create_function(|_, t: f64| Ok(dsin(t + 0.25)))?)?;

        // sound: sfx(name [, {vol=, pitch=}]), music(name|nil [, {vol=, fade=, loop=}]),
        // play(mml [, {inst=, vol=}]), music_playing()
        let st = self.st.clone();
        g.set(
            "sfx",
            lua.create_function(move |_, (name, o): (String, Option<mlua::Table>)| {
                let (vol, pitch) = match &o {
                    Some(o) => (o.get::<Option<f32>>("vol")?.unwrap_or(1.0), o.get::<Option<f32>>("pitch")?.unwrap_or(0.0)),
                    None => (1.0, 0.0),
                };
                st.borrow_mut().audio.sfx(&name, vol.clamp(0.0, 2.0), pitch).map_err(mlua::Error::runtime)
            })?,
        )?;
        let st = self.st.clone();
        g.set(
            "music",
            lua.create_function(move |_, (name, o): (Option<String>, Option<mlua::Table>)| {
                let (vol, fade, lp) = match &o {
                    Some(o) => (o.get::<Option<f32>>("vol")?.unwrap_or(1.0), o.get::<Option<f32>>("fade")?.unwrap_or(0.0), o.get::<Option<bool>>("loop")?),
                    None => (1.0, 0.0, None),
                };
                st.borrow_mut().audio.music(name.as_deref(), vol.clamp(0.0, 2.0), fade.max(0.0), lp).map_err(mlua::Error::runtime)
            })?,
        )?;
        let st = self.st.clone();
        g.set(
            "play",
            lua.create_function(move |_, (mml, o): (String, Option<mlua::Table>)| {
                let (inst, vol) = match &o {
                    Some(o) => (o.get::<Option<String>>("inst")?.unwrap_or_else(|| "square".into()), o.get::<Option<f32>>("vol")?.unwrap_or(0.7)),
                    None => ("square".into(), 0.7),
                };
                st.borrow_mut().audio.play_mml(&mml, &inst, vol.clamp(0.0, 2.0)).map_err(|e| mlua::Error::runtime(format!("play: {e}")))
            })?,
        )?;
        let st = self.st.clone();
        g.set("music_playing", lua.create_function(move |_, ()| Ok(st.borrow().audio.playing()))?)?;

        let bg = lua.create_table()?;
        let st = self.st.clone();
        bg.set(
            "map",
            lua.create_function(move |_, (layer, name): (usize, String)| {
                let mut s = st.borrow_mut();
                let l = check_layer(layer)?;
                let Some(Ref::Map(mi)) = s.assets.names.get(&name).copied() else {
                    return Err(mlua::Error::runtime(format!("bg.map: no map called `{name}`")));
                };
                let m = s.assets.maps[mi as usize].clone();
                let ly = &mut s.gfx.layers[l];
                ly.w = m.w;
                ly.h = m.h;
                ly.cells = m.cells;
                ly.visible = true;
                Ok(())
            })?,
        )?;
        let st = self.st.clone();
        bg.set(
            "scroll",
            lua.create_function(move |_, (layer, x, y): (usize, f64, f64)| {
                let mut s = st.borrow_mut();
                let l = &mut s.gfx.layers[check_layer(layer)?];
                l.sx = x.floor() as i32;
                l.sy = y.floor() as i32;
                Ok(())
            })?,
        )?;
        let st = self.st.clone();
        bg.set(
            "show",
            lua.create_function(move |_, (layer, on): (usize, bool)| {
                st.borrow_mut().gfx.layers[check_layer(layer)?].visible = on;
                Ok(())
            })?,
        )?;
        let st = self.st.clone();
        bg.set(
            "tile",
            lua.create_function(move |_, (layer, tx, ty, name): (usize, usize, usize, Option<String>)| {
                let mut s = st.borrow_mut();
                let id = match name {
                    None => 0,
                    Some(n) => match s.assets.names.get(&n) {
                        Some(Ref::Tile(t)) => t + 1,
                        _ => return Err(mlua::Error::runtime(format!("bg.tile: no tile called `{n}`"))),
                    },
                };
                let l = &mut s.gfx.layers[check_layer(layer)?];
                if tx >= l.w || ty >= l.h {
                    return Err(mlua::Error::runtime("bg.tile: outside the layer's map"));
                }
                l.cells[ty * l.w + tx] = id;
                Ok(())
            })?,
        )?;
        g.set("bg", bg)?;
        Ok(())
    }
}

/// Compile `main.lua` without running it: `Err` is the syntax error, line included.
pub fn check_lua(src: &str) -> Result<(), String> {
    let lua = Lua::new_with(StdLib::NONE, LuaOptions::default()).map_err(|e| e.to_string())?;
    lua.load(src).set_name("@main.lua").into_function().map(|_| ()).map_err(|e| clean(&e))
}

fn check_layer(l: usize) -> mlua::Result<usize> {
    if l < LAYERS {
        Ok(l)
    } else {
        Err(mlua::Error::runtime("layers are 0 (back) to 3 (front)"))
    }
}

fn next_rng(s: &mut u64) -> u64 {
    *s ^= *s >> 12;
    *s ^= *s << 25;
    *s ^= *s >> 27;
    s.wrapping_mul(0x2545F4914F6CDD1D)
}

/// sin(2π·t) from a polynomial on plain + and ×, which IEEE makes identical everywhere.
pub fn dsin(t: f64) -> f64 {
    let mut x = t - t.floor(); // [0,1)
    let mut sign = 1.0;
    if x >= 0.5 {
        x -= 0.5;
        sign = -1.0;
    }
    if x > 0.25 {
        x = 0.5 - x;
    }
    let r = x * std::f64::consts::TAU; // [0, π/2]
    let r2 = r * r;
    // Taylor to r^15: error < 1e-9 on [0, π/2]
    let p = r * (1.0 + r2 * (-1.0 / 6.0 + r2 * (1.0 / 120.0 + r2 * (-1.0 / 5040.0 + r2 * (1.0 / 362880.0 + r2 * (-1.0 / 39916800.0 + r2 * (1.0 / 6227020800.0 + r2 * (-1.0 / 1307674368000.0))))))));
    sign * p
}

pub(crate) fn clean(e: &mlua::Error) -> String {
    // mlua wraps callback errors; the first line is what a cart author needs.
    let s = e.to_string();
    s.lines().filter(|l| !l.trim().is_empty() && !l.starts_with("stack traceback")).take(3).collect::<Vec<_>>().join("\n")
}

fn wrap(s: &str, cols: usize) -> Vec<String> {
    let mut out = vec![];
    for para in s.lines() {
        let mut line = String::new();
        for w in para.split(' ') {
            if line.len() + w.len() + 1 > cols && !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(w);
            while line.len() > cols {
                let rest = line.split_off(cols);
                out.push(std::mem::replace(&mut line, rest));
            }
        }
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsin_is_close() {
        for i in 0..1000 {
            let t = i as f64 / 997.0 - 0.3;
            assert!((dsin(t) - (t * std::f64::consts::TAU).sin()).abs() < 1e-8, "t={t}");
        }
    }

    #[test]
    fn runaway_cart_is_stopped() {
        let a = "palette p\n . clear\n k #ffffff\n";
        let mut c = Console::new(a, "function update() while true do end end", 1).unwrap();
        c.step(0);
        assert!(c.error.as_deref().unwrap_or("").contains("budget"), "{:?}", c.error);
    }

    #[test]
    fn save_and_load() {
        let a = "palette p\n . clear\n k #ffffff\n";
        let m = "n = (load() or 0) + 1\nfunction update() if frame() == 1 then save('x' .. n) save(n) end end\nfunction draw() rnd() end";
        let mut c = Console::new_saved(a, m, 1, Some("41".into())).unwrap();
        assert_eq!(c.take_save(), None);
        c.step(0);
        c.step(0);
        assert_eq!(c.take_save().as_deref(), Some("42"), "{:?}", c.error);
        assert_eq!(c.take_save(), None);
        // the save leaves the picture and the RNG alone: same hash as a run that never saved
        let mut plain = Console::new(a, "function draw() rnd() end", 1).unwrap();
        plain.step(0);
        plain.step(0);
        assert_eq!(c.hash(), plain.hash());
        let mut c = Console::new(a, "function init() load2 = load() end", 1).unwrap();
        c.set_saved(Some("later".into()));
        assert_eq!(c.eval_json("load2", 1).unwrap(), "null");
        assert_eq!(c.eval_json("load()", 1).unwrap(), "\"later\"");
        let mut c = Console::new(a, "function update() save(string.rep('x', 4097)) end", 1).unwrap();
        c.step(0);
        assert!(c.error.as_deref().unwrap_or("").contains("save: too big (4097 bytes, max 4096)"), "{:?}", c.error);
    }

    #[test]
    fn deterministic() {
        let a = "palette p\n . clear\n k #ffffff\nsprite s 1x1 pal=p\nk\n";
        let m = "function draw() for i=1,50 do spr('s', rnd(320), rnd(240)) end end";
        let run = || {
            let mut c = Console::new(a, m, 7).unwrap();
            for _ in 0..10 {
                c.step(0);
            }
            c.hash()
        };
        assert_eq!(run(), run());
    }
}
