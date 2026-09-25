//! Goal bots: a plan of `goto key` / `goto flag exit` / `tap a` steps, played closed-loop
//! against the running cart. Each frame the bot reads the hero's box through the inspector,
//! path-finds over the live tile map (solid = the `solid` flag, or tile names given with
//! `solid`), and presses one direction. Pushed back by an enemy? It replans next frame.
//!
//! ```text
//! # plan file: one step per line
//! hero hero                   the expression for the player object (default: hero)
//! solid wall,block            also treat these tiles as solid (carts without flags)
//! tap a                       press a for one frame, release for one
//! hold right+a 20             hold buttons for N frames
//! wait 30                     N frames of nothing
//! wait until state == "play"  until a Lua expression is true
//! goto key                    walk until the hero overlaps that object (done if it's gone)
//! goto flag exit              walk into the nearest tile with that flag (or push into it if solid)
//! goto tile 5,22              walk into that tile
//! fight bats a 28             while walking: an enemy within N px? face it, press the button
//! abort world.state == "over" stop the run (as a failure) as soon as this is true
//! collect world.pickups       go to the nearest (by path) object in the list, again and
//!                             again, until the list is empty
//! avoid bats 24               while walking, route around these objects (a list or one
//!                             object; entries with alive=false or dead=true are skipped),
//!                             keeping N px away when there is another way (default 20)
//! ```
//! Any step can end with `max=N` (frames before it counts as failed; default 1800 / 600).

use kartu_core::{Console, BUTTONS};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
enum Target {
    Expr(String),
    Flag(String),
    Tile(i32, i32),
    Collect(String),
}

#[derive(Debug, Clone)]
enum Cmd {
    Hero(String),
    Solid(Vec<String>),
    Avoid(String, i32),
    Fight(String, u16, i32),
    Abort(String),
    Tap(u16),
    Hold(u16, u64),
    Wait(u64),
    WaitUntil(String),
    Goto(Target),
}

#[derive(Clone)]
struct Step {
    cmd: Cmd,
    max: Option<u64>,
    line: usize,
    text: String,
}

pub struct Bot {
    steps: Vec<Step>,
    pc: usize,
    started: Option<u64>,
    hero: String,
    solid_names: Vec<String>,
    avoid: Vec<(String, i32)>,
    fight: Option<(String, u16, i32)>,
    swing: u64,
    swing_dir: u16,
    aborts: Vec<String>,
    last_pos: Option<(i32, i32)>,
    still: u64,
    pushed: u64,
    waited: u64,
    pub events: Vec<String>,
    pub failed: Option<String>,
    pub done: bool,
}

fn bits(names: &str) -> Result<u16, String> {
    let mut b = 0;
    for n in names.split('+').filter(|n| !n.is_empty()) {
        b |= 1 << BUTTONS.iter().position(|x| *x == n).ok_or_else(|| format!("unknown button `{n}`"))?;
    }
    Ok(b)
}

const LEFT: u16 = 1;
const RIGHT: u16 = 2;
const UP: u16 = 4;
const DOWN: u16 = 8;

impl Bot {
    pub fn parse(src: &str) -> Result<Bot, String> {
        let mut steps = vec![];
        for (i, raw) in src.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let err = |m: &str| format!("plan line {}: {m} (`{line}`)", i + 1);
            let mut words: Vec<&str> = line.split_whitespace().collect();
            let mut max = None;
            if let Some(m) = words.last().and_then(|w| w.strip_prefix("max=")) {
                max = Some(m.parse().map_err(|_| err("bad max="))?);
                words.pop();
            }
            let rest = words[1..].join(" ");
            let num = |s: Option<&&str>| -> Result<u64, String> { s.and_then(|v| v.parse().ok()).ok_or_else(|| err("expected a frame count")) };
            let cmd = match words[0] {
                "hero" if !rest.is_empty() => Cmd::Hero(rest),
                "avoid" if words.len() >= 2 => {
                    let (e, r) = match words.last().and_then(|w| w.parse::<i32>().ok()) {
                        Some(r) if words.len() > 2 => (words[1..words.len() - 1].join(" "), r),
                        _ => (rest.clone(), 20),
                    };
                    Cmd::Avoid(e, r)
                }
                "abort" if !rest.is_empty() => Cmd::Abort(rest),
                "fight" if words.len() >= 3 => {
                    let r = words.get(3).and_then(|w| w.parse().ok()).unwrap_or(28);
                    Cmd::Fight(words[1].to_string(), bits(words[2]).map_err(|e| err(&e))?, r)
                }
                "solid" => Cmd::Solid(rest.split([',', ' ']).filter(|s| !s.is_empty()).map(String::from).collect()),
                "tap" => Cmd::Tap(bits(words.get(1).ok_or_else(|| err("tap what?"))?).map_err(|e| err(&e))?),
                "hold" => Cmd::Hold(bits(words.get(1).ok_or_else(|| err("hold what?"))?).map_err(|e| err(&e))?, num(words.get(2))?),
                "wait" if words.get(1) == Some(&"until") => Cmd::WaitUntil(words[2..].join(" ")),
                "wait" => Cmd::Wait(num(words.get(1))?),
                "collect" if !rest.is_empty() => Cmd::Goto(Target::Collect(rest)),
                "goto" => match words.get(1) {
                    Some(&"flag") => Cmd::Goto(Target::Flag(words.get(2).ok_or_else(|| err("goto flag <name>"))?.to_string())),
                    Some(&"tile") => {
                        let xy: Vec<i32> = words[2..].join("").split(',').filter_map(|v| v.trim().parse().ok()).collect();
                        let [x, y] = xy[..] else { return Err(err("goto tile X,Y")) };
                        Cmd::Goto(Target::Tile(x, y))
                    }
                    Some(_) => Cmd::Goto(Target::Expr(rest)),
                    None => return Err(err("goto where?")),
                },
                _ => return Err(err("unknown step (hero, solid, avoid, fight, abort, tap, hold, wait, goto, collect)")),
            };
            steps.push(Step { cmd, max, line: i + 1, text: line.to_string() });
        }
        Ok(Bot { steps, pc: 0, started: None, hero: "hero".into(), solid_names: vec![], avoid: vec![], fight: None, swing: 0, swing_dir: 0, aborts: vec![], last_pos: None, still: 0, pushed: 0, waited: 0, events: vec![], failed: None, done: false })
    }

    fn event(&mut self, f: u64, msg: String) {
        let e = format!("[bot f{f}] {msg}");
        println!("{e}");
        self.events.push(e);
    }

    fn next(&mut self, f: u64, outcome: &str) {
        let s = &self.steps[self.pc];
        let took = f - self.started.unwrap_or(f);
        let msg = format!("{}: {outcome} ({took} frames)", s.text);
        self.event(f, msg);
        self.pc += 1;
        self.started = None;
        self.last_pos = None;
        self.still = 0;
        self.pushed = 0;
        if self.pc == self.steps.len() {
            self.done = true;
            self.event(f, "plan done".into());
        }
    }

    fn fail(&mut self, f: u64, why: String) -> u16 {
        let s = &self.steps[self.pc];
        let msg = format!("FAILED at plan line {} `{}`: {why}", s.line, s.text);
        self.event(f, msg.clone());
        self.failed = Some(msg);
        0
    }

    /// Buttons for frame `f` (call before `c.step`).
    pub fn buttons(&mut self, c: &Console, f: u64) -> u16 {
        loop {
            if self.done || self.failed.is_some() {
                return 0;
            }
            for e in self.aborts.clone() {
                if c.eval_bool(&e).unwrap_or(false) {
                    return self.fail(f, format!("abort: `{e}`"));
                }
            }
            let step = self.steps[self.pc].clone();
            let start = *self.started.get_or_insert(f);
            let t = f - start;
            let max = step.max.unwrap_or(match step.cmd {
                Cmd::Goto(_) => 1800,
                _ => 600,
            });
            match &step.cmd {
                Cmd::Hero(e) => {
                    self.hero = e.clone();
                    self.pc_quiet();
                    continue;
                }
                Cmd::Solid(names) => {
                    self.solid_names.extend(names.iter().cloned());
                    self.pc_quiet();
                    continue;
                }
                Cmd::Abort(e) => {
                    self.aborts.push(e.clone());
                    self.pc_quiet();
                    continue;
                }
                Cmd::Fight(e, b, r) => {
                    self.fight = Some((e.clone(), *b, *r));
                    self.pc_quiet();
                    continue;
                }
                Cmd::Avoid(e, r) => {
                    self.avoid.push((e.clone(), *r));
                    self.pc_quiet();
                    continue;
                }
                Cmd::Tap(b) => {
                    return match t {
                        0 => *b,
                        _ => {
                            self.next(f, "done");
                            0
                        }
                    };
                }
                Cmd::Hold(b, n) => {
                    if t < *n {
                        return *b;
                    }
                    self.next(f, "done");
                    continue;
                }
                Cmd::Wait(n) => {
                    if t < *n {
                        return 0;
                    }
                    self.next(f, "done");
                    continue;
                }
                Cmd::WaitUntil(e) => match c.eval_bool(e) {
                    Ok(true) => {
                        self.next(f, "true");
                        continue;
                    }
                    Ok(false) if t >= max => return self.fail(f, format!("still false after {max} frames")),
                    Ok(false) => return 0,
                    Err(e) => return self.fail(f, e),
                },
                Cmd::Goto(target) => {
                    if t >= max {
                        return self.fail(f, format!("not there after {max} frames"));
                    }
                    match self.attack(c) {
                        Ok(Some(b)) => return b,
                        Ok(None) => {}
                        Err(e) => return self.fail(f, e),
                    }
                    match self.steer(c, target, t) {
                        Ok(Steer::Press(b)) => {
                            let pos = c.eval_box(&self.hero).ok().flatten().map(|b| (b.0, b.1));
                            if b != 0 && pos.is_some() && pos == self.last_pos {
                                self.still += 1;
                            } else {
                                self.still = 0;
                            }
                            self.last_pos = pos;
                            if self.still >= 60 {
                                let (x, y) = pos.unwrap_or((0, 0));
                                return self.fail(f, format!("stuck at ({x},{y}) px, tile ({},{}), for 60 frames", (x).div_euclid(16), (y).div_euclid(16)));
                            }
                            return b;
                        }
                        Ok(Steer::Arrived(how)) => {
                            self.next(f, how);
                            continue;
                        }
                        Err(e) => return self.fail(f, e),
                    }
                }
            }
        }
    }

    fn pc_quiet(&mut self) {
        self.pc += 1;
        self.started = None;
        if self.pc == self.steps.len() {
            self.done = true;
        }
    }

    fn steer(&mut self, c: &Console, target: &Target, t: u64) -> Result<Steer, String> {
        let hb = c.eval_box(&self.hero)?.ok_or_else(|| format!("hero `{}` is nil", self.hero))?;
        let grid = Grid::read(c, &self.solid_names);
        let (hx, hy) = ((hb.0 + hb.2 / 2).div_euclid(16), (hb.1 + hb.3 / 2).div_euclid(16));
        // goal tile + how to know we're there
        let (goal, obj) = match target {
            Target::Tile(x, y) => ((*x, *y), None),
            Target::Flag(name) => {
                let bit = grid.flag_bit(c, name).ok_or_else(|| format!("no tile has flag `{name}`"))?;
                match grid.nearest(c, (hx, hy), bit) {
                    Some(g) => (g, None),
                    // the door we were pushing on opened (its flag is gone): that's arriving
                    None if !grid.any_flag(c, bit) && t > 0 => return Ok(Steer::Arrived("the flagged tile changed")),
                    None if !grid.any_flag(c, bit) => return Err(format!("no tile with flag `{name}` on the map now")),
                    None => return Err(format!("no reachable tile with flag `{name}`")),
                }
            }
            Target::Expr(e) => match c.eval_box(e)? {
                None => return Ok(Steer::Arrived("gone")),
                Some(b) => (((b.0 + b.2 / 2).div_euclid(16), (b.1 + b.3 / 2).div_euclid(16)), Some(b)),
            },
            Target::Collect(e) => {
                let items = c.eval_boxes(e)?;
                if items.is_empty() {
                    return Ok(Steer::Arrived("all collected"));
                }
                let dist = grid.distances((hx, hy));
                let tile = |b: &(i32, i32, i32, i32)| ((b.0 + b.2 / 2).div_euclid(16), (b.1 + b.3 / 2).div_euclid(16));
                let best = items
                    .iter()
                    .filter_map(|b| {
                        let t = tile(b);
                        let d = *dist.get((t.1 * grid.w + t.0) as usize).filter(|_| t.0 >= 0 && t.1 >= 0 && t.0 < grid.w && t.1 < grid.h)?;
                        (d >= 0).then_some((d, *b))
                    })
                    .min_by_key(|x| x.0)
                    .ok_or_else(|| format!("{} left in `{e}` but none reachable", items.len()))?;
                (tile(&best.1), Some(best.1))
            }
        };
        if let Some(b) = obj {
            if hb.0 < b.0 + b.2 && b.0 < hb.0 + hb.2 && hb.1 < b.1 + b.3 && b.1 < hb.1 + hb.3 {
                if matches!(target, Target::Collect(_)) {
                    return Ok(Steer::Press(0)); // the game takes it this frame; then on to the next
                }
                return Ok(Steer::Arrived("reached"));
            }
        }
        let goal_blocked = grid.blocked(goal.0, goal.1);
        if obj.is_none() && !goal_blocked && inside(hb, goal) {
            return Ok(Steer::Arrived("reached"));
        }
        let danger = self.danger(c, &grid)?;
        let path = grid
            .path_avoiding((hx, hy), goal, &danger)
            .or_else(|| grid.path((hx, hy), goal))
            .ok_or_else(|| format!("no path from tile ({hx},{hy}) to ({},{})", goal.0, goal.1))?;
        if path.len() <= 1 {
            // in the goal tile: close in on the object's centre / the tile's centre
            let (tx, ty) = match obj {
                Some(b) => (b.0 + b.2 / 2, b.1 + b.3 / 2),
                None => (goal.0 * 16 + 8, goal.1 * 16 + 8),
            };
            return Ok(Steer::Press(toward(hb, tx, ty)));
        }
        let n = path[1];
        // timing: if the next cell is dangerous and this one isn't, let the enemy pass first
        // (at most 2 s in a row, so a parked enemy can't freeze the bot)
        let di = |p: (i32, i32)| danger.get((p.1 * grid.w + p.0) as usize).copied().unwrap_or(false);
        if di(n) && !di((hx, hy)) && self.waited < 120 {
            self.waited += 1;
            self.still = 0;
            return Ok(Steer::Press(0));
        }
        if !di(n) {
            self.waited = 0;
        }
        if goal_blocked && n == goal {
            // pushing into a solid goal (a locked door): lined up, then push a while
            let d = dir((hx, hy), n);
            if lined_up(hb, n, d) {
                self.pushed += 1;
                if self.pushed > 20 {
                    return Ok(Steer::Arrived("pushed into it"));
                }
                self.still = 0;
                return Ok(Steer::Press(d));
            }
        }
        let d = dir((hx, hy), n);
        if lined_up(hb, n, d) {
            Ok(Steer::Press(d))
        } else if d == LEFT || d == RIGHT {
            Ok(Steer::Press(if hb.1 + hb.3 / 2 < n.1 * 16 + 8 { DOWN } else { UP }))
        } else {
            Ok(Steer::Press(if hb.0 + hb.2 / 2 < n.0 * 16 + 8 { RIGHT } else { LEFT }))
        }
    }
}

impl Bot {
    /// Fighting: enemy in range → one frame facing it (a fresh press, so the game turns),
    /// then one frame of the attack button, then a short cooldown.
    fn attack(&mut self, c: &Console) -> Result<Option<u16>, String> {
        let Some((e, btn, r)) = self.fight.clone() else { return Ok(None) };
        if self.swing > 0 {
            self.swing -= 1;
            return Ok(match self.swing {
                s if s >= 18 => Some(btn),
                _ => None,
            });
        }
        let Some(h) = c.eval_box(&self.hero)? else { return Ok(None) };
        let (hx, hy) = (h.0 + h.2 / 2, h.1 + h.3 / 2);
        let near = c.eval_boxes(&e)?.into_iter().map(|b| (b.0 + b.2 / 2, b.1 + b.3 / 2)).filter(|(x, y)| (x - hx).abs() <= r && (y - hy).abs() <= r).min_by_key(|(x, y)| (x - hx).abs() + (y - hy).abs());
        let Some((ex, ey)) = near else { return Ok(None) };
        self.swing_dir = toward(h, ex, ey);
        self.swing = 19; // this frame: face; next frame: button; then 18 frames of walking
        Ok(Some(self.swing_dir))
    }

    /// Cells within the avoid radius of any listed object, right now.
    fn danger(&self, c: &Console, g: &Grid) -> Result<Vec<bool>, String> {
        let mut d = vec![false; (g.w * g.h) as usize];
        for (e, r) in &self.avoid {
            for b in c.eval_boxes(e)? {
                let (x0, y0) = ((b.0 - r).div_euclid(16), (b.1 - r).div_euclid(16));
                let (x1, y1) = ((b.0 + b.2 + r).div_euclid(16), (b.1 + b.3 + r).div_euclid(16));
                for y in y0.max(0)..=y1.min(g.h - 1) {
                    for x in x0.max(0)..=x1.min(g.w - 1) {
                        d[(y * g.w + x) as usize] = true;
                    }
                }
            }
        }
        Ok(d)
    }
}

enum Steer {
    Press(u16),
    Arrived(&'static str),
}

fn dir(a: (i32, i32), b: (i32, i32)) -> u16 {
    match (b.0 - a.0, b.1 - a.1) {
        (1, _) => RIGHT,
        (-1, _) => LEFT,
        (_, 1) => DOWN,
        _ => UP,
    }
}

/// Is the box inside tile `n`'s row (moving sideways) or column (moving up/down)?
/// Boxes of 16 px or more only need to be within 2 px of centred.
fn lined_up(b: (i32, i32, i32, i32), n: (i32, i32), d: u16) -> bool {
    let fits = |pos: i32, len: i32, cell: i32| {
        if len >= 16 {
            (pos + len / 2 - (cell * 16 + 8)).abs() <= 2
        } else {
            pos >= cell * 16 && pos + len <= cell * 16 + 16
        }
    };
    if d == LEFT || d == RIGHT {
        fits(b.1, b.3, n.1)
    } else {
        fits(b.0, b.2, n.0)
    }
}

fn inside(b: (i32, i32, i32, i32), t: (i32, i32)) -> bool {
    lined_up(b, t, LEFT) && lined_up(b, t, UP)
}

fn toward(b: (i32, i32, i32, i32), x: i32, y: i32) -> u16 {
    let (dx, dy) = (x - (b.0 + b.2 / 2), y - (b.1 + b.3 / 2));
    if dx.abs() >= dy.abs() && dx != 0 {
        if dx > 0 { RIGHT } else { LEFT }
    } else if dy != 0 {
        if dy > 0 { DOWN } else { UP }
    } else {
        0
    }
}

/// The walkable grid right now: a cell is blocked if it's outside every map or any mapped
/// layer has a solid tile there.
struct Grid {
    w: i32,
    h: i32,
    blocked: Vec<bool>,
}

impl Grid {
    fn read(c: &Console, names: &[String]) -> Grid {
        let s = c.st.borrow();
        let bit = s.assets.flag_bit("solid").unwrap_or(0);
        let ids: Vec<bool> = s.assets.tiles.iter().map(|t| t.flags & bit != 0 || names.contains(&t.name)).collect();
        let layers: Vec<_> = s.gfx.layers.iter().filter(|l| !l.cells.is_empty()).collect();
        let w = layers.iter().map(|l| l.w).max().unwrap_or(0) as i32;
        let h = layers.iter().map(|l| l.h).max().unwrap_or(0) as i32;
        let mut blocked = vec![true; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let mut inmap = false;
                let mut solid = false;
                for l in &layers {
                    if (x as usize) < l.w && (y as usize) < l.h {
                        inmap = true;
                        let cell = l.cells[y as usize * l.w + x as usize];
                        solid |= cell != 0 && ids[cell as usize - 1];
                    }
                }
                blocked[(y * w + x) as usize] = !inmap || solid;
            }
        }
        Grid { w, h, blocked }
    }

    fn blocked(&self, x: i32, y: i32) -> bool {
        x < 0 || y < 0 || x >= self.w || y >= self.h || self.blocked[(y * self.w + x) as usize]
    }

    fn flag_bit(&self, c: &Console, name: &str) -> Option<u16> {
        c.st.borrow().assets.flag_bit(name)
    }

    fn has_flag(&self, c: &Console, x: i32, y: i32, bit: u16) -> bool {
        let s = c.st.borrow();
        s.gfx.layers.iter().any(|l| {
            (x as usize) < l.w && (y as usize) < l.h && x >= 0 && y >= 0 && {
                let cell = l.cells[y as usize * l.w + x as usize];
                cell != 0 && s.assets.tiles[cell as usize - 1].flags & bit != 0
            }
        })
    }

    fn any_flag(&self, c: &Console, bit: u16) -> bool {
        (0..self.h).any(|y| (0..self.w).any(|x| self.has_flag(c, x, y, bit)))
    }

    /// BFS to the nearest flagged cell (it may itself be solid: a door).
    fn nearest(&self, c: &Console, from: (i32, i32), bit: u16) -> Option<(i32, i32)> {
        self.bfs(from, |x, y| self.has_flag(c, x, y, bit)).map(|p| *p.last().unwrap())
    }

    /// Steps from `from` to every cell (-1 = unreachable); a blocked cell gets a distance
    /// if a neighbour is reachable (things can sit in a wall's cell edge).
    fn distances(&self, from: (i32, i32)) -> Vec<i32> {
        let mut d = vec![-1; (self.w * self.h) as usize];
        if self.w == 0 {
            return d;
        }
        let start = (from.0.clamp(0, self.w - 1), from.1.clamp(0, self.h - 1));
        let mut q = VecDeque::from([start]);
        d[(start.1 * self.w + start.0) as usize] = 0;
        while let Some((x, y)) = q.pop_front() {
            let here = d[(y * self.w + x) as usize];
            if (x, y) != start && self.blocked(x, y) {
                continue;
            }
            for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if nx < 0 || ny < 0 || nx >= self.w || ny >= self.h || d[(ny * self.w + nx) as usize] >= 0 {
                    continue;
                }
                d[(ny * self.w + nx) as usize] = here + 1;
                q.push_back((nx, ny));
            }
        }
        d
    }

    fn path(&self, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
        self.bfs(from, |x, y| (x, y) == to)
    }

    /// Path that keeps out of `danger` cells (except where it starts and ends).
    fn path_avoiding(&self, from: (i32, i32), to: (i32, i32), danger: &[bool]) -> Option<Vec<(i32, i32)>> {
        if !danger.iter().any(|d| *d) {
            return self.path(from, to);
        }
        let mut g = Grid { w: self.w, h: self.h, blocked: self.blocked.clone() };
        for (i, d) in danger.iter().enumerate() {
            let (x, y) = (i as i32 % self.w, i as i32 / self.w);
            if *d && (x, y) != from && (x, y) != to {
                g.blocked[i] = true;
            }
        }
        g.path(from, to)
    }

    fn bfs(&self, from: (i32, i32), goal: impl Fn(i32, i32) -> bool) -> Option<Vec<(i32, i32)>> {
        if self.w == 0 {
            return None;
        }
        let idx = |x: i32, y: i32| (y * self.w + x) as usize;
        let mut prev = vec![usize::MAX; (self.w * self.h) as usize];
        let mut q = VecDeque::new();
        let start = (from.0.clamp(0, self.w - 1), from.1.clamp(0, self.h - 1));
        prev[idx(start.0, start.1)] = idx(start.0, start.1);
        q.push_back(start);
        while let Some((x, y)) = q.pop_front() {
            if goal(x, y) {
                let mut p = vec![(x, y)];
                let mut i = idx(x, y);
                while i != idx(start.0, start.1) {
                    i = prev[i];
                    p.push((i as i32 % self.w, i as i32 / self.w));
                }
                p.reverse();
                return Some(p);
            }
            if (x, y) != start && self.blocked(x, y) {
                continue; // a solid goal can be the last step, never a way through
            }
            for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if nx < 0 || ny < 0 || nx >= self.w || ny >= self.h || prev[idx(nx, ny)] != usize::MAX {
                    continue;
                }
                if self.blocked(nx, ny) && !goal(nx, ny) {
                    continue;
                }
                prev[idx(nx, ny)] = idx(x, y);
                q.push_back((nx, ny));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plans() {
        let b = Bot::parse("# c\nhero {x = p.x, y = p.y, w = 8, h = 8}\nsolid wall, block\navoid bats 24\ntap a\nhold right+a 12\nwait 3\nwait until state == 'play' max=90\ngoto key\ngoto flag door\ngoto tile 5, 22\n").unwrap();
        assert_eq!(b.steps.len(), 10);
        assert!(matches!(&b.steps[0].cmd, Cmd::Hero(e) if e == "{x = p.x, y = p.y, w = 8, h = 8}"));
        assert!(matches!(&b.steps[1].cmd, Cmd::Solid(n) if n == &["wall", "block"]));
        assert!(matches!(&b.steps[2].cmd, Cmd::Avoid(e, 24) if e == "bats"));
        assert!(matches!(b.steps[4].cmd, Cmd::Hold(0b10010, 12)));
        assert!(matches!(&b.steps[6].cmd, Cmd::WaitUntil(e) if e == "state == 'play'") && b.steps[6].max == Some(90));
        assert!(matches!(b.steps[9].cmd, Cmd::Goto(Target::Tile(5, 22))));
        assert!(Bot::parse("jump").err().unwrap().contains("line 1"));
        assert!(Bot::parse("tap z").err().unwrap().contains("unknown button"));
    }
}
