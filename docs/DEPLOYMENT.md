# Deployment

Orbit has two deployable parts: the contracts (NEAR testnet) and the web app (Vercel). There is no database or background worker.

## 1. Testnet account

Install the NEAR CLI (`near-cli-rs` 0.30.1):

```bash
cargo install --locked near-cli-rs@0.30.1
```

One account is enough. It is the factory and its owner. Create it from the testnet faucet, which funds it with 10 NEAR. The key is saved to the macOS keychain, never to the repository:

```bash
near account create-account sponsor-by-faucet-service orbit-factory.testnet autogenerate-new-keypair save-to-keychain network-config testnet create
```

**Funding.** Nothing is burned. The factory stores its own code and the launch contract template as ordinary account storage, at 0.00001 NEAR per byte:

| Item | Size | Locked on the factory |
|---|---|---|
| Factory code | 280,382 B | ~2.80 NEAR |
| Launch template | 406,868 B | ~4.07 NEAR |
| Registry and gas | | < 0.1 NEAR |
| **Total** | | **~6.9 NEAR** |

A 10 NEAR faucet account covers this and leaves about 3 NEAR. That isn't enough for the ORBIT GENESIS demo below, which a creator pays for like any other launch.

**What creators pay.** Each launch is one keyless account holding a copy of the template. The factory quotes about **4.37 NEAR** per launch:

| Item | Amount |
|---|---|
| Code storage | ~4.07 NEAR, locked with the launch |
| State allowance | 0.2 NEAR, locked with the launch. The largest launch the limits allow uses 0.14 NEAR. |
| Registry allowance | 0.1 NEAR, mostly refunded. |

## 2. Deploy the contracts

```bash
ORBIT_FACTORY=orbit-factory.testnet \
ORBIT_OWNER=orbit-factory.testnet \
./scripts/deploy-testnet.sh
```

The script:
1. Builds both contracts.
2. Deploys the factory and calls `new(owner_id)`.
3. Uploads the launch template with `set_template`, passing the wasm as the raw call input.
4. Prints `get_config` together with the expected template hash.

Verify:

```bash
near contract call-function as-read-only orbit-factory.testnet get_config json-args '{}' network-config testnet now
```

`template.hash` must match `node scripts/code-hash.mjs contracts/target/near/orbit_launch/orbit_launch.wasm`. Every launch account shows the same code hash on explorers.

## 3. Web app on Vercel

1. Import the repository and set **Root Directory** to `apps/web`. The framework preset is detected as Next.js.
2. Set environment variables for Production and Preview. All are documented in `.env.example`:
   - `NEXT_PUBLIC_NEAR_NETWORK=testnet`
   - `NEXT_PUBLIC_ORBIT_FACTORY=orbit-factory.testnet`
   - `PINATA_JWT` (optional, server-only): enables image uploads.
3. Deploy. The app is static apart from `/api/uploads/sign`, which runs as a function.

`NEXT_PUBLIC_*` values are inlined at build time, so redeploy after changing them.

## 4. Create the ORBIT GENESIS demo (real transactions)

The creator account needs about 4.4 NEAR.

1. Upload `assets/genesis-cover.svg` to IPFS (for example in the Pinata dashboard) and note its CID.
2. Run:

```bash
ORBIT_FACTORY=orbit-factory.testnet \
ORBIT_CREATOR=<your-account>.testnet \
ORBIT_COVER=ipfs://<cid> \
./scripts/seed-genesis.sh
```

This creates a 2,000-pass collection and a 10,000,000 ORBIT token in one launch contract, `genesis.<factory>`. It holds 2,500,000 ORBIT as the holder reserve:
- 1,000 ORBIT per pass for claims;
- 500,000 ORBIT for a 30-day holder sale (up to 250 per pass at 0.001 NEAR).

Open `/launches/0` in the app to see it.

## 5. Smoke test on testnet

1. Connect a testnet wallet.
2. Create a connected launch with a small NFT supply.
3. Mint an NFT and check ownership:
   ```bash
   near contract call-function as-read-only <slug>.orbit-factory.testnet nft_tokens_for_owner json-args '{"account_id":"<you>.testnet"}' network-config testnet now
   ```
4. Claim its allocation and check the token balance with `ft_balance_of` on the same account.
5. Try to claim again. The transaction must fail with "this NFT has already been claimed".
6. Repeat from a second wallet.
