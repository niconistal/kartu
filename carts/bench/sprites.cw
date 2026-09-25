-- P0 bench cart: hand-drawn sprites (palette-char grids, design §7)
-- text() colours index the first palette, so `ui` goes first.

palette ui
  . clear
  w #f4f4f4 white
  k #1a1c2c ink
  y #ffcd75 yellow
  g #38b764 green
  r #e43b44 red
  b #29366f navy

palette hero
  . clear
  k #1a1c2c ink
  O #ef7d57 orange
  o #b35a3a dark orange
  W #ffe8c8 cream
  p #f7a1c4 pink
  r #b13e53 scarf
  w #ffffff shine

palette ghost
  . clear
  k #1a1c2c ink
  W #f4f4f4 sheet
  l #c2c3f2 lavender
  v #5d4a9c violet
  p #f7a1c4 blush

palette snack
  . clear
  k #1a1c2c ink
  d #7a4a28 chip
  c #c28a4a dough
  C #e8b878 light dough
  w #fff4e0 shine

sprite cat_walk.1 16x16 pal=hero
................
................
.k.....k........
kOk...kOk.......
kOOkkkOOk.......
kOOOOOOOk.......
kOwkOOwkOk......
kOOOOppOOk......
.kWWWWWOk.....k.
..krrrrk.....kOk
.kOOOOOOOkkkkOk.
.kOOoOOoOOOOOk..
.kOOOOOOOOOOk...
.kOk.kOk.kOk....
.kk..kk..kk.....
................

sprite cat_walk.2 16x16 pal=hero
................
.k.....k........
kOk...kOk.......
kOOkkkOOk.......
kOOOOOOOk.......
kOwkOOwkOk......
kOOOOppOOk....k.
.kWWWWWOk....kOk
..krrrrk....kOk.
.kOOOOOOOkkkOk..
.kOOoOOoOOOOk...
.kOOOOOOOOOOk...
..kOkkOk.kOk....
...kk.kk..kk....
................
................

sprite ghost.1 16x16 pal=ghost
................
.....kkkkkk.....
...kkWWWWWWkk...
..kWWWWWWWWWWk..
..kWWWWWWWWWWk..
.kWWkkWWWWkkWWk.
.kWWkvWWWWkvWWk.
.kWWWWWWWWWWWWk.
.kWWppWWWWppWWk.
.kWWWWWkkWWWWWk.
.kWWWWWWWWWWWWk.
.kWlWWWWWWWWlWk.
.kllWWlWWlWWllk.
.klllllllllllk..
.kllkllkllkllk..
..kk.kk.kk.kk...

sprite ghost.2 16x16 pal=ghost
................
.....kkkkkk.....
...kkWWWWWWkk...
..kWWWWWWWWWWk..
..kWWWWWWWWWWk..
.kWWkkWWWWkkWWk.
.kWWkvWWWWkvWWk.
.kWWWWWWWWWWWWk.
.kWWppWWWWppWWk.
.kWWWWWkkWWWWWk.
.kWWWWWWWWWWWWk.
.kWlWWWWWWWWlWk.
.kllWWlWWlWWllk.
..klllllllllllk.
..kllkllkllkllk.
...kk.kk.kk.kk..

sprite cookie 12x12 pal=snack
....kkkk....
..kkCCCCkk..
.kCwCCcCCCk.
.kCCdCCCdCk.
kCCCCCcCCCCk
kCcCCCCCdCCk
kCCCdCCCCCck
kCCCCCCdCCCk
.kCdCCcCCCk.
.kCCCCCCCdk.
..kkcCCCkk..
....kkkk....

clip cat_walk fps=8 cat_walk.1 cat_walk.2
clip ghost fps=4 ghost.1 ghost.2
