#!/bin/bash
# Every cart to bundle (web player, Miyoo): carts/*/ first, then the carts in your config's
# `library` dirs (~/.config/kartu/config.toml). One dir per line; a name seen first wins.
cd "$(dirname "$0")/.."
cw=target/release/kartu; [ -x $cw ] || cw=kartu
declare -A seen
{ for c in carts/*/; do echo "${c%/}"; done
  $cw config get library 2>/dev/null | while read -r d; do
    [ -f "$d/main.lua" ] && { echo "$d"; continue; }
    for c in "$d"/*/; do [ -d "$c" ] && echo "${c%/}"; done
  done; } | while read -r c; do
  n=$(basename "$c")
  [ -f "$c/main.lua" ] && [ -f "$c/assets.cw" ] && [ -z "${seen[$n]:-}" ] || continue
  seen[$n]=1; echo "$c"
done
