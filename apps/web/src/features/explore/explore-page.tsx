"use client";

import { Search, SearchX } from "lucide-react";
import { useMemo, useState } from "react";

import { Input } from "@/components/ui/field";
import { EmptyState } from "@/components/ui/status";
import { cn } from "@/lib/cn";
import { KIND_LABEL, LaunchTable } from "@/features/launches/launch-display";
import { useAllLaunches } from "@/features/launches/queries";
import { RegistryState } from "@/features/launches/registry-state";

import { matchesMintFilter, useMintStates, type MintFilter } from "./mint-states";
import type { Launch, LaunchKind } from "@/lib/orbit/schemas";

type KindFilter = "all" | LaunchKind;
type StatusFilter = "all" | Launch["status"];

export function filterLaunches(
  launches: Launch[],
  kind: KindFilter,
  status: StatusFilter,
  query: string,
  order: "newest" | "oldest",
) {
  const q = query.trim().toLowerCase();
  const matches = launches.filter(
    (l) =>
      (kind === "all" || l.kind === kind) &&
      (status === "all" || l.status === status) &&
      (!q ||
        [l.name, l.symbol, l.slug, l.creator, l.description].some((field) =>
          field.toLowerCase().includes(q),
        )),
  );
  return order === "newest" ? matches : [...matches].reverse();
}

function Chips<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: Array<[T, string]>;
  onChange: (v: T) => void;
}) {
  return (
    <div role="radiogroup" aria-label={label} className="flex flex-wrap gap-1">
      {options.map(([key, text]) => (
        <button
          key={key}
          type="button"
          role="radio"
          aria-checked={value === key}
          onClick={() => onChange(key)}
          className={cn(
            "h-7 rounded-sm px-2.5 text-xs transition-colors",
            value === key ? "bg-raised text-ink" : "text-ink-muted hover:bg-raised/60 hover:text-ink",
          )}
        >
          {text}
        </button>
      ))}
    </div>
  );
}

export function ExplorePage() {
  const launches = useAllLaunches();
  const [kind, setKind] = useState<KindFilter>("all");
  const [status, setStatus] = useState<StatusFilter>("live");
  const [order, setOrder] = useState<"newest" | "oldest">("newest");
  const [query, setQuery] = useState("");
  const [minting, setMinting] = useState<MintFilter>("all");
  const mintStates = useMintStates(launches.data, minting !== "all");
  const filtered = useMemo(() => {
    const base = filterLaunches(launches.data ?? [], kind, status, query, order);
    return minting === "all" ? base : base.filter((l) => matchesMintFilter(l, minting, mintStates.data));
  }, [launches.data, kind, status, query, order, minting, mintStates.data]);

  return (
    <div className="mx-auto max-w-6xl space-y-5 px-4 py-6 lg:px-8 lg:py-8">
      <div className="space-y-1">
        <h1 className="text-xl font-semibold tracking-tight text-ink">Explore</h1>
        <p className="text-sm text-ink-muted">Every launch in the Orbit factory registry on testnet.</p>
      </div>

      <div className="flex flex-col gap-3 border-y border-line py-3 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
          <Chips
            label="Launch type"
            value={kind}
            onChange={setKind}
            options={[["all", "All types"], ...(Object.entries(KIND_LABEL) as Array<[LaunchKind, string]>)]}
          />
          <span className="hidden h-4 w-px bg-line sm:block" aria-hidden />
          <Chips
            label="Status"
            value={status}
            onChange={setStatus}
            options={[
              ["live", "Live"],
              ["all", "All"],
              ["failed", "Failed"],
            ]}
          />
          <span className="hidden h-4 w-px bg-line sm:block" aria-hidden />
          <Chips
            label="Minting"
            value={minting}
            onChange={setMinting}
            options={[
              ["all", "Any mint state"],
              ["open", "Minting now"],
              ["closed", "Not minting"],
            ]}
          />
          <span className="hidden h-4 w-px bg-line sm:block" aria-hidden />
          <Chips
            label="Order"
            value={order}
            onChange={setOrder}
            options={[
              ["newest", "Newest"],
              ["oldest", "Oldest"],
            ]}
          />
        </div>
        <div className="relative lg:w-72">
          <Search className="pointer-events-none absolute left-2.5 top-2 size-4 text-ink-faint" aria-hidden />
          <Input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Name, symbol, slug or creator"
            className="pl-8"
            aria-label="Search launches"
          />
        </div>
      </div>

      <div className="rounded-md border border-line">
        <RegistryState query={launches}>
          {() =>
            filtered.length ? (
              <LaunchTable launches={filtered} caption="Launches" />
            ) : (
              <EmptyState
                icon={SearchX}
                title="No launches match"
                description="Change the filters or search for something else."
              />
            )
          }
        </RegistryState>
      </div>
      {launches.data && (
        <p className="tabular text-xs text-ink-faint">
          {filtered.length} of {launches.data.length} launches
        </p>
      )}
    </div>
  );
}
