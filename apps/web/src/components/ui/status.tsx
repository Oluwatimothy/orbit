import { AlertTriangle, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib/cn";

const tones = {
  neutral: "bg-raised text-ink-muted border-line-strong",
  positive: "bg-positive-soft text-positive border-positive/25",
  negative: "bg-negative-soft text-negative border-negative/25",
  caution: "bg-caution-soft text-caution border-caution/25",
  inverse: "bg-primary text-ink-inverse border-primary",
} as const;

export type Tone = keyof typeof tones;

export function Badge({
  tone = "neutral",
  children,
  className,
}: {
  tone?: Tone;
  children: ReactNode;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex h-5 items-center gap-1 rounded-xs border px-1.5 text-2xs font-medium uppercase tracking-wide",
        tones[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}

export function Skeleton({ className }: { className?: string }) {
  return <span aria-hidden className={cn("block animate-pulse-soft rounded-xs bg-raised", className)} />;
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: {
  icon?: LucideIcon;
  title: string;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex flex-col items-center gap-3 px-6 py-14 text-center", className)}>
      {Icon && <Icon className="size-5 text-ink-faint" aria-hidden />}
      <div className="space-y-1">
        <p className="text-sm font-medium text-ink">{title}</p>
        {description && <p className="mx-auto max-w-sm text-xs text-ink-muted">{description}</p>}
      </div>
      {action}
    </div>
  );
}

export function ErrorState({
  title,
  error,
  onRetry,
}: {
  title: string;
  error: unknown;
  onRetry?: () => void;
}) {
  const message = error instanceof Error ? error.message : String(error);
  return (
    <div
      role="alert"
      className="flex items-start gap-3 rounded-md border border-negative/25 bg-negative-soft px-4 py-3"
    >
      <AlertTriangle className="mt-0.5 size-4 shrink-0 text-negative" aria-hidden />
      <div className="min-w-0 flex-1 space-y-1">
        <p className="text-sm font-medium text-ink">{title}</p>
        <p className="break-words text-xs text-ink-muted">{message}</p>
      </div>
      {onRetry && (
        <button onClick={onRetry} className="text-xs font-medium text-ink underline-offset-2 hover:underline">
          Retry
        </button>
      )}
    </div>
  );
}

export function Notice({
  tone = "neutral",
  children,
}: {
  tone?: Exclude<Tone, "inverse">;
  children: ReactNode;
}) {
  return <div className={cn("rounded-md border px-3 py-2.5 text-xs leading-5", tones[tone])}>{children}</div>;
}
