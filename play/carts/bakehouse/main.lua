-- title: Bakehouse (top-down kit)
-- The bakery probe brief, remade with the kit: a cat collects every cookie in a
-- haunted bakery, dodging ghosts. Four rooms; a friendly ghost baker gives tips.

local td = kit("topdown")

td.setup {
  title = "BAKEHOUSE",
  intro = "Collect every cookie in the haunted bakery. Ghosts sting!",
  rooms = { { "kitchen", "pantry" }, { "cellar", "ovenroom" } },
  names = { ovenroom = "OVEN ROOM" },
  backdrop = "#1a1c2c",
  hero = { spr = "cat_a", walk = "cat_walk", speed = 1.5, hp = 3 },
  enemies = {
    ghost = { spr = "ghost", move = "bounce_x", speed = 0.9 },
    ghostv = { spr = "ghost", move = "bounce_y", speed = 0.9 },
    spook = { spr = "ghost", move = "chase", speed = 0.5, range = 100, flying = true },
  },
  pickups = {
    cookie = { spr = "cookie", goal = true },
  },
  npcs = {
    baker = { spr = "baker", name = "OLD BAKER", say = function(w)
      local left = td.goals_left("cookie")
      if left == 0 then return "All my cookies! Bless you, cat." end
      return { "Oh, a cat! My ghosts ate the whole bakery...",
               left .. " cookies still hidden. The oven room ghost follows you, so keep moving!" }
    end },
  },
  win = "collect",
  win_text = "ALL BAKED!",
  lose_text = "SPOOKED!",
  hud = { hearts = { "heart" }, show = { "cookie" } },
  ui = { ink = "k", paper = "w", accent = "y" },
  music = "village",
}

function update() td.update() end
function draw() td.draw() end
