/**
 * Transaction history from FastNEAR's transactions API (testnet). FastNEAR is
 * an index, not a source of truth: it is used to list hashes and decode what
 * happened; balances and ownership are always read from the contracts.
 */
import { z } from "zod";

import { parseEvents, type NepEvent } from "@/lib/near/outcome";

const TX_API = "https://tx.test.fastnear.com";
const MAX_HASHES_PER_LOOKUP = 50;

const accountTxSchema = z.object({
  transaction_hash: z.string(),
  tx_block_timestamp: z.string(),
  is_success: z.boolean(),
});

const accountResponseSchema = z.object({ account_txs: z.array(accountTxSchema), txs_count: z.number() });

const outcomeSchema = z.object({
  outcome: z.object({ logs: z.array(z.string()), status: z.unknown() }),
});

const txSchema = z.object({
  transaction: z.object({
    hash: z.string(),
    signer_id: z.string(),
    receiver_id: z.string(),
    actions: z.array(z.unknown()),
  }),
  execution_outcome: z.object({ block_timestamp: z.number() }),
  receipts: z.array(z.object({ execution_outcome: outcomeSchema })),
});

export interface ActivityItem {
  hash: string;
  timestampMs: number;
  signer: string;
  receiver: string;
  method: string | null;
  succeeded: boolean;
  events: NepEvent[];
}

async function post<S extends z.ZodType>(path: string, body: unknown, schema: S): Promise<z.infer<S>> {
  const response = await fetch(`${TX_API}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (response.status === 429)
    throw new Error("The activity index is rate-limiting requests. Try again shortly.");
  if (!response.ok) throw new Error(`The activity index returned ${response.status}.`);
  const parsed = schema.safeParse(await response.json());
  if (!parsed.success) throw new Error("The activity index returned an unexpected response.");
  return parsed.data as z.infer<S>;
}

/** Newest transactions that touched `accountId`. */
export async function accountTransactions(accountId: string, limit = 25) {
  const data = await post("/v0/account", { account_id: accountId, desc: true, limit }, accountResponseSchema);
  return data.account_txs;
}

function methodOf(actions: unknown[]): string | null {
  const first = actions[0];
  if (first && typeof first === "object" && "FunctionCall" in first) {
    const call = (first as { FunctionCall: { method_name?: unknown } }).FunctionCall;
    return typeof call.method_name === "string" ? call.method_name : null;
  }
  return first && typeof first === "object" ? (Object.keys(first)[0] ?? null) : null;
}

function receiptFailed(status: unknown): boolean {
  return typeof status === "object" && status !== null && "Failure" in status;
}

/** Full details for up to 50 hashes, in the order given. */
export async function transactionDetails(hashes: string[]): Promise<ActivityItem[]> {
  if (hashes.length === 0) return [];
  const data = await post(
    "/v0/transactions",
    { tx_hashes: hashes.slice(0, MAX_HASHES_PER_LOOKUP) },
    z.object({ transactions: z.array(txSchema) }),
  );
  return data.transactions.map((tx) => {
    const outcomes = tx.receipts.map((r) => r.execution_outcome.outcome);
    return {
      hash: tx.transaction.hash,
      timestampMs: Math.floor(tx.execution_outcome.block_timestamp / 1_000_000),
      signer: tx.transaction.signer_id,
      receiver: tx.transaction.receiver_id,
      method: methodOf(tx.transaction.actions),
      succeeded: !outcomes.some((o) => receiptFailed(o.status)),
      events: parseEvents(outcomes.flatMap((o) => o.logs)),
    };
  });
}

/** Merged, de-duplicated, newest-first activity across several accounts. */
export async function activityFor(accountIds: string[], limit = 25): Promise<ActivityItem[]> {
  const lists = await Promise.all(accountIds.map((id) => accountTransactions(id, limit)));
  const newest = [...new Map(lists.flat().map((tx) => [tx.transaction_hash, tx])).values()]
    .sort((a, b) => (BigInt(b.tx_block_timestamp) > BigInt(a.tx_block_timestamp) ? 1 : -1))
    .slice(0, limit);
  const details = await transactionDetails(newest.map((tx) => tx.transaction_hash));
  return details.sort((a, b) => b.timestampMs - a.timestampMs);
}
