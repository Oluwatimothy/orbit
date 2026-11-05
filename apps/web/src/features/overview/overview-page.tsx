"use client";

import { ArrowRight, Coins, Image as ImageIcon, Waypoints } from "lucide-react";
import Link from "next/link";

import { Button } from "@/components/ui/button";
import { SectionHeader, Stat } from "@/components/ui/data";
import { Skeleton } from "@/components/ui/status";
import { network } from "@/config/network";
import { LaunchTable } from "@/features/launches/launch-display";
import { useAllLaunches } from "@/features/launches/queries";
import { RegistryState } from "@/features/launches/registry-state";
import type { Launch } from "@/lib/orbit/schemas";

const PATHS = [
  {
    href: "/create/token",
    icon: Coins,
    title: "Token",
    summary: "A fixed-supply NEP-141 token.",
    enforced: "Supply minted once to your allocations. No mint function exists afterwards.",
  },
  {
    href: "/create/collection",
    icon: ImageIcon,
    title: "NFT collection",
    summary: "A NEP-171 collection with a public mint.",
    enforced: "Price, supply cap, wallet limit and mint window are checked on every mint.",
  },
  {
    href: "/create/connected",
    icon: Waypoints,
    title: "Connected launch",
    summary: "A token and a collection whose holders can claim allocations.",
    enforced: "Each NFT claims its tier's allocation once, from a reserve funded at launch.",
  },
];

function PlatformStats({ launches, unavailable }: { launches: Launch[] | undefined; unavailable: boolean }) {
  const count = (kind?: Launch["kind"]) => launches?.filter((l) => !kind || l.kind === kind).length;
  const items = [
    { label: "Launches", value: count() },
    { label: "Tokens", value: count("token") },
    { label: "Collections", value: count("collection") },
    { label: "Connected", value: count("connected") },
  ];
  return (
    <dl className="grid grid-cols-2 gap-x-6 gap-y-4 sm:grid-cols-4">
      {items.map((item) => (
        <Stat
          key={item.label}
          label={item.label}
          value={unavailable ? "—" : (item.value ?? <Skeleton className="mt-1 h-4 w-8" />)}
        />
      ))}
    </dl>
  );
}

export function OverviewPage() {
  const launches = useAllLaunches();
  return (
    <div className="mx-auto max-w-6xl space-y-10 px-4 py-8 lg:px-8 lg:py-10">
      <section className="flex flex-col gap-6 md:flex-row md:items-end md:justify-between">
        <div className="max-w-xl space-y-3">
          <p className="text-2xs font-semibold uppercase tracking-[0.18em] text-ink-faint">
            The on-chain launch studio
          </p>
          <h1 className="text-2xl font-semibold leading-tight tracking-tight text-ink md:text-[28px]">
            Launch tokens, NFT collections, and the rules that connect them.
          </h1>
          <p className="text-sm text-ink-muted">
            Every launch is a set of standard NEAR contracts. The economics you configure — supply, price,
            allocations, holder claims — are enforced by those contracts, and every number on this site is
            read from them.
          </p>
        </div>
        <div className="flex gap-2">
          <Button asChild variant="primary" size="lg">
            <Link href="/create">Create a launch</Link>
          </Button>
          <Button asChild variant="secondary" size="lg">
            <Link href="/explore">Explore</Link>
          </Button>
        </div>
      </section>

      <section aria-label="Launch types" className="space-y-4">
        <div className="grid divide-y divide-line border-y border-line md:grid-cols-3 md:divide-x md:divide-y-0">
          {PATHS.map(({ href, icon: Icon, title, summary, enforced }) => (
            <Link
              key={href}
              href={href}
              className="group flex flex-col gap-3 p-5 transition-colors hover:bg-surface"
            >
              <div className="flex items-center justify-between">
                <span className="flex items-center gap-2 text-sm font-semibold text-ink">
                  <Icon className="size-4 text-ink-muted" aria-hidden /> {title}
                </span>
                <ArrowRight
                  className="size-4 text-ink-faint transition-transform group-hover:translate-x-0.5 group-hover:text-ink"
                  aria-hidden
                />
              </div>
              <p className="text-sm text-ink-muted">{summary}</p>
              <p className="mt-auto border-t border-line pt-3 text-xs text-ink-faint">
                <span className="font-medium text-ink-muted">Enforced on-chain: </span>
                {enforced}
              </p>
            </Link>
          ))}
        </div>
      </section>

      <section aria-labelledby="activity-stats" className="space-y-4">
        <SectionHeader
          id="activity-stats"
          title="Platform"
          description="Counted from the factory registry on testnet."
        />
        <PlatformStats launches={launches.data} unavailable={!network.factoryId || launches.isError} />
      </section>

      <section className="space-y-3">
        <SectionHeader
          title="Recent launches"
          action={
            <Link href="/explore" className="text-xs text-ink-muted hover:text-ink">
              View all
            </Link>
          }
        />
        <div className="rounded-md border border-line">
          <RegistryState query={launches}>
            {(items) => <LaunchTable launches={items.slice(0, 8)} caption="Recent launches" />}
          </RegistryState>
        </div>
      </section>
    </div>
  );
}
