import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TopBar } from "@/components/layout/top-bar";
import { describe as describeActivity } from "@/features/activity/activity-table";
import { filterLaunches } from "@/features/explore/explore-page";
import { PortfolioPage } from "@/features/portfolio/portfolio-page";
import type { ActivityItem } from "@/lib/activity/fastnear";
import type { Launch } from "@/lib/orbit/schemas";
import { renderWithWallet, walletState } from "@/test/render";

vi.mock("next/navigation", () => ({ useRouter: () => ({ push: vi.fn() }), usePathname: () => "/" }));

function launch(id: number, overrides: Partial<Launch> = {}): Launch {
  return {
    id,
    kind: "token",
    slug: `launch-${id}`,
    name: `Launch ${id}`,
    symbol: `L${id}`,
    description: "",
    image: null,
    creator: "creator.testnet",
    created_at_ns: BigInt(id),
    status: "live",
    token: `launch-${id}-ft.f.testnet`,
    nft: null,
    claim: null,
    sale: null,
    ...overrides,
  };
}

describe("TopBar", () => {
  it("always shows the testnet badge and a connect button when disconnected", () => {
    renderWithWallet(<TopBar />);
    expect(screen.getByText("Testnet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Connect wallet" })).toBeInTheDocument();
  });

  it("warns when the connected account cannot exist on testnet", () => {
    renderWithWallet(
      <TopBar />,
      walletState({ status: "connected", accountId: "alice.near", networkMismatch: true }),
    );
    expect(screen.getByRole("alert")).toHaveTextContent("alice.near is not a testnet account");
  });
});

describe("filterLaunches", () => {
  const launches = [
    launch(3, { kind: "connected", name: "Orbit Genesis" }),
    launch(2, { status: "failed" }),
    launch(1, { creator: "alice.testnet" }),
  ];

  it("filters by type, status and search, and reverses for oldest-first", () => {
    expect(filterLaunches(launches, "connected", "all", "", "newest").map((l) => l.id)).toEqual([3]);
    expect(filterLaunches(launches, "all", "live", "", "newest").map((l) => l.id)).toEqual([3, 1]);
    expect(filterLaunches(launches, "all", "all", "ALICE", "newest").map((l) => l.id)).toEqual([1]);
    expect(filterLaunches(launches, "all", "all", "genesis", "newest").map((l) => l.id)).toEqual([3]);
    expect(filterLaunches(launches, "all", "all", "", "oldest").map((l) => l.id)).toEqual([1, 2, 3]);
  });
});

describe("activity descriptions", () => {
  const item = (overrides: Partial<ActivityItem>): ActivityItem => ({
    hash: "h",
    timestampMs: 0,
    signer: "alice.testnet",
    receiver: "x.testnet",
    method: null,
    succeeded: true,
    events: [],
    ...overrides,
  });

  it("uses emitted events for detail", () => {
    expect(
      describeActivity(
        item({
          method: "nft_mint",
          events: [
            { standard: "nep171", version: "1.2.0", event: "nft_mint", data: [{ token_ids: ["4", "5"] }] },
          ],
        }),
      ),
    ).toBe("Minted 2 NFTs");
    expect(
      describeActivity(
        item({
          method: "claim",
          events: [
            { standard: "orbit", version: "1.0.0", event: "claim_rejected", data: [{ nft_token_id: "9" }] },
          ],
        }),
      ),
    ).toBe("Claim rejected for NFT #9");
    expect(describeActivity(item({ method: "create_connected_launch" }))).toBe("Created connected launch");
  });
});

describe("PortfolioPage", () => {
  it("never shows balances without a connected wallet", () => {
    renderWithWallet(<PortfolioPage />);
    expect(screen.queryByText(/Orbit tokens held/)).not.toBeInTheDocument();
  });
});

describe("navigation", () => {
  it("does not mark My launches active on a launch detail page", async () => {
    const { isActive } = await import("@/components/layout/navigation");
    expect(isActive("/launches/3", "/launches")).toBe(false);
    expect(isActive("/launches", "/launches")).toBe(true);
    expect(isActive("/create/token", "/create")).toBe(true);
    expect(isActive("/explore", "/")).toBe(false);
  });
});
