"use client";

import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { toast } from "sonner";

import { useWallet } from "@/features/wallet/wallet-provider";
import { txUrl } from "@/lib/near/explorer";
import { confirmTransaction } from "@/lib/near/rpc";

/**
 * Wallets that complete signing with a full-page redirect return
 * `?transactionHashes=…` or `?errorCode=…`. Each hash is confirmed on-chain
 * before anything is reported, then the parameters are removed from the URL.
 */
export function parseRedirect(search: string): { hashes: string[]; error: string | null } {
  const params = new URLSearchParams(search);
  const hashes = (params.get("transactionHashes") ?? "")
    .split(",")
    .map((h) => h.trim())
    .filter((h) => /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(h));
  const errorCode = params.get("errorCode");
  return { hashes, error: errorCode ? (params.get("errorMessage") ?? errorCode) : null };
}

export function RedirectOutcome() {
  const { accountId, status } = useWallet();
  const queryClient = useQueryClient();
  const handled = useRef(false);

  useEffect(() => {
    if (handled.current || status === "initializing") return;
    const { hashes, error } = parseRedirect(window.location.search);
    if (hashes.length === 0 && !error) return;
    handled.current = true;

    const url = new URL(window.location.href);
    ["transactionHashes", "errorCode", "errorMessage"].forEach((key) => url.searchParams.delete(key));
    window.history.replaceState(null, "", url);

    if (error) {
      toast.error("Transaction not completed", { description: decodeURIComponent(error) });
      return;
    }
    if (!accountId) {
      hashes.forEach((hash) =>
        toast.message("Transaction submitted", {
          description: "Reconnect your wallet to confirm the result here, or check the explorer.",
          action: { label: "View", onClick: () => window.open(txUrl(hash), "_blank", "noopener,noreferrer") },
        }),
      );
      return;
    }
    for (const hash of hashes) {
      const id = toast.loading("Confirming transaction on testnet…");
      confirmTransaction(hash, accountId)
        .then(async (summary) => {
          await queryClient.invalidateQueries();
          const action = {
            label: "View",
            onClick: () => window.open(txUrl(hash), "_blank", "noopener,noreferrer"),
          };
          if (summary.succeeded) toast.success("Transaction confirmed", { id, action });
          else toast.error("Transaction failed", { id, description: summary.failure ?? undefined, action });
        })
        .catch((e: unknown) => toast.error("Could not confirm transaction", { id, description: String(e) }));
    }
  }, [accountId, status, queryClient]);

  return null;
}
