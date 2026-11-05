"use client";

import { useQuery } from "@tanstack/react-query";

import { claim, nft, token } from "@/lib/orbit/contracts";
import type { FtMetadata, Launch } from "@/lib/orbit/schemas";

export interface TokenHolding {
  launch: Launch;
  tokenId: string;
  metadata: FtMetadata;
  balance: bigint;
}

export interface CollectionHolding {
  launch: Launch;
  nftId: string;
  tokenIds: string[];
  /** For connected launches: how many held NFTs are still unclaimed and in a tier. */
  claimable: number | null;
  claimed: number | null;
}

const CONCURRENCY = 6;

async function mapLimited<T, R>(items: T[], fn: (item: T) => Promise<R>): Promise<R[]> {
  const results: R[] = new Array(items.length);
  let next = 0;
  async function worker() {
    while (next < items.length) {
      const index = next++;
      results[index] = await fn(items[index]!);
    }
  }
  await Promise.all(Array.from({ length: Math.min(CONCURRENCY, items.length) }, worker));
  return results;
}

/**
 * Reads the account's balance in every live Orbit token and its NFTs in every
 * live Orbit collection directly from the contracts. No third-party index is
 * trusted for holdings.
 */
export async function fetchHoldings(accountId: string, launches: Launch[]) {
  const live = launches.filter((l) => l.status === "live");
  const tokens = await mapLimited(
    live.filter((l) => l.token),
    async (launch): Promise<TokenHolding | null> => {
      const tokenId = launch.token!;
      const balance = await token.balanceOf(tokenId, accountId);
      if (balance === 0n) return null;
      return { launch, tokenId, balance, metadata: await token.metadata(tokenId) };
    },
  );
  const collections = await mapLimited(
    live.filter((l) => l.nft),
    async (launch): Promise<CollectionHolding | null> => {
      const nftId = launch.nft!;
      const owned = await nft.tokensForOwner(nftId, accountId);
      if (owned.length === 0) return null;
      const tokenIds = owned.map((t) => t.token_id);
      if (!launch.claim) return { launch, nftId, tokenIds, claimable: null, claimed: null };
      const [info, records] = await Promise.all([
        claim.info(launch.claim),
        claim.claims(launch.claim, tokenIds.slice(0, 50)),
      ]);
      const inTier = (id: string) => info.tiers.some((t) => t.from <= Number(id) && Number(id) <= t.to);
      const claimed = records.filter((r) => r?.state === "claimed").length;
      const claimable = tokenIds.slice(0, 50).filter((id, i) => inTier(id) && !records[i]).length;
      return { launch, nftId, tokenIds, claimable, claimed };
    },
  );
  return {
    tokens: tokens.filter((t): t is TokenHolding => t !== null),
    collections: collections.filter((c): c is CollectionHolding => c !== null),
  };
}

export function useHoldings(accountId: string | null, launches: Launch[] | undefined) {
  return useQuery({
    queryKey: ["holdings", accountId, launches?.length],
    queryFn: () => fetchHoldings(accountId ?? "", launches ?? []),
    enabled: Boolean(accountId && launches),
  });
}
