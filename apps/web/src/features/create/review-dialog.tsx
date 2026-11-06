"use client";

import { X } from "lucide-react";
import { Dialog } from "radix-ui";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { KeyValue } from "@/components/ui/data";
import { Notice } from "@/components/ui/status";
import type { TransactionState } from "@/features/transactions/use-transaction";
import { formatNear } from "@/lib/format/units";

export interface ReviewItem {
  label: string;
  value: ReactNode;
}

/** Final confirmation before the wallet is asked to sign. */
export function ReviewDialog({
  open,
  onOpenChange,
  title,
  items,
  deposit,
  depositProblem = null,
  consequences,
  state,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  items: ReviewItem[];
  deposit: bigint | null;
  /** Why the deposit cannot be quoted, if it cannot. */
  depositProblem?: string | null;
  consequences: string[];
  state: TransactionState;
  onConfirm: () => void;
}) {
  const busy = state.phase === "signing" || state.phase === "confirming";
  return (
    <Dialog.Root open={open} onOpenChange={(next) => !busy && onOpenChange(next)}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/70 animate-fade-in" />
        <Dialog.Content className="fixed inset-x-4 top-[8vh] z-50 mx-auto max-h-[84vh] max-w-lg overflow-y-auto rounded-lg border border-line-strong bg-surface p-5 animate-fade-in">
          <div className="flex items-start justify-between gap-4">
            <div>
              <Dialog.Title className="text-base font-semibold text-ink">{title}</Dialog.Title>
              <Dialog.Description className="mt-0.5 text-xs text-ink-muted">
                Check every value. Once the transaction executes, these rules are fixed in the contracts.
              </Dialog.Description>
            </div>
            <Dialog.Close
              className="rounded-xs p-1 text-ink-faint hover:text-ink disabled:opacity-40"
              disabled={busy}
              aria-label="Close"
            >
              <X className="size-4" aria-hidden />
            </Dialog.Close>
          </div>

          <dl className="mt-4">
            {items.map((item) => (
              <KeyValue key={item.label} label={item.label}>
                {item.value}
              </KeyValue>
            ))}
            <KeyValue label="Deposit attached">
              <span className="tabular font-medium">
                {deposit !== null ? formatNear(deposit) : depositProblem ? "Unavailable" : "Loading…"}
              </span>
            </KeyValue>
          </dl>

          <ul className="mt-4 list-disc space-y-1 pl-4 text-xs text-ink-muted">
            {consequences.map((c) => (
              <li key={c}>{c}</li>
            ))}
          </ul>

          {deposit === null && depositProblem && (
            <div className="mt-4">
              <Notice tone="negative">{depositProblem}</Notice>
            </div>
          )}

          {state.phase === "failed" && (
            <div className="mt-4">
              <Notice tone="negative">{state.message}</Notice>
            </div>
          )}

          <div className="mt-5 flex justify-end gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={busy}>
              Back
            </Button>
            <Button variant="primary" onClick={onConfirm} loading={busy} disabled={deposit === null}>
              {state.phase === "signing"
                ? "Waiting for wallet"
                : state.phase === "confirming"
                  ? "Confirming"
                  : "Sign and launch"}
            </Button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
