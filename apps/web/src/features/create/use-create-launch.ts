"use client";

import { useQuery } from "@tanstack/react-query";
import { useRouter } from "next/navigation";

import { useTransaction } from "@/features/transactions/use-transaction";
import { launchKeys } from "@/features/launches/queries";
import { network } from "@/config/network";
import { findEvent } from "@/lib/near/outcome";
import { factory, factoryId } from "@/lib/orbit/contracts";
import type { LaunchKind } from "@/lib/orbit/schemas";
import { createLaunchTx } from "@/lib/orbit/transactions";

const METHOD: Record<LaunchKind, string> = {
  token: "create_token_launch",
  collection: "create_collection_launch",
  connected: "create_connected_launch",
};

/** Quotes the deposit, signs the factory call and opens the new launch's page. */
export function useCreateLaunch(kind: LaunchKind) {
  const router = useRouter();
  const tx = useTransaction();
  const quote = useQuery({
    queryKey: ["quote", kind],
    queryFn: () => factory.quote(kind),
    enabled: Boolean(network.factoryId),
  });

  async function submit(args: Record<string, unknown>, label: string) {
    if (quote.data === undefined) return;
    const summary = await tx.run(createLaunchTx(factoryId(), METHOD[kind], args, quote.data), {
      label,
      invalidate: [launchKeys.all],
      check: (s) => (findEvent(s, "orbit", "launch_created") ? null : "The factory did not record a launch."),
    });
    // Also navigate after a failed deployment: the launch page shows the failure and refund.
    const created = summary ? findEvent(summary, "orbit", "launch_created") : undefined;
    if (typeof created?.id === "number") router.push(`/launches/${created.id}`);
  }

  // Without a quote the launch cannot be signed, so say why instead of loading forever.
  const quoteProblem = !network.factoryId
    ? "No launch factory is configured for this site (NEXT_PUBLIC_ORBIT_FACTORY), so launches cannot be submitted."
    : quote.isError
      ? `Could not load the deposit from the factory: ${quote.error.message}`
      : null;

  return { quote, quoteProblem, submit, state: tx.state, reset: tx.reset };
}
