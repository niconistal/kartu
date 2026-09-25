//! `kartu` — the native player and headless runner.
//!
//!   kartu run   <cart-dir> [options]           headless: logs, hash, screenshots (see USAGE)
//!   kartu check <cart-dir> [--frames N]        every asset/syntax/name error in one pass
//!   kartu art import <image.png> --name N    PNG → assets.cw palette + sprite text
//!   kartu bench <cart-dir> [--frames N]        headless timing: gate scene, then A → stress
//!   kartu play  <cart-dir> [--bench-secs N]    Linux framebuffer + evdev (Miyoo Mini, Anbernic, Pi)
//!                          [--pick-out F]      a `PICK <name>` log line writes <name> to F and exits
//!                          [--audio DEV|--mute] sound via OSS (/dev/dsp by default)
//!
//! On emscripten the same crate builds the web player instead (see web.rs).

#[cfg(not(target_os = "emscripten"))]
mod art;
#[cfg(not(target_os = "emscripten"))]
mod bot;
#[cfg(not(target_os = "emscripten"))]
mod cart;
#[cfg(not(target_os = "emscripten"))]
mod check;
#[cfg(all(target_os = "linux", not(target_os = "emscripten")))]
mod fbdev;
#[cfg(all(target_os = "linux", not(target_os = "emscripten")))]
mod oss;
#[cfg(not(target_os = "emscripten"))]
mod headless;
#[cfg(target_os = "emscripten")]
mod web;

#[cfg(not(target_os = "emscripten"))]
fn main() {
    // `kartu run … | head` should end quietly, not panic on a closed pipe
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let args: Vec<String> = std::env::args().collect();
    let code = match args.get(1).map(|s| s.as_str()) {
        Some("run") => headless::run(&args[2..]),
        Some("check") => check::check(&args[2..]),
        Some("sound") => headless::sound(&args[2..]),
        Some("bench") => headless::bench(&args[2..]),
        Some("art") => art::art(&args[2..]),
        #[cfg(target_os = "linux")]
        Some("play") => fbdev::play(&args[2..]),
        Some("mcp") => maker(&args[2..]),
        Some("new" | "setup" | "doctor" | "config" | "docs" | "playtest" | "web") => maker(&args[1..]),
        Some("version") => {
            println!("kartu {} ({})", env!("CARGO_PKG_VERSION"), std::env::consts::ARCH);
            Ok(())
        }
        Some("help" | "--help" | "-h") | None => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command `{other}`\n{USAGE}")),
    };
    if let Err(e) = code {
        eprintln!("kartu: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "emscripten"))]
const USAGE: &str = "\
usage: kartu <command> <cart-dir> [options]

  new <folder>   a new cart: a small working game + AGENTS.md + .mcp.json for your AI
                 --title T  --template topdown|blank
  web <cart>     play it in the browser on localhost (reload after edits) --port N
  playtest <cart>  the goal bot plays bot.txt on seeds 1-3; PASS/FAIL (--seeds 1,2 --plan F)
  docs [topic]   the cart API reference (no topic: the list of topics)
  setup          connect Kartu to Claude Code (skill; --mcp: MCP in every session)
  doctor         check the install · config [get KEY]: your settings and paths

  check <cart>   list every problem at once: all assets.cw errors, main.lua syntax,
                 unknown names in spr/bg.map/bg.tile/pal literals, then a smoke run
                 --frames N (120)  --script F  --press S   exit 1 on any error
  run <cart>     play headless, print [log fN] lines and the final frame hash
                 --frames N        frames to run (600)
                 --seed S          rnd() seed (1; the web player uses 1 too)
                 --script FILE     input script (format below)
                 --press \"S,S\"     input steps inline, added to --script
                 --until TEXT      stop after the frame whose log contains TEXT;
                                   exit 3 if it never appears (win check: --until WIN)
                 --shot out.png    screenshot of the last frame run
                 --shot-at 70,190  screenshots of those frames: out-f70.png, out-f190.png
                 --shot-every N    a screenshot every N frames (same naming)
                 --scale K         screenshot scale (2)
                 --record FILE     write the input actually used, as a script
                 --wav FILE        write the sound of the run (24 kHz mono WAV)
                 --sounds          print [sound fN] lines: sfx/music the cart started
                 --bot PLAN        let a goal bot play (see API.md: goto key, goto flag exit,
                                   tap a, wait until EXPR); exit 4 if it fails or gets stuck
                 --watch \"hero.x;hero.y;state\"   print Lua expressions (globals and
                                   file-level locals) every --watch-every N frames (60)
                 --dump F,F|end    print the cart's whole state as JSON at those frames
                 --save FILE       write a save at the end (the input so far + a hash check)
                 --load FILE       start from a save instead of from boot
                 --persist FILE    the cart's save()/load() string: read at boot, written at
                                   the end ([save fN] lines); without it load() is nil
                 exit 2 if the cart errors (the run stops at the error frame)
  sound <cart>   render one sound without the game: --song NAME | --sfx NAME
                 [--secs N] [--wav F]; no name: list songs, sfx, instruments
  art import <image.png> --name NAME
                 turn any PNG (e.g. from your image AI) into assets.cw text: key out the
                 background, crop, downscale, quantise; prints a palette + sprite(s)
                 --size 32|48x32   longest side, or exact WxH (32); >64 px is cut into
                                   NAME_0_0… pieces of 64 (a comment says where each goes)
                 --colors N        palette size, 1..15 (15)
                 --bg auto|black|none|#rrggbb   background to remove where it touches the
                                   border (auto: the border's main colour; alpha always)
                 --tol N           how close to the background colour counts (34)
                 --outline         1 px outline in the darkest colour
                 --dither 0.3      ordered dither strength (0)
                 --out FILE        write there instead of stdout
                 --append CART     add to CART/assets.cw (refuses name clashes, re-checks)
                 --pal NAME        with --append: reuse the cart's palette NAME
  bench <cart>   headless timing
  play <cart>    Linux framebuffer player (handhelds); --record FILE saves your input
  mcp            the maker tools as an MCP server for AI agents (runs kartu-mcp):
                 --cart . (this folder is the cart) or --root DIR (a folder of carts)
  version

input script: one step per line, or comma/space separated
  30:right+a   from frame 30 hold exactly these (replaces what was held)
  40:          release everything
  +12:up       frame relative to the step before (here 52)
  60:a!        tap: add a on top of what is held for 1 frame (a!5 = 5 frames)
  # or -- starts a comment
frame F means: the buttons btn()/btnp() see in the update() of frame F, which is the
frame logged as [log fF]. btnp is true only if the button was up in frame F-1.
";

/// `kartu mcp …`, `kartu new …` etc. = the AGPL maker's `kartu-mcp` next to this binary (or on PATH).
#[cfg(not(target_os = "emscripten"))]
fn maker(args: &[String]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let sib = exe.with_file_name("kartu-mcp");
    let prog = if sib.exists() { sib } else { "kartu-mcp".into() };
    let st = std::process::Command::new(&prog).args(args).status()
        .map_err(|e| format!("cannot run {}: {e} (install the maker: cargo install kartu-maker)", prog.display()))?;
    std::process::exit(st.code().unwrap_or(1));
}

#[cfg(target_os = "emscripten")]
fn main() {
    // The JS side drives frames through the exported cw_* functions.
}

/// Tiny flag parser shared by the native commands: `--key value` pairs and one positional.
#[cfg(not(target_os = "emscripten"))]
pub struct Args {
    pub pos: Option<String>,
    pub kv: std::collections::HashMap<String, String>,
}

#[cfg(not(target_os = "emscripten"))]
impl Args {
    pub fn parse(a: &[String]) -> Args {
        let mut pos = None;
        let mut kv = std::collections::HashMap::new();
        let mut i = 0;
        while i < a.len() {
            if let Some(k) = a[i].strip_prefix("--") {
                let v = a.get(i + 1).filter(|v| !v.starts_with("--")).cloned();
                if v.is_some() {
                    i += 1;
                }
                kv.insert(k.to_string(), v.unwrap_or_else(|| "1".into()));
            } else if pos.is_none() {
                pos = Some(a[i].clone());
            }
            i += 1;
        }
        Args { pos, kv }
    }
    pub fn num<T: std::str::FromStr>(&self, k: &str, d: T) -> T {
        self.kv.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
    }
    pub fn cart(&self) -> Result<String, String> {
        self.pos.clone().ok_or_else(|| "missing <cart-dir>".to_string())
    }
}
