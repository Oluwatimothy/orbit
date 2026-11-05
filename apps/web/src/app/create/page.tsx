import { ArrowRight, Coins, Image as ImageIcon, Waypoints } from "lucide-react";
import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = { title: "Create a launch" };

const OPTIONS = [
  {
    href: "/create/token",
    icon: Coins,
    title: "Token",
    standard: "NEP-141",
    description: "A fungible token with a fixed supply distributed to the accounts you choose.",
    deploys: ["One launch contract: NEP-141 token"],
    enforced: ["Exact supply, minted once", "Allocations sum to supply", "No further minting"],
  },
  {
    href: "/create/collection",
    icon: ImageIcon,
    title: "NFT collection",
    standard: "NEP-171",
    description: "A collection with a paid or free public mint, without a token.",
    deploys: ["One launch contract: NEP-171 collection"],
    enforced: ["Supply cap", "Price and payment", "Wallet limit and mint window"],
  },
  {
    href: "/create/connected",
    icon: Waypoints,
    title: "Connected token + NFT",
    standard: "NEP-141 + NEP-171",
    description:
      "A token and a collection launched together. NFT holders claim token allocations by tier, buy in a gated sale, or both.",
    deploys: ["One launch contract: token, collection, claims and sale"],
    enforced: ["Reserve funded at launch", "One claim per NFT", "Ownership checked at claim time"],
  },
];

export default function Page() {
  return (
    <div className="mx-auto max-w-6xl space-y-6 px-4 py-6 lg:px-8 lg:py-8">
      <div className="space-y-1">
        <h1 className="text-xl font-semibold tracking-tight text-ink">Create a launch</h1>
        <p className="max-w-2xl text-sm text-ink-muted">
          Each option deploys one standard NEAR contract through the Orbit factory. You review every value
          before signing, and the rules cannot be changed afterwards.
        </p>
      </div>
      <div className="grid divide-y divide-line rounded-md border border-line lg:grid-cols-3 lg:divide-x lg:divide-y-0">
        {OPTIONS.map(({ href, icon: Icon, title, standard, description, deploys, enforced }) => (
          <Link
            key={href}
            href={href}
            className="group flex flex-col gap-4 p-5 transition-colors hover:bg-surface"
          >
            <div className="flex items-start justify-between">
              <Icon className="size-5 text-ink-muted" aria-hidden />
              <span className="tabular text-2xs text-ink-faint">{standard}</span>
            </div>
            <div className="space-y-1.5">
              <h2 className="text-base font-semibold text-ink">{title}</h2>
              <p className="text-sm text-ink-muted">{description}</p>
            </div>
            <dl className="space-y-3 border-t border-line pt-4 text-xs">
              <div>
                <dt className="mb-1 text-2xs uppercase tracking-wider text-ink-faint">Deploys</dt>
                <dd className="text-ink-muted">{deploys.join(" · ")}</dd>
              </div>
              <div>
                <dt className="mb-1 text-2xs uppercase tracking-wider text-ink-faint">Enforced on-chain</dt>
                <dd>
                  <ul className="space-y-0.5 text-ink-muted">
                    {enforced.map((rule) => (
                      <li key={rule}>{rule}</li>
                    ))}
                  </ul>
                </dd>
              </div>
            </dl>
            <span className="mt-auto inline-flex items-center gap-1.5 text-sm font-medium text-ink">
              Start{" "}
              <ArrowRight className="size-4 transition-transform group-hover:translate-x-0.5" aria-hidden />
            </span>
          </Link>
        ))}
      </div>
    </div>
  );
}
