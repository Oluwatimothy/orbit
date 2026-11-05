/** Mirrors `sale_cost` in contracts/common: price per whole token, rounded up. */
export function saleCost(amount: bigint, pricePerToken: bigint, decimals: number): bigint {
  const unit = 10n ** BigInt(decimals);
  const numerator = amount * pricePerToken;
  return numerator / unit + (numerator % unit === 0n ? 0n : 1n);
}
