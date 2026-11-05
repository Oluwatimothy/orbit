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
 │ orbit-factory.testnet  (launch registry, stores the template)  │
 │   └─ <slug>.orbit-factory.testnet   one launch contract:       │
 │        token (NEP-141/145/148) and/or collection               │
 │        (NEP-171/177/178/181), holder claims, gated sale        │
 │ Each launch runs a copy of the template; code hash is public   │
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

Rust workspace at `contracts/`, `near-sdk` 5.30.0, `near-contract-standards` 5.30.0, built with `cargo near` (ABI not embedded, to keep the code small). Two contracts, plus the shared crate `orbit-common` (config types, limits, validation, events). Why one launch contract instead of four: D-026.

### 2.1 Factory (`contracts/factory`)

**State**
- `owner_id` may replace the template used by **future** launches and pause new launches. It cannot touch deployed launches: they have no access keys.
- `template: Option<{ hash, size }>`. The wasm itself sits under a raw storage key outside the contract state, so ordinary calls never load it.
- `launches: Vector<Launch>`: an append-only registry, where `id` is the index.
- `launches_by_creator: LookupMap<AccountId, Vec<u64>>`.
- `slugs: LookupSet<String>` reserves a slug before deployment.

**Launch record:** `id`, `kind` (`token | collection | connected`), `slug`, display fields, `creator`, `created_at_ns`, `status` (`deploying | live | failed`), `account` (`<slug>.<factory>`), and `token` / `nft` / `claim` / `sale`. The last four name that same account for each part the launch has, and are `null` otherwise.

**Change methods**
- `set_template()`: owner only, 1 yocto. The raw call input is the wasm; it must start with the wasm header.
- `set_paused(bool)`: owner only, 1 yocto.
- `create_token_launch(slug, description, token)`, `create_collection_launch(slug, collection)` and `create_connected_launch(slug, description, token, collection, claim?, sale?)`. All are payable.

**Views:** `get_config()`, `quote_deposit(kind)`, `is_slug_available(slug)`, `get_launch(id)`, `get_launches(from_index, limit ≤ 50)`, `get_launches_by_creator(account_id, from_index, limit)`, `get_launch_count()`.

**Deployment flow**
1. Validate the slug (2–20 chars, `[a-z0-9-]`, no leading, trailing or double hyphen) and every part with `validate_launch`. This is the same function the launch contract runs in `new`.
2. Reserve the slug, append the launch as `deploying`, and measure the registry storage it used.
3. Require `attached ≥ registry storage + launch balance`, where launch balance = template size × 0.00001 NEAR + 0.2 NEAR state allowance. Refund the excess immediately.
4. One batch receipt: `create_account → transfer(launch balance) → deploy_contract(template) → new(creator_id, token?, collection?, claim?, sale?)`. Actions in a receipt are atomic, so if `new` panics the account is not created and the balance returns to the factory.
5. `on_launch_deployed(id, balance)` marks the launch `live`, or `failed` with the balance refunded to the creator. The sandbox verifies that the failed deposit returns to the factory before this callback runs, so the refund does not depend on the factory's spare balance.

### 2.2 Launch contract (`contracts/launch`)

One contract per launch. Its state is `creator_id` plus optional parts: `token`, `collection`, `claims` and `sale`. A call to a part the launch lacks panics with "this launch has no …". `new` runs `validate_launch` against its own account ID. That requires:
- a token, a collection, or both;
- with both, holder claims, a sale, or both;
- claims and sales only with both;
- the token allocates to the launch account exactly `claim reserve + sale total` (zero when there is neither, so no tokens can be locked by accident).

**Token** (`token.rs`). Fixed supply minted in `new` to 1–10 allocations that sum exactly to the supply; there is no mint method afterwards. It implements NEP-141, NEP-145 and NEP-148 by delegating to `near-contract-standards`. View: `orbit_allocations()`.

**Collection** (`collection.rs`, `nft_standards.rs`). Supply is `Fixed(n ≤ 10,000)` or explicit `Unlimited`. The price, mint window, per-wallet limit and public-mint switch are fixed at launch. IDs are sequential from `"1"`.
- `nft_mint(quantity ≤ 10)` measures every write, including the wallet counter, which is flushed before measuring. It requires price + storage + a 0.0025 NEAR per-token transfer reserve, pays the price to the creator and refunds the rest.
- `creator_mint` (owner only) ignores the price and mint window but not the supply cap.
- `set_paused` is owner only.
- It implements NEP-171, 177, 178 and 181. Views: `orbit_config()`, `orbit_mint_stats()`, `orbit_minted_by(account)`.

**Holder claims** (`claim.rs`). There are 1–10 non-overlapping tiers of NFT IDs, each with an amount per NFT. `claim(nft_token_id)` is payable and synchronous:
1. Check the start time, the canonical ID inside a tier, and that the ID has no existing record.
2. Read the owner from the collection in the same contract; reject unless it is the caller.
3. Write the record, update the totals, register the caller with the token if needed, and transfer the tier amount from the launch account's balance.
4. Charge the storage this added (about 0.003 NEAR) from the deposit and refund the rest.

All of it happens in one receipt: a rejected or failed claim changes nothing, and there are no pending states.

Views: `claim_info()`, `claim_allocation(id)`, `claim_record(id)`, `claim_records(ids ≤ 50)`.

**Gated sale** (`sale.rs`). It has a holder window with a per-NFT limit and an optional public window with a per-account limit; it ends at `ends_at_ns`.

`buy(amount, nft_token_id?)` is payable and synchronous:
1. Check the phase and the remaining tokens.
2. In the holder window, check ownership of the NFT.
3. Update the limit counter and `sold`.
4. Transfer the tokens.
5. Require `cost + measured storage`, pay the cost to the creator, and refund the rest.

Cost uses 256-bit arithmetic and rounds up. `withdraw_unsold()` lets the creator take back unsold tokens once, after the end.

Views: `sale_info()`, `sale_quote(amount)`, `sale_purchased_by_nft(id)`, `sale_purchased_by_account(account)`.

**Reserve invariant.** Only `claim`, `buy` and `withdraw_unsold` move tokens out of the launch account. Claims are capped by the tier totals and sales by `total_for_sale`, and their sum is exactly what was minted to the account. The account has no keys and never calls itself except for the standard `*_resolve_transfer` callbacks, so no other path can spend its balance.

### 2.3 Events

Standard events (`ft_mint`, `ft_transfer`, `ft_burn`, `nft_mint`, `nft_transfer`) come from `near-contract-standards`. Orbit events use NEP-297 with `standard: "orbit"`, `version: "1.0.0"`: `template_set`, `launch_created`, `launch_live`, `launch_failed`, `claim`, `purchase`, `sale_closed`.

### 2.4 Code upgrades

Launch contracts are not upgradeable: they have no keys, and the code each one runs is visible by hash and matches the factory's `template.hash`. The factory owner can set a new template for **future** launches. The factory account itself is controlled by a full-access key held by the project owner; this is a disclosed trust assumption (see THREAT-MODEL).

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
| Contract integration | `near-workspaces` 0.24 on local sandbox | Factory → template upload → launches → mint → claim → duplicate and non-owner claims → gated sale → balances, storage limits, failure refunds |
| Web unit | Vitest + Testing Library | Units, validation, tx-outcome parsing, components and states |
| Web e2e | Playwright (production build, desktop + mobile) | Navigation, forms, launch page rendering, filters, disconnected states, no horizontal overflow. NEAR RPC and FastNEAR are mocked in the test harness only. |
| Testnet | Scripted journey with real accounts (`scripts/`) | Full creator → participant flow |
