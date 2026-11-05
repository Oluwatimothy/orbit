"use client";

import { useQueryClient, type QueryKey } from "@tanstack/react-query";
import { useCallback, useRef, useState } from "react";
import { toast } from "sonner";

import { WalletRejectedError, useWallet, type TransactionRequest } from "@/features/wallet/wallet-provider";
import { txUrl } from "@/lib/near/explorer";
import type { TxSummary } from "@/lib/near/outcome";
import { confirmTransaction } from "@/lib/near/rpc";

export type TransactionState =
  | { phase: "idle" }
  | { phase: "signing" }
  | { phase: "confirming"; hash: string }
  | { phase: "succeeded"; summary: TxSummary }
  | { phase: "failed"; message: string; summary?: TxSummary };

interface RunOptions {
  /** Toast titles for this action, e.g. "Mint 2 NFTs". */
  label: string;
  /** Queries to refetch after a confirmed success. */
  invalidate?: QueryKey[];
  /** Custom success check, e.g. a claim that returned `false`. */
  check?: (summary: TxSummary) => string | null;
}

/**
 * Sign → confirm by hash → verify every receipt → refresh → report.
 * Success is never announced from the wallet response alone (D-008).
 */
export function useTransaction() {
  const { signAndSend, accountId } = useWallet();
  const queryClient = useQueryClient();
  const [state, setState] = useState<TransactionState>({ phase: "idle" });
  const inFlight = useRef(false);

  const run = useCallback(
    async (request: TransactionRequest, options: RunOptions): Promise<TxSummary | null> => {
      if (inFlight.current || !accountId) return null;
      inFlight.current = true;
      const toastId = toast.loading(options.label, { description: "Confirm in your wallet" });
      setState({ phase: "signing" });
      try {
        const walletOutcome = await signAndSend(request);
        const hash = walletOutcome.transaction.hash;
        setState({ phase: "confirming", hash });
        toast.loading(options.label, { id: toastId, description: "Confirming on testnet…" });

        const summary = await confirmTransaction(hash, accountId);
        const failure = summary.failure ?? options.check?.(summary) ?? null;
        await Promise.all(
          (options.invalidate ?? []).map((queryKey) => queryClient.invalidateQueries({ queryKey })),
        );

        if (failure) {
          setState({ phase: "failed", message: failure, summary });
          toast.error(`${options.label} failed`, {
            id: toastId,
            description: failure,
            action: explorerAction(hash),
          });
        } else {
          setState({ phase: "succeeded", summary });
          toast.success(options.label, {
            id: toastId,
            description: "Confirmed on testnet",
            action: explorerAction(hash),
          });
        }
        return summary;
      } catch (error) {
        const message =
          error instanceof WalletRejectedError
            ? error.message
            : error instanceof Error
              ? error.message
              : "The transaction could not be sent.";
        setState({ phase: "failed", message });
        toast.error(`${options.label} not completed`, { id: toastId, description: message });
        return null;
      } finally {
        inFlight.current = false;
      }
    },
    [accountId, queryClient, signAndSend],
  );

  const reset = useCallback(() => setState({ phase: "idle" }), []);
  const busy = state.phase === "signing" || state.phase === "confirming";
  return { state, run, reset, busy };
}

function explorerAction(hash: string) {
  return { label: "View", onClick: () => window.open(txUrl(hash), "_blank", "noopener,noreferrer") };
}
