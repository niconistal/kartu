-- BAKEHOUSE: the bakery probe brief remade with the top-down kit (art from an earlier bakery cart)
palette main
  . clear
  w #ffffff white
  k #1a1c2c ink
  y #ffcd75 cream
  r #b13e53 red
  o #ef7d57 orange
  g #a7f070 green
  b #3b5dc9 blue
  c #41a6f6 sky
  s #94b0c2 grey
  d #566c86 slate
  n #333c57 navy
  m #5d275d plum
  t #8b5a2b brown
  l #c28a52 tan
  p #f4b4c4 pink

palette friendly
  . clear
  w #ffe0a8 flour
  k #1a1c2c ink
  y #ffcd75 cream
  r #b13e53 red
  o #ef7d57 orange
  g #a7f070 green
  b #3b5dc9 blue
  c #41a6f6 sky
  s #94b0c2 grey
  d #566c86 slate
  n #333c57 navy
  m #8b5a2b crust
  t #8b5a2b brown
  l #c28a52 tan
  p #f4b4c4 pink

sprite cat_a 16x16 pal=main
................
..k..........k..
.kok........kok.
.kook......kook.
.koooookkoooook.
.kooooooooooook.
kookkooooookkook
kooooooppooooook
.kowwwokkowwwok.
..kkoowwwwookk..
...koooooooook..
..koowwwwwwook..
..koowwwwwwookkk
..kowwkkkkwwok.k
...kkk....kkk...
................
sprite cat_b 16x16 pal=main
................
..k..........k..
.kok........kok.
.kook......kook.
.koooookkoooook.
.kooooooooooook.
kookkooooookkook
kooooooppooooook
.kowwwokkowwwok.
..kkoowwwwookk..
...koooooooook.k
..koowwwwwwookk.
..koowwwwwwook..
...kwwkkkkwwk...
....kk....kk....
................
clip cat_walk fps=8 cat_a cat_b

sprite ghost_a 16x16 pal=main
................
.....mmmmmm.....
....mwwwwwwm....
...mwwwwwwwwm...
..mwwwwwwwwwwm..
..mwwkkwwkkwwm..
..mwwkkwwkkwwm..
.mwwwwwwwwwwwwm.
.mwwwwwmmwwwwwm.
.mwwwwmwwmwwwwm.
.mwwwwwwwwwwwsm.
.mwwwwwwwwwwwsm.
.mswwwwwwwwwssm.
.mswwwmwwwwmssm.
.mwm..mwwm..msm.
..m....mm....m..
sprite ghost_b 16x16 pal=main
................
.....mmmmmm.....
....mwwwwwwm....
...mwwwwwwwwm...
..mwwwwwwwwwwm..
..mwwkkwwkkwwm..
..mwwkkwwkkwwm..
.mwwwwwwwwwwwwm.
.mwwwwwmmwwwwwm.
.mwwwwmwwmwwwwm.
.mwwwwwwwwwwwsm.
.mwwwwwwwwwwwsm.
.mswwwwwwwwwssm.
.mswmwwwwwmwssm.
..m.mwwm.mwsm...
.....mm...mm....
clip ghost fps=4 ghost_a ghost_b

sprite cookie 12x12 pal=main
....tttt....
..ttllkltt..
.tyyllllklt.
tyklllllllt.
tllllllllllt
tllllklllllt
tllllllllllt
tlkllllllklt
tllllllllllt
.tlllkllllt.
..tttttttt..
....tttt....

sprite heart 8x8 pal=main
.rr.rr..
rrrrrrr.
rwrrrrr.
rrrrrrr.
.rrrrr..
..rrr...
...r....
........

sprite cookie_icon 8x8 pal=main
..tttt..
.tlkllt.
tllllllt
tlllkllt
tklllllt
tllllklt
.tllllt.
..tttt..

tile floor 16x16 pal=main
nnnnnnnddddddddk
nnnnnnnddddddddk
nnnnnnnddddddddk
nnnnnnnddddddddk
nnnnnnnddddddddk
nnnnnnnddddddddk
nnnnnnnddddddddk
kkkkkkkkkkkkkkkk
ddddddddnnnnnnnk
ddddddddnnnnnnnk
ddddddddnnnnnnnk
ddddddddnnnnnnnk
ddddddddnnnnnnnk
ddddddddnnnnnnnk
ddddddddnnnnnnnk
kkkkkkkkkkkkkkkk

tile wall 16x16 pal=main flags=solid
kkkkkkkkkkkkkkkk
rmmmmmmkrmmmmmmk
mmmmmmmkmmmmmmmk
mmmmmmmkmmmmmmmk
kkkkkkkkkkkkkkkk
mmmkrmmmmmmkrmmm
mmmkmmmmmmmkmmmm
mmmkmmmmmmmkmmmm
kkkkkkkkkkkkkkkk
rmmmmmmkrmmmmmmk
mmmmmmmkmmmmmmmk
mmmmmmmkmmmmmmmk
kkkkkkkkkkkkkkkk
mmmkrmmmmmmkrmmm
mmmkmmmmmmmkmmmm
mmmkmmmmmmmkmmmm

tile table 16x16 pal=main flags=solid
kkkkkkkkkkkkkkkk
kyyyyyyyyyyyyyyk
kyllllllllllllyk
kllllllwwllllllk
kllllllwwwlllllk
kllllllllllllllk
klllllllllllwllk
kllllllllllllllk
kllllllllllllllk
kllllwlllllllllk
kllllllllllllllk
kllllllllllllllk
kttttttttttttttk
kttttttttttttttk
kttttttttttttttk
kkkkkkkkkkkkkkkk

tile oven 16x16 pal=main flags=solid
kkkkkkkkkkkkkkkk
kssssssssssssssk
ksdssdssdssssssk
kssssssssssssssk
kskkkkkkkkkkkksk
kskooooooooooksk
kskooooooooooksk
kskooooooooooksk
kskoyooyooyooksk
kskyyoyyoyyoyksk
kskkkkkkkkkkkksk
kssssssssssssssk
kssssssssssssssk
kdddddddddddddsk
kssssssssssssssk
kkkkkkkkkkkkkkkk

tile sack 16x16 pal=main flags=solid
................
......kkkk......
.....kswwsk.....
......kttk......
.....kwwwwk.....
....kwwwwwwk....
...kwwwwwwwwk...
..kwwwwwwwwwsk..
..kwwbbbbwwwsk..
..kwwbwwbwwwsk..
..kwwbbbbwwwsk..
..kwwwwwwwwwsk..
..kwwwwwwwwssk..
...kssssssssk...
....kkkkkkkk....
................

tile web_l 16x16 pal=main
ssssssssssss....
ss....s...s.....
s.s...s..s......
s..sssssss......
s..s..s.s.......
s...s.ss........
ssssssss........
s...ss..........
s..s.s..........
s.s.s...........
sss.............
ss..............
s...............
................
................
................

tile web_r 16x16 pal=main
....ssssssssssss
.....s...s....ss
......s..s...s.s
......sssssss..s
.......s.s..s..s
........ss.s...s
........ssssssss
..........ss...s
..........s.s..s
...........s.s.s
.............sss
..............ss
...............s
................
................
................

tile hud 16x16 pal=main
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
mmmmmmmmmmmmmmmm
kkkkkkkkkkkkkkkk

tile panel 16x16 pal=main
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkk

sprite baker from=ghost_a pal=friendly

map kitchen 20x15   -- KITCHEN
legend # wall  T table  O oven  S sack  < web_l  > web_r  , floor  . floor
legend @ @hero:floor  k @cookie:floor  g @ghost:floor  v @ghostv:floor  s @spook:floor  B @baker:floor
####################
####################
#<.......T........>#
#.@k.....T.....k...#
#........T..g......#
#..OO..............#
#..............SS..#
#...................
#...TTTT............
#...TTTT.......k...#
#..................#
#.....Sv.......S...#
#..................#
#..................#
#########..#########

map pantry 20x15   -- PANTRY
legend # wall  T table  O oven  S sack  < web_l  > web_r  , floor  . floor
legend @ @hero:floor  k @cookie:floor  g @ghost:floor  v @ghostv:floor  s @spook:floor  B @baker:floor
####################
####################
#<................>#
#.SS.SSgSS...SS.SS.#
#..................#
#..k...........k...#
#......TTTTTT......#
.......T....T......#
.......T..k.T......#
#......TT..TT......#
#...B..............#
#........g.........#
#..SS..........SS..#
#..................#
#########..#########

map cellar 20x15   -- CELLAR
legend # wall  T table  O oven  S sack  < web_l  > web_r  , floor  . floor
legend @ @hero:floor  k @cookie:floor  g @ghost:floor  v @ghostv:floor  s @spook:floor  B @baker:floor
#########..#########
#########..#########
#<................>#
#..................#
#..SS..SS..SS..SS..#
#..................#
#..k.....g......k..#
#...................
#...................
#..SS..SS..SS..SS..#
#..................#
#..k...............#
#.............g....#
#..................#
####################

map ovenroom 20x15   -- OVEN ROOM
legend # wall  T table  O oven  S sack  < web_l  > web_r  , floor  . floor
legend @ @hero:floor  k @cookie:floor  g @ghost:floor  v @ghostv:floor  s @spook:floor  B @baker:floor
#########..#########
#########..#########
#.................>#
#.OO..OO....OO..OO.#
#..................#
#..k.........k.....#
#......TTTTTT......#
.......T....T......#
.......T..k.T......#
#..v...T....T..v...#
#......TT..TT......#
#..................#
#................s.#
#..................#
####################
