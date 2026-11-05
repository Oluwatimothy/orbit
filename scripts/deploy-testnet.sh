#!/usr/bin/env bash
# Deploys Orbit to NEAR testnet:
#   1. builds the contracts,
#   2. deploys and initialises the factory,
#   3. uploads the launch contract template to the factory.
#
# Cost (staked as storage on the factory account, not burned): about 0.00001
# NEAR per byte for the factory code (~280 kB) and the template (~407 kB),
# so roughly 7 NEAR. A 10 NEAR faucet account is enough.
#
# Required environment:
#   ORBIT_FACTORY  factory account, e.g. orbit-factory.testnet (must exist, with its key in the keychain)
#   ORBIT_OWNER    account allowed to replace the template / pause the factory (key in the keychain)
#
# Keys are read from the macOS keychain by near-cli-rs; nothing is written to the repository.
set -euo pipefail

: "${ORBIT_FACTORY:?set ORBIT_FACTORY}"
: "${ORBIT_OWNER:?set ORBIT_OWNER}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/contracts/target/near"
FACTORY_WASM="$OUT/orbit_factory/orbit_factory.wasm"
LAUNCH_WASM="$OUT/orbit_launch/orbit_launch.wasm"

"$ROOT/scripts/build-contracts.sh"

echo "→ deploying factory ($(wc -c < "$FACTORY_WASM") bytes) to $ORBIT_FACTORY"
near contract deploy "$ORBIT_FACTORY" use-file "$FACTORY_WASM" \
  with-init-call new json-args "{\"owner_id\":\"$ORBIT_OWNER\"}" \
  prepaid-gas '100 Tgas' attached-deposit '0 NEAR' \
  network-config testnet sign-with-keychain send

echo "→ uploading launch template ($(wc -c < "$LAUNCH_WASM") bytes)"
near contract call-function as-transaction "$ORBIT_FACTORY" set_template file-args "$LAUNCH_WASM" \
  prepaid-gas '300 Tgas' attached-deposit '1 yoctoNEAR' \
  sign-as "$ORBIT_OWNER" network-config testnet sign-with-keychain send

near contract call-function as-read-only "$ORBIT_FACTORY" get_config json-args '{}' network-config testnet now
cat <<DONE

Factory deployed. The template hash above must equal:
  $(node "$ROOT/scripts/code-hash.mjs" "$LAUNCH_WASM")
Set in apps/web/.env.local (and in Vercel):
  NEXT_PUBLIC_ORBIT_FACTORY=$ORBIT_FACTORY
DONE
