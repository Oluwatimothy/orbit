"use client";

import { Check, Copy, ExternalLink } from "lucide-react";
import { useState, type ReactNode } from "react";

import { cn } from "@/lib/cn";
import { accountUrl, shortAccount } from "@/lib/near/explorer";

/** A labelled value in a definition grid. */
export function Stat({
  label,
  value,
  sub,
  className,
}: {
  label: string;
  value: ReactNode;
  sub?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("min-w-0 space-y-1", className)}>
      <dt className="text-2xs font-medium uppercase tracking-wider text-ink-faint">{label}</dt>
      <dd className="tabular truncate text-base text-ink">{value}</dd>
      {sub && <dd className="text-xs text-ink-muted">{sub}</dd>}
    </div>
  );
}

export function KeyValue({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-4 border-b border-line py-2.5 last:border-0">
      <dt className="shrink-0 text-xs text-ink-muted">{label}</dt>
      <dd className="min-w-0 text-right text-sm text-ink">{children}</dd>
    </div>
  );
}

export function CopyButton({ value, label = "Copy" }: { value: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  const Icon = copied ? Check : Copy;
  return (
    <button
      type="button"
      onClick={async () => {
        await navigator.clipboard.writeText(value);
        setCopied(true);
        setTimeout(() => setCopied(false), 1_200);
      }}
      className="rounded-xs p-1 text-ink-faint transition-colors hover:text-ink"
      aria-label={copied ? "Copied" : label}
    >
      <Icon className="size-3.5" aria-hidden />
    </button>
  );
}

/** Account ID with copy and explorer actions. */
export function Address({ accountId, short = true }: { accountId: string; short?: boolean }) {
  return (
    <span className="inline-flex min-w-0 items-center gap-0.5">
      <span className="tabular truncate text-xs" title={accountId}>
        {short ? shortAccount(accountId) : accountId}
      </span>
      <CopyButton value={accountId} label={`Copy ${accountId}`} />
      <a
        href={accountUrl(accountId)}
        target="_blank"
        rel="noreferrer"
        className="rounded-xs p-1 text-ink-faint transition-colors hover:text-ink"
        aria-label={`View ${accountId} on NearBlocks`}
      >
        <ExternalLink className="size-3.5" aria-hidden />
      </a>
    </span>
  );
}

export function SectionHeader({
  title,
  description,
  action,
  id,
}: {
  title: string;
  description?: ReactNode;
  action?: ReactNode;
  /** Lets the enclosing section reference this heading with aria-labelledby. */
  id?: string;
}) {
  return (
    <div className="flex flex-wrap items-end justify-between gap-3">
      <div className="space-y-0.5">
        <h2 id={id} className="text-sm font-semibold text-ink">
          {title}
        </h2>
        {description && <p className="text-xs text-ink-muted">{description}</p>}
      </div>
      {action}
    </div>
  );
}
