#!/bin/sh
# Kartu for Onion OS: a menu of every cart; MENU in a game returns to the menu, MENU there quits.
# In game: L2 perf HUD · R2 flips the picture. The P0 bench is the last menu entry.
cd "$(dirname "$0")" || exit 1
# Sound: kartu is static, so Onion's libpadsp shim can't route it through audioserver. Free the
# device with Onion's own script (keeps the volume); Onion restarts audioserver before MainUI.
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
