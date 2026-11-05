import { FailoverRpcProvider, JsonRpcProvider } from "near-api-js";

import { network } from "@/config/network";

import { summarizeOutcome, type OutcomeLike, type TxSummary } from "./outcome";

export class RpcError extends Error {
  constructor(message: string, cause?: unknown) {
    super(message, { cause });
  }
}

let provider: FailoverRpcProvider | undefined;

export function rpc(): FailoverRpcProvider {
  provider ??= new FailoverRpcProvider(network.rpcUrls.map((url) => new JsonRpcProvider({ url })));
  return provider;
}

/** Calls a view method and returns its JSON result. */
export async function view<T>(
  contractId: string,
  method: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  try {
    return (await rpc().callFunction({ contractId, method, args })) as T;
  } catch (error) {
    throw new RpcError(`Could not read ${method} on ${contractId}`, error);
  }
}

/** True when the account exists on the configured network. */
export async function accountExists(accountId: string): Promise<boolean> {
  try {
    await rpc().viewAccount({ accountId });
    return true;
  } catch (error) {
    if (String(error).includes("does not exist")) return false;
    throw new RpcError(`Could not look up ${accountId}`, error);
  }
}

const CONFIRM_ATTEMPTS = 20;
const CONFIRM_INTERVAL_MS = 1_500;

/**
 * Re-reads a transaction by hash until every receipt has executed, so that
 * results reported by wallets with `wait_until: NONE` are never trusted as-is.
 */
export async function confirmTransaction(txHash: string, signerId: string): Promise<TxSummary> {
  let lastError: unknown;
  for (let attempt = 0; attempt < CONFIRM_ATTEMPTS; attempt++) {
    try {
      const response = await rpc().viewTransactionStatusWithReceipts({
        txHash,
        accountId: signerId,
        waitUntil: "EXECUTED_OPTIMISTIC",
      });
      const summary = summarizeOutcome(response as unknown as OutcomeLike);
      if (summary.succeeded || summary.failure !== "Transaction has not finished executing") {
        return summary;
      }
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, CONFIRM_INTERVAL_MS));
  }
  throw new RpcError(`Transaction ${txHash} could not be confirmed. Check it in the explorer.`, lastError);
}
