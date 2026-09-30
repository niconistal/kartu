#!/bin/sh
# Kartu for Allium (Apps/Kartu.pak): a menu of every cart; MENU in a game returns to the menu,
# MENU there quits. In game: L2 perf HUD · R2 flips the picture. The P0 bench is the last entry.
cd "$(dirname "$0")" || exit 1
# Sound: see sound.sh (system volume applied, nothing touched when muted).
. ./sound.sh
sound_begin
while :; do
  rm -f .pick
  ./kartu play menu --pick-out .pick $KARTU_SOUND >./kartu.log 2>&1
  [ -s .pick ] || break
  cart=$(cat .pick)
  if [ "$cart" = bench ]; then
    ./kartu play carts/bench --bench-secs 20 --then-play --out ./bench.txt $KARTU_SOUND >>./kartu.log 2>&1
  else
    ./kartu play "carts/$cart" $KARTU_SOUND >>./kartu.log 2>&1
  fi
done
sound_end
exit 0
