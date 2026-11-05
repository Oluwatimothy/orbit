# Deployment

Orbit has two deployable parts: the contracts (NEAR testnet) and the web app (Vercel). There is no database or background worker.

## 1. Testnet accounts

Install the NEAR CLI (`near-cli-rs` 0.30.1):

```bash
cargo install --locked near-cli-rs@0.30.1
```

Create accounts from the testnet faucet. Keys are saved to the macOS keychain, never to the repository:

```bash
near account create-account sponsor-by-faucet-service orbit-factory.testnet autogenerate-new-keypair save-to-keychain network-config testnet create
near account create-account sponsor-by-faucet-service orbit-publisher.testnet autogenerate-new-keypair save-to-keychain network-config testnet create
```

**Funding.** Publishing global contracts burns 0.0001 NEAR per byte of code:

| Contract | Size | Publish cost |
|---|---|---|
| Token | 179,961 B | ~18 NEAR |
| NFT | 282,335 B | ~28 NEAR |
| Claim | ~217 kB | ~22 NEAR |
| Sale | ~227 kB | ~23 NEAR |

The publisher account needs about **95 testnet NEAR** in total, more than one faucet account provides. Fund it from several faucet accounts or a larger testnet faucet. The factory account needs about 5 NEAR: ~2.8 NEAR for its own code storage plus a buffer for refunds.

## 2. Deploy the contracts

```bash
ORBIT_PUBLISHER=orbit-publisher.testnet \
ORBIT_FACTORY=orbit-factory.testnet \
ORBIT_OWNER=<your-account>.testnet \
./scripts/deploy-testnet.sh
```

The script builds every contract, publishes the four launch templates (token, NFT, claim, sale) by hash, deploys the factory, calls `new` with the hashes, and prints `get_config`.

Verify:

```bash
near contract call-function as-read-only orbit-factory.testnet get_config json-args '{}' network-config testnet now
```

## 3. Web app on Vercel

1. Import the repository and set **Root Directory** to `apps/web`. The framework preset is detected as Next.js.
2. Set environment variables for Production and Preview. All are documented in `.env.example`:
   - `NEXT_PUBLIC_NEAR_NETWORK=testnet`
   - `NEXT_PUBLIC_ORBIT_FACTORY=orbit-factory.testnet`
   - `PINATA_JWT` (optional, server-only; enables image uploads)
3. Deploy. The app is static apart from `/api/uploads/sign`, which runs as a function.

`NEXT_PUBLIC_*` values are inlined at build time, so redeploy after changing them.

## 4. Create the ORBIT GENESIS demo (real transactions)

1. Upload `assets/genesis-cover.svg` to IPFS (for example in the Pinata dashboard) and note its CID.
2. Run:

```bash
ORBIT_FACTORY=orbit-factory.testnet \
ORBIT_CREATOR=<your-account>.testnet \
ORBIT_COVER=ipfs://<cid> \
./scripts/seed-genesis.sh
```

This creates a 2,000-pass collection and a 10,000,000 ORBIT token:
- 2,000,000 ORBIT reserved for holder claims (1,000 per pass);
- 500,000 ORBIT for a 30-day holder sale (up to 250 per pass at 0.001 NEAR).

Open `/launches/0` in the app to see it.

## 5. Smoke test on testnet

1. Connect a testnet wallet.
2. Create a connected launch with a small NFT supply.
3. Mint an NFT and check ownership:
   ```bash
   near contract call-function as-read-only <slug>-nft.orbit-factory.testnet nft_tokens_for_owner json-args '{"account_id":"<you>.testnet"}' network-config testnet now
   ```
4. Claim its allocation and check the token balance with `ft_balance_of`.
5. Try to claim again: the transaction must fail with "this NFT has already been claimed".
6. Repeat from a second wallet.
