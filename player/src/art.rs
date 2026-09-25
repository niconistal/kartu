//! `kartu art import <image.png> --name NAME`: any PNG → assets.cw text.
//!
//! Key out the background (flood fill from the border, so dark details inside the subject
//! survive), crop to the subject, area-average down to the target size, quantise to ≤15
//! colours (darkest first), optional ordered dither and 1 px outline, then cut into ≤64 px
//! sprites. Output goes to stdout, `--out FILE`, or is appended to a cart's assets.cw.

use crate::Args;
use kartu_core::assets::{to15, Assets};
use std::fmt::Write as _;

const CHARS: &[u8] = b"123456789abcdef";
const PIECE: usize = 64;
const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

pub fn art(a: &[String]) -> Result<(), String> {
    match a.first().map(|s| s.as_str()) {
        Some("import") => import(&a[1..]),
        _ => Err("usage: kartu art import <image.png> --name NAME [options] (see kartu help)".into()),
    }
}

type Rgb = [f32; 3];

struct Pic {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

fn import(args: &[String]) -> Result<(), String> {
    let mut a = Args::parse(args);
    // a bare flag swallows the next word: `--outline hero.png`
    if a.pos.is_none() {
        if let Some(v) = a.kv.get("outline").filter(|v| *v != "1" && *v != "0").cloned() {
            a.pos = Some(v);
            a.kv.insert("outline".into(), "1".into());
        }
    }
    let path = a.pos.clone().ok_or("missing <image.png>")?;
    let name = a.kv.get("name").cloned().ok_or("missing --name NAME")?;
    if name.is_empty() || name.chars().any(|c| c.is_whitespace() || c == '=') {
        return Err(format!("bad name `{name}`"));
    }
    let (tw, th) = parse_size(a.kv.get("size").map(|s| s.as_str()).unwrap_or("32"))?;
    let colors: usize = a.num("colors", 15);
    if !(1..=15).contains(&colors) {
        return Err("--colors must be 1..15".into());
    }
    let dither: f32 = a.num("dither", 0.0);
    let tol: i32 = a.num("tol", 34);
    let outline = a.kv.get("outline").is_some_and(|v| v != "0");
    let cart = a.kv.get("append").cloned();
    if cart.is_some() && a.kv.contains_key("out") {
        return Err("use --out or --append, not both".into());
    }

    let existing = match &cart {
        Some(dir) => {
            let p = std::path::Path::new(dir).join("assets.cw");
            Some(std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?)
        }
        None => None,
    };
    let parsed = existing.as_deref().map(|s| Assets::parse_all(s).0);

    let pic = load(&path)?;
    let mask = key(&pic, a.kv.get("bg").map(|s| s.as_str()).unwrap_or("auto"), tol)?;
    let (bx0, by0, bx1, by1) = bbox(&pic, &mask).ok_or("nothing left after keying out the background (try --bg none)")?;
    let pad = if outline { 1 } else { 0 };
    let (ow, oh) = match (tw, th) {
        (n, None) => {
            let (sw, sh) = ((bx1 - bx0) as f32, (by1 - by0) as f32);
            let s = (n - 2 * pad) as f32 / sw.max(sh);
            (((sw * s).round() as usize).max(1) + 2 * pad, ((sh * s).round() as usize).max(1) + 2 * pad)
        }
        (w, Some(h)) => (w, h),
    };
    if ow <= 2 * pad || oh <= 2 * pad {
        return Err("--size too small".into());
    }
    let small = shrink(&pic, &mask, (bx0, by0, bx1, by1), ow - 2 * pad, oh - 2 * pad);
    if small.px.iter().all(|c| c[3] == 0) {
        return Err("the subject vanished at this --size (try a bigger one)".into());
    }

    // palette: (char, colour) for entries 1.., darkest first
    let (pal_name, pal, new_pal): (String, Vec<(char, [u8; 3])>, bool) = match a.kv.get("pal") {
        Some(pn) => {
            let assets = parsed.as_ref().ok_or("--pal NAME needs --append <cart> (the palette comes from its assets.cw)")?;
            let pi = *assets.pal_names.get(pn).ok_or_else(|| format!("no palette `{pn}` in the cart's assets.cw"))?;
            let p = &assets.palettes[pi as usize];
            let mut v: Vec<_> = p.chars.iter().copied().zip(p.colors.iter().copied()).skip(1).collect();
            v.sort_by_key(|(_, c)| lum(c));
            (pn.clone(), v, false)
        }
        None => {
            let reserve = outline && colors > 1;
            let mut cols = quantise(&small, if reserve { colors - 1 } else { colors });
            if reserve && cols.first().is_none_or(|c| lum(c) > 40_000) {
                cols.insert(0, [16, 12, 24]);
            }
            let cols = cols.iter().enumerate().map(|(i, c)| (CHARS[i] as char, *c)).collect();
            (name.clone(), cols, true)
        }
    };

    let mut grid = vec!['.'; ow * oh];
    let fcols: Vec<Rgb> = pal.iter().map(|(_, c)| c.map(|v| v as f32)).collect();
    for y in 0..small.h {
        for x in 0..small.w {
            let p = small.px[y * small.w + x];
            if p[3] == 0 {
                continue;
            }
            let d = (BAYER[y & 3][x & 3] as f32 / 16.0 - 0.5) * 48.0 * dither;
            let c = [p[0] as f32 + d, p[1] as f32 + d, p[2] as f32 + d];
            grid[(y + pad) * ow + x + pad] = pal[nearest(&fcols, c)].0;
        }
    }
    if outline {
        let ink = pal[0].0;
        let src = grid.clone();
        for y in 0..oh {
            for x in 0..ow {
                let solid = |dx: isize, dy: isize| {
                    let (nx, ny) = (x as isize + dx, y as isize + dy);
                    nx >= 0 && ny >= 0 && (nx as usize) < ow && (ny as usize) < oh && src[ny as usize * ow + nx as usize] != '.'
                };
                if src[y * ow + x] == '.' && (solid(1, 0) || solid(-1, 0) || solid(0, 1) || solid(0, -1)) {
                    grid[y * ow + x] = ink;
                }
            }
        }
    }

    // pieces of ≤64 px, empty ones skipped
    let mut sprites = vec![];
    let split = ow > PIECE || oh > PIECE;
    for j in (0..oh).step_by(PIECE) {
        for i in (0..ow).step_by(PIECE) {
            let (w, h) = (PIECE.min(ow - i), PIECE.min(oh - j));
            let rows: Vec<String> = (j..j + h).map(|y| grid[y * ow + i..y * ow + i + w].iter().collect()).collect();
            if split && rows.iter().all(|r| r.chars().all(|c| c == '.')) {
                continue;
            }
            let n = if split { format!("{name}_{}_{}", i / PIECE, j / PIECE) } else { name.clone() };
            sprites.push((n, i, j, w, h, rows));
        }
    }

    let file = std::path::Path::new(&path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or(path.clone());
    let mut out = format!("-- {name}: {ow}x{oh}, imported from {file} by kartu art import\n");
    if new_pal {
        let _ = writeln!(out, "palette {pal_name}\n  . clear");
        for (ch, c) in &pal {
            let _ = writeln!(out, "  {ch} #{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
        }
    }
    if split {
        let at: Vec<String> = sprites.iter().map(|s| format!("{} at +{},+{}", s.0, s.1, s.2)).collect();
        let _ = writeln!(out, "-- {name} is cut into 64 px pieces; draw each at (x, y) plus: {}", at.join(", "));
    }
    for (n, _, _, w, h, rows) in &sprites {
        let _ = writeln!(out, "sprite {n} {w}x{h} pal={pal_name}");
        for r in rows {
            let _ = writeln!(out, "{r}");
        }
    }

    match (&cart, existing, parsed) {
        (Some(dir), Some(old), Some(assets)) => {
            if new_pal && assets.pal_names.contains_key(&pal_name) {
                return Err(format!("the cart already has a palette `{pal_name}` (pick another --name, or reuse it with --pal {pal_name})"));
            }
            if let Some(s) = sprites.iter().find(|s| assets.names.contains_key(&s.0)) {
                return Err(format!("the cart already has something called `{}`", s.0));
            }
            let sep = if old.is_empty() || old.ends_with("\n\n") { "" } else if old.ends_with('\n') { "\n" } else { "\n\n" };
            let new = format!("{old}{sep}{out}");
            let (_, errs) = Assets::parse_all(&new);
            if !errs.is_empty() {
                return Err(format!("not appended, assets.cw would not check:\n  {}", errs.join("\n  ")));
            }
            let p = std::path::Path::new(dir).join("assets.cw");
            std::fs::write(&p, new).map_err(|e| format!("{}: {e}", p.display()))?;
            let names: Vec<&str> = sprites.iter().map(|s| s.0.as_str()).collect();
            eprintln!("{}: added {}{}", p.display(), if new_pal { format!("palette {pal_name}, ") } else { String::new() }, names.join(" "));
        }
        _ => match a.kv.get("out") {
            Some(f) => std::fs::write(f, out).map_err(|e| format!("{f}: {e}"))?,
            None => print!("{out}"),
        },
    }
    Ok(())
}

fn parse_size(s: &str) -> Result<(usize, Option<usize>), String> {
    let bad = || format!("--size `{s}`: use N (longest side) or WxH, 1..=256");
    let ok = |n: usize| (1..=256).contains(&n);
    match s.split_once('x') {
        Some((w, h)) => {
            let (w, h): (usize, usize) = (w.parse().map_err(|_| bad())?, h.parse().map_err(|_| bad())?);
            if ok(w) && ok(h) { Ok((w, Some(h))) } else { Err(bad()) }
        }
        None => s.parse().ok().filter(|&n| ok(n)).map(|n| (n, None)).ok_or_else(bad),
    }
}

fn load(path: &str) -> Result<Pic, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    if !bytes.starts_with(b"\x89PNG") {
        return Err(format!("{path}: not a PNG; convert it first (e.g. `convert in.jpg out.png` or `ffmpeg -i in.webp out.png`)"));
    }
    let mut dec = png::Decoder::new(&bytes[..]);
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut r = dec.read_info().map_err(|e| format!("{path}: {e}"))?;
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).map_err(|e| format!("{path}: {e}"))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let n = info.color_type.samples();
    let px = buf[..info.buffer_size()]
        .chunks(n)
        .map(|c| match n {
            1 => [c[0], c[0], c[0], 255],
            2 => [c[0], c[0], c[0], c[1]],
            3 => [c[0], c[1], c[2], 255],
            _ => [c[0], c[1], c[2], c[3]],
        })
        .collect();
    Ok(Pic { w, h, px })
}

/// true = subject. Transparent pixels are always background; `--bg` colour only where it
/// touches the border (flood fill), so the same colour inside the subject stays.
fn key(p: &Pic, bg: &str, tol: i32) -> Result<Vec<bool>, String> {
    let border: Vec<usize> = (0..p.w).flat_map(|x| [x, (p.h - 1) * p.w + x]).chain((0..p.h).flat_map(|y| [y * p.w, y * p.w + p.w - 1])).collect();
    let target: Option<[i32; 3]> = match bg {
        "none" => None,
        "black" => Some([0, 0, 0]),
        "auto" => {
            // most common border colour (4-bit buckets), if it rules the border
            let opaque: Vec<[u8; 4]> = border.iter().map(|&i| p.px[i]).filter(|c| c[3] >= 128).collect();
            let mut counts = std::collections::BTreeMap::new();
            for c in &opaque {
                *counts.entry([c[0] >> 4, c[1] >> 4, c[2] >> 4]).or_insert(0usize) += 1;
            }
            let top = counts.iter().max_by_key(|(k, n)| (**n, std::cmp::Reverse(**k)));
            match top {
                Some((k, &n)) if opaque.len() * 2 > border.len() && n * 5 >= opaque.len() * 2 => {
                    let m: Vec<_> = opaque.iter().filter(|c| [c[0] >> 4, c[1] >> 4, c[2] >> 4] == *k).collect();
                    let avg = |i: usize| (m.iter().map(|c| c[i] as usize).sum::<usize>() / m.len()) as i32;
                    let c = [avg(0), avg(1), avg(2)];
                    eprintln!("art: background #{:02x}{:02x}{:02x} (from the border; --bg none to keep it)", c[0], c[1], c[2]);
                    Some(c)
                }
                _ => None,
            }
        }
        s => {
            let c = kartu_core::assets::parse_hex(s).ok_or_else(|| format!("--bg `{s}`: use auto, black, none or #rrggbb"))?;
            Some(c.map(|v| v as i32))
        }
    };
    let mut keep: Vec<bool> = p.px.iter().map(|c| c[3] >= 128).collect();
    if let Some(t) = target {
        let near = |c: [u8; 4]| (0..3).all(|k| (c[k] as i32 - t[k]).abs() <= tol);
        let mut st: Vec<usize> = border.into_iter().filter(|&i| keep[i] && near(p.px[i])).collect();
        while let Some(i) = st.pop() {
            if !keep[i] || !near(p.px[i]) {
                continue;
            }
            keep[i] = false;
            let (x, y) = (i % p.w, i / p.w);
            if x > 0 { st.push(i - 1) }
            if x + 1 < p.w { st.push(i + 1) }
            if y > 0 { st.push(i - p.w) }
            if y + 1 < p.h { st.push(i + p.w) }
        }
    }
    Ok(keep)
}

fn bbox(p: &Pic, m: &[bool]) -> Option<(usize, usize, usize, usize)> {
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for (i, _) in m.iter().enumerate().filter(|(_, &k)| k) {
        let (x, y) = (i % p.w, i / p.w);
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
    }
    (x1 > 0).then_some((x0, y0, x1, y1))
}

/// Area-average the box down (or up) to w×h. Colour averages opaque pixels only, so the
/// keyed-out background does not bleed into the edges; a pixel is opaque if ≥43% covered.
fn shrink(p: &Pic, m: &[bool], (x0, y0, x1, y1): (usize, usize, usize, usize), w: usize, h: usize) -> Pic {
    let (sx, sy) = ((x1 - x0) as f32 / w as f32, (y1 - y0) as f32 / h as f32);
    let mut px = Vec::with_capacity(w * h);
    for oy in 0..h {
        let (fy0, fy1) = (y0 as f32 + oy as f32 * sy, y0 as f32 + (oy + 1) as f32 * sy);
        for ox in 0..w {
            let (fx0, fx1) = (x0 as f32 + ox as f32 * sx, x0 as f32 + (ox + 1) as f32 * sx);
            let (mut acc, mut cov, mut area) = ([0f32; 3], 0f32, 0f32);
            for y in fy0.floor() as usize..(fy1.ceil() as usize).min(y1) {
                let wy = (fy1.min(y as f32 + 1.0) - fy0.max(y as f32)).max(0.0);
                for x in fx0.floor() as usize..(fx1.ceil() as usize).min(x1) {
                    let wt = wy * (fx1.min(x as f32 + 1.0) - fx0.max(x as f32)).max(0.0);
                    area += wt;
                    let i = y * p.w + x;
                    if m[i] {
                        cov += wt;
                        for (a, &v) in acc.iter_mut().zip(&p.px[i]) {
                            *a += v as f32 * wt;
                        }
                    }
                }
            }
            px.push(if area > 0.0 && cov / area >= 0.43 {
                let c = acc.map(|v| (v / cov).round().clamp(0.0, 255.0) as u8);
                [c[0], c[1], c[2], 255]
            } else {
                [0; 4]
            });
        }
    }
    Pic { w, h, px }
}

fn lum(c: &[u8; 3]) -> u32 {
    299 * c[0] as u32 + 587 * c[1] as u32 + 114 * c[2] as u32
}

fn dist(a: Rgb, b: Rgb) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    2.0 * d[0] * d[0] + 4.0 * d[1] * d[1] + 3.0 * d[2] * d[2]
}

fn nearest(cols: &[Rgb], c: Rgb) -> usize {
    (0..cols.len()).min_by(|&i, &j| dist(cols[i], c).total_cmp(&dist(cols[j], c))).unwrap_or(0)
}

/// Median cut over the opaque pixels, a few k-means passes, snapped to the console's 15-bit
/// colours, near-duplicates dropped, darkest first.
fn quantise(p: &Pic, n: usize) -> Vec<[u8; 3]> {
    let mut hist = std::collections::BTreeMap::new();
    for c in p.px.iter().filter(|c| c[3] > 0) {
        *hist.entry([c[0], c[1], c[2]]).or_insert(0u32) += 1;
    }
    let pts: Vec<([u8; 3], f32)> = hist.into_iter().map(|(c, k)| (c, k as f32)).collect();
    let mut boxes: Vec<Vec<usize>> = vec![(0..pts.len()).collect()];
    while boxes.len() < n {
        // split the box with the widest channel spread (× its population); boxes spanning
        // under 16 levels are left alone, so anti-aliased edges don't eat the palette
        let spread = |b: &Vec<usize>| -> (f32, usize) {
            (0..3)
                .map(|k| {
                    let (lo, hi) = b.iter().fold((255u8, 0u8), |(lo, hi), &i| (lo.min(pts[i].0[k]), hi.max(pts[i].0[k])));
                    let pop: f32 = b.iter().map(|&i| pts[i].1).sum();
                    let r = hi - lo;
                    (if r < 16 { 0.0 } else { r as f32 * pop.sqrt() }, k)
                })
                .fold((0.0, 0), |a, b| if b.0 > a.0 { b } else { a })
        };
        let Some((bi, (s, k))) = boxes.iter().map(spread).enumerate().fold(None, |a: Option<(usize, (f32, usize))>, (i, s)| match a {
            Some(a) if a.1 .0 >= s.0 => Some(a),
            _ => Some((i, s)),
        }) else { break };
        if s <= 0.0 {
            break;
        }
        let mut b = boxes.swap_remove(bi);
        b.sort_by_key(|&i| (pts[i].0[k], pts[i].0));
        let half: f32 = b.iter().map(|&i| pts[i].1).sum::<f32>() / 2.0;
        let (mut run, mut cut) = (0.0, 1);
        for (j, &i) in b.iter().enumerate() {
            run += pts[i].1;
            if run >= half {
                cut = (j + 1).clamp(1, b.len() - 1);
                break;
            }
        }
        let rest = b.split_off(cut);
        boxes.push(b);
        boxes.push(rest);
    }
    let mean = |ix: &mut dyn Iterator<Item = usize>| -> Option<Rgb> {
        let (mut s, mut n) = ([0f32; 3], 0f32);
        for i in ix {
            for (a, &v) in s.iter_mut().zip(&pts[i].0) {
                *a += v as f32 * pts[i].1;
            }
            n += pts[i].1;
        }
        (n > 0.0).then(|| s.map(|v| v / n))
    };
    let mut cents: Vec<Rgb> = boxes.iter().filter_map(|b| mean(&mut b.iter().copied())).collect();
    for _ in 0..6 {
        let own: Vec<usize> = pts.iter().map(|(c, _)| nearest(&cents, c.map(|v| v as f32))).collect();
        cents = (0..cents.len())
            .map(|j| mean(&mut (0..pts.len()).filter(|&i| own[i] == j)).unwrap_or(cents[j]))
            .collect();
    }
    let mut out: Vec<[u8; 3]> = cents.iter().map(|c| to15(c.map(|v| v.round() as u8))).collect();
    out.sort_by_key(|c| (lum(c), *c));
    let mut kept: Vec<[u8; 3]> = vec![];
    for c in out {
        if !kept.iter().any(|k| (0..3).all(|i| k[i].abs_diff(c[i]) <= 12)) {
            kept.push(c);
        }
    }
    kept
}
