/**
 * Form schemas and builders for launch creation. The rules mirror
 * `contracts/common/src/validate.rs`; the contracts remain authoritative and
 * re-check everything. Amounts are parsed to bigint — never floats.
 */
import { z } from "zod";

import { parseUnits, shareOf, tryParseUnits, YOCTO_DECIMALS } from "@/lib/format/units";
import { isValidAccountId } from "@/lib/near/accounts";

export const LIMITS = {
  allocations: 10,
  tiers: 10,
  fixedSupply: 10_000,
  name: 64,
  label: 32,
  description: 1_000,
  url: 512,
  slug: { min: 2, max: 20 },
  symbol: { min: 2, max: 12 },
  decimals: 24,
} as const;

const text = (field: string, max: number) =>
  z
    .string()
    .trim()
    .min(1, `${field} is required`)
    .max(max, `${field} must be at most ${max} characters`)
    .refine((v) => !/[\u0000-\u001f\u007f]/.test(v), `${field} cannot contain control characters`);

export const slugSchema = z
  .string()
  .regex(/^[a-z0-9]+(-[a-z0-9]+)*$/, "Lowercase letters, digits and single hyphens")
  .min(LIMITS.slug.min, `At least ${LIMITS.slug.min} characters`)
  .max(LIMITS.slug.max, `At most ${LIMITS.slug.max} characters`);

export const symbolSchema = z
  .string()
  .regex(/^[A-Z0-9]+$/, "Uppercase letters and digits only")
  .min(LIMITS.symbol.min, `At least ${LIMITS.symbol.min} characters`)
  .max(LIMITS.symbol.max, `At most ${LIMITS.symbol.max} characters`);

const URL_CHARS = /^[A-Za-z0-9\-._~:/?#[\]@!$&()*+,;=%]+$/;

/** `ipfs://` or `https://` reference, as the contracts accept. */
export const mediaSchema = z
  .string()
  .trim()
  .max(LIMITS.url, `At most ${LIMITS.url} characters`)
  .refine(
    (v) =>
      (v.startsWith("ipfs://") || v.startsWith("https://")) &&
      URL_CHARS.test(v) &&
      !/^(ipfs|https):\/\/$/.test(v),
    "Use an ipfs:// or https:// URL",
  );

export const accountSchema = z.string().trim().refine(isValidAccountId, "Not a valid NEAR account ID");

/** A positive token amount with at most `decimals` fraction digits. */
const amountString = z
  .string()
  .trim()
  .regex(/^\d[\d,]*(\.\d+)?$/, "Enter a number");

export function slugify(name: string): string {
  return name
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, LIMITS.slug.max)
    .replace(/-+$/, "");
}

// ── Token ───────────────────────────────────────────────────────────────────

export const allocationRowSchema = z.object({
  label: text("Label", LIMITS.label),
  accountId: accountSchema,
  mode: z.enum(["amount", "percent"]),
  value: amountString,
});
export type AllocationRow = z.infer<typeof allocationRowSchema>;

export const tokenFormSchema = z.object({
  slug: slugSchema,
  name: text("Name", LIMITS.name),
  symbol: symbolSchema,
  description: text("Description", LIMITS.description),
  icon: mediaSchema.or(z.literal("")),
  decimals: z.number().int().min(0).max(LIMITS.decimals),
  totalSupply: amountString,
  allocations: z.array(allocationRowSchema).max(LIMITS.allocations),
});
export type TokenForm = z.infer<typeof tokenFormSchema>;

export interface ContractAllocation {
  account_id: string;
  amount: string;
  label: string;
}

export interface AllocationResult {
  allocations: ContractAllocation[];
  total: bigint;
  /** total − Σ allocations; must be exactly zero to launch. */
  unallocated: bigint;
  /** Rounding remainder from percentages that was added to the first percentage row. */
  dust: bigint;
  error: string | null;
}

/**
 * Resolves allocation rows (amounts or percentages) into exact base units.
 * Percentages round down; when percentage rows together with fixed rows
 * would cover the whole supply, the rounding remainder goes to the first
 * percentage row so the sum is exact.
 */
export function resolveAllocations(
  totalSupply: string,
  decimals: number,
  rows: AllocationRow[],
  fixed: ContractAllocation[] = [],
): AllocationResult {
  const empty = { allocations: [], total: 0n, unallocated: 0n, dust: 0n };
  const total = tryParseUnits(totalSupply, decimals);
  if (total === null || total === 0n) return { ...empty, error: "Enter a total supply greater than zero" };

  const resolved: ContractAllocation[] = [...fixed];
  let percentRows = 0;
  let firstPercentIndex = -1;
  for (const row of rows) {
    let amount: bigint;
    try {
      amount = row.mode === "percent" ? shareOf(total, row.value) : parseUnits(row.value, decimals);
    } catch (error) {
      return { ...empty, total, error: `${row.label || "Allocation"}: ${(error as Error).message}` };
    }
    if (amount === 0n)
      return { ...empty, total, error: `${row.label || "Allocation"} must be greater than zero` };
    if (row.mode === "percent") {
      percentRows += 1;
      if (firstPercentIndex < 0) firstPercentIndex = resolved.length;
    }
    resolved.push({ account_id: row.accountId.trim(), amount: amount.toString(), label: row.label.trim() });
  }

  // Each percentage rounds down by less than one base unit, so a shortfall
  // smaller than the number of percentage rows is rounding, not a gap.
  let sum = resolved.reduce((acc, a) => acc + BigInt(a.amount), 0n);
  let dust = 0n;
  const shortfall = total - sum;
  if (firstPercentIndex >= 0 && shortfall > 0n && shortfall < BigInt(percentRows)) {
    dust = shortfall;
    const target = resolved[firstPercentIndex]!;
    target.amount = (BigInt(target.amount) + dust).toString();
    sum = total;
  }

  const accounts = resolved.map((a) => a.account_id);
  const duplicate = accounts.find((a, i) => accounts.indexOf(a) !== i);
  if (resolved.length === 0)
    return { allocations: [], total, unallocated: total, dust, error: "Add at least one allocation" };
  if (resolved.length > LIMITS.allocations) {
    return {
      allocations: resolved,
      total,
      unallocated: total - sum,
      dust,
      error: `At most ${LIMITS.allocations} allocations`,
    };
  }
  if (duplicate)
    return {
      allocations: resolved,
      total,
      unallocated: total - sum,
      dust,
      error: `${duplicate} appears more than once`,
    };
  const error =
    sum === total
      ? null
      : sum > total
        ? "Allocations exceed the total supply"
        : "Allocations must add up to the total supply";
  return { allocations: resolved, total, unallocated: total - sum, dust, error };
}

export function buildTokenConfig(form: TokenForm, fixed: ContractAllocation[] = []) {
  const result = resolveAllocations(form.totalSupply, form.decimals, form.allocations, fixed);
  if (result.error) throw new Error(result.error);
  return {
    name: form.name.trim(),
    symbol: form.symbol,
    decimals: form.decimals,
    icon: form.icon || null,
    total_supply: result.total.toString(),
    allocations: result.allocations,
  };
}

// ── Collection ──────────────────────────────────────────────────────────────

export const collectionFormSchema = z
  .object({
    slug: slugSchema,
    name: text("Name", LIMITS.name),
    symbol: symbolSchema,
    description: text("Description", LIMITS.description),
    cover: mediaSchema,
    coverHash: z.string().optional(),
    mediaMode: z.enum(["shared", "directory"]),
    baseUri: z.string().trim(),
    extension: z.enum(["png", "jpg", "jpeg", "webp", "gif", "svg"]),
    supplyMode: z.enum(["fixed", "unlimited"]),
    maxSupply: z.string().trim(),
    price: amountString,
    startsAt: z.string(),
    endsAt: z.string(),
    maxPerWallet: z.string().trim(),
    publicMint: z.boolean(),
  })
  .superRefine((form, ctx) => {
    if (form.supplyMode === "fixed") {
      const max = Number(form.maxSupply);
      if (!/^\d+$/.test(form.maxSupply) || max < 1 || max > LIMITS.fixedSupply) {
        ctx.addIssue({
          code: "custom",
          path: ["maxSupply"],
          message: `Between 1 and ${LIMITS.fixedSupply.toLocaleString()}`,
        });
      }
    }
    if (
      form.mediaMode === "directory" &&
      !(form.baseUri.startsWith("https://") && URL_CHARS.test(form.baseUri) && !form.baseUri.endsWith("/"))
    ) {
      ctx.addIssue({
        code: "custom",
        path: ["baseUri"],
        message: "An https:// gateway folder URL without a trailing slash",
      });
    }
    if (form.maxPerWallet && (!/^\d+$/.test(form.maxPerWallet) || Number(form.maxPerWallet) < 1)) {
      ctx.addIssue({
        code: "custom",
        path: ["maxPerWallet"],
        message: "A whole number of at least 1, or leave empty",
      });
    }
    if (tryParseUnits(form.price, YOCTO_DECIMALS) === null) {
      ctx.addIssue({ code: "custom", path: ["price"], message: "Enter a price in NEAR (0 for a free mint)" });
    }
    const start = toNs(form.startsAt);
    const end = toNs(form.endsAt);
    if (form.endsAt && end === null)
      ctx.addIssue({ code: "custom", path: ["endsAt"], message: "Invalid date" });
    if (end !== null && end <= (start ?? 0n)) {
      ctx.addIssue({ code: "custom", path: ["endsAt"], message: "Must be after the start time" });
    }
  });
export type CollectionForm = z.infer<typeof collectionFormSchema>;

/** `datetime-local` value → nanoseconds since the epoch; empty → null. */
export function toNs(value: string): bigint | null {
  if (!value) return null;
  const ms = new Date(value).getTime();
  return Number.isFinite(ms) ? BigInt(ms) * 1_000_000n : null;
}

export function buildCollectionConfig(form: CollectionForm) {
  return {
    name: form.name.trim(),
    symbol: form.symbol,
    description: form.description.trim(),
    cover_media: form.cover,
    media:
      form.mediaMode === "shared"
        ? { kind: "shared", media: form.cover, media_hash: form.coverHash || null }
        : { kind: "directory", base_uri: form.baseUri, extension: form.extension },
    supply:
      form.supplyMode === "fixed" ? { kind: "fixed", max: Number(form.maxSupply) } : { kind: "unlimited" },
    price: parseUnits(form.price, YOCTO_DECIMALS).toString(),
    starts_at_ns: (toNs(form.startsAt) ?? 0n).toString(),
    ends_at_ns: toNs(form.endsAt)?.toString() ?? null,
    max_per_wallet: form.maxPerWallet ? Number(form.maxPerWallet) : null,
    public_mint: form.publicMint,
  };
}

// ── Holder claims ───────────────────────────────────────────────────────────

export const tierRowSchema = z.object({
  name: text("Tier name", LIMITS.label),
  from: z.string().regex(/^\d+$/, "Whole number"),
  to: z.string().regex(/^\d+$/, "Whole number"),
  amount: amountString,
});
export type TierRow = z.infer<typeof tierRowSchema>;

export interface TierResult {
  tiers: Array<{ name: string; from: number; to: number; amount_per_nft: string }>;
  reserved: bigint;
  error: string | null;
}

/** Mirrors `validate_claim_config`: non-overlapping ranges inside the supply, positive amounts. */
export function resolveTiers(rows: TierRow[], maxSupply: number, decimals: number): TierResult {
  if (rows.length === 0) return { tiers: [], reserved: 0n, error: "Add at least one tier" };
  if (rows.length > LIMITS.tiers) return { tiers: [], reserved: 0n, error: `At most ${LIMITS.tiers} tiers` };
  const tiers: TierResult["tiers"] = [];
  let reserved = 0n;
  for (const row of rows) {
    const from = Number(row.from);
    const to = Number(row.to);
    if (!Number.isInteger(from) || !Number.isInteger(to) || from < 1 || from > to || to > maxSupply) {
      return {
        tiers,
        reserved,
        error: `${row.name || "Tier"}: IDs must be between 1 and ${maxSupply}, with from ≤ to`,
      };
    }
    const amount = tryParseUnits(row.amount, decimals);
    if (!amount)
      return { tiers, reserved, error: `${row.name || "Tier"}: enter an amount per NFT greater than zero` };
    tiers.push({ name: row.name.trim(), from, to, amount_per_nft: amount.toString() });
    reserved += amount * BigInt(to - from + 1);
  }
  const sorted = [...tiers].sort((a, b) => a.from - b.from);
  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i]!.from <= sorted[i - 1]!.to) {
      return { tiers, reserved, error: `${sorted[i]!.name} overlaps ${sorted[i - 1]!.name}` };
    }
  }
  return { tiers, reserved, error: null };
}

// ── NFT-gated sale ──────────────────────────────────────────────────────────

export interface SaleForm {
  price: string;
  total: string;
  perNft: string;
  holderStartsAt: string;
  publicEnabled: boolean;
  publicStartsAt: string;
  perAccount: string;
  endsAt: string;
}

export interface SaleResult {
  config: {
    price_per_token: string;
    total_for_sale: string;
    per_nft_limit: string;
    holder_starts_at_ns: string;
    public_starts_at_ns: string | null;
    per_account_public_limit: string | null;
    ends_at_ns: string;
  } | null;
  total: bigint;
  error: string | null;
}

/** Mirrors `validate_sale_config`: positive amounts, ordered windows, a required end. */
export function resolveSale(form: SaleForm, decimals: number): SaleResult {
  const fail = (error: string, total = 0n): SaleResult => ({ config: null, total, error });
  const price = tryParseUnits(form.price, YOCTO_DECIMALS);
  if (!price) return fail("Enter a price per token greater than zero");
  const total = tryParseUnits(form.total, decimals);
  if (!total) return fail("Enter how many tokens are for sale");
  const perNft = tryParseUnits(form.perNft, decimals);
  if (!perNft) return fail("Enter a per-NFT purchase limit", total);
  const holderStart = toNs(form.holderStartsAt) ?? 0n;
  const end = toNs(form.endsAt);
  if (end === null) return fail("Set when the sale ends", total);
  let publicStart: bigint | null = null;
  let perAccount: bigint | null = null;
  if (form.publicEnabled) {
    publicStart = toNs(form.publicStartsAt);
    perAccount = tryParseUnits(form.perAccount, decimals);
    if (publicStart === null || publicStart <= holderStart || publicStart >= end) {
      return fail("The public window must start after the holder window and before the end", total);
    }
    if (!perAccount) return fail("Enter a per-account limit for the public window", total);
  } else if (end <= holderStart) {
    return fail("The sale must end after it starts", total);
  }
  return {
    total,
    error: null,
    config: {
      price_per_token: price.toString(),
      total_for_sale: total.toString(),
      per_nft_limit: perNft.toString(),
      holder_starts_at_ns: holderStart.toString(),
      public_starts_at_ns: publicStart?.toString() ?? null,
      per_account_public_limit: perAccount?.toString() ?? null,
      ends_at_ns: end.toString(),
    },
  };
}
