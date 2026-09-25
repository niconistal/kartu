-- Kartu handheld menu. build-miyoo.sh prepends `CARTS = { {dir=, title=}, … }`.
-- A picks (logs `PICK <dir>`, which the player turns into launch.sh's next cart); MENU quits.
local sel, top = 1, 1
local ROWS = 16

function init()
  backdrop("#1a1c2c")
end

function update()
  if btnp("down") then sel = sel % #CARTS + 1 end
  if btnp("up") then sel = (sel - 2) % #CARTS + 1 end
  if btnp("right") or btnp("r") then sel = math.min(#CARTS, sel + ROWS) end
  if btnp("left") or btnp("l") then sel = math.max(1, sel - ROWS) end
  if sel < top then top = sel end
  if sel >= top + ROWS then top = sel - ROWS + 1 end
  if btnp("a") or btnp("start") then log("PICK " .. CARTS[sel].dir) end
end

function draw()
  text("KARTU", 160, 10, {col = "y", align = "center", scale = 2, shadow = "o"})
  for i = top, math.min(#CARTS, top + ROWS - 1) do
    local y = 40 + (i - top) * 11
    if i == sel then
      text(CARTS[i].title, 24, y, {col = "k", bg = "l", pad = 1})
      text(">", 12, y, "l")
    else
      text(CARTS[i].title, 24, y, "s")
    end
  end
  text("A play   MENU back", 160, 226, {col = "m", align = "center"})
end
