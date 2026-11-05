"use client";

import { useQuery } from "@tanstack/react-query";
import { ExternalLink, History } from "lucide-react";

import { Badge } from "@/components/ui/status";
import { EmptyState, ErrorState } from "@/components/ui/status";
import { TableSkeleton } from "@/features/launches/registry-state";
import { activityFor, type ActivityItem } from "@/lib/activity/fastnear";
import { shortAccount, txUrl } from "@/lib/near/explorer";

const ACTION_LABEL: Record<string, string> = {
  create_token_launch: "Created token launch",
  create_collection_launch: "Created collection launch",
  create_connected_launch: "Created connected launch",
  nft_mint: "Minted",
  creator_mint: "Creator minted",
  claim: "Claimed allocation",
  ft_transfer: "Transferred tokens",
  ft_transfer_call: "Transferred tokens",
  nft_transfer: "Transferred NFT",
  nft_transfer_call: "Transferred NFT",
  storage_deposit: "Registered storage",
  buy: "Bought tokens",
  withdraw_unsold: "Withdrew unsold tokens",
  set_paused: "Changed pause state",
};

/** Human description built from the method and the events the transaction emitted. */
export function describe(item: ActivityItem): string {
  const base = (item.method && ACTION_LABEL[item.method]) ?? item.method ?? "Transaction";
  const mint = item.events.find((e) => e.event === "nft_mint")?.data[0] as
    { token_ids?: string[] } | undefined;
  if (item.method === "nft_mint" && mint?.token_ids?.length) {
    return `${base} ${mint.token_ids.length === 1 ? `NFT #${mint.token_ids[0]}` : `${mint.token_ids.length} NFTs`}`;
  }
  const claim = item.events.find((e) => e.standard === "orbit" && e.event === "claim");
  const claimData = claim?.data[0] as { nft_token_id?: string } | undefined;
  if (claimData?.nft_token_id) return `Claimed for NFT #${claimData.nft_token_id}`;
  return base;
}

function timestamp(ms: number) {
  return new Date(ms).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

export function ActivityTable({
  accounts,
  queryKey,
  emptyText,
}: {
  accounts: string[];
  queryKey: readonly unknown[];
  emptyText: string;
}) {
  const activity = useQuery({
    queryKey: ["activity", ...queryKey],
    queryFn: () => activityFor(accounts),
    enabled: accounts.length > 0,
    refetchInterval: 30_000,
  });

  if (activity.isPending) return <TableSkeleton rows={4} />;
  if (activity.isError)
    return (
      <ErrorState title="Could not load activity" error={activity.error} onRetry={() => activity.refetch()} />
    );
  if (activity.data.length === 0)
    return <EmptyState icon={History} title="No activity yet" description={emptyText} />;

  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[560px] text-left">
        <caption className="sr-only">Transactions</caption>
        <thead>
          <tr className="border-b border-line text-2xs uppercase tracking-wider text-ink-faint">
            <th scope="col" className="py-2 pl-4 pr-3 font-medium">
              Action
            </th>
            <th scope="col" className="px-3 py-2 font-medium">
              Account
            </th>
            <th scope="col" className="px-3 py-2 font-medium">
              Status
            </th>
            <th scope="col" className="hidden px-3 py-2 font-medium md:table-cell">
              Time
            </th>
            <th scope="col" className="py-2 pl-3 pr-4 text-right font-medium">
              Transaction
            </th>
          </tr>
        </thead>
        <tbody>
          {activity.data.map((item) => (
            <tr key={item.hash} className="border-b border-line last:border-0">
              <td className="py-2.5 pl-4 pr-3 text-sm text-ink">{describe(item)}</td>
              <td className="tabular px-3 py-2.5 text-xs text-ink-muted" title={item.signer}>
                {shortAccount(item.signer, 22)}
              </td>
              <td className="px-3 py-2.5">
                <Badge tone={item.succeeded ? "positive" : "negative"}>
                  {item.succeeded ? "Success" : "Failed"}
                </Badge>
              </td>
              <td className="hidden px-3 py-2.5 text-xs text-ink-muted md:table-cell">
                {timestamp(item.timestampMs)}
              </td>
              <td className="py-2.5 pl-3 pr-4 text-right">
                <a
                  href={txUrl(item.hash)}
                  target="_blank"
                  rel="noreferrer"
                  className="tabular inline-flex items-center gap-1 text-xs text-ink-muted hover:text-ink"
                >
                  {item.hash.slice(0, 8)}… <ExternalLink className="size-3" aria-hidden />
                </a>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
