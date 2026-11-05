/**
 * Integer-exact conversion between human amounts ("1,234.5") and on-chain
 * base units (bigint). Never uses floating point.
 */

export const YOCTO_DECIMALS = 24;

const DECIMAL_PATTERN = /^\d+(\.\d+)?$/;

export class AmountError extends Error {}

/** Parses a decimal string into base units. Rejects more fraction digits than `decimals`. */
export function parseUnits(input: string, decimals: number): bigint {
  const value = input.trim().replaceAll(",", "");
  if (!DECIMAL_PATTERN.test(value)) {
    throw new AmountError("Enter a positive number");
  }
  const [whole = "0", fraction = ""] = value.split(".");
  if (fraction.length > decimals) {
    throw new AmountError(`At most ${decimals} decimal places`);
  }
  return BigInt(whole) * 10n ** BigInt(decimals) + BigInt(fraction.padEnd(decimals, "0") || "0");
}

export function tryParseUnits(input: string, decimals: number): bigint | null {
  try {
    return parseUnits(input, decimals);
  } catch {
    return null;
  }
}

export interface FormatOptions {
  /** Maximum fraction digits shown; the value is truncated, never rounded up. */
  maxFractionDigits?: number;
  /** Group the whole part with thousands separators. */
  grouping?: boolean;
}

/** Formats base units as a decimal string, truncating extra fraction digits. */
export function formatUnits(amount: bigint, decimals: number, options: FormatOptions = {}): string {
  const { maxFractionDigits = decimals, grouping = true } = options;
  const negative = amount < 0n;
  const absolute = negative ? -amount : amount;
  const base = 10n ** BigInt(decimals);
  const whole = absolute / base;
  const fraction = (absolute % base)
    .toString()
    .padStart(decimals, "0")
    .slice(0, maxFractionDigits)
    .replace(/0+$/, "");
  const wholeText = grouping ? whole.toLocaleString("en-US") : whole.toString();
  return `${negative ? "-" : ""}${wholeText}${fraction ? `.${fraction}` : ""}`;
}

/** Compact display for large token amounts, e.g. `10M`, `2.5K`. Exact below 10,000. */
export function formatCompact(amount: bigint, decimals: number): string {
  const whole = amount / 10n ** BigInt(decimals);
  const units: Array<[bigint, string]> = [
    [10n ** 12n, "T"],
    [10n ** 9n, "B"],
    [10n ** 6n, "M"],
    [10n ** 3n, "K"],
  ];
  for (const [size, suffix] of units) {
    if (whole >= size && whole >= 10_000n) {
      const scaled = (amount * 10n) / (size * 10n ** BigInt(decimals));
      const text = `${scaled / 10n}${scaled % 10n ? `.${scaled % 10n}` : ""}`;
      return `${text}${suffix}`;
    }
  }
  return formatUnits(amount, decimals, { maxFractionDigits: 4 });
}

export function formatNear(yocto: bigint, maxFractionDigits = 4): string {
  return `${formatUnits(yocto, YOCTO_DECIMALS, { maxFractionDigits })} NEAR`;
}

/** `part / total` as a percentage string with two decimals, computed in integers. */
export function formatShare(part: bigint, total: bigint): string {
  if (total === 0n) return "0%";
  const basisPoints = (part * 10_000n) / total;
  const text = `${basisPoints / 100n}.${(basisPoints % 100n).toString().padStart(2, "0")}`;
  return `${text.replace(/\.?0+$/, "")}%`;
}

/** Base units for `percent` (up to two decimals) of `total`, rounded down. */
export function shareOf(total: bigint, percent: string): bigint {
  const basisPoints = parseUnits(percent, 2);
  if (basisPoints > 10_000n) throw new AmountError("Percentage cannot exceed 100");
  return (total * basisPoints) / 10_000n;
}
