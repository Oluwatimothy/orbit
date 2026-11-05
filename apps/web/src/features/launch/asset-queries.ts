"use client";

import { useQuery } from "@tanstack/react-query";

import { claim, nft, token } from "@/lib/orbit/contracts";

export const assetKeys = {
  token: (id: string) => ["token", id] as const,
  tokenBalance: (id: string, account: string) => ["token", id, "balance", account] as const,
  collection: (id: string) => ["nft", id] as const,
  gallery: (id: string, page: number) => ["nft", id, "gallery", page] as const,
  owned: (id: string, account: string) => ["nft", id, "owned", account] as const,
  claim: (id: string) => ["claim", id] as const,
  claimRecords: (id: string, account: string) => ["claim", id, "records", account] as const,
};

export function useTokenData(id: string) {
  return useQuery({
    queryKey: assetKeys.token(id),
    queryFn: async () => {
      const [metadata, totalSupply, allocations] = await Promise.all([
        token.metadata(id),
        token.totalSupply(id),
        token.allocations(id),
      ]);
      return { metadata, totalSupply, allocations };
    },
  });
}

export function useTokenBalance(id: string | null, accountId: string | null) {
  return useQuery({
    queryKey: assetKeys.tokenBalance(id ?? "", accountId ?? ""),
    queryFn: () => token.balanceOf(id ?? "", accountId ?? ""),
    enabled: Boolean(id && accountId),
  });
}

export function useCollectionData(id: string) {
  return useQuery({
    queryKey: assetKeys.collection(id),
    queryFn: async () => {
      const [config, stats, metadata] = await Promise.all([nft.config(id), nft.stats(id), nft.metadata(id)]);
      return { config, stats, metadata };
    },
  });
}

export const GALLERY_PAGE_SIZE = 24;

export function useGallery(id: string, page: number) {
  return useQuery({
    queryKey: assetKeys.gallery(id, page),
    queryFn: () => nft.tokens(id, page * GALLERY_PAGE_SIZE, GALLERY_PAGE_SIZE),
  });
}

export function useOwnedTokens(id: string, accountId: string | null) {
  return useQuery({
    queryKey: assetKeys.owned(id, accountId ?? ""),
    queryFn: () => nft.tokensForOwner(id, accountId ?? ""),
    enabled: Boolean(accountId),
  });
}

export function useClaimInfo(id: string) {
  return useQuery({ queryKey: assetKeys.claim(id), queryFn: () => claim.info(id) });
}
