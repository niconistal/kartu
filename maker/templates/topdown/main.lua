-- title: Star Field
-- Starter cart (top-down kit): collect the three stars, keep away from the slime.
-- Change the brief by editing the td.setup table and assets.cw; API: docs tool / API.md.

local td = kit("topdown")

td.setup {
  title = "STAR FIELD",
  intro = "Collect the three stars. Keep away from the slime.",
  map = "field",
  hero = { spr = "hero1", walk = "hero_walk", speed = 1.5, hp = 3 },
  enemies = {
    slime = { spr = "slime_hop", move = "wander", speed = 0.5, hp = 1 },
  },
  pickups = {
    star = { spr = "star", goal = true, toast = "A star!" },
  },
  win = "collect",
  win_text = "ALL STARS!",
  hud = { hearts = { "heart", "heart0" }, show = { "star" } },
}

function update() td.update() end
function draw() td.draw() end
