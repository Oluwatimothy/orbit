#!/usr/bin/env bash
# Builds every Orbit contract to contracts/target/near/<crate>/<crate>.wasm.
# Requires cargo-near 0.22 (it runs wasm-opt, which NEAR needs for rustc >= 1.82).
set -euo pipefail

cd "$(dirname "$0")/../contracts"
for crate in fungible-token nft-collection claim sale factory test-support/global-publisher; do
  echo "→ building $crate"
  (cd "$crate" && cargo near build non-reproducible-wasm --locked)
done
ls -l target/near/*/*.wasm
