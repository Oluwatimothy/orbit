# Orbit — The On-Chain Launch Studio

Orbit is a NEAR-native launch platform. Creators deploy fungible tokens, NFT collections, or a **connected token + NFT launch**, where NFT holders claim token allocations under rules enforced by smart contracts.

> **Status (2026-11-05):** the contracts and web app are implemented and tested locally:
> - 119 contract tests, 11 of them against a real NEAR sandbox;
> - 52 web unit tests and 18 browser tests.
>
> Each launch is now one contract, deployed from a template the factory stores (D-026). Running Orbit therefore costs about 7 testnet NEAR instead of about 96. **Not yet deployed to testnet**, so no contract addresses or live URL exist yet. See [Deployment](docs/DEPLOYMENT.md).

## Why it exists

Launching on NEAR today means stitching tools together. NEP-141 factories exist (e.g. NEAR's token factory, tkn.homes). The main NEAR NFT launch tools, Mintbase and Paras, have shut down or pivoted. Connecting a token to NFT ownership means writing your own claim contract. Orbit does all three through one interface, and keeps the economics visible: every number shown is read from the contracts that enforce it.

What Orbit does **not** claim to have invented: token factories, Merkle claims or phased mint pages. Its contribution is one coherent studio that deploys standard contracts and **enforces the relationship between them**.

## Features

| Launch type | Deploys | Enforced on-chain |
|---|---|---|
| Token | One launch contract (NEP-141) | Fixed supply minted once; allocations must equal supply; no mint method afterwards |
| NFT collection | One launch contract (NEP-171) | Supply cap (or explicit "unlimited"); price; per-wallet limit; mint window; storage paid by minter, excess refunded |
| Connected | One contract that is both, plus holder claims and/or a gated sale | **Holder claims:** a reserve minted to the launch account at launch; each NFT ID claims its tier's amount once; ownership checked at claim time; check, record and transfer in one receipt. **NFT-gated sale:** holder window with per-NFT limits checked against live ownership; optional public window with per-account limits; check, transfer and creator payment in one receipt; unsold tokens returned after the end |

The web app covers the overview, explore (filters and search), the three create flows with review before signing, launch pages (allocation, mint panel, gallery, claim panel, transaction history), portfolio, my launches, activity, and a "How it works" page.

## Architecture

```
Browser (Next.js) ──NEAR Connect──► user's wallet signs every transaction
     │ view calls / tx status                    │
     ▼                                           ▼
NEAR testnet RPC ◄──────────── orbit-factory (registry + launch template)
                                  └─ <slug>.<factory>  one launch contract:
                                       token (NEP-141/145/148) and/or
                                       collection (NEP-171/177/178/181),
                                       holder claims, gated sale
```

- **No database, no backend source of truth.** Discovery reads the factory's on-chain registry. Balances and ownership are read from the asset contracts. Activity comes from the FastNEAR index and links to NearBlocks.
- **Keyless launch accounts.** Nobody, including Orbit and the creator, can upgrade or drain a launch. Each one runs a copy of the factory's template, and the code hash is public.
- **One receipt per claim or purchase.** The token, collection and reserve live in the same contract. A claim or purchase therefore either completes entirely or changes nothing.
- **Success means confirmed.** After signing, the app re-reads the transaction by hash and checks every receipt, so a failed cross-contract callback is reported as a failure.

Details: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · decisions: [docs/DECISIONS.md](docs/DECISIONS.md) · threats: [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md) · research and sources: [docs/RESEARCH.md](docs/RESEARCH.md).

## NEAR standards

NEP-141, NEP-145, NEP-148 (token) · NEP-171, NEP-177, NEP-178, NEP-181 (NFT) · NEP-297 events · NEP-330 source metadata. NEP-199 royalties are **not** implemented.

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
  launch/             one launch: NEP-141 token, NEP-171 collection, holder claims, gated sale
  factory/            registry, template storage and deployment
  integration-tests/  sandbox tests of the real compiled contracts
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
cargo test --workspace              # 108 unit tests + 11 sandbox integration tests
cargo clippy --workspace --all-targets -- -D warnings

cd ../apps/web
npm test                            # 52 unit/component tests
npm run test:e2e                    # 18 Playwright tests (desktop + mobile, production build)
npm run typecheck && npm run lint && npm run build
npm run check:secrets               # fails if a server secret reaches client assets
```

The sandbox tests cover the full connected-launch journey:
- deploy factory → upload template → create launch (code verified byte-for-byte)
- metadata, supply and reserve checks
- keyless launch accounts
- minting up to the cap, with the over-cap mint rejected
- claiming; duplicate and non-owner claims rejected
- transferring an NFT, then the new holder claims
- exact final balances
- forged callbacks rejected
- failed deployments recorded as failed and refunded, even when the factory has almost no spare balance
- the largest launch the limits allow fits its storage allowance
- underpayment and wallet limits enforced
- gated sale: holder purchase, non-owner rejection, per-NFT and per-account limits, creator payment
- measured mint storage stays within the web app's allowance
- the launch account's balance still covers its storage after a full journey

## Testnet deployment

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md). The steps:
1. Fund one account with about 7 NEAR; one faucet account is enough.
2. Run `scripts/deploy-testnet.sh`.
3. Deploy `apps/web` to Vercel.
4. Run `scripts/seed-genesis.sh` to create ORBIT GENESIS through real transactions.

**Contract addresses:** not deployed yet. **Live URL:** not deployed yet.

## Try it (after deployment)

1. Connect a testnet wallet and open ORBIT GENESIS (`/launches/0`).
2. Mint a pass, then confirm ownership on the collection contract:
   ```bash
   near contract call-function as-read-only genesis.<factory> nft_tokens_for_owner json-args '{"account_id":"<you>.testnet"}' network-config testnet now
   ```
3. Claim its 1,000 ORBIT, then claim again: the contract rejects the second claim.
4. Buy ORBIT in the holder sale with your pass, then try buying with a pass you don't own.
5. Open any transaction from the history table on NearBlocks.

## Known limitations

- Not deployed to testnet yet.
- Each launch locks about 4.37 testnet NEAR: its own copy of the 407 kB code plus a state allowance. A 10 NEAR faucet account covers two launches. Publishing the template as a global contract (about 41 NEAR, burned) would bring that under 0.5 NEAR; see D-026.
- Not audited. Reviewed adversarially; see [the threat model](docs/THREAT-MODEL.md) for findings and accepted risks.
- Vesting, Merkle-snapshot claims, royalties and allow-list mints are not implemented.
- Wallet signing has not been exercised against testnet yet. It needs a deployed factory.
- Upload rate limiting is per serverless instance.
- Browser tests mock NEAR RPC in the test harness; end-to-end chain behaviour is covered by the sandbox tests.
- Activity depends on FastNEAR's public index. Transactions in flight are shown in notifications, not the activity table.

## Security assumptions

Summarised from [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md):
- The factory owner can replace the template used by *future* launches and pause new launches.
- Eligibility follows the current NFT holder.
- Media availability depends on pinning.
- Creators choose the economics; Orbit checks consistency and funding, not fairness.
