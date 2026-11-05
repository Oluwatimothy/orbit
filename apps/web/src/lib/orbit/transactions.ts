/**
 * Builders for the transactions Orbit asks a wallet to sign. Amounts are
 * bigint here and decimal strings on the wire.
 */
import type { TransactionRequest } from "@/features/wallet/wallet-provider";

import { parseUnits, YOCTO_DECIMALS } from "../format/units";

export const MAX_GAS = 300n * 10n ** 12n;
const CLAIM_GAS = 150n * 10n ** 12n;
const MINT_GAS = 100n * 10n ** 12n;

/** Mirrors `CLAIM_DEPOSIT` in contracts/claim. */
export const CLAIM_DEPOSIT = parseUnits("0.005", YOCTO_DECIMALS);
/**
 * Storage allowance per minted NFT (2,000 bytes). Measured in the sandbox at
 * 879–1,086 bytes per NFT with 400-character media URLs
 * (`mint_storage_fits_the_web_allowance`). The contract measures the real
 * cost and refunds the rest.
 */
export const MINT_STORAGE_ALLOWANCE = parseUnits("0.02", YOCTO_DECIMALS);

function call(
  receiverId: string,
  methodName: string,
  args: Record<string, unknown>,
  deposit: bigint,
  gas: bigint,
): TransactionRequest {
  return {
    receiverId,
    actions: [{ methodName, args, deposit: deposit.toString(), gas: gas.toString() }],
  };
}

export function mintDeposit(price: bigint, quantity: number): bigint {
  return (price + MINT_STORAGE_ALLOWANCE) * BigInt(quantity);
}

export function mintTx(nftId: string, price: bigint, quantity: number): TransactionRequest {
  return call(nftId, "nft_mint", { quantity }, mintDeposit(price, quantity), MINT_GAS);
}

export function claimTx(claimId: string, nftTokenId: string): TransactionRequest {
  return call(claimId, "claim", { nft_token_id: nftTokenId }, CLAIM_DEPOSIT, CLAIM_GAS);
}

/** Mirrors `MIN_PURCHASE_DEPOSIT` in contracts/sale: covers token registration; excess refunded. */
export const PURCHASE_MARGIN = parseUnits("0.005", YOCTO_DECIMALS);
const BUY_GAS = 150n * 10n ** 12n;

export function buyTx(
  saleId: string,
  amount: bigint,
  cost: bigint,
  nftTokenId: string | null,
): TransactionRequest {
  return call(
    saleId,
    "buy",
    { amount: amount.toString(), nft_token_id: nftTokenId },
    cost + PURCHASE_MARGIN,
    BUY_GAS,
  );
}

export function createLaunchTx(
  factoryId: string,
  method: string,
  args: Record<string, unknown>,
  deposit: bigint,
): TransactionRequest {
  return call(factoryId, method, args, deposit, MAX_GAS);
}
