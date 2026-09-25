#!/usr/bin/env python3
"""Generate the P0 bench cart's BG tiles + maps as text-native grids (design §7).

Hand-drawn sprites live in carts/bench/sprites.cw; this script writes
carts/bench/tiles.cw (palettes sky/field, tiles, maps far/near/fx) and then
concatenates both into carts/bench/assets.cw.
"""
import math, os, random

HERE = os.path.dirname(os.path.abspath(__file__))
CART = os.path.join(HERE, "..", "carts", "bench")
random.seed(16)

SKY = [(".", "clear"), ("a", "#29366f"), ("b", "#3b5dc9"), ("c", "#41a6f6"), ("w", "#f4f4f4"),
       ("s", "#c2d4ec"), ("h", "#5d6c9c"), ("H", "#3c4a7a"), ("m", "#8b9bb4"), ("y", "#ffcd75")]
FIELD = [(".", "clear"), ("g", "#38b764"), ("G", "#257179"), ("l", "#a7f070"), ("d", "#7a4a28"),
         ("D", "#4a2c18"), ("b", "#b86f50"), ("B", "#733e39"), ("k", "#1a1c2c"), ("f", "#ffcd75"),
         ("r", "#e43b44"), ("w", "#f4f4f4")]

tiles = {}  # name -> (pal, rows)
maps = []


def grid(w, h, fill="."):
    return [[fill] * w for _ in range(h)]


def cut(name, pal, g, tw, th):
    """Cut a pixel grid into 16x16 tiles named name_x_y; returns the name grid."""
    names = []
    for ty in range(th):
        row = []
        for tx in range(tw):
            rows = ["".join(g[ty * 16 + y][tx * 16:tx * 16 + 16]) for y in range(16)]
            if all(set(r) == {"."} for r in rows):
                row.append(None)
                continue
            n = f"{name}_{tx}_{ty}"
            tiles[n] = (pal, rows)
            row.append(n)
        names.append(row)
    return names


# ---- far layer: clouds + hills on a sky that fades from the backdrop ----
def cloud(w, h):
    g = grid(w, h)
    blobs = [(w * 0.30, h * 0.62, h * 0.36), (w * 0.52, h * 0.45, h * 0.44), (w * 0.72, h * 0.62, h * 0.34), (w * 0.15, h * 0.72, h * 0.24), (w * 0.86, h * 0.74, h * 0.22)]
    for y in range(h):
        for x in range(w):
            for bx, by, r in blobs:
                if (x - bx) ** 2 + (y - by) ** 2 * 1.3 <= r * r and y < h - 3:
                    g[y][x] = "s" if y > h * 0.66 else "w"
    return g


def hills(w, h):
    g = grid(w, h)
    for x in range(w):
        top_far = h * 0.35 + math.sin(x / w * math.tau * 2) * h * 0.18 + math.sin(x / w * math.tau * 5) * h * 0.05
        top_near = h * 0.62 + math.sin(x / w * math.tau * 3 + 1) * h * 0.14
        for y in range(h):
            if y >= top_near:
                g[y][x] = "H"
            elif y >= top_far:
                g[y][x] = "h" if (x + y) % 7 else "m"
    return g


FAR_W, FAR_H = 40, 15
far = [[None] * FAR_W for _ in range(FAR_H)]
cl = cut("cloud", "sky", cloud(48, 32), 3, 2)
cl2 = cut("puff", "sky", cloud(32, 16), 2, 1)
for (cx, cy, c) in [(2, 1, cl), (14, 3, cl2), (22, 1, cl), (33, 2, cl2)]:
    for ty, row in enumerate(c):
        for tx, n in enumerate(row):
            if n:
                far[cy + ty][cx + tx] = n
hl = cut("hill", "sky", hills(160, 64), 10, 4)
for rep in range(FAR_W // 10):
    for ty, row in enumerate(hl):
        for tx, n in enumerate(row):
            if n:
                far[FAR_H - 4 - 2 + ty][rep * 10 + tx] = n
# solid dark band under the hills
sky_fill = [["H"] * 16 for _ in range(16)]
tiles["hillfill"] = ("sky", ["".join(r) for r in sky_fill])
for y in range(FAR_H - 2, FAR_H):
    for x in range(FAR_W):
        far[y][x] = "hillfill"
# a dithered sky band near the top so the backdrop isn't flat
band = ["".join("a" if (x + y) % 2 == 0 or y < 6 else "." for x in range(16)) if y < 10 else "." * 16 for y in range(16)]
band = ["a" * 16 if y < 5 else "".join("a" if (x + y) % 2 == 0 else "." for x in range(16)) if y < 11 else "".join("a" if (x % 4 == 0 and y % 4 == 3) else "." for x in range(16)) for y in range(16)]
tiles["skyband"] = ("sky", band)
for x in range(FAR_W):
    if far[0][x] is None:
        far[0][x] = "skyband"
maps.append(("far", far, FAR_W, FAR_H))

# ---- near layer: ground, grass, floating brick platforms ----
def tile_px(fn):
    return ["".join(fn(x, y) for x in range(16)) for y in range(16)]


def grass_top(x, y):
    blade = (x * 7 + 3) % 5
    if y < 3:
        return "l" if y >= 3 - (blade % 3) and x % 3 != 1 else "."
    if y < 5:
        return "l" if (x + y) % 3 == 0 else "g"
    if y < 7:
        return "g" if (x * 3 + y) % 5 else "G"
    if y == 7:
        return "G" if x % 2 else "d"
    return dirt(x, y)


def dirt(x, y):
    n = (x * 13 + y * 7 + (x * y) % 5) % 11
    return "D" if n == 0 or (n == 4 and y % 2) else ("B" if n == 7 else "d")


def brick(x, y):
    if y in (0, 15):
        return "k"
    if y % 5 == 0:
        return "B"
    off = 0 if (y // 5) % 2 == 0 else 4
    if (x + off) % 8 == 0:
        return "B"
    return "l" if y in (1, 2) and x % 5 else "b"


def flower(x, y):
    petals = {(7, 9), (9, 9), (8, 8), (8, 10)}
    if (x, y) in petals:
        return "r"
    if (x, y) == (8, 9):
        return "f"
    if x == 8 and 11 <= y <= 15:
        return "G"
    if (x, y) in {(7, 13), (9, 12)}:
        return "g"
    return "."


def tuft(x, y):
    return "l" if y >= 12 and (x + y) % 3 == 0 and 3 < x < 13 else ("g" if y >= 14 and 2 < x < 14 else ".")


tiles["grass"] = ("field", tile_px(grass_top))
tiles["dirt"] = ("field", tile_px(dirt))
tiles["brick"] = ("field", tile_px(brick))
tiles["flower"] = ("field", tile_px(flower))
tiles["tuft"] = ("field", tile_px(tuft))

NEAR_W, NEAR_H = 64, 15
near = [[None] * NEAR_W for _ in range(NEAR_H)]
ground = [12] * NEAR_W
for x in range(NEAR_W):
    ground[x] = 12 if (x // 8) % 3 else (11 if (x // 4) % 2 else 12)
for x in range(NEAR_W):
    gy = ground[x]
    near[gy][x] = "grass"
    for y in range(gy + 1, NEAR_H):
        near[y][x] = "dirt"
    if random.random() < 0.18:
        near[gy - 1][x] = random.choice(["flower", "tuft"])
for (px, py, n) in [(5, 8, 4), (16, 6, 3), (27, 8, 5), (38, 5, 3), (46, 8, 4), (56, 6, 4)]:
    for i in range(n):
        near[py][px + i] = "brick"
maps.append(("near", near, NEAR_W, NEAR_H))

# ---- fx layers (stress mode only): foreground tufts + a second cloud deck ----
FX_W, FX_H = 40, 15
fx = [[None] * FX_W for _ in range(FX_H)]
for x in range(FX_W):
    if x % 3 != 1:
        fx[14][x] = "tuft"
fx2 = [[None] * FX_W for _ in range(FX_H)]
for (cx, cy) in [(4, 5), (19, 7), (30, 4)]:
    for ty, row in enumerate(cl2):
        for tx, n in enumerate(row):
            if n:
                fx2[cy + ty][cx + tx] = n
maps.append(("fx_front", fx, FX_W, FX_H))
maps.append(("fx_clouds", fx2, FX_W, FX_H))


def emit():
    out = ["-- generated by scripts/gen_bench_tiles.py: BG palettes, tiles and maps", ""]
    for pname, pal in (("sky", SKY), ("field", FIELD)):
        out.append(f"palette {pname}")
        out += [f"  {c} {v}" for c, v in pal]
        out.append("")
    for n, (pal, rows) in tiles.items():
        out.append(f"tile {n} 16x16 pal={pal}")
        out += rows
        out.append("")
    for mname, m, w, h in maps:
        used = sorted({n for row in m for n in row if n})
        keys = "#%&*+=@$ABCDEFHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        legend = dict(zip(used, keys))
        out.append(f"map {mname} {w}x{h}")
        pairs = [f"{c} {n}" for n, c in legend.items()]
        for i in range(0, len(pairs), 6):
            out.append("legend " + "  ".join(pairs[i:i + 6]))
        out += ["".join(legend[n] if n else "." for n in row) for row in m]
        out.append("")
    return "\n".join(out)


with open(os.path.join(CART, "tiles.cw"), "w") as f:
    f.write(emit())
with open(os.path.join(CART, "sprites.cw")) as f:
    spr = f.read()
with open(os.path.join(CART, "assets.cw"), "w") as f:
    f.write(spr.rstrip() + "\n\n" + emit())
print(f"{len(tiles)} tiles, {len(maps)} maps -> carts/bench/assets.cw")
