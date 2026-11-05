# Orbit — Technical Research

Research date: **2026-11-05**. Versions were checked against crates.io, the npm registry and GitHub releases on that date; protocol limits were read from live `EXPERIMENTAL_protocol_config` RPC calls. Items marked **unverified** could not be confirmed from a primary source.

## 1. Environment

| Item | Value |
|---|---|
| Machine | macOS (Darwin 24.6.0), Apple Silicon `arm64` |
| Node / npm | 24.19.0 / 11.17.0 |
| Rust | 1.99.1 stable via rustup, target `wasm32-unknown-unknown` |
| Docker | 29.7.2 (needed only for reproducible contract builds) |
| Not installed at start | pnpm, cargo-near, near-cli-rs, PostgreSQL |

No prior Orbit code existed; the project is created at `~/Documents/orbit`.

## 2. NEAR smart contracts

### SDK and standards

| Crate | Version | Notes |
|---|---|---|
| `near-sdk` | 5.30.0 (2026-10-27) | `rust-version = 1.95`. `Promise` is `#[must_use]`. `env::promise_result` is deprecated in favour of `env::promise_result_checked(idx, max_len)`. |
| `near-contract-standards` | 5.30.0 | All `impl_*` macros are deprecated; contracts embed `FungibleToken` / `NonFungibleToken` and implement the traits by hand. |
| `cargo-near` | 0.22.0 (2026-07-14) | Builds wasm and runs `wasm-opt`, which the source describes as required for correct builds on rustc ≥ 1.82. Reproducible (Docker) builds emit NEP-330 build metadata. |
| `near-workspaces` | 0.24.0 (2026-10-26) | Sandbox integration testing; depends on `near-sandbox` 0.4.0. |
| `near-sandbox` | 0.4.0 | Supports Darwin arm64 natively. |
| `near-cli-rs` | 0.30.1 (2026-09-02) | Account creation via the testnet faucet; global contract deployment. |

Standards coverage in `near-contract-standards` 5.30.0:

| NEP | Provided | Orbit use |
|---|---|---|
| 141 Fungible Token | `FungibleToken`, `FungibleTokenCore`, `FungibleTokenResolver` | Token contract |
| 145 Storage Management | `StorageManagement` + `FungibleToken` helpers | Token contract |
| 148 FT Metadata | `FungibleTokenMetadata`, `FungibleTokenMetadataProvider` | Token contract |
| 171 NFT Core | `NonFungibleToken`, `NonFungibleTokenCore`, `NonFungibleTokenResolver` | NFT contract |
| 177 NFT Metadata | `NFTContractMetadata`, `TokenMetadata` | NFT contract |
| 178 Approvals | `NonFungibleTokenApproval` | NFT contract (keeps marketplace compatibility) |
| 181 Enumeration | `NonFungibleTokenEnumeration` | NFT contract (gallery, portfolio) |
| 199 Royalties / payouts | **Not provided** | Deferred — see DECISIONS D-011 |
| 297 Events | `FtMint`, `FtTransfer`, `NftMint`, … | All contracts; Orbit adds its own `orbit` event standard |
| 330 Source metadata | `#[near(contract_metadata(...))]` | All contracts |

### Protocol limits (live, testnet and mainnet)

| Parameter | Value |
|---|---|
| `storage_amount_per_byte` | 1e19 yoctoNEAR (100 kB = 1 NEAR) |
| Global contract storage | 1e20 yoctoNEAR/byte, burned (10× normal) |
| `max_contract_size` | 4,194,304 bytes |
| `max_transaction_size` | 1,572,864 bytes |
| `max_total_prepaid_gas` | 1 PGas per function call (was 300 Tgas historically) |
| `reftypes_bulk_memory` | `true` on both networks |

### Account IDs

2–64 characters from `[a-z0-9]`, separated by single `-`, `_` or `.` that cannot lead, trail or repeat. An account can create only its **direct** subaccounts. With a factory at `orbit-factory.testnet` (21 chars), a launch account `<slug>-<suffix>.orbit-factory.testnet` leaves roughly 38 characters for slug and suffix. Source: https://nomicon.io/DataStructures/Account

### Factory deployment: global contracts (NEP-591)

Global contracts are live on testnet and mainnet (`global_contract_host_fns: true`, nearcore ≥ 2.6.0). Code is published once and accounts reference it by hash or by publishing account. `near-sdk` 5.30.0 exposes `Promise::deploy_global_contract`, `deploy_global_contract_by_account_id`, `use_global_contract(code_hash)` and `use_global_contract_by_account_id(account_id)`. The official example `examples/factory-contract-global` chains `create_account().transfer().use_global_contract(hash)`.

Cost: publishing burns 0.0001 NEAR/byte (≈ 10 NEAR per 100 kB, not recoverable). Each account that uses it stores only a reference (< 0.001 NEAR). The classic alternative — embedding wasm with `include_bytes!` and `deploy_contract` — stores a full copy of the code in every launch account (≈ 1–3 NEAR of storage each) and bloats the factory.

**Unverified:** that docs.near.org explicitly names global contracts as *the* recommended factory method (the tutorial page returned 404). The SDK example and NEP text support the pattern.

### Cross-contract safety

- Callbacks are `#[private]` and take `#[callback_result] r: Result<T, PromiseError>`.
- State is changed before the outgoing call (e.g. a claim is marked `Pending`) and rolled back in the callback on failure; the callback must not panic before rolling back.
- `ft_transfer` requires exactly 1 yoctoNEAR and a registered receiver.
- Standards gas constants: `GAS_FOR_RESOLVE_TRANSFER` = 5 Tgas. Orbit's own budgets are set conservatively and verified in sandbox tests.

Sources: https://crates.io/crates/near-sdk · https://github.com/near/near-sdk-rs/releases/tag/near-sdk-v5.30.0 · https://github.com/near/near-sdk-rs/tree/master/examples/factory-contract-global · https://docs.near.org/smart-contracts/global-contracts · https://github.com/near/NEPs/blob/master/neps/nep-0591.md · https://github.com/near/cargo-near · https://github.com/near/near-workspaces-rs · https://github.com/near/near-cli-rs/releases · https://github.com/near/nearcore/blob/master/core/parameters/res/runtime_configs/parameters.yaml

## 3. Frontend and wallets

| Package | Version | Finding |
|---|---|---|
| `@hot-labs/near-connect` | 0.12.0 (2026-10-26) | Used by the docs.near.org wallet-login tutorial and `create-near-app` 10.0.0. Each wallet's connector runs in a sandboxed iframe, loaded from a manifest. Zero npm dependencies. 0.x — pin exactly. |
| `@near-wallet-selector/*` | 10.1.4 (2025-12-04) | Maintenance mode; the README recommends the new connector. Its `react-hook` hard-pins React 18.3.1. **Not used.** |
| `near-api-js` | 7.4.0 (2026-10-23) | One consolidated package (the `@near-js/*` packages stopped at 2.5.1). Provides `JsonRpcProvider`, `FailoverRpcProvider`, `viewTransactionStatus`, unit helpers. |
| `next` | 16.4.5 | Latest stable; wallet code must be client-only (`'use client'`, `next/dynamic` with `ssr: false`). |
| `react` / `react-dom` | 19.3.1 | |
| `tailwindcss` | 4.3.4 | CSS-first configuration (`@theme`) — no `tailwind.config.js`. |
| `radix-ui` | 1.8.1 | Accessible primitives (dialog, dropdown, tabs, tooltip). |
| `zod` | 4.6.7 | Schema validation shared by forms and API routes. |
| `react-hook-form` | 7.91.0 | Form state. |
| `lucide-react` | 1.69.0 | Single icon family. |
| `vitest` | 5.1.0 | Unit/component tests (does not support async Server Components — those are covered by Playwright). |
| `@playwright/test` | 1.64.1 | Browser tests. |

Transaction results: `signAndSendTransaction` in NEAR Connect returns a `FinalExecutionOutcome`. Some wallets return results fetched with `wait_until: NONE`, so Orbit always re-queries the transaction by hash with `EXECUTED_OPTIMISTIC` before reporting success, and handles `?transactionHashes=…&errorCode=…` on page load for redirect-based flows.

Amounts: native `bigint` throughout; amounts cross the wallet/contract boundary as decimal strings. `JSON.stringify` cannot serialise `bigint`.

### RPC endpoints (testnet, probed 2026-11-05)

| Endpoint | Status |
|---|---|
| `https://test.rpc.fastnear.com` | Works; listed first on docs.near.org. **Primary.** |
| `https://near-testnet.drpc.org` | Works; ≈ 2,100 CU/s free. **Failover.** |
| `https://rpc.testnet.near.org` | Deprecated; 30 req/min. Not used. |
| `https://archival-rpc.testnet.fastnear.com` | Archival; answered without a key on 2026-11-05 (docs say paid). |

Sources: https://docs.near.org/web3-apps/tutorials/wallet-login · https://github.com/hot-dao/near-selector · https://github.com/near/wallet-selector · https://github.com/near/near-api-js · https://docs.near.org/api/rpc/providers · https://nextjs.org/docs/app/guides/testing/vitest

## 4. Data and indexing

| Option | Testnet | Assessment |
|---|---|---|
| Direct RPC view calls | Yes | Source of truth for balances, ownership, launch registry. |
| FastNEAR APIs (`test.api.fastnear.com`, `tx.test.fastnear.com`) | Yes, no key required | Account FT/NFT holdings and per-account transaction history. Rate limits unpublished. |
| neardata.xyz (`testnet.neardata.xyz`) | Yes | Block streaming, 180 req/min/IP. Official replacement for NEAR Lake. Basis for a future indexer. |
| NearBlocks API v3 | Yes | Free plan 6 req/min, 333/day — too limited to serve the app. Used only for explorer links. |
| NEAR Lake Framework | — | **Stopped indexing on 2026-03-24.** Not used. |
| QueryAPI | — | Ended with Pagoda. Not used. |
| Goldsky, The Graph, Pikespeak | Unverified on testnet | Not used. |

**Conclusion:** a custom indexer is not justified for the hackathon scope. The factory keeps an on-chain launch registry with paginated views; contracts emit NEP-297 events; activity is read on demand from FastNEAR's transaction API and confirmed against RPC. See DECISIONS D-006.

Explorer links: `https://testnet.nearblocks.io/txns/{hash}` (documented by NEAR). The account path `/address/{account}` is the usual NearBlocks format but could not be fetched automatically (HTTP 403) — **unverified**.

Sources: https://docs.near.org/data-infrastructure/data-services · https://docs.near.org/data-infrastructure/lake-framework/near-lake · https://docs.fastnear.com · https://github.com/fastnear/neardata-server · https://api.nearblocks.io/openapi.json

## 5. Media storage

| Option | Status | Assessment |
|---|---|---|
| Pinata (IPFS) | Active. Free: 1 GB, 500 files. Presigned upload URLs. | **Selected.** The browser uploads directly with a short-lived signed URL; the API key stays on the server; avoids Vercel's 4.5 MB body limit. |
| nft.storage Classic | Uploads stopped 2024-06-30 | Not used. |
| Storacha / web3.storage | Redirects to fil.one; gateway status unclear | Not used. |
| Arweave via Irys / Turbo | Free < 100 KiB | Possible future option for metadata JSON. |
| Filebase | Free tier size **unverified** | Not used. |

An IPFS CID is content-addressed — the bytes cannot change without the CID changing — but it is only *available* while some node pins it. Orbit does not describe pinned content as "permanent". Plain HTTPS URLs are accepted only with a visible warning that the host can change or remove them.

Sources: https://pinata.cloud/pricing · https://docs.pinata.cloud/files/presigned-urls · https://github.com/nftstorage/nft.storage · https://docs.irys.xyz/overview/cost-to-upload

## 6. Hosting

- Vercel supports Next.js 16 natively. Function body limit 4.5 MB. Hobby cron runs at most daily, so a polling indexer cannot live there.
- Vercel Postgres was discontinued (moved to Neon, Dec 2024). Not needed in this scope.
- A future continuous indexer belongs on a worker host (Railway / Fly.io / Render — pricing **unverified**) writing to Neon.

Sources: https://vercel.com/docs/functions/limitations · https://vercel.com/docs/cron-jobs/usage-and-pricing · https://vercel.com/docs/postgres

## 7. Competitors and positioning

| Product | Chain | What it does | Status (2026-11) |
|---|---|---|---|
| Meme.cooking | NEAR | Fair-launch meme tokens → Ref Finance | Live, near-idle (≈ $2.3k TVL) |
| tkn.homes | NEAR (inferred) | No-code FT deployment | Live |
| NEAR token factory (`tkn.primitives.testnet`) | NEAR | Canonical NEP-141 from metadata | Active developer primitive |
| Aidols | NEAR | AI-agent tokens | Live; mechanics unverified |
| Mintbase → Bitte | NEAR | Was the main NFT minter | NFT product gone; acquired Feb 2026 |
| Paras | NEAR | NFT marketplace | Shut down / redirects |
| Few and Far | NEAR | NFT launchpad | Probably inactive |
| Jump NFT Staking | NEAR | Stake NFTs for token rewards | Status unverified |
| Smithii NEAR Snapshot | NEAR | Paid NFT holder export | Active |
| Pump.fun | Solana | Bonding-curve launches | Active |
| Magic Eden / OpenSea Drops / Zora | Multi | Phased mint pages | Active (OpenSea status unverified) |
| Uniswap merkle-distributor, thirdweb claimable airdrop | EVM | Merkle token claims | Reference implementations |

**What Orbit should not claim as original:** token factories, Merkle claims, phased mint pages, NFT-staking rewards.

**Gap Orbit fills:** no active NEAR product deploys a token and an NFT collection together and connects them with contract-enforced holder claims or NFT-gated sales. The NFT launch side alone is under-served since Mintbase and Paras left.

### Interface references

Hyperliquid and Lighter (from production CSS, 2026-11-05):
- Hyperliquid: Inter for UI; dark teal-black background; one teal accent.
- Lighter: Inter Variable for UI, Geist Mono for numbers, a serif display accent; **13 px base size**; oklch colour tokens in paired themes; Sonner toasts; dedicated zero-state and error-state styles.

Transferable principles: a neutral UI face plus tabular/mono figures for every number and address; dense 12–13 px tables; hairline dividers and surface tiers instead of cards; one accent plus semantic colours; non-blocking toasts with explorer links; inline step lists for multi-step processes; one-line empty states with one action; skeletons shaped like the final layout. Both apps use a top bar rather than a sidebar — Orbit keeps the sidebar the brief asks for, made compact (DECISIONS D-013).

Sources: https://defillama.com/protocol/meme-cooking · https://docs.tkn.homes · https://docs.near.org/primitives/ft · https://bitteprotocol.substack.com/p/mintbase-is-now-bitte-protocol · https://smithii.io/snapshot-near/ · https://github.com/Uniswap/merkle-distributor · https://portal.thirdweb.com/contracts/explore/pre-built-contracts/airdrop-erc20-claimable · live CSS of app.hyperliquid.xyz and app.lighter.xyz
