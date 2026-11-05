/**
 * Interprets a transaction's execution outcome. A transaction "succeeds" only
 * when the transaction and every receipt it spawned succeeded — a failed
 * cross-contract callback is a failure even if the first receipt was fine.
 */

type ExecutionStatus =
  | "Unknown"
  | "NotStarted"
  | "Started"
  | { Failure: unknown }
  | { SuccessValue: string }
  | { SuccessReceiptId: string };

export interface OutcomeLike {
  status: ExecutionStatus;
  transaction: { hash: string; signer_id: string; receiver_id: string };
  transaction_outcome: { id: string; outcome: { logs: string[]; status: ExecutionStatus } };
  receipts_outcome: Array<{
    id: string;
    outcome: { logs: string[]; status: ExecutionStatus; executor_id: string };
  }>;
}

export interface NepEvent {
  standard: string;
  version: string;
  event: string;
  data: unknown[];
}

export interface TxSummary {
  hash: string;
  succeeded: boolean;
  /** Human-readable reason for the first failure, if any. */
  failure: string | null;
  /** Decoded `SuccessValue` of the transaction, if it returned JSON. */
  returnValue: unknown;
  events: NepEvent[];
  logs: string[];
}

function failureOf(status: ExecutionStatus): unknown {
  return typeof status === "object" && "Failure" in status ? status.Failure : undefined;
}

/** Extracts the contract's panic message from a NEAR `TxExecutionError`. */
export function describeFailure(failure: unknown): string {
  const text = JSON.stringify(failure) ?? "unknown error";
  const panic = /"(?:ExecutionError|PanicMsg|panic_msg)":"((?:[^"\\]|\\.)*)"/.exec(text);
  if (panic?.[1]) {
    return panic[1].replace(/^Smart contract panicked: /, "").replaceAll('\\"', '"');
  }
  const kind = /"ActionError":\{[^}]*"kind":\{"(\w+)"/.exec(text);
  return kind?.[1] ? `Action failed: ${kind[1]}` : text;
}

function decodeBase64Json(value: string): unknown {
  if (!value) return undefined;
  try {
    const bytes = Uint8Array.from(atob(value), (c) => c.charCodeAt(0));
    return JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    return undefined;
  }
}

export function parseEvents(logs: string[]): NepEvent[] {
  const events: NepEvent[] = [];
  for (const log of logs) {
    if (!log.startsWith("EVENT_JSON:")) continue;
    try {
      const parsed = JSON.parse(log.slice("EVENT_JSON:".length)) as Partial<NepEvent>;
      if (typeof parsed.standard === "string" && typeof parsed.event === "string") {
        events.push({
          standard: parsed.standard,
          version: String(parsed.version ?? ""),
          event: parsed.event,
          data: Array.isArray(parsed.data) ? parsed.data : [],
        });
      }
    } catch {
      // A malformed event log from some contract is not a transaction failure.
    }
  }
  return events;
}

export function summarizeOutcome(outcome: OutcomeLike): TxSummary {
  const statuses = [
    outcome.status,
    outcome.transaction_outcome.outcome.status,
    ...outcome.receipts_outcome.map((r) => r.outcome.status),
  ];
  const firstFailure = statuses.map(failureOf).find((f) => f !== undefined);
  const logs = [
    ...outcome.transaction_outcome.outcome.logs,
    ...outcome.receipts_outcome.flatMap((r) => r.outcome.logs),
  ];
  const finalStatus = outcome.status;
  const returnValue =
    typeof finalStatus === "object" && "SuccessValue" in finalStatus
      ? decodeBase64Json(finalStatus.SuccessValue)
      : undefined;
  const pending = finalStatus === "NotStarted" || finalStatus === "Started" || finalStatus === "Unknown";

  return {
    hash: outcome.transaction.hash,
    succeeded: firstFailure === undefined && !pending,
    failure:
      firstFailure !== undefined
        ? describeFailure(firstFailure)
        : pending
          ? "Transaction has not finished executing"
          : null,
    returnValue,
    events: parseEvents(logs),
    logs,
  };
}

export function findEvent(
  summary: TxSummary,
  standard: string,
  event: string,
): Record<string, unknown> | undefined {
  const match = summary.events.find((e) => e.standard === standard && e.event === event);
  const first = match?.data[0];
  return first && typeof first === "object" ? (first as Record<string, unknown>) : undefined;
}
