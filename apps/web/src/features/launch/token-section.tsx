"use client";

import { Address, SectionHeader, Stat } from "@/components/ui/data";
import { ErrorState, Skeleton } from "@/components/ui/status";
import { useWallet } from "@/features/wallet/wallet-provider";
import { formatCompact, formatShare, formatUnits } from "@/lib/format/units";
import type { Allocation } from "@/lib/orbit/schemas";

import { useTokenBalance, useTokenData } from "./asset-queries";

const SEGMENT_SHADES = ["bg-ink", "bg-ink-muted", "bg-ink-faint", "bg-line-strong"];

/** Proportional bar of the initial allocation; widths come from integer basis points. */
function AllocationBar({ allocations, total }: { allocations: Allocation[]; total: bigint }) {
  return (
    <div className="flex h-1.5 w-full gap-px overflow-hidden rounded-full bg-raised" aria-hidden>
      {allocations.map((a, i) => (
        <span
          key={a.account_id}
          className={SEGMENT_SHADES[i % SEGMENT_SHADES.length]}
          style={{ width: `${Number((a.amount * 10_000n) / total) / 100}%` }}
        />
      ))}
    </div>
  );
}

export function TokenSection({ tokenId, highlight }: { tokenId: string; highlight?: string | null }) {
  const { accountId } = useWallet();
  const data = useTokenData(tokenId);
  const balance = useTokenBalance(tokenId, accountId);

  if (data.isPending) return <Skeleton className="h-48 w-full" />;
  if (data.isError)
    return (
      <ErrorState
        title="Could not read the token contract"
        error={data.error}
        onRetry={() => data.refetch()}
      />
    );

  const { metadata, totalSupply, allocations } = data.data;
  const decimals = metadata.decimals;
  const allocated = allocations.reduce((sum, a) => sum + a.amount, 0n);

  return (
    <section className="space-y-5" aria-labelledby="token-heading">
      <SectionHeader
        id="token-heading"
        title={`${metadata.name} token`}
        description="NEP-141 · fixed supply · read from the token contract"
      />
      <dl className="grid grid-cols-2 gap-x-6 gap-y-4 md:grid-cols-4">
        <Stat label="Total supply" value={formatCompact(totalSupply, decimals)} sub={metadata.symbol} />
        <Stat label="Decimals" value={decimals} />
        <Stat label="Allocations" value={allocations.length} />
        <Stat
          label="Your balance"
          value={
            !accountId ? (
              "—"
            ) : balance.isPending ? (
              <Skeleton className="mt-1 h-4 w-16" />
            ) : balance.isError ? (
              "Unavailable"
            ) : (
              formatCompact(balance.data, decimals)
            )
          }
          sub={accountId ? metadata.symbol : "Connect a wallet"}
        />
      </dl>

      <div className="space-y-3">
        <AllocationBar allocations={allocations} total={totalSupply} />
        <div className="overflow-x-auto rounded-md border border-line">
          <table className="w-full min-w-[520px] text-left">
            <caption className="sr-only">Initial token allocation</caption>
            <thead>
              <tr className="border-b border-line text-2xs uppercase tracking-wider text-ink-faint">
                <th scope="col" className="py-2 pl-4 pr-3 font-medium">
                  Allocation
                </th>
                <th scope="col" className="px-3 py-2 font-medium">
                  Recipient
                </th>
                <th scope="col" className="px-3 py-2 text-right font-medium">
                  Amount
                </th>
                <th scope="col" className="py-2 pl-3 pr-4 text-right font-medium">
                  Share
                </th>
              </tr>
            </thead>
            <tbody>
              {allocations.map((a, i) => (
                <tr key={a.account_id} className="border-b border-line last:border-0">
                  <td className="py-2.5 pl-4 pr-3">
                    <span className="flex items-center gap-2 text-sm text-ink">
                      <span
                        className={`size-2 rounded-full ${SEGMENT_SHADES[i % SEGMENT_SHADES.length]}`}
                        aria-hidden
                      />
                      {a.label}
                      {a.account_id === highlight && (
                        <span className="text-2xs text-ink-faint">(holder reserve)</span>
                      )}
                    </span>
                  </td>
                  <td className="px-3 py-2.5">
                    <Address accountId={a.account_id} />
                  </td>
                  <td className="tabular px-3 py-2.5 text-right text-sm">
                    {formatUnits(a.amount, decimals, { maxFractionDigits: 2 })}
                  </td>
                  <td className="tabular py-2.5 pl-3 pr-4 text-right text-sm text-ink-muted">
                    {formatShare(a.amount, totalSupply)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="text-xs text-ink-faint">
          {allocated === totalSupply
            ? "The initial allocation accounts for the entire supply. Balances may have changed since through transfers."
            : `Current supply differs from the initial allocation by ${formatUnits(allocated - totalSupply, decimals)} ${metadata.symbol} — tokens were burned by unregistering storage.`}
        </p>
      </div>
    </section>
  );
}
