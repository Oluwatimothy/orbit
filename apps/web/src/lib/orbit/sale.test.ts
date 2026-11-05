import { describe, expect, it } from "vitest";

import { saleCost } from "./sale";

describe("saleCost", () => {
  const NEAR = 10n ** 24n;
  it("matches the contract, rounding up", () => {
    expect(saleCost(15n * 10n ** 17n, 2n * NEAR, 18)).toBe(3n * NEAR);
    expect(saleCost(1n, 1n, 18)).toBe(1n);
    expect(saleCost(10_000_000n * 10n ** 18n, 1_000n * NEAR, 18)).toBe(10_000_000_000n * NEAR);
  });
});
