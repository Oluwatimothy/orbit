import { Activity, BookOpen, Compass, LayoutGrid, Layers, Plus, Wallet, type LucideIcon } from "lucide-react";

export interface NavItem {
  href: string;
  label: string;
  icon: LucideIcon;
  /** Shown in the mobile bottom bar. */
  primary?: boolean;
}

export const NAV_ITEMS: NavItem[] = [
  { href: "/", label: "Overview", icon: LayoutGrid, primary: true },
  { href: "/explore", label: "Explore", icon: Compass, primary: true },
  { href: "/create", label: "Create", icon: Plus, primary: true },
  { href: "/launches", label: "My launches", icon: Layers },
  { href: "/portfolio", label: "My assets", icon: Wallet, primary: true },
  { href: "/activity", label: "Activity", icon: Activity, primary: true },
];

export const SECONDARY_NAV: NavItem[] = [{ href: "/docs", label: "How it works", icon: BookOpen }];

/** Sections whose sub-paths belong to other pages (`/launches/:id` is a launch, not "My launches"). */
const EXACT_ONLY = new Set(["/", "/launches"]);

export function isActive(pathname: string, href: string): boolean {
  return EXACT_ONLY.has(href) ? pathname === href : pathname === href || pathname.startsWith(`${href}/`);
}
