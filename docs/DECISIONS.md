# Orbit — Decision Log

Each entry records what was decided, why, and what was given up. Later changes are added as new entries that reference the one they replace.

---

### D-001 · Testnet only
**Decision:** every component targets NEAR testnet. The web app refuses any other `NEXT_PUBLIC_NEAR_NETWORK`.
**Why:** this is the project brief; there is no audit and no real value at stake.

### D-002 · Factory with one account per asset, using global contracts (NEP-591)
**Decision:** the factory creates `<slug>-ft`, `<slug>-nft` and `<slug>-claim` subaccounts that reference token, NFT and claim code published once as global contracts (by hash).
**Why:** each asset is an ordinary, independently addressable contract that wallets, explorers and other dApps already understand. Referencing a global contract costs < 0.001 NEAR per account, against ≈ 1–3 NEAR of storage per copy if each launch held its own wasm. Referencing by **hash** (not by account) makes deployed code immutable.
**Alternatives:**
- One shared multi-token contract — rejected: assets would not be standard standalone contracts.
- `include_bytes!` + `deploy_contract` — rejected: per-launch storage cost and a factory too large for one transaction.
**Cost:** publishing burns 0.0001 NEAR/byte (≈ 10 NEAR per 100 kB) once per contract version. The exact amount is recorded after the optimised builds exist. Testnet NEAR for this has to come from faucet accounts.

### D-003 · No access keys on launch accounts
**Decision:** launch accounts are created without keys.
**Why:** nobody — the creator, Orbit or an attacker with a leaked key — can upgrade, re-initialise or drain them. Rules shown on a launch page are the rules that run.
**Trade-off:** bugs cannot be patched in place, and each account's initial storage balance (≈ 0.3–0.5 NEAR) stays locked with it.

### D-004 · Holder allocations are checked at claim time, not by a Merkle snapshot
**Decision:** the claim contract verifies `nft_token(id).owner_id == claimer` by cross-contract call and allows each NFT ID to be claimed once.
**Why:** everything is enforced on-chain with no trusted party. A snapshot needs someone to rebuild ownership at a past block off-chain and publish a Merkle root, which the contract cannot verify — only that a proof matches the root.
**Consequence (disclosed in the UI):** the NFT's current holder claims its allocation. If an NFT is sold before it is claimed, the buyer can claim. Once claimed, it can never be claimed again.
**Deferred:** a Merkle snapshot mode for allocations that must not follow transfers. It would use the Uniswap merkle-distributor pattern and archival RPC, with the trust assumption stated.

### D-005 · Tiers are ID ranges inside the claim contract
**Decision:** a tier is a contiguous NFT ID range with a fixed amount per NFT (max 10 tiers).
**Why:** bounded storage, constant-time lookup, verifiable totals. The factory requires the token allocation to the claim contract to equal `Σ range size × amount` exactly, so a connected launch cannot be under-funded at creation.

### D-006 · No indexer or database in this scope
**Decision:**
- Discovery uses the factory's on-chain registry.
- Activity uses FastNEAR's per-account transaction API, read on demand.
- Portfolio uses FastNEAR holdings, confirmed by RPC view calls.

**Why:** NEAR Lake stopped indexing in March 2026, testnet has no hosted event stream, and NearBlocks' free tier allows 6 requests a minute. A custom neardata-polling worker plus Postgres is a real system to run and secure. It isn't needed for registry-based discovery at hackathon volume.
**Revisit when:** discovery needs sorting or filtering the registry can't support, or activity needs aggregation across many accounts.

### D-007 · NEAR Connect instead of Wallet Selector
**Decision:** `@hot-labs/near-connect` 0.12.0 (pinned exactly) + `near-api-js` 7.4.0.
**Why:** NEAR's own docs and `create-near-app` 10 use it. Wallet Selector is in maintenance mode, and its React hook pins React 18. NEAR Connect has no npm dependencies and loads each wallet in a sandboxed iframe.
**Risk:** 0.x API. Wallet code is isolated in `lib/near/wallet` so a breaking change touches one module.

### D-008 · Always confirm transactions by hash
**Decision:** after signing, the app re-queries the transaction status with receipts (`EXECUTED_OPTIMISTIC`) and treats any failed receipt as a failure.
**Why:** some wallets return an outcome fetched with `wait_until: NONE`. Cross-contract failures (e.g. a failed claim transfer) only show up in later receipts.

### D-009 · Fixed token supply
**Decision:** the token contract mints its whole supply in `new` and has no mint method.
**Why:** this is the simplest property to verify. Anyone can check `ft_total_supply` against the launch page.

### D-010 · Vesting deferred
**Decision:** not in the initial scope.
**Why:** safe vesting is its own contract with time-based release, beneficiary changes and storage questions. The brief puts the core launch experience first. A frontend countdown would be dishonest.

### D-011 · Royalties (NEP-199) deferred
**Decision:** not implemented.
**Why:** `near-contract-standards` 5.30.0 does not provide it. Orbit has no marketplace that would honour payouts, and an untested hand-written implementation would invite exactly the claims the brief forbids.

### D-012 · Pinata with presigned uploads
**Decision:** browser → Pinata via a signed URL from a server route.
**Why:** the key stays server-side, and the upload bypasses Vercel's 4.5 MB function body limit. nft.storage no longer accepts uploads; Storacha's status is unclear.
**Wording:** Orbit says "stored on IPFS (content-addressed, pinned by Pinata)", never "permanent".

### D-013 · Compact left sidebar despite the references using top bars
**Decision:** a 220 px left sidebar on desktop, a bottom tab bar plus sheet on mobile, and a top bar for network and wallet.
**Why:** the brief asks for a sidebar. Hyperliquid and Lighter use top bars because their single trading screen needs the width; Orbit has six distinct sections that benefit from persistent navigation. Density, typography and colour principles are taken from the references.

### D-014 · Radix primitives (`radix-ui`) for accessible components
**Decision:** `radix-ui` 1.8.1. Orbit's components are written on top of it, not generated in bulk.
**Why:** mature focus management and ARIA behaviour for dialogs, menus, tabs and tooltips. Base UI is a reasonable alternative; Radix's API is stable and widely known, which helps reviewers.

### D-015 · No JS monorepo tooling
**Decision:** `apps/web` is a standalone npm project. The Rust workspace lives in `contracts/`. No `packages/*` until a second consumer of shared TypeScript exists.
**Why:** only one TypeScript application exists. Shared Rust validation lives in a `common` crate used by every contract.

### D-016 · Global contract publishing cost, measured
**Decision:** keep NEP-591 global contracts (D-002) after measuring the optimised builds.
**Measured (cargo-near 0.22, `opt-level = "z"`, LTO):**
| Contract | Size |
|---|---|
| Token | 179,961 B |
| NFT | 282,335 B |
| Claim | 217,531 B |
| Factory | 279,897 B (ordinary deploy, ≈ 2.8 NEAR storage) |

Publishing the three templates burns ≈ 68 testnet NEAR once.
**Why still worth it:** without global code, a connected launch would need ≈ 6.8 NEAR of storage per creator. A judge with 10 faucet NEAR could afford one launch. With global code a connected launch costs 0.68 NEAR.
**Follow-up:** reducing wasm size (fewer `serde_json` paths, no embedded ABI) is a Phase 6 task.

### D-017 · Activity from FastNEAR's transactions API
**Decision:** activity tables query `POST https://tx.test.fastnear.com/v0/account` for the relevant accounts, then `POST /v0/transactions` for signer, method and receipt logs.
**Verified 2026-11-05:** both endpoints answer on testnet without an API key and return `Access-Control-Allow-Origin` for the requesting origin.
**Trade-off:** FastNEAR lists only executed transactions. Pending transactions appear in the transaction toast until confirmed. If the index is unavailable, the table shows an error, not an empty list.

### D-018 · Portfolio reads balances from the registry's contracts (amends D-006)
**Decision:** portfolio iterates live launches in the registry and calls `ft_balance_of` / `nft_tokens_for_owner` directly, six requests at a time. FastNEAR holdings are not used.
**Why:** no third-party list is trusted for holdings, and the registry is small at hackathon scale.
**Revisit when:** the registry grows past a few hundred launches. Then use FastNEAR holdings to choose which contracts to read.

### D-019 · Sandbox-only global-contract publisher
**Decision:** `contracts/test-support/global-publisher` publishes code via `Promise::deploy_global_contract` in integration tests.
**Why:** `near-workspaces` 0.24 has no API for the `DeployGlobalContract` action. The helper is never deployed to testnet; testnet publishing uses `near contract deploy-as-global`.

### D-020 · Dark theme only
**Decision:** one dark theme.
**Why:** the references (Hyperliquid, Lighter) are dark-first. A second theme doubles the visual QA surface for little benefit in this scope. Contrast checks were made against the dark tokens.

### D-021 · NFT-gated sale as a separate contract (supersedes "Phase 4b planned")
**Decision:** `contracts/sale` implements the second relationship type:
- a holder window with a per-NFT limit, ownership verified by cross-contract call at purchase time;
- an optional public window with a per-account limit;
- a required end date, after which the creator can reclaim unsold tokens.

A connected launch takes `claim?` and `sale?`, at least one required. The token must allocate each one's reserve exactly.
**Why:** each mechanism stays small and independently testable. Reservations are made before any async call, so limits and supply hold under concurrent purchases. The creator is paid only after delivery.
**Arithmetic:** `amount × price` exceeds u128 at realistic scales (10^18 × 10^24), so cost uses 256-bit integers (`uint` 0.10) and rounds up.

### D-022 · Contract size: optimisation attempted
**Finding:** `--no-embed-abi` saved about 2 kB. An extra `wasm-opt -Oz` pass saved under 1%, because `cargo near` already runs `wasm-opt -O`. The rest is the JSON and SDK code every NEAR contract carries.
**Decision:** accept the publishing cost of about 91 testnet NEAR (four templates). It is a one-time cost on testnet. Each launch's cost stays below 0.8 NEAR.

### D-023 · Redirect-based wallet results are confirmed, not trusted
**Decision:** on load, `?transactionHashes=` and `?errorCode=` are parsed (only valid base58 hashes are accepted) and each hash is confirmed by RPC before a result is shown. The parameters are then removed from the URL.

### D-024 · Explore mint-state filter reads contracts on demand
**Decision:** the "Minting now / Not minting" filter calls `orbit_config` and `orbit_mint_stats` on each live collection, and only while the filter is active. This keeps the registry the only list and the contracts the only source of mint state.

### D-025 · Storage is always paid by whoever causes it (from the contract review)
**Decision:**
- The sale and NFT contracts measure the storage each call adds and keep it from the caller's deposit.
- Sale records that drop to zero are deleted.
- Each mint keeps a 0.0025 NEAR per-token reserve for the owner sets that transfers create (the standard frees empty sets, so sets never outnumber tokens).
- Rejected claims and purchases keep 0.001 NEAR.

**Why:** a keyless contract that pays for storage it cannot recover can be drained until outgoing refunds fail, which would lock funds permanently. The fees make griefing cost something without affecting honest users, and are disclosed in the UI.
**Related:** the deploy callback now reserves 80 Tgas; the registry allowance in quotes is 0.1 NEAR. Quotes are 0.35 NEAR (token), 0.40 NEAR (collection) and up to 0.85 NEAR (connected), with unused parts refunded.
