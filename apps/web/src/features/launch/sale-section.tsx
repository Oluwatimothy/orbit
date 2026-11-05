"use client";

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { KeyValue, SectionHeader, Stat } from "@/components/ui/data";
import { Input } from "@/components/ui/field";
import { Badge, ErrorState, Notice, Skeleton, type Tone } from "@/components/ui/status";
import { useTransaction } from "@/features/transactions/use-transaction";
import { useWallet } from "@/features/wallet/wallet-provider";
import { formatCompact, formatNear, formatShare, formatUnits, tryParseUnits } from "@/lib/format/units";
import { sale } from "@/lib/orbit/contracts";
import { saleCost } from "@/lib/orbit/sale";
import type { FtMetadata, SaleInfo } from "@/lib/orbit/schemas";
import { buyTx, PURCHASE_MARGIN } from "@/lib/orbit/transactions";

import { useOwnedTokens, useTokenData } from "./asset-queries";

const PHASE: Record<SaleInfo["phase"], { label: string; tone: Tone }> = {
  not_started: { label: "Not started", tone: "neutral" },
  holders: { label: "Holder window", tone: "inverse" },
  public: { label: "Public window", tone: "positive" },
  ended: { label: "Ended", tone: "neutral" },
};

function when(ns: bigint) {
  return new Date(Number(ns / 1_000_000n)).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

const saleKey = (id: string) => ["sale", id] as const;

function useSale(saleId: string) {
  return useQuery({ queryKey: saleKey(saleId), queryFn: () => sale.info(saleId) });
}

/** Purchase form: in the holder window the buyer picks one of their NFTs. */
function BuyPanel({ saleId, info, metadata }: { saleId: string; info: SaleInfo; metadata: FtMetadata }) {
  const { accountId, connect, networkMismatch } = useWallet();
  const { run, busy } = useTransaction();
  const owned = useOwnedTokens(info.nft_id, accountId);
  const ownedIds = owned.data?.map((t) => t.token_id) ?? [];
  const holder = info.phase === "holders";
  const used = useQuery({
    queryKey: [...saleKey(saleId), "used", accountId, holder, ownedIds.join(",")],
    queryFn: async () =>
      holder
        ? sale.purchasedByNft(saleId, ownedIds)
        : [await sale.purchasedByAccount(saleId, accountId ?? "")],
    enabled: Boolean(accountId) && (!holder || ownedIds.length > 0),
  });
  const [selected, setSelected] = useState<string | null>(null);
  const [input, setInput] = useState("");

  if (!accountId) {
    return (
      <Button variant="primary" className="w-full" onClick={() => void connect()}>
        Connect wallet to buy
      </Button>
    );
  }
  if (owned.isPending || (used.isPending && used.fetchStatus !== "idle"))
    return <Skeleton className="h-24 w-full" />;
  if (holder && ownedIds.length === 0) {
    return (
      <Notice>The holder window is open to owners of this collection only. You hold none of its NFTs.</Notice>
    );
  }
  if (used.isError) return <ErrorState title="Could not read your purchase limits" error={used.error} />;

  const nftId = holder ? (selected ?? ownedIds[0] ?? null) : null;
  const usedAmount = holder ? (used.data?.[ownedIds.indexOf(nftId ?? "")] ?? 0n) : (used.data?.[0] ?? 0n);
  const limit = holder ? info.config.per_nft_limit : (info.config.per_account_public_limit ?? 0n);
  const remainingInSale = info.config.total_for_sale - info.sold;
  const remaining = [limit - usedAmount, remainingInSale].reduce((a, b) => (a < b ? a : b));
  const amount = tryParseUnits(input || "0", metadata.decimals) ?? -1n;
  const cost = amount > 0n ? saleCost(amount, info.config.price_per_token, metadata.decimals) : 0n;
  const error =
    amount < 0n
      ? `Enter an amount with at most ${metadata.decimals} decimals`
      : amount > remaining
        ? "Above your remaining limit"
        : null;

  async function submit() {
    await run(buyTx(saleId, amount, cost, nftId), {
      label: `Buy ${formatUnits(amount, metadata.decimals, { maxFractionDigits: 4 })} ${metadata.symbol}`,
      invalidate: [saleKey(saleId), ["token", info.token_id]],
    });
    setInput("");
  }

  return (
    <div className="space-y-3">
      {holder && (
        <label className="block space-y-1.5">
          <span className="text-xs text-ink-muted">Buy with NFT</span>
          <select
            value={nftId ?? ""}
            onChange={(e) => setSelected(e.target.value)}
            className="h-8 w-full rounded-sm border border-line-strong bg-surface px-2 text-sm"
          >
            {ownedIds.map((id) => (
              <option key={id} value={id}>
                #{id}
              </option>
            ))}
          </select>
        </label>
      )}
      <label className="block space-y-1.5">
        <span className="flex justify-between text-xs text-ink-muted">
          Amount
          <button
            type="button"
            className="tabular hover:text-ink"
            onClick={() => setInput(formatUnits(remaining, metadata.decimals, { grouping: false }))}
          >
            Max {formatUnits(remaining, metadata.decimals, { maxFractionDigits: 4 })}
          </button>
        </span>
        <Input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          inputMode="decimal"
          className="tabular"
          aria-invalid={Boolean(error)}
          placeholder="0"
        />
      </label>
      {error && <p className="text-xs text-negative">{error}</p>}
      <dl>
        <KeyValue label="Cost">{formatNear(cost)}</KeyValue>
        <KeyValue label="Attached">
          <span className="tabular font-medium">{formatNear(cost + PURCHASE_MARGIN)}</span>
        </KeyValue>
      </dl>
      <p className="text-xs text-ink-faint">
        The extra {formatNear(PURCHASE_MARGIN)} covers registering you with the token and storing your
        purchase record; the unused part is refunded. The ownership check, the token transfer and the payment
        to the creator happen in one step, so a rejected purchase changes nothing.
      </p>
      <Button
        variant="primary"
        className="w-full"
        loading={busy}
        disabled={networkMismatch || amount <= 0n || Boolean(error)}
        onClick={submit}
      >
        Buy {metadata.symbol}
      </Button>
    </div>
  );
}

function WithdrawUnsold({ saleId, info }: { saleId: string; info: SaleInfo }) {
  const { accountId } = useWallet();
  const { run, busy } = useTransaction();
  if (accountId !== info.creator_id || info.status !== "active" || info.phase !== "ended") return null;
  const unsold = info.config.total_for_sale - info.sold;
  if (unsold === 0n) return null;
  return (
    <Button
      variant="secondary"
      className="w-full"
      loading={busy}
      onClick={() =>
        run(
          {
            receiverId: saleId,
            actions: [
              { methodName: "withdraw_unsold", args: {}, deposit: "0", gas: (100n * 10n ** 12n).toString() },
            ],
          },
          { label: "Withdraw unsold tokens", invalidate: [saleKey(saleId)] },
        )
      }
    >
      Withdraw unsold tokens
    </Button>
  );
}

export function SaleSection({ saleId, collectionName }: { saleId: string; collectionName: string }) {
  const info = useSale(saleId);
  const token = useTokenData(info.data?.token_id ?? "");

  if (info.isPending || (info.data && token.isPending)) return <Skeleton className="h-64 w-full" />;
  if (info.isError)
    return <ErrorState title="Could not read the sale" error={info.error} onRetry={() => info.refetch()} />;
  if (token.isError) return <ErrorState title="Could not read the token" error={token.error} />;
  if (!token.data) return null;

  const data = info.data;
  const { metadata } = token.data;
  const { config } = data;
  const fmt = (v: bigint) => `${formatCompact(v, metadata.decimals)} ${metadata.symbol}`;
  const phase = PHASE[data.phase];
  const soldBasisPoints = Number((data.sold * 10_000n) / config.total_for_sale);

  return (
    <section className="space-y-6" aria-labelledby="sale-heading">
      <SectionHeader
        id="sale-heading"
        title="Holder sale"
        description={`Owners of ${collectionName} NFTs can buy ${metadata.symbol} first, at a fixed price.`}
        action={<Badge tone={phase.tone}>{phase.label}</Badge>}
      />
      <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
        <div className="space-y-5">
          <dl className="grid grid-cols-2 gap-x-6 gap-y-4 md:grid-cols-4">
            <Stat label="Price" value={formatNear(config.price_per_token)} sub={`per ${metadata.symbol}`} />
            <Stat label="Sold" value={fmt(data.sold)} sub={formatShare(data.sold, config.total_for_sale)} />
            <Stat label="For sale" value={fmt(config.total_for_sale)} />
            <Stat label="Per NFT" value={fmt(config.per_nft_limit)} />
          </dl>
          <div
            className="h-1 w-full overflow-hidden rounded-full bg-raised"
            role="progressbar"
            aria-label="Sold"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.floor(soldBasisPoints / 100)}
          >
            <div className="h-full bg-ink" style={{ width: `${soldBasisPoints / 100}%` }} />
          </div>
          <ul className="space-y-1.5 text-xs text-ink-muted">
            <li>
              Holder window opens{" "}
              {config.holder_starts_at_ns === 0n ? "at launch" : when(config.holder_starts_at_ns)}: each NFT
              may buy up to {fmt(config.per_nft_limit)}, checked against current ownership at purchase time.
            </li>
            <li>
              {config.public_starts_at_ns !== null && config.per_account_public_limit !== null
                ? `Public window opens ${when(config.public_starts_at_ns)}: any account may buy up to ${fmt(config.per_account_public_limit)}.`
                : "There is no public window: only NFT holders can buy."}
            </li>
            <li>Sale ends {when(config.ends_at_ns)}. Afterwards the creator can reclaim unsold tokens.</li>
          </ul>
        </div>
        <div className="space-y-4 rounded-md border border-line bg-surface p-4">
          <h3 className="text-sm font-semibold text-ink">Buy</h3>
          {data.status === "closed" ? (
            <Notice>The sale is closed.</Notice>
          ) : data.phase === "holders" || data.phase === "public" ? (
            <BuyPanel saleId={saleId} info={data} metadata={metadata} />
          ) : (
            <Notice>
              {data.phase === "not_started"
                ? `The sale opens ${when(config.holder_starts_at_ns)}.`
                : "The sale has ended."}
            </Notice>
          )}
          <WithdrawUnsold saleId={saleId} info={data} />
        </div>
      </div>
    </section>
  );
}
