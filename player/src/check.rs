//! `kartu check <cart>`: every problem in one pass, before a single frame is judged.
//!
//! 1. assets.cw: all parse errors (not just the first)
//! 2. main.lua: syntax
//! 3. names: string literals passed to spr / bg.map / bg.tile / pal that assets.cw lacks
//! 4. a short smoke run (init + N frames, optional input) for runtime errors and dropped sprites

use crate::cart::{self, Script};
use crate::Args;
use kartu_core::assets::{Assets, Ref};
use kartu_core::Console;

pub fn check(a: &[String]) -> Result<(), String> {
    let a = Args::parse(a);
    let dir = a.cart()?;
    let (assets_src, lua_src) = cart::read(&dir)?;
    let mut errors: Vec<String> = vec![];
    let mut warnings: Vec<String> = vec![];

    let (assets, asset_errs) = Assets::parse_all(&assets_src);
    errors.extend(asset_errs);
    if let Err(e) = kartu_core::check_lua(&lua_src) {
        errors.push(e);
    }
    warnings.extend(names(&lua_src, &assets));

    let frames: u64 = a.num("frames", 120);
    if errors.is_empty() {
        let mut script = Script::default();
        if let Some(p) = a.kv.get("script") {
            script.add(&std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?)?;
        }
        if let Some(p) = a.kv.get("press") {
            script.add(p)?;
        }
        match Console::new(&assets_src, &lua_src, a.num("seed", 1)) {
            Err(e) => errors.push(format!("init: {e}")),
            Ok(mut c) => {
                let mut dropped = 0;
                for f in 0..frames {
                    c.step(script.buttons_at(f));
                    for l in c.take_logs() {
                        // kits report config problems through log (e.g. a marker with no config)
                        if l.starts_with("kit:") && !warnings.contains(&l) {
                            warnings.push(l);
                        }
                    }
                    dropped = dropped.max(c.st.borrow().gfx.dropped);
                    if let Some(e) = &c.error {
                        errors.push(format!("frame {f}: {e}"));
                        break;
                    }
                }
                if dropped > 0 {
                    warnings.push(format!("over 128 sprites in one frame: {dropped} were dropped"));
                }
            }
        }
    }

    let own = |k: &str| assets.sound.declared.iter().filter(|d| d.starts_with(k)).count();
    let snd = if assets.sound.declared.is_empty() { String::new() } else {
        format!(", {} instruments, {} sfx, {} songs", own("instrument "), own("sfx "), own("song "))
    };
    println!(
        "{dir}: {} palettes, {} sprites, {} clips, {} tiles, {} maps{snd}",
        assets.palettes.len(),
        assets.sprites.len(),
        assets.clips.len(),
        assets.tiles.len(),
        assets.maps.len()
    );
    for l in assets.sound.describe() {
        match l.strip_prefix("warning: ") {
            Some(w) => warnings.push(w.to_string()),
            None => println!("{l}"),
        }
    }
    for e in &errors {
        println!("error: {e}");
    }
    for w in &warnings {
        println!("warning: {w}");
    }
    let ran = if errors.is_empty() { format!(", ran {frames} frames clean") } else { String::new() };
    println!("check: {} error(s), {} warning(s){ran}", errors.len(), warnings.len());
    if !errors.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Want {
    SprOrClip,
    Map,
    Tile,
    Palette,
    Flag,
    Sfx,
    Song,
}

/// Literal asset names in calls that assets.cw doesn't define. Only literals: a name built
/// at run time (`"cat_"..dir`) can't be checked before it runs.
fn names(lua: &str, a: &Assets) -> Vec<String> {
    // (call, which argument holds the name)
    // usize::MAX = the last argument
    const CALLS: [(&str, usize, Want); 11] = [
        ("sfx(", 0, Want::Sfx),
        ("music(", 0, Want::Song),
        ("spr(", 0, Want::SprOrClip),
        ("bg.map(", 1, Want::Map),
        ("bg.tile(", 3, Want::Tile),
        ("pal(", 0, Want::Palette),
        ("spawns(", 0, Want::Map),
        ("bg.flag(", 3, Want::Flag),
        ("touching(", usize::MAX, Want::Flag),
        ("solid(", usize::MAX, Want::Flag),
        ("move(", 3, Want::Flag),
    ];
    let mut out = vec![];
    if a.flag_bit("solid").is_none() {
        if let Some(ln) = lua.lines().position(|l| {
            let c = strip_lua_comment(l);
            c.contains("move(") && !c.contains("function move(")
        }) {
            out.push(format!("main.lua:{}: move() is used but no tile has flags=solid, so nothing blocks it", ln + 1));
        }
    }
    for (ln, line) in lua.lines().enumerate() {
        let code = strip_lua_comment(line);
        for (call, argi, want) in CALLS {
            let mut from = 0;
            while let Some(p) = code[from..].find(call).map(|p| p + from) {
                from = p + call.len();
                let before = code[..p].chars().last();
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || (c == '.' && !call.starts_with("bg."))) {
                    continue;
                }
                if code[..p].trim_end().ends_with("function") {
                    continue;
                }
                let args = split_args(&code[from..]);
                let mut found: Vec<(String, Want)> = vec![];
                let argi = if argi == usize::MAX { args.len().saturating_sub(1) } else { argi };
                if let Some(n) = args.get(argi).and_then(|s| literal(s)) {
                    found.push((n, want));
                }
                if call == "spr(" {
                    if let Some(opt) = args.get(3) {
                        if let Some(n) = opt.split_once("pal").and_then(|(_, r)| r.trim_start().strip_prefix('=')).and_then(|r| literal(r.split([',', '}']).next().unwrap_or(""))) {
                            found.push((n, Want::Palette));
                        }
                    }
                }
                for (n, w) in found {
                    let ok = match w {
                        Want::SprOrClip => matches!(a.names.get(&n), Some(Ref::Sprite(_) | Ref::Clip(_))),
                        Want::Map => matches!(a.names.get(&n), Some(Ref::Map(_))),
                        Want::Tile => matches!(a.names.get(&n), Some(Ref::Tile(_))),
                        Want::Palette => a.pal_names.contains_key(&n),
                        Want::Flag => a.flag_names.contains(&n) || n == "solid",
                        Want::Sfx => a.sound.sfx.contains_key(&n),
                        Want::Song => a.sound.songs.contains_key(&n),
                    };
                    if !ok {
                        let (what, pool): (&str, Vec<&String>) = match w {
                            Want::SprOrClip => ("sprite or clip", a.names.iter().filter(|(_, r)| matches!(r, Ref::Sprite(_) | Ref::Clip(_))).map(|x| x.0).collect()),
                            Want::Map => ("map", a.names.iter().filter(|(_, r)| matches!(r, Ref::Map(_))).map(|x| x.0).collect()),
                            Want::Tile => ("tile", a.names.iter().filter(|(_, r)| matches!(r, Ref::Tile(_))).map(|x| x.0).collect()),
                            Want::Palette => ("palette", a.pal_names.keys().collect()),
                            Want::Flag => ("tile flag", a.flag_names.iter().collect()),
                            Want::Sfx => ("sfx", a.sound.sfx.keys().collect()),
                            Want::Song => ("song", a.sound.songs.keys().collect()),
                        };
                        let hint = pool.iter().min_by_key(|c| (edit(c, &n), c.as_str())).filter(|c| edit(c, &n) <= 2.max(n.len() / 3)).map(|c| format!(" (did you mean `{c}`?)")).unwrap_or_default();
                        out.push(format!("main.lua:{}: {}…: no {what} called `{n}` in assets.cw{hint}", ln + 1, call));
                    }
                }
            }
        }
    }
    out
}

fn strip_lua_comment(l: &str) -> &str {
    let mut q: Option<char> = None;
    let b: Vec<char> = l.chars().collect();
    let mut byte = 0;
    for (i, &c) in b.iter().enumerate() {
        match q {
            Some(open) if c == open => q = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => q = Some(c),
            None if c == '-' && b.get(i + 1) == Some(&'-') => return &l[..byte],
            None => {}
        }
        byte += c.len_utf8();
    }
    l
}

/// Top-level arguments of a call, given the text right after its `(`.
fn split_args(s: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut depth = 0;
    let mut q: Option<char> = None;
    for c in s.chars() {
        if let Some(open) = q {
            if c == open {
                q = None;
            }
            out.last_mut().unwrap().push(c);
            continue;
        }
        match c {
            '"' | '\'' => q = Some(c),
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' if depth == 0 => break,
            ')' | '}' | ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(String::new());
                continue;
            }
            _ => {}
        }
        out.last_mut().unwrap().push(c);
    }
    out
}

fn literal(s: &str) -> Option<String> {
    let s = s.trim();
    let q = s.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let inner = s.strip_prefix(q)?.strip_suffix(q)?;
    (!inner.contains(q) && !inner.is_empty()).then(|| inner.to_string())
}

fn edit(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for j in 0..b.len() {
            let cur = row[j + 1];
            row[j + 1] = (prev + (ca != b[j]) as usize).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_bad_literal_names() {
        let a = Assets::parse("palette ui\n . clear\n k #000000\nsprite cat 1x1 pal=ui\nk\n").unwrap();
        let lua = "spr('cat', 1, 2)\nspr(\"catt\", 1, 2, {pal=\"uii\"})\nspr(name, 0, 0) -- spr('nope')\nlocal function myspr(x) end\npal('ui', 'k', '#ffffff')\nbg.map(0, \"level1\")";
        let w = names(lua, &a);
        assert_eq!(w.len(), 3, "{w:#?}");
        assert!(w[0].starts_with("main.lua:2:") && w[0].contains("did you mean `cat`"), "{}", w[0]);
        assert!(w[1].contains("palette called `uii`"), "{}", w[1]);
        assert!(w[2].starts_with("main.lua:6:") && w[2].contains("no map"), "{}", w[2]);
    }
}
