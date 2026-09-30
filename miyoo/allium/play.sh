#!/bin/sh
# Play one cart from the Games list. Roms/Kartu/<Title>.port/launch.sh calls this with the cart's
# folder name; Allium runs a .port folder with its "Native" core (cd into it, exec launch.sh).
# MENU quits back to the list.
app=$(cd "$(dirname "$0")" && pwd)
cart=$1
[ -n "$cart" ] && [ -f "$app/carts/$cart/main.lua" ] || { echo "no cart '$cart'" >"$app/kartu.log"; exit 1; }
cd "$app" || exit 1
# Sound: see sound.sh (system volume applied, nothing touched when muted).
. ./sound.sh
sound_begin
./kartu play "carts/$cart" $KARTU_SOUND >./kartu.log 2>&1
rc=$?
sound_end
exit $rc
