import { describe, expect, it } from "vitest";

import {
  buildCollectionConfig,
  collectionFormSchema,
  mediaSchema,
  resolveAllocations,
  resolveSale,
  resolveTiers,
  slugify,
  slugSchema,
  type AllocationRow,
  type CollectionForm,
} from "./launch";

const row = (accountId: string, mode: AllocationRow["mode"], value: string): AllocationRow => ({
  label: accountId.split(".")[0]!,
  accountId,
  mode,
  value,
});

describe("resolveAllocations", () => {
  it("accepts amounts that add up exactly", () => {
    const result = resolveAllocations("10,000,000", 18, [
      row("creator.testnet", "amount", "8000000"),
      row("treasury.testnet", "amount", "2000000"),
    ]);
    expect(result.error).toBeNull();
    expect(result.allocations[0]!.amount).toBe((8_000_000n * 10n ** 18n).toString());
  });

  it("reports the unallocated remainder", () => {
    const result = resolveAllocations("100", 0, [row("a.testnet", "amount", "60")]);
    expect(result.error).toBe("Allocations must add up to the total supply");
    expect(result.unallocated).toBe(40n);
  });

  it("rejects over-allocation, zero rows, duplicates and excess precision", () => {
    expect(resolveAllocations("100", 0, [row("a.testnet", "amount", "101")]).error).toMatch(/exceed/);
    expect(resolveAllocations("100", 0, [row("a.testnet", "amount", "0")]).error).toMatch(
      /greater than zero/,
    );
    expect(
      resolveAllocations("100", 0, [row("a.testnet", "amount", "50"), row("a.testnet", "amount", "50")])
        .error,
    ).toMatch(/more than once/);
    expect(resolveAllocations("100", 2, [row("a.testnet", "amount", "1.001")]).error).toMatch(
      /decimal places/,
    );
  });

  it("assigns percentage rounding dust to the first percentage row", () => {
    const result = resolveAllocations("100", 0, [
      row("a.testnet", "percent", "33.33"),
      row("b.testnet", "percent", "33.33"),
      row("c.testnet", "percent", "33.34"),
    ]);
    expect(result.error).toBeNull();
    expect(result.dust).toBe(1n);
    expect(result.allocations.map((a) => a.amount)).toEqual(["34", "33", "33"]);
  });

  it("does not hide a genuine gap as rounding", () => {
    const result = resolveAllocations("1000", 0, [row("a.testnet", "percent", "50")]);
    expect(result.error).not.toBeNull();
    expect(result.dust).toBe(0n);
  });

  it("combines fixed allocations (the claim reserve) with user rows", () => {
    const reserve = { account_id: "genesis-claim.factory.testnet", amount: "2000", label: "NFT holders" };
    const result = resolveAllocations("10000", 0, [row("creator.testnet", "percent", "80")], [reserve]);
    expect(result.error).toBeNull();
    expect(result.allocations[0]).toEqual(reserve);
  });

  it("never uses floating point for huge supplies", () => {
    const result = resolveAllocations("340282366920938463463", 18, [row("a.testnet", "percent", "100")]);
    expect(result.error).toBeNull();
    expect(result.allocations[0]!.amount).toBe("340282366920938463463000000000000000000");
  });
});

describe("resolveTiers", () => {
  it("computes the exact reserve", () => {
    const result = resolveTiers(
      [
        { name: "Genesis", from: "1", to: "1000", amount: "1000" },
        { name: "Founders", from: "1001", to: "2000", amount: "5000" },
      ],
      2000,
      18,
    );
    expect(result.error).toBeNull();
    expect(result.reserved).toBe((1000n * 1000n + 1000n * 5000n) * 10n ** 18n);
  });

  it("rejects overlap and out-of-range IDs", () => {
    expect(
      resolveTiers(
        [
          { name: "A", from: "1", to: "10", amount: "1" },
          { name: "B", from: "10", to: "20", amount: "1" },
        ],
        20,
        0,
      ).error,
    ).toMatch(/overlaps/);
    expect(resolveTiers([{ name: "A", from: "1", to: "21", amount: "1" }], 20, 0).error).toMatch(
      /between 1 and 20/,
    );
    expect(resolveTiers([{ name: "A", from: "1", to: "2", amount: "0" }], 20, 0).error).toMatch(
      /greater than zero/,
    );
  });
});

describe("collection form", () => {
  const base: CollectionForm = {
    slug: "genesis",
    name: "Genesis",
    symbol: "GEN",
    description: "Genesis pass",
    cover: "ipfs://bafycover",
    mediaMode: "shared",
    baseUri: "",
    extension: "png",
    supplyMode: "fixed",
    maxSupply: "2000",
    price: "0.5",
    startsAt: "",
    endsAt: "",
    maxPerWallet: "",
    publicMint: true,
  };

  it("builds contract arguments in yoctoNEAR and nanoseconds", () => {
    expect(collectionFormSchema.safeParse(base).success).toBe(true);
    const config = buildCollectionConfig(base);
    expect(config.price).toBe("500000000000000000000000");
    expect(config.supply).toEqual({ kind: "fixed", max: 2000 });
    expect(config.starts_at_ns).toBe("0");
    expect(config.media).toEqual({ kind: "shared", media: "ipfs://bafycover", media_hash: null });
  });

  it("rejects bad supply, windows and directory URIs", () => {
    expect(collectionFormSchema.safeParse({ ...base, maxSupply: "10001" }).success).toBe(false);
    expect(
      collectionFormSchema.safeParse({ ...base, startsAt: "2030-01-02T00:00", endsAt: "2030-01-01T00:00" })
        .success,
    ).toBe(false);
    expect(
      collectionFormSchema.safeParse({ ...base, mediaMode: "directory", baseUri: "ipfs://bafy" }).success,
    ).toBe(false);
    expect(collectionFormSchema.safeParse({ ...base, supplyMode: "unlimited", maxSupply: "" }).success).toBe(
      true,
    );
  });
});

describe("identifiers", () => {
  it("slugifies names within the factory's rules", () => {
    expect(slugify("Orbit Genesis!")).toBe("orbit-genesis");
    expect(slugSchema.safeParse(slugify("A very long collection name indeed")).success).toBe(true);
    expect(slugSchema.safeParse("a--b").success).toBe(false);
  });

  it("accepts only ipfs and https media", () => {
    expect(mediaSchema.safeParse("ipfs://bafy").success).toBe(true);
    expect(mediaSchema.safeParse("javascript:alert(1)").success).toBe(false);
    expect(mediaSchema.safeParse("https://").success).toBe(false);
  });
});

describe("resolveSale", () => {
  const base = {
    price: "0.1",
    total: "100000",
    perNft: "100",
    holderStartsAt: "",
    publicEnabled: false,
    publicStartsAt: "",
    perAccount: "",
    endsAt: "2030-01-01T00:00",
  };

  it("builds a holder-only sale in base units", () => {
    const result = resolveSale(base, 18);
    expect(result.error).toBeNull();
    expect(result.config?.price_per_token).toBe("100000000000000000000000");
    expect(result.total).toBe(100_000n * 10n ** 18n);
    expect(result.config?.public_starts_at_ns).toBeNull();
  });

  it("requires ordered windows and a limit for the public window", () => {
    expect(resolveSale({ ...base, endsAt: "" }, 18).error).toMatch(/ends/);
    expect(
      resolveSale({ ...base, publicEnabled: true, publicStartsAt: "2031-01-01T00:00", perAccount: "5" }, 18)
        .error,
    ).toMatch(/before the end/);
    expect(
      resolveSale({ ...base, publicEnabled: true, publicStartsAt: "2029-01-01T00:00", perAccount: "" }, 18)
        .error,
    ).toMatch(/per-account/);
    expect(
      resolveSale({ ...base, publicEnabled: true, publicStartsAt: "2029-01-01T00:00", perAccount: "5" }, 18)
        .error,
    ).toBeNull();
    expect(resolveSale({ ...base, price: "0" }, 18).error).toMatch(/price/);
  });
});
