import { z } from "zod";

/**
 * Public, build-time configuration. Orbit is testnet-only (DECISIONS D-001):
 * any other network is a configuration error, not a runtime option.
 */
const accountId = z
  .string()
  .regex(/^(([a-z\d]+[-_])*[a-z\d]+\.)*([a-z\d]+[-_])*[a-z\d]+$/, "invalid NEAR account ID")
  .min(2)
  .max(64);

const schema = z.object({
  networkId: z.literal("testnet", { error: "NEXT_PUBLIC_NEAR_NETWORK must be 'testnet'" }),
  factoryId: accountId.optional(),
  rpcUrls: z.array(z.url({ protocol: /^https$/ })).min(1),
  ipfsGateway: z.url({ protocol: /^https$/ }),
});

export type NetworkConfig = z.infer<typeof schema>;

const DEFAULT_RPC_URLS = ["https://test.rpc.fastnear.com", "https://near-testnet.drpc.org"];
const DEFAULT_IPFS_GATEWAY = "https://gateway.pinata.cloud/ipfs";

function splitList(value: string | undefined): string[] | undefined {
  const items = value
    ?.split(",")
    .map((item) => item.trim())
    .filter(Boolean);
  return items?.length ? items : undefined;
}

export function parseNetworkConfig(env: Record<string, string | undefined>): NetworkConfig {
  const result = schema.safeParse({
    networkId: env.NEXT_PUBLIC_NEAR_NETWORK ?? "testnet",
    factoryId: env.NEXT_PUBLIC_ORBIT_FACTORY || undefined,
    rpcUrls: splitList(env.NEXT_PUBLIC_NEAR_RPC_URLS) ?? DEFAULT_RPC_URLS,
    ipfsGateway: (env.NEXT_PUBLIC_IPFS_GATEWAY || DEFAULT_IPFS_GATEWAY).replace(/\/$/, ""),
  });
  if (!result.success) {
    throw new Error(`Invalid Orbit configuration: ${z.prettifyError(result.error)}`);
  }
  return result.data;
}

// `process.env.NEXT_PUBLIC_*` must be referenced literally for Next.js to inline it.
export const network = parseNetworkConfig({
  NEXT_PUBLIC_NEAR_NETWORK: process.env.NEXT_PUBLIC_NEAR_NETWORK,
  NEXT_PUBLIC_ORBIT_FACTORY: process.env.NEXT_PUBLIC_ORBIT_FACTORY,
  NEXT_PUBLIC_NEAR_RPC_URLS: process.env.NEXT_PUBLIC_NEAR_RPC_URLS,
  NEXT_PUBLIC_IPFS_GATEWAY: process.env.NEXT_PUBLIC_IPFS_GATEWAY,
});

export const accountIdSchema = accountId;
