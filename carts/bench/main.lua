-- title: P0 bench (gate scene)
-- P0 gate scene: 2 parallax BG layers + 64 sprites at 60 fps.
-- A: stress mode (128 sprites + all 4 layers). Left/right: scroll faster/back.

local things = {}
local stress = false
local cam = 0

local KINDS = { "cat_walk", "ghost", "cookie" }

local function spawn(i)
  local kind = KINDS[(i - 1) % 3 + 1]
  return {
    kind = kind,
    x = rnd(320), y = 24 + rnd(176),
    vx = (0.4 + rnd(1.4)) * (irnd(0, 1) * 2 - 1),
    vy = kind == "cookie" and (rnd(2) - 1) or 0,
    phase = rnd(1),
  }
end

local function populate(n)
  for i = #things + 1, n do things[i] = spawn(i) end
  for i = n + 1, #things do things[i] = nil end
end

function init()
  backdrop("#3b5dc9")
  bg.map(0, "far")
  bg.map(1, "near")
  bg.map(2, "fx_clouds")
  bg.map(3, "fx_front")
  bg.show(2, false)
  bg.show(3, false)
  populate(64)
end

function update()
  cam = cam + 1
  if btn("right") then cam = cam + 3 end
  if btn("left") then cam = cam - 2 end
  if btnp("a") then
    stress = not stress
    populate(stress and 128 or 64)
    bg.show(2, stress)
    bg.show(3, stress)
  end
  for _, t in ipairs(things) do
    t.x = t.x + t.vx
    if t.x < -16 then t.x = 320 elseif t.x > 320 then t.x = -16 end
    if t.kind == "cookie" then
      t.y = t.y + t.vy
      if t.y < 16 or t.y > 212 then t.vy = -t.vy end
    end
  end
end

function draw()
  bg.scroll(0, cam * 0.25, 0)
  bg.scroll(1, cam, 0)
  bg.scroll(2, cam * 0.5, 0)
  bg.scroll(3, cam * 1.5, 0)
  local f = frame()
  for _, t in ipairs(things) do
    local y = t.y
    if t.kind == "ghost" then y = y + sin(f / 90 + t.phase) * 6 end
    spr(t.kind, t.x, y, { fx = t.vx < 0, layer = 1 })
  end
  text("KARTU . P0", 8, 8, 1)
  text((stress and "STRESS 128 spr 4 layers" or "GATE 64 spr 2 layers"), 8, 20, 3)
end
