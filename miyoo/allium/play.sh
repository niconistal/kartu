#!/bin/sh
# Play one cart from the Games list. Roms/Kartu/<Title>.port/launch.sh calls this with the cart's
# folder name; Allium runs a .port folder with its "Native" core (cd into it, exec launch.sh).
# MENU quits back to the list.
app=$(cd "$(dirname "$0")" && pwd)
cart=$1
[ -n "$cart" ] && [ -f "$app/carts/$cart/main.lua" ] || { echo "no cart '$cart'" >"$app/kartu.log"; exit 1; }
/mnt/SDCARD/.tmp_update/script/stop_audioserver.sh >/dev/null 2>&1
cd "$app" && ./kartu play "carts/$cart" >./kartu.log 2>&1
rc=$?
/mnt/SDCARD/.tmp_update/script/start_audioserver.sh >/dev/null 2>&1
exit $rc
