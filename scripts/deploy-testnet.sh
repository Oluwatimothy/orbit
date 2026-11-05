#!/usr/bin/env bash
# Deploys Orbit to NEAR testnet:
#   1. builds the contracts,
#   2. publishes token, NFT, claim and sale code as global contracts (by hash),
#   3. deploys and initialises the factory.
#
# Required environment:
#   ORBIT_PUBLISHER  account that pays to publish global code (≈ 0.0001 NEAR/byte, burned)
#   ORBIT_FACTORY    factory account, e.g. orbit-factory.testnet (must exist, with its key in the keychain)
#   ORBIT_OWNER      account allowed to update code hashes / pause the factory
#
# Keys are read from the macOS keychain by near-cli-rs; nothing is written to the repository.
set -euo pipefail

: "${ORBIT_PUBLISHER:?set ORBIT_PUBLISHER}"
: "${ORBIT_FACTORY:?set ORBIT_FACTORY}"
: "${ORBIT_OWNER:?set ORBIT_OWNER}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/contracts/target/near"
NETWORK=(network-config testnet sign-with-keychain send)

"$ROOT/scripts/build-contracts.sh"

hash_of() { node "$ROOT/scripts/code-hash.mjs" "$OUT/$1/$1.wasm"; }

for crate in orbit_fungible_token orbit_nft_collection orbit_claim orbit_sale; do
  echo "→ publishing $crate ($(wc -c < "$OUT/$crate/$crate.wasm") bytes) as a global contract"
  near contract deploy-as-global use-file "$OUT/$crate/$crate.wasm" as-global-hash "$ORBIT_PUBLISHER" "${NETWORK[@]}"
done

TOKEN_HASH="$(hash_of orbit_fungible_token)"
NFT_HASH="$(hash_of orbit_nft_collection)"
CLAIM_HASH="$(hash_of orbit_claim)"
SALE_HASH="$(hash_of orbit_sale)"
INIT_ARGS="{\"owner_id\":\"$ORBIT_OWNER\",\"code\":{\"token\":\"$TOKEN_HASH\",\"nft\":\"$NFT_HASH\",\"claim\":\"$CLAIM_HASH\",\"sale\":\"$SALE_HASH\"}}"

echo "→ deploying factory to $ORBIT_FACTORY"
near contract deploy "$ORBIT_FACTORY" use-file "$OUT/orbit_factory/orbit_factory.wasm" \
  with-init-call new json-args "$INIT_ARGS" prepaid-gas '100 Tgas' attached-deposit '0 NEAR' "${NETWORK[@]}"

near contract call-function as-read-only "$ORBIT_FACTORY" get_config json-args '{}' network-config testnet now
cat <<DONE

Factory deployed. Set in apps/web/.env.local (and in Vercel):
  NEXT_PUBLIC_ORBIT_FACTORY=$ORBIT_FACTORY
Code hashes: token=$TOKEN_HASH nft=$NFT_HASH claim=$CLAIM_HASH sale=$SALE_HASH
DONE
