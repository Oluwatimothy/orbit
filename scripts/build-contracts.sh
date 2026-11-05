#!/usr/bin/env bash
# Builds the Orbit contracts to contracts/target/near/<crate>/<crate>.wasm.
# Requires cargo-near 0.22 (it runs wasm-opt, which NEAR needs for rustc >= 1.82).
# The ABI is written next to each wasm but not embedded, which keeps the code
# (and the storage every launch pays for) smaller.
set -euo pipefail

cd "$(dirname "$0")/../contracts"
for crate in launch factory; do
  echo "→ building $crate"
  (cd "$crate" && cargo near build non-reproducible-wasm --locked --no-embed-abi)
done
ls -l target/near/orbit_launch/orbit_launch.wasm target/near/orbit_factory/orbit_factory.wasm
