//! The art library: assets.cw files of finished carts (or packs), split into blocks
//! (palette / sprite / tile / clip / map) so they can be searched and copied with
//! everything they depend on.

use crate::tools::text;
use crate::Config;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const KINDS: [&str; 8] = ["palette", "sprite", "tile", "clip", "map", "instrument", "sfx", "song"];

#[derive(Clone)]
pub struct Block {
    pub kind: String,
    pub name: String,
    pub header: String,
    pub text: String,
    /// The section comment above it (`-- ---------- hero ----------`), for search.
    pub note: String,
    /// (namespace, name): see `ns`
    pub deps: Vec<(&'static str, String)>,
}

impl Block {
    fn key(&self) -> (&'static str, String) {
        (ns(&self.kind), self.name.clone())
    }
}

/// Namespaces: palettes, instruments, sfx and songs each have their own; sprites, tiles,
/// clips and maps share one ("obj").
fn ns(kind: &str) -> &'static str {
    match kind {
        "palette" => "palette",
        "instrument" => "instrument",
        "sfx" => "sfx",
        "song" => "song",
        _ => "obj",
    }
}

fn strip_comment(l: &str) -> &str {
    let t = l.trim();
    if t.starts_with("--") {
        return "";
    }
    match t.find(" --") {
        Some(i) => t[..i].trim_end(),
        None => t,
    }
}

pub fn parse(src: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = vec![];
    let mut note = String::new();
    let mut rows_left = 0usize;
    for raw in src.lines() {
        if rows_left > 0 {
            if let Some(b) = blocks.last_mut() {
                if b.kind == "map" && raw.trim_start().starts_with("legend ") {
                    add_legend(b, raw);
                    b.text.push_str(raw);
                    b.text.push('\n');
                    continue;
                }
                b.text.push_str(raw);
                b.text.push('\n');
            }
            rows_left -= 1;
            continue;
        }
        let l = strip_comment(raw);
        if l.is_empty() {
            let c = raw.trim().trim_start_matches('-').trim().trim_end_matches('-').trim();
            // only section headings (`-- ---------- hero ----------`) name what follows
            if raw.trim().starts_with("-- --") && !c.is_empty() && c.len() < 60 {
                note = c.to_string();
            }
            continue;
        }
        let mut w = l.split_whitespace();
        let first = w.next().unwrap_or("");
        if KINDS.contains(&first) {
            let name = w.next().unwrap_or("").to_string();
            let mut b = Block { kind: first.into(), name, header: l.to_string(), text: format!("{raw}\n"),
                                note: note.clone(), deps: vec![] };
            let rest: Vec<&str> = l.split_whitespace().skip(2).collect();
            let derived = rest.iter().any(|t| t.starts_with("from="));
            for t in &rest {
                if let Some(p) = t.strip_prefix("pal=") { b.deps.push(("palette", p.into())); }
                if let Some(f) = t.strip_prefix("from=") {
                    b.deps.push((if first == "instrument" { "instrument" } else { "obj" }, f.into()));
                }
                if let Some(f) = t.strip_prefix("inst=") { b.deps.push(("instrument", f.into())); }
            }
            match first {
                "sprite" | "tile" | "map" if !derived => {
                    let size = rest.iter().find_map(|t| t.split_once('x').and_then(|(_, h)| h.parse::<usize>().ok()));
                    rows_left = if first == "tile" { size.unwrap_or(16) } else { size.unwrap_or(0) };
                }
                "clip" => {
                    for t in rest.iter().filter(|t| !t.contains('=')) {
                        b.deps.push(("obj", t.to_string()));
                    }
                }
                "sfx" if l.contains("notes=\"") => {
                    // inst= inside a notes sfx: already caught above; @name inside the notes
                    for n in at_names(raw) { b.deps.push(("instrument", n)); }
                }
                _ => {}
            }
            blocks.push(b);
        } else if let Some(b) = blocks.last_mut() {
            // palette entries, a legend line of a map, or a song's channel line
            if first == "legend" && b.kind == "map" {
                add_legend(b, raw);
            }
            if b.kind == "song" {
                for n in at_names(raw) {
                    b.deps.push(("instrument", n));
                }
            }
            b.text.push_str(raw);
            b.text.push('\n');
        }
    }
    blocks
}

/// `@name` instrument references in MML (not `@drums`).
fn at_names(l: &str) -> Vec<String> {
    let mut out = vec![];
    for (i, _) in l.match_indices('@') {
        let n: String = l[i + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        if !n.is_empty() && n != "drums" {
            out.push(n);
        }
    }
    out
}

fn add_legend(b: &mut Block, raw: &str) {
    let toks: Vec<&str> = strip_comment(raw).split_whitespace().skip(1).collect();
    for pair in toks.chunks(2) {
        let Some(t) = pair.get(1) else { continue };
        let tile = match t.strip_prefix('@') {
            Some(m) => m.split_once(':').map(|(_, tile)| tile),
            None => Some(*t),
        };
        if let Some(tile) = tile.filter(|t| *t != "empty") {
            b.deps.push(("obj", tile.to_string()));
        }
    }
}

/// (source name, assets.cw path) for everything in the library, plus the carts in the
/// root when the server isn't pinned to one cart.
fn sources(cfg: &Config) -> Vec<(String, PathBuf)> {
    let mut dirs: Vec<PathBuf> = cfg.library.clone();
    if cfg.pinned.is_none() {
        dirs.push(cfg.root.clone());
    }
    let mut v = vec![];
    for d in dirs {
        if d.join("assets.cw").exists() {
            v.push((name_of(&d), d.join("assets.cw")));
            continue;
        }
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut sub: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.join("assets.cw").exists()).collect();
        sub.sort();
        for p in sub {
            if name_of(&p) == "bench" {
                continue; // the P0 gate cart: noise tiles, not art
            }
            v.push((name_of(&p), p.join("assets.cw")));
        }
    }
    v
}

fn name_of(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn describe(b: &Block) -> String {
    let rest: Vec<&str> = b.header.split_whitespace().skip(2).collect();
    match b.kind.as_str() {
        "palette" => {
            let n = b.text.lines().skip(1).filter(|l| !strip_comment(l).is_empty()).count();
            format!("palette, {n} colours")
        }
        "song" => {
            let n = b.text.lines().skip(1).filter(|l| !strip_comment(l).trim().is_empty()).count();
            format!("song {} ({n} channel lines)", rest.join(" "))
        }
        _ => format!("{} {}", b.kind, rest.join(" ")),
    }
}

pub fn search(cfg: &Config, a: &Value) -> Result<Vec<Value>, String> {
    let srcs = sources(cfg);
    if srcs.is_empty() {
        return Err("the library is empty (start the server with --library DIR)".into());
    }
    if let Some(show) = a.get("show").and_then(|v| v.as_str()) {
        let (src, name) = show.split_once('/').ok_or("show=\"source/name\", e.g. \"dungeon/knight1\"")?;
        let (_, path) = srcs.iter().find(|(n, _)| n == src).ok_or(format!("no source `{src}`"))?;
        let blocks = parse(&std::fs::read_to_string(path).unwrap_or_default());
        let hits: Vec<String> = blocks.iter().filter(|b| b.name == name).map(|b| b.text.clone()).collect();
        if hits.is_empty() {
            return Err(format!("no asset `{name}` in `{src}`"));
        }
        return Ok(vec![text(hits.join("\n"))]);
    }
    let q: Vec<String> = a.get("query").and_then(|v| v.as_str()).unwrap_or("")
        .to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(String::from).collect();
    let mut hits: Vec<(usize, String)> = vec![];
    let mut per_src: Vec<String> = vec![];
    for (src, path) in &srcs {
        let blocks = parse(&std::fs::read_to_string(path).unwrap_or_default());
        per_src.push(format!("{src} ({})", blocks.len()));
        for b in &blocks {
            let hay = format!("{} {} {}", b.name, b.note, if b.kind == "palette" { b.text.as_str() } else { "" }).to_lowercase();
            let score = q.iter().filter(|w| hay.contains(w.as_str())).count()
                + q.iter().filter(|w| b.name.to_lowercase().contains(w.as_str())).count();
            if score > 0 {
                hits.push((score, format!("{src}/{}  {}", b.name, describe(b))));
            }
        }
    }
    if q.is_empty() {
        return Ok(vec![text(format!("library sources (assets): {}\nsearch with query=\"words\"", per_src.join(", ")))]);
    }
    hits.sort_by(|x, y| y.0.cmp(&x.0));
    let total = hits.len();
    hits.truncate(50);
    let mut out = hits.into_iter().map(|h| h.1).collect::<Vec<_>>().join("\n");
    if total > 50 {
        out.push_str(&format!("\n… {} more", total - 50));
    }
    if total == 0 {
        out = format!("nothing for `{}`. Sources: {}", q.join(" "), per_src.join(", "));
    }
    out.push_str("\n\nasset_copy(from=\"<source>\", names=[...]) brings them in with their palettes; show=\"source/name\" prints one.");
    Ok(vec![text(out)])
}

pub fn copy(cfg: &Config, a: &Value, dir: &Path) -> Result<String, String> {
    let from = a.get("from").and_then(|v| v.as_str()).ok_or("missing `from`")?;
    let names: Vec<String> = a.get("names").and_then(|v| v.as_array())
        .map(|v| v.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    if names.is_empty() {
        return Err("missing `names`".into());
    }
    let srcs = sources(cfg);
    let (_, path) = srcs.iter().find(|(n, _)| n == from).ok_or(format!("no library source `{from}`"))?;
    let blocks = parse(&std::fs::read_to_string(path).unwrap_or_default());
    // wanted: the names, then their dependencies, transitively
    let mut want: BTreeSet<(&'static str, String)> = BTreeSet::new();
    let mut todo: Vec<(&'static str, String)> = vec![];
    for n in &names {
        let found: Vec<&Block> = blocks.iter().filter(|b| &b.name == n).collect();
        if found.is_empty() {
            return Err(format!("no asset `{n}` in `{from}` (asset_search to find names)"));
        }
        todo.extend(found.iter().map(|b| b.key()));
    }
    while let Some(k) = todo.pop() {
        if !want.insert(k.clone()) {
            continue;
        }
        if let Some(b) = blocks.iter().find(|b| b.key() == k) {
            todo.extend(b.deps.iter().cloned());
        }
    }
    let target_path = dir.join("assets.cw");
    let target_src = std::fs::read_to_string(&target_path).unwrap_or_default();
    let target = parse(&target_src);
    // Clashes get renamed on the way in (every cart calls its first palette `main`):
    // palettes become `<from>_<name>`, the rest `<name>_<from>`, and references follow.
    let mut skipped = vec![];
    let mut added = vec![];
    let mut renames: std::collections::HashMap<(&'static str, String), String> = Default::default();
    let taken = |k: &(&'static str, String), renames: &std::collections::HashMap<(&'static str, String), String>| {
        target.iter().any(|t| t.key() == *k) || renames.iter().any(|(rk, v)| rk.0 == k.0 && *v == k.1)
    };
    let mut copy: Vec<&Block> = vec![];
    // source order keeps declarations before uses (palettes, then `from=` sources)
    for b in blocks.iter().filter(|b| want.contains(&b.key())) {
        match target.iter().find(|t| t.key() == b.key()) {
            Some(t) if t.text.trim() == b.text.trim() => skipped.push(b.name.clone()),
            Some(_) => {
                let base = if b.kind == "palette" { format!("{from}_{}", b.name) } else { format!("{}_{from}", b.name) };
                let mut nm = base.clone();
                let mut k = 2;
                while taken(&(ns(&b.kind), nm.clone()), &renames) {
                    nm = format!("{base}{k}");
                    k += 1;
                }
                renames.insert(b.key(), nm.clone());
                added.push(format!("{} {} (as `{nm}`: the cart has its own `{}`)", b.kind, b.name, b.name));
                copy.push(b);
            }
            None => {
                added.push(format!("{} {}", b.kind, b.name));
                copy.push(b);
            }
        }
    }
    if added.is_empty() {
        return Ok(format!("nothing to add: already in the cart ({})", skipped.join(", ")));
    }
    // palettes must come before the sprites that use them: new palettes go right after the
    // cart's last palette block (or at the top); everything else at the end.
    // instruments must precede the songs/sfx that use them, like palettes before sprites
    let new_pals: String = copy.iter().filter(|b| b.kind == "palette" || b.kind == "instrument").map(|b| rewrite(b, &renames)).collect();
    let rest: String = copy.iter().filter(|b| b.kind != "palette" && b.kind != "instrument").map(|b| rewrite(b, &renames)).collect();
    let mut out = String::new();
    let lines: Vec<&str> = target_src.lines().collect();
    let last_pal_end = last_palette_end(&lines);
    let pal_block = if new_pals.is_empty() { String::new() } else { format!("-- palettes and instruments from library: {from}\n{new_pals}") };
    match last_pal_end {
        Some(i) => {
            out.push_str(&lines[..i].join("\n"));
            out.push('\n');
            out.push_str(&pal_block);
            out.push_str(&lines[i..].join("\n"));
        }
        None => {
            out.push_str(&pal_block);
            out.push_str(&target_src);
        }
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    if !rest.is_empty() {
        out.push_str(&format!("\n-- from library: {from}\n{rest}"));
    }
    std::fs::write(&target_path, out).map_err(|e| e.to_string())?;
    let mut msg = format!("added to assets.cw: {}", added.join(", "));
    if !skipped.is_empty() {
        msg.push_str(&format!(" (already there: {})", skipped.join(", ")));
    }
    Ok(msg)
}

/// A block's text with renamed names/references (unchanged if nothing it touches was renamed).
fn rewrite(b: &Block, ren: &std::collections::HashMap<(&'static str, String), String>) -> String {
    let own = ren.contains_key(&b.key());
    if !own && !b.deps.iter().any(|d| ren.contains_key(d)) {
        return b.text.clone();
    }
    let obj = |n: &str| ren.get(&("obj", n.to_string())).cloned().unwrap_or_else(|| n.to_string());
    let inst = |n: &str| ren.get(&("instrument", n.to_string())).cloned().unwrap_or_else(|| n.to_string());
    // @name → @renamed inside MML (song channel lines and notes="...")
    let mml = |l: &str| -> String {
        let mut out = String::new();
        let mut rest = l;
        while let Some(i) = rest.find('@') {
            out.push_str(&rest[..=i]);
            let n: String = rest[i + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            out.push_str(&inst(&n));
            rest = &rest[i + 1 + n.len()..];
        }
        out.push_str(rest);
        out
    };
    if b.kind == "song" || b.kind == "sfx" {
        let mut out = String::new();
        for (i, line) in b.text.lines().enumerate() {
            let l = if i == 0 {
                let mut l = line.replacen(&format!("{} {}", b.kind, b.name), &format!("{} {}", b.kind, ren.get(&b.key()).cloned().unwrap_or_else(|| b.name.clone())), 1);
                if let Some(p) = l.find("inst=") {
                    let n: String = l[p + 5..].chars().take_while(|c| !c.is_whitespace()).collect();
                    l = format!("{}inst={}{}", &l[..p], inst(&n), &l[p + 5 + n.len()..]);
                }
                l
            } else {
                line.to_string()
            };
            out.push_str(&mml(&l));
            out.push('\n');
        }
        return out;
    }
    let mut out = String::new();
    for (i, line) in b.text.lines().enumerate() {
        let toks: Vec<&str> = strip_comment(line).split_whitespace().collect();
        if i == 0 {
            let mut v: Vec<String> = vec![toks[0].to_string(), ren.get(&b.key()).cloned().unwrap_or_else(|| b.name.clone())];
            for t in &toks[2..] {
                v.push(if let Some(p) = t.strip_prefix("pal=") {
                    format!("pal={}", ren.get(&("palette", p.to_string())).map(String::as_str).unwrap_or(p))
                } else if let Some(f) = t.strip_prefix("from=") {
                    format!("from={}", if b.kind == "instrument" { inst(f) } else { obj(f) })
                } else if b.kind == "clip" && !t.contains('=') {
                    obj(t)
                } else {
                    t.to_string()
                });
            }
            out.push_str(&v.join(" "));
        } else if b.kind == "map" && toks.first() == Some(&"legend") {
            let mut v = vec!["legend".to_string()];
            for pair in toks[1..].chunks(2) {
                v.push(pair[0].to_string());
                if let Some(t) = pair.get(1) {
                    v.push(match t.strip_prefix('@').and_then(|m| m.split_once(':')) {
                        Some((kind, tile)) => format!("@{kind}:{}", obj(tile)),
                        None if t.starts_with('@') || *t == "empty" => t.to_string(),
                        None => obj(t),
                    });
                }
            }
            out.push_str(&v.join(" "));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// Line index just after the last palette block of a file (None if it has none).
fn last_palette_end(lines: &[&str]) -> Option<usize> {
    let mut end = None;
    let mut in_pal = false;
    for (i, l) in lines.iter().enumerate() {
        let t = strip_comment(l);
        let first = t.split_whitespace().next().unwrap_or("");
        if KINDS.contains(&first) {
            in_pal = first == "palette";
            if in_pal {
                end = Some(i + 1);
            }
        } else if in_pal && !t.is_empty() {
            end = Some(i + 1);
        }
    }
    end
}
