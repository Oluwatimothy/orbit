"use client";

import { useQuery } from "@tanstack/react-query";
import { ChevronLeft, ChevronRight, ImageOff } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { SectionHeader, Stat } from "@/components/ui/data";
import { EmptyState, ErrorState, Notice, Skeleton } from "@/components/ui/status";
import { LaunchImage } from "@/features/launches/launch-display";
import { useWallet } from "@/features/wallet/wallet-provider";
import { formatNear } from "@/lib/format/units";
import { msToNs, useNow } from "@/lib/hooks/use-now";
import { shortAccount } from "@/lib/near/explorer";
import { nft } from "@/lib/orbit/contracts";
import { isContentAddressed } from "@/lib/orbit/media";
import type { CollectionConfig } from "@/lib/orbit/schemas";

import { GALLERY_PAGE_SIZE, useCollectionData, useGallery } from "./asset-queries";
import { MintPanel, mintWindow } from "./mint-panel";

function formatTime(ns: bigint): string {
  return new Date(Number(ns / 1_000_000n)).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function Gallery({ nftId, baseUri, symbol }: { nftId: string; baseUri: string | null; symbol: string }) {
  const [page, setPage] = useState(0);
  const tokens = useGallery(nftId, page);

  if (tokens.isPending) {
    return (
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
        {Array.from({ length: 8 }, (_, i) => (
          <Skeleton key={i} className="aspect-square w-full" />
        ))}
      </div>
    );
  }
  if (tokens.isError)
    return <ErrorState title="Could not load NFTs" error={tokens.error} onRetry={() => tokens.refetch()} />;
  if (tokens.data.length === 0 && page === 0) {
    return (
      <EmptyState
        icon={ImageOff}
        title="Nothing minted yet"
        description="Minted NFTs appear here with their current owner."
      />
    );
  }

  return (
    <div className="space-y-3">
      <ul className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
        {tokens.data.map((token) => (
          <li key={token.token_id} className="overflow-hidden rounded-md border border-line bg-surface">
            <LaunchImage
              src={token.metadata?.media}
              baseUri={baseUri}
              alt={token.metadata?.title ?? `#${token.token_id}`}
              fallback={symbol}
              className="aspect-square w-full rounded-none border-0"
            />
            <div className="space-y-0.5 border-t border-line px-3 py-2">
              <p className="truncate text-xs font-medium text-ink">
                {token.metadata?.title ?? `#${token.token_id}`}
              </p>
              <p className="tabular truncate text-2xs text-ink-faint" title={token.owner_id}>
                {shortAccount(token.owner_id, 22)}
              </p>
            </div>
          </li>
        ))}
      </ul>
      <div className="flex items-center justify-end gap-2">
        <Button
          size="sm"
          variant="ghost"
          disabled={page === 0}
          onClick={() => setPage(page - 1)}
          aria-label="Previous page"
        >
          <ChevronLeft className="size-4" aria-hidden />
        </Button>
        <span className="tabular text-xs text-ink-muted">Page {page + 1}</span>
        <Button
          size="sm"
          variant="ghost"
          disabled={tokens.data.length < GALLERY_PAGE_SIZE}
          onClick={() => setPage(page + 1)}
          aria-label="Next page"
        >
          <ChevronRight className="size-4" aria-hidden />
        </Button>
      </div>
    </div>
  );
}

function MintTerms({ config }: { config: CollectionConfig }) {
  const media = config.media.kind === "shared" ? config.media.media : config.media.base_uri;
  return (
    <ul className="space-y-1.5 text-xs text-ink-muted">
      <li>
        Window: {config.starts_at_ns === 0n ? "open from launch" : `opens ${formatTime(config.starts_at_ns)}`}
        {config.ends_at_ns !== null ? `, closes ${formatTime(config.ends_at_ns)}` : ", no end date"}
      </li>
      <li>Per wallet: {config.max_per_wallet === null ? "no limit" : `at most ${config.max_per_wallet}`}</li>
      <li>Price, supply and timing are fixed in the contract and cannot be changed by the creator.</li>
      {!isContentAddressed(media) && (
        <li className="text-caution">
          Media is served from an ordinary HTTPS URL, which its host can change or remove.
        </li>
      )}
    </ul>
  );
}

export function CollectionSection({ nftId }: { nftId: string }) {
  const { accountId } = useWallet();
  const data = useCollectionData(nftId);
  const now = useNow();
  const mintedByYou = useQuery({
    queryKey: ["nft", nftId, "minted-by", accountId],
    queryFn: () => nft.mintedBy(nftId, accountId ?? ""),
    enabled: Boolean(accountId),
  });

  if (data.isPending) return <Skeleton className="h-64 w-full" />;
  if (data.isError)
    return (
      <ErrorState
        title="Could not read the collection contract"
        error={data.error}
        onRetry={() => data.refetch()}
      />
    );

  const { config, stats, metadata } = data.data;
  const state = mintWindow(config, stats, msToNs(now));
  const remaining =
    stats.max_supply === null ? "Unlimited" : (stats.max_supply - stats.minted).toLocaleString();

  return (
    <section className="space-y-6" aria-labelledby="collection-heading">
      <SectionHeader
        id="collection-heading"
        title={`${config.name} collection`}
        description="NEP-171 · read from the collection contract"
      />
      <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
        <div className="space-y-5">
          <dl className="grid grid-cols-2 gap-x-6 gap-y-4 sm:grid-cols-4">
            <Stat label="Minted" value={stats.minted.toLocaleString()} />
            <Stat label="Max supply" value={stats.max_supply?.toLocaleString() ?? "Unlimited"} />
            <Stat label="Remaining" value={remaining} />
            <Stat label="Price" value={formatNear(config.price)} />
          </dl>
          {stats.max_supply !== null && (
            <div
              className="h-1 w-full overflow-hidden rounded-full bg-raised"
              role="progressbar"
              aria-label="Minted supply"
              aria-valuemin={0}
              aria-valuemax={stats.max_supply}
              aria-valuenow={stats.minted}
            >
              <div
                className="h-full bg-ink"
                style={{ width: `${(stats.minted / stats.max_supply) * 100}%` }}
              />
            </div>
          )}
          <MintTerms config={config} />
          {state === "sold-out" && (
            <Notice tone="positive">Sold out — {stats.minted.toLocaleString()} minted.</Notice>
          )}
        </div>
        <MintPanel nftId={nftId} config={config} stats={stats} mintedByYou={mintedByYou.data} />
      </div>
      <div className="space-y-3">
        <h3 className="text-sm font-semibold text-ink">Gallery</h3>
        <Gallery nftId={nftId} baseUri={metadata.base_uri} symbol={config.symbol} />
      </div>
    </section>
  );
}
