import { describe, expect, it } from "vitest";

import {
  AmountError,
  formatCompact,
  formatNear,
  formatShare,
  formatUnits,
  parseUnits,
  shareOf,
} from "./units";

describe("parseUnits", () => {
  it("converts decimals exactly", () => {
    expect(parseUnits("1", 24)).toBe(10n ** 24n);
    expect(parseUnits("0.000000000000000000000001", 24)).toBe(1n);
    expect(parseUnits("10,000,000", 18)).toBe(10_000_000n * 10n ** 18n);
    expect(parseUnits("1.5", 6)).toBe(1_500_000n);
  });

  it("handles amounts beyond Number precision", () => {
    expect(parseUnits("340282366920938.463463374607431768211455", 24).toString()).toBe(
      "340282366920938463463374607431768211455",
    );
  });

  it("rejects too many decimals, negatives and garbage", () => {
    expect(() => parseUnits("1.1234567", 6)).toThrow(AmountError);
    expect(() => parseUnits("-1", 6)).toThrow(AmountError);
    expect(() => parseUnits("1e18", 18)).toThrow(AmountError);
    expect(() => parseUnits("", 18)).toThrow(AmountError);
    expect(() => parseUnits("1.", 18)).toThrow(AmountError);
  });
});

describe("formatUnits", () => {
  it("formats with grouping and trims trailing zeros", () => {
    expect(formatUnits(1_234_500_000n, 6)).toBe("1,234.5");
    expect(formatUnits(10n ** 24n, 24)).toBe("1");
    expect(formatUnits(0n, 18)).toBe("0");
  });

  it("truncates rather than rounds", () => {
    expect(formatUnits(1_999_999n, 6, { maxFractionDigits: 2 })).toBe("1.99");
  });

  it("round-trips with parseUnits", () => {
    for (const value of ["0.1", "123456789.000001", "7"]) {
      expect(formatUnits(parseUnits(value, 6), 6, { grouping: false })).toBe(value);
    }
  });
});

describe("display helpers", () => {
  it("formats NEAR", () => {
    expect(formatNear(1_250_000_000_000_000_000_000n)).toBe("0.0012 NEAR");
  });

  it("compacts large amounts", () => {
    expect(formatCompact(10_000_000n * 10n ** 18n, 18)).toBe("10M");
    expect(formatCompact(2_500_000n, 0)).toBe("2.5M");
    expect(formatCompact(9_999n, 0)).toBe("9,999");
  });

  it("computes shares in integers", () => {
    expect(formatShare(2n, 10n)).toBe("20%");
    expect(formatShare(1n, 3n)).toBe("33.33%");
    expect(formatShare(0n, 0n)).toBe("0%");
    expect(shareOf(10_000_000n, "12.5")).toBe(1_250_000n);
    expect(() => shareOf(1n, "100.01")).toThrow(AmountError);
  });
});
