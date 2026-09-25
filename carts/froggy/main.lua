-- title: Froggy
-- FROGGY: hop across the road and the river to fill all five lily pads.

local HOPCD   = 8      -- frames between hops
local TIME    = 1800   -- frames per life (30 s)
local START_X = 152
local START_Y = 13     -- tile row
local BAYS    = {32, 96, 160, 224, 288}   -- centre x of each lily pad

-- lanes by tile row. objects: xs = start offsets, len = pixel length.
-- x(t) = (x0 + spd*t) % P - len  (P >= 320 + len so they leave fully before wrapping)
local LANES = {
  [2]  = {kind="log",    spd= 0.75, P=400, len=64, xs={0,133,266}},
  [3]  = {kind="turtle", spd=-1.0,  P=384, len=32, xs={0,96,192,288}},
  [4]  = {kind="log",    spd= 0.5,  P=448, len=96, xs={0,224}},
  [5]  = {kind="log",    spd= 1.0,  P=384, len=48, xs={0,128,256}},
  [6]  = {kind="turtle", spd=-0.75, P=384, len=48, xs={0,128,256}},
  [8]  = {kind="truck",  spd=-0.5,  P=400, len=32, xs={0,200}},
  [9]  = {kind="racer",  spd= 1.5,  P=352, len=16, xs={0,176}, pal="carR"},
  [10] = {kind="car",    spd=-1.0,  P=352, len=16, xs={0,110,220}, pal="carP"},
  [11] = {kind="car",    spd= 0.5,  P=352, len=16, xs={0,120,240}, pal="carG"},
  [12] = {kind="car",    spd=-0.75, P=352, len=16, xs={0,110,220}, pal="carY"},
}

local T = 0                 -- world clock (lanes move on it, even on the title)
local state = "title"       -- title | play | dying | win | over
local fx, fy, dir, hopcd, timer
local lives, score, hi, best, homes = 3, 0, 0, 13, {}
local deadt, deadkind, msgt = 0, "car", 0

local function objx(l, i) return (l.xs[i] + l.spd * T) % l.P - l.len end

local function reset_frog()
  fx, fy, dir, hopcd, timer, best = START_X, START_Y, "u", HOPCD, TIME, 13
end

local function new_game()
  lives, score, homes = 3, 0, {}
  reset_frog()
  state = "play"
  log("START T=" .. T)
end

local function die(kind)
  state, deadkind, deadt = "dying", kind, 45
  log("DIE " .. kind .. " T=" .. T .. " row=" .. fy)
end

local function homes_filled()
  local n = 0
  for i = 1, 5 do if homes[i] then n = n + 1 end end
  return n
end

local function update_play()
  timer = timer - 1
  if hopcd > 0 then hopcd = hopcd - 1
  else
    local nx, ny = fx, fy
    if     btnp("up")    then ny, dir = fy - 1, "u"
    elseif btnp("down")  then ny, dir = fy + 1, "d"
    elseif btnp("left")  then nx, dir = fx - 16, "l"
    elseif btnp("right") then nx, dir = fx + 16, "r" end
    if (nx ~= fx or ny ~= fy) and ny <= START_Y and nx >= -8 and nx <= 312 then
      fx, fy, hopcd = nx, ny, HOPCD
      if fy < best then best = fy; score = score + 10 end
    end
  end

  local cx = fx + 8
  local lane = LANES[fy]
  if fy == 1 then
    for i, bx in ipairs(BAYS) do
      if not homes[i] and math.abs(cx - bx) <= 10 then
        homes[i] = true
        score = score + 50 + (timer // 60) * 2
        log("HOME " .. i .. " T=" .. T)
        if homes_filled() == 5 then
          state, msgt = "win", 0
          score = score + 1000
          if score > hi then hi = score end
          log("WIN score=" .. score .. " T=" .. T)
        else
          reset_frog()
        end
        return
      end
    end
    return die("hedge")
  elseif lane and (lane.kind == "log" or lane.kind == "turtle") then
    local on = false
    for i = 1, #lane.xs do
      local x = objx(lane, i)
      if cx >= x and cx < x + lane.len then on = true break end
    end
    if not on then return die("water") end
    fx = fx + lane.spd
    if fx < -8 or fx > 312 then return die("water") end
  elseif lane then
    for i = 1, #lane.xs do
      local x = objx(lane, i)
      if fx + 3 < x + lane.len - 1 and fx + 13 > x + 1 then return die("car") end
    end
  end
  if timer <= 0 then return die("time") end
end

function init()
  bg.map(0, "sea")
  bg.map(1, "level")
  reset_frog()
end

function update()
  T = T + 1
  if state == "title" then
    if btnp("a") or btnp("start") then new_game() end
  elseif state == "play" then
    update_play()
  elseif state == "dying" then
    deadt = deadt - 1
    if deadt <= 0 then
      lives = lives - 1
      if lives <= 0 then
        state, msgt = "over", 0
        if score > hi then hi = score end
      else reset_frog(); state = "play" end
    end
  else -- win / over
    msgt = msgt + 1
    if msgt > 30 and (btnp("a") or btnp("start")) then state = "title" end
  end
end

---------------------------------------------------------------- drawing
local function panel(x, y, w)     -- w in 32px pieces, 16px tall
  for i = 0, w - 1 do spr("panel", x + i * 32, y) end
end

local function ctext(s, y, col)  -- centred 8x8 text
  text(s, 160 - #s * 4, y, col)
end

local function draw_lanes()
  for row, l in pairs(LANES) do
    local y = row * 16
    for i = 1, #l.xs do
      local x = objx(l, i)
      if l.kind == "log" then
        local n = l.len // 16
        spr("log_l", x, y)
        for k = 1, n - 2 do spr("log_m", x + k * 16, y) end
        spr("log_r", x + (n - 1) * 16, y)
      elseif l.kind == "turtle" then
        for k = 0, l.len // 16 - 1 do spr("turtle", x + k * 16, y) end
      elseif l.kind == "truck" then
        spr("truck", x, y, {fx = l.spd > 0})
      else
        spr(l.kind, x, y, {fx = l.spd > 0, pal = l.pal})
      end
    end
  end
end

local function draw_frog()
  local x, y = fx, fy * 16
  if state == "dying" then
    if deadkind == "water" then spr("splash", x, y)
    elseif deadkind == "time" then spr("skull", x, y)
    else spr("splat", x, y) end
    return
  end
  local hop = hopcd > HOPCD - 4
  if dir == "u" or dir == "d" then
    spr(hop and "frog_uh" or "frog_u", x, y, {fy = dir == "d"})
  else
    spr(hop and "frog_sh" or "frog_s", x, y, {fx = dir == "r"})
  end
end

local function draw_hud()
  text("SCORE " .. score, 8, 4, 1)
  text("HI " .. hi, 244, 4, 2)
  for i = 1, lives - 1 do spr("lifefrog", 8 + (i - 1) * 10, 228) end
  if state == "play" or state == "dying" then
    local segs = (timer + 89) // 90       -- 20 segments = 30 s
    local s = segs <= 5 and "tbarlow" or "tbar"
    for i = 1, segs do spr(s, 312 - i * 8, 228) end
    text("TIME", 120, 228, 2)
  end
end

function draw()
  backdrop("#0b0b14")
  bg.scroll(0, -(T // 6) % 16, 0)   -- slow drift of the water
  bg.scroll(1, 0, 0)
  -- lily pads / filled homes
  for i, bx in ipairs(BAYS) do
    if homes[i] then spr("frog_u", bx - 8, 16) else spr("lily", bx - 8, 16) end
  end
  draw_lanes()

  if state == "title" then
    local word = {"F", "R", "O", "G", "G", "Y"}
    panel(64, 96, 6); panel(64, 112, 6); panel(64, 128, 6)
    for i, c in ipairs(word) do
      local bob = (math.floor(T / 8 + i) % 2)
      spr("L" .. c, 112 + (i - 1) * 16, 100 - bob)
    end
    ctext("FILL ALL 5 LILY PADS", 120, 6)
    if (T // 30) % 2 == 0 then ctext("PRESS Z TO START", 132, 2) end
    spr("frog_u", START_X, START_Y * 16)
  elseif state == "play" or state == "dying" then
    draw_frog()
  elseif state == "over" then
    panel(96, 104, 4); panel(96, 120, 4)
    ctext("GAME OVER", 108, 3)
    if msgt > 30 then ctext("PRESS Z", 124, 1) end
  elseif state == "win" then
    panel(80, 96, 5); panel(80, 112, 5); panel(80, 128, 5)
    ctext("YOU MADE IT HOME!", 102, 4)
    ctext("SCORE " .. score, 118, 2)
    if msgt > 30 then ctext("PRESS Z", 134, 1) end
  end
  draw_hud()
end
