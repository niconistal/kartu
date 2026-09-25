//! Runner-side inspector: read a cart's state from outside without the cart's help.
//! `hero.x`, `#bats`, `state == "play"` are evaluated against the cart's globals and then
//! its file-level `local`s (found as upvalues of init/update/draw and the functions they
//! reach). Uses Lua's `debug` library, which is loaded for this only: carts never see it.

use crate::Console;
use mlua::{Function, Lua, Table, Value};

/// Lua side. Receives the real `debug` and `load` before the sandbox removes them.
const INSPECTOR: &str = r#"
local dbg, load, setmetatable, rawget, type, pairs, G = ...
local builtin = {}
for k in pairs(G) do builtin[k] = true end

-- every upvalue reachable from the cart's entry points: name -> value (first found wins)
local function locals()
  local out, seen, queue, qi = {}, {}, {}, 1
  for _, k in ipairs({"update", "draw", "init"}) do
    local f = rawget(G, k)
    if type(f) == "function" then queue[#queue + 1] = f end
  end
  for k, v in pairs(G) do
    if not builtin[k] and type(v) == "function" then queue[#queue + 1] = v end
  end
  while qi <= #queue and qi <= 400 do
    local f = queue[qi]; qi = qi + 1
    if not seen[f] then
      seen[f] = true
      local i = 1
      while true do
        local n, v = dbg.getupvalue(f, i)
        if not n then break end
        if n ~= "_ENV" and out[n] == nil and v ~= nil then out[n] = v end
        if type(v) == "function" then queue[#queue + 1] = v end
        i = i + 1
      end
    end
  end
  return out
end

local env = setmetatable({}, { __index = function(_, k)
  local v = rawget(G, k)
  if v ~= nil then return v end
  return locals()[k]
end })

local function eval(expr)
  local f, err = load("return " .. expr, "=" .. expr, "t", env)
  if not f then return error(err, 0) end
  return f()
end

-- the cart's own state: new globals + file-level locals, minus functions
local function state()
  local out = {}
  for k, v in pairs(G) do
    if not builtin[k] and type(v) ~= "function" then out[k] = v end
  end
  for k, v in pairs(locals()) do
    if out[k] == nil and type(v) ~= "function" then out[k] = v end
  end
  return out
end

-- the third function re-snapshots the builtin names once the console API is installed
return eval, state, function() builtin = {} for k in pairs(G) do builtin[k] = true end end
"#;

pub(crate) struct Inspector {
    eval: Function,
    state: Function,
    pub(crate) mark_builtin: Function,
}

impl Inspector {
    /// Must run before the sandbox strips `load` and `debug` from the globals.
    pub(crate) fn new(lua: &Lua) -> mlua::Result<Inspector> {
        let g = lua.globals();
        let f = lua.load(INSPECTOR).set_name("=inspector").into_function()?;
        let (eval, state, mark): (Function, Function, Function) =
            f.call((g.get::<Value>("debug")?, g.get::<Value>("load")?, g.get::<Value>("setmetatable")?, g.get::<Value>("rawget")?, g.get::<Value>("type")?, g.get::<Value>("pairs")?, g.clone()))?;
        g.set("debug", Value::Nil)?;
        Ok(Inspector { eval, state, mark_builtin: mark })
    }
}

/// Compact JSON for a Lua value; tables to `depth` levels, 64 entries each.
pub fn to_json(v: &Value, depth: usize) -> String {
    match v {
        Value::Nil => "null".into(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) if n.is_finite() => {
            let r = (n * 1000.0).round() / 1000.0;
            if r == r.trunc() && r.abs() < 1e15 { format!("{}", r as i64) } else { format!("{r}") }
        }
        Value::Number(_) => "null".into(),
        Value::String(s) => format!("{:?}", s.to_string_lossy()),
        Value::Table(t) => table_json(t, depth),
        Value::Function(_) => "\"<fn>\"".into(),
        _ => format!("\"<{}>\"", v.type_name()),
    }
}

fn table_json(t: &Table, depth: usize) -> String {
    if depth == 0 {
        return "\"{…}\"".into();
    }
    let n = t.raw_len();
    let mut pairs: Vec<(Value, Value)> = t.pairs::<Value, Value>().filter_map(|p| p.ok()).collect();
    let is_array = n > 0 && pairs.len() == n;
    if is_array {
        let items: Vec<String> = (1..=n.min(64)).map(|i| to_json(&t.raw_get::<Value>(i).unwrap_or(Value::Nil), depth - 1)).collect();
        let more = if n > 64 { format!(",\"…+{}\"", n - 64) } else { String::new() };
        return format!("[{}{more}]", items.join(","));
    }
    // stable order: sort by key text
    pairs.sort_by_key(|(k, _)| key_text(k));
    let total = pairs.len();
    let items: Vec<String> = pairs
        .iter()
        .filter(|(_, v)| !matches!(v, Value::Function(_)))
        .take(64)
        .map(|(k, v)| format!("{:?}:{}", key_text(k), to_json(v, depth - 1)))
        .collect();
    let more = if total > 64 { format!(",\"…\":\"+{}\"", total - 64) } else { String::new() };
    format!("{{{}{more}}}", items.join(","))
}

fn key_text(k: &Value) -> String {
    match k {
        Value::String(s) => s.to_string_lossy(),
        other => to_json(other, 0),
    }
}

impl Console {
    fn inspector(&self) -> Result<&Inspector, String> {
        self.inspect.as_ref().ok_or_else(|| "inspector unavailable".to_string())
    }

    /// Evaluate a Lua expression against the cart (globals, then file-level locals).
    pub fn eval(&self, expr: &str) -> Result<Value, String> {
        self.ticks.set(0);
        self.inspector()?.eval.call::<Value>(expr).map_err(|e| crate::clean(&e))
    }

    pub fn eval_json(&self, expr: &str, depth: usize) -> Result<String, String> {
        self.eval(expr).map(|v| to_json(&v, depth))
    }

    pub fn eval_bool(&self, expr: &str) -> Result<bool, String> {
        self.eval(expr).map(|v| !matches!(v, Value::Nil | Value::Boolean(false)))
    }

    /// The world box of the object an expression names (same rules as `hitbox()`), or
    /// None when it evaluates to nil/false (e.g. a key that was picked up).
    pub fn eval_box(&self, expr: &str) -> Result<Option<(i32, i32, i32, i32)>, String> {
        match self.eval(expr)? {
            Value::Nil | Value::Boolean(false) => Ok(None),
            Value::Table(t) => {
                let s = self.st.borrow();
                let has_size = t.contains_key("hit").unwrap_or(false) || t.contains_key("spr").unwrap_or(false) || t.contains_key("w").unwrap_or(false);
                if has_size {
                    crate::world::hitbox(&s.assets, &t).map(Some).map_err(|e| crate::clean(&e))
                } else {
                    let x: f64 = t.get::<Option<f64>>("x").ok().flatten().or(t.get::<Option<f64>>(1).ok().flatten()).ok_or_else(|| format!("`{expr}` has no x"))?;
                    let y: f64 = t.get::<Option<f64>>("y").ok().flatten().or(t.get::<Option<f64>>(2).ok().flatten()).ok_or_else(|| format!("`{expr}` has no y"))?;
                    Ok(Some((x.floor() as i32, y.floor() as i32, 1, 1)))
                }
            }
            v => Err(format!("`{expr}` is a {}, not an object with x, y", v.type_name())),
        }
    }

    /// Boxes of every object in a list (or of one object). Skips `alive == false` and
    /// `dead == true` entries, and anything without a position.
    pub fn eval_boxes(&self, expr: &str) -> Result<Vec<(i32, i32, i32, i32)>, String> {
        let v = self.eval(expr)?;
        let Value::Table(t) = v else { return Ok(vec![]) };
        let s = self.st.borrow();
        let one = |o: &Table| -> Option<(i32, i32, i32, i32)> {
            if o.get::<Option<bool>>("alive").ok().flatten() == Some(false) || o.get::<Option<bool>>("dead").ok().flatten() == Some(true) {
                return None;
            }
            if o.contains_key("hit").unwrap_or(false) || o.contains_key("spr").unwrap_or(false) || o.contains_key("w").unwrap_or(false) {
                return crate::world::hitbox(&s.assets, o).ok();
            }
            let x = o.get::<Option<f64>>("x").ok().flatten()?;
            let y = o.get::<Option<f64>>("y").ok().flatten()?;
            Some((x.floor() as i32, y.floor() as i32, 16, 16))
        };
        if t.contains_key("x").unwrap_or(false) {
            return Ok(one(&t).into_iter().collect());
        }
        Ok(t.sequence_values::<Value>().filter_map(|v| match v {
            Ok(Value::Table(o)) => one(&o),
            _ => None,
        }).collect())
    }

    /// The cart's own state (new globals + file-level locals, no functions) as JSON.
    pub fn dump(&self, depth: usize) -> Result<String, String> {
        self.ticks.set(0);
        let t: Value = self.inspector()?.state.call(()).map_err(|e| crate::clean(&e))?;
        Ok(to_json(&t, depth))
    }
}

#[cfg(test)]
mod tests {
    use crate::Console;

    const A: &str = "palette p\n . clear\n k #000000\nsprite s 8x8 pal=p hit=1,2,6,5\nkkkkkkkk\nkkkkkkkk\nkkkkkkkk\nkkkkkkkk\nkkkkkkkk\nkkkkkkkk\nkkkkkkkk\nkkkkkkkk\n";

    #[test]
    fn carts_cannot_reach_debug() {
        // `load` is the save-game call, not Lua's: it never compiles code
        let mut c = Console::new(A, "function update() log(type(debug), type(load('return 1')), type(loadstring)) end", 1).unwrap();
        c.step(0);
        assert_eq!(c.take_logs(), ["nil nil nil"]);
    }

    #[test]
    fn eval_sees_globals_and_file_locals() {
        let lua = r#"
            local hero = {x = 10.5, y = 20, spr = "s"}
            local bats = {}
            score = 3
            local function helper() return bats end
            function update() hero.x = hero.x + 1; helper() end"#;
        let mut c = Console::new(A, lua, 1).unwrap();
        c.step(0);
        assert_eq!(c.eval_json("hero.x", 1).unwrap(), "11.5");
        assert_eq!(c.eval_json("#bats + score", 1).unwrap(), "3");
        assert_eq!(c.eval_box("hero").unwrap(), Some((12, 22, 6, 5)));
        assert_eq!(c.eval_box("nothing_here").unwrap(), None);
        assert!(c.eval_bool("score == 3").unwrap());
        assert_eq!(c.dump(2).unwrap(), r#"{"bats":{},"hero":{"spr":"s","x":11.5,"y":20},"score":3}"#);
        assert!(c.eval("hero..").is_err());
    }
}
