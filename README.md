# Orbit — The On-Chain Launch Studio

Orbit is a NEAR-native launch platform. Creators deploy fungible tokens, NFT collections, or a **connected token + NFT launch**, where NFT holders claim token allocations under rules enforced by smart contracts.

> **Status (2026-11-05):** contracts and web app are implemented and tested locally: 111 contract tests (including 9 against a real NEAR sandbox), 52 web unit tests and 18 browser tests. The contracts were reviewed adversarially and the findings fixed. **Not yet deployed to testnet** — no contract addresses or live URL exist yet. See [Deployment](docs/DEPLOYMENT.md).

## Why it exists

Launching on NEAR today means stitching tools together. NEP-141 factories exist (e.g. NEAR's token factory, tkn.homes). The main NEAR NFT launch tools, Mintbase and Paras, have shut down or pivoted. Connecting a token to NFT ownership means writing your own claim contract. Orbit does all three through one interface, and keeps the economics visible: every number shown is read from the contracts that enforce it.

What Orbit does **not** claim to have invented: token factories, Merkle claims or phased mint pages. Its contribution is one coherent studio that deploys standard contracts and **enforces the relationship between them**.

## Features

| Launch type | Deploys | Enforced on-chain |
|---|---|---|
| Token | NEP-141 token | Fixed supply minted once; allocations must equal supply; no mint method afterwards |
| NFT collection | NEP-171 collection | Supply cap (or explicit "unlimited"); price; per-wallet limit; mint window; storage paid by minter, excess refunded |
| Connected | Collection + token + claim and/or sale contract | **Holder claims:** reserve funded at launch and verified before opening; each NFT ID claims its tier's amount once; ownership checked at claim time; claim recorded only after the transfer succeeds. **NFT-gated sale:** holder window with per-NFT limits checked against live ownership; optional public window with per-account limits; limits and supply reserved before async calls; creator paid only after delivery; unsold tokens returned after the end |

The web app covers the overview, explore (filters and search), the three create flows with review before signing, launch pages (allocation, mint panel, gallery, claim panel, transaction history), portfolio, my launches, activity, and a "How it works" page.

## Architecture

```
Browser (Next.js) ──NEAR Connect──► user's wallet signs every transaction
     │ view calls / tx status                    │
     ▼                                           ▼
NEAR testnet RPC ◄──────────── orbit-factory (registry + deployer)
                                  ├─ <slug>-ft     NEP-141/145/148
                                  ├─ <slug>-nft    NEP-171/177/178/181
                                  ├─ <slug>-claim  holder claims
                                  └─ <slug>-sale   NFT-gated sale
                                  code: NEP-591 global contracts, by hash
```

- **No database, no backend source of truth.** Discovery reads the factory's on-chain registry. Balances and ownership are read from the asset contracts. Activity comes from the FastNEAR index and links to NearBlocks.
- **Keyless launch accounts.** Nobody, including Orbit and the creator, can upgrade or drain a launch.
- **Success means confirmed.** After signing, the app re-reads the transaction by hash and checks every receipt, so a failed cross-contract callback is reported as a failure.

Details: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · decisions: [docs/DECISIONS.md](docs/DECISIONS.md) · threats: [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md) · research and sources: [docs/RESEARCH.md](docs/RESEARCH.md).

## NEAR standards

NEP-141, NEP-145, NEP-148 (token) · NEP-171, NEP-177, NEP-178, NEP-181 (NFT) · NEP-297 events · NEP-330 source metadata · NEP-591 global contracts. NEP-199 royalties are **not** implemented.

## Stack

| Layer | Choice |
|---|---|
| Contracts | Rust, `near-sdk` 5.30.0, `near-contract-standards` 5.30.0, `cargo-near` 0.22.0 |
| Contract tests | `cargo test`, `near-workspaces` 0.24.0 (local sandbox) |
| Web | Next.js 16.4.5, React 19.3.1, TypeScript 6.0 (strict), Tailwind 4.3 |
| Wallet / RPC | `@hot-labs/near-connect` 0.12.0, `near-api-js` 7.4.0 |
| UI / forms | `radix-ui`, `react-hook-form` + `zod` 4, `lucide-react`, `sonner`, TanStack Query |
| Web tests | Vitest, Testing Library, Playwright |
| Media | IPFS via Pinata presigned uploads (optional) |

## Repository

```
contracts/            Rust workspace
  common/             shared config types, validation, limits, events
  fungible-token/     fixed-supply NEP-141
  nft-collection/     NEP-171 with mint rules
  claim/              holder-claim distribution
  sale/               NFT-gated token sale
  factory/            registry + deployment
  integration-tests/  sandbox tests of the contracts together
  test-support/       sandbox-only global-contract publisher
apps/web/             Next.js application (e2e/ holds Playwright tests)
assets/               ORBIT GENESIS cover artwork
scripts/              build, deploy, seed and hash helpers
docs/                 research, architecture, decisions, threat model, plan, deployment
```

## Local setup

Requirements: Rust (the toolchain in `contracts/rust-toolchain.toml` is installed automatically), `cargo-near` 0.22, Node 22+.

```bash
# Contracts
cargo install --locked cargo-near@0.22.0
./scripts/build-contracts.sh

# Web
cd apps/web
npm ci
cp ../../.env.example .env.local   # set NEXT_PUBLIC_ORBIT_FACTORY once deployed
npm run dev
```

## Tests

```bash
cd contracts
../scripts/build-contracts.sh       # sandbox tests load the built wasm
cargo test --workspace              # 102 unit tests + 9 sandbox integration tests
cargo clippy --workspace --all-targets -- -D warnings

cd ../apps/web
npm test                            # 52 unit/component tests
npm run test:e2e                    # 18 Playwright tests (desktop + mobile, production build)
npm run typecheck && npm run lint && npm run build
npm run check:secrets               # fails if a server secret reaches client assets
```

The sandbox tests cover the full connected-launch journey:
- publish code → deploy factory → create launch
- metadata, supply and funding checks
- keyless launch accounts
- minting up to the cap, with the over-cap mint rejected
- claiming; duplicate and non-owner claims rejected
- transferring an NFT, then the new holder claims
- exact final balances
- forged callbacks rejected
- failed deployments recorded as failed and refunded
- underpayment and wallet limits enforced
- gated sale: holder purchase, non-owner rejection, per-NFT and per-account limits, creator payment
- measured mint storage stays within the web app's allowance

## Testnet deployment

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md). The steps:
1. Fund the accounts.
2. Run `scripts/deploy-testnet.sh`.
3. Deploy `apps/web` to Vercel.
4. Run `scripts/seed-genesis.sh` to create ORBIT GENESIS through real transactions.

**Contract addresses:** not deployed yet. **Live URL:** not deployed yet.

## Try it (after deployment)

1. Connect a testnet wallet and open ORBIT GENESIS (`/launches/0`).
2. Mint a pass, then confirm ownership on the collection contract:
   ```bash
   near contract call-function as-read-only genesis-nft.<factory> nft_tokens_for_owner json-args '{"account_id":"<you>.testnet"}' network-config testnet now
   ```
3. Claim its 1,000 ORBIT, then claim again: the contract rejects the second claim.
4. Buy ORBIT in the holder sale with your pass, then try buying with a pass you don't own.
5. Open any transaction from the history table on NearBlocks.

## Known limitations

- Not deployed to testnet yet. Publishing the four global contracts costs about 91 testnet NEAR (burned) at current sizes.
- Not audited. Reviewed adversarially; see [the threat model](docs/THREAT-MODEL.md) for findings and accepted risks.
- Vesting, Merkle-snapshot claims, royalties and allow-list mints are not implemented.
- Wallet signing has not been exercised against testnet yet. It needs a deployed factory.
- Upload rate limiting is per serverless instance.
- Browser tests mock NEAR RPC in the test harness; end-to-end chain behaviour is covered by the sandbox tests.
- Activity depends on FastNEAR's public index. Transactions in flight are shown in notifications, not the activity table.

## Security assumptions

Summarised from [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md):
- The factory owner can change the code used by *future* launches and pause new launches.
- Eligibility follows the current NFT holder.
- Media availability depends on pinning.
- Creators choose the economics; Orbit checks consistency and funding, not fairness.
