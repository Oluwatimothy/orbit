import type { Page, Route } from "@playwright/test";

/** A connected launch as the factory, token, NFT and claim contracts would report it. */
const FACTORY = "e2e-factory.testnet";
const TOKEN = "genesis-ft.e2e-factory.testnet";
const NFT = "genesis-nft.e2e-factory.testnet";
const CLAIM = "genesis-claim.e2e-factory.testnet";
const E18 = 10n ** 18n;

export const LAUNCH = {
  id: 0,
  kind: "connected",
  slug: "genesis",
  name: "Orbit Genesis",
  symbol: "ORBIT",
  description: "Genesis pass holders claim ORBIT.",
  image: null,
  creator: "creator.testnet",
  created_at_ns: String(BigInt(Date.now() - 3_600_000) * 1_000_000n),
  status: "live",
  token: TOKEN,
  nft: NFT,
  claim: CLAIM,
  sale: null,
};

const VIEWS: Record<string, Record<string, unknown>> = {
  [FACTORY]: {
    get_launch_count: 1,
    get_launches: [LAUNCH],
    get_launch: LAUNCH,
    get_launches_by_creator: [LAUNCH],
    quote_deposit: "780000000000000000000000",
    is_slug_available: true,
  },
  [TOKEN]: {
    ft_metadata: { spec: "ft-1.0.0", name: "Orbit Genesis", symbol: "ORBIT", icon: null, decimals: 18 },
    ft_total_supply: String(10_000_000n * E18),
    orbit_allocations: [
      { account_id: "creator.testnet", amount: String(8_000_000n * E18), label: "Creator" },
      { account_id: CLAIM, amount: String(2_000_000n * E18), label: "NFT holders" },
    ],
    ft_balance_of: "0",
  },
  [NFT]: {
    orbit_config: {
      name: "Orbit Genesis",
      symbol: "GEN",
      description: "Genesis pass",
      cover_media: "ipfs://bafycover",
      media: { kind: "shared", media: "ipfs://bafycover", media_hash: null },
      supply: { kind: "fixed", max: 2000 },
      price: "100000000000000000000000",
      starts_at_ns: "0",
      ends_at_ns: null,
      max_per_wallet: 5,
      public_mint: true,
    },
    orbit_mint_stats: { minted: 2, max_supply: 2000, paused: false, owner_id: "creator.testnet" },
    nft_metadata: { spec: "nft-1.0.0", name: "Orbit Genesis", symbol: "GEN", base_uri: null },
    nft_tokens: [
      {
        token_id: "1",
        owner_id: "alice.testnet",
        metadata: { title: "Orbit Genesis #1", media: "ipfs://bafycover" },
      },
      {
        token_id: "2",
        owner_id: "bob.testnet",
        metadata: { title: "Orbit Genesis #2", media: "ipfs://bafycover" },
      },
    ],
    nft_tokens_for_owner: [],
    orbit_minted_by: 0,
  },
  [CLAIM]: {
    get_info: {
      token_id: TOKEN,
      nft_id: NFT,
      creator_id: "creator.testnet",
      tiers: [{ name: "Genesis", from: 1, to: 2000, amount_per_nft: String(1_000n * E18) }],
      starts_at_ns: "0",
      status: "active",
      total_reserved: String(2_000_000n * E18),
      claimed_total: String(1_000n * E18),
      pending_total: "0",
      claims_count: 1,
    },
  },
};

function encode(value: unknown): number[] {
  return Array.from(new TextEncoder().encode(JSON.stringify(value)));
}

async function rpc(route: Route) {
  const body = route.request().postDataJSON() as {
    id: unknown;
    params?: { account_id?: string; method_name?: string };
  };
  const view = VIEWS[body.params?.account_id ?? ""]?.[body.params?.method_name ?? ""];
  if (view === undefined) {
    return route.fulfill({
      json: {
        jsonrpc: "2.0",
        id: body.id,
        error: { code: -32000, message: "unmocked view", data: "unmocked" },
      },
    });
  }
  return route.fulfill({
    json: {
      jsonrpc: "2.0",
      id: body.id,
      result: { result: encode(view), logs: [], block_height: 1, block_hash: "1" },
    },
  });
}

/** Routes NEAR RPC and the FastNEAR index to deterministic test data. */
export async function mockChain(page: Page) {
  await page.route(/rpc\.fastnear\.com|drpc\.org/, rpc);
  await page.route(/tx\.test\.fastnear\.com/, (route) =>
    route.fulfill({ json: { account_txs: [], resume_token: null, txs_count: 0, transactions: [] } }),
  );
  await page.route(/gateway\.pinata\.cloud/, (route) => route.fulfill({ status: 404 }));
}
