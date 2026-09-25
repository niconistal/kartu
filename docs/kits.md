# Kits

A **kit** is a genre library built into the console. `local td = kit("topdown")` loads it. Kits
run as ordinary cart code — same sandbox, same budget — so anything a kit does you could do by
hand, and you can mix both freely. They exist because the first thirty lines of every game
in a genre are the same thirty lines, and a model (or a person) should spend its attention
on what is different about *this* game.

Today there is one kit. A platformer kit and a shoot-'em-up kit are the most wanted
contributions.

## The top-down kit

Zelda-like adventures: a hero, rooms, enemies, pickups, locked doors, NPCs with dialogue, a HUD,
title / win / lose screens, y-sorted drawing, sound for every event. A whole cart:

```lua
local td = kit("topdown")
td.setup {
  title = "DUNGEON", intro = "Find the key. A swings your sword.",
  rooms = { { "hall", "gallery", "vault" } },        -- or map = "one_big_map"
  hero = { spr = "knight1", walk = "knight_walk", hp = 3 },
  attack = { btn = "a", spr = "sword", spr_v = "swordv" },
  enemies = { bat = { spr = "bat_fly", move = "chase", speed = 0.8, hp = 1, drop = "heart" } },
  pickups = { key = { spr = "key", toast = "You found the key!" }, heart = { spr = "heart", heal = 1 } },
  doors = { door = { needs = "key", opens = "dooropen" } },   -- tile names
  win = "flag:exit",                                          -- dooropen has flags=exit
  hud = { hearts = { "heart", "heart0" }, show = { "key" } },
}
function update() td.update() end
function draw() td.draw() end
```

Things are placed with map markers in `assets.cw`, one kind per `@name`:

```text
legend P @hero:floor  B @bat:floor  K @key:floor
```

Each marker kind must be a key of `enemies`, `pickups` or `npcs` (or `hero`).

### Config

All fields are optional except `map`/`rooms` and `hero`.

| field | what |
|---|---|
| `map = "name"` | one map (scrolls if bigger than the screen) |
| `rooms = {{"a","b"},{"c","d"}}` | a grid of maps; walking off an edge enters the neighbour |
| `names = { a = "KITCHEN" }` | room names for the HUD (default: the map name in capitals) |
| `hero = { spr=, hit=, walk=, up=, down=, walk_up=, walk_down=, left=, walk_left=, speed=1.5, hp=3, diag=true }` | sprites face right and are flipped for left unless `left=` is given. `hit` defaults to the sprite's `hit=`, else a lower-body box that fits one-tile gaps |
| `attack = { btn="a", spr=, spr_v=, reach=14, time=12, damage=1 }` | no attack if absent |
| `enemies = { bat = { spr=, hit=, hp=1, speed=1, move="still\|bounce_x\|bounce_y\|wander\|chase", range=80, damage=1, flying=false, drop="heart" } }` | |
| `pickups = { key = { spr=, item="key", heal=0, score=0, goal=false, say=, toast= } }` | `goal = true` pickups count toward `win = "collect"` |
| `npcs = { owl = { spr=, name="OWL", say = {"line", "line"} or function(world) … end } }` | |
| `doors = { door = { needs="key", consume=true, opens="dooropen", locked="Locked." } }` | tile names |
| `win = "flag:exit" \| "collect" \| "collect:cookie" \| function(world)` | |
| `title=, intro=, win_text="YOU WIN!", lose_text="GAME OVER"` | |
| `hud = { hearts = {"heart", "heart0"}, show = {"key", "cookie"} }` or `false` | |
| `sounds = { pickup="coin", hurt="hurt", … }` or `false` | override the default sfx per event. Events: `start pickup goal key heal hurt swing hit defeat door locked talk win lose` |
| `music = "song"` or `{ title=, play=, win=, lose=, rooms = { cellar = "cave" } }` | songs from `assets.cw` or built in (`adventure cave village boss victory`) |
| `ui = { ink="k", paper="w", accent="y" }` | default: darkest / lightest of the first palette |
| `on = { start=, hurt=, defeat=, pickup=, room=, update=, draw=, hud= }` | hooks; `draw` runs in world space after the actors |

### Rules of thumb

- Room maps are 20 × 15. The top 14 px are under the HUD, so make row 0 wall (or a copy of
  row 1 where a doorway leads up). Leave gaps in the outer wall where rooms connect.
- Give solid tiles `flags=solid`; a door tile `flags=solid,door`; its open version
  `flags=exit` if walking through it wins.
- Don't put an enemy within ~6 tiles of a room entrance or the start: playtest flags a hit
  within one second of arriving as unfair.
- Sprites face **right**; the kit flips them for left.

### Talking to the kit

`td.say(lines [, who])` dialogue · `td.toast(msg)` · `td.box(x, y, w, h)` framed panel ·
`td.spawn(kind, x, y)` · `td.hurt(n)` · `td.win()` · `td.lose()` · `td.goals_left([kind])`.

Everything the kit knows lives in the global `world`: `world.hero`, `world.enemies`,
`world.pickups`, `world.npcs` (current room), `world.room`, `world.state`
(`title | play | win | over | pause`), `world.items` (inventory counts), `world.talk`. Bots,
`playtest`, `--watch` and your own code all read it.

The kit logs `state <s>`, `room <name>`, `got <kind>`, `hurt hp=<n>`, `defeated <kind>`,
`door open`, `locked`, `talk <npc>`, `WIN`, `LOSE` — which is exactly what the playtest reads.

### A kit bot plan

```text
hero world.hero
avoid world.enemies 20          # or: fight world.enemies a 30  (if the hero has an attack)
abort world.state == "over"
tap a
wait until world.state == "play"
collect world.pickups           # everything in this room
goto tile 19,7                  # walk to the east doorway…
hold right 16                   # …and through it
wait until world.room == "pantry"
```

### Source

The kit is `core/kits/topdown.lua`, about 750 lines of Lua compiled into the console with
`include_str!`. `kartu docs kit_source` prints it. Reading it is the best way to learn the
console API, and changing it is the best way to start contributing.

## Writing a kit

A kit is a Lua file that returns a table; the console loads it with `kit(name)`. Conventions
that make a kit useful to agents:

- **One config table** (`setup{}`) with sensible defaults for everything, so a tiny game is a
  tiny file.
- **Log what happens** in plain words, and always `WIN` / `LOSE`.
- **Put state in one readable global** (`world`) so bots and `--watch` can see it.
- **Fair by default**: hit boxes that fit through one-tile gaps, a grace period on entering a room.

Open an issue before starting a big one — we'd love to help shape it.
