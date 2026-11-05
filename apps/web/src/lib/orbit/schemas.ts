/**
 * Runtime schemas for Orbit contract view results. Mirrors the Rust types in
 * `contracts/*`; u128 values arrive as decimal strings and become bigint.
 */
import { z } from "zod";

const u128 = z
  .string()
  .regex(/^\d+$/)
  .transform((value) => BigInt(value));
const u64 = u128;

export const launchKindSchema = z.enum(["token", "collection", "connected"]);
export type LaunchKind = z.infer<typeof launchKindSchema>;

export const launchSchema = z.object({
  id: z.number().int().nonnegative(),
  kind: launchKindSchema,
  slug: z.string(),
  name: z.string(),
  symbol: z.string(),
  description: z.string(),
  image: z.string().nullable(),
  creator: z.string(),
  created_at_ns: u64,
  status: z.enum(["deploying", "live", "failed"]),
  token: z.string().nullable(),
  nft: z.string().nullable(),
  claim: z.string().nullable(),
  sale: z.string().nullable(),
});
export type Launch = z.infer<typeof launchSchema>;

export const factoryConfigSchema = z.object({
  owner_id: z.string(),
  code: z.object({ token: z.string(), nft: z.string(), claim: z.string(), sale: z.string() }),
  paused: z.boolean(),
  launch_count: z.number().int().nonnegative(),
});
export type FactoryConfig = z.infer<typeof factoryConfigSchema>;

export const ftMetadataSchema = z.object({
  spec: z.string(),
  name: z.string(),
  symbol: z.string(),
  icon: z.string().nullable().optional(),
  decimals: z.number().int().min(0).max(24),
});
export type FtMetadata = z.infer<typeof ftMetadataSchema>;

export const allocationSchema = z.object({ account_id: z.string(), amount: u128, label: z.string() });
export type Allocation = z.infer<typeof allocationSchema>;

export const supplySchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("fixed"), max: z.number().int() }),
  z.object({ kind: z.literal("unlimited") }),
]);

export const tokenMediaSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("shared"), media: z.string(), media_hash: z.string().nullable() }),
  z.object({ kind: z.literal("directory"), base_uri: z.string(), extension: z.string() }),
]);

export const collectionConfigSchema = z.object({
  name: z.string(),
  symbol: z.string(),
  description: z.string(),
  cover_media: z.string(),
  media: tokenMediaSchema,
  supply: supplySchema,
  price: u128,
  starts_at_ns: u64,
  ends_at_ns: u64.nullable(),
  max_per_wallet: z.number().int().nullable(),
  public_mint: z.boolean(),
});
export type CollectionConfig = z.infer<typeof collectionConfigSchema>;

export const mintStatsSchema = z.object({
  minted: z.number().int(),
  max_supply: z.number().int().nullable(),
  paused: z.boolean(),
  owner_id: z.string(),
});
export type MintStats = z.infer<typeof mintStatsSchema>;

export const nftMetadataSchema = z.object({
  name: z.string(),
  symbol: z.string(),
  base_uri: z.string().nullable(),
});

export const nftTokenSchema = z.object({
  token_id: z.string(),
  owner_id: z.string(),
  metadata: z
    .object({
      title: z.string().nullable().optional(),
      media: z.string().nullable().optional(),
    })
    .nullable()
    .optional(),
});
export type NftToken = z.infer<typeof nftTokenSchema>;

export const tierSchema = z.object({
  name: z.string(),
  from: z.number().int(),
  to: z.number().int(),
  amount_per_nft: u128,
});
export type Tier = z.infer<typeof tierSchema>;

export const claimInfoSchema = z.object({
  token_id: z.string(),
  nft_id: z.string(),
  creator_id: z.string(),
  tiers: z.array(tierSchema),
  starts_at_ns: u64,
  status: z.enum(["awaiting_funding", "active"]),
  total_reserved: u128,
  claimed_total: u128,
  pending_total: u128,
  claims_count: z.number().int(),
});
export type ClaimInfo = z.infer<typeof claimInfoSchema>;

export const claimRecordSchema = z.object({
  claimer: z.string(),
  amount: u128,
  state: z.enum(["pending", "claimed"]),
  updated_at_ns: u64,
});
export type ClaimRecord = z.infer<typeof claimRecordSchema>;

export const saleConfigSchema = z.object({
  price_per_token: u128,
  total_for_sale: u128,
  per_nft_limit: u128,
  holder_starts_at_ns: u64,
  public_starts_at_ns: u64.nullable(),
  per_account_public_limit: u128.nullable(),
  ends_at_ns: u64,
});

export const saleInfoSchema = z.object({
  token_id: z.string(),
  nft_id: z.string(),
  creator_id: z.string(),
  decimals: z.number().int(),
  config: saleConfigSchema,
  status: z.enum(["awaiting_funding", "active", "closed"]),
  phase: z.enum(["not_started", "holders", "public", "ended"]),
  sold: u128,
  pending: u128,
});
export type SaleInfo = z.infer<typeof saleInfoSchema>;
