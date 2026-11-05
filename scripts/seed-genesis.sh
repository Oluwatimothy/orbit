#!/usr/bin/env bash
# Creates the ORBIT GENESIS demo through a real testnet transaction: a 2,000-NFT
# collection, a 10,000,000 ORBIT token, holder claims of 1,000 ORBIT per NFT
# (2,000,000 reserved) and a 30-day holder sale of 500,000 ORBIT.
#
# Required:
#   ORBIT_FACTORY  the deployed factory
#   ORBIT_CREATOR  creator account (key in the macOS keychain)
#   ORBIT_COVER    ipfs:// (preferred) or https:// URL of assets/genesis-cover.svg after uploading it
set -euo pipefail
: "${ORBIT_FACTORY:?set ORBIT_FACTORY}"
: "${ORBIT_CREATOR:?set ORBIT_CREATOR}"
: "${ORBIT_COVER:?upload assets/genesis-cover.svg to IPFS and set ORBIT_COVER=ipfs://<cid>}"

E18="000000000000000000"
NOW_NS="$(node -e 'console.log((BigInt(Date.now()) * 1000000n).toString())')"
END_NS="$(node -e 'console.log(((BigInt(Date.now()) + 30n * 86400000n) * 1000000n).toString())')"
CLAIM="genesis-claim.$ORBIT_FACTORY"
SALE="genesis-sale.$ORBIT_FACTORY"

ARGS=$(cat <<JSON
{
  "slug": "genesis",
  "description": "ORBIT GENESIS: 2,000 Genesis passes. Each pass claims 1,000 ORBIT and can buy up to 250 ORBIT in the holder sale.",
  "token": {
    "name": "Orbit Genesis", "symbol": "ORBIT", "decimals": 18, "icon": null,
    "total_supply": "10000000$E18",
    "allocations": [
      { "account_id": "$ORBIT_CREATOR", "amount": "7500000$E18", "label": "Creator treasury" },
      { "account_id": "$CLAIM", "amount": "2000000$E18", "label": "NFT holder claims" },
      { "account_id": "$SALE", "amount": "500000$E18", "label": "NFT holder sale" }
    ]
  },
  "collection": {
    "name": "Orbit Genesis Pass", "symbol": "GENESIS", "description": "The Orbit Genesis pass.",
    "cover_media": "$ORBIT_COVER",
    "media": { "kind": "shared", "media": "$ORBIT_COVER", "media_hash": null },
    "supply": { "kind": "fixed", "max": 2000 },
    "price": "10000000000000000000000",
    "starts_at_ns": "0", "ends_at_ns": null, "max_per_wallet": 5, "public_mint": true
  },
  "claim": {
    "tiers": [{ "name": "Genesis", "from": 1, "to": 2000, "amount_per_nft": "1000$E18" }],
    "starts_at_ns": "0"
  },
  "sale": {
    "price_per_token": "1000000000000000000000",
    "total_for_sale": "500000$E18",
    "per_nft_limit": "250$E18",
    "holder_starts_at_ns": "$NOW_NS",
    "public_starts_at_ns": null,
    "per_account_public_limit": null,
    "ends_at_ns": "$END_NS"
  }
}
JSON
)

near contract call-function as-transaction "$ORBIT_FACTORY" create_connected_launch json-args "$ARGS" \
  prepaid-gas '300 Tgas' attached-deposit '0.9 NEAR' sign-as "$ORBIT_CREATOR" network-config testnet sign-with-keychain send
