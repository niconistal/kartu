-- title: Crypt (Core 1.1 demo)
-- Find the key, open the door in the south wall, get out. Bats bounce around the halls.
-- Shows the Core 1.1 API: tile flags + move/touching/overlap, spawns from the map,
-- camera with clamp, rect, and text boxes/alignment.

local hero, bats, key, hearts, state, t, hurt

local function start()
  bg.map(0, "crypt")
  local h = spawns("crypt", "hero")[1]
  hero = { x = h.x, y = h.y, spr = "knight1", face = 1 }
  bats = {}
  for i, b in ipairs(spawns("crypt", "bat")) do
    -- alternate horizontal and vertical patrols
    bats[i] = { x = b.x, y = b.y, spr = "bat1", vx = (i % 2 == 1) and 1 or 0, vy = (i % 2 == 0) and 1 or 0 }
  end
  local k = spawns("crypt", "key")[1]
  key = { x = k.x, y = k.y, spr = "key" }
  hearts, hurt, t = 3, 0, 0
end

function init()
  start()
  state = "title"
end

function update()
  t = t + 1
  if state ~= "play" then
    if btnp("a") or btnp("start") then
      start()
      state = "play"
    end
    return
  end

  local dx, dy = 0, 0
  if btn("left") then dx = -2 end
  if btn("right") then dx = 2 end
  if btn("up") then dy = -2 end
  if btn("down") then dy = 2 end
  if dx ~= 0 then hero.face = dx end
  move(hero, dx, dy)

  for _, b in ipairs(bats) do
    local bx, by = move(b, b.vx, b.vy)
    if bx then b.vx = -b.vx end
    if by then b.vy = -b.vy end
    if hurt == 0 and overlap(hero, b) then
      hearts = hearts - 1
      hurt = 60
      log("ouch", hearts)
      if hearts == 0 then state = "over" log("LOSE") end
    end
  end
  if hurt > 0 then hurt = hurt - 1 end

  if key and overlap(hero, key) then
    key = nil
    log("got key")
  end
  -- walking into the locked door: open it if we have the key
  local tx, ty = touching({ x = hero.x, y = hero.y + 2, spr = "knight1" }, "door")
  if tx and not key then
    bg.tile(0, tx, ty, "dooropen")
    log("door open")
  end
  if touching(hero, "exit") then
    state = "win"
    log("WIN", t)
  end
end

function draw()
  backdrop("#1a1c2c")
  camera(hero.x - 152, hero.y - 112, { clamp = 0 })

  if key then spr("key", key.x, key.y + sin(t / 60) * 2) end
  for _, b in ipairs(bats) do spr("bat_fly", b.x, b.y, { fx = b.vx < 0 }) end
  if hurt % 8 < 4 then
    spr(hero.face < 0 and "walk_l" or "walk_r", hero.x, hero.y)
  end

  -- HUD: screen-space bar
  rect(0, 0, 320, 14, "k", { screen = true })
  for i = 1, 3 do spr(i <= hearts and "heart" or "heart0", 4 + (i - 1) * 11, 3, { screen = true }) end
  if not key then spr("key", 300, -1, { screen = true }) end
  text("CRYPT", 296, 3, { col = "w", align = "right" })

  if state == "title" then
    text("CRYPT", 160, 70, { col = "y", scale = 3, align = "center", shadow = "p" })
    text("Find the key. Open the south door.\nMind the bats.", 160, 120, { col = "w", align = "center", bg = "k", pad = 6 })
    if t % 60 < 40 then text("press A", 160, 170, { col = "l", align = "center", bg = "k" }) end
  elseif state == "win" then
    text("YOU GOT OUT!", 160, 100, { col = "l", scale = 2, align = "center", bg = "k", pad = 8 })
  elseif state == "over" then
    text("THE BATS WIN", 160, 100, { col = "r", scale = 2, align = "center", bg = "k", pad = 8 })
  end
end
