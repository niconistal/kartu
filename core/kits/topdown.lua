-- Kartu top-down kit. A whole top-down adventure from one config table:
--
--   local td = kit("topdown")
--   td.setup { title = "CRYPT", map = "crypt", hero = { spr = "knight" }, ... }
--   function update() td.update() end
--   function draw() td.draw() end
--
-- Everything lives in the global `world` (hero, enemies, pickups, npcs, room, state,
-- items), so bots, test tools and your own code can read it. Things are placed with
-- map markers: `legend @ @hero:floor  b @bat:floor  k @key:floor`, and each marker kind
-- must be a key of cfg.enemies, cfg.pickups or cfg.npcs (or "hero").
--
-- cfg fields (all optional except map/rooms and hero):
--   map = "name"                 one map (scrolls if bigger than the screen)
--   rooms = {{"a","b"},{"c","d"}}  a grid of maps; walking off an edge enters the neighbour
--   names = { a = "KITCHEN" }    room names for the HUD (default: map name in capitals)
--   hero = { spr=, hit={x,y,w,h}, walk=, up=, down=, walk_up=, walk_down=, left=, walk_left=,
--            speed=1.5, hp=3, diag=true }
--            spr/walk face right (flipped for left unless left= is given); up/down optional
--            hit: default = the sprite's hit=, else a lower-body box (fits 1-tile gaps)
--   attack = { btn="a", spr=, spr_v=, reach=14, time=12, damage=1 }   (no attack if absent)
--   enemies = { bat = { spr=, hit=, hp=1, speed=1, move="still|bounce_x|bounce_y|wander|chase",
--                       range=80, damage=1, flying=false, drop="heart" } }
--   pickups = { key = { spr=, item="key" (default: the kind), heal=0, score=0,
--                       goal=false, say=, toast= } }
--   npcs = { owl = { spr=, name="OWL", say = {"line", "line"} or function(world) ... end } }
--   doors = { door = { needs="key", consume=true, opens="dooropen", locked="Locked." } }
--   win = "flag:exit" | "collect" (every goal pickup) | "collect:cookie" | function(world)
--   title=, intro=, win_text="YOU WIN!", lose_text="GAME OVER"
--   hud = { hearts = {"heart", "heart0"}, show = {"key", "cookie"} } or false
--   sounds = { pickup="coin", hurt="hurt", ... } override the default sfx per event, or
--            sounds = false for silence. Events and defaults: start=start pickup=pickup
--            goal=coin key=key heal=heal hurt=hurt swing=swing hit=hit defeat=explosion
--            door=door locked=locked talk=talk win=win lose=lose (a pickup's own sfx= wins)
--   music = "song"  or { title=, play=, win=, lose=, rooms = { cellar = "cave" } }
--            songs from assets.cw or built in (adventure cave village boss victory)
--   ui = { ink="k", paper="w", accent="y" }  (default: darkest / lightest of the 1st palette)
--   on = { start=fn(w), hurt=fn(w, n), defeat=fn(w, e), pickup=fn(w, p), room=fn(w, name),
--          update=fn(w), draw=fn(w) (world space, after actors), hud=fn(w) }
--
-- Logs (what bots and test tools read): state <s>, room <name>, got <kind>, hurt hp=<n>,
-- defeated <kind>, door open, locked, talk <npc>, WIN, LOSE.

local td = {}
local cfg, ui
local held_guard = 0

local function log_(...) log(...) end

-- ---------------------------------------------------------------- sound
local SOUNDS = { start = "start", pickup = "pickup", goal = "coin", key = "key", heal = "heal", hurt = "hurt",
  swing = "swing", hit = "hit", defeat = "explosion", door = "door", locked = "locked", talk = "talk",
  win = "win", lose = "lose" }
local VOL = { defeat = 0.45, swing = 0.8, talk = 0.7 }

local function snd(ev, name)
  if cfg.sounds == false then return end
  local n = name
  if n == nil and cfg.sounds then n = cfg.sounds[ev] end
  if n == nil then n = SOUNDS[ev] end
  if n then sfx(n, { vol = VOL[ev] or 1 }) end
end

-- the song for the current state/room (music() keeps playing if it's the same song)
local function tune()
  local m = cfg.music
  if not m then return end
  if type(m) == "string" then m = { play = m } end
  local s = world and world.state or "play"
  local song
  if s == "title" then song = m.title or m.play
  elseif s == "play" or s == "pause" then song = (m.rooms and m.rooms[world.room]) or m.play
  elseif s == "win" then song = m.win
  elseif s == "over" then song = m.lose end
  if song then music(song) elseif s == "win" or s == "over" then music(nil, { fade = 0.3 }) end
end

-- ---------------------------------------------------------------- helpers
local function lum(hex)
  local r, g, b = tonumber(hex:sub(2, 3), 16), tonumber(hex:sub(4, 5), 16), tonumber(hex:sub(6, 7), 16)
  return r * 3 + g * 6 + b
end

-- UI colours: cfg.ui, else the darkest and lightest entries of the first palette
-- (bare chars like "k" are colours of the first palette).
local function pick_ui()
  local u = cfg.ui or {}
  if not u.ink or not u.paper then
    local lo, hi
    for _, e in ipairs(palette()) do
      if not lo or lum(e.hex) < lum(lo.hex) then lo = e end
      if not hi or lum(e.hex) > lum(hi.hex) then hi = e end
    end
    u.ink = u.ink or lo.ch
    u.paper = u.paper or hi.ch
  end
  u.accent = u.accent or u.paper
  return u
end

local function center(o)
  local x, y, w, h = hitbox(o)
  return x + w / 2, y + h / 2
end

local function sign(v) return v > 0 and 1 or (v < 0 and -1 or 0) end

local function dir_of(dx, dy)
  if math.abs(dx) >= math.abs(dy) then return dx < 0 and "left" or "right" end
  return dy < 0 and "up" or "down"
end

-- Default boxes when the sprite has no hit=: the hero collides with its lower body (fits
-- 1-tile gaps and reads right when walking "behind" things); enemies get a forgiving inset.
local function default_hit(name, kind)
  local w, h = sprsize(name)
  local _, _, hw, hh = hitbox({ x = 0, y = 0, spr = name })
  if hw ~= w or hh ~= h then return nil end   -- the art declares its own hit=
  if kind == "hero" then
    local bw, bh = math.max(4, w - 6), math.max(4, math.min(h - 4, h * 5 // 8))
    return { (w - bw) // 2, h - bh, bw, bh }
  end
  local i = math.min(3, w // 5)
  return { i, i, w - 2 * i, h - 2 * i }
end

local DIRV = { left = { -1, 0 }, right = { 1, 0 }, up = { 0, -1 }, down = { 0, 1 } }

-- ---------------------------------------------------------------- panels & text
-- A framed panel: filled with ink, 1 px paper outline. Screen space.
function td.box(x, y, w, h, opts)
  opts = opts or {}
  rect(x, y, w, h, opts.bg or ui.ink, { screen = true })
  rect(x, y, w, h, opts.edge or ui.paper, { screen = true, line = true })
end

function td.toast(msg, frames)
  world.toast = { text = msg, t = frames or 120 }
end

-- Open a dialogue: a list of lines (or one string). Play pauses until it's read.
function td.say(lines, who)
  if type(lines) == "string" then lines = { lines } end
  world.talk = { lines = lines, i = 1, shown = 0, who = who }
end

-- ---------------------------------------------------------------- world building
local function room_name(map)
  return (cfg.names and cfg.names[map]) or map:upper():gsub("_", " ")
end

local function grid_pos(map)
  if not cfg.rooms then return nil end
  for gy, row in ipairs(cfg.rooms) do
    for gx, m in ipairs(row) do
      if m == map then return gx, gy end
    end
  end
end

local function neighbour(map, dx, dy)
  local gx, gy = grid_pos(map)
  if not gx then return nil end
  local row = cfg.rooms[gy + dy]
  return row and row[gx + dx] or nil
end

local function all_maps()
  if cfg.map then return { cfg.map } end
  local out = {}
  for _, row in ipairs(cfg.rooms) do
    for _, m in ipairs(row) do if m then out[#out + 1] = m end end
  end
  return out
end

local function make_hero(x, y)
  local h = cfg.hero
  return {
    x = x, y = y, spr = h.spr, hit = h.hit or default_hit(h.spr, "hero"), hp = h.hp or 3, maxhp = h.hp or 3, dir = "right",
    inv = 0, kx = 0, ky = 0, moving = false, swing = 0,
  }
end

local function spawn_actor(kind, x, y, key)
  local e = cfg.enemies and cfg.enemies[kind]
  if e then
    local a = { kind = kind, x = x, y = y, spr = e.spr, hit = e.hit or default_hit(e.spr, "enemy"), hp = e.hp or 1, key = key, inv = 0,
                vx = 0, vy = 0, t = irnd(0, 59) }
    if e.move == "bounce_x" then a.vx = e.speed or 1 end
    if e.move == "bounce_y" then a.vy = e.speed or 1 end
    world.enemies[#world.enemies + 1] = a
    return a
  end
  local p = cfg.pickups and cfg.pickups[kind]
  if p then
    local a = { kind = kind, x = x, y = y, spr = p.spr, key = key }
    world.pickups[#world.pickups + 1] = a
    return a
  end
  local n = cfg.npcs and cfg.npcs[kind]
  if n then
    local a = { kind = kind, x = x, y = y, spr = n.spr, key = key }
    world.npcs[#world.npcs + 1] = a
    return a
  end
  if kind ~= "hero" and not world.warned[kind] then
    world.warned[kind] = true
    log_("kit: @" .. kind .. " is not in cfg.enemies, cfg.pickups or cfg.npcs")
  end
end

function td.spawn(kind, x, y) return spawn_actor(kind, x, y, nil) end

local function enter(map, hx, hy)
  local r = world.rooms[map]
  world.room = map
  world.enemies, world.pickups, world.npcs = {}, {}, {}
  bg.map(0, map)
  for key, tile in pairs(r.tiles) do
    local tx, ty = key:match("(%d+),(%d+)")
    bg.tile(0, tonumber(tx), tonumber(ty), tile)
  end
  for _, s in ipairs(spawns(map)) do
    local key = s.tx .. "," .. s.ty
    if s.kind ~= "hero" and not r.gone[key] then spawn_actor(s.kind, s.x, s.y, key) end
  end
  if hx then world.hero.x, world.hero.y = hx, hy end
  world.banner = { text = room_name(map), t = 90 }
  log_("room " .. room_name(map))
  if world.state == "play" then tune() end
  if cfg.on and cfg.on.room then cfg.on.room(world, map) end
end

local function new_game()
  world = {
    state = "play", t = 0, items = {}, score = 0, rooms = {}, warned = {},
    enemies = {}, pickups = {}, npcs = {}, goals = 0,
  }
  local start, sx, sy
  for _, m in ipairs(all_maps()) do
    world.rooms[m] = { gone = {}, tiles = {} }
    for _, s in ipairs(spawns(m)) do
      if s.kind == "hero" and not start then start, sx, sy = m, s.x, s.y end
      local known = s.kind == "hero" or (cfg.enemies and cfg.enemies[s.kind]) or (cfg.pickups and cfg.pickups[s.kind]) or (cfg.npcs and cfg.npcs[s.kind])
      if not known and not world.warned[s.kind] then
        world.warned[s.kind] = true
        log_("kit: marker @" .. s.kind .. " in map " .. m .. " is not in cfg.enemies, cfg.pickups or cfg.npcs")
      end
      local p = cfg.pickups and cfg.pickups[s.kind]
      if p and p.goal then world.goals = world.goals + 1 end
    end
  end
  if not start then error("topdown kit: no @hero marker in any map (legend @ @hero:floor)") end
  world.hero = make_hero(sx, sy)
  enter(start)
  if cfg.on and cfg.on.start then cfg.on.start(world) end
end

local function set_state(s)
  world.state = s
  world.state_t = 0
  held_guard = 20
  log_("state " .. s)
  tune()
  local m = type(cfg.music) == "table" and cfg.music or {}
  if s == "win" and not m.win then snd("win") end
  if s == "over" and not m.lose then snd("lose") end
end

-- ---------------------------------------------------------------- setup
function td.setup(c)
  cfg = c
  assert(cfg.map or cfg.rooms, "topdown kit: give cfg.map or cfg.rooms")
  assert(cfg.hero and cfg.hero.spr, "topdown kit: give cfg.hero = { spr = \"...\" }")
  ui = pick_ui()
  new_game()
  -- sound names are checked up front: a typo would otherwise only fail when the event happens
  for ev, n in pairs(cfg.sounds or {}) do
    if n and not has(n, "sfx") then error("topdown kit: sounds." .. ev .. " = \"" .. n .. "\": no sfx called that") end
  end
  for _, pc in pairs(cfg.pickups or {}) do
    if pc.sfx and not has(pc.sfx, "sfx") then error("topdown kit: pickup sfx \"" .. pc.sfx .. "\": no sfx called that") end
  end
  local m = cfg.music
  for k, n in pairs(type(m) == "table" and m or { play = m }) do
    if type(n) == "string" and not has(n, "song") then error("topdown kit: music." .. k .. " = \"" .. n .. "\": no song called that") end
    if type(n) == "table" then
      for room, s in pairs(n) do
        if not has(s, "song") then error("topdown kit: music.rooms." .. room .. " = \"" .. s .. "\": no song called that") end
      end
    end
  end
  world.state = cfg.title and "title" or "play"
  world.state_t = 0
  log_("state " .. world.state)
  tune()
end

-- ---------------------------------------------------------------- hero
local function hurt(n, fromx, fromy)
  local h = world.hero
  if h.inv > 0 or world.state ~= "play" then return end
  h.hp = h.hp - n
  h.inv = 60
  local cx, cy = center(h)
  local ax, ay = cx - (fromx or cx), cy - (fromy or cy)
  local d = math.sqrt(ax * ax + ay * ay)
  if d > 0 then h.kx, h.ky = ax / d * 3, ay / d * 3 end
  log_("hurt hp=" .. h.hp)
  if h.hp > 0 then snd("hurt") end
  if cfg.on and cfg.on.hurt then cfg.on.hurt(world, n) end
  if h.hp <= 0 then
    set_state("over")
    log_("LOSE")
  end
end
td.hurt = hurt

function td.win()
  if world.state == "win" then return end
  set_state("win")
  log_("WIN")
end

function td.lose()
  set_state("over")
  log_("LOSE")
end

local function swing_box()
  local h, a = world.hero, cfg.attack
  local x, y, w, hh = hitbox(h)
  local reach = a.reach or 14
  local d = DIRV[h.dir]
  if d[1] ~= 0 then
    return { x = d[1] > 0 and x + w or x - reach, y = y - 2, w = reach, h = hh + 4 }
  end
  return { x = x - 2, y = d[2] > 0 and y + hh or y - reach, w = w + 4, h = reach }
end

local function update_hero()
  local h, hc = world.hero, cfg.hero
  local dx, dy = 0, 0
  if btn("left") then dx = dx - 1 end
  if btn("right") then dx = dx + 1 end
  if btn("up") then dy = dy - 1 end
  if btn("down") then dy = dy + 1 end
  if dx ~= 0 and dy ~= 0 then
    if hc.diag == false then dy = 0 else dx, dy = dx * 0.7071, dy * 0.7071 end
  end
  -- facing follows the newest press, else the axis being held
  for _, b in ipairs({ "left", "right", "up", "down" }) do
    if btnp(b) then h.dir = b end
  end
  if (dx ~= 0 or dy ~= 0) and not btn(h.dir) then h.dir = dir_of(dx, dy) end
  local sp = hc.speed or 1.5
  h.moving = dx ~= 0 or dy ~= 0
  move(h, dx * sp + h.kx, dy * sp + h.ky)
  h.kx, h.ky = h.kx * 0.7, h.ky * 0.7
  if math.abs(h.kx) < 0.2 then h.kx = 0 end
  if math.abs(h.ky) < 0.2 then h.ky = 0 end
  if h.inv > 0 then h.inv = h.inv - 1 end

  -- talk beats attack when someone is in reach
  local near
  for _, n in ipairs(world.npcs) do
    local x, y, w, hh = hitbox(n)
    if overlap(h, { x = x - 6, y = y - 6, w = w + 12, h = hh + 12 }) then near = n end
  end
  world.near = near
  local abtn = cfg.attack and cfg.attack.btn or "a"
  if near and btnp(abtn) then
    local say = cfg.npcs[near.kind].say
    if type(say) == "function" then say = say(world, near) end
    log_("talk " .. near.kind)
    snd("talk")
    td.say(say or "...", cfg.npcs[near.kind].name)
    return
  end
  if cfg.attack then
    if h.swing > 0 then h.swing = h.swing - 1
    elseif btnp(abtn) then h.swing = cfg.attack.time or 12; snd("swing") end
  end
end

-- ---------------------------------------------------------------- enemies
local function update_enemy(e)
  local ec = cfg.enemies[e.kind]
  local sp = ec.speed or 1
  local h = world.hero
  e.t = e.t + 1
  local mode = ec.move or "still"
  local dx, dy = 0, 0
  if mode == "bounce_x" or mode == "bounce_y" then
    dx, dy = e.vx, e.vy
  elseif mode == "wander" or mode == "chase" then
    local hx, hy = center(h)
    local ex, ey = center(e)
    local ax, ay = hx - ex, hy - ey
    local d = math.sqrt(ax * ax + ay * ay)
    if mode == "chase" and d < (ec.range or 80) and d > 0 then
      dx, dy = ax / d * sp, ay / d * sp
    else
      if e.t % 90 == 0 or (e.vx == 0 and e.vy == 0 and e.t % 30 == 0) then
        local r = irnd(0, 4)
        local v = ({ { sp, 0 }, { -sp, 0 }, { 0, sp }, { 0, -sp }, { 0, 0 } })[r + 1]
        e.vx, e.vy = v[1] * 0.6, v[2] * 0.6
      end
      dx, dy = e.vx, e.vy
    end
  end
  dx, dy = dx + (e.kx or 0), dy + (e.ky or 0)
  if e.kx then
    e.kx, e.ky = e.kx * 0.7, e.ky * 0.7
    if math.abs(e.kx) + math.abs(e.ky) < 0.3 then e.kx, e.ky = nil, nil end
  end
  if ec.flying then
    e.x, e.y = e.x + dx, e.y + dy
  else
    local bx, by = move(e, dx, dy)
    if bx then e.vx = -e.vx end
    if by then e.vy = -e.vy end
  end
  if e.inv > 0 then e.inv = e.inv - 1 end
  if overlap(h, e) then
    local x, y = center(e)
    hurt(ec.damage or 1, x, y)
  end
end

local function defeat(e, i)
  local ec = cfg.enemies[e.kind]
  table.remove(world.enemies, i)
  if e.key then world.rooms[world.room].gone[e.key] = true end
  log_("defeated " .. e.kind)
  if ec.drop then td.spawn(ec.drop, e.x, e.y) end
  if cfg.on and cfg.on.defeat then cfg.on.defeat(world, e) end
end

local function update_attack()
  local h = world.hero
  if not cfg.attack or h.swing <= 0 then return end
  local box = swing_box()
  for i = #world.enemies, 1, -1 do
    local e = world.enemies[i]
    if e.inv == 0 and overlap(box, e) then
      e.hp = e.hp - (cfg.attack.damage or 1)
      e.inv = 20
      local d = DIRV[h.dir]
      e.kx, e.ky = d[1] * 4, d[2] * 4
      if e.hp <= 0 then defeat(e, i); snd("defeat") else snd("hit") end
    end
  end
end

-- ---------------------------------------------------------------- pickups, doors, rooms, win
local function goals_left(kind)
  local n = 0
  for m, r in pairs(world.rooms) do
    for _, s in ipairs(spawns(m)) do
      local p = cfg.pickups and cfg.pickups[s.kind]
      if p and (kind and s.kind == kind or (not kind and p.goal)) and not r.gone[s.tx .. "," .. s.ty] then n = n + 1 end
    end
  end
  return n
end
td.goals_left = goals_left

local function update_pickups()
  local h = world.hero
  for i = #world.pickups, 1, -1 do
    local p = world.pickups[i]
    if overlap(h, p) then
      local pc = cfg.pickups[p.kind]
      table.remove(world.pickups, i)
      if p.key then world.rooms[world.room].gone[p.key] = true end
      local item = pc.item or p.kind
      world.items[item] = (world.items[item] or 0) + 1
      world.score = world.score + (pc.score or 0)
      if pc.heal then h.hp = math.min(h.maxhp, h.hp + pc.heal) end
      log_("got " .. p.kind)
      snd(pc.heal and pc.heal > 0 and "heal" or item == "key" and "key" or pc.goal and "goal" or "pickup", pc.sfx)
      if pc.toast then td.toast(pc.toast) end
      if pc.say then td.say(pc.say) end
      if cfg.on and cfg.on.pickup then cfg.on.pickup(world, p) end
    end
  end
end

local function update_doors()
  if not cfg.doors then return end
  local h = world.hero
  local d = DIRV[h.dir]
  local x, y, w, hh = hitbox(h)
  local probe = { x = x + d[1] * 2, y = y + d[2] * 2, w = w, h = hh }
  for tile, rule in pairs(cfg.doors) do
    -- find a door tile under the probe box
    local tx0, ty0 = probe.x // 16, probe.y // 16
    local tx1, ty1 = (probe.x + probe.w - 1) // 16, (probe.y + probe.h - 1) // 16
    for ty = ty0, ty1 do
      for tx = tx0, tx1 do
        if bg.get(0, tx, ty) == tile then
          local need = rule.needs
          if not need or (world.items[need] or 0) > 0 then
            if need and rule.consume ~= false then world.items[need] = world.items[need] - 1 end
            local new = rule.opens
            bg.tile(0, tx, ty, new)
            world.rooms[world.room].tiles[tx .. "," .. ty] = new or false
            log_("door open")
            snd("door")
          elseif (world.locked_t or 0) <= 0 then
            td.toast(rule.locked or "Locked.", 90)
            world.locked_t = 90
            log_("locked")
            snd("locked")
          end
        end
      end
    end
  end
  if world.locked_t then world.locked_t = world.locked_t - 1 end
end

local function update_rooms()
  local h = world.hero
  local mw, mh = bg.size(0)
  local cx, cy = center(h)
  local W, H = mw * 16, mh * 16
  local dx = cx < 0 and -1 or (cx >= W and 1 or 0)
  local dy = cy < 0 and -1 or (cy >= H and 1 or 0)
  if dx == 0 and dy == 0 then return end
  local n = neighbour(world.room, dx, dy)
  if n then
    -- keep the hero's offset across the edge: x = -5 in the old room is x = W' - 5 in the new
    local nw, nh = bg.size(0)
    bg.map(0, n)
    nw, nh = bg.size(0)
    local x, y = h.x, h.y
    if dx > 0 then x = x - W elseif dx < 0 then x = x + nw * 16 end
    if dy > 0 then y = y - H elseif dy < 0 then y = y + nh * 16 end
    enter(n, x, y)
  else
    -- no room there: keep the hero on the map
    local x, y, w, hh = hitbox(h)
    local ox, oy = x - h.x, y - h.y
    h.x = math.max(-ox, math.min(W - w - ox, h.x))
    h.y = math.max(-oy, math.min(H - hh - oy, h.y))
  end
end

local function check_win()
  local w = cfg.win
  if not w then return end
  if type(w) == "function" then
    if w(world) then td.win() end
  elseif w:sub(1, 5) == "flag:" then
    if touching(world.hero, w:sub(6)) then td.win() end
  elseif w == "collect" then
    if world.goals > 0 and goals_left() == 0 then td.win() end
  elseif w:sub(1, 8) == "collect:" then
    if goals_left(w:sub(9)) == 0 then td.win() end
  end
end

-- ---------------------------------------------------------------- update
local function update_talk()
  local tk = world.talk
  local line = tk.lines[tk.i]
  tk.shown = math.min(#line, tk.shown + 1)
  if btnp("a") or btnp("b") then
    if tk.shown < #line then tk.shown = #line
    elseif tk.i < #tk.lines then tk.i, tk.shown = tk.i + 1, 0
    else world.talk = nil end
  end
end

function td.update()
  world.state_t = (world.state_t or 0) + 1
  if held_guard > 0 then held_guard = held_guard - 1 end
  local s = world.state
  if world.toast then
    world.toast.t = world.toast.t - 1
    if world.toast.t <= 0 then world.toast = nil end
  end
  if world.banner then
    world.banner.t = world.banner.t - 1
    if world.banner.t <= 0 then world.banner = nil end
  end
  if s == "title" then
    if held_guard == 0 and (btnp("a") or btnp("start")) then
      new_game()
      set_state("play")
      snd("start")
    end
    return
  elseif s == "win" or s == "over" then
    if held_guard == 0 and world.state_t > 45 and (btnp("a") or btnp("start")) then
      new_game()
      set_state(cfg.title and "title" or "play")
    end
    return
  elseif s == "pause" then
    if btnp("start") then world.state = "play" end
    return
  end
  if world.talk then update_talk() return end
  if btnp("start") then world.state = "pause" return end
  world.t = world.t + 1
  update_hero()
  if world.talk then return end
  update_attack()
  for i = #world.enemies, 1, -1 do
    local e = world.enemies[i]
    if e then update_enemy(e) end
  end
  update_pickups()
  update_doors()
  update_rooms()
  if cfg.on and cfg.on.update then cfg.on.update(world) end
  if world.state == "play" then check_win() end
end

-- ---------------------------------------------------------------- draw
local function hero_sprite()
  local h, hc = world.hero, cfg.hero
  local d = h.dir
  local fx = false
  local name
  if d == "up" and hc.up then name = h.moving and hc.walk_up or hc.up
  elseif d == "down" and hc.down then name = h.moving and hc.walk_down or hc.down
  elseif d == "left" and hc.left then name = h.moving and hc.walk_left or hc.left
  else
    name = h.moving and hc.walk or hc.spr
    fx = d == "left" or (d ~= "right" and h.lastfx)
  end
  if d == "left" then h.lastfx = true elseif d == "right" then h.lastfx = false end
  return name or hc.spr, fx
end

local function bottom(a)
  local _, h = sprsize(a.spr)
  return a.y + h
end

function td.draw()
  local h = world.hero
  if cfg.backdrop then backdrop(cfg.backdrop) end
  local cx, cy = center(h)
  camera(cx - 160, cy - 128, { clamp = 0 })

  -- actors, sorted by their feet so nearer things cover farther ones
  local list = {}
  for _, p in ipairs(world.pickups) do list[#list + 1] = { a = p, kind = "pickup" } end
  for _, n in ipairs(world.npcs) do list[#list + 1] = { a = n, kind = "npc" } end
  for _, e in ipairs(world.enemies) do list[#list + 1] = { a = e, kind = "enemy" } end
  list[#list + 1] = { a = h, kind = "hero" }
  for i, it in ipairs(list) do it.y, it.i = bottom(it.a), i end
  table.sort(list, function(p, q) if p.y ~= q.y then return p.y < q.y end return p.i < q.i end)
  for _, it in ipairs(list) do
    local a = it.a
    if it.kind == "hero" then
      if h.inv % 8 < 4 then
        local name, fx = hero_sprite()
        spr(name, h.x, h.y, { fx = fx })
      end
    elseif it.kind == "pickup" then
      spr(a.spr, a.x, a.y + sin((frame() + a.x) / 60) * 1.5)
    elseif it.kind == "enemy" then
      if a.inv % 4 < 2 then spr(a.spr, a.x, a.y, { fx = (a.vx or 0) < 0 }) end
    else
      spr(a.spr, a.x, a.y, { fx = center(a) > cx })
    end
  end
  -- attack
  if cfg.attack and h.swing > 0 then
    local b = swing_box()
    local a = cfg.attack
    if DIRV[h.dir][2] ~= 0 and a.spr_v then spr(a.spr_v, b.x + 2, b.y, { fy = h.dir == "up" })
    elseif a.spr then spr(a.spr, b.x, b.y + 2, { fx = h.dir == "left" }) end
  end
  if world.near and not world.talk then
    local nx, ny = hitbox(world.near)
    text("A", nx - camera() + 2, ny - select(2, camera()) - 10, { col = ui.accent, bg = ui.ink, pad = 1 })
  end
  if cfg.on and cfg.on.draw then cfg.on.draw(world) end

  -- HUD
  if cfg.hud ~= false then
    local hud = cfg.hud or {}
    rect(0, 0, 320, 14, ui.ink, { screen = true })
    local x = 4
    for i = 1, h.maxhp do
      local full = i <= h.hp
      if hud.hearts then
        spr(full and hud.hearts[1] or (hud.hearts[2] or hud.hearts[1]), x, 3, { screen = true })
        x = x + (sprsize(hud.hearts[1])) + 2
      else
        rect(x, 4, 7, 7, full and ui.paper or ui.ink, { screen = true })
        rect(x, 4, 7, 7, ui.paper, { screen = true, line = true })
        x = x + 9
      end
    end
    x = x + 8
    for _, kind in ipairs(hud.show or {}) do
      local pc = cfg.pickups and cfg.pickups[kind]
      local n = world.items[(pc and pc.item) or kind] or 0
      if pc and pc.goal then n = n .. "/" .. (n + goals_left(kind)) end
      if pc and pc.spr then
        local _, sh = sprsize(pc.spr)
        spr(pc.spr, x, math.max(0, 7 - sh // 2), { screen = true })
        x = x + (sprsize(pc.spr)) + 2
      end
      local w = text(tostring(n), x, 3, ui.paper)
      x = x + w + 10
    end
    text(room_name(world.room), 316, 3, { col = ui.paper, align = "right" })
    if cfg.on and cfg.on.hud then cfg.on.hud(world) end
  end

  if world.banner and world.state == "play" and cfg.rooms then
    text(world.banner.text, 160, 30, { col = ui.paper, align = "center", bg = ui.ink, pad = 4 })
  end
  if world.toast then
    text(world.toast.text, 160, 200, { col = ui.accent, align = "center", bg = ui.ink, pad = 4 })
  end
  if world.talk then
    local tk = world.talk
    td.box(8, 170, 304, 62)
    local y = 178
    if tk.who then text(tk.who, 16, y, ui.accent) y = y + 12 end
    text(tk.lines[tk.i]:sub(1, tk.shown), 16, y, { col = ui.paper, wrap = 288 })
    if tk.shown >= #tk.lines[tk.i] and frame() % 40 < 26 then text(">", 300, 222, ui.accent) end
  end

  local s = world.state
  if s == "title" then
    local tw, th = text(cfg.title, 160, 70, { col = ui.accent, scale = 3, align = "center", shadow = ui.ink })
    if cfg.intro then text(cfg.intro, 160, 120, { col = ui.paper, align = "center", bg = ui.ink, pad = 6, wrap = 280 }) end
    if frame() % 60 < 40 then text("press A", 160, 180, { col = ui.accent, align = "center", bg = ui.ink, pad = 3 }) end
  elseif s == "win" then
    text(cfg.win_text or "YOU WIN!", 160, 100, { col = ui.accent, scale = 2, align = "center", bg = ui.ink, pad = 8 })
    if world.state_t > 45 then text("press A", 160, 140, { col = ui.paper, align = "center" }) end
  elseif s == "over" then
    text(cfg.lose_text or "GAME OVER", 160, 100, { col = ui.accent, scale = 2, align = "center", bg = ui.ink, pad = 8 })
    if world.state_t > 45 then text("press A", 160, 140, { col = ui.paper, align = "center" }) end
  elseif s == "pause" then
    text("PAUSED", 160, 110, { col = ui.paper, scale = 2, align = "center", bg = ui.ink, pad = 8 })
  end
end

return td
