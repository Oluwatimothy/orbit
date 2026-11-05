# Orbit — Architecture

Status: **implemented and tested locally** (2026-11-05). Not yet deployed to testnet.

## 1. System overview

```
                    ┌──────────────────────────────────────────────┐
 Browser            │ Next.js app (apps/web)                       │
 ─────────          │  UI ─ features ─ lib/near (typed clients)    │
 NEAR Connect ◄────►│  wallet context, tx lifecycle, validation    │
 (wallet iframe)    └──────┬──────────────────────┬────────────────┘
                           │ view calls / tx status│ /api/uploads/sign
                           ▼                       ▼
                ┌────────────────────┐   ┌──────────────────┐
                │ NEAR testnet RPC   │   │ Pinata (IPFS)    │
                │ FastNEAR, dRPC     │   │ presigned upload │
                └─────────┬──────────┘   └──────────────────┘
                          ▼
 ┌────────────────────────────────────────────────────────────────┐
 │ orbit-factory.testnet  (launch registry, deploys launches)     │
 │   ├─ <slug>-ft.orbit-factory.testnet     NEP-141/145/148       │
 │   ├─ <slug>-nft.orbit-factory.testnet    NEP-171/177/178/181   │
 │   ├─ <slug>-claim.orbit-factory.testnet  holder claims         │
 │   └─ <slug>-sale.orbit-factory.testnet   NFT-gated sale        │
 │ Global contract code (NEP-591), referenced by hash             │
 └────────────────────────────────────────────────────────────────┘
```

Where each responsibility lives:

| Responsibility | Location | Why |
|---|---|---|
| Balances, ownership, supply caps, claim rules | Contracts | Must be enforced, not displayed |
| Launch registry and discovery | Factory contract (paginated views) | Source of truth; no indexer to operate |
| Configuration validation | Contracts (authoritative) **and** browser (immediate feedback) | Shared rules are mirrored in `apps/web/src/lib/validation` and in each contract's `new`; contract tests and web tests cover the same cases |
| Transaction signing | User's wallet only | Orbit holds no user keys |
| Media upload signing | Next.js API route | Keeps the Pinata key server-side |
| Activity history | FastNEAR transaction API, read on demand | No indexer in this scope (D-006) |
| Portfolio discovery | FastNEAR account holdings API, confirmed by RPC view calls | Third-party lists are hints; balances come from the contracts |

There is **no database** in this scope. Nothing off-chain is a source of truth.

## 2. Contracts

Rust workspace at `contracts/`, `near-sdk` 5.30.0, `near-contract-standards` 5.30.0, built with `cargo near`.

### 2.1 Factory (`contracts/factory`)

Deployed once at `orbit-factory.testnet` (name final at deployment).

**State**
- `owner_id` — may update the code hashes used for **future** launches and pause new launches. Cannot touch deployed launches (they have no access keys and reference immutable code by hash).
- `code_hashes { token, nft, claim }` — NEP-591 global contract hashes.
- `launches: Vector<Launch>` — append-only registry; `id` = index.
- `launches_by_creator: LookupMap<AccountId, Vector<u64>>`.
- `slugs: LookupSet<String>` — reserves a slug before deployment, so two launches can never target the same accounts.

**Launch record**: `id`, `kind` (`Token | Collection | Connected`), `slug`, `creator`, `created_at_ns`, `status` (`Deploying | Live | Failed`), `token`, `nft`, `claim` (optional account IDs).

**Change methods** (all `#[payable]`):
- `create_token_launch(slug, token)`
- `create_collection_launch(slug, collection)`
- `create_connected_launch(slug, description, token, collection, claim?, sale?)` (at least one of claim / sale)

**Views**: `get_launch(id)`, `get_launches(from_index, limit)` (limit ≤ 50), `get_launches_by_creator(account_id, from_index, limit)`, `get_launch_count()`, `quote_deposit(kind)`, `get_config()`.

**Deployment flow**
1. Validate the slug (2–20 chars, `[a-z0-9-]`, no leading/trailing/double hyphen) and every init argument with the **same** validation functions the asset contracts use (shared crate `orbit-common`).
2. Reserve the slug, append the launch with status `Deploying`, measure the registry storage it used.
3. Require `attached ≥ registry storage + Σ initial balances of new accounts`; refund the excess immediately.
4. For each asset, one batch receipt: `create_account → transfer(initial balance) → use_global_contract(hash) → function_call("new", args)`. Actions in one receipt are atomic: if `new` panics, the account is not created and the transferred balance returns to the factory.
5. Combine the batches with `.and()` and attach one callback, `on_launch_deployed(id, …)`.
6. The callback reads every promise result. All succeeded → `Live`; for a connected launch it then calls `activate()` on the claim and/or sale contract, each of which checks its own token balance before opening. Any failed → `Failed`, the slug stays reserved (accounts may exist) and the creator is refunded the balances of the accounts that were not created.

No access keys are added to launch accounts. They cannot be upgraded, drained or re-initialised by anyone, including Orbit. Their initial balance pays for their own storage and is locked with them.

Factory-side validation makes a partial failure in a connected launch very unlikely (the only remaining causes are gas exhaustion or protocol errors). If it happens, the record is `Failed` and the UI states which accounts exist.

### 2.2 Fungible token (`contracts/fungible-token`)

Fixed supply. **No mint method exists after initialisation.**

`new(metadata, total_supply, allocations, creator_id, launch_id)`:
- `metadata.assert_valid()`; `decimals ≤ 24`; symbol 2–12 uppercase alphanumerics; icon is a `data:image/(svg+xml|png|webp);base64,` URI ≤ 8 KiB **or** an `ipfs://` / `https://` URL.
- 1–10 allocations; amounts > 0; no duplicate accounts; `Σ amounts == total_supply` with checked `u128` addition.
- Registers each recipient (storage paid by the contract's own balance), credits it and emits one NEP-297 `ft_mint` per allocation.

Implements NEP-141 (`ft_transfer`, `ft_transfer_call`, `ft_total_supply`, `ft_balance_of`, `ft_resolve_transfer`), NEP-145 (`storage_deposit`, `storage_withdraw`, `storage_unregister`, `storage_balance_bounds`, `storage_balance_of`), NEP-148 (`ft_metadata`) and NEP-330 (`contract_source_metadata`). Orbit views: `orbit_allocations()`, `orbit_info()`.

### 2.3 NFT collection (`contracts/nft-collection`)

`new(owner_id, metadata, config)`; `config`:
- `supply`: `Fixed(n)` (1 ≤ n ≤ 10,000) or `Unlimited` — explicit, never implied by absence.
- `price` (yoctoNEAR, may be 0), `starts_at_ns`, optional `ends_at_ns`, optional `max_per_wallet`, `public_mint: bool`.
- `media`: `Shared { media, media_hash? }` (every token shows the collection artwork) or `Directory { extension }` (token `n` uses `{base_uri}/{n}.{extension}`, per NEP-177 `base_uri`).

Token IDs are sequential decimal strings starting at `"1"`; duplicates are impossible by construction.

`nft_mint(quantity)` (payable, `1 ≤ quantity ≤ 10`): checks public mint, time window, supply cap, wallet limit; mints; measures storage used; requires `attached ≥ price × quantity + storage cost`; refunds the excess; transfers `price × quantity` to the owner. Any failed check panics, so the whole receipt (and the attached deposit) reverts.

`creator_mint(receiver_id, quantity)` — owner only, same caps; the owner pays storage.

`set_paused(bool)` — owner only. Price, supply and timing are **immutable** after initialisation.

Implements NEP-171, 177, 178, 181, 297, 330. Views: `orbit_mint_config()`, `orbit_mint_stats()`, `orbit_minted_by(account_id)`.

Royalties (NEP-199) are not implemented (D-011).

### 2.4 Holder claim (`contracts/claim`)

The connected-launch distribution contract. Eligibility is **checked against live ownership at claim time**; each NFT ID can be claimed once (D-004).

`new(token_id, nft_id, creator_id, tiers, claim_starts_at_ns)`:
- 1–10 tiers, each `{ name, from, to, amount_per_nft }` with `1 ≤ from ≤ to ≤ nft supply`, ranges non-overlapping, `amount_per_nft > 0`.
- `total_reserved = Σ (to − from + 1) × amount_per_nft` (checked). The factory sets the token allocation to the claim contract equal to `total_reserved`.

Status: `AwaitingFunding → Active`. `activate()` (callable by anyone) reads `ft_balance_of(self)` on the token and activates only if `balance ≥ total_reserved`.

`claim(nft_token_id)` (payable, attach `CLAIM_DEPOSIT` = 0.005 NEAR — covers token storage registration and the claim record):
1. Require `Active`, start time reached, the ID inside a tier, and no existing record for it.
2. Record `Pending { claimer }`; add the amount to `pending_total`.
3. `nft.nft_token(id)` → `on_owner_checked`.
4. Owner ≠ claimer or call failed → delete the record, subtract `pending_total`, refund the deposit, emit `claim_rejected`.
5. Owner = claimer → `token.storage_deposit(claimer, registration_only)` → `token.ft_transfer(claimer, amount)` → `on_transferred`.
6. Transfer succeeded → `Claimed { claimer, amount, at }`, move the amount from `pending_total` to `claimed_total`, emit `claim`.
7. Transfer failed → delete the record, subtract `pending_total`, refund the deposit minus what the storage registration used, emit `claim_failed`.

Why this is safe:
- A `Pending` record blocks a concurrent second claim of the same NFT.
- `Claimed` is written only after the transfer result is known.
- Claims are keyed by NFT ID, so transferring an NFT to another wallet does not make it claimable again.
- The contract has no other method that moves tokens, so `claimed + pending + unclaimed ≤ total_reserved ≤ balance` always holds.

Views: `get_config()`, `get_stats()`, `get_claim(nft_token_id)`, `get_claims(nft_token_ids)` (≤ 50).

### 2.5 NFT-gated sale (`contracts/sale`)

The second relationship type. The creator funds `total_for_sale` (an exact token allocation to `<slug>-sale`, enforced by the factory). Buyers pay a fixed NEAR price per whole token; cost rounds up and is computed in 256-bit arithmetic.

| Window | Who can buy | Limit |
|---|---|---|
| Holder (`holder_starts_at_ns` → public start or end) | The current owner of the NFT ID passed to `buy` | `per_nft_limit` per NFT ID |
| Public (optional, `public_starts_at_ns` → `ends_at_ns`) | Anyone | `per_account_public_limit` per account |

`buy(amount, nft_token_id?)`:
1. Checks the phase, the remaining supply and the deposit (cost + 0.005 NEAR).
2. Reserves the amount against the NFT or account counter and `pending` **before** any cross-contract call, so concurrent purchases cannot exceed limits or supply.
3. In the holder window, asks the NFT contract who owns the NFT and rolls back with a full refund if the caller is not the owner.
4. Registers the buyer with the token and transfers the amount.
5. On success, moves the amount from `pending` to `sold`, pays the creator the cost and refunds the change. On failure, releases the reservation and refunds.

After `ends_at_ns`, with nothing pending, the creator can `withdraw_unsold()`; a failed withdrawal re-opens it. A connected launch has holder claims, a sale, or both.

### 2.6 Events

Standard events come from `near-contract-standards`. Orbit-specific events use NEP-297 with `standard: "orbit"`, `version: "1.0.0"`: `launch_created`, `launch_live`, `launch_failed`, `claim`, `claim_rejected`, `claim_failed`, `claim_activated`, `purchase`, `purchase_rejected`, `purchase_failed`, `sale_activated`, `sale_closed`.

### 2.7 Code upgrades

Launch contracts are not upgradeable (no keys, code referenced by immutable hash). The factory owner can point **future** launches at new code hashes. The factory itself is deployed from a full-access key held by the project owner — a disclosed trust assumption (see THREAT-MODEL).

## 3. Web application (`apps/web`)

```
apps/web/src/
  app/                    routes (App Router)
    (app)/                shell layout: sidebar + top bar
      page.tsx            overview
      explore/  create/{token,collection,connected}/  launches/[id]/
      portfolio/  activity/  docs/
    api/uploads/sign/     Pinata presigned URL (server only)
  components/ui/          primitives (button, input, dialog, table, …)
  components/layout/      sidebar, top bar, mobile navigation
  features/               wallet, create, launch, mint, claim, portfolio, activity
  lib/near/               network config, RPC provider, typed contract clients,
                          transaction lifecycle, explorer links
  lib/units/              bigint token-unit parsing and formatting
  lib/validation/         zod schemas mirroring contract rules
```

Rules:
- UI components never call RPC or the wallet directly; they use feature hooks, which use `lib/near`.
- All on-chain amounts are `bigint` in TypeScript and decimal strings on the wire.
- Every data flow has loading, empty, error and disconnected states.

### Transaction lifecycle

`idle → validating → review → awaiting-signature → submitted → confirmed | failed`

1. Validate with the zod schema.
2. Show a review dialog: method, contract, deposit, gas, what it does.
3. Ask the wallet to sign.
4. Read the hash from the outcome (or `?transactionHashes=` after a redirect).
5. Re-query `viewTransactionStatusWithReceipts` with `EXECUTED_OPTIMISTIC`, walk every receipt outcome, and treat any `Failure` as a failure — including failures inside callbacks.
6. Parse Orbit events from the logs (e.g. the new launch ID).
7. Refresh the affected queries, then report the result with an explorer link.

Wallet rejection, timeouts, RPC errors and network mismatch each have their own message.

### Network

`NEXT_PUBLIC_NEAR_NETWORK` must be `testnet`; the app refuses to start otherwise. The connected account's suffix is checked (`.testnet` or an implicit account), and the network is shown permanently in the top bar.

## 4. Media

- Images go straight from the browser to Pinata with a presigned URL from `/api/uploads/sign` (max 5 MB; `image/png`, `image/jpeg`, `image/webp`, `image/gif`, `image/svg+xml`; 60 s expiry; rate-limited per IP).
- The app stores `ipfs://<cid>` style references plus a gateway `base_uri`, and a SHA-256 `media_hash` where NEP-177 asks for one.
- Plain `https://` media URLs are accepted with a visible "not content-addressed" warning.
- Without a Pinata key configured, uploading is disabled and the form says so; creators can still paste an existing IPFS CID or URL.

## 5. Testing

| Layer | Tool | Scope |
|---|---|---|
| Contract unit | `cargo test` with `VMContextBuilder` | Validation, authorisation, arithmetic, state transitions, callback branches |
| Contract integration | `near-workspaces` 0.24 on local sandbox | Factory → global contracts → launches → mint → fund → claim → duplicate claim → balances |
| Web unit | Vitest + Testing Library | Units, validation, tx-outcome parsing, components and states |
| Web e2e | Playwright (production build, desktop + mobile) | Navigation, forms, launch page rendering, filters, disconnected states, no horizontal overflow. NEAR RPC and FastNEAR are mocked in the test harness only. |
| Testnet | Scripted journey with real accounts (`scripts/`) | Full creator → participant flow |
