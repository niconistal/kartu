//! `kartu-mcp` — the Kartu maker tools as an MCP server (stdio, JSON-RPC 2.0).
//!
//!   kartu-mcp [--root DIR] [--cart NAME] [--library DIR]... [--log FILE]
//!
//! Every cart is its own workspace: a directory `<root>/<cart>` holding `main.lua`,
//! `assets.cw`, `bot.txt` and whatever notes the agent keeps, plus a private scratch dir
//! `<root>/<cart>/.cw/` for screenshots and runner output, so parallel agents never share
//! files. `--cart NAME` pins the server to one cart (a hosted maker runs one
//! server per cart); otherwise every tool takes a `cart` argument.
//!
//! The tools shell out to the `kartu` runner (same directory as this binary, or
//! `$KARTU_BIN`), so a runaway cart can't take the server down with it.
//!
//! `--cart .` makes the folder you start it in the one cart (what `kartu new` writes into
//! .mcp.json). Without `--library`, the art library comes from your config + the shipped
//! starter library (see config.rs).
//!
//! The same binary also runs the shell side of the maker: `kartu-mcp new|setup|doctor|config|
//! docs|playtest|web …` (the `kartu` runner forwards those commands here).
//!
//! Add to Claude Code:  kartu setup   (or: claude mcp add kartu -- kartu mcp --root ./carts)

mod cli;
mod config;
mod library;
mod tools;

use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

pub struct Config {
    pub root: PathBuf,
    pub pinned: Option<String>,
    /// `--cart .`: this directory is the cart (whatever its name)
    pub here: Option<PathBuf>,
    pub library: Vec<PathBuf>,
    pub runner: PathBuf,
    pub log: Option<PathBuf>,
}

const INSTRUCTIONS: &str = "\
Kartu is a 320x240 fantasy console: a game (cart) is main.lua (Lua 5.4) + assets.cw (text art \
and maps). These tools are the whole workflow, one workspace per cart:
1. docs — read the API (start with docs(topic=\"kits\") for top-down games; docs() lists topics).
2. cart_new — make the cart from the top-down starter (a tiny working game), then change it to fit the brief.
3. Art: asset_search / asset_copy reuse library art (sprites, tiles, palettes); or draw your own in assets.cw.
4. cart_write / cart_patch edit files; every edit of main.lua or assets.cw is checked at once and the \
problems come back in the result.
5. run plays it headless: logs, watched values, screenshots you can look at (shots=[frames]).
6. Write bot.txt (a goal plan: docs(topic=\"bots\")) and call playtest: the bot must reach WIN on every \
seed. A game is done when playtest passes and the screenshots look right.
Make log() lines say what happens (got key, room 2, hurt, WIN): bots and playtest read them.";

fn main() {
    // `kartu playtest . | head` should end quietly, not panic on a closed pipe
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(cmd) = args.first().filter(|a| !a.starts_with('-') && a.as_str() != "serve") {
        let code = cli::run(cmd, &args[1..]).unwrap_or_else(|e| {
            eprintln!("kartu: {e}");
            1
        });
        std::process::exit(code);
    }
    let args = if args.first().map(String::as_str) == Some("serve") { args[1..].to_vec() } else { args };
    let mut cfg = Config {
        root: PathBuf::from("."),
        pinned: None,
        here: None,
        library: vec![],
        runner: runner_path(),
        log: None,
    };
    let mut i = 0;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--root" => cfg.root = PathBuf::from(v),
            "--cart" => cfg.pinned = Some(v),
            "--library" => cfg.library.push(PathBuf::from(v)),
            "--log" => cfg.log = Some(PathBuf::from(v)),
            "--runner" => cfg.runner = PathBuf::from(v),
            "-h" | "--help" => {
                println!("usage: kartu-mcp [serve] [--root DIR] [--cart NAME|.] [--library DIR]... [--log FILE]\n       \
                          kartu-mcp new|setup|doctor|config|docs|playtest|web …");
                return;
            }
            other => {
                eprintln!("kartu-mcp: unknown option {other}");
                std::process::exit(1);
            }
        }
        i += 2;
    }
    if cfg.library.is_empty() {
        match config::load() {
            Ok(t) => cfg.library = config::library_dirs(&t),
            Err(e) => eprintln!("kartu-mcp: {e}"),
        }
    }
    if cfg.pinned.as_deref() == Some(".") {
        match cli::here_cfg(&cfg.root) {
            Ok(h) => (cfg.pinned, cfg.here) = (h.pinned, h.here),
            Err(e) => {
                eprintln!("kartu-mcp: --cart .: {e}");
                std::process::exit(1);
            }
        }
    } else if let Some(c) = &cfg.pinned {
        if let Err(e) = tools::valid_cart(c) {
            eprintln!("kartu-mcp: --cart: {e}");
            std::process::exit(1);
        }
    }
    let _ = std::fs::create_dir_all(&cfg.root);

    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                reply(&mut out, json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":e.to_string()}}));
                continue;
            }
        };
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let result: Result<Value, (i64, String)> = match method {
            "initialize" => {
                let ver = params.get("protocolVersion").and_then(|v| v.as_str()).unwrap_or("2025-06-18");
                Ok(json!({
                    "protocolVersion": ver,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "kartu", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": INSTRUCTIONS,
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools::list(&cfg)})),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let a = params.get("arguments").cloned().unwrap_or(json!({}));
                let t0 = std::time::Instant::now();
                let r = tools::call(&cfg, name, &a);
                log_call(&cfg, name, &a, &r, t0.elapsed().as_millis());
                Ok(match r {
                    Ok(content) => json!({"content": content, "isError": false}),
                    Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
                })
            }
            m if m.starts_with("notifications/") => continue,
            _ => Err((-32601, format!("unknown method {method}"))),
        };
        let Some(id) = id else { continue };
        reply(&mut out, match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((code, m)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": m}}),
        });
    }
}

fn reply(out: &mut std::io::Stdout, v: Value) {
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

pub fn runner_path() -> PathBuf {
    if let Ok(p) = std::env::var("KARTU_BIN") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        let sib = exe.with_file_name("kartu");
        if sib.exists() {
            return sib;
        }
    }
    PathBuf::from("kartu")
}

/// One JSON line per tool call (name, cart, ms, ok, sizes): for cost and usage stats.
fn log_call(cfg: &Config, name: &str, a: &Value, r: &Result<Vec<Value>, String>, ms: u128) {
    let Some(path) = &cfg.log else { return };
    let (ok, chars) = match r {
        Ok(c) => (true, c.iter().map(|x| x.to_string().len()).sum::<usize>()),
        Err(e) => (false, e.len()),
    };
    let cart = a.get("cart").and_then(|c| c.as_str()).or(cfg.pinned.as_deref()).unwrap_or("");
    let line = json!({"tool": name, "cart": cart, "ms": ms, "ok": ok, "out_chars": chars,
                      "in_chars": a.to_string().len()});
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}
