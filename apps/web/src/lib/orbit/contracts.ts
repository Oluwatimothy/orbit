/**
 * Typed, validated reads from Orbit contracts. Writes are built in
 * `transactions.ts` and signed by the user's wallet.
 */
import { z } from "zod";

import { network } from "@/config/network";
import { view } from "@/lib/near/rpc";

import {
  allocationSchema,
  claimInfoSchema,
  claimRecordSchema,
  collectionConfigSchema,
  factoryConfigSchema,
  ftMetadataSchema,
  launchSchema,
  mintStatsSchema,
  nftMetadataSchema,
  nftTokenSchema,
  saleInfoSchema,
  type LaunchKind,
} from "./schemas";

export class FactoryNotConfiguredError extends Error {
  constructor() {
    super("NEXT_PUBLIC_ORBIT_FACTORY is not set, so there is no launch registry to read.");
  }
}

export function factoryId(): string {
  if (!network.factoryId) throw new FactoryNotConfiguredError();
  return network.factoryId;
}

async function read<S extends z.ZodType>(
  schema: S,
  contractId: string,
  method: string,
  args: Record<string, unknown> = {},
) {
  const raw = await view<unknown>(contractId, method, args);
  const parsed = schema.safeParse(raw);
  if (!parsed.success) {
    throw new Error(`Unexpected response from ${contractId}.${method}: ${z.prettifyError(parsed.error)}`);
  }
  return parsed.data as z.infer<S>;
}

const u128String = z.string().regex(/^\d+$/).transform(BigInt);

export const factory = {
  config: () => read(factoryConfigSchema, factoryId(), "get_config"),
  count: () => read(z.number().int(), factoryId(), "get_launch_count"),
  launch: (id: number) => read(launchSchema.nullable(), factoryId(), "get_launch", { id }),
  launches: (fromIndex: number, limit: number) =>
    read(z.array(launchSchema), factoryId(), "get_launches", { from_index: fromIndex, limit }),
  byCreator: (accountId: string, fromIndex = 0, limit = 50) =>
    read(z.array(launchSchema), factoryId(), "get_launches_by_creator", {
      account_id: accountId,
      from_index: fromIndex,
      limit,
    }),
  quote: (kind: LaunchKind) => read(u128String, factoryId(), "quote_deposit", { kind }),
  isSlugAvailable: (slug: string) => read(z.boolean(), factoryId(), "is_slug_available", { slug }),
};

export const token = {
  metadata: (id: string) => read(ftMetadataSchema, id, "ft_metadata"),
  totalSupply: (id: string) => read(u128String, id, "ft_total_supply"),
  balanceOf: (id: string, accountId: string) =>
    read(u128String, id, "ft_balance_of", { account_id: accountId }),
  allocations: (id: string) => read(z.array(allocationSchema), id, "orbit_allocations"),
  storageBalanceOf: (id: string, accountId: string) =>
    read(z.object({ total: z.string() }).nullable(), id, "storage_balance_of", { account_id: accountId }),
};

export const nft = {
  config: (id: string) => read(collectionConfigSchema, id, "orbit_config"),
  stats: (id: string) => read(mintStatsSchema, id, "orbit_mint_stats"),
  metadata: (id: string) => read(nftMetadataSchema, id, "nft_metadata"),
  mintedBy: (id: string, accountId: string) =>
    read(z.number().int(), id, "orbit_minted_by", { account_id: accountId }),
  tokens: (id: string, fromIndex: number, limit: number) =>
    read(z.array(nftTokenSchema), id, "nft_tokens", { from_index: String(fromIndex), limit }),
  tokensForOwner: (id: string, accountId: string, fromIndex = 0, limit = 50) =>
    read(z.array(nftTokenSchema), id, "nft_tokens_for_owner", {
      account_id: accountId,
      from_index: String(fromIndex),
      limit,
    }),
  token: (id: string, tokenId: string) =>
    read(nftTokenSchema.nullable(), id, "nft_token", { token_id: tokenId }),
};

export const claim = {
  info: (id: string) => read(claimInfoSchema, id, "get_info"),
  claims: (id: string, nftTokenIds: string[]) =>
    read(z.array(claimRecordSchema.nullable()), id, "get_claims", { nft_token_ids: nftTokenIds }),
};

export const sale = {
  info: (id: string) => read(saleInfoSchema, id, "get_info"),
  quote: (id: string, amount: bigint) => read(u128String, id, "quote", { amount: amount.toString() }),
  purchasedByNft: (id: string, nftTokenIds: string[]) =>
    Promise.all(
      nftTokenIds.map((nftTokenId) =>
        read(u128String, id, "get_purchased_by_nft", { nft_token_id: nftTokenId }),
      ),
    ),
  purchasedByAccount: (id: string, accountId: string) =>
    read(u128String, id, "get_purchased_by_account", { account_id: accountId }),
};
