//! P0 carts are directories: `assets.cw` + `main.lua`. (The .cart.png container is later.)

use kartu_core::{Console, BUTTONS};
use std::path::Path;

pub fn load(dir: &str, seed: u64) -> Result<Console, String> {
    boot(dir, seed, None)
}

/// `load` with the cart's saved string (what its `load()` returns) from a previous session.
pub fn boot(dir: &str, seed: u64, saved: Option<String>) -> Result<Console, String> {
    let (assets, lua) = read(dir)?;
    Console::new_saved(&assets, &lua, seed, saved)
}

pub fn read(dir: &str) -> Result<(String, String), String> {
    let d = Path::new(dir);
    let read = |f: &str| std::fs::read_to_string(d.join(f)).map_err(|e| format!("{}: {e}", d.join(f).display()));
    Ok((read("assets.cw")?, read("main.lua")?))
}

/// An input script. One step per line (or comma-separated):
///
/// ```text
/// # comment (also `--`); blank lines are fine
/// 30:a          hold exactly these buttons from frame 30 on (replaces what was held)
/// 40:           release everything
/// +12:right+up  frame relative to the previous step (here 52)
/// 60:a!         tap: add `a` on top of what is held, for 1 frame
/// 70:b!5        add `b` for 5 frames
/// ```
#[derive(Default, Clone)]
pub struct Script {
    /// (frame, buttons) — held sets, sorted by frame
    holds: Vec<(u64, u16)>,
    /// (from, until exclusive, buttons) — added on top of the held set
    taps: Vec<(u64, u64, u16)>,
}

fn bits_of(names: &str, tok: &str) -> Result<u16, String> {
    let mut bits = 0u16;
    for name in names.split('+').filter(|n| !n.is_empty()) {
        let i = BUTTONS
            .iter()
            .position(|x| *x == name)
            .ok_or_else(|| format!("unknown button `{name}` in `{tok}` (use {})", BUTTONS.join(" ")))?;
        bits |= 1 << i;
    }
    Ok(bits)
}

impl Script {
    #[cfg(test)]
    pub fn parse(s: &str) -> Result<Script, String> {
        let mut sc = Script::default();
        sc.add(s)?;
        Ok(sc)
    }

    /// Add more steps (`--script` then `--press`); relative frames restart from 0.
    pub fn add(&mut self, s: &str) -> Result<(), String> {
        let mut prev = 0u64;
        for line in s.lines() {
            let line = line.split('#').next().unwrap_or("");
            let line = line.split("--").next().unwrap_or("");
            for tok in line.split([',', ' ', '\t']).map(str::trim).filter(|t| !t.is_empty()) {
                let (f, b) = tok.split_once(':').ok_or_else(|| format!("bad input step `{tok}` (want FRAME:btn+btn)"))?;
                let f: u64 = match f.strip_prefix('+') {
                    Some(r) => prev + r.parse::<u64>().map_err(|_| format!("bad frame in `{tok}`"))?,
                    None => f.parse().map_err(|_| format!("bad frame in `{tok}`"))?,
                };
                prev = f;
                match b.split_once('!') {
                    Some((names, n)) => {
                        let n: u64 = if n.is_empty() { 1 } else { n.parse().map_err(|_| format!("bad tap length in `{tok}`"))? };
                        self.taps.push((f, f + n, bits_of(names, tok)?));
                    }
                    None => self.holds.push((f, bits_of(b, tok)?)),
                }
            }
        }
        self.holds.sort_by_key(|x| x.0);
        Ok(())
    }

    pub fn buttons_at(&self, frame: u64) -> u16 {
        let held = self.holds.iter().take_while(|(f, _)| *f <= frame).last().map(|x| x.1).unwrap_or(0);
        self.taps.iter().filter(|(a, b, _)| (*a..*b).contains(&frame)).fold(held, |acc, t| acc | t.2)
    }
}

/// Turns per-frame button bits back into a script: one `F:btn+btn` line per change.
#[derive(Default)]
pub struct Recorder {
    last: u16,
    pub out: String,
}

impl Recorder {
    pub fn push(&mut self, frame: u64, bits: u16) {
        if bits != self.last {
            let names: Vec<&str> = BUTTONS.iter().enumerate().filter(|(i, _)| bits & (1 << i) != 0).map(|(_, n)| *n).collect();
            self.out.push_str(&format!("{frame}:{}\n", names.join("+")));
            self.last = bits;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_syntax() {
        let s = Script::parse("# start\n10:right -- walk\n+5:\n20:a!\n30:up\n31:b!3, 40:").unwrap();
        let bits = |f| s.buttons_at(f);
        assert_eq!((bits(9), bits(10), bits(14), bits(15)), (0, 2, 2, 0));
        assert_eq!((bits(20), bits(21)), (1 << 4, 0));
        assert_eq!((bits(31), bits(33), bits(34)), (4 | 32, 4 | 32, 4));
        assert_eq!(bits(40), 0);
    }

    #[test]
    fn recorder_round_trips() {
        let s = Script::parse("3:left 5:left+a 9:").unwrap();
        let mut r = Recorder::default();
        for f in 0..12 {
            r.push(f, s.buttons_at(f));
        }
        assert_eq!(r.out, "3:left\n5:left+a\n9:\n");
        let back = Script::parse(&r.out).unwrap();
        assert!((0..12).all(|f| back.buttons_at(f) == s.buttons_at(f)));
    }
}
