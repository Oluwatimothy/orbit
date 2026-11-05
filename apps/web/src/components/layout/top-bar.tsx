"use client";

import { BookOpen, Layers, Menu } from "lucide-react";
import Link from "next/link";
import { DropdownMenu } from "radix-ui";

import { OrbitMark } from "@/components/brand/orbit-mark";
import { WalletButton } from "@/features/wallet/wallet-button";
import { useWallet } from "@/features/wallet/wallet-provider";

function NetworkBadge() {
  return (
    <span className="inline-flex h-7 items-center gap-1.5 rounded-sm border border-caution/30 bg-caution-soft px-2 text-2xs font-semibold uppercase tracking-wider text-caution">
      <span className="size-1.5 rounded-full bg-caution" aria-hidden />
      Testnet
    </span>
  );
}

function MobileMenu() {
  const item =
    "flex h-9 items-center gap-2 rounded-xs px-2 text-sm text-ink-muted outline-none data-[highlighted]:bg-raised data-[highlighted]:text-ink";
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger
        className="rounded-sm p-1.5 text-ink-muted hover:text-ink lg:hidden"
        aria-label="More sections"
      >
        <Menu className="size-5" aria-hidden />
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="start"
          sideOffset={8}
          className="z-50 min-w-48 rounded-md border border-line-strong bg-surface p-1"
        >
          <DropdownMenu.Item className={item} asChild>
            <Link href="/launches">
              <Layers className="size-4" aria-hidden /> My launches
            </Link>
          </DropdownMenu.Item>
          <DropdownMenu.Item className={item} asChild>
            <Link href="/docs">
              <BookOpen className="size-4" aria-hidden /> How it works
            </Link>
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

export function TopBar() {
  const { networkMismatch, accountId } = useWallet();
  return (
    <>
      <header className="sticky top-0 z-20 flex h-14 items-center gap-3 border-b border-line bg-canvas/90 px-4 backdrop-blur lg:px-6">
        <MobileMenu />
        <Link href="/" className="text-ink lg:hidden" aria-label="Orbit home">
          <OrbitMark />
        </Link>
        <div className="ml-auto flex items-center gap-2">
          <NetworkBadge />
          <WalletButton />
        </div>
      </header>
      {networkMismatch && (
        <div
          role="alert"
          className="border-b border-negative/30 bg-negative-soft px-4 py-2 text-xs text-negative lg:px-6"
        >
          {accountId} is not a testnet account. Orbit runs on NEAR testnet only — switch to a{" "}
          <span className="tabular">.testnet</span> account to sign transactions.
        </div>
      )}
    </>
  );
}
