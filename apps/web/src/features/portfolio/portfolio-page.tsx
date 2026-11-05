"use client";

import { Wallet } from "lucide-react";
import Link from "next/link";

import { Button } from "@/components/ui/button";
import { SectionHeader, Stat } from "@/components/ui/data";
import { Badge, EmptyState, ErrorState } from "@/components/ui/status";
import { ActivityTable } from "@/features/activity/activity-table";
import { LaunchImage, LaunchTable } from "@/features/launches/launch-display";
import { useAllLaunches, useCreatorLaunches } from "@/features/launches/queries";
import { FactoryMissingNotice, TableSkeleton } from "@/features/launches/registry-state";
import { useWallet } from "@/features/wallet/wallet-provider";
import { network } from "@/config/network";
import { formatUnits } from "@/lib/format/units";

import { useHoldings } from "./holdings";

export function ConnectPrompt({ title, description }: { title: string; description: string }) {
  const { connect, status } = useWallet();
  return (
    <EmptyState
      icon={Wallet}
      title={title}
      description={description}
      action={
        <Button
          variant="primary"
          size="sm"
          disabled={status === "initializing"}
          onClick={() => void connect()}
        >
          Connect wallet
        </Button>
      }
    />
  );
}

function Holdings({ accountId }: { accountId: string }) {
  const launches = useAllLaunches();
  const holdings = useHoldings(accountId, launches.data);

  if (launches.isError)
    return (
      <ErrorState
        title="Could not load the registry"
        error={launches.error}
        onRetry={() => launches.refetch()}
      />
    );
  if (holdings.isError)
    return (
      <ErrorState
        title="Could not read your balances"
        error={holdings.error}
        onRetry={() => holdings.refetch()}
      />
    );
  if (!holdings.data) return <TableSkeleton rows={3} />;

  const { tokens, collections } = holdings.data;
  const claimable = collections.reduce((n, c) => n + (c.claimable ?? 0), 0);
  const claimed = collections.reduce((n, c) => n + (c.claimed ?? 0), 0);

  return (
    <div className="space-y-8">
      <dl className="grid grid-cols-2 gap-x-6 gap-y-4 sm:grid-cols-4">
        <Stat label="Orbit tokens held" value={tokens.length} />
        <Stat label="NFTs held" value={collections.reduce((n, c) => n + c.tokenIds.length, 0)} />
        <Stat label="Claims available" value={claimable} />
        <Stat label="Claims completed" value={claimed} />
      </dl>

      <section className="space-y-3">
        <SectionHeader title="Tokens" description="Balances read from each token contract." />
        {tokens.length === 0 ? (
          <p className="rounded-md border border-line px-4 py-6 text-center text-xs text-ink-muted">
            You hold no Orbit tokens.
          </p>
        ) : (
          <ul className="divide-y divide-line rounded-md border border-line">
            {tokens.map((t) => (
              <li key={t.tokenId}>
                <Link
                  href={`/launches/${t.launch.id}`}
                  className="flex items-center gap-3 px-4 py-3 hover:bg-surface"
                >
                  <LaunchImage
                    src={t.metadata.icon ?? t.launch.image}
                    alt=""
                    fallback={t.metadata.symbol}
                    className="size-8"
                  />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm text-ink">{t.metadata.name}</span>
                    <span className="tabular block text-2xs text-ink-faint">{t.tokenId}</span>
                  </span>
                  <span className="tabular text-sm">
                    {formatUnits(t.balance, t.metadata.decimals, { maxFractionDigits: 4 })}{" "}
                    {t.metadata.symbol}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="space-y-3">
        <SectionHeader title="NFTs" description="Ownership read from each collection contract." />
        {collections.length === 0 ? (
          <p className="rounded-md border border-line px-4 py-6 text-center text-xs text-ink-muted">
            You hold no Orbit NFTs.
          </p>
        ) : (
          <ul className="divide-y divide-line rounded-md border border-line">
            {collections.map((c) => (
              <li key={c.nftId}>
                <Link
                  href={`/launches/${c.launch.id}`}
                  className="flex flex-wrap items-center gap-3 px-4 py-3 hover:bg-surface"
                >
                  <LaunchImage src={c.launch.image} alt="" fallback={c.launch.symbol} className="size-8" />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm text-ink">{c.launch.name}</span>
                    <span className="tabular block truncate text-2xs text-ink-faint">
                      {c.tokenIds
                        .slice(0, 8)
                        .map((id) => `#${id}`)
                        .join(" ")}
                      {c.tokenIds.length > 8 ? ` +${c.tokenIds.length - 8}` : ""}
                    </span>
                  </span>
                  {c.claimable !== null && c.claimable > 0 && (
                    <Badge tone="inverse">{c.claimable} to claim</Badge>
                  )}
                  {c.claimed !== null && c.claimed > 0 && <Badge tone="positive">{c.claimed} claimed</Badge>}
                  <span className="tabular text-sm">{c.tokenIds.length} held</span>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

export function PortfolioPage() {
  const { accountId } = useWallet();
  return (
    <div className="mx-auto max-w-6xl space-y-6 px-4 py-6 lg:px-8 lg:py-8">
      <div className="space-y-1">
        <h1 className="text-xl font-semibold tracking-tight text-ink">My assets</h1>
        <p className="text-sm text-ink-muted">
          Orbit tokens and NFTs held by your account, and the allocations you can claim.
        </p>
      </div>
      {!network.factoryId ? (
        <FactoryMissingNotice />
      ) : !accountId ? (
        <ConnectPrompt
          title="Connect a wallet to see your assets"
          description="Orbit never shows balances it has not read from the contracts for your account."
        />
      ) : (
        <>
          <Holdings accountId={accountId} />
          <section className="space-y-3">
            <SectionHeader title="Your recent transactions" />
            <div className="rounded-md border border-line">
              <ActivityTable
                accounts={[accountId]}
                queryKey={["account", accountId]}
                emptyText="Transactions signed by your account appear here."
              />
            </div>
          </section>
        </>
      )}
    </div>
  );
}

export function MyLaunchesPage() {
  const { accountId } = useWallet();
  const launches = useCreatorLaunches(accountId);
  return (
    <div className="mx-auto max-w-6xl space-y-6 px-4 py-6 lg:px-8 lg:py-8">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="space-y-1">
          <h1 className="text-xl font-semibold tracking-tight text-ink">My launches</h1>
          <p className="text-sm text-ink-muted">Launches created by your account.</p>
        </div>
        <Button asChild variant="primary">
          <Link href="/create">New launch</Link>
        </Button>
      </div>
      {!network.factoryId ? (
        <FactoryMissingNotice />
      ) : !accountId ? (
        <ConnectPrompt
          title="Connect a wallet to see your launches"
          description="Launches are listed by the account that created them."
        />
      ) : (
        <div className="rounded-md border border-line">
          {launches.isPending ? (
            <TableSkeleton />
          ) : launches.isError ? (
            <ErrorState
              title="Could not load your launches"
              error={launches.error}
              onRetry={() => launches.refetch()}
            />
          ) : launches.data.length === 0 ? (
            <EmptyState
              title="You have not launched anything yet"
              action={
                <Button asChild size="sm" variant="primary">
                  <Link href="/create">Create a launch</Link>
                </Button>
              }
            />
          ) : (
            <LaunchTable launches={launches.data} caption="Your launches" />
          )}
        </div>
      )}
    </div>
  );
}
