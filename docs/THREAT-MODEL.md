# Orbit — Threat Model

Scope: Orbit contracts, the web application and its API route, on NEAR testnet. **Orbit is not audited.** Nothing here should be read as a security guarantee beyond what the tests demonstrate.

## Assets

| Asset | Where it lives |
|---|---|
| Token balances and supply | Token contracts |
| NFT ownership and supply caps | NFT contracts |
| Tokens reserved for holder claims | Claim contracts |
| Mint proceeds | Paid to the creator in the mint receipt |
| Launch registry integrity | Factory |
| Users' keys | Users' wallets — never in Orbit |
| Pinata API key | Server environment only |
| Factory owner key | Project owner's machine — never in the repository |

## Actors

- **Creator:** configures and pays for a launch.
- **Participant:** mints, holds, claims.
- **Factory owner:** sets code hashes for future launches.
- **Outside attacker:** any account, any calls, any arguments.
- **Third-party services:** RPC providers, FastNEAR, Pinata, wallets.

## Threats and mitigations

### Contracts

| # | Threat | Mitigation | Verified by |
|---|---|---|---|
| C1 | Anyone mints extra fungible tokens | No mint method after `new`; `new` runs once in the deployment batch and is `#[init]` | Unit tests: no mint path; total supply equals the sum of allocations |
| C2 | Allocations don't add up to supply / overflow | Checked `u128` arithmetic; exact equality required | Unit tests with boundary and overflow inputs |
| C3 | Minting beyond max supply | Cap checked before each mint in `nft_mint` and `creator_mint` | Unit + sandbox tests |
| C4 | Duplicate NFT IDs | IDs are a sequential counter | Unit test |
| C5 | Minting below price or without storage payment | Required deposit computed after measuring actual storage; panic reverts the whole mint | Unit + sandbox tests |
| C6 | Unauthorised admin calls (`creator_mint`, `set_paused`, factory config) | `predecessor_account_id == owner` assertions | Unit tests for each privileged method |
| C7 | Double claim of one NFT | Record written as `Pending` before any cross-contract call; one record per NFT ID | Unit + sandbox tests (sequential and in-flight) |
| C8 | Claim with an NFT the caller does not own | Ownership read from the NFT contract in a cross-contract call | Sandbox test |
| C9 | Claim recorded as complete when the transfer failed | `Claimed` written only in the callback after a successful `ft_transfer` result; failure rolls back | Unit test with mocked promise failure |
| C10 | Claims exceed funded balance | `total_reserved` fixed at init; activation requires `balance ≥ total_reserved`; no other outbound transfer path | Unit + sandbox tests |
| C11 | Forged callbacks | Callbacks are `#[private]` | Unit test calling a callback from another account |
| C12 | Re-initialisation or upgrade of a launch | No access keys on launch accounts; code referenced by immutable hash | Sandbox test: account has no keys |
| C13 | Slug collision / account squatting | Only the factory can create its subaccounts; slugs reserved in factory state before deployment | Unit test |
| C14 | Gas or storage exhaustion through unbounded input | Limits on allocations (10), tiers (10), mint quantity (10), page size (50), string lengths | Unit tests at limit and limit + 1 |
| C15 | Partial failure of a connected launch | Atomic batch per account; callback records `Failed` and refunds balances of accounts that were not created | Sandbox test with a forced init failure |

| C16 | Over-selling or exceeding sale limits via concurrent purchases | NFT/account counters and `pending` reserved before any cross-contract call; `amount ≤ total − sold − pending` | Sale unit tests (in-flight limit, oversell) + sandbox |
| C17 | Buying in the holder window without owning the NFT | Ownership read from the NFT contract; rejection releases the reservation | Sale unit + sandbox tests |
| C18 | Creator unpaid or buyer charged on failed delivery | Creator paid only in `on_delivered` after a successful transfer; failure releases and refunds | Sale unit tests |
| C19 | Draining a contract's storage balance with records it never charges for (review H-1, M-2, M-3) | Sale measures and keeps the storage each purchase adds and deletes records that drop to zero; holder-window IDs must be canonical and ≤ supply; mints charge for every write, including the wallet counter, plus a 0.0025 NEAR per-token transfer reserve (owner sets never outnumber tokens) | Sale and NFT unit tests; sandbox storage measurement |
| C20 | Holding supply or claims hostage for free with unowned or fake NFT IDs (review M-1, L-3) | Invalid IDs rejected outright; rejected purchases and claims keep 0.001 NEAR | Sale and claim unit tests |
| C21 | Deploy callback out of gas → launch stuck `Deploying`, refund lost (review L-1) | Callback reserves 80 Tgas static gas (two 30 Tgas activations plus its own work) | Sandbox tests with 300 Tgas |
| C22 | Quoted deposit too low for large icons (review L-2) | Registry allowance raised to 0.1 NEAR (largest record); unused part refunded | Factory unit tests |

### Independent review (2026-11-05)

An adversarial review of all contracts was run against the source and the near-sdk / near-contract-standards 5.30.0 code. It found no way to mint, double-claim, over-sell or get past purchase limits. Its storage and gas findings were fixed (C19–C22).

Accepted or disclosed instead of fixed:
- **Slugs are never freed** after a failed launch, and token-only launches reserve a name for about 0.35 NEAR. Squatting is possible but costly; suffixes cannot collide across slugs.
- **`creator_mint` ignores the mint window.** In a connected launch the creator can mint unminted NFTs and use their claims or sale quota. This is disclosed on the "How it works" page.
- **Public minting during a holder window** lets anyone mint more NFTs and so gain more per-NFT quota. This is a configuration choice; creators can close the mint before the sale.
- **The per-account public limit** can be split across many accounts.
- **Unclaimed claim tokens stay in the claim contract permanently.** NEAR fees kept by the sale contract cannot be withdrawn.

### Web application and API

| # | Threat | Mitigation |
|---|---|---|
| W1 | Secret leaks into the browser bundle | Only `NEXT_PUBLIC_*` variables reach the client; the Pinata JWT is read only in the route handler; `npm run check:secrets` builds with a sentinel value and fails if it appears in any client asset |
| W2 | Upload endpoint abuse | Per-IP rate limit; signed URLs restricted by size, MIME type and 60 s expiry |
| W3 | Malicious metadata (oversized, `javascript:` URLs, SVG scripts) | URL scheme allow-list (`ipfs://`, `https://`, `data:image/…`); SVG is rendered only via `<img>`, never inlined; length limits |
| W4 | Fake success shown to users | Success shown only after the transaction is confirmed by hash with all receipts successful (D-008) |
| W5 | Network confusion | Network fixed by configuration; non-testnet accounts are rejected with an explanation; network shown in the top bar |
| W6 | Duplicate submissions | Actions are disabled from signature request until a final result |
| W7 | Spoofed identity in API calls | The API grants no privileges by account ID; the only route signs public uploads |
| W9 | Forged redirect parameters | `?transactionHashes=` values must be valid base58; each is confirmed by RPC before any result is shown |
| W8 | Third-party data presented as truth | FastNEAR holdings are used only to find contracts; balances and ownership are re-read from the contracts |

## Trust assumptions (disclosed in the app's "How it works" page)

1. **Factory owner** can change which code future launches use and pause new launches. They cannot alter existing launches.
2. **Collection creators** can pause minting and mint to any account within the supply cap, even outside the mint window.
3. **Claim-time eligibility** follows the current holder of each NFT, not a historical snapshot.
4. **Media availability** depends on IPFS pinning (Pinata) or the host of an HTTPS URL. The contracts store references, not images.
5. **RPC and FastNEAR** can be unavailable or rate-limited; the app reports this instead of showing stale or empty data as fact.
6. **Creators** choose the economic rules. Orbit enforces that the rules are consistent and funded, not that they are fair.
7. **Not audited.** The contracts are tested and were reviewed adversarially, but not formally verified or independently audited.
