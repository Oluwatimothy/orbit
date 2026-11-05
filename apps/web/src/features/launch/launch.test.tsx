import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { CollectionConfig, MintStats, Tier } from "@/lib/orbit/schemas";
import { renderWithWallet, walletState } from "@/test/render";

import { allocationFor } from "./claim-section";
import { MintPanel, mintWindow } from "./mint-panel";

vi.mock("next/navigation", () => ({ useRouter: () => ({ push: vi.fn() }), usePathname: () => "/" }));

const NOW_NS = 1_000n;

const config: CollectionConfig = {
  name: "Genesis",
  symbol: "GEN",
  description: "",
  cover_media: "ipfs://cover",
  media: { kind: "shared", media: "ipfs://cover", media_hash: null },
  supply: { kind: "fixed", max: 10 },
  price: 10n ** 23n,
  starts_at_ns: 0n,
  ends_at_ns: null,
  max_per_wallet: 3,
  public_mint: true,
};
const stats: MintStats = { minted: 2, max_supply: 10, paused: false, owner_id: "creator.testnet" };

describe("mintWindow", () => {
  it("reports each state the contract enforces", () => {
    expect(mintWindow(config, stats, NOW_NS)).toBe("open");
    expect(mintWindow(config, { ...stats, minted: 10 }, NOW_NS)).toBe("sold-out");
    expect(mintWindow(config, { ...stats, paused: true }, NOW_NS)).toBe("paused");
    expect(mintWindow({ ...config, public_mint: false }, stats, NOW_NS)).toBe("not-public");
    expect(mintWindow({ ...config, starts_at_ns: 2_000n }, stats, NOW_NS)).toBe("upcoming");
    expect(mintWindow({ ...config, ends_at_ns: 1_000n }, stats, NOW_NS)).toBe("ended");
  });
});

describe("MintPanel", () => {
  it("asks a disconnected visitor to connect instead of minting", async () => {
    const wallet = walletState();
    renderWithWallet(
      <MintPanel nftId="g-nft.f.testnet" config={config} stats={stats} mintedByYou={undefined} />,
      wallet,
    );
    await userEvent.click(screen.getByRole("button", { name: "Connect wallet to mint" }));
    expect(wallet.connect).toHaveBeenCalled();
  });

  it("caps quantity at the wallet limit and shows the attached total", async () => {
    renderWithWallet(
      <MintPanel nftId="g-nft.f.testnet" config={config} stats={stats} mintedByYou={1} />,
      walletState({ status: "connected", accountId: "alice.testnet" }),
    );
    const increase = screen.getByRole("button", { name: "Increase quantity" });
    await userEvent.click(increase);
    await userEvent.click(increase);
    expect(screen.getByRole("button", { name: "Mint 2" })).toBeEnabled();
    expect(increase).toBeDisabled();
    // 2 × (0.1 NEAR price + 0.02 NEAR storage allowance)
    expect(screen.getByText("0.24 NEAR")).toBeInTheDocument();
  });

  it("explains when the wallet limit is reached", () => {
    renderWithWallet(
      <MintPanel nftId="g-nft.f.testnet" config={config} stats={stats} mintedByYou={3} />,
      walletState({ status: "connected", accountId: "alice.testnet" }),
    );
    expect(screen.getByText(/reached this collection's limit of 3/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Mint/ })).not.toBeInTheDocument();
  });

  it("blocks signing from a non-testnet account", () => {
    renderWithWallet(
      <MintPanel nftId="g-nft.f.testnet" config={config} stats={stats} mintedByYou={0} />,
      walletState({ status: "connected", accountId: "alice.near", networkMismatch: true }),
    );
    expect(screen.getByRole("button", { name: "Mint 1" })).toBeDisabled();
  });

  it("shows why minting is closed", () => {
    renderWithWallet(
      <MintPanel nftId="g-nft.f.testnet" config={config} stats={{ ...stats, minted: 10 }} mintedByYou={0} />,
      walletState({ status: "connected", accountId: "alice.testnet" }),
    );
    expect(screen.getByText("Every NFT in this collection has been minted.")).toBeInTheDocument();
  });
});

describe("allocationFor", () => {
  const tiers: Tier[] = [
    { name: "Genesis", from: 1, to: 1000, amount_per_nft: 1000n },
    { name: "Founders", from: 1001, to: 1100, amount_per_nft: 5000n },
  ];

  it("matches the contract's tier lookup, including canonical IDs", () => {
    expect(allocationFor(tiers, "1000")?.amount).toBe(1000n);
    expect(allocationFor(tiers, "1001")?.tier.name).toBe("Founders");
    expect(allocationFor(tiers, "1101")).toBeNull();
    expect(allocationFor(tiers, "007")).toBeNull();
    expect(allocationFor(tiers, "0")).toBeNull();
  });
});
