# Orbit — Threat Model

Scope: Orbit contracts, the web application and its API route, on NEAR testnet. **Orbit is not audited.** Nothing here should be read as a security guarantee beyond what the tests demonstrate.

## Assets

| Asset | Where it lives |
|---|---|
| Token balances and supply | Launch contracts (token part) |
| NFT ownership and supply caps | Launch contracts (collection part) |
| Tokens reserved for holder claims and the sale | The launch account's own token balance |
| Mint proceeds | Paid to the creator in the mint receipt |
| Launch registry integrity and the launch template | Factory |
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
| C1 | Anyone mints extra fungible tokens | No mint method after `new`; `new` runs once in the deployment batch and is `#[init]` | Unit tests: total supply equals the sum of allocations and never changes |
| C2 | Allocations don't add up to supply / overflow | Checked `u128` arithmetic; exact equality required | Unit tests with boundary and overflow inputs |
| C3 | Minting beyond max supply | Cap checked before each mint in `nft_mint` and `creator_mint` | Unit + sandbox tests |
| C4 | Duplicate NFT IDs | IDs are a sequential counter | Unit test |
| C5 | Minting below price or without storage payment | Required deposit computed after measuring all writes (wallet counter flushed first); panic reverts the whole mint | Unit + sandbox tests |
| C6 | Unauthorised admin calls (`creator_mint`, `set_paused`, `set_template`) | Predecessor checked against the collection owner / factory owner; admin methods require 1 yocto | Unit tests for each privileged method; sandbox test for `set_template` |
| C7 | Double claim of one NFT | One record per canonical NFT ID, checked and written in the same receipt as the transfer | Unit + sandbox tests (same holder, other holder, after transfer) |
| C8 | Claim or holder purchase with an NFT the caller does not own | Owner read from the collection in the same contract, in the same receipt | Unit + sandbox tests |
| C9 | Claim recorded although the transfer failed (or the reverse) | Record, totals and transfer happen in one receipt; any panic reverts all of them | By construction: no cross-contract calls in `claim`/`buy` |
| C10 | Claims or sales spend more than the reserve, or each other's share | Reserve minted to the launch account equals claim total + sale total exactly (checked in `new`); claims capped by tiers, sales by `total_for_sale` | Unit test spending both pools to exactly zero |
| C11 | Forged callbacks | The only callbacks are `on_launch_deployed` and the standard `*_resolve_transfer`, all `#[private]` | Sandbox test calling them from another account |
| C12 | Re-initialisation or upgrade of a launch | No access keys; `#[init]` refuses existing state | Sandbox test: account has no keys and runs the exact template |
| C13 | Slug collision / account squatting | Only the factory can create its subaccounts; slugs reserved in factory state before deployment | Unit test |
| C14 | Gas or storage exhaustion through unbounded input | Limits on allocations (10), tiers (10), mint quantity (10), page size (50), string lengths | Unit tests at limit and limit + 1; sandbox test of the largest launch (13.8 kB state, 79 Tgas) |
| C15 | Failed deployment leaves the creator unpaid | One atomic batch; callback marks `failed` and refunds the account balance | Sandbox tests, including a factory with only 0.5 NEAR to spare |
| C16 | Over-selling or exceeding sale limits | Limits and `sold` checked and updated in the same receipt as the transfer | Unit tests (per-NFT across holders, per-account, sold out) |
| C17 | Creator unpaid or buyer charged without delivery | Payment, transfer and refund in one receipt | Unit + sandbox tests |
| C18 | Draining the launch account's locked balance with storage it never charges for | Claims, purchases and mints measure and keep the storage they add; mints keep a 0.0025 NEAR per-token transfer reserve | Unit tests; sandbox checks the account's balance still covers its storage after a full journey |
| C19 | Spending the reserve through a path other than claim/buy/withdraw | The launch account has no keys and never calls itself except for standard resolve callbacks; `storage_unregister`, `ft_transfer*` act on the predecessor's own balance | Review (see below) |
| C21 | Draining the launch's spare NEAR by registering through a claim or purchase, then calling `storage_unregister` (review finding 1) | A new registration costs at least `storage_balance_bounds().min`, which is what unregistering refunds. Registrations at launch (≤ 10) and the creator's one `withdraw_unsold` registration are bounded and covered by the state allowance | Unit test with a short account ID |
| C20 | Malicious or broken template | Owner-only; affects future launches only; the template hash is public and equals each launch's code hash | Sandbox test; disclosed trust assumption 1 |

### Independent review of the single-contract design (2026-11-05)

An adversarial review read the launch contract, the factory, the shared validation and the relevant `near-contract-standards` / `near-sdk` source. It found nothing critical or high:
- No path moves tokens out of the launch account other than `claim`, `buy` and `withdraw_unsold`.
- Claims and the sale cannot overspend each other.
- Storage measurement covers every write.
- Init arguments cannot be injected.

How each finding was handled:
1. **Fixed (C21).** Medium/low: register/unregister cycles could drain about 0.0005 NEAR each from the launch's spare balance.
2. **Disclosed.** `creator_mint` ignores the mint window and price. In a connected launch the creator can mint unminted NFTs and claim or buy with them. This is trust assumption 2 and is stated on the "How it works" page.
3. **Refuted by test.** The review suspected the failure refund depends on the factory's spare balance. The sandbox test `failed_launch_refund_does_not_need_spare_factory_balance` shows that the failed deposit returns before the callback runs.
4. **Info.** The registry quote counts characters, not bytes. The real cost is measured and enforced, so an oversized description fails cleanly instead of being under-charged.
5. **Info.** Contract metadata lists both token and NFT standards for every launch. Methods for a part the launch does not have fail with "this launch has no …".

**Accepted or disclosed instead of fixed:**
- **Slugs are never freed** after a failed launch. Squatting a name costs a full launch deposit, minus the refund.
- **`creator_mint` ignores the mint window** (finding 2 above).
- **Public minting during a holder window** lets anyone mint more NFTs and so gain more per-NFT sale quota. This is a configuration choice; creators can close the mint before the sale.
- **The per-account public limit** can be split across many accounts.
- **Some tokens stay in the launch contract permanently:** unclaimed claim tokens (including the reserve for NFTs that are never minted), and tokens or NFTs sent to the launch account.

The review of the original four-contract design found no way to mint, double-claim, over-sell or bypass limits. Its storage and gas findings were fixed before the redesign; the redesign removed the asynchronous paths they concerned.

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

1. **Factory owner** can replace the template future launches use and pause new launches. They cannot alter existing launches.
2. **Collection creators** can pause minting and mint to any account within the supply cap, even outside the mint window.
3. **Claim-time eligibility** follows the current holder of each NFT, not a historical snapshot.
4. **Media availability** depends on IPFS pinning (Pinata) or the host of an HTTPS URL. The contracts store references, not images.
5. **RPC and FastNEAR** can be unavailable or rate-limited; the app reports this instead of showing stale or empty data as fact.
6. **Creators** choose the economic rules. Orbit enforces that the rules are consistent and funded, not that they are fair.
7. **Not audited.** The contracts are tested and were reviewed adversarially, but not formally verified or independently audited.
