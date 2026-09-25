-- CRYPT: Core 1.1 demo (flags, hit boxes, spawns, camera, rect, text options).
-- Art borrowed from keyquest.

palette main
  . clear
  k #1a1c2c ink
  p #5d275d plum
  r #b13e53 red
  o #ef7d57 orange
  y #ffcd75 yellow
  l #a7f070 lime
  g #38b764 green
  t #257179 teal
  n #29366f navy
  b #3b5dc9 blue
  c #41a6f6 cyan
  w #f4f4f4 white
  s #94b0c2 silver
  m #566c86 slate
  d #333c57 dusk

palette bat
  . clear
  k #1a1c2c ink
  p #9a5bc4 wing
  q #5d275d wingdark
  r #ff4f4f eye
  w #f4f4f4 fang

sprite knight1 16x16 pal=main hit=4,6,8,9
.......rr.......
......rrkkk.....
.....kkssswk....
....ksssssswk...
....ksskkkkkk...
....ksskkkwkk...
....ksssskkkk...
.....kkkkkkk....
...krrrrrrrrk...
..kprrryyrrrpk..
..ksprryyrrpsk..
..kskrrrrrrksk..
...kkppppppkk...
....kmmkkmmk....
....kmmkkmmk....
....kkkkkkkk....
sprite knight2 16x16 pal=main hit=4,6,8,9
.......rr.......
......rrkkk.....
.....kkssswk....
....ksssssswk...
....ksskkkkkk...
....ksskkkwkk...
....ksssskkkk...
.....kkkkkkk....
...krrrrrrrrk...
..kprrryyrrrpk..
..ksprryyrrpsk..
..kskrrrrrrksk..
...kkppppppkk...
....kmmkkmmk....
...kmmk..kmmk...
...kkk....kkk...
sprite knight1_l from=knight1 fx
sprite knight2_l from=knight2 fx
clip walk_r fps=8 knight1 knight2
clip walk_l fps=8 knight1_l knight2_l
sprite bat1 16x16 pal=bat hit=3,4,10,8
................
.k............k.
.qk..........kq.
.pqk..k..k..kqp.
.ppqk.kkkk.kqpp.
.pppqkkkkkkqppp.
.pppkkrkkrkkppp.
..ppkkkkkkkkpp..
..p..kkwwkk..p..
.....kkkkkk.....
......k..k......
................
................
................
................
................
sprite bat2 16x16 pal=bat hit=3,4,10,8
................
................
................
......k..k......
......kkkk......
.....kkkkkk.....
....kkrkkrkk....
..qqkkkkkkkkqq..
.pqqkkkwwkkkqqp.
ppppq.kkkk.qpppp
pppp..k..k..pppp
ppp..........ppp
pp............pp
p..............p
................
................
clip bat_fly fps=10 bat1 bat2
sprite key 16x16 pal=main hit=3,3,10,10
................
................
................
..kkkk..........
.kyyyyk.........
kyywkyyk........
kyk..kyk........
kyk..kykkkkkkkk.
kyykkyyyyyyyyyyk
.kyyyykkkkykkyk.
..kkkk....kykyk.
...........k.k..
................
................
................
................
sprite heart 9x8 pal=main
.kk...kk.
krrk.krwk
krrrkrrrk
krrrrrrrk
.krrrrrk.
..krrrk..
...krk...
....k....
sprite heart0 9x8 pal=main
.kk...kk.
kddk.kddk
kdddkdddk
kdddddddk
.kdddddk.
..kdddk..
...kdk...
....k....
tile floor 16x16 pal=main
tnnnnnnktnnnnnnk
nnnnnnnknnnnnnnk
nnnntnnknnnnnntk
nnnnnnnknnnnnnnk
nnnnnnnkntnnnnnk
nntnnnnknnnnnnnk
nnnnnnnknnnnnnnk
kkkkkkkkkkkkkkkk
nnnktnnnnnnknnnn
nnnknnnnnnnknnnn
ntnknnnntnnknntn
nnnknnnnnnnknnnn
nnnknnnnnntknnnn
nnnknnnnnnnknnnn
nntknnnnnnnknnnn
kkkkkkkkkkkkkkkk
tile wall 16x16 pal=main flags=solid
kkkkkkkkkkkkkkkk
swwwwwmkswwwwwmk
ssssssmkssssssmk
smmmmmmksmmmmmmk
kkkkkkkkkkkkkkkk
wwmkswwwwwmkswww
ssmkssssssmkssss
mmmksmmmmmmksmmm
kkkkkkkkkkkkkkkk
swwwwwmkswwwwwmk
ssssssmkssssssmk
smmmmmmksmmmmmmk
kkkkkkkkkkkkkkkk
wwmkswwwwwmkswww
ssmkssssssmkssss
mmmksmmmmmmksmmm
tile block 16x16 pal=main flags=solid
kkkkkkkkkkkkkkkk
kwwwwwwwwwwwwwsk
kwssssssssssssmk
kwsmmmmmmmmmwsmk
kwsmsssssssswsmk
kwsmsssssssswsmk
kwsmsssssssswsmk
kwsmsssssssswsmk
kwsmsssssssswsmk
kwsmsssssssswsmk
kwsmsssssssswsmk
kwsmwwwwwwwwwsmk
kwssssssssssssmk
ksmmmmmmmmmmmmmk
kddddddddddddddk
kkkkkkkkkkkkkkkk
tile door 16x16 pal=main flags=solid,door
kkkkkkkkkkkkkkkk
kkkkrrrrrrrrkkkk
kkkroooooooorkkk
kkroookookooorkk
kkrmmmmmmmmmmrkk
kkroookookooorkk
kkroookookooorkk
kkroooyyyyooorkk
kkroooykkyooorkk
kkroooyykyooorkk
kkroooyyyyooorkk
kkroookookooorkk
kkroookookooorkk
kkrmmmmmmmmmmrkk
kkroookookooorkk
kkroookookooorkk
tile dooropen 16x16 pal=main flags=exit
kkkkkkkkkkkkkkkk
kkkkrrrrrrrrkkkk
kkkrkkkkkkkkrkkk
kkrkkkkkkkkkkrkk
kkrkkkkkkkkkkrkk
kkrkkkkkkkkkkrkk
kkrkkkkkkkkkkrkk
kkrkkkkkkkkkkrkk
kkrkkkkkkkkkkrkk
kkrddddddddddrkk
kkrddddddddddrkk
kkrmmmmmmmmmmrkk
kkrmmmmmmmmmmrkk
kkrssssssssssrkk
kkrssssssssssrkk
kkrwwwwwwwwwwrkk
map crypt 40x24
legend # wall , floor . floor D door X block
legend @ @hero:floor b @bat:floor K @key:floor
########################################
#......................#...............#
#..@...................#.......b.......#
#......................#...............#
#.......########.......#.......#########
#.......#......#.......#.......#.......#
#.......#..b...#...............#...K...#
#.......#......#...............#.......#
#.......####.###.......#.......#.......#
#......................#.......#.......#
###########.############.......###.#####
#......................#...............#
#......................#...............#
#..X...........b.......#...............#
#......................####.#######....#
#......................#...........#...#
########################...........#...#
#......................#.....b.....#...#
#..................................#...#
#......................#...........#...#
#......................#############...#
#......................................#
#......................................#
#####D##################################
