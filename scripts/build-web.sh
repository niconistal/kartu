#!/bin/bash
# Build the WebAssembly player into dist/web (kartu.js + .wasm + index.html + carts).
set -euo pipefail
cd "$(dirname "$0")/.."
source /opt/emsdk/emsdk_env.sh >/dev/null 2>&1
export PATH=$HOME/.cargo/bin:$PATH
# Lua is compiled as C++ here; its exceptions must use the same wasm EH model Rust links with.
export CFLAGS_wasm32_unknown_emscripten="-fwasm-exceptions" CXXFLAGS_wasm32_unknown_emscripten="-fwasm-exceptions"
cargo build --release -p kartu   # native runner: bundle_web.py takes the reference hash from it
cargo build --release --target wasm32-unknown-emscripten -p kartu
out=dist/web
mkdir -p $out
cp target/wasm32-unknown-emscripten/release/kartu.{js,wasm} $out/
scripts/sync-web-carts.sh
ls -la $out
