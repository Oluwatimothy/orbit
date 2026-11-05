"use client";

import { useQuery } from "@tanstack/react-query";
import { ArrowRight, CheckCircle2, Wallet } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Address, SectionHeader, Stat } from "@/components/ui/data";
import { Badge, EmptyState, ErrorState, Notice, Skeleton } from "@/components/ui/status";
import { useTransaction } from "@/features/transactions/use-transaction";
import { useWallet } from "@/features/wallet/wallet-provider";
import { formatCompact, formatNear, formatShare, formatUnits } from "@/lib/format/units";
import { msToNs, useNow } from "@/lib/hooks/use-now";
import { claim } from "@/lib/orbit/contracts";
import type { ClaimInfo, FtMetadata, Tier } from "@/lib/orbit/schemas";
import { CLAIM_DEPOSIT, claimTx } from "@/lib/orbit/transactions";

import { assetKeys, useClaimInfo, useOwnedTokens, useTokenData } from "./asset-queries";

export function allocationFor(tiers: Tier[], tokenId: string): { tier: Tier; amount: bigint } | null {
  if (!/^[1-9]\d*$/.test(tokenId)) return null;
  const id = Number(tokenId);
  const tier = tiers.find((t) => t.from <= id && id <= t.to);
  return tier ? { tier, amount: tier.amount_per_nft } : null;
}

function Relationship({ collection, symbol, info }: { collection: string; symbol: string; info: ClaimInfo }) {
  const nodes = [
    { label: "NFT collection", value: collection, account: info.nft_id },
    { label: "Holder reserve", value: "Held by the launch", account: null },
    { label: "Token", value: symbol, account: info.token_id },
  ];
  return (
    <div className="grid items-stretch gap-2 md:grid-cols-[1fr_auto_1fr_auto_1fr]">
      {nodes.map((node, i) => (
        <div key={node.label} className="contents">
          <div
            className={`rounded-md border px-4 py-3 ${i === 1 ? "border-ink-faint/50 bg-raised" : "border-line bg-surface"}`}
          >
            <p className="text-2xs uppercase tracking-wider text-ink-faint">{node.label}</p>
            <p className="mt-1 truncate text-sm font-medium text-ink">{node.value}</p>
            {node.account && (
              <p className="tabular mt-0.5 truncate text-2xs text-ink-faint">{node.account}</p>
            )}
          </div>
          {i < nodes.length - 1 && (
            <div className="flex items-center justify-center text-ink-faint" aria-hidden>
              <ArrowRight className={`size-4 rotate-90 md:rotate-0 ${i === 1 ? "md:rotate-180" : ""}`} />
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

function TierTable({ tiers, metadata }: { tiers: Tier[]; metadata: FtMetadata }) {
  return (
    <div className="overflow-x-auto rounded-md border border-line">
      <table className="w-full min-w-[480px] text-left">
        <caption className="sr-only">Allocation tiers</caption>
        <thead>
          <tr className="border-b border-line text-2xs uppercase tracking-wider text-ink-faint">
            <th scope="col" className="py-2 pl-4 pr-3 font-medium">
              Tier
            </th>
            <th scope="col" className="px-3 py-2 font-medium">
              NFT IDs
            </th>
            <th scope="col" className="px-3 py-2 text-right font-medium">
              Per NFT
            </th>
            <th scope="col" className="py-2 pl-3 pr-4 text-right font-medium">
              Tier total
            </th>
          </tr>
        </thead>
        <tbody>
          {tiers.map((tier) => (
            <tr key={tier.name} className="border-b border-line last:border-0">
              <td className="py-2.5 pl-4 pr-3 text-sm text-ink">{tier.name}</td>
              <td className="tabular px-3 py-2.5 text-sm text-ink-muted">
                #{tier.from}
                {tier.to !== tier.from && `–#${tier.to}`}
              </td>
              <td className="tabular px-3 py-2.5 text-right text-sm">
                {formatUnits(tier.amount_per_nft, metadata.decimals, { maxFractionDigits: 4 })}{" "}
                {metadata.symbol}
              </td>
              <td className="tabular py-2.5 pl-3 pr-4 text-right text-sm text-ink-muted">
                {formatCompact(tier.amount_per_nft * BigInt(tier.to - tier.from + 1), metadata.decimals)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function YourClaims({ claimId, info, metadata }: { claimId: string; info: ClaimInfo; metadata: FtMetadata }) {
  const { accountId, connect, networkMismatch } = useWallet();
  const owned = useOwnedTokens(info.nft_id, accountId);
  const ids = owned.data?.map((t) => t.token_id) ?? [];
  const records = useQuery({
    queryKey: assetKeys.claimRecords(info.nft_id, `${accountId}:${ids.join(",")}`),
    queryFn: () => claim.claims(claimId, ids),
    enabled: ids.length > 0,
  });
  const { run, busy } = useTransaction();
  const now = useNow();

  if (!accountId) {
    return (
      <EmptyState
        icon={Wallet}
        title="Connect a wallet to see your eligibility"
        description="Orbit reads the NFTs your account holds from the launch contract and checks each one against its claim records."
        action={
          <Button variant="primary" size="sm" onClick={() => void connect()}>
            Connect wallet
          </Button>
        }
      />
    );
  }
  if (owned.isPending || (ids.length > 0 && records.isPending)) return <Skeleton className="h-24 w-full" />;
  if (owned.isError)
    return (
      <ErrorState title="Could not read your NFTs" error={owned.error} onRetry={() => owned.refetch()} />
    );
  if (records.isError)
    return (
      <ErrorState
        title="Could not read claim records"
        error={records.error}
        onRetry={() => records.refetch()}
      />
    );
  if (ids.length === 0) {
    return (
      <EmptyState
        title="You hold no NFTs from this collection"
        description="Mint or acquire one, then return here to claim its allocation."
      />
    );
  }

  const notStarted = msToNs(now) < info.starts_at_ns;

  async function submit(tokenId: string, amount: bigint) {
    await run(claimTx(claimId, tokenId), {
      label: `Claim ${formatUnits(amount, metadata.decimals, { maxFractionDigits: 2 })} ${metadata.symbol} for NFT #${tokenId}`,
      invalidate: [assetKeys.claim(claimId), ["claim", info.nft_id], ["token", info.token_id]],
    });
  }

  return (
    <div className="overflow-x-auto rounded-md border border-line">
      <table className="w-full min-w-[480px] text-left">
        <caption className="sr-only">Your NFTs and their claims</caption>
        <thead>
          <tr className="border-b border-line text-2xs uppercase tracking-wider text-ink-faint">
            <th scope="col" className="py-2 pl-4 pr-3 font-medium">
              NFT
            </th>
            <th scope="col" className="px-3 py-2 font-medium">
              Tier
            </th>
            <th scope="col" className="px-3 py-2 text-right font-medium">
              Allocation
            </th>
            <th scope="col" className="py-2 pl-3 pr-4 text-right font-medium">
              Status
            </th>
          </tr>
        </thead>
        <tbody>
          {ids.map((tokenId, i) => {
            const allocation = allocationFor(info.tiers, tokenId);
            const record = records.data?.[i] ?? null;
            return (
              <tr key={tokenId} className="border-b border-line last:border-0">
                <td className="tabular py-2.5 pl-4 pr-3 text-sm">#{tokenId}</td>
                <td className="px-3 py-2.5 text-sm text-ink-muted">
                  {allocation?.tier.name ?? "Not eligible"}
                </td>
                <td className="tabular px-3 py-2.5 text-right text-sm">
                  {allocation
                    ? `${formatUnits(allocation.amount, metadata.decimals, { maxFractionDigits: 2 })} ${metadata.symbol}`
                    : "—"}
                </td>
                <td className="py-2.5 pl-3 pr-4 text-right">
                  {record ? (
                    <Badge tone="positive">
                      <CheckCircle2 className="size-3" aria-hidden /> Claimed
                    </Badge>
                  ) : allocation && !notStarted ? (
                    <Button
                      size="sm"
                      variant="primary"
                      loading={busy}
                      disabled={networkMismatch}
                      onClick={() => submit(tokenId, allocation.amount)}
                    >
                      Claim
                    </Button>
                  ) : (
                    <span className="text-xs text-ink-faint">{allocation ? "Not open" : "—"}</span>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function ClaimStatus({ startsAt }: { startsAt: bigint }) {
  const now = useNow();
  const open = msToNs(now) >= startsAt;
  return <Badge tone={open ? "positive" : "caution"}>{open ? "Claims open" : "Not started"}</Badge>;
}

export function ClaimSection({ claimId, collectionName }: { claimId: string; collectionName: string }) {
  const info = useClaimInfo(claimId);
  const tokenId = info.data?.token_id;
  const token = useTokenData(tokenId ?? "");

  if (info.isPending || (tokenId && token.isPending)) return <Skeleton className="h-72 w-full" />;
  if (info.isError)
    return (
      <ErrorState title="Could not read holder claims" error={info.error} onRetry={() => info.refetch()} />
    );
  if (token.isError)
    return (
      <ErrorState title="Could not read the token" error={token.error} onRetry={() => token.refetch()} />
    );
  if (!token.data) return null;

  const data = info.data;
  const { metadata } = token.data;
  const remaining = data.total_reserved - data.claimed_total;
  const fmt = (value: bigint) => `${formatCompact(value, metadata.decimals)} ${metadata.symbol}`;

  return (
    <section className="space-y-6" aria-labelledby="claim-heading">
      <SectionHeader
        id="claim-heading"
        title="Holder allocation"
        description={`Each eligible ${collectionName} NFT can claim its tier's ${metadata.symbol} allocation once.`}
        action={<ClaimStatus startsAt={data.starts_at_ns} />}
      />
      <Relationship collection={collectionName} symbol={metadata.symbol} info={data} />

      <dl className="grid grid-cols-2 gap-x-6 gap-y-4 md:grid-cols-3">
        <Stat label="Reserved" value={fmt(data.total_reserved)} />
        <Stat
          label="Claimed"
          value={fmt(data.claimed_total)}
          sub={`${formatShare(data.claimed_total, data.total_reserved)} · ${data.claims_count} ${data.claims_count === 1 ? "claim" : "claims"}`}
        />
        <Stat label="Unclaimed" value={fmt(remaining)} />
      </dl>

      <TierTable tiers={data.tiers} metadata={metadata} />

      <Notice>
        <strong className="font-medium text-ink">How eligibility works.</strong> Ownership is checked at the
        moment you claim — not from a snapshot. Whoever holds an NFT can claim its allocation, once. If an NFT
        is sold before it is claimed, the buyer can claim it; after a claim, that NFT can never claim again.
        The ownership check, the claim record and the token transfer happen in one step: a rejected claim
        changes nothing. Claiming attaches {formatNear(CLAIM_DEPOSIT)}; the contract keeps only the storage
        the claim adds (about 0.003 NEAR) and refunds the rest.
      </Notice>

      <div className="space-y-3">
        <h3 className="text-sm font-semibold text-ink">Your NFTs</h3>
        <YourClaims claimId={claimId} info={data} metadata={metadata} />
      </div>

      <p className="flex flex-wrap items-center gap-x-2 text-xs text-ink-faint">
        Launch contract <Address accountId={claimId} />
      </p>
    </section>
  );
}
