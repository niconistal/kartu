#!/bin/bash
# Every cart: `check`, then prove it still wins: win.txt replay and bot.txt plan, each --until WIN.
set -uo pipefail
cd "$(dirname "$0")/.."
cw=target/release/kartu
fail=0
for c in carts/*/; do
  c=${c%/}
  if ! out=$($cw check "$c" 2>&1); then echo "FAIL check $c"; echo "$out" | sed 's/^/    /'; fail=1; continue; fi
  if [ -f "$c/win.txt" ]; then
    if r=$($cw run "$c" --frames 20000 --script "$c/win.txt" --until WIN 2>&1); then
      echo "ok   $c  $(echo "$r" | grep '^until:')"
    else
      echo "FAIL win  $c  $(echo "$r" | tail -1)"; fail=1
    fi
  else
    echo "ok   $c  (checked; no win.txt)"
  fi
  if [ -f "$c/bot.txt" ]; then
    if r=$($cw run "$c" --frames 20000 --bot "$c/bot.txt" --until WIN 2>&1); then
      echo "ok   $c  bot $(echo "$r" | grep '^until:')"
    else
      echo "FAIL bot  $c  $(echo "$r" | grep -E 'FAILED|NOT' | tail -1)"; fail=1
    fi
  fi
done
exit $fail
