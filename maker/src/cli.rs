//! The maker's shell commands (`kartu new|setup|doctor|config|docs|playtest|web` land here):
//! everything an AI without MCP, or a person, needs around the runner.

use crate::config::{self, Val};
use crate::{tools, Config};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const AGENTS: &str = include_str!("../skill/AGENTS.md");

pub fn run(cmd: &str, args: &[String]) -> Result<i32, String> {
    match cmd {
        "new" => new(args),
        "setup" => setup(args),
        "doctor" => doctor(),
        "config" => show_config(args),
        "docs" => {
            let a = json!({"topic": args.first().cloned().unwrap_or_default(), "cli": true});
            print_text(&tools::call(&here_cfg(Path::new("."))?, "docs", &a)?);
            Ok(0)
        }
        "playtest" => playtest(args),
        "web" => web(args),
        _ => Err(format!("unknown command {cmd}")),
    }
}

fn print_text(v: &[Value]) {
    for x in v.iter().filter_map(|x| x.get("text").and_then(|t| t.as_str())) {
        println!("{x}");
    }
}

fn flag<'a>(args: &'a [String], k: &str) -> Option<&'a str> {
    args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).map(|s| s.as_str())
}

fn positional(args: &[String]) -> Option<&str> {
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
        } else if a.starts_with("--") {
            skip = !matches!(a.as_str(), "--mcp" | "--force");
        } else {
            return Some(a);
        }
    }
    None
}

/// A server/CLI config where `dir` itself is the (only) cart.
pub fn here_cfg(dir: &Path) -> Result<Config, String> {
    let dir = dir.canonicalize().map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "cart".into());
    let t = config::load()?;
    Ok(Config {
        root: dir.parent().map(Path::to_path_buf).unwrap_or_else(|| dir.clone()),
        pinned: Some(name),
        here: Some(dir),
        library: config::library_dirs(&t),
        runner: crate::runner_path(),
        log: None,
    })
}

/// How other tools should start `kartu`: the bare name if that's this install, else a full path.
fn kartu_cmd() -> String {
    let mine = crate::runner_path();
    let on_path = std::env::var_os("PATH").into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .map(|d| d.join("kartu"))
        .find(|p| p.is_file());
    match (on_path.and_then(|p| p.canonicalize().ok()), mine.canonicalize()) {
        (Some(p), Ok(m)) if p == m => "kartu".into(),
        (_, Ok(m)) => m.display().to_string(),
        _ => "kartu".into(),
    }
}

fn title_of(name: &str) -> String {
    name.split(['-', '_', ' ']).filter(|w| !w.is_empty())
        .map(|w| w[..1].to_uppercase() + &w[1..]).collect::<Vec<_>>().join(" ")
}

// ---------- new ----------

fn new(args: &[String]) -> Result<i32, String> {
    let Some(path) = positional(args) else {
        return Err("usage: kartu new <folder> [--title T] [--template topdown|blank]".into());
    };
    let dir = PathBuf::from(path);
    if dir.join("main.lua").exists() {
        return Err(format!("{} is already a cart", dir.display()));
    }
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let title = flag(args, "--title").map(str::to_string).unwrap_or_else(|| title_of(&name));
    let tpl = flag(args, "--template").unwrap_or("topdown");
    let (main, assets, bot) = tools::template(tpl, &title)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mcp = json!({"mcpServers": {"kartu": {"command": kartu_cmd(), "args": ["mcp", "--cart", "."]}}});
    let files = [
        ("main.lua", main),
        ("assets.cw", assets),
        ("bot.txt", bot),
        ("kartu.toml", format!("title = {title:?}\nformat = {}\n", config::FORMAT)),
        ("AGENTS.md", AGENTS.to_string()),
        ("CLAUDE.md", "@AGENTS.md\n".into()),
        (".mcp.json", format!("{}\n", serde_json::to_string_pretty(&mcp).unwrap())),
        (".gitignore", ".cw/\n".into()),
    ];
    for (f, c) in &files {
        std::fs::write(dir.join(f), c).map_err(|e| format!("{f}: {e}"))?;
    }
    println!("created {} \"{title}\" ({tpl}): a small working game to change into yours", dir.display());
    println!("  {}", files.iter().map(|f| f.0).collect::<Vec<_>>().join("  "));
    println!("\nnext:\n  cd {path}\n  claude            # then: \"make this a game where …\"\n  kartu web .       # play it in the browser");
    Ok(0)
}

// ---------- setup / doctor ----------

fn skill_text() -> String {
    format!("---\nname: kartu\ndescription: Make small retro games with Kartu, a 320x240 fantasy console (Lua + text art). \
Use when the user wants to make, change, test or play a game or \"cart\", or says \"make me a game\".\n---\n\n\
Start a new game with `kartu new <folder>` (a working starter game + this guide + MCP config), then work \
inside that folder. If the `kartu` MCP tools aren't loaded in this session, use the shell commands below; \
they do the same thing. An existing cart is any folder with main.lua + assets.cw.\n\n{}",
        AGENTS.split_once('\n').map(|x| x.1).unwrap_or(AGENTS))
}

fn claude_dir() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".claude"))
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .map(|d| d.join(bin)).find(|p| p.is_file())
}

fn setup(args: &[String]) -> Result<i32, String> {
    let Some(claude) = which("claude") else {
        println!("Claude Code not found (https://claude.com/claude-code). Kartu still works from any AI \
                  with a shell: `kartu new mygame` writes an AGENTS.md it can follow.");
        return Ok(1);
    };
    let dir = claude_dir().join("skills/kartu");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("SKILL.md"), skill_text()).map_err(|e| e.to_string())?;
    println!("✓ Claude Code skill: {}", dir.join("SKILL.md").display());
    if args.iter().any(|a| a == "--mcp") {
        let _ = std::process::Command::new(&claude).args(["mcp", "remove", "-s", "user", "kartu"]).output();
        let st = std::process::Command::new(&claude)
            .args(["mcp", "add", "-s", "user", "kartu", "--", &kartu_cmd(), "mcp", "--root", "."])
            .output().map_err(|e| e.to_string())?;
        if !st.status.success() {
            return Err(format!("claude mcp add failed: {}", String::from_utf8_lossy(&st.stderr).trim()));
        }
        println!("✓ MCP server \"kartu\" for every Claude Code session (carts = folders under where you start it)");
    } else {
        println!("  carts made with `kartu new` bring their own MCP config (.mcp.json); \
                  `kartu setup --mcp` adds it to every session instead");
    }
    println!("\nready:  kartu new my-game && cd my-game && claude");
    Ok(0)
}

fn doctor() -> Result<i32, String> {
    let mut bad = 0;
    let mut line = |ok: bool, what: String| {
        println!("{} {what}", if ok { "✓" } else { "✗" });
        if !ok {
            bad += 1;
        }
    };
    let runner = crate::runner_path();
    let ver = std::process::Command::new(&runner).arg("version").output().ok()
        .filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    line(ver.is_some(), format!("runner {} {}", runner.display(), ver.unwrap_or_else(|| "(not runnable)".into())));
    println!("✓ kartu-mcp {}", env!("CARGO_PKG_VERSION"));
    let share = config::share_dir();
    line(share.join("web/kartu.wasm").exists(), format!("web player {}", share.join("web").display()));
    let cfgp = config::config_path();
    match config::load() {
        Ok(t) => {
            println!("{} config {}{}", if cfgp.exists() { "✓" } else { "·" }, cfgp.display(),
                if cfgp.exists() { format!(" ({} keys)", t.len()) } else { " (none: defaults)".into() });
            let lib = config::library_dirs(&t);
            line(!lib.is_empty(), format!("art library: {}", if lib.is_empty() { "none".into() } else {
                lib.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ") }));
        }
        Err(e) => line(false, format!("config: {e}")),
    }
    match which("claude") {
        Some(p) => {
            println!("✓ claude {}", p.display());
            line(claude_dir().join("skills/kartu/SKILL.md").exists(), "Claude Code skill (kartu setup)".into());
        }
        None => println!("· claude not found (any AI with a shell works: AGENTS.md)"),
    }
    Ok(if bad == 0 { 0 } else { 1 })
}

fn show_config(args: &[String]) -> Result<i32, String> {
    let t = config::load()?;
    if args.first().map(String::as_str) == Some("get") {
        let k = args.get(1).ok_or("usage: kartu config get KEY")?;
        return Ok(match (k.as_str(), t.get(k.as_str())) {
            ("library", _) => {
                config::library_dirs(&t).iter().for_each(|p| println!("{}", p.display()));
                0
            }
            (_, Some(v)) => {
                println!("{}", v.show());
                0
            }
            _ => 1,
        });
    }
    println!("# config  {}\n# share   {}", config::config_path().display(), config::share_dir().display());
    for (k, v) in &t {
        match v {
            Val::Str(s) => println!("{k} = {s:?}"),
            Val::List(l) => println!("{k} = {l:?}"),
            other => println!("{k} = {}", other.show()),
        }
    }
    Ok(0)
}

// ---------- playtest ----------

fn playtest(args: &[String]) -> Result<i32, String> {
    let dir = PathBuf::from(positional(args).unwrap_or("."));
    config::cart_meta(&dir)?;
    let cfg = here_cfg(&dir)?;
    let mut a = json!({});
    if let Some(p) = flag(args, "--plan") {
        a["plan"] = json!(std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?);
    }
    if let Some(s) = flag(args, "--seeds") {
        a["seeds"] = json!(s.split(',').filter_map(|x| x.trim().parse::<i64>().ok()).collect::<Vec<_>>());
    }
    let out = tools::call(&cfg, "playtest", &a)?;
    print_text(&out);
    println!("(end screenshot: {})", dir.join(".cw/end.png").display());
    let pass = out.first().and_then(|x| x["text"].as_str()).is_some_and(|t| t.starts_with("playtest PASS"));
    Ok(if pass { 0 } else { 1 })
}

// ---------- web ----------

/// `kartu web <cart>`: the browser player on localhost, reading the cart fresh on every
/// load (edit, reload). Serves <share>/web and the one cart.
fn web(args: &[String]) -> Result<i32, String> {
    let dir = PathBuf::from(positional(args).unwrap_or(".")).canonicalize().map_err(|e| e.to_string())?;
    if !dir.join("main.lua").exists() {
        return Err(format!("{} is not a cart (no main.lua)", dir.display()));
    }
    let meta = config::cart_meta(&dir)?;
    let webdir = config::share_dir().join("web");
    if !webdir.join("kartu.wasm").exists() {
        return Err(format!("no web player in {} (reinstall Kartu, or set KARTU_SHARE)", webdir.display()));
    }
    let name = "cart";
    let title = match meta.get("title") {
        Some(Val::Str(t)) => t.clone(),
        _ => dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
    };
    let port: u16 = flag(args, "--port").and_then(|p| p.parse().ok()).unwrap_or(7070);
    let listener = (port..port + 20).find_map(|p| std::net::TcpListener::bind(("127.0.0.1", p)).ok())
        .ok_or("no free port")?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    println!("playing {} at http://localhost:{port}/#cart={name}   (reload after edits · Ctrl-C stops)", dir.display());
    for stream in listener.incoming().flatten() {
        let mut r = BufReader::new(&stream);
        let mut req = String::new();
        if r.read_line(&mut req).is_err() {
            continue;
        }
        loop {
            let mut h = String::new();
            if r.read_line(&mut h).map_or(true, |n| n == 0) || h == "\r\n" || h == "\n" {
                break;
            }
        }
        let path = req.split_whitespace().nth(1).unwrap_or("/").split(['?', '#']).next().unwrap_or("/").to_string();
        let file = match path.as_str() {
            "/" | "/index.html" => Some(webdir.join("index.html")),
            "/kartu.js" | "/kartu.wasm" => Some(webdir.join(&path[1..])),
            p if p == format!("/carts/{name}/main.lua") => Some(dir.join("main.lua")),
            p if p == format!("/carts/{name}/assets.cw") => Some(dir.join("assets.cw")),
            _ => None,
        };
        let body = if path == "/carts/index.json" {
            Some(json!([{"name": name, "title": title}]).to_string().into_bytes())
        } else {
            file.and_then(|f| std::fs::read(f).ok())
        };
        let ct = match path.rsplit('.').next() {
            Some("js") => "text/javascript",
            Some("wasm") => "application/wasm",
            Some("json") => "application/json",
            Some("lua" | "cw") => "text/plain; charset=utf-8",
            _ => "text/html; charset=utf-8",
        };
        let mut w = &stream;
        let _ = match body {
            Some(b) => w.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {}\r\n\
                Cache-Control: no-cache\r\nConnection: close\r\n\r\n", b.len()).as_bytes())
                .and_then(|_| w.write_all(&b)),
            None => w.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
        };
    }
    Ok(0)
}
