//! The tools: what they take (JSON schema for `tools/list`) and what they do.

use crate::{library, Config};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const API: &str = include_str!("../../API.md");
const KIT_TOPDOWN: &str = include_str!("../../core/kits/topdown.lua");
const TPL_MAIN: &str = include_str!("../templates/topdown/main.lua");
const TPL_ASSETS: &str = include_str!("../templates/topdown/assets.cw");
const TPL_BOT: &str = include_str!("../templates/topdown/bot.txt");

const MAX_SHOTS: usize = 6;

pub fn list(cfg: &Config) -> Vec<Value> {
    // With a pinned cart, `cart` is optional everywhere (it can only be that one).
    let cart = json!({"type": "string", "description": "cart name (its workspace directory)"});
    let pinned = cfg.pinned.is_some();
    let req = |r: Vec<&'static str>| -> Vec<&'static str> { r.into_iter().filter(|x| !(pinned && *x == "cart")).collect() };
    let inputs = json!({"type": "string", "description":
        "input script: steps like \"60:a!, 90:right, +30:, 200:up+a\" (frame:buttons holds from that frame, \
         `+N:` is relative, `a!` taps for 1 frame, empty releases). See docs(topic=\"Input scripts\")."});
    vec![
        json!({"name": "docs", "description": "The Kartu API reference, by section. No topic: the intro and the list of topics. \
               topic=\"all\" for everything, \"kit_source\" for the top-down kit's Lua source.",
               "inputSchema": {"type": "object", "properties": {"topic": {"type": "string"}}}}),
        json!({"name": "cart_new", "description": "Create a cart workspace. template \"topdown\" (default) = a tiny working \
               top-down game (hero, 3 stars, a slime, bot.txt) to reshape; \"blank\" = empty files.",
               "inputSchema": {"type": "object", "properties": {"cart": cart, "template": {"type": "string", "enum": ["topdown", "blank"]},
               "title": {"type": "string"}}, "required": req(vec!["cart"])}}),
        json!({"name": "cart_list", "description": "The carts in this workspace root, with titles and file sizes.",
               "inputSchema": {"type": "object", "properties": {}}}),
        json!({"name": "cart_read", "description": "Read a cart file (main.lua, assets.cw, bot.txt, ...). No file: list the cart's files. \
               from_line/to_line (1-based) read part of a big file.",
               "inputSchema": {"type": "object", "properties": {"cart": cart, "file": {"type": "string"},
               "from_line": {"type": "integer"}, "to_line": {"type": "integer"}}, "required": req(vec!["cart"])}}),
        json!({"name": "cart_write", "description": "Write a whole cart file (creates or replaces). Writing main.lua or assets.cw runs `check` \
               and returns every problem found. Prefer cart_patch for small changes.",
               "inputSchema": {"type": "object", "properties": {"cart": cart, "file": {"type": "string"}, "content": {"type": "string"}},
               "required": req(vec!["cart", "file", "content"])}}),
        json!({"name": "cart_patch", "description": "Edit a cart file by exact text replacement: each edit's `old` must appear exactly once \
               (or set all=true). All edits apply or none do. main.lua / assets.cw edits are then checked like cart_write.",
               "inputSchema": {"type": "object", "properties": {"cart": cart, "file": {"type": "string"},
               "edits": {"type": "array", "items": {"type": "object", "properties": {"old": {"type": "string"}, "new": {"type": "string"},
               "all": {"type": "boolean"}}, "required": ["old", "new"]}}}, "required": req(vec!["cart", "file", "edits"])}}),
        json!({"name": "check", "description": "Every problem at once: assets.cw errors, Lua syntax, unknown names/flags (did-you-mean), \
               then a short smoke run for runtime errors.",
               "inputSchema": {"type": "object", "properties": {"cart": cart}, "required": req(vec!["cart"])}}),
        json!({"name": "run", "description": "Play the cart headless and look at it: [log fN] lines, [sound fN] lines (sfx/music started), watched Lua values, the frame hash, \
               and screenshots (PNG images) of the frames in `shots` (max 6; default: the last frame). Stops at a cart error or at `until`. \
               bot=true lets the bot from bot.txt play instead of `inputs`.",
               "inputSchema": {"type": "object", "properties": {"cart": cart,
               "frames": {"type": "integer", "description": "frames to run (default 600 = 10 s; max 30000)"},
               "inputs": inputs, "seed": {"type": "integer"},
               "until": {"type": "string", "description": "stop after the frame whose log contains this (e.g. WIN)"},
               "watch": {"type": "array", "items": {"type": "string"}, "description": "Lua expressions to print, e.g. [\"world.hero.x\", \"world.room\"]"},
               "watch_every": {"type": "integer", "description": "frames between watch prints (60)"},
               "shots": {"type": "array", "items": {"type": "integer"}, "description": "frames to screenshot"},
               "scale": {"type": "integer", "description": "screenshot scale 1 or 2 (1)"},
               "bot": {"type": "boolean"}}, "required": req(vec!["cart"])}}),
        json!({"name": "state", "description": "The cart's whole state (globals + file-level locals, as JSON) at the given frames, \
               e.g. to find object coordinates for a bot plan.",
               "inputSchema": {"type": "object", "properties": {"cart": cart,
               "frames": {"type": "array", "items": {"type": "integer"}}, "inputs": inputs, "seed": {"type": "integer"},
               "depth": {"type": "integer", "description": "how deep to print nested tables (4)"}},
               "required": req(vec!["cart", "frames"])}}),
        json!({"name": "playtest", "description": "Prove the game can be won: the goal bot plays bot.txt (or `plan`) once per seed \
               (default 1,2,3) until the log says WIN. Reports per seed: won + time, bot failures (no path = softlock/map bug), \
               cart errors, hits within 1 s of arriving (unfair); a trace and a screenshot of the end. PASS = every seed won, fairly.",
               "inputSchema": {"type": "object", "properties": {"cart": cart, "plan": {"type": "string", "description": "a bot plan instead of bot.txt"},
               "seeds": {"type": "array", "items": {"type": "integer"}}, "frames": {"type": "integer", "description": "per run (20000)"},
               "win": {"type": "string", "description": "log text that means won (WIN)"}}, "required": req(vec!["cart"])}}),
        json!({"name": "asset_search", "description": "Search the art library (sprites, clips, tiles, palettes, maps from finished carts) \
               by words, e.g. \"cat\", \"door key\", \"tree grass\". show=\"source/name\" prints that asset's text.",
               "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "show": {"type": "string"}}}}),
        json!({"name": "asset_copy", "description": "Copy library assets into the cart's assets.cw, with everything they need \
               (palettes, `from=` sources, clip frames, map tiles). Names already in the cart with the same text are skipped.",
               "inputSchema": {"type": "object", "properties": {"cart": cart, "from": {"type": "string", "description": "the source, as asset_search shows it"},
               "names": {"type": "array", "items": {"type": "string"}}}, "required": req(vec!["cart", "from", "names"])}}),
    ]
}

pub fn call(cfg: &Config, name: &str, a: &Value) -> Result<Vec<Value>, String> {
    match name {
        "docs" => docs(a),
        "cart_new" => cart_new(cfg, a),
        "cart_list" => cart_list(cfg),
        "cart_read" => cart_read(cfg, a),
        "cart_write" => cart_write(cfg, a),
        "cart_patch" => cart_patch(cfg, a),
        "check" => {
            let (_, dir) = cart_dir(cfg, a, true)?;
            Ok(vec![text(check(cfg, &dir))])
        }
        "run" => run(cfg, a),
        "state" => state(cfg, a),
        "playtest" => playtest(cfg, a),
        "asset_search" => library::search(cfg, a),
        "asset_copy" => {
            let (_, dir) = cart_dir(cfg, a, true)?;
            let msg = library::copy(cfg, a, &dir)?;
            Ok(vec![text(format!("{msg}\n\n{}", check(cfg, &dir)))])
        }
        _ => Err(format!("unknown tool {name}")),
    }
}

// ---------- helpers ----------

pub fn text(s: impl Into<String>) -> Value {
    json!({"type": "text", "text": s.into()})
}

fn s<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(|v| v.as_str())
}

fn n(a: &Value, k: &str) -> Option<i64> {
    a.get(k).and_then(|v| v.as_i64())
}

fn ints(a: &Value, k: &str) -> Vec<i64> {
    a.get(k).and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_i64()).collect()).unwrap_or_default()
}

pub fn valid_cart(c: &str) -> Result<(), String> {
    let ok = !c.is_empty()
        && c.len() <= 40
        && c.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
        && !c.starts_with(['-', '_']);
    if ok { Ok(()) } else { Err(format!("bad cart name `{c}`: use 1-40 of a-z 0-9 _ -")) }
}

fn valid_file(f: &str) -> Result<(), String> {
    let (stem, ext) = f.rsplit_once('.').unwrap_or((f, ""));
    let ok = !stem.is_empty()
        && stem.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        && matches!(ext, "lua" | "cw" | "txt" | "md");
    if ok { Ok(()) } else { Err(format!("bad file name `{f}`: a plain name ending .lua .cw .txt or .md (no folders)")) }
}

/// The cart's workspace dir; `exists` = it must already be a cart.
fn cart_dir(cfg: &Config, a: &Value, exists: bool) -> Result<(String, PathBuf), String> {
    let name = match (s(a, "cart"), &cfg.pinned) {
        (Some(c), Some(p)) if c != p => return Err(format!("this server only works on cart `{p}`")),
        (_, Some(p)) => p.clone(),
        (Some(c), None) => c.to_string(),
        (None, None) => return Err("missing `cart`".into()),
    };
    if let Some(d) = &cfg.here {
        if exists && !d.join("main.lua").exists() {
            return Err(format!("no cart here yet: make it with cart_new"));
        }
        return Ok((name, d.clone()));
    }
    valid_cart(&name)?;
    let dir = cfg.root.join(&name);
    if exists && !dir.join("main.lua").exists() {
        return Err(format!("no cart `{name}` yet: make it with cart_new"));
    }
    Ok((name, dir))
}

fn scratch(dir: &Path) -> PathBuf {
    let d = dir.join(".cw");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Run the runner with a timeout; stdout+stderr together.
fn exec(cfg: &Config, args: &[String], work: &Path, secs: u64) -> (i32, String) {
    let out_path = work.join("out.txt");
    let Ok(f) = std::fs::File::create(&out_path) else { return (-1, "cannot write scratch output".into()) };
    let f2 = f.try_clone().unwrap();
    let child = Command::new(&cfg.runner).args(args).stdin(Stdio::null()).stdout(f).stderr(f2).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return (-1, format!("cannot start the runner {}: {e}", cfg.runner.display())),
    };
    let t0 = Instant::now();
    let code = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st.code().unwrap_or(-1),
            Ok(None) if t0.elapsed() > Duration::from_secs(secs) => {
                let _ = child.kill();
                let _ = child.wait();
                break -9;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break -1,
        }
    };
    let mut out = std::fs::read_to_string(&out_path).unwrap_or_default();
    if code == -9 {
        out.push_str(&format!("\n(stopped: the run took over {secs} s)"));
    }
    (code, out)
}

/// Fold identical consecutive lines and keep head + tail of long output.
fn fold(out: &str, max: usize, keep: impl Fn(&str) -> bool) -> String {
    let mut lines: Vec<String> = vec![];
    let (mut last, mut rep) = (String::new(), 0);
    let body = |l: &str| l.split_once("] ").map(|(_, b)| b.to_string()).unwrap_or_else(|| l.to_string());
    for l in out.lines().filter(|l| keep(l)) {
        let b = body(l);
        if b == last && l.starts_with('[') {
            rep += 1;
            continue;
        }
        if rep > 0 {
            lines.push(format!("  (same line {rep} more times)"));
        }
        lines.push(l.to_string());
        last = b;
        rep = 0;
    }
    if rep > 0 {
        lines.push(format!("  (same line {rep} more times)"));
    }
    if lines.len() > max {
        let cut = lines.len() - max;
        let mut v = lines[..max / 2].to_vec();
        v.push(format!("  … {cut} lines cut …"));
        v.extend_from_slice(&lines[lines.len() - max / 2..]);
        lines = v;
    }
    lines.join("\n")
}

fn inputs_file(a: &Value, work: &Path, args: &mut Vec<String>) {
    if let Some(i) = s(a, "inputs").filter(|i| !i.trim().is_empty()) {
        let p = work.join("input.txt");
        let _ = std::fs::write(&p, i);
        args.extend(["--script".into(), p.display().to_string()]);
    }
}

fn check(cfg: &Config, dir: &Path) -> String {
    let (_, out) = exec(cfg, &["check".into(), dir.display().to_string()], &scratch(dir), 30);
    let root = format!("{}/", dir.display());
    fold(&out.replace(&root, ""), 80, |_| true)
}

// ---------- docs ----------

fn docs(a: &Value) -> Result<Vec<Value>, String> {
    let note = if a.get("cli").is_some() { "" } else { "(Over MCP: `kartu check` = the check tool, `kartu run` = run, `kartu playtest` = playtest. \
                Ignore shell paths.)\n\n" };
    let heads: Vec<(usize, usize, &str)> = API
        .lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let lvl = l.chars().take_while(|c| *c == '#').count();
            (lvl == 2 || lvl == 3).then(|| (i, lvl, l[lvl..].trim()))
        })
        .collect();
    let lines: Vec<&str> = API.lines().collect();
    let topic = s(a, "topic").map(|t| t.trim().to_lowercase()).unwrap_or_default();
    if topic == "all" {
        return Ok(vec![text(format!("{note}{API}"))]);
    }
    if topic == "sound_songs" || topic == "sound_presets" {
        use kartu_core::audio::{INSTRUMENTS, SFX, SONGS};
        let mut o = String::from("Built-in songs, as MML (copy one into assets.cw under a new name and change it):\n\n");
        for (_, s) in SONGS {
            o.push_str(s);
            o.push_str("\n\n");
        }
        o.push_str("Built-in instruments (`instrument <name> <settings>`; use from=<name> to start from one):\n");
        for (n, d) in INSTRUMENTS {
            o.push_str(&format!("  {n}: {d}\n"));
        }
        o.push_str("\nBuilt-in sfx (`sfx <name> <settings>`):\n");
        for (n, d) in SFX {
            o.push_str(&format!("  {n}: {d}\n"));
        }
        return Ok(vec![text(o)]);
    }
    if topic == "kit_source" || topic == "kit source" {
        return Ok(vec![text(format!("-- core/kits/topdown.lua (built in: kit(\"topdown\"))\n{KIT_TOPDOWN}"))]);
    }
    if topic.is_empty() {
        let intro = lines[..heads.first().map_or(lines.len(), |h| h.0)].join("\n");
        let list: Vec<String> = heads
            .iter()
            .map(|(_, lvl, h)| format!("{}{}", if *lvl == 3 { "    " } else { "  " }, h))
            .collect();
        return Ok(vec![text(format!("{note}{intro}\nTopics (docs(topic=...)):\n{}\n  sound_songs (built-in songs, instruments, sfx as text)\n  kit_source\n  all", list.join("\n")))]);
    }
    let hit = heads.iter().position(|(_, _, h)| h.to_lowercase() == topic)
        .or_else(|| heads.iter().position(|(_, _, h)| h.to_lowercase().contains(&topic)))
        .or_else(|| heads.iter().position(|(_, _, h)| topic.split_whitespace().all(|w| h.to_lowercase().contains(w))));
    let Some(k) = hit else {
        let names: Vec<&str> = heads.iter().map(|h| h.2).collect();
        return Err(format!("no topic `{topic}`. Topics: {}, sound_songs, kit_source, all", names.join(" · ")));
    };
    let (start, lvl, _) = heads[k];
    let end = heads[k + 1..].iter().find(|h| h.1 <= lvl).map_or(lines.len(), |h| h.0);
    Ok(vec![text(lines[start..end].join("\n"))])
}

// ---------- files ----------

fn cart_new(cfg: &Config, a: &Value) -> Result<Vec<Value>, String> {
    let (name, dir) = cart_dir(cfg, a, false)?;
    if dir.join("main.lua").exists() {
        return Err(format!("cart `{name}` already exists: read it with cart_read"));
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let title = s(a, "title").unwrap_or(&name).to_string();
    let (main, assets, bot) = template(s(a, "template").unwrap_or("topdown"), &title)?;
    for (f, c) in [("main.lua", &main), ("assets.cw", &assets), ("bot.txt", &bot)] {
        std::fs::write(dir.join(f), c).map_err(|e| e.to_string())?;
    }
    let mut msg = format!("created cart `{name}` ({}), files: main.lua, assets.cw, bot.txt", s(a, "template").unwrap_or("topdown"));
    if s(a, "template").unwrap_or("topdown") == "topdown" {
        msg.push_str("\nIt already runs and playtest passes. main.lua:\n\n");
        msg.push_str(&main);
        msg.push_str("\nbot.txt:\n");
        msg.push_str(&bot);
        msg.push_str("\n(assets.cw: palette main; sprites hero1 hero2 slime1 slime2 star heart heart0; clips hero_walk \
                      slime_hop; tiles grass rock(solid); map field 20x15 with markers P=@hero * =@star S=@slime. \
                      Read it with cart_read.)");
    }
    Ok(vec![text(msg)])
}

/// (main.lua, assets.cw, bot.txt) of a starter cart.
pub fn template(tpl: &str, title: &str) -> Result<(String, String, String), String> {
    Ok(match tpl {
        "topdown" => (
            TPL_MAIN.replace("-- title: Star Field", &format!("-- title: {title}"))
                .replace("\"STAR FIELD\"", &format!("{:?}", title.to_uppercase())),
            TPL_ASSETS.to_string(),
            TPL_BOT.to_string(),
        ),
        "blank" => (
            format!("-- title: {title}\n\nfunction init() end\nfunction update() end\nfunction draw()\n  text(\"{}\", 160, 116, {{align=\"center\"}})\nend\n", title.to_uppercase()),
            "palette main\n  . clear\n  k #1a1c2c ink\n  w #f4f4f4 white\n".to_string(),
            "# bot plan: see docs(topic=\"bots\")\n".to_string(),
        ),
        t => return Err(format!("unknown template `{t}` (topdown, blank)")),
    })
}

fn cart_list(cfg: &Config) -> Result<Vec<Value>, String> {
    let mut rows = vec![];
    let names: Vec<String> = match &cfg.pinned {
        Some(p) => vec![p.clone()],
        None => {
            let mut v: Vec<String> = std::fs::read_dir(&cfg.root).map_err(|e| e.to_string())?
                .flatten().filter(|e| e.path().join("main.lua").exists())
                .map(|e| e.file_name().to_string_lossy().into_owned()).collect();
            v.sort();
            v
        }
    };
    for c in names {
        let dir = cfg.here.clone().unwrap_or_else(|| cfg.root.join(&c));
        let Ok(main) = std::fs::read_to_string(dir.join("main.lua")) else {
            rows.push(format!("{c}  (not created yet)"));
            continue;
        };
        let title = main.lines().find_map(|l| l.strip_prefix("-- title:")).unwrap_or("").trim().to_string();
        rows.push(format!("{c}  \"{title}\"  {}", files(&dir).join(", ")));
    }
    Ok(vec![text(if rows.is_empty() { "no carts yet (cart_new)".into() } else { rows.join("\n") })])
}

fn files(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).into_iter().flatten().flatten()
        .filter(|e| e.path().is_file())
        .map(|e| {
            let t = std::fs::read_to_string(e.path()).unwrap_or_default();
            format!("{} ({} lines)", e.file_name().to_string_lossy(), t.lines().count())
        })
        .collect();
    v.sort();
    v
}

fn cart_read(cfg: &Config, a: &Value) -> Result<Vec<Value>, String> {
    let (name, dir) = cart_dir(cfg, a, true)?;
    let Some(f) = s(a, "file") else {
        return Ok(vec![text(format!("{name}: {}", files(&dir).join(", ")))]);
    };
    valid_file(f)?;
    let t = std::fs::read_to_string(dir.join(f)).map_err(|_| format!("no file `{f}` in `{name}`"))?;
    let (from, to) = (n(a, "from_line"), n(a, "to_line"));
    if from.is_none() && to.is_none() {
        return Ok(vec![text(t)]);
    }
    let lines: Vec<&str> = t.lines().collect();
    let from = from.unwrap_or(1).max(1) as usize;
    let to = (to.unwrap_or(lines.len() as i64).max(0) as usize).min(lines.len());
    if from > to {
        return Err(format!("{f} has {} lines", lines.len()));
    }
    Ok(vec![text(format!("{f} lines {from}-{to} of {}:\n{}", lines.len(), lines[from - 1..to].join("\n")))])
}

fn after_edit(cfg: &Config, dir: &Path, f: &str, what: String) -> Vec<Value> {
    if f == "main.lua" || f == "assets.cw" {
        vec![text(format!("{what}\n{}", check(cfg, dir)))]
    } else {
        vec![text(what)]
    }
}

fn cart_write(cfg: &Config, a: &Value) -> Result<Vec<Value>, String> {
    let (_, dir) = cart_dir(cfg, a, true)?;
    let f = s(a, "file").ok_or("missing `file`")?;
    valid_file(f)?;
    let c = s(a, "content").ok_or("missing `content`")?;
    std::fs::write(dir.join(f), c).map_err(|e| e.to_string())?;
    Ok(after_edit(cfg, &dir, f, format!("wrote {f} ({} lines)", c.lines().count())))
}

fn cart_patch(cfg: &Config, a: &Value) -> Result<Vec<Value>, String> {
    let (name, dir) = cart_dir(cfg, a, true)?;
    let f = s(a, "file").ok_or("missing `file`")?;
    valid_file(f)?;
    let mut t = std::fs::read_to_string(dir.join(f)).map_err(|_| format!("no file `{f}` in `{name}`"))?;
    let edits = a.get("edits").and_then(|e| e.as_array()).ok_or("missing `edits`")?;
    for (i, e) in edits.iter().enumerate() {
        let old = s(e, "old").ok_or(format!("edit {}: missing `old`", i + 1))?;
        let new = s(e, "new").ok_or(format!("edit {}: missing `new`", i + 1))?;
        if old.is_empty() {
            return Err(format!("edit {}: `old` is empty", i + 1));
        }
        let count = t.matches(old).count();
        let all = e.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
        if count == 0 {
            let first = old.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
            let near = t.lines().enumerate().find(|(_, l)| !first.is_empty() && l.contains(first))
                .map(|(k, l)| format!(" Its first line is at line {}: `{}` (check whitespace in the rest).", k + 1, l.trim()))
                .unwrap_or_default();
            return Err(format!("edit {}: `old` not found in {f}; nothing changed.{near}", i + 1));
        }
        if count > 1 && !all {
            return Err(format!("edit {}: `old` appears {count} times in {f}; add context or set all=true. Nothing changed.", i + 1));
        }
        t = t.replace(old, new);
    }
    std::fs::write(dir.join(f), &t).map_err(|e| e.to_string())?;
    Ok(after_edit(cfg, &dir, f, format!("patched {f}: {} edit(s), now {} lines", edits.len(), t.lines().count())))
}

// ---------- running ----------

fn exit_meaning(code: i32) -> &'static str {
    match code {
        0 => "ran to the end",
        2 => "STOPPED: cart error",
        3 => "the `until` text never appeared",
        4 => "the bot FAILED",
        -9 => "timed out",
        _ => "runner error",
    }
}

fn images(out: &str, scale_note: &str) -> Vec<Value> {
    let mut v = vec![];
    for l in out.lines() {
        let Some(rest) = l.strip_prefix("screenshot ") else { continue };
        let (label, path) = match rest.split_once(' ') {
            Some((f, p)) => (f.to_string(), p),
            None => ("last frame".to_string(), rest),
        };
        if let Ok(b) = std::fs::read(path) {
            v.push(text(format!("screenshot {label}{scale_note}")));
            v.push(json!({"type": "image", "data": b64(&b), "mimeType": "image/png"}));
            let _ = std::fs::remove_file(path);
        }
    }
    v
}

fn run(cfg: &Config, a: &Value) -> Result<Vec<Value>, String> {
    let (_, dir) = cart_dir(cfg, a, true)?;
    let work = scratch(&dir);
    let frames = n(a, "frames").unwrap_or(600).clamp(1, 30000);
    let mut args: Vec<String> = vec!["run".into(), dir.display().to_string(), "--frames".into(), frames.to_string(), "--sounds".into()];
    if let Some(sd) = n(a, "seed") {
        args.extend(["--seed".into(), sd.to_string()]);
    }
    inputs_file(a, &work, &mut args);
    if let Some(u) = s(a, "until").filter(|u| !u.is_empty()) {
        args.extend(["--until".into(), u.into()]);
    }
    let watch: Vec<String> = a.get("watch").and_then(|w| w.as_array())
        .map(|w| w.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    if !watch.is_empty() {
        args.extend(["--watch".into(), watch.join(";"), "--watch-every".into(), n(a, "watch_every").unwrap_or(60).max(1).to_string()]);
    }
    if a.get("bot").and_then(|b| b.as_bool()).unwrap_or(false) {
        if !dir.join("bot.txt").exists() {
            return Err("bot=true but there is no bot.txt".into());
        }
        args.extend(["--bot".into(), dir.join("bot.txt").display().to_string()]);
    }
    let scale = n(a, "scale").unwrap_or(1).clamp(1, 2);
    let mut shots = ints(a, "shots");
    shots.retain(|f| *f >= 0 && *f < frames);
    shots.dedup();
    let dropped = shots.len().saturating_sub(MAX_SHOTS);
    shots.truncate(MAX_SHOTS);
    args.extend(["--shot".into(), work.join("shot.png").display().to_string(), "--scale".into(), scale.to_string()]);
    if !shots.is_empty() {
        args.extend(["--shot-at".into(), shots.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")]);
    }
    let (code, out) = exec(cfg, &args, &work, 90);
    let mut body = fold(&out, 160, |l| !l.starts_with("screenshot "));
    if dropped > 0 {
        body.push_str(&format!("\n({dropped} shots dropped: max {MAX_SHOTS} per run)"));
    }
    let mut v = vec![text(format!("exit {code}: {}\n{body}", exit_meaning(code)))];
    // the last-frame shot only when no frames were asked for (it would repeat the last one)
    let out = if shots.is_empty() { out } else {
        out.lines().filter(|l| l.starts_with("screenshot f")).collect::<Vec<_>>().join("\n")
    };
    v.extend(images(&out, if scale == 1 { " (320x240)" } else { " (640x480)" }));
    let _ = std::fs::remove_file(work.join("shot.png"));
    Ok(v)
}

fn state(cfg: &Config, a: &Value) -> Result<Vec<Value>, String> {
    let (_, dir) = cart_dir(cfg, a, true)?;
    let work = scratch(&dir);
    let mut frames = ints(a, "frames");
    frames.retain(|f| (0..30000).contains(f));
    frames.truncate(4);
    if frames.is_empty() {
        return Err("give frames, e.g. [61]".into());
    }
    let last = *frames.iter().max().unwrap();
    let mut args: Vec<String> = vec!["run".into(), dir.display().to_string(), "--frames".into(), (last + 1).to_string(),
        "--dump".into(), frames.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")];
    if let Some(sd) = n(a, "seed") {
        args.extend(["--seed".into(), sd.to_string()]);
    }
    inputs_file(a, &work, &mut args);
    let (code, out) = exec(cfg, &args, &work, 60);
    let mut dumps: Vec<String> = out.lines().filter(|l| l.starts_with("[dump") || l.starts_with("error")).map(String::from).collect();
    if dumps.is_empty() {
        dumps.push(fold(&out, 40, |_| true));
    }
    let mut t = format!("exit {code}: {}\n{}", exit_meaning(code), dumps.join("\n"));
    if t.len() > 24000 {
        t.truncate(24000);
        t.push_str("\n… (cut at 24k chars: ask for fewer frames)");
    }
    Ok(vec![text(t)])
}

struct Facts {
    won: Option<i64>,
    frames: Option<i64>,
    bot_failure: Option<String>,
    cart_error: Option<String>,
    early: Vec<String>,
}

fn facts(out: &str) -> Facts {
    let grab = |pre: &str| out.lines().find_map(|l| l.strip_prefix(pre).map(String::from));
    let won = out.lines().find_map(|l| l.strip_prefix("until: ").and_then(|r| r.rsplit_once(" at f")).and_then(|(_, f)| f.trim().parse().ok()));
    let frames = grab("frames ").and_then(|r| r.split_whitespace().next().and_then(|x| x.parse().ok()));
    let bot_failure = out.lines().find_map(|l| l.strip_prefix("[bot f").and_then(|r| r.split_once("] ")).map(|(_, m)| m)
        .filter(|m| m.starts_with("FAILED")).map(String::from));
    let cart_error = grab("error at ");
    // hits before anyone could react: within 1 s of play starting or entering a room
    let (mut arrived, mut early) = (0i64, vec![]);
    for l in out.lines() {
        let Some((f, msg)) = l.strip_prefix("[log f").and_then(|r| r.split_once("] ")) else { continue };
        let Ok(f) = f.parse::<i64>() else { continue };
        if msg.starts_with("room ") || msg.starts_with("state play") {
            arrived = f;
        } else if ["hurt", "ouch", "hit"].iter().any(|w| msg.starts_with(w)) && f - arrived < 60 {
            early.push(format!("f{f} ({:.2} s after arriving)", (f - arrived) as f64 / 60.0));
        }
    }
    Facts { won, frames, bot_failure, cart_error, early }
}

fn playtest(cfg: &Config, a: &Value) -> Result<Vec<Value>, String> {
    let (_, dir) = cart_dir(cfg, a, true)?;
    let work = scratch(&dir);
    let plan = match s(a, "plan").filter(|p| !p.trim().is_empty()) {
        Some(p) => {
            let path = work.join("plan.txt");
            std::fs::write(&path, p).map_err(|e| e.to_string())?;
            path
        }
        None if dir.join("bot.txt").exists() => dir.join("bot.txt"),
        None => return Err("no bot.txt: write one (docs(topic=\"bots\")) or pass `plan`".into()),
    };
    let mut seeds = ints(a, "seeds");
    if seeds.is_empty() {
        seeds = vec![1, 2, 3];
    }
    seeds.truncate(8);
    let frames = n(a, "frames").unwrap_or(20000).clamp(60, 60000);
    let win = s(a, "win").unwrap_or("WIN");
    let (mut rows, mut pass, mut extra) = (vec![], true, vec![]);
    for (k, seed) in seeds.iter().enumerate() {
        let mut args: Vec<String> = vec!["run".into(), dir.display().to_string(), "--bot".into(), plan.display().to_string(),
            "--until".into(), win.into(), "--frames".into(), frames.to_string(), "--seed".into(), seed.to_string()];
        if k == 0 {
            args.extend(["--shot".into(), work.join("end.png").display().to_string(), "--scale".into(), "1".into()]);
        }
        let (code, out) = exec(cfg, &args, &work, 120);
        let fx = facts(&out);
        let ok = fx.won.is_some() && fx.cart_error.is_none() && fx.early.is_empty();
        pass &= ok;
        let mut row = format!("seed {seed}: {}", match fx.won {
            Some(f) => format!("WON at f{f} ({:.1} s)", f as f64 / 60.0),
            None => format!("not won ({} frames, exit {code}: {})", fx.frames.unwrap_or(0), exit_meaning(code)),
        });
        if let Some(b) = &fx.bot_failure { row.push_str(&format!(" · bot {b}")); }
        if let Some(e) = &fx.cart_error { row.push_str(&format!(" · cart error at {e}")); }
        if !fx.early.is_empty() { row.push_str(&format!(" · UNFAIR early hits {}", fx.early.join(", "))); }
        rows.push(row);
        if k == 0 {
            let tr = fold(&out, 70, |l| l.starts_with("[log") || l.starts_with("[bot") || l.starts_with("error"));
            extra.push(text(format!("trace (seed {seed}):\n{tr}")));
            extra.extend(images(&out, " (end of the seed-1 run)"));
        }
    }
    let head = format!("playtest {}: {}\n{}", if pass { "PASS" } else { "FAIL" },
        if pass { "the bot won on every seed, no unfair hits" } else { "fix the game or the plan, then playtest again" },
        rows.join("\n"));
    let mut v = vec![text(head)];
    v.extend(extra);
    Ok(v)
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}
