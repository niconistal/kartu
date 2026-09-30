#!/bin/sh
# Kartu for Allium (Apps/Kartu.pak): a menu of every cart; MENU in a game returns to the menu,
# MENU there quits. In game: L2 perf HUD · R2 flips the picture. The P0 bench is the last entry.
cd "$(dirname "$0")" || exit 1
# Sound: kartu is static and writes /dev/dsp itself, so free it from the audioserver Allium runs
# for its RetroArch cores, and put the audioserver back on the way out (Allium's ffplay core does
# the same dance).
/mnt/SDCARD/.tmp_update/script/stop_audioserver.sh >/dev/null 2>&1
while :; do
  rm -f .pick
  ./kartu play menu --pick-out .pick >./kartu.log 2>&1
  [ -s .pick ] || break
  cart=$(cat .pick)
  if [ "$cart" = bench ]; then
    ./kartu play carts/bench --bench-secs 20 --then-play --out ./bench.txt >>./kartu.log 2>&1
  else
    ./kartu play "carts/$cart" >>./kartu.log 2>&1
  fi
done
/mnt/SDCARD/.tmp_update/script/start_audioserver.sh >/dev/null 2>&1
exit 0
