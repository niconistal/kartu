-- title: Dungeon (top-down kit)
-- The keyquest probe brief, remade with the kit: a knight finds the key in the vault
-- and opens the door out of the hall. Bats chase; the sword (A) swats them.

local td = kit("topdown")

td.setup {
  title = "DUNGEON",
  intro = "The key is somewhere east. Bring it back to the door in the hall. A swings your sword.",
  rooms = { { "hall", "gallery", "vault" } },
  names = { hall = "THE HALL", gallery = "THE GALLERY", vault = "THE VAULT" },
  backdrop = "#1a1c2c",
  hero = { spr = "knight1", walk = "knight_walk", speed = 1.5, hp = 3 },
  attack = { btn = "a", spr = "sword", spr_v = "swordv", reach = 14, time = 12 },
  enemies = {
    bat = { spr = "bat_fly", move = "chase", speed = 0.8, range = 90, hp = 1, drop = "heart" },
  },
  pickups = {
    key = { spr = "key", toast = "You found the key!" },
    heart = { spr = "heart", heal = 1 },
  },
  doors = { door = { needs = "key", opens = "dooropen", locked = "Locked! Find the key." } },
  win = "flag:exit",
  win_text = "ESCAPED!",
  hud = { hearts = { "heart", "heart0" }, show = { "key" } },
  ui = { ink = "k", paper = "w", accent = "y" },
  music = { title = "adventure", play = "cave", win = "victory" },
}

function update() td.update() end
function draw() td.draw() end
