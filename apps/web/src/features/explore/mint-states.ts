"use client";

import { useQuery } from "@tanstack/react-query";

import { mintWindow, type MintWindow } from "@/features/launch/mint-panel";
import { nft } from "@/lib/orbit/contracts";
import type { Launch } from "@/lib/orbit/schemas";

export type MintFilter = "all" | "open" | "closed";

/**
 * Current mint state of every live collection, read from the collection
 * contracts. Only fetched while the mint filter is in use.
 */
export function useMintStates(launches: Launch[] | undefined, enabled: boolean) {
  const collections = (launches ?? []).filter((l) => l.status === "live" && l.nft);
  return useQuery({
    queryKey: ["mint-states", collections.map((l) => l.nft).join(",")],
    enabled: enabled && collections.length > 0,
    queryFn: async () => {
      const now = BigInt(Date.now()) * 1_000_000n;
      const entries = await Promise.all(
        collections.map(async (l): Promise<[string, MintWindow]> => {
          const [config, stats] = await Promise.all([nft.config(l.nft!), nft.stats(l.nft!)]);
          return [l.nft!, mintWindow(config, stats, now)];
        }),
      );
      return new Map(entries);
    },
  });
}

export function matchesMintFilter(
  launch: Launch,
  filter: MintFilter,
  states: Map<string, MintWindow> | undefined,
): boolean {
  if (filter === "all") return true;
  const state = launch.nft ? states?.get(launch.nft) : undefined;
  if (!state) return filter === "closed" && !launch.nft;
  return filter === "open" ? state === "open" : state !== "open";
}
