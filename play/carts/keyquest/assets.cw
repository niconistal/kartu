-- KEY QUEST assets. Palette "main" is first => text colours:
-- 1 k ink, 2 p plum, 3 r red, 4 o orange, 5 y yellow, 6 l lime, 7 g green, 8 t teal,
-- 9 n navy, 10 b blue, 11 c cyan, 12 w white, 13 s silver, 14 m slate, 15 d dusk
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

-- ---------- hero ----------
sprite knight1 16x16 pal=main
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
sprite knight2 16x16 pal=main
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
clip knight_walk fps=8 knight1 knight2

sprite sword 14x5 pal=main
.k............
kyk.kkkkkkkkk.
kykkwwwwwwwwwk
kyk.kkkkkkkkk.
.k............

sprite swordv 5x14 pal=main
..k..
.kwk.
.kwk.
.kwk.
.kwk.
.kwk.
.kwk.
.kwk.
.kwk.
.kwk.
kkkkk
.kyk.
.kyk.
..k..

-- ---------- bat ----------
sprite bat1 16x16 pal=bat
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
sprite bat2 16x16 pal=bat
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

-- ---------- items / fx ----------
sprite key 16x16 pal=main
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
sprite flame1 8x8 pal=main
...y....
..yoy...
..yooy..
.yorroy.
.yorroy.
..yooy..
...kk...
...kk...
sprite flame2 8x8 pal=main
....y...
...yoy..
..yooy..
.yooroy.
.yorroy.
..yooy..
...kk...
...kk...
clip flame fps=6 flame1 flame2
sprite puff1 16x16 pal=main
................
................
................
................
......ww........
.....wssw.......
....wsswsw......
....wswwsw..w...
.....wssw.......
......ww........
................
..w.............
................
................
................
................
sprite puff2 16x16 pal=main
................
.....w....w.....
..w..........w..
................
.w....s..s....w.
.....s....s.....
................
w..s........s..w
................
.....s....s.....
.w....s..s....w.
................
..w..........w..
.....w....w.....
................
................
clip puff puff1 puff2 fps=8
sprite shade 64x16 pal=main
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk

-- ---------- tiles ----------
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
tile wall 16x16 pal=main
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
tile torch 16x16 pal=main
kkkkkkkkkkkkkkkk
swwwwwmkswwwwwmk
ssssssmkssssssmk
smmmmmmksmmmmmmk
kkkkkkkkkkkkkkkk
wwmkswwwwwmkswww
ssmkssskkmmkssss
mmmksmkmmkmksmmm
kkkkkkkmmkkkkkkk
swwwwwkmmkwwwwmk
ssssssmkkmssssmk
smmmmmmksmmmmmmk
kkkkkkkkkkkkkkkk
wwmkswwwwwmkswww
ssmkssssssmkssss
mmmksmmmmmmksmmm
tile block 16x16 pal=main
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
tile door 16x16 pal=main
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
tile dooropen 16x16 pal=main
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

map blank 20x15
legend # wall
....................
....................
....................
....................
....................
....................
....................
....................
....................
....................
....................
....................
....................
....................
....................
