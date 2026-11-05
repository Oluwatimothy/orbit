"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import { OrbitWordmark } from "@/components/brand/orbit-mark";
import { cn } from "@/lib/cn";

import { NAV_ITEMS, SECONDARY_NAV, isActive, type NavItem } from "./navigation";

function NavLink({ item, pathname }: { item: NavItem; pathname: string }) {
  const active = isActive(pathname, item.href);
  const Icon = item.icon;
  return (
    <Link
      href={item.href}
      aria-current={active ? "page" : undefined}
      className={cn(
        "group flex h-8 items-center gap-2.5 rounded-sm px-2.5 text-sm transition-colors",
        active ? "bg-raised text-ink" : "text-ink-muted hover:bg-raised/60 hover:text-ink",
      )}
    >
      <Icon
        className={cn("size-4", active ? "text-ink" : "text-ink-faint group-hover:text-ink-muted")}
        aria-hidden
      />
      {item.label}
    </Link>
  );
}

export function Sidebar() {
  const pathname = usePathname();
  return (
    <aside className="sticky top-0 hidden h-dvh w-[220px] shrink-0 flex-col border-r border-line bg-canvas lg:flex">
      <div className="flex h-14 items-center px-4">
        <Link href="/" aria-label="Orbit home">
          <OrbitWordmark />
        </Link>
      </div>
      <nav aria-label="Main" className="flex flex-1 flex-col gap-0.5 px-2 pt-2">
        {NAV_ITEMS.map((item) => (
          <NavLink key={item.href} item={item} pathname={pathname} />
        ))}
      </nav>
      <div className="space-y-0.5 border-t border-line px-2 py-3">
        {SECONDARY_NAV.map((item) => (
          <NavLink key={item.href} item={item} pathname={pathname} />
        ))}
        <p className="px-2.5 pt-2 text-2xs leading-4 text-ink-faint">
          The on-chain launch studio. Testnet only — assets have no monetary value.
        </p>
      </div>
    </aside>
  );
}
