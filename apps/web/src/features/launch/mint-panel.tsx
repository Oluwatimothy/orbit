"use client";

import { Minus, Plus } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { KeyValue } from "@/components/ui/data";
import { Notice } from "@/components/ui/status";
import { useTransaction } from "@/features/transactions/use-transaction";
import { useWallet } from "@/features/wallet/wallet-provider";
import { formatNear } from "@/lib/format/units";
import { msToNs, useNow } from "@/lib/hooks/use-now";
import type { CollectionConfig, MintStats } from "@/lib/orbit/schemas";
import { MINT_STORAGE_ALLOWANCE, mintDeposit, mintTx } from "@/lib/orbit/transactions";

import { assetKeys } from "./asset-queries";

const MAX_PER_CALL = 10;

export type MintWindow = "paused" | "not-public" | "upcoming" | "open" | "ended" | "sold-out";

export function mintWindow(config: CollectionConfig, stats: MintStats, nowNs: bigint): MintWindow {
  if (stats.max_supply !== null && stats.minted >= stats.max_supply) return "sold-out";
  if (stats.paused) return "paused";
  if (!config.public_mint) return "not-public";
  if (nowNs < config.starts_at_ns) return "upcoming";
  if (config.ends_at_ns !== null && nowNs >= config.ends_at_ns) return "ended";
  return "open";
}

const WINDOW_MESSAGE: Record<Exclude<MintWindow, "open">, string> = {
  paused: "The creator has paused minting.",
  "not-public": "This collection is minted by its creator only.",
  upcoming: "Minting has not started yet.",
  ended: "The mint window has closed.",
  "sold-out": "Every NFT in this collection has been minted.",
};

export function MintPanel({
  nftId,
  config,
  stats,
  mintedByYou,
}: {
  nftId: string;
  config: CollectionConfig;
  stats: MintStats;
  mintedByYou: number | undefined;
}) {
  const { accountId, connect, networkMismatch } = useWallet();
  const { run, busy } = useTransaction();
  const [quantity, setQuantity] = useState(1);

  const now = useNow();
  const state = mintWindow(config, stats, msToNs(now));
  const remainingSupply = stats.max_supply === null ? Infinity : stats.max_supply - stats.minted;
  const remainingForWallet =
    config.max_per_wallet === null ? Infinity : config.max_per_wallet - (mintedByYou ?? 0);
  const maxQuantity = Math.max(0, Math.min(MAX_PER_CALL, remainingSupply, remainingForWallet));
  const clamped = Math.min(quantity, Math.max(1, maxQuantity));
  const priceTotal = config.price * BigInt(clamped);

  async function mint() {
    await run(mintTx(nftId, config.price, clamped), {
      label: `Mint ${clamped} ${config.symbol}`,
      invalidate: [assetKeys.collection(nftId), ["nft", nftId]],
    });
  }

  return (
    <div className="space-y-4 rounded-md border border-line bg-surface p-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-ink">Mint</h3>
        <span className="tabular text-xs text-ink-muted">{formatNear(config.price)} each</span>
      </div>

      {state !== "open" ? (
        <Notice>{WINDOW_MESSAGE[state]}</Notice>
      ) : (
        <>
          <div className="flex items-center justify-between gap-3">
            <span className="text-xs text-ink-muted">Quantity</span>
            <div className="flex items-center rounded-sm border border-line-strong">
              <button
                type="button"
                className="p-2 text-ink-muted hover:text-ink disabled:opacity-40"
                onClick={() => setQuantity(Math.max(1, clamped - 1))}
                disabled={clamped <= 1}
                aria-label="Decrease quantity"
              >
                <Minus className="size-3.5" aria-hidden />
              </button>
              <output className="tabular w-8 text-center text-sm" aria-live="polite">
                {clamped}
              </output>
              <button
                type="button"
                className="p-2 text-ink-muted hover:text-ink disabled:opacity-40"
                onClick={() => setQuantity(Math.min(maxQuantity, clamped + 1))}
                disabled={clamped >= maxQuantity}
                aria-label="Increase quantity"
              >
                <Plus className="size-3.5" aria-hidden />
              </button>
            </div>
          </div>

          <dl>
            <KeyValue label="Price">{formatNear(priceTotal)}</KeyValue>
            <KeyValue label="Storage allowance">
              <span className="text-ink-muted">
                up to {formatNear(MINT_STORAGE_ALLOWANCE * BigInt(clamped))}
              </span>
            </KeyValue>
            <KeyValue label="Attached">
              <span className="tabular font-medium">{formatNear(mintDeposit(config.price, clamped))}</span>
            </KeyValue>
          </dl>
          <p className="text-xs text-ink-faint">
            The contract measures the storage your NFTs use and refunds the unused allowance in the same
            transaction. The price goes to the creator.
          </p>

          {maxQuantity === 0 ? (
            <Notice tone="caution">
              You have reached this collection&apos;s limit of {config.max_per_wallet} per wallet.
            </Notice>
          ) : !accountId ? (
            <Button variant="primary" className="w-full" onClick={() => void connect()}>
              Connect wallet to mint
            </Button>
          ) : (
            <Button
              variant="primary"
              className="w-full"
              onClick={mint}
              loading={busy}
              disabled={networkMismatch}
            >
              Mint {clamped}
            </Button>
          )}
        </>
      )}
    </div>
  );
}
