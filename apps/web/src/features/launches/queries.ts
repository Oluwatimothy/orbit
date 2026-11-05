"use client";

import { useQuery } from "@tanstack/react-query";

import { network } from "@/config/network";
import { factory } from "@/lib/orbit/contracts";
import type { Launch } from "@/lib/orbit/schemas";

const PAGE = 50;

export const launchKeys = {
  all: ["launches"] as const,
  detail: (id: number) => ["launches", id] as const,
  byCreator: (accountId: string) => ["launches", "creator", accountId] as const,
};

/**
 * Every launch in the registry, newest first. The registry is small at
 * hackathon scale; pages are fetched in parallel, 50 at a time (D-006).
 */
async function fetchAllLaunches(): Promise<Launch[]> {
  const count = await factory.count();
  const pages = Array.from({ length: Math.ceil(count / PAGE) }, (_, i) => factory.launches(i * PAGE, PAGE));
  return (await Promise.all(pages)).flat().reverse();
}

export function useAllLaunches() {
  return useQuery({
    queryKey: launchKeys.all,
    queryFn: fetchAllLaunches,
    enabled: Boolean(network.factoryId),
  });
}

export function useLaunch(id: number) {
  return useQuery({
    queryKey: launchKeys.detail(id),
    queryFn: async () => {
      const launch = await factory.launch(id);
      if (!launch) throw new Error(`Launch #${id} does not exist in the registry.`);
      return launch;
    },
    enabled: Boolean(network.factoryId) && Number.isInteger(id) && id >= 0,
    // A launch is "deploying" only for a few seconds; poll until it settles.
    refetchInterval: (query) => (query.state.data?.status === "deploying" ? 2_000 : false),
  });
}

export function useCreatorLaunches(accountId: string | null) {
  return useQuery({
    queryKey: launchKeys.byCreator(accountId ?? ""),
    queryFn: async () => (await factory.byCreator(accountId ?? "")).reverse(),
    enabled: Boolean(network.factoryId && accountId),
  });
}
