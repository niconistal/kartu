//! Sound: 8 voices, instruments, text-written music (MML), parametric SFX.
//!
//! Everything is synthesised here, in the core, 400 samples per frame at 24 kHz mono, with
//! plain IEEE + − × ÷ and our own exp2/sin polynomials, so the same cart makes the same
//! samples on every target (the runner hashes them like frames). Hosts only play `out`.
//!
//! Text forms (in assets.cw; all three have built-in presets that work without declaring):
//! ```text
//! instrument buzz from=lead duty=0.25 vibrato=0.3,6       -- a synth patch
//! sfx zap wave=saw from=880 to=110 len=0.25               -- a pitch sweep
//! sfx chime notes="o6 l16 c e g >c4" inst=bell            -- or a little tune
//! song theme bpm=120                                      -- music: one MML line per channel
//!   lead  @lead o5 l8 e d c d e e e4
//!   bass  @bass o3 l4 c g c g
//!   drums @drums k8 h8 s8 h8
//! ```

use std::collections::HashMap;
use std::rc::Rc;

pub const RATE: u32 = 24_000;
pub const PER_FRAME: usize = 400; // RATE / 60
pub const VOICES: usize = 8;
pub const MAX_SONG_CHANNELS: usize = 6;
const TICKS_WHOLE: u32 = 192; // a whole note; a quarter is 48
const CTL: u32 = 16; // samples between pitch/vibrato updates
const MASTER: f32 = 0.3;

// ---------- deterministic math ----------

/// 2^x from a polynomial + exact exponent bits (libm's exp2/powf differ across targets).
#[allow(clippy::approx_constant)] // a fitted coefficient, not ln 2: changing it changes the audio hash
pub fn exp2f(x: f32) -> f32 {
    let x = x.clamp(-60.0, 60.0);
    let fl = x.floor();
    let f = x - fl;
    let p = 1.0 + f * (0.693_147_2 + f * (0.240_226_5 + f * (0.055_504_1 + f * (0.009_618_1 + f * (0.001_333_4 + f * (0.000_154_0 + f * 0.000_015_25))))));
    p * f32::from_bits(((fl as i32 + 127) as u32) << 23)
}

/// sin of a phase in turns (0..1).
fn sin_turn(t: f32) -> f32 {
    let mut x = t - t.floor();
    let mut sign = 1.0;
    if x >= 0.5 {
        x -= 0.5;
        sign = -1.0;
    }
    if x > 0.25 {
        x = 0.5 - x;
    }
    let r = x * std::f32::consts::TAU;
    let r2 = r * r;
    sign * r * (1.0 + r2 * (-1.0 / 6.0 + r2 * (1.0 / 120.0 + r2 * (-1.0 / 5040.0 + r2 * (1.0 / 362_880.0)))))
}

fn hz(midi: f32) -> f32 {
    440.0 * exp2f((midi - 69.0) / 12.0)
}

fn inc(midi: f32) -> u32 {
    ((hz(midi) as f64) * 4_294_967_296.0 / RATE as f64).clamp(0.0, 2_000_000_000.0) as u32
}

/// Per-sample multiplier that falls 60 dB (≈2^-10) in `secs`.
fn decay_k(secs: f32) -> f32 {
    if secs <= 0.0 {
        0.0
    } else {
        exp2f(-10.0 / (secs * RATE as f32))
    }
}

// ---------- patches ----------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wave {
    Square,
    Triangle,
    Saw,
    Sine,
    Noise,
    Pluck,
    Fm,
}

#[derive(Clone, Debug)]
pub struct Patch {
    pub wave: Wave,
    pub duty: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub vol: f32,
    pub vib: (f32, f32, f32), // depth (semitones), rate (Hz), delay (s)
    pub slide: (f32, f32),    // start offset (semitones), time (s): glides to the note
    pub lowpass: f32,         // 0..1, 1 = open
    pub detune: f32,          // semitones; non-zero adds a second oscillator
    pub fm: (f32, f32, f32),  // ratio, index, index decay (s)
    pub noise: f32,           // mix of noise on top of the wave (drums)
    pub transpose: f32,
    pub note: Option<f32>, // fixed pitch (drums)
}

impl Default for Patch {
    fn default() -> Self {
        Patch {
            wave: Wave::Square,
            duty: 0.5,
            attack: 0.005,
            decay: 0.2,
            sustain: 0.7,
            release: 0.08,
            vol: 1.0,
            vib: (0.0, 5.5, 0.15),
            slide: (0.0, 0.0),
            lowpass: 1.0,
            detune: 0.0,
            fm: (1.0, 0.0, 0.0),
            noise: 0.0,
            transpose: 0.0,
            note: None,
        }
    }
}

/// name, definition (the same `key=value` text a cart writes after `instrument <name>`).
pub const INSTRUMENTS: &[(&str, &str)] = &[
    ("square", "wave=square lowpass=0.7"),
    ("pulse", "wave=square duty=0.125 lowpass=0.7"),
    ("triangle", "wave=triangle"),
    ("saw", "wave=saw lowpass=0.6"),
    ("sine", "wave=sine"),
    ("noise", "wave=noise decay=0.3 sustain=0"),
    ("lead", "wave=square duty=0.25 attack=0.01 decay=0.3 sustain=0.6 vibrato=0.25,5.5 lowpass=0.55 vol=0.8"),
    ("softlead", "wave=triangle attack=0.02 decay=0.4 sustain=0.7 vibrato=0.2,5 vol=1"),
    ("piano", "wave=fm fm=1,1.6,0.5 attack=0.002 decay=1.4 sustain=0 release=0.25 vol=0.9"),
    ("epiano", "wave=fm fm=1,2.4,0.9 attack=0.002 decay=1.8 sustain=0.1 release=0.3 vol=0.85"),
    ("bell", "wave=fm fm=3.5,3,1.2 attack=0.001 decay=2.2 sustain=0 release=0.8 vol=0.7"),
    ("marimba", "wave=fm fm=4,1.8,0.08 attack=0.001 decay=0.45 sustain=0 release=0.1 vol=0.95"),
    ("organ", "wave=fm fm=2,0.6,0 attack=0.01 decay=0.1 sustain=0.9 release=0.08 vol=0.6"),
    ("flute", "wave=sine attack=0.06 decay=0.2 sustain=0.8 release=0.12 vibrato=0.2,5 noise=0.04 vol=0.9"),
    ("strings", "wave=saw detune=0.12 attack=0.15 decay=0.3 sustain=0.8 release=0.4 lowpass=0.18 vibrato=0.12,5 vol=0.7"),
    ("pad", "wave=triangle detune=0.1 attack=0.4 decay=0.5 sustain=0.8 release=0.9 lowpass=0.5 vol=0.8"),
    ("brass", "wave=saw attack=0.04 decay=0.2 sustain=0.75 release=0.1 lowpass=0.3 vibrato=0.15,5 vol=0.7"),
    ("pluck", "wave=pluck decay=1.2 sustain=1 release=0.1 lowpass=0.8 vol=0.9"),
    ("harp", "wave=pluck decay=2.5 sustain=1 release=0.4 vol=0.8"),
    ("bass", "wave=square duty=0.5 attack=0.005 decay=0.25 sustain=0.7 release=0.05 lowpass=0.18 vol=1.1"),
    ("synbass", "wave=saw attack=0.003 decay=0.3 sustain=0.35 release=0.05 lowpass=0.22 vol=1"),
    ("pluckbass", "wave=pluck decay=0.8 sustain=1 release=0.05 lowpass=0.4 vol=1.1"),
    ("kick", "wave=sine slide=30,0.06 attack=0.001 decay=0.35 sustain=0 release=0.02 note=36 vol=1.4"),
    ("snare", "wave=triangle noise=0.75 slide=7,0.03 attack=0.001 decay=0.2 sustain=0 release=0.02 note=50 lowpass=0.7 vol=0.9"),
    ("hat", "wave=noise attack=0.001 decay=0.05 sustain=0 release=0.01 note=108 vol=0.35"),
    ("openhat", "wave=noise attack=0.001 decay=0.35 sustain=0 release=0.05 note=108 vol=0.3"),
    ("clap", "wave=noise attack=0.001 decay=0.16 sustain=0 release=0.02 note=90 lowpass=0.5 vol=0.8"),
    ("tom", "wave=sine slide=12,0.08 attack=0.001 decay=0.35 sustain=0 release=0.02 note=45 vol=1.1"),
    ("crash", "wave=noise attack=0.001 decay=1.3 sustain=0 release=0.2 note=100 lowpass=0.8 vol=0.35"),
];

/// Drum letters in a `@drums` channel.
pub const DRUMS: &[(char, &str)] = &[('k', "kick"), ('s', "snare"), ('h', "hat"), ('o', "openhat"), ('c', "clap"), ('t', "tom"), ('x', "crash")];

/// Built-in sound effects (the same text a cart writes after `sfx <name>`).
pub const SFX: &[(&str, &str)] = &[
    ("coin", "notes=\"o6 l32 b >e8\" inst=square vol=0.6 bpm=150"),
    ("pickup", "notes=\"o5 l32 c e g >c8\" inst=square vol=0.55 bpm=150"),
    ("key", "notes=\"o6 l32 g >d g8\" inst=bell vol=0.8 bpm=150"),
    ("heal", "notes=\"o5 l24 c e g >c e g8\" inst=softlead vol=0.8 bpm=150"),
    ("powerup", "wave=square from=C4 to=C6 len=0.35 env=flat vibrato=0.6,18 vol=0.45"),
    ("jump", "wave=square duty=0.25 from=D4 to=A5 len=0.16 vol=0.5"),
    ("hit", "wave=noise from=C4 to=C2 len=0.12 vol=0.7"),
    ("hurt", "wave=square duty=0.5 from=A4 to=C3 len=0.28 vibrato=0.8,25 vol=0.55"),
    ("swing", "wave=noise from=C7 to=C5 len=0.1 vol=0.35"),
    ("laser", "wave=saw from=C7 to=C4 len=0.18 vol=0.45"),
    ("explosion", "wave=noise from=G3 to=C1 len=0.7 vol=0.9"),
    ("blip", "wave=square duty=0.5 from=A5 to=A5 len=0.05 vol=0.4"),
    ("select", "notes=\"o5 l32 a >e8\" inst=pulse vol=0.5 bpm=150"),
    ("door", "wave=triangle noise=0.3 from=C3 to=G2 len=0.3 env=flat vol=0.7"),
    ("locked", "notes=\"o3 l16 c r c\" inst=square vol=0.5 bpm=150"),
    ("talk", "wave=square duty=0.25 from=E5 to=G5 len=0.04 vol=0.3"),
    ("step", "wave=noise from=C4 to=C4 len=0.03 vol=0.2"),
    ("magic", "notes=\"o6 l48 c e g b >d f a8\" inst=bell vol=0.5 bpm=120"),
    ("win", "notes=\"o5 l12 c e g >c4 r8 <g8 >c2\" inst=lead vol=0.7 bpm=140"),
    ("lose", "notes=\"o4 l8 g f# f e2\" inst=lead vol=0.6 bpm=100"),
    ("start", "notes=\"o5 l16 c g >c e g4\" inst=square vol=0.5 bpm=150"),
];

/// Built-in songs: examples to copy from, and music for carts that don't write their own.
pub const SONGS: &[(&str, &str)] = &[
    ("adventure", "song adventure bpm=132
  lead @lead o5 l8 [e4 g c > c4 < b g e | f4 a > c < b a g4. | e4 g > c e d c < g | a g f d c2]2
  harm @pulse v8 o4 l8 [c e g e c e g e | c f a f c f a f | c e g e c e g e | < b > d g d < b > d g d]2
  bass @bass o2 l4 [c c g g | f f f g | a a e e | f g c c]2
  drums @drums [k8 h8 s8 h8 k8 k8 s8 h8]8"),
    ("cave", "song cave bpm=84
  lead @flute o5 l4 [a2 > c < b8 a8 | e2. r | f2 e8 d8 e | a1]2
  pad @pad v9 o3 l1 [a | e | f | a]2
  bass @pluckbass o2 l4 [a r a r | e r e r | f r f r | a r a r]2
  drums @drums [k2 r4 h8 h8 | r2 t4 r4]4"),
    ("village", "song village bpm=108
  lead @marimba o5 l8 [c d e g e d c4 | d e f a g f e4 | e f g > c < b a g4 | f e d g c2]2
  chords @epiano v9 o4 l2 [c e | < b > d | c f | < g > c]2
  bass @pluckbass o2 l4 [c g e g | g d g d | a e f c | g g c c]2
  drums @drums [k4 h4 s4 h4]8"),
    ("boss", "song boss bpm=168
  lead @brass o4 l8 [d d f d g f e c | d d f d a g f e]2 [> d4 c4 < a4 g4 | f4 e4 d2]2
  bass @synbass o2 l8 [d d > d < d d d > d < d]4 [b- b- > b- < b- a a > a < a]2
  drums @drums [k8 h8 s8 k8 k8 h8 s8 h8]6 [k8 k8 s8 k8 x4 s8 s8]2"),
    ("victory", "song victory bpm=150 once
  lead @lead o5 l8 g g g e4 g4 > c2. r4
  harm @pulse v8 o4 l8 e e e c4 e4 g2. r4
  bass @bass o2 l4 c c c c c2. r4
  drums @drums k8 k8 k8 s4 s4 x2. r4"),
];

fn wave_of(s: &str) -> Option<Wave> {
    Some(match s {
        "square" | "pulse" => Wave::Square,
        "triangle" | "tri" => Wave::Triangle,
        "saw" => Wave::Saw,
        "sine" => Wave::Sine,
        "noise" => Wave::Noise,
        "pluck" => Wave::Pluck,
        "fm" => Wave::Fm,
        _ => return None,
    })
}

fn nums(v: &str, n: usize, key: &str) -> Result<Vec<f32>, String> {
    let out: Vec<f32> = v.split(',').map(|x| x.trim().parse::<f32>()).collect::<Result<_, _>>().map_err(|_| format!("`{key}={v}`: expected {n} number(s)"))?;
    if out.len() > n || out.is_empty() {
        return Err(format!("`{key}={v}`: expected up to {n} comma-separated numbers"));
    }
    Ok(out)
}

/// A note name (`C4`, `f#5`, `Bb3`) or a frequency in Hz → midi number (fractional ok).
pub fn pitch(s: &str) -> Option<f32> {
    if let Ok(h) = s.parse::<f32>() {
        if h <= 0.0 {
            return None;
        }
        // midi = 69 + 12·log2(h/440), by bisection on our exp2 (no libm log)
        let (mut lo, mut hi) = (-24.0f32, 160.0f32);
        for _ in 0..40 {
            let mid = (lo + hi) / 2.0;
            if hz(mid) < h { lo = mid } else { hi = mid }
        }
        return Some((lo + hi) / 2.0);
    }
    let c: Vec<char> = s.to_lowercase().chars().collect();
    let base = match c.first()? {
        'c' => 0, 'd' => 2, 'e' => 4, 'f' => 5, 'g' => 7, 'a' => 9, 'b' => 11,
        _ => return None,
    };
    let mut i = 1;
    let mut acc = 0;
    while i < c.len() && matches!(c[i], '#' | '+' | 'b' | '-') {
        acc += if matches!(c[i], '#' | '+') { 1 } else { -1 };
        i += 1;
    }
    let oct: i32 = c[i..].iter().collect::<String>().parse().ok()?;
    Some(((oct + 1) * 12 + base + acc) as f32)
}

/// `key=value` words after `instrument <name>` → a patch (`from=` starts from another).
pub fn parse_patch(words: &[&str], lookup: &dyn Fn(&str) -> Option<Patch>) -> Result<Patch, String> {
    let mut p = Patch::default();
    if let Some(f) = words.iter().find_map(|w| w.strip_prefix("from=")) {
        p = lookup(f).ok_or_else(|| format!("`from={f}`: no instrument called `{f}`"))?;
    }
    for w in words {
        let (k, v) = w.split_once('=').ok_or_else(|| format!("`{w}`: expected key=value"))?;
        let one = || nums(v, 1, k).map(|x| x[0]);
        match k {
            "from" => {}
            "wave" => p.wave = wave_of(v).ok_or_else(|| format!("wave `{v}`: use square, triangle, saw, sine, noise, pluck, fm"))?,
            "duty" => p.duty = { let d = one()?; if d > 1.0 { d / 100.0 } else { d } }.clamp(0.01, 0.99),
            "attack" => p.attack = one()?.max(0.0),
            "decay" => p.decay = one()?.max(0.001),
            "sustain" => p.sustain = one()?.clamp(0.0, 1.0),
            "release" => p.release = one()?.max(0.0),
            "vol" => p.vol = one()?.clamp(0.0, 4.0),
            "lowpass" => p.lowpass = one()?.clamp(0.01, 1.0),
            "detune" => p.detune = one()?,
            "noise" => p.noise = one()?.clamp(0.0, 1.0),
            "transpose" => p.transpose = one()?,
            "note" => p.note = Some(pitch(v).ok_or_else(|| format!("note `{v}`: a note like C4 or a midi number"))?),
            "vibrato" => {
                let x = nums(v, 3, k)?;
                p.vib = (x[0], *x.get(1).unwrap_or(&5.5), *x.get(2).unwrap_or(&0.15));
            }
            "slide" => {
                let x = nums(v, 2, k)?;
                p.slide = (x[0], *x.get(1).unwrap_or(&0.05));
            }
            "fm" => {
                let x = nums(v, 3, k)?;
                p.fm = (x[0], *x.get(1).unwrap_or(&1.0), *x.get(2).unwrap_or(&0.0));
            }
            _ => return Err(format!("unknown instrument setting `{k}` (wave duty attack decay sustain release vol vibrato slide lowpass detune fm noise transpose note from)")),
        }
    }
    if let Some(n) = words.iter().find_map(|w| w.strip_prefix("note=")) {
        if n.parse::<f32>().is_ok() {
            p.note = n.parse::<f32>().ok(); // a bare number is a midi note, not Hz
        }
    }
    Ok(p)
}

// ---------- MML ----------

#[derive(Clone, Debug)]
pub enum Act {
    /// midi, velocity 0..1, patch index
    On(f32, f32, u16),
    Off,
}

/// A timed list of actions for one voice, in samples.
#[derive(Clone, Debug, Default)]
pub struct Seq {
    pub acts: Vec<(u64, Act)>,
    pub end: u64,
    /// for check/docs: notes, lowest/highest midi, length in ticks
    pub notes: usize,
    pub range: (f32, f32),
    pub ticks: u32,
}

/// Expand `[ ... ]N` repeats (N defaults to 2), nested.
fn expand(s: &str) -> Result<String, String> {
    let c: Vec<char> = s.chars().collect();
    fn go(c: &[char], i: &mut usize, depth: usize) -> Result<String, String> {
        let mut out = String::new();
        while *i < c.len() {
            match c[*i] {
                '[' => {
                    *i += 1;
                    let inner = go(c, i, depth + 1)?;
                    // go() returns at the matching ']'
                    if *i >= c.len() {
                        return Err("`[` without a matching `]`".into());
                    }
                    *i += 1;
                    let st = *i;
                    while *i < c.len() && c[*i].is_ascii_digit() {
                        *i += 1;
                    }
                    let n: usize = if st == *i { 2 } else { c[st..*i].iter().collect::<String>().parse().unwrap_or(2) };
                    if n == 0 || n > 64 {
                        return Err(format!("repeat count {n}: use 1..64"));
                    }
                    for _ in 0..n {
                        out.push_str(&inner);
                        out.push(' ');
                    }
                    if out.len() > 200_000 {
                        return Err("repeats expand to more than 200k characters".into());
                    }
                }
                ']' => {
                    if depth == 0 {
                        return Err("`]` without a matching `[`".into());
                    }
                    return Ok(out);
                }
                ch => {
                    out.push(ch);
                    *i += 1;
                }
            }
        }
        if depth > 0 {
            return Err("`[` without a matching `]`".into());
        }
        Ok(out)
    }
    let mut i = 0;
    go(&c, &mut i, 0)
}

/// Parse one channel (or phrase) of MML into actions. `inst` resolves `@name` to a patch index.
pub fn parse_mml(src: &str, bpm: &mut u32, allow_tempo: bool, start_inst: Option<u16>, inst: &dyn Fn(&str) -> Option<u16>, drum: &dyn Fn(char) -> Option<u16>) -> Result<Seq, String> {
    let s = expand(src)?;
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let (mut oct, mut len, mut vol, mut gate) = (4i32, 48u32, 12u32, 7u32);
    let mut cur = start_inst;
    let mut drums = false;
    let mut tick = 0u32;
    // (tick_on, ticks, midi, vel, patch)
    let mut notes: Vec<(u32, u32, f32, f32, u16)> = vec![];
    let num = |c: &[char], i: &mut usize| -> Option<u32> {
        let st = *i;
        while *i < c.len() && c[*i].is_ascii_digit() {
            *i += 1;
        }
        if st == *i { None } else { c[st..*i].iter().collect::<String>().parse().ok() }
    };
    let length = |c: &[char], i: &mut usize, dflt: u32| -> Result<u32, String> {
        let mut t = match num(c, i) {
            None => dflt,
            Some(n) if (1..=TICKS_WHOLE).contains(&n) && TICKS_WHOLE % n == 0 => TICKS_WHOLE / n,
            Some(n) => return Err(format!("length {n}: use 1 2 3 4 6 8 12 16 24 32 48 64 96")),
        };
        let mut add = t / 2;
        while *i < c.len() && c[*i] == '.' {
            t += add;
            add /= 2;
            *i += 1;
        }
        Ok(t)
    };
    while i < c.len() {
        let ch = c[i].to_ascii_lowercase();
        i += 1;
        match ch {
            ' ' | '\t' | '|' | '\n' | ',' => {}
            '@' => {
                let st = i;
                while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '_') {
                    i += 1;
                }
                let name: String = c[st..i].iter().collect();
                if name == "drums" {
                    drums = true;
                } else {
                    drums = false;
                    cur = Some(inst(&name).ok_or_else(|| format!("`@{name}`: no instrument called `{name}`"))?);
                }
            }
            d if drums && DRUMS.iter().any(|x| x.0 == d) => {
                let p = drum(d).ok_or_else(|| format!("drum `{d}`: its instrument is missing"))?;
                let t = length(&c, &mut i, len)?;
                notes.push((tick, t, f32::NAN, vol as f32 / 15.0, p));
                tick += t;
            }
            'o' => oct = num(&c, &mut i).ok_or("`o` needs an octave number, e.g. o4")? as i32,
            '>' => oct += 1,
            '<' => oct -= 1,
            'l' => len = length(&c, &mut i, len)?,
            'v' => vol = num(&c, &mut i).ok_or("`v` needs 0-15")?.min(15),
            'q' => gate = num(&c, &mut i).ok_or("`q` needs 1-8")?.clamp(1, 8),
            't' => {
                let n = num(&c, &mut i).ok_or("`t` needs a tempo")?;
                if !allow_tempo {
                    return Err("set the tempo with bpm= on the song line, not `t` inside a channel".into());
                }
                *bpm = n.clamp(30, 400);
            }
            '^' => {
                let t = length(&c, &mut i, len)?;
                match notes.last_mut() {
                    Some(n) if n.0 + n.1 == tick => n.1 += t,
                    _ => return Err("`^` (tie) must follow a note".into()),
                }
                tick += t;
            }
            'r' => tick += length(&c, &mut i, len)?,
            d if drums && d.is_ascii_alphabetic() => {
                return Err(format!("`{d}` in a @drums channel: use k s h o c t x (kick snare hat openhat clap tom crash) or r"));
            }
            'a'..='g' => {
                let base = match ch { 'c' => 0, 'd' => 2, 'e' => 4, 'f' => 5, 'g' => 7, 'a' => 9, _ => 11 };
                let mut acc = 0;
                while i < c.len() && matches!(c[i], '#' | '+' | '-') {
                    acc += if c[i] == '-' { -1 } else { 1 };
                    i += 1;
                }
                let t = length(&c, &mut i, len)?;
                let p = cur.ok_or("a note before any instrument: start the channel with @name (e.g. @lead)")?;
                let midi = ((oct + 1) * 12 + base + acc) as f32;
                notes.push((tick, t, midi, vol as f32 / 15.0, p));
                tick += t;
            }
            x => return Err(format!("`{x}` is not MML (notes a-g with # or -, r rest, o4 < > octave, l8 length, v0-15 volume, q1-8 gate, @instrument, [ ]N repeat, ^ tie)")),
        }
    }
    // ticks → samples
    let b = *bpm as u64;
    let at = |t: u32| t as u64 * (RATE as u64 * 60 / 48) / b; // 48 ticks per beat
    let mut seq = Seq { acts: vec![], end: at(tick), notes: notes.len(), range: (f32::MAX, f32::MIN), ticks: tick };
    for (t0, dur, midi, vel, p) in notes {
        let on = at(t0);
        let off = at(t0) + (at(t0 + dur) - at(t0)) * gate as u64 / 8;
        seq.acts.push((on, Act::On(midi, vel, p)));
        seq.acts.push((off.max(on + 1), Act::Off));
        if midi.is_finite() {
            seq.range = (seq.range.0.min(midi), seq.range.1.max(midi));
        }
    }
    seq.acts.sort_by_key(|a| (a.0, matches!(a.1, Act::On(..)))); // an Off before an On at the same time
    Ok(seq)
}

// ---------- the bank: everything a cart can play ----------

#[derive(Clone, Debug)]
pub struct Song {
    pub name: String,
    pub bpm: u32,
    pub looped: bool,
    pub vol: f32,
    pub channels: Vec<(String, Seq)>,
    pub end: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Bank {
    pub patches: Vec<Patch>,
    pub inst_names: HashMap<String, u16>,
    pub sfx: HashMap<String, Rc<Seq>>,
    pub songs: HashMap<String, Rc<Song>>,
    /// user-declared names, for `check` and docs
    pub declared: Vec<String>,
}

impl Bank {
    pub fn new() -> Bank {
        let mut b = Bank::default();
        for (n, d) in INSTRUMENTS {
            let words: Vec<&str> = d.split_whitespace().collect();
            let p = parse_patch(&words, &|_| None).expect("preset instrument");
            b.add_patch(n, p);
        }
        for (n, d) in SFX {
            let s = b.parse_sfx(d).unwrap_or_else(|e| panic!("preset sfx {n}: {e}"));
            b.sfx.insert(n.to_string(), Rc::new(s));
        }
        for (n, d) in SONGS {
            let lines: Vec<&str> = d.lines().collect();
            let mut i = 0;
            let s = b.parse_song(&lines, &mut i).unwrap_or_else(|e| panic!("preset song {n}: {e:?}"));
            b.songs.insert(n.to_string(), Rc::new(s));
        }
        b
    }

    fn add_patch(&mut self, name: &str, p: Patch) {
        match self.inst_names.get(name) {
            Some(&i) => self.patches[i as usize] = p,
            None => {
                self.inst_names.insert(name.to_string(), self.patches.len() as u16);
                self.patches.push(p);
            }
        }
    }

    pub fn instrument(&mut self, name: &str, words: &[&str]) -> Result<(), String> {
        let p = {
            let lookup = |n: &str| self.inst_names.get(n).map(|&i| self.patches[i as usize].clone());
            parse_patch(words, &lookup)?
        };
        self.add_patch(name, p);
        self.declared.push(format!("instrument {name}"));
        Ok(())
    }

    /// The text after `sfx <name>`: a sweep (`wave= from= to= len=`) or a phrase (`notes="..."`).
    pub fn parse_sfx(&mut self, def: &str) -> Result<Seq, String> {
        let mut rest = def.to_string();
        let mut notes = None;
        if let Some(i) = rest.find("notes=\"") {
            let j = rest[i + 7..].find('"').ok_or("notes=\"...\" is missing its closing quote")? + i + 7;
            notes = Some(rest[i + 7..j].to_string());
            rest = format!("{} {}", &rest[..i], &rest[j + 1..]);
        }
        let words: Vec<&str> = rest.split_whitespace().collect();
        let get = |k: &str| words.iter().find_map(|w| w.strip_prefix(k).and_then(|r| r.strip_prefix('=')));
        let vol = get("vol").map(|v| v.parse::<f32>().map_err(|_| format!("vol={v}: a number"))).transpose()?.unwrap_or(0.7);
        if let Some(n) = notes {
            let inst = get("inst").unwrap_or("square");
            let pi = *self.inst_names.get(inst).ok_or_else(|| format!("inst={inst}: no instrument called `{inst}`"))?;
            let mut bpm = get("bpm").and_then(|b| b.parse().ok()).unwrap_or(150);
            let names = self.inst_names.clone();
            let mut s = parse_mml(&n, &mut bpm, true, Some(pi), &|x| names.get(x).copied(), &|d| drum_index(&names, d))?;
            for a in &mut s.acts {
                if let Act::On(_, v, _) = &mut a.1 {
                    *v *= vol;
                }
            }
            return Ok(s);
        }
        // a sweep: one note that glides from `from` to `to` over `len` seconds
        let known = ["wave", "from", "to", "len", "vol", "duty", "vibrato", "noise", "env", "lowpass"];
        for w in &words {
            let k = w.split('=').next().unwrap_or("");
            if !known.contains(&k) {
                return Err(format!("sfx: unknown setting `{k}` (sweep: wave from to len vol duty vibrato noise env lowpass; or notes=\"...\" inst= bpm= vol=)"));
            }
        }
        let from = pitch(get("from").unwrap_or("C5")).ok_or("from=: a note (C5) or Hz (523)")?;
        let to = match get("to") { Some(t) => pitch(t).ok_or("to=: a note (C5) or Hz (523)")?, None => from };
        let len = get("len").unwrap_or("0.2").parse::<f32>().map_err(|_| "len=: seconds")?.clamp(0.01, 5.0);
        let mut w: Vec<String> = vec![format!("wave={}", get("wave").unwrap_or("square"))];
        for k in ["duty", "vibrato", "noise", "lowpass"] {
            if let Some(v) = get(k) {
                w.push(format!("{k}={v}"));
            }
        }
        match get("env").unwrap_or("decay") {
            "decay" => w.extend(["attack=0.002".into(), format!("decay={}", len * 1.3), "sustain=0".into(), "release=0.02".into()]),
            "flat" => w.extend(["attack=0.005".into(), "decay=0.1".into(), "sustain=1".into(), "release=0.03".into()]),
            "swell" => w.extend([format!("attack={}", len * 0.8), "decay=0.1".into(), "sustain=1".into(), "release=0.05".into()]),
            e => return Err(format!("env={e}: decay, flat or swell")),
        }
        w.push(format!("slide={},{}", from - to, len));
        let wr: Vec<&str> = w.iter().map(|s| s.as_str()).collect();
        let p = parse_patch(&wr, &|_| None)?;
        let pi = self.patches.len() as u16;
        self.patches.push(p);
        let end = (len * RATE as f32) as u64;
        Ok(Seq { acts: vec![(0, Act::On(to, vol, pi)), (end, Act::Off)], end: end + 1, notes: 1, range: (to.min(from), to.max(from)), ticks: 0 })
    }

    pub fn sfx_def(&mut self, name: &str, def: &str) -> Result<(), String> {
        let s = self.parse_sfx(def)?;
        self.sfx.insert(name.to_string(), Rc::new(s));
        self.declared.push(format!("sfx {name}"));
        Ok(())
    }

    /// `song name bpm=N [once] [vol=x]` at `lines[*i]`, then channel lines until the next
    /// assets.cw header. `line0` = file line of `lines[0]` (for messages).
    pub fn parse_song(&mut self, lines: &[&str], i: &mut usize) -> Result<Song, (usize, String)> {
        let at = *i;
        let head: Vec<&str> = strip(lines[*i]).split_whitespace().collect();
        let name = head.get(1).ok_or((at, "song needs a name".to_string()))?.to_string();
        let mut bpm = 120u32;
        let (mut looped, mut vol) = (true, 1.0f32);
        for w in &head[2..] {
            match w.split_once('=') {
                Some(("bpm", v)) => bpm = v.parse().map_err(|_| (at, format!("bpm={v}: a number")))?,
                Some(("vol", v)) => vol = v.parse().map_err(|_| (at, format!("vol={v}: a number")))?,
                None if *w == "once" => looped = false,
                None if *w == "loop" => looped = true,
                _ => return Err((at, format!("song `{name}`: unknown `{w}` (bpm=N, vol=x, once)"))),
            }
        }
        bpm = bpm.clamp(30, 400);
        // channel name → its MML (lines with the same name join up)
        let mut chans: Vec<(String, String, usize)> = vec![];
        *i += 1;
        while *i < lines.len() {
            let l = strip(lines[*i]).trim();
            if crate::assets::is_header(lines[*i]) {
                break;
            }
            if !l.is_empty() {
                let (ch, mml) = l.split_once(char::is_whitespace).unwrap_or((l, ""));
                match chans.iter_mut().find(|c| c.0 == ch) {
                    Some(c) => {
                        c.1.push(' ');
                        c.1.push_str(mml);
                    }
                    None => chans.push((ch.to_string(), mml.to_string(), *i)),
                }
            }
            *i += 1;
        }
        if chans.is_empty() {
            return Err((at, format!("song `{name}` has no channels: indent lines like `lead @lead o5 l8 c d e`")));
        }
        if chans.len() > MAX_SONG_CHANNELS {
            return Err((at, format!("song `{name}` has {} channels; max {MAX_SONG_CHANNELS} (sound effects need the rest)", chans.len())));
        }
        let names = self.inst_names.clone();
        let mut out = vec![];
        for (ch, mml, li) in chans {
            let mut b = bpm;
            let s = parse_mml(&mml, &mut b, false, None, &|x| names.get(x).copied(), &|d| drum_index(&names, d))
                .map_err(|e| (li, format!("song `{name}` channel `{ch}`: {e}")))?;
            out.push((ch, s));
        }
        let end = out.iter().map(|c| c.1.end).max().unwrap_or(0);
        Ok(Song { name, bpm, looped, vol, channels: out, end })
    }

    pub fn song_def(&mut self, lines: &[&str], i: &mut usize) -> Result<(), (usize, String)> {
        let s = self.parse_song(lines, i)?;
        self.declared.push(format!("song {}", s.name));
        self.songs.insert(s.name.clone(), Rc::new(s));
        Ok(())
    }

    /// Human summary for `check`: bars per channel, and uneven channels (a common mistake).
    pub fn describe(&self) -> Vec<String> {
        let mut out = vec![];
        for d in &self.declared {
            if let Some(n) = d.strip_prefix("song ") {
                if let Some(s) = self.songs.get(n) {
                    let bars = |t: u32| t as f32 / TICKS_WHOLE as f32;
                    let longest = s.channels.iter().map(|c| c.1.ticks).max().unwrap_or(0);
                    let chans: Vec<String> = s.channels.iter().map(|(n, q)| format!("{n} {:.1} bars", bars(q.ticks))).collect();
                    out.push(format!("song {n}: {} bpm, {:.1} s, {}", s.bpm, s.end as f32 / RATE as f32, chans.join(", ")));
                    for (cn, q) in &s.channels {
                        if q.ticks < longest {
                            out.push(format!("warning: song {n}: channel `{cn}` is {:.1} bars but the song is {:.1}: it goes quiet before the loop", bars(q.ticks), bars(longest)));
                        }
                    }
                }
            }
        }
        out
    }
}

fn drum_index(names: &HashMap<String, u16>, d: char) -> Option<u16> {
    DRUMS.iter().find(|x| x.0 == d).and_then(|x| names.get(x.1).copied())
}

fn strip(l: &str) -> &str {
    if l.trim_start().starts_with("--") {
        return "";
    }
    match l.find(" --") {
        Some(i) => &l[..i],
        None => l,
    }
}

// ---------- playback ----------

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Off,
    Attack,
    Decay,
    Release,
}

#[derive(Clone)]
struct Voice {
    stage: Stage,
    p: Patch,
    midi: f32,
    vel: f32,
    env: f32,
    k_decay: f32,
    k_release: f32,
    k_fm: f32,
    fm_i: f32,
    phase: u32,
    phase2: u32,
    mphase: u32,
    inc: u32,
    inc2: u32,
    minc: u32,
    t: u32, // samples since note on
    lfsr: u32,
    nval: f32,
    ks: Vec<f32>,
    ks_pos: usize,
    ks_damp: f32,
    lp: f32,
    age: u64,
    stage_was_off: bool,
}

impl Voice {
    fn new() -> Voice {
        Voice {
            stage: Stage::Off, p: Patch::default(), midi: 60.0, vel: 0.0, env: 0.0, k_decay: 0.0, k_release: 0.0, k_fm: 0.0,
            fm_i: 0.0, phase: 0, phase2: 0, mphase: 0, inc: 0, inc2: 0, minc: 0, t: 0, lfsr: 0x4a5d, nval: 0.0, ks: vec![],
            ks_pos: 0, ks_damp: 0.99, lp: 0.0, age: 0, stage_was_off: true,
        }
    }

    fn rnd(&mut self) -> f32 {
        let bit = (self.lfsr ^ (self.lfsr >> 1)) & 1;
        self.lfsr = (self.lfsr >> 1) | (bit << 14);
        if self.lfsr & 1 == 1 { 1.0 } else { -1.0 }
    }

    fn on(&mut self, p: &Patch, midi: f32, vel: f32, age: u64) {
        self.stage_was_off = self.stage == Stage::Off;
        self.p = p.clone();
        self.midi = if midi.is_finite() { midi } else { p.note.unwrap_or(60.0) } + p.transpose;
        self.vel = vel;
        self.stage = Stage::Attack;
        if p.attack <= 0.0 {
            self.env = 1.0;
            self.stage = Stage::Decay;
        }
        self.k_decay = decay_k(p.decay);
        self.k_release = decay_k(p.release.max(0.005));
        self.fm_i = p.fm.1;
        self.k_fm = if p.fm.2 > 0.0 { decay_k(p.fm.2) } else { 1.0 };
        // a voice that is still sounding keeps its phase: resetting it mid-wave clicks
        if self.stage_was_off {
            self.phase = 0;
            self.phase2 = 0x4000_0000;
            self.mphase = 0;
            self.lp = 0.0;
        }
        self.t = 0;
        self.age = age;
        self.pitch();
        if p.wave == Wave::Pluck {
            let n = ((RATE as f32 / hz(self.midi)) as usize).clamp(2, 2400);
            self.ks = (0..n).map(|_| self.rnd() * 0.8).collect();
            self.ks_pos = 0;
            self.ks_damp = exp2f(-10.0 / (p.decay.max(0.05) * hz(self.midi))).min(0.9995);
        }
    }

    fn off(&mut self) {
        if self.stage != Stage::Off {
            self.stage = Stage::Release;
        }
    }

    /// Recompute the phase increments (vibrato, slide) — every CTL samples.
    fn pitch(&mut self) {
        let secs = self.t as f32 / RATE as f32;
        let mut m = self.midi;
        let (sl, st) = self.p.slide;
        if sl != 0.0 && secs < st {
            m += sl * (1.0 - secs / st);
        }
        let (vd, vr, vdel) = self.p.vib;
        if vd != 0.0 && secs > vdel {
            m += vd * sin_turn((secs - vdel) * vr);
        }
        self.inc = inc(m);
        self.inc2 = if self.p.detune != 0.0 { inc(m + self.p.detune) } else { 0 };
        self.minc = if self.p.wave == Wave::Fm { ((self.inc as f64) * self.p.fm.0 as f64).min(2e9) as u32 } else { 0 };
    }

    fn osc(&mut self, phase: u32) -> f32 {
        let ph = phase as f32 / 4_294_967_296.0;
        match self.p.wave {
            Wave::Square => if ph < self.p.duty { 1.0 } else { -1.0 },
            Wave::Triangle => if ph < 0.5 { 4.0 * ph - 1.0 } else { 3.0 - 4.0 * ph },
            Wave::Saw => 2.0 * ph - 1.0,
            Wave::Sine => sin_turn(ph),
            Wave::Fm => {
                let m = sin_turn(self.mphase as f32 / 4_294_967_296.0);
                sin_turn(ph + self.fm_i * m / std::f32::consts::TAU)
            }
            Wave::Noise | Wave::Pluck => 0.0,
        }
    }

    fn sample(&mut self) -> f32 {
        if self.stage == Stage::Off {
            return 0.0;
        }
        if self.t % CTL == 0 && self.t > 0 {
            self.pitch();
        }
        // envelope
        match self.stage {
            Stage::Attack => {
                self.env += 1.0 / (self.p.attack * RATE as f32);
                if self.env >= 1.0 {
                    self.env = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => self.env = self.p.sustain + (self.env - self.p.sustain) * self.k_decay,
            Stage::Release => {
                self.env *= self.k_release;
                if self.env < 0.0005 {
                    self.stage = Stage::Off;
                    self.env = 0.0;
                }
            }
            Stage::Off => {}
        }
        let mut x = match self.p.wave {
            Wave::Noise => {
                let (p, o) = self.phase.overflowing_add(self.inc.saturating_mul(4));
                self.phase = p;
                if o {
                    self.nval = self.rnd();
                }
                self.nval
            }
            Wave::Pluck => {
                let n = self.ks.len();
                let a = self.ks[self.ks_pos];
                let b = self.ks[(self.ks_pos + 1) % n];
                self.ks[self.ks_pos] = (a + b) * 0.5 * self.ks_damp;
                self.ks_pos = (self.ks_pos + 1) % n;
                a
            }
            _ => {
                let a = self.osc(self.phase);
                self.phase = self.phase.wrapping_add(self.inc);
                let v = if self.inc2 != 0 {
                    let b = self.osc(self.phase2);
                    self.phase2 = self.phase2.wrapping_add(self.inc2);
                    (a + b) * 0.6
                } else {
                    a
                };
                if self.p.wave == Wave::Fm {
                    self.mphase = self.mphase.wrapping_add(self.minc);
                    self.fm_i *= self.k_fm;
                }
                v
            }
        };
        if self.p.noise > 0.0 {
            let (p, o) = self.phase2.overflowing_add(0x2000_0000);
            self.phase2 = p;
            if o || self.t == 0 {
                self.nval = self.rnd();
            }
            x = x * (1.0 - self.p.noise) + self.nval * self.p.noise;
        }
        self.lp += self.p.lowpass * (x - self.lp);
        self.t = self.t.wrapping_add(1);
        self.lp * self.env * self.vel * self.p.vol
    }
}

struct Player {
    seq: Rc<Seq>,
    pos: u64,
    ptr: usize,
    voice: usize,
    vol: f32,
    pitch: f32,
}

struct SongPlay {
    song: Rc<Song>,
    pos: u64,
    ptrs: Vec<usize>,
    vol: f32,
    fade: Option<(f32, f32)>, // current gain, per-sample step
    looped: bool,
}

pub struct Mixer {
    pub bank: Bank,
    voices: Vec<Voice>,
    song: Option<SongPlay>,
    sfx: Vec<Player>,
    age: u64,
    pub out: Vec<i16>,
    /// FNV-1a over every sample ever produced
    pub hash: u64,
    pub events: Vec<String>,
    pub peak: i16,
    pub clipped: u64,
    pub loud_frames: u64,
    pub master: f32,
    dc: (f32, f32),
}

impl Mixer {
    pub fn new(bank: Bank) -> Mixer {
        Mixer {
            bank,
            voices: (0..VOICES).map(|_| Voice::new()).collect(),
            song: None,
            sfx: vec![],
            age: 0,
            out: vec![0; PER_FRAME],
            hash: 0xcbf2_9ce4_8422_2325,
            events: vec![],
            peak: 0,
            clipped: 0,
            loud_frames: 0,
            master: 1.0,
            dc: (0.0, 0.0),
        }
    }

    fn song_voices(&self) -> usize {
        self.song.as_ref().map(|s| s.song.channels.len()).unwrap_or(0)
    }

    pub fn sfx(&mut self, name: &str, vol: f32, pitch: f32) -> Result<(), String> {
        let seq = self.bank.sfx.get(name).cloned().ok_or_else(|| {
            let mut n: Vec<&String> = self.bank.sfx.keys().collect();
            n.sort();
            format!("sfx: no sound called `{name}` (built in: {})", n.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "))
        })?;
        self.events.push(format!("sfx {name}"));
        self.start(seq, vol, pitch);
        Ok(())
    }

    pub fn play_mml(&mut self, mml: &str, inst: &str, vol: f32) -> Result<(), String> {
        let def = format!("notes=\"{}\" inst={inst} vol={vol}", mml.replace('"', ""));
        let seq = self.bank.parse_sfx(&def)?;
        self.events.push(format!("play {mml}"));
        self.start(Rc::new(seq), 1.0, 0.0);
        Ok(())
    }

    fn start(&mut self, seq: Rc<Seq>, vol: f32, pitch: f32) {
        // a free sfx voice, else steal the oldest sfx
        let first = self.song_voices();
        let used: Vec<usize> = self.sfx.iter().map(|p| p.voice).collect();
        let v = (first..VOICES).find(|v| !used.contains(v) && self.voices[*v].stage == Stage::Off)
            .or_else(|| (first..VOICES).find(|v| !used.contains(v)))
            .unwrap_or_else(|| {
                let oldest = (0..self.sfx.len()).min_by_key(|&i| self.voices[self.sfx[i].voice].age).unwrap_or(0);
                self.sfx.remove(oldest).voice
            });
        self.sfx.push(Player { seq, pos: 0, ptr: 0, voice: v, vol, pitch });
    }

    pub fn music(&mut self, name: Option<&str>, vol: f32, fade: f32, looped: Option<bool>) -> Result<(), String> {
        match name {
            None => {
                if let Some(s) = &mut self.song {
                    self.events.push("music stop".into());
                    if fade > 0.0 {
                        s.fade = Some((1.0, 1.0 / (fade * RATE as f32)));
                    } else {
                        let n = s.song.channels.len();
                        self.song = None;
                        for v in 0..n {
                            self.voices[v].off();
                        }
                    }
                }
            }
            Some(n) => {
                if self.song.as_ref().is_some_and(|s| s.song.name == n && s.fade.is_none()) {
                    return Ok(()); // already playing: calling music() every frame is fine
                }
                let song = self.bank.songs.get(n).cloned().ok_or_else(|| {
                    let mut k: Vec<&String> = self.bank.songs.keys().collect();
                    k.sort();
                    format!("music: no song called `{n}` (built in: {})", k.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "))
                })?;
                for v in 0..VOICES {
                    if v < song.channels.len() {
                        self.voices[v].off();
                    }
                }
                // sfx sitting on voices the song now needs move out of the way (they end)
                let nch = song.channels.len();
                self.sfx.retain(|p| p.voice >= nch);
                self.events.push(format!("music {n}"));
                self.song = Some(SongPlay { looped: looped.unwrap_or(song.looped), ptrs: vec![0; nch], song, pos: 0, vol, fade: None });
            }
        }
        Ok(())
    }

    pub fn playing(&self) -> Option<String> {
        self.song.as_ref().filter(|s| s.fade.is_none()).map(|s| s.song.name.clone())
    }

    /// Make one frame of sound into `out`.
    pub fn frame(&mut self) {
        let mut loud = false;
        for k in 0..PER_FRAME {
            // song events due now
            let mut song_gain = 0.0;
            let mut end_song = false;
            if let Some(sp) = &mut self.song {
                if sp.pos >= sp.song.end {
                    if sp.looped && sp.song.end > 0 {
                        sp.pos = 0;
                        sp.ptrs.iter_mut().for_each(|p| *p = 0);
                    } else {
                        end_song = true;
                    }
                }
                if !end_song {
                    for (c, (_, seq)) in sp.song.channels.iter().enumerate() {
                        while sp.ptrs[c] < seq.acts.len() && seq.acts[sp.ptrs[c]].0 <= sp.pos {
                            match &seq.acts[sp.ptrs[c]].1 {
                                Act::On(m, v, p) => {
                                    self.age += 1;
                                    let patch = &self.bank.patches[*p as usize];
                                    self.voices[c].on(patch, *m, *v, self.age);
                                }
                                Act::Off => self.voices[c].off(),
                            }
                            sp.ptrs[c] += 1;
                        }
                    }
                    sp.pos += 1;
                    song_gain = sp.vol * sp.song.vol;
                    if let Some((g, step)) = &mut sp.fade {
                        *g -= *step;
                        if *g <= 0.0 {
                            end_song = true;
                        }
                        song_gain *= g.max(0.0);
                    }
                }
            }
            if end_song {
                let n = self.song.as_ref().map(|s| s.song.channels.len()).unwrap_or(0);
                self.song = None;
                for v in 0..n {
                    self.voices[v].off();
                }
            }
            // sfx events
            let mut i = 0;
            while i < self.sfx.len() {
                let p = &mut self.sfx[i];
                while p.ptr < p.seq.acts.len() && p.seq.acts[p.ptr].0 <= p.pos {
                    match &p.seq.acts[p.ptr].1 {
                        Act::On(m, v, pi) => {
                            self.age += 1;
                            let patch = &self.bank.patches[*pi as usize];
                            self.voices[p.voice].on(patch, *m + p.pitch, *v * p.vol, self.age);
                        }
                        Act::Off => self.voices[p.voice].off(),
                    }
                    p.ptr += 1;
                }
                p.pos += 1;
                if p.pos > p.seq.end && p.ptr >= p.seq.acts.len() {
                    self.sfx.remove(i);
                } else {
                    i += 1;
                }
            }
            // mix
            let nsong = self.song_voices();
            let mut mix = 0.0f32;
            for (v, voice) in self.voices.iter_mut().enumerate() {
                let s = voice.sample();
                mix += if v < nsong { s * song_gain } else { s };
            }
            // DC blocker (pulse waves carry an offset that would eat headroom)
            let y = mix - self.dc.0 + 0.997 * self.dc.1;
            self.dc = (mix, y);
            let mut x = y * MASTER * self.master;
            // soft knee above 0.8, hard stop at 1
            let a = x.abs();
            if a > 0.8 {
                let over = a - 0.8;
                let y = 0.8 + over / (1.0 + over * 5.0);
                if y > 0.99 {
                    self.clipped += 1;
                }
                x = y.min(1.0) * x.signum();
            }
            let s = (x * 32767.0) as i16;
            if s.unsigned_abs() > 300 {
                loud = true;
            }
            self.peak = self.peak.max(s.saturating_abs());
            self.out[k] = s;
            for b in s.to_le_bytes() {
                self.hash ^= b as u64;
                self.hash = self.hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        if loud {
            self.loud_frames += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_parse_and_play() {
        let mut m = Mixer::new(Bank::new());
        m.music(Some("adventure"), 1.0, 0.0, None).unwrap();
        m.sfx("coin", 1.0, 0.0).unwrap();
        for _ in 0..120 {
            m.frame();
        }
        assert!(m.peak > 2000, "peak {}", m.peak);
        assert!(m.loud_frames > 100);
    }

    #[test]
    fn deterministic() {
        let run = || {
            let mut m = Mixer::new(Bank::new());
            m.music(Some("boss"), 1.0, 0.0, None).unwrap();
            for f in 0..300 {
                if f % 40 == 0 {
                    m.sfx("explosion", 1.0, 0.0).unwrap();
                }
                m.frame();
            }
            m.hash
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn exp2_close() {
        for i in -200..200 {
            let x = i as f32 / 17.0;
            let r = (exp2f(x) - x.exp2()).abs() / x.exp2();
            assert!(r < 5e-6, "x={x} rel {r}");
        }
    }

    #[test]
    fn mml_errors_are_readable() {
        let b = Bank::new();
        let n = b.inst_names.clone();
        let e = parse_mml("@lead o4 c d h", &mut 120, false, None, &|x| n.get(x).copied(), &|_| None).unwrap_err();
        assert!(e.contains("`h` is not MML"), "{e}");
        let e = parse_mml("c d", &mut 120, false, None, &|x| n.get(x).copied(), &|_| None).unwrap_err();
        assert!(e.contains("instrument"), "{e}");
    }

    #[test]
    fn note_names() {
        assert_eq!(pitch("A4"), Some(69.0));
        assert_eq!(pitch("c#5"), Some(73.0));
        assert!((pitch("440").unwrap() - 69.0).abs() < 0.01);
    }
}
