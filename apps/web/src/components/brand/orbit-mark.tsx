import { cn } from "@/lib/cn";

/**
 * A ring broken by a single body in orbit. One colour (currentColor) by design.
 */
export function OrbitMark({ className, title }: { className?: string; title?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      className={cn("size-5", className)}
      fill="none"
      role={title ? "img" : undefined}
      aria-hidden={title ? undefined : true}
      aria-label={title}
    >
      <path d="M17.66 6.34A8 8 0 1 0 20 12" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" />
      <circle cx="19.07" cy="8.46" r="2.1" fill="currentColor" />
      <circle cx="12" cy="12" r="2.25" fill="currentColor" />
    </svg>
  );
}

export function OrbitWordmark({ className }: { className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-2 text-ink", className)}>
      <OrbitMark />
      <span className="text-[15px] font-semibold tracking-[0.14em]">ORBIT</span>
    </span>
  );
}
