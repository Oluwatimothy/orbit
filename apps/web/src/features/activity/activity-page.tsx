"use client";

import { SectionHeader } from "@/components/ui/data";
import { FactoryMissingNotice } from "@/features/launches/registry-state";
import { ConnectPrompt } from "@/features/portfolio/portfolio-page";
import { useWallet } from "@/features/wallet/wallet-provider";
import { network } from "@/config/network";

import { ActivityTable } from "./activity-table";

export function ActivityPage() {
  const { accountId } = useWallet();
  return (
    <div className="mx-auto max-w-6xl space-y-8 px-4 py-6 lg:px-8 lg:py-8">
      <div className="space-y-1">
        <h1 className="text-xl font-semibold tracking-tight text-ink">Activity</h1>
        <p className="max-w-2xl text-sm text-ink-muted">
          Executed testnet transactions, indexed by FastNEAR and linked to NearBlocks. Transactions you are
          signing right now appear in the notifications corner until they are confirmed.
        </p>
      </div>
      {!network.factoryId ? (
        <FactoryMissingNotice />
      ) : (
        <section className="space-y-3">
          <SectionHeader title="Platform" description="Launches created through the Orbit factory." />
          <div className="rounded-md border border-line">
            <ActivityTable
              accounts={[network.factoryId]}
              queryKey={["factory"]}
              emptyText="No launches have been created yet."
            />
          </div>
        </section>
      )}
      <section className="space-y-3">
        <SectionHeader title="Your account" description="Transactions signed by or sent to your account." />
        <div className="rounded-md border border-line">
          {accountId ? (
            <ActivityTable
              accounts={[accountId]}
              queryKey={["account", accountId]}
              emptyText="Your transactions appear here once executed."
            />
          ) : (
            <ConnectPrompt
              title="Connect a wallet to see your activity"
              description="Orbit reads your transaction history from the public index."
            />
          )}
        </div>
      </section>
    </div>
  );
}
