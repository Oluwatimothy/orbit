"use client";

import { Coins, Image as ImageIcon, Waypoints } from "lucide-react";
import Link from "next/link";
import { useState } from "react";

import { Badge, type Tone } from "@/components/ui/status";
import { cn } from "@/lib/cn";
import { useNow } from "@/lib/hooks/use-now";
import { shortAccount } from "@/lib/near/explorer";
import { resolveMedia } from "@/lib/orbit/media";
import type { Launch, LaunchKind } from "@/lib/orbit/schemas";

export const KIND_LABEL: Record<LaunchKind, string> = {
  token: "Token",
  collection: "Collection",
  connected: "Connected",
};

const KIND_ICON = { token: Coins, collection: ImageIcon, connected: Waypoints } as const;

export function KindBadge({ kind }: { kind: LaunchKind }) {
  const Icon = KIND_ICON[kind];
  return (
    <Badge tone={kind === "connected" ? "inverse" : "neutral"}>
      <Icon className="size-3" aria-hidden />
      {KIND_LABEL[kind]}
    </Badge>
  );
}

const STATUS_TONE: Record<Launch["status"], Tone> = {
  live: "positive",
  deploying: "caution",
  failed: "negative",
};

export function StatusBadge({ status }: { status: Launch["status"] }) {
  return <Badge tone={STATUS_TONE[status]}>{status}</Badge>;
}

/** Square artwork with a typographic fallback when media is missing or fails to load. */
export function LaunchImage({
  src,
  alt,
  fallback,
  className,
  baseUri,
}: {
  src: string | null | undefined;
  alt: string;
  fallback: string;
  className?: string;
  baseUri?: string | null;
}) {
  const url = resolveMedia(src, baseUri);
  const [failed, setFailed] = useState(false);
  return (
    <div
      className={cn("relative shrink-0 overflow-hidden rounded-sm border border-line bg-raised", className)}
    >
      {url && !failed ? (
        // External, user-supplied media: next/image would need every gateway allow-listed.
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={url}
          alt={alt}
          className="size-full object-cover"
          loading="lazy"
          onError={() => setFailed(true)}
        />
      ) : (
        <span className="flex size-full items-center justify-center text-2xs font-semibold tracking-wider text-ink-faint">
          {fallback.slice(0, 4).toUpperCase()}
        </span>
      )}
    </div>
  );
}

export function timeAgo(nanoseconds: bigint, now: number): string {
  const seconds = Math.max(0, Math.floor((now - Number(nanoseconds / 1_000_000n)) / 1000));
  if (seconds < 60) return "just now";
  const units: Array<[number, string]> = [
    [86_400, "d"],
    [3_600, "h"],
    [60, "m"],
  ];
  for (const [size, suffix] of units) {
    if (seconds >= size) return `${Math.floor(seconds / size)}${suffix} ago`;
  }
  return "just now";
}

/** A dense, linked row used by the overview, explore and portfolio tables. */
export function LaunchRow({ launch }: { launch: Launch }) {
  const now = useNow();
  return (
    <tr className="group border-b border-line last:border-0 hover:bg-surface">
      <td className="py-2.5 pl-4 pr-3">
        <Link href={`/launches/${launch.id}`} className="flex items-center gap-3 outline-none">
          <LaunchImage src={launch.image} alt="" fallback={launch.symbol} className="size-8" />
          <span className="min-w-0">
            <span className="block truncate text-sm font-medium text-ink group-hover:underline">
              {launch.name}
            </span>
            <span className="tabular block text-2xs text-ink-faint">{launch.symbol}</span>
          </span>
        </Link>
      </td>
      <td className="px-3 py-2.5">
        <KindBadge kind={launch.kind} />
      </td>
      <td className="hidden px-3 py-2.5 md:table-cell">
        <span className="tabular text-xs text-ink-muted">{shortAccount(launch.creator)}</span>
      </td>
      <td className="hidden px-3 py-2.5 sm:table-cell">
        <StatusBadge status={launch.status} />
      </td>
      <td className="py-2.5 pl-3 pr-4 text-right">
        <span className="tabular text-xs text-ink-muted">{timeAgo(launch.created_at_ns, now)}</span>
      </td>
    </tr>
  );
}

export function LaunchTable({ launches, caption }: { launches: Launch[]; caption: string }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[480px] text-left">
        <caption className="sr-only">{caption}</caption>
        <thead>
          <tr className="border-b border-line text-2xs uppercase tracking-wider text-ink-faint">
            <th scope="col" className="py-2 pl-4 pr-3 font-medium">
              Launch
            </th>
            <th scope="col" className="px-3 py-2 font-medium">
              Type
            </th>
            <th scope="col" className="hidden px-3 py-2 font-medium md:table-cell">
              Creator
            </th>
            <th scope="col" className="hidden px-3 py-2 font-medium sm:table-cell">
              Status
            </th>
            <th scope="col" className="py-2 pl-3 pr-4 text-right font-medium">
              Created
            </th>
          </tr>
        </thead>
        <tbody>
          {launches.map((launch) => (
            <LaunchRow key={launch.id} launch={launch} />
          ))}
        </tbody>
      </table>
    </div>
  );
}
