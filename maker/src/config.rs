//! Where Kartu's files live, Omarchy-style: shipped defaults you never edit, your
//! overrides, then the cart's own settings.
//!
//!   <share>  = $KARTU_SHARE | $XDG_DATA_HOME/kartu | ~/.local/share/kartu
//!              library/ (starter carts = art to reuse) · packs/ — replaced by the installer
//!   <config> = $KARTU_CONFIG | $XDG_CONFIG_HOME/kartu/config.toml | ~/.config/kartu/config.toml
//!              yours: `library = ["~/art"]`, `judge = "cmd"`, `[device.miyoo] ip = "…"`
//!   <cart>/kartu.toml   title, format — travels with the game
//!
//! The TOML is the flat subset those need: `key = "str" | 12 | true | ["a", "b"]` and
//! `[section]` headers (keys become `section.key`). Anything else is an error with a line number.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The cart format this build reads and writes (`format = N` in kartu.toml).
pub const FORMAT: i64 = 1;

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Str(String),
    Int(i64),
    Bool(bool),
    List(Vec<String>),
}

impl Val {
    pub fn show(&self) -> String {
        match self {
            Val::Str(s) => s.clone(),
            Val::Int(n) => n.to_string(),
            Val::Bool(b) => b.to_string(),
            Val::List(v) => v.join("\n"),
        }
    }
}

pub type Table = BTreeMap<String, Val>;

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

fn xdg(var: &str, dflt: &str) -> PathBuf {
    std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| home().join(dflt))
}

pub fn share_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("KARTU_SHARE") {
        return PathBuf::from(p);
    }
    let user = xdg("XDG_DATA_HOME", ".local/share").join("kartu");
    if user.is_dir() {
        return user;
    }
    // a tarball unpacked anywhere: <prefix>/bin/kartu-mcp + <prefix>/share/kartu
    let beside = std::env::current_exe().ok()
        .and_then(|e| e.parent()?.parent().map(|p| p.join("share/kartu")))
        .filter(|p| p.is_dir());
    beside.unwrap_or(user)
}

pub fn config_path() -> PathBuf {
    std::env::var_os("KARTU_CONFIG").map(PathBuf::from)
        .unwrap_or_else(|| xdg("XDG_CONFIG_HOME", ".config").join("kartu/config.toml"))
}

/// `~/x` → $HOME/x; relative paths are relative to the config file's folder.
pub fn expand(p: &str, base: &Path) -> PathBuf {
    if let Some(r) = p.strip_prefix("~/") {
        home().join(r)
    } else if Path::new(p).is_absolute() {
        PathBuf::from(p)
    } else {
        base.join(p)
    }
}

pub fn parse(src: &str) -> Result<Table, String> {
    let mut t = Table::new();
    let mut section = String::new();
    for (i, raw) in src.lines().enumerate() {
        let l = strip_comment(raw).trim();
        if l.is_empty() {
            continue;
        }
        let err = |m: &str| format!("line {}: {m}: {}", i + 1, raw.trim());
        if let Some(s) = l.strip_prefix('[') {
            section = s.strip_suffix(']').ok_or_else(|| err("bad section"))?.trim().to_string();
            continue;
        }
        let (k, v) = l.split_once('=').ok_or_else(|| err("expected key = value"))?;
        let k = k.trim();
        if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
            return Err(err("bad key"));
        }
        let key = if section.is_empty() { k.to_string() } else { format!("{section}.{k}") };
        t.insert(key, value(v.trim()).ok_or_else(|| err("bad value (\"text\", 12, true or [\"a\", \"b\"])"))?);
    }
    Ok(t)
}

fn strip_comment(l: &str) -> &str {
    let mut in_str = false;
    for (i, c) in l.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '#' if !in_str => return &l[..i],
            _ => {}
        }
    }
    l
}

fn value(v: &str) -> Option<Val> {
    if let Some(s) = v.strip_prefix('"') {
        return s.strip_suffix('"').map(|s| Val::Str(s.replace("\\\"", "\"").replace("\\\\", "\\")));
    }
    if let Some(inner) = v.strip_prefix('[') {
        let inner = inner.strip_suffix(']')?.trim();
        let mut out = vec![];
        for item in inner.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match value(item)? {
                Val::Str(s) => out.push(s),
                _ => return None,
            }
        }
        return Some(Val::List(out));
    }
    match v {
        "true" => Some(Val::Bool(true)),
        "false" => Some(Val::Bool(false)),
        _ => v.parse().ok().map(Val::Int),
    }
}

/// Your config (empty if there is none).
pub fn load() -> Result<Table, String> {
    let p = config_path();
    match std::fs::read_to_string(&p) {
        Ok(s) => parse(&s).map_err(|e| format!("{}: {e}", p.display())),
        Err(_) => Ok(Table::new()),
    }
}

/// Art library dirs: yours first, then the shipped starter library and packs.
pub fn library_dirs(t: &Table) -> Vec<PathBuf> {
    let base = config_path().parent().map(Path::to_path_buf).unwrap_or_default();
    let mut v: Vec<PathBuf> = match t.get("library") {
        Some(Val::List(l)) => l.iter().map(|p| expand(p, &base)).collect(),
        Some(Val::Str(s)) => vec![expand(s, &base)],
        _ => vec![],
    };
    let share = share_dir();
    v.push(share.join("library"));
    if let Ok(rd) = std::fs::read_dir(share.join("packs")) {
        let mut packs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        packs.sort();
        v.extend(packs);
    }
    v.retain(|p| p.is_dir());
    v
}

/// The cart's kartu.toml, if it has one; errors if it needs a newer Kartu.
pub fn cart_meta(dir: &Path) -> Result<Table, String> {
    let Ok(s) = std::fs::read_to_string(dir.join("kartu.toml")) else { return Ok(Table::new()) };
    let t = parse(&s).map_err(|e| format!("kartu.toml {e}"))?;
    if let Some(Val::Int(f)) = t.get("format") {
        if *f > FORMAT {
            return Err(format!("this cart is format {f}; this Kartu reads up to {FORMAT}: update Kartu"));
        }
    }
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_subset() {
        let t = parse("# mine\nlibrary = [\"~/art\", \"packs/neon\"]  # two\njudge = \"my-judge #1\"\n\n[device.miyoo]\nip = \"192.168.1.50\"\nport = 22\n").unwrap();
        assert_eq!(t["library"], Val::List(vec!["~/art".into(), "packs/neon".into()]));
        assert_eq!(t["judge"], Val::Str("my-judge #1".into()));
        assert_eq!(t["device.miyoo.ip"], Val::Str("192.168.1.50".into()));
        assert_eq!(t["device.miyoo.port"], Val::Int(22));
        assert!(parse("oops").unwrap_err().starts_with("line 1"));
        assert!(parse("x = [1, 2]").is_err());
    }
}
