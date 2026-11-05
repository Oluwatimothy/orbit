# Orbit — Implementation Plan

Each phase ends with a progress report (completed / verified / outstanding / decisions / next). A phase is not closed while critical functionality in it is broken.

## Phase 0 — Research and design ✅ done
- [x] Inspect workspace and toolchain
- [x] Research contracts, frontend, data, storage, hosting, competitors → `RESEARCH.md`
- [x] Architecture, decisions, threat model → `ARCHITECTURE.md`, `DECISIONS.md`, `THREAT-MODEL.md`
- [x] Install `cargo-near` 0.22.0 and `near-cli-rs` 0.30.1

**Acceptance:** documents reference sources and versions; every major dependency has a recorded reason.

## Status at 2026-11-05

| Phase | State |
|---|---|
| 0 Research and design | Done |
| 1 Foundation | Done |
| 2 Token launch | Done locally; testnet check pending deployment |
| 3 NFT collection | Done locally; testnet check pending deployment |
| 4 Connected launch | Done locally, including 4b (NFT-gated sale); testnet check pending deployment |
| 5 Discovery and portfolio | Done |
| 6 Security and quality review | Done: adversarial contract review, every confirmed finding fixed or disclosed, client-secret check, Playwright suite |
| 7 Deployment | Scripts and docs ready; deployment itself is the project owner's step |
| 7a Cost redesign (D-026) | Done: one launch contract per launch, deployed from a template the factory stores. Running Orbit costs about 7 NEAR instead of about 96 |

Evidence:
- **Contracts:** 108 unit tests and 11 sandbox tests; clippy runs with warnings as errors.
- **Web:** 52 unit tests and 18 Playwright tests; typecheck, lint, production build and the secret check all pass.

## Phase 1 — Foundation
- Rust workspace: `common`, `factory`, `fungible-token`, `nft-collection`, `claim`, `integration-tests`; rustfmt + clippy (deny warnings); pinned `rust-toolchain.toml`.
- `apps/web`: Next.js 16.4.5, React 19.3.1, TypeScript strict, Tailwind 4.3.4, ESLint, Prettier, Vitest, Playwright.
- Design system: colour, type and spacing tokens; button, input, field, table, badge, dialog, tabs, tooltip, toast, skeleton, empty state; Orbit mark and wordmark.
- App shell: sidebar, top bar (network badge, wallet), mobile navigation.
- Network config with testnet enforcement; RPC provider with failover; wallet context with NEAR Connect.

**Acceptance:** `npm run build`, lint, typecheck and tests pass; `cargo clippy` and `cargo test` pass; the wallet connects to a testnet account in a real browser.

## Phase 2 — Token launch
- `common` validation + `fungible-token` contract with unit tests.
- Factory: registry, `create_token_launch`, deployment callback, unit tests.
- Sandbox integration test: publish global code → create token launch → metadata, supply and allocation balances.
- Web: bigint units, allocation editor (amount or percentage, exact-sum check), review dialog, tx lifecycle, token page.

**Acceptance:** a token launch created from the UI on testnet, with balances verified by `ft_balance_of`.

## Phase 3 — NFT collection launch
- `nft-collection` contract + unit tests; factory `create_collection_launch`.
- Sandbox: mint, payment, refund, cap, wallet limit, ownership, enumeration.
- Web: media upload route, collection form, collection page with gallery, mint panel.

**Acceptance:** a collection created and an NFT minted on testnet from the UI; supply cap rejection demonstrated in tests.

## Phase 4 — Connected launch
- `claim` contract + unit tests (including mocked callback failures).
- Factory `create_connected_launch` with exact funding consistency and activation.
- Sandbox: the full 15-step journey from the brief, including transfer-then-claim and duplicate claim rejection.
- Web: connected create flow, connected launch page (relationship, tiers, eligibility per owned NFT, claim).
- 4b (only after 4 is solid): NFT-gated sale contract.

**Acceptance:** a real claim on testnet; a duplicate claim rejected on testnet; final balances checked.

## Phase 5 — Discovery and portfolio
- Explore (type/status filters, search over registry), overview (recent launches, live stats), portfolio (created launches, holdings, eligible and completed claims), activity (FastNEAR + receipt status), how-it-works page.

## Phase 6 — Security and quality review
- Walk through `THREAT-MODEL.md` item by item; add any missing tests; run a code review across contracts and web; fix findings.

## Phase 7 — Testnet deployment and polish
- Publish global contracts, deploy the factory, create the ORBIT GENESIS example through real transactions, test with a second wallet, desktop and mobile review, Vercel preparation, final README and `DEPLOYMENT.md`.

## Items that need the project owner
| Needed for | What |
|---|---|
| Phase 2 testnet check | A funded testnet account for the factory (can be created from the faucet with `near-cli-rs`; key stored in the macOS keychain) |
| Phase 7 | Enough testnet NEAR to publish three global contracts (≈ 10 NEAR per 100 kB, exact figure after build) |
| Media uploads | A Pinata JWT in `.env.local` / Vercel (optional — pasting a CID works without it) |
| Phase 7 | Vercel project access for deployment |
