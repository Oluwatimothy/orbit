"use client";

import { ChevronDown, Copy, ExternalLink, LogOut } from "lucide-react";
import { DropdownMenu } from "radix-ui";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/status";
import { accountUrl, shortAccount } from "@/lib/near/explorer";

import { useWallet } from "./wallet-provider";

const menuItem =
  "flex h-8 cursor-default select-none items-center gap-2 rounded-xs px-2 text-sm text-ink-muted outline-none data-[highlighted]:bg-raised data-[highlighted]:text-ink";

export function WalletButton() {
  const { status, accountId, connect, disconnect } = useWallet();

  if (status === "initializing") return <Skeleton className="h-8 w-28" />;

  if (!accountId) {
    return (
      <Button
        variant="primary"
        onClick={() =>
          connect().catch((error: unknown) =>
            toast.error("Could not connect", { description: String(error) }),
          )
        }
      >
        Connect wallet
      </Button>
    );
  }

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <Button variant="secondary" className="max-w-[200px]">
          <span className="size-1.5 shrink-0 rounded-full bg-positive" aria-hidden />
          <span className="tabular truncate text-xs">{shortAccount(accountId, 20)}</span>
          <ChevronDown className="size-3.5 text-ink-faint" aria-hidden />
        </Button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={6}
          className="z-50 min-w-52 animate-fade-in rounded-md border border-line-strong bg-surface p-1 shadow-xl shadow-black/40"
        >
          <DropdownMenu.Label className="px-2 py-1.5 text-2xs uppercase tracking-wider text-ink-faint">
            Connected · testnet
          </DropdownMenu.Label>
          <DropdownMenu.Item
            className={menuItem}
            onSelect={() =>
              navigator.clipboard.writeText(accountId).then(() => toast.success("Account ID copied"))
            }
          >
            <Copy className="size-4" aria-hidden /> Copy account ID
          </DropdownMenu.Item>
          <DropdownMenu.Item className={menuItem} asChild>
            <a href={accountUrl(accountId)} target="_blank" rel="noreferrer">
              <ExternalLink className="size-4" aria-hidden /> View on NearBlocks
            </a>
          </DropdownMenu.Item>
          <DropdownMenu.Separator className="my-1 h-px bg-line" />
          <DropdownMenu.Item className={menuItem} onSelect={() => void disconnect()}>
            <LogOut className="size-4" aria-hidden /> Disconnect
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
