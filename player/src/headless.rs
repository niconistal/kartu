//! Headless runner: no screen, as fast as the CPU goes. Same core as every other target,
//! so a frame hash here is the frame hash in the browser and on the handheld.

use crate::bot::Bot;
use crate::cart::{self, Recorder, Script};
use crate::Args;
use kartu_core::{Console, H, W};
use std::time::Instant;

pub fn run(a: &[String]) -> Result<(), String> {
    let a = Args::parse(a);
    let frames: u64 = a.num("frames", 600);
    let dir = a.cart()?;
    // --load: replay a save's input up to its frame (and check the hash) before anything else
    let save = match a.kv.get("load") {
        Some(p) => Some(Save::read(p)?),
        None => None,
    };
    let seed = save.as_ref().map(|s| s.seed).unwrap_or(a.num("seed", 1));
    let mut c = cart::load(&dir, seed)?;
    let mut script = Script::default();
    if let Some(p) = a.kv.get("script") {
        script.add(&std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?)?;
    }
    if let Some(p) = a.kv.get("press") {
        script.add(p)?;
    }
    let mut bot = match a.kv.get("bot") {
        Some(p) => Some(Bot::parse(&std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?)?),
        None => None,
    };
    let until = a.kv.get("until");
    let shot_base = a.kv.get("shot").cloned().unwrap_or_else(|| "shot.png".into());
    let mut shot_at: Vec<u64> = frame_list(a.kv.get("shot-at"), "--shot-at")?;
    let every: u64 = a.num("shot-every", 0);
    if every > 0 {
        shot_at.extend((every - 1..frames).step_by(every as usize));
    }
    let watches: Vec<String> = a.kv.get("watch").map(|w| w.split(';').map(|e| e.trim().to_string()).filter(|e| !e.is_empty()).collect()).unwrap_or_default();
    let watch_every: u64 = a.num("watch-every", 60).max(1);
    let dump_end = a.kv.get("dump").is_some_and(|d| d.split(',').any(|x| x.trim() == "end"));
    let dump_at = frame_list(a.kv.get("dump").map(|d| d.split(',').filter(|x| x.trim() != "end").collect::<Vec<_>>().join(",")).as_ref(), "--dump")?;
    let mut rec = Recorder::default();
    let mut wav: Option<Vec<i16>> = a.kv.get("wav").map(|_| vec![]);
    let sounds = a.kv.contains_key("sounds");
    let t = Instant::now();
    let mut ran = 0;
    let mut hit = None;
    let mut first = 0;
    if let Some(sv) = &save {
        for f in 0..sv.frame {
            let b = sv.script.buttons_at(f);
            rec.push(f, b);
            c.step(b);
            c.take_logs();
            if let Some(e) = &c.error {
                return Err(format!("save replay: cart error at f{f}: {e}"));
            }
        }
        if c.hash() != sv.hash {
            return Err(format!("save {} does not replay: hash at f{} is {:016x}, the save says {:016x} (cart or engine changed)", a.kv["load"], sv.frame, c.hash(), sv.hash));
        }
        println!("loaded f{} (hash {:016x} ok)", sv.frame, sv.hash);
        first = sv.frame;
        ran = first;
    }
    for f in first..frames {
        let mut b = script.buttons_at(f);
        if let Some(bt) = bot.as_mut() {
            b |= bt.buttons(&c, f);
        }
        rec.push(f, b);
        c.step(b);
        ran = f + 1;
        if let Some(w) = wav.as_mut() {
            w.extend(c.audio_out());
        }
        for e in c.take_sound_events() {
            if sounds {
                println!("[sound f{f}] {e}");
            }
        }
        for l in c.take_logs() {
            println!("[log f{f}] {l}");
            if hit.is_none() && until.is_some_and(|u| l.contains(u.as_str())) {
                hit = Some(f);
            }
        }
        if !watches.is_empty() && (f + 1) % watch_every == 0 {
            println!("[watch f{f}] {}", watch_line(&c, &watches));
        }
        if dump_at.contains(&f) {
            println!("[dump f{f}] {}", c.dump(4)?);
        }
        if shot_at.contains(&f) {
            let p = numbered(&shot_base, f);
            shot(&c, &p, a.num("scale", 2))?;
            println!("screenshot f{f} {p}");
        }
        if hit.is_some() || c.error.is_some() || bot.as_ref().is_some_and(|b| b.failed.is_some()) {
            break;
        }
    }
    let dt = t.elapsed().as_secs_f64();
    if !watches.is_empty() && ran % watch_every != 0 {
        println!("[watch f{}] {}", ran.saturating_sub(1), watch_line(&c, &watches));
    }
    if dump_end {
        println!("[dump f{}] {}", ran.saturating_sub(1), c.dump(4)?);
    }
    println!("frames {ran}  hash {:016x}  {:.0} fps headless", c.hash(), (ran - first) as f64 / dt);
    {
        let s = c.st.borrow();
        if s.audio.loud_frames > 0 {
            println!("audio {:016x}  {:.1} s with sound, peak {:.0}%{}", s.audio.hash, s.audio.loud_frames as f64 / 60.0,
                s.audio.peak as f64 / 327.67, if s.audio.clipped > 0 { format!(", {} samples clipped", s.audio.clipped) } else { String::new() });
        }
    }
    if let (Some(w), Some(p)) = (&wav, a.kv.get("wav")) {
        write_wav(p, w)?;
        println!("wav {p} ({:.1} s)", w.len() as f64 / kartu_core::audio::RATE as f64);
    }
    if let Some(e) = &c.error {
        println!("error at f{}: {e}", ran - 1);
    }
    if a.kv.contains_key("shot") {
        shot(&c, &shot_base, a.num("scale", 2))?;
        println!("screenshot {shot_base}");
    }
    if let Some(p) = a.kv.get("record") {
        std::fs::write(p, &rec.out).map_err(|e| format!("{p}: {e}"))?;
        println!("input recorded to {p}");
    }
    if let Some(p) = a.kv.get("save") {
        let body = format!("# kartu save seed={seed} frame={ran} hash={:016x}\n# replays {dir}; continue with: kartu run {dir} --load {p} ...\n{}", c.hash(), rec.out);
        std::fs::write(p, body).map_err(|e| format!("{p}: {e}"))?;
        println!("saved f{ran} to {p}");
    }
    if let Some(bt) = &bot {
        if bt.failed.is_some() {
            std::process::exit(4);
        }
        if !bt.done {
            println!("[bot] plan not finished when the run ended");
        }
    }
    if let Some(u) = until {
        match hit {
            Some(f) => println!("until: `{u}` at f{f}"),
            None => {
                println!("until: `{u}` NOT logged in {ran} frames");
                std::process::exit(3);
            }
        }
    }
    if c.error.is_some() {
        std::process::exit(2);
    }
    Ok(())
}

fn frame_list(v: Option<&String>, what: &str) -> Result<Vec<u64>, String> {
    match v {
        Some(v) if !v.trim().is_empty() => v.split(',').map(|f| f.trim().parse().map_err(|_| format!("{what}: bad frame `{f}`"))).collect(),
        _ => Ok(vec![]),
    }
}

fn watch_line(c: &Console, exprs: &[String]) -> String {
    exprs.iter().map(|e| format!("{e}={}", c.eval_json(e, 2).unwrap_or_else(|err| format!("<{err}>")))).collect::<Vec<_>>().join("  ")
}

/// A save = the input that got here, replayed from boot (the console is deterministic).
struct Save {
    seed: u64,
    frame: u64,
    hash: u64,
    script: Script,
}

impl Save {
    fn read(p: &str) -> Result<Save, String> {
        let src = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
        let head = src.lines().next().unwrap_or("");
        let field = |k: &str| head.split_whitespace().find_map(|w| w.strip_prefix(k).and_then(|r| r.strip_prefix('='))).ok_or_else(|| format!("{p}: not a save (first line needs `# kartu save seed= frame= hash=`)"));
        Ok(Save {
            seed: field("seed")?.parse().map_err(|_| "bad seed in save")?,
            frame: field("frame")?.parse().map_err(|_| "bad frame in save")?,
            hash: u64::from_str_radix(field("hash")?, 16).map_err(|_| "bad hash in save")?,
            script: {
                let mut s = Script::default();
                s.add(&src)?;
                s
            },
        })
    }
}

/// `out.png` + frame 70 → `out-f70.png`
fn numbered(base: &str, f: u64) -> String {
    match base.rsplit_once('.') {
        Some((stem, ext)) if !ext.contains('/') => format!("{stem}-f{f}.{ext}"),
        _ => format!("{base}-f{f}.png"),
    }
}

pub fn shot(c: &Console, path: &str, scale: usize) -> Result<(), String> {
    let s = c.st.borrow();
    let lut = s.gfx.lut(|[r, g, b]| u32::from_le_bytes([r, g, b, 255]));
    let (w, h) = (W * scale, H * scale);
    let mut buf = vec![0u8; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let p = lut[s.gfx.fb[(y / scale) * W + x / scale] as usize].to_le_bytes();
            buf[(y * w + x) * 3..(y * w + x) * 3 + 3].copy_from_slice(&p[..3]);
        }
    }
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut wr| wr.write_image_data(&buf)).map_err(|e| e.to_string())
}

pub struct Stats {
    v: Vec<f64>,
}

impl Stats {
    pub fn new() -> Stats {
        Stats { v: vec![] }
    }
    pub fn add(&mut self, ms: f64) {
        self.v.push(ms);
    }
    pub fn clear(&mut self) {
        self.v.clear();
    }
    pub fn len(&self) -> usize {
        self.v.len()
    }
    pub fn summary(&self) -> String {
        if self.v.is_empty() {
            return "n/a".into();
        }
        let mut s = self.v.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg = s.iter().sum::<f64>() / s.len() as f64;
        let p99 = s[((s.len() as f64 * 0.99) as usize).min(s.len() - 1)];
        format!("avg {avg:.2} ms  p99 {p99:.2} ms  max {:.2} ms", s[s.len() - 1])
    }
    pub fn avg(&self) -> f64 {
        self.v.iter().sum::<f64>() / self.v.len().max(1) as f64
    }
}

/// Headless timing of the gate scene then the stress scene (A pressed), core work only.
pub fn bench(a: &[String]) -> Result<(), String> {
    let a = Args::parse(a);
    let frames: u64 = a.num("frames", 600);
    let mut c = cart::load(&a.cart()?, a.num("seed", 1))?;
    let mut out = vec![0u32; W * H];
    for (label, press) in [("gate  ", 0u16), ("stress", 1 << 4)] {
        if press != 0 {
            c.step(press);
        }
        let mut st = Stats::new();
        let mut cv = Stats::new();
        for _ in 0..frames {
            let t = Instant::now();
            c.step(0);
            st.add(t.elapsed().as_secs_f64() * 1e3);
            let t = Instant::now();
            let s = c.st.borrow();
            let lut = s.gfx.lut(|[r, g, b]| u32::from_le_bytes([b, g, r, 255]));
            for (o, i) in out.iter_mut().zip(&s.gfx.fb) {
                *o = lut[*i as usize];
            }
            cv.add(t.elapsed().as_secs_f64() * 1e3);
        }
        std::hint::black_box(&out);
        println!("{label}  step (lua+render): {}  |  colour convert: {}", st.summary(), cv.summary());
        println!("{label}  => {:.0}% of a 16.7 ms frame", (st.avg() + cv.avg()) / 16.667 * 100.0);
    }
    if let Some(e) = &c.error {
        return Err(format!("cart error: {e}"));
    }
    Ok(())
}

/// 16-bit mono PCM at the console's rate.
pub fn write_wav(path: &str, s: &[i16]) -> Result<(), String> {
    let rate = kartu_core::audio::RATE;
    let n = (s.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + n as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + n).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&n.to_le_bytes());
    for v in s {
        b.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, b).map_err(|e| format!("{path}: {e}"))
}

/// `kartu sound <cart> --song NAME | --sfx NAME | --list [--secs N] [--wav F]`:
/// hear (or measure) one sound without playing the game.
pub fn sound(a: &[String]) -> Result<(), String> {
    use kartu_core::audio::{Mixer, PER_FRAME, RATE};
    let a = Args::parse(a);
    let dir = a.cart()?;
    let src = std::fs::read_to_string(format!("{dir}/assets.cw")).unwrap_or_default();
    let assets = kartu_core::assets::Assets::parse(&src)?;
    let bank = assets.sound;
    if a.kv.contains_key("list") || (!a.kv.contains_key("song") && !a.kv.contains_key("sfx")) {
        let mut songs: Vec<&String> = bank.songs.keys().collect();
        songs.sort();
        let mut sfx: Vec<&String> = bank.sfx.keys().collect();
        sfx.sort();
        let mut inst: Vec<&String> = bank.inst_names.keys().collect();
        inst.sort();
        println!("songs: {}", songs.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "));
        println!("sfx: {}", sfx.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "));
        println!("instruments: {}", inst.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "));
        for l in bank.describe() {
            println!("{l}");
        }
        return Ok(());
    }
    let mut m = Mixer::new(bank);
    let secs: f64 = match (a.kv.get("secs"), a.kv.get("song")) {
        (Some(s), _) => s.parse().map_err(|_| "--secs: a number")?,
        (None, Some(n)) => m.bank.songs.get(n).map(|s| (s.end as f64 / RATE as f64).min(60.0)).unwrap_or(10.0),
        _ => 1.5,
    };
    if let Some(n) = a.kv.get("song") {
        m.music(Some(n), 1.0, 0.0, Some(false))?;
    } else if let Some(n) = a.kv.get("sfx") {
        m.sfx(n, 1.0, 0.0)?;
    }
    let frames = (secs * 60.0).ceil() as usize;
    let mut out = Vec::with_capacity(frames * PER_FRAME);
    for _ in 0..frames {
        m.frame();
        out.extend_from_slice(&m.out);
    }
    println!("{:.1} s  audio {:016x}  peak {:.0}%  {:.1} s with sound{}", secs, m.hash, m.peak as f64 / 327.67, m.loud_frames as f64 / 60.0,
        if m.clipped > 0 { format!("  {} samples clipped", m.clipped) } else { String::new() });
    if let Some(p) = a.kv.get("wav") {
        write_wav(p, &out)?;
        println!("wav {p}");
    }
    Ok(())
}
