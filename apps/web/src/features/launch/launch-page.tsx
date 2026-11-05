"use client";

import { ArrowLeft } from "lucide-react";
import Link from "next/link";

import { Address } from "@/components/ui/data";
import { ErrorState, Notice, Skeleton } from "@/components/ui/status";
import { KindBadge, LaunchImage, StatusBadge, timeAgo } from "@/features/launches/launch-display";
import { useLaunch } from "@/features/launches/queries";
import { FactoryMissingNotice } from "@/features/launches/registry-state";
import { network } from "@/config/network";
import { useNow } from "@/lib/hooks/use-now";
import type { Launch } from "@/lib/orbit/schemas";

import { ActivityTable } from "@/features/activity/activity-table";

import { ClaimSection } from "./claim-section";
import { CollectionSection } from "./collection-section";
import { SaleSection } from "./sale-section";
import { TokenSection } from "./token-section";

function Contracts({ launch }: { launch: Launch }) {
  const rows = [
    { label: "Token (NEP-141)", id: launch.token },
    { label: "Collection (NEP-171)", id: launch.nft },
    { label: "Holder claims", id: launch.claim },
    { label: "Holder sale", id: launch.sale },
  ].filter((row): row is { label: string; id: string } => Boolean(row.id));
  return (
    <dl className="divide-y divide-line rounded-md border border-line">
      {rows.map((row) => (
        <div key={row.id} className="flex items-center justify-between gap-3 px-4 py-2.5">
          <dt className="text-xs text-ink-muted">{row.label}</dt>
          <dd className="min-w-0">
            <Address accountId={row.id} short={false} />
          </dd>
        </div>
      ))}
      <div className="flex items-center justify-between gap-3 px-4 py-2.5">
        <dt className="text-xs text-ink-muted">Creator</dt>
        <dd className="min-w-0">
          <Address accountId={launch.creator} short={false} />
        </dd>
      </div>
    </dl>
  );
}

function Header({ launch }: { launch: Launch }) {
  const now = useNow();
  return (
    <header className="flex flex-col gap-5 border-b border-line pb-6 md:flex-row md:items-start">
      <LaunchImage
        src={launch.image}
        alt={`${launch.name} artwork`}
        fallback={launch.symbol}
        className="size-20 md:size-24"
      />
      <div className="min-w-0 flex-1 space-y-2">
        <div className="flex flex-wrap items-center gap-2">
          <KindBadge kind={launch.kind} />
          <StatusBadge status={launch.status} />
          <span className="text-xs text-ink-faint">Launched {timeAgo(launch.created_at_ns, now)}</span>
        </div>
        <h1 className="text-2xl font-semibold tracking-tight text-ink">
          {launch.name} <span className="tabular text-base font-normal text-ink-faint">{launch.symbol}</span>
        </h1>
        <p className="max-w-2xl whitespace-pre-line text-sm text-ink-muted">{launch.description}</p>
      </div>
    </header>
  );
}

export function LaunchPage({ id }: { id: number }) {
  const launch = useLaunch(id);

  return (
    <div className="mx-auto max-w-6xl space-y-8 px-4 py-6 lg:px-8 lg:py-8">
      <Link
        href="/explore"
        className="inline-flex items-center gap-1.5 text-xs text-ink-muted hover:text-ink"
      >
        <ArrowLeft className="size-3.5" aria-hidden /> Explore
      </Link>
      {!network.factoryId ? (
        <FactoryMissingNotice />
      ) : launch.isPending ? (
        <div className="space-y-4">
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-64 w-full" />
        </div>
      ) : launch.isError ? (
        <ErrorState title="Launch not available" error={launch.error} onRetry={() => launch.refetch()} />
      ) : (
        <LaunchBody launch={launch.data} />
      )}
    </div>
  );
}

function LaunchBody({ launch }: { launch: Launch }) {
  return (
    <>
      <Header launch={launch} />
      {launch.status === "deploying" && (
        <Notice tone="caution">
          Contracts are being deployed. This page updates as soon as the factory records the result.
        </Notice>
      )}
      {launch.status === "failed" ? (
        <Notice tone="negative">
          Deployment failed and the account balances that were not used were refunded to the creator. The slug
          stays reserved in the registry.
        </Notice>
      ) : launch.status === "live" ? (
        <div className="space-y-12">
          {launch.kind === "connected" && launch.claim && (
            <ClaimSection claimId={launch.claim} collectionName={launch.name} />
          )}
          {launch.sale && <SaleSection saleId={launch.sale} collectionName={launch.name} />}
          {launch.nft && <CollectionSection nftId={launch.nft} />}
          {launch.token && <TokenSection tokenId={launch.token} highlight={launch.claim} />}
        </div>
      ) : null}
      {launch.status === "live" && (
        <section className="space-y-3" aria-labelledby="history-heading">
          <h2 id="history-heading" className="text-sm font-semibold text-ink">
            Transaction history
          </h2>
          <div className="rounded-md border border-line">
            <ActivityTable
              accounts={[launch.token, launch.nft, launch.claim, launch.sale].filter((a): a is string =>
                Boolean(a),
              )}
              queryKey={["launch", launch.id]}
              emptyText="Mints, claims and transfers involving this launch's contracts appear here."
            />
          </div>
          <p className="text-xs text-ink-faint">
            Indexed by FastNEAR; every row links to the transaction on NearBlocks.
          </p>
        </section>
      )}
      <section className="space-y-3" aria-labelledby="contracts-heading">
        <h2 id="contracts-heading" className="text-sm font-semibold text-ink">
          Contracts
        </h2>
        <Contracts launch={launch} />
        <p className="text-xs text-ink-faint">
          Launch accounts are created without access keys: nobody, including the creator and Orbit, can
          upgrade or re-initialise them.
        </p>
      </section>
    </>
  );
}
