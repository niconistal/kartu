-- title: Jukebox (sound test)
-- Every built-in song, sound effect and instrument. Left/right: page. Up/down: pick.
-- A: play. B: stop the music.

local pages = {
  { name = "SONGS", items = { "adventure", "village", "cave", "boss", "victory" } },
  { name = "SOUND EFFECTS", items = { "coin", "pickup", "key", "heal", "powerup", "jump", "hit", "hurt",
      "swing", "laser", "explosion", "blip", "select", "door", "locked", "talk", "step", "magic",
      "win", "lose", "start" } },
  { name = "INSTRUMENTS", items = { "piano", "epiano", "bell", "marimba", "organ", "flute", "lead",
      "softlead", "strings", "pad", "brass", "pluck", "harp", "bass", "synbass", "pluckbass",
      "square", "pulse", "triangle", "saw", "sine", "kick", "snare", "hat", "openhat", "clap",
      "tom", "crash" } },
}
local DRUMKIT = { kick = true, snare = true, hat = true, openhat = true, clap = true, tom = true, crash = true }
local BASS = { bass = true, synbass = true, pluckbass = true }
local page, sel, last = 1, { 1, 1, 1 }, ""

local function fire()
  local p = pages[page]
  local it = p.items[sel[page]]
  if page == 1 then
    music(it)
  elseif page == 2 then
    sfx(it)
  elseif DRUMKIT[it] then
    play("o4 l8 c c c4", { inst = it, vol = 0.9 })
  elseif BASS[it] then
    play("o2 l8 c c g c > c < g e c", { inst = it, vol = 0.9 })
  else
    play("o4 l8 c e g > c e g > c4", { inst = it, vol = 0.8 })
  end
  last = it
  log("play " .. it)
end

function update()
  local p = pages[page]
  if btnp("left") then page = (page - 2) % #pages + 1 end
  if btnp("right") then page = page % #pages + 1 end
  if btnp("up") then sel[page] = (sel[page] - 2) % #pages[page].items + 1; sfx("blip", { vol = 0.4 }) end
  if btnp("down") then sel[page] = sel[page] % #pages[page].items + 1; sfx("blip", { vol = 0.4 }) end
  if btnp("a") then fire() end
  if btnp("b") then music(nil, { fade = 0.5 }) end
end

function draw()
  backdrop("#1a1c2c")
  local p = pages[page]
  rect(0, 0, 320, 20, "n", { screen = true })
  text("< " .. p.name .. " >", 160, 6, { col = "y", align = "center" })
  local n = #p.items
  local per = 16
  local top = math.max(0, math.min(sel[page] - 8, n - per))
  for i = 1, math.min(per, n) do
    local k = top + i
    local y = 22 + i * 11
    local it = p.items[k]
    if k == sel[page] then
      rect(40, y - 2, 240, 11, "b", { screen = true })
      text(it, 56, y, "w")
      text(">", 44, y, "y")
    else
      text(it, 56, y, "s")
    end
  end
  local now = music_playing()
  rect(0, 214, 320, 26, "n", { screen = true })
  text(now and ("music: " .. now) or "music: -", 8, 218, now and "g" or "m")
  text("A play   B stop music   < > page", 8, 229, "m")
end
