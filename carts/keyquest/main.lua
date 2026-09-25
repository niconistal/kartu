-- title: Key Quest
-- KEY QUEST: a little knight must find the key and unlock the exit door.
-- Arrows move, A swings the sword. 3 rooms, bats, 3 hearts.

TRACE = false
local W, H = 320, 240

-- Room layouts live here (not in assets.cw) because Lua cannot read map cells back.
-- # wall  T torch-wall  o block  E exit door  H hud strip  . floor
-- P player start  B bat  K key   (all three are floor underneath)
local ROOMS = {
  { name = "THE HALL", rows = {
    "HHHHHHHHHHHHHHHHHHHH",
    "####T####E####T#####",
    "#..................#",
    "#..................#",
    "#...o..........o...#",
    "#..................#",
    "#..................#",
    "#.....P.............",
    "#...................",
    "#..................#",
    "#...o..........o...#",
    "#..........B.......#",
    "#..................#",
    "#..................#",
    "####################" } },
  { name = "THE GALLERY", rows = {
    "HHHHHHHHHHHHHHHHHHHH",
    "#####T######T#######",
    "#.....#............#",
    "#.....#............#",
    "#.....#....B.......#",
    "#.....#......####..#",
    "#..........#.......#",
    "...........#........",
    "...........#........",
    "#..B....o..#.......#",
    "#......#...#...o...#",
    "#......#...........#",
    "#......#...........#",
    "#......#...........#",
    "####################" } },
  { name = "THE VAULT", rows = {
    "HHHHHHHHHHHHHHHHHHHH",
    "######T#######T#####",
    "#.............#....#",
    "#......B......#.K..#",
    "#.............#....#",
    "#....#####....#....#",
    "#....#...........B.#",
    ".....#.....o.......#",
    ".....#.............#",
    "#....#....#######..#",
    "#.........#........#",
    "#.....B...#...o....#",
    "#.........#........#",
    "#.........#........#",
    "####################" } },
}

local TILE  = { ["#"]="wall", T="torch", o="block", E="door", D="dooropen", H="hud" }
local SOLID = { ["#"]=true, T=true, o=true, E=true, H=true }

---------------------------------------------------------------- generic helpers
local function overlap(ax, ay, aw, ah, bx, by, bw, bh)
  return ax < bx + bw and bx < ax + aw and ay < by + bh and by < ay + ah
end

local function ctext(s, y, col) text(s, 160 - #s * 4, y, col) end

local function banner(y, rows)       -- dark strip behind text (no rect primitive)
  for r = 0, rows - 1 do
    for x = 0, W - 1, 64 do spr("shade", x, y + r * 16) end
  end
end

---------------------------------------------------------------- state
local state, stateT
local rooms, room, cells
local p, haskey, msg, msgT, fx

local function cell(tx, ty)
  if ty < 0 or ty > 14 then return "#" end
  if tx < 0 or tx > 19 then return "." end        -- off-screen sideways = doorway
  return cells[ty + 1][tx + 1]
end

local function solid_at(px, py) return SOLID[cell(px // 16, py // 16)] end

local function box_solid(x, y, w, h)
  x, y = math.floor(x), math.floor(y)
  return solid_at(x, y) or solid_at(x + w - 1, y) or solid_at(x, y + h - 1) or solid_at(x + w - 1, y + h - 1)
end

-- move an object with hitbox (hx,hy,hw,hh), sliding along walls, 1px sub-steps
local function move(o, dx, dy)
  local n = math.ceil(math.max(math.abs(dx), math.abs(dy)))
  if n == 0 then return end
  local sx, sy = dx / n, dy / n
  for _ = 1, n do
    if sx ~= 0 then
      if box_solid(o.x + sx + o.hx, o.y + o.hy, o.hw, o.hh) then sx = 0 else o.x = o.x + sx end
    end
    if sy ~= 0 then
      if box_solid(o.x + o.hx, o.y + sy + o.hy, o.hw, o.hh) then sy = 0 else o.y = o.y + sy end
    end
  end
end

-- is any tile with char ch within `pad` px of the object's hitbox?
local function touching(o, ch, pad)
  local x0, y0 = math.floor(o.x + o.hx - pad), math.floor(o.y + o.hy - pad)
  local x1, y1 = x0 + o.hw - 1 + 2 * pad, y0 + o.hh - 1 + 2 * pad
  for ty = y0 // 16, y1 // 16 do
    for tx = x0 // 16, x1 // 16 do
      if cell(tx, ty) == ch then return tx, ty end
    end
  end
end

local function paint()
  bg.map(0, "blank")
  for ty = 0, 14 do
    for tx = 0, 19 do
      bg.tile(0, tx, ty, TILE[cells[ty + 1][tx + 1]] or "floor")
    end
  end
end

---------------------------------------------------------------- game setup
local function new_game()
  rooms = {}
  for i, def in ipairs(ROOMS) do
    local r = { cells = {}, bats = {}, torches = {} }
    for ty, row in ipairs(def.rows) do
      r.cells[ty] = {}
      for tx = 1, #row do
        local c = row:sub(tx, tx)
        local x, y = (tx - 1) * 16, (ty - 1) * 16
        if c == "B" then
          r.bats[#r.bats + 1] = { x = x, y = y, vx = 0, vy = 0, t = irnd(0, 99), alive = true }
        elseif c == "K" then r.key = { x = x, y = y }
        elseif c == "P" then p = { x = x, y = y, hx = 3, hy = 4, hw = 10, hh = 11,
                                   face = 1, dir = "right", hp = 3, inv = 0, kx = 0, ky = 0, kt = 0, swing = 0, walk = false }
        elseif c == "T" then r.torches[#r.torches + 1] = { x = x + 4, y = y + 1 } end
        if c == "B" or c == "K" or c == "P" then c = "." end
        r.cells[ty][tx] = c
      end
    end
    rooms[i] = r
  end
  haskey, msg, msgT, fx = false, nil, 0, {}
end

local function enter_room(i)
  room = i
  cells = rooms[i].cells
  fx = {}
  paint()
  log("room " .. i)
end

local function set_state(s)
  state, stateT = s, 0
  log("state " .. s)
end

function init()
  new_game()
  enter_room(1)
  set_state("title")
end

---------------------------------------------------------------- update
local function hurt(from)
  p.hp = p.hp - 1
  p.inv = 90
  local dx, dy = p.x - from.x, p.y - from.y
  local d = math.sqrt(dx * dx + dy * dy); if d < 0.01 then dx, dy, d = 1, 0, 1 end
  p.kx, p.ky, p.kt = dx / d * 3, dy / d * 3, 10
  from.vx, from.vy = -dx / d, -dy / d
  log("hurt hp=" .. p.hp)
  if p.hp <= 0 then set_state("lose") end
end

local function update_bats()
  for _, b in ipairs(rooms[room].bats) do
    if b.alive then
      b.t = b.t + 1
      local dx, dy = p.x - b.x, p.y - b.y
      local d = math.sqrt(dx * dx + dy * dy) + 0.01
      if d < 80 then
        b.vx, b.vy = b.vx + dx / d * 0.025, b.vy + dy / d * 0.025
      else
        b.vx, b.vy = b.vx + (rnd() - 0.5) * 0.08, b.vy + (rnd() - 0.5) * 0.08
      end
      local sp = math.sqrt(b.vx * b.vx + b.vy * b.vy)
      if sp > 0.75 then b.vx, b.vy = b.vx / sp * 0.75, b.vy / sp * 0.75 end
      b.x = b.x + b.vx
      b.y = b.y + b.vy + sin(b.t / 45) * 0.6
      if b.x < 16 then b.x, b.vx = 16, math.abs(b.vx) end
      if b.x > 288 then b.x, b.vx = 288, -math.abs(b.vx) end
      if b.y < 32 then b.y, b.vy = 32, math.abs(b.vy) end
      if b.y > 208 then b.y, b.vy = 208, -math.abs(b.vy) end
      if p.inv == 0 and state == "play"
         and overlap(p.x + p.hx, p.y + p.hy, p.hw, p.hh, b.x + 3, b.y + 3, 10, 8) then
        hurt(b)
      end
    end
  end
end

local function sword_box()
  if p.dir == "up" then return p.x + 1, p.y - 12, 14, 16 end
  if p.dir == "down" then return p.x + 1, p.y + 10, 14, 16 end
  if p.dir == "right" then return p.x + 12, p.y + 4, 16, 10 end
  return p.x - 12, p.y + 4, 16, 10
end

local function update_play()
  local dx, dy = 0, 0
  if btn("left") then dx = dx - 1 end
  if btn("right") then dx = dx + 1 end
  if btn("up") then dy = dy - 1 end
  if btn("down") then dy = dy + 1 end
  if dx ~= 0 then p.face = dx end
  if btnp("left") then p.dir = "left" elseif btnp("right") then p.dir = "right"
  elseif btnp("up") then p.dir = "up" elseif btnp("down") then p.dir = "down" end
  if dx == 0 and dy ~= 0 and not (btn("left") or btn("right")) then p.dir = dy < 0 and "up" or "down" end
  if dy == 0 and dx ~= 0 then p.dir = dx < 0 and "left" or "right" end
  if dx ~= 0 and dy ~= 0 then dx, dy = dx * 0.7071, dy * 0.7071 end
  p.walk = dx ~= 0 or dy ~= 0
  local spd = 1.5
  if p.kt > 0 then
    p.kt = p.kt - 1
    move(p, p.kx, p.ky)
  else
    move(p, dx * spd, dy * spd)
  end
  if p.inv > 0 then p.inv = p.inv - 1 end

  -- sword
  if p.swing > 0 then p.swing = p.swing - 1 end
  if btnp("a") and p.swing == 0 then p.swing = 16 end
  if p.swing > 6 then
    local sx, sy, sw, sh = sword_box()
    for _, b in ipairs(rooms[room].bats) do
      if b.alive and overlap(sx, sy, sw, sh, b.x + 2, b.y + 2, 12, 10) then
        b.alive = false
        fx[#fx + 1] = { x = b.x, y = b.y, t = 16 }
        log("bat slain")
      end
    end
  end

  -- doorways at the screen edges
  local cx = p.x + p.hx + p.hw / 2
  if cx >= W and room < #rooms then
    enter_room(room + 1); p.x = 4 - p.hx - p.hw / 2
    p.inv = math.max(p.inv, 60)
  elseif cx < 0 and room > 1 then
    enter_room(room - 1); p.x = W - 4 - p.hx - p.hw / 2
    p.inv = math.max(p.inv, 60)
  end

  -- key
  local k = rooms[room].key
  if k and overlap(p.x + p.hx, p.y + p.hy, p.hw, p.hh, k.x + 1, k.y + 3, 15, 9) then
    rooms[room].key = nil
    haskey = true
    msg, msgT = "YOU FOUND THE KEY!", 120
    log("got key")
  end

  -- exit door
  local tx, ty = touching(p, "E", 2)
  if tx then
    if haskey then
      cells[ty + 1][tx + 1] = "D"
      bg.tile(0, tx, ty, "dooropen")
      log("WIN")
      set_state("win")
    elseif msgT < 60 then
      msg, msgT = "LOCKED! FIND THE KEY.", 90
    end
  end

  update_bats()
  if TRACE and frame() % 10 == 0 then log(string.format("T r%d x%d y%d hp%d", room, math.floor(p.x), math.floor(p.y), p.hp)) end
  if msgT > 0 then msgT = msgT - 1 end
end

function update()
  stateT = stateT + 1
  for i = #fx, 1, -1 do
    fx[i].t = fx[i].t - 1
    if fx[i].t <= 0 then table.remove(fx, i) end
  end
  if state == "title" then
    if btnp("a") or btnp("start") then
      new_game(); enter_room(1); set_state("play")
    end
  elseif state == "play" then
    update_play()
  else
    if stateT > 45 and (btnp("a") or btnp("start")) then
      new_game(); enter_room(1); set_state("title")
    end
  end
end

---------------------------------------------------------------- draw
local function draw_world()
  local r = rooms[room]
  for _, t in ipairs(r.torches) do spr("flame", t.x, t.y) end
  if r.key then spr("key", r.key.x, r.key.y + sin(frame() / 60) * 2) end
  for _, b in ipairs(r.bats) do
    if b.alive then spr("bat_fly", b.x, b.y) end
  end
  for _, f in ipairs(fx) do spr("puff", f.x, f.y) end
  -- knight (blinks while invulnerable)
  if not (p.inv > 0 and (p.inv // 4) % 2 == 0) or state ~= "play" then
    local flip = p.face < 0
    spr(p.walk and state == "play" and "knight_walk" or "knight1", p.x, p.y, { fx = flip })
    if p.swing > 6 then
      if p.dir == "up" then spr("swordv", p.x + 6, p.y - 11)
      elseif p.dir == "down" then spr("swordv", p.x + 6, p.y + 13, { fy = true })
      elseif p.dir == "left" then spr("sword", p.x - 9, p.y + 7, { fx = true })
      else spr("sword", p.x + 11, p.y + 7) end
    end
  end
end

local function draw_hud()
  for i = 1, 3 do spr(i <= p.hp and "heart" or "heart0", 6 + (i - 1) * 12, 4) end
  if haskey then spr("key", 44, 0) end
  local name = ROOMS[room].name
  text(name, W - #name * 8 - 6, 4, 13)
  if msgT > 0 then
    banner(200, 1)
    ctext(msg, 204, 5)
  end
end

function draw()
  backdrop("#1a1c2c")
  if state == "title" then
    draw_world()
    banner(56, 6)
    ctext("K E Y   Q U E S T", 68, 5)
    ctext("The little knight must find", 90, 13)
    ctext("the key and escape the castle.", 100, 13)
    ctext("ARROWS move    A swing sword", 118, 11)
    if (frame() // 30) % 2 == 0 then ctext("PRESS A TO START", 136, 12) end
    return
  end
  draw_world()
  draw_hud()
  if state == "win" then
    banner(88, 4)
    ctext("YOU ESCAPED!", 100, 6)
    ctext("The knight is free. Well done!", 116, 12)
    if stateT > 45 then ctext("PRESS A TO PLAY AGAIN", 136, 13) end
  elseif state == "lose" then
    banner(88, 4)
    ctext("THE KNIGHT HAS FALLEN", 100, 3)
    ctext("The bats were too many...", 116, 12)
    if stateT > 45 then ctext("PRESS A TO TRY AGAIN", 136, 13) end
  end
end
