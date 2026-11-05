import { ArrowLeft } from "lucide-react";
import Link from "next/link";
import type { ReactNode } from "react";

import { Notice } from "@/components/ui/status";
import { network } from "@/config/network";

export function CreateLayout({
  title,
  description,
  aside,
  children,
}: {
  title: string;
  description: string;
  aside: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="mx-auto max-w-6xl space-y-6 px-4 py-6 lg:px-8 lg:py-8">
      <div className="space-y-3">
        <Link
          href="/create"
          className="inline-flex items-center gap-1.5 text-xs text-ink-muted hover:text-ink"
        >
          <ArrowLeft className="size-3.5" aria-hidden /> All launch types
        </Link>
        <div className="space-y-1">
          <h1 className="text-xl font-semibold tracking-tight text-ink">{title}</h1>
          <p className="max-w-2xl text-sm text-ink-muted">{description}</p>
        </div>
      </div>
      {!network.factoryId && (
        <Notice tone="caution">
          No launch factory is configured for this deployment, so launches cannot be submitted. You can still
          fill in and validate the form.
        </Notice>
      )}
      <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_300px]">
        <div className="min-w-0">{children}</div>
        <aside className="lg:sticky lg:top-20 lg:self-start">{aside}</aside>
      </div>
    </div>
  );
}

/** A numbered form section separated by hairlines rather than boxed in a card. */
export function FormSection({
  step,
  title,
  description,
  children,
}: {
  step: number;
  title: string;
  description?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="grid gap-4 border-t border-line py-6 first:border-t-0 first:pt-0 md:grid-cols-[180px_minmax(0,1fr)] md:gap-8">
      <div className="space-y-1">
        <p className="tabular text-2xs text-ink-faint">{String(step).padStart(2, "0")}</p>
        <h2 className="text-sm font-semibold text-ink">{title}</h2>
        {description && <p className="text-xs text-ink-muted">{description}</p>}
      </div>
      <div className="min-w-0 space-y-4">{children}</div>
    </section>
  );
}

export function SummaryPanel({
  title,
  children,
  footer,
}: {
  title: string;
  children: ReactNode;
  footer: ReactNode;
}) {
  return (
    <div className="rounded-md border border-line bg-surface">
      <div className="border-b border-line px-4 py-3">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-ink-muted">{title}</h2>
      </div>
      <div className="px-4 py-1">{children}</div>
      <div className="space-y-3 border-t border-line px-4 py-4">{footer}</div>
    </div>
  );
}
