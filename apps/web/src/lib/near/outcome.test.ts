import { describe, expect, it } from "vitest";

import { describeFailure, findEvent, summarizeOutcome, type OutcomeLike } from "./outcome";

const ok = { SuccessValue: "" };

function outcome(overrides: Partial<OutcomeLike> = {}): OutcomeLike {
  return {
    status: { SuccessValue: btoa("true") },
    transaction: { hash: "HASH", signer_id: "alice.testnet", receiver_id: "claim.testnet" },
    transaction_outcome: { id: "t", outcome: { logs: [], status: { SuccessReceiptId: "r1" } } },
    receipts_outcome: [
      {
        id: "r1",
        outcome: {
          executor_id: "claim.testnet",
          status: ok,
          logs: [
            'EVENT_JSON:{"standard":"orbit","version":"1.0.0","event":"claim","data":[{"nft_token_id":"1"}]}',
          ],
        },
      },
    ],
    ...overrides,
  };
}

describe("summarizeOutcome", () => {
  it("reports success, return value and events", () => {
    const summary = summarizeOutcome(outcome());
    expect(summary.succeeded).toBe(true);
    expect(summary.failure).toBeNull();
    expect(summary.returnValue).toBe(true);
    expect(findEvent(summary, "orbit", "claim")).toEqual({ nft_token_id: "1" });
  });

  it("treats a failed later receipt as failure even if the final status succeeded", () => {
    const failed = outcome();
    failed.receipts_outcome.push({
      id: "r2",
      outcome: {
        executor_id: "ft.testnet",
        logs: [],
        status: {
          Failure: {
            ActionError: {
              index: 0,
              kind: {
                FunctionCallError: {
                  ExecutionError: "Smart contract panicked: The account bob.testnet is not registered",
                },
              },
            },
          },
        },
      },
    });
    const summary = summarizeOutcome(failed);
    expect(summary.succeeded).toBe(false);
    expect(summary.failure).toBe("The account bob.testnet is not registered");
  });

  it("does not report success for unfinished transactions", () => {
    const summary = summarizeOutcome(outcome({ status: "Started" }));
    expect(summary.succeeded).toBe(false);
  });

  it("ignores malformed event logs", () => {
    const o = outcome();
    o.receipts_outcome[0]!.outcome.logs = ["EVENT_JSON:{not json", "plain log"];
    expect(summarizeOutcome(o).events).toEqual([]);
  });
});

describe("describeFailure", () => {
  it("falls back to the action error kind", () => {
    expect(
      describeFailure({ ActionError: { index: 0, kind: { AccountAlreadyExists: { account_id: "x" } } } }),
    ).toBe("Action failed: AccountAlreadyExists");
  });
});
