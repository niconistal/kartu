#!/bin/sh
# Onion "Kartu" console (Games tab). A ROM is Roms/KARTU/<Title>.kartu holding one line: the cart's
# folder name under App/Kartu/carts (the player + carts live in the App). MENU quits to the list.
unset LD_PRELOAD # Onion preloads libpadsp for games; it could hold the audio device open
app=/mnt/SDCARD/App/Kartu
cart=$(head -n1 "$1" | tr -d '\r\n ')
[ -n "$cart" ] && [ -f "$app/carts/$cart/main.lua" ] || { echo "no cart '$cart' for $1" >"$app/kartu.log"; exit 1; }
# Sound: free /dev/dsp from Onion's audioserver (see App/Kartu/launch.sh); Onion restarts it after.
/mnt/SDCARD/.tmp_update/script/stop_audioserver.sh >/dev/null 2>&1
cd "$app" && ./kartu play "carts/$cart" >./kartu.log 2>&1
