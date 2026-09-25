-- title: Pirate Treasure + shanty (AI sound edit)
-- A pirate digs up 3 chests on a small island, dodging crabs, then returns to the ship to win.

local td = kit("topdown")

td.setup {
  title = "PIRATE TREASURE",
  intro = "Dig up all 3 chests, dodge the crabs, then get back to your ship!",
  map = "island",
  hero = { spr = "pirate1", walk = "pirate_walk", speed = 1.5, hp = 3 },
  enemies = {
    crab = { spr = "crab_walk", move = "bounce_x", speed = 0.9, hp = 1, damage = 1 },
  },
  pickups = {
    chest = { spr = "chest", goal = true, toast = "You dug up a chest!", sfx = "treasure" },
  },
  npcs = {
    parrot = { spr = "parrot", name = "PARROT", say = "Arrr! Three chests be hidden on this island, matey! Look high and low!" },
  },
  win = function(w) return td.goals_left() == 0 and touching(w.hero, "exit") end,
  win_text = "TREASURE HOME!",
  music = "shanty",
  hud = { hearts = { "heart", "heart0" }, show = { "chest" } },
  on = {
    pickup = function(w, p)
      if td.goals_left() == 0 then td.toast("All treasure found! Back to the ship!") end
    end,
  },
}

function update() td.update() end
function draw() td.draw() end
