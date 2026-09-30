#!/bin/bash
# Install Kartu on the Miyoo, then (Onion only) run the P0 gate benchmark on the device and fetch
# the numbers. Works out which OS the card runs: Allium (a .allium folder on the card, root login
# with no password) gets dist/allium; Onion (onion/onion) gets dist/miyoo.
#
#   DEVICE_IP=192.168.x.y scripts/deliver-miyoo.sh   # device awake + on wifi
#   (or once, in ~/.config/kartu/config.toml:  [device.miyoo]  ip = "192.168.x.y")
#   WAIT_MIN=240 scripts/deliver-miyoo.sh    # sit and wait for it to wake
#   BENCH=0 scripts/deliver-miyoo.sh         # install only
#
# The bench pauses Onion's MainUI (SIGSTOP) so we own the framebuffer for ~20 s, runs
# `kartu play --bench-secs 20`, then resumes it. Results: dist/miyoo/bench-<ts>.txt
set -u
cd "$(dirname "$0")/.."
IP=${DEVICE_IP:-$(target/release/kartu config get device.miyoo.ip 2>/dev/null || kartu config get device.miyoo.ip 2>/dev/null)}
: "${IP:?set DEVICE_IP (or [device.miyoo] ip in ~/.config/kartu/config.toml) to the Miyoo IP address}"
USER=${DEVICE_USER:-}
PASS=${DEVICE_PASS:-}
SECS=${BENCH_SECS:-20}
say() { echo "[kartu] $(date +%H:%M:%S) $*"; }
SSHO=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=15 -o ServerAliveInterval=5)
dev() { SSHPASS=$PASS sshpass -e ssh "${SSHO[@]}" "$USER@$IP" "$@"; }

[ -x dist/miyoo/Kartu/kartu ] || { say "build first: scripts/build-miyoo.sh"; exit 1; }
WAIT_MIN=${WAIT_MIN:-0}
deadline=$(( $(date +%s) + WAIT_MIN * 60 ))
login() {  # Allium first (root, empty password), then Onion (onion/onion)
  if [ -n "$USER" ]; then dev true 2>/dev/null; return; fi
  for cred in root: onion:onion; do
    USER=${cred%%:*} PASS=${cred#*:}
    dev true 2>/dev/null && return 0
  done
  USER= PASS=; return 1
}
until ping -c2 -W3 "$IP" >/dev/null 2>&1 && login; do
  [ "$(date +%s)" -lt "$deadline" ] || { say "device $IP unreachable — wake it and join wifi"; exit 1; }
  sleep 20
done
say "device up: $(dev 'uname -m; cat /proc/cpuinfo | grep -m1 -i "model name\|Processor"' | tr '\n' ' ')"

if dev '[ -d /mnt/SDCARD/.allium ]'; then
  [ -x dist/allium/Apps/Kartu.pak/kartu ] || { say "build first: scripts/build-miyoo.sh"; exit 1; }
  APP=/mnt/SDCARD/Apps/Kartu.pak
  say "Allium $(dev 'cat /mnt/SDCARD/.allium/version.txt' | tr -d '\r\n')"
  for try in 1 2 3 4; do
    # Apps/Kartu.pak (player, carts, menu) + Roms/Kartu/<Title>.port (the Games list) + box art.
    # Old .port folders go first so renamed carts don't linger.
    if dev "rm -rf /mnt/SDCARD/Roms/Kartu/*.port /mnt/SDCARD/Roms/Kartu/Imgs" &&
       tar -C dist/allium -cf - Apps Roms | dev "tar -C /mnt/SDCARD -xf - && sync"; then ok=1; break; fi
    ok=0; say "copy failed (try $try), retrying"; sleep $((try * 4))
  done
  [ "$ok" = 1 ] || { say "copy failed"; exit 1; }
  say "installed at $APP ($(dev "$APP/kartu version"))"
  say "open Apps → Kartu for the cart menu (the bench is its last entry), or Games → Kartu"
  exit 0
fi
APP=/mnt/SDCARD/App/Kartu

for try in 1 2 3 4; do
  # App/Kartu (player, carts, menu) + the Games-tab console (Emu/KARTU, Roms/KARTU/*.kartu).
  # Stale .kartu files and Onion's game-list cache go first so renamed carts don't linger.
  if tar -C dist/miyoo -cf - Kartu | dev "mkdir -p /mnt/SDCARD/App && tar -C /mnt/SDCARD/App -xf -" &&
     dev "rm -f /mnt/SDCARD/Roms/KARTU/*.kartu /mnt/SDCARD/Roms/KARTU/*cache*.db" &&
     tar -C dist/miyoo -cf - Emu Roms | dev "tar -C /mnt/SDCARD -xf - && sync"; then ok=1; break; fi
  ok=0; say "copy failed (try $try), retrying"; sleep $((try * 4))
done
[ "$ok" = 1 ] || { say "copy failed"; exit 1; }
say "installed at $APP ($(dev "$APP/kartu version"))"

[ "${BENCH:-1}" = 1 ] || exit 0
# Only take the screen over when he's sitting at the Onion menu, never mid-game.
busy=$(dev 'pidof retroarch ra32.miyoo drastic 2>/dev/null; ps | grep -v grep | grep -c "App/.*launch.sh" | grep -v "^0$"' | tr -d '\n ')
if [ -n "$busy" ] || [ -z "$(dev 'pidof MainUI')" ]; then
  say "device busy (a game or app is running): installed only; open Apps → Kartu to bench"
  exit 0
fi
ts=$(date +%Y%m%d-%H%M%S)
say "running ${SECS}s on-device bench (screen shows the gate scene, then stress)"
dev "cd $APP && pids=\$(pidof MainUI); [ -n \"\$pids\" ] && kill -STOP \$pids; \
     ./kartu play carts/bench --bench-secs $SECS --out bench.txt > bench.log 2>&1; rc=\$?; \
     [ -n \"\$pids\" ] && kill -CONT \$pids; echo rc=\$rc; cat bench.txt; echo ---; tail -20 bench.log" | tee "dist/miyoo/bench-$ts.txt"
say "saved dist/miyoo/bench-$ts.txt"
