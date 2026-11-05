import { describe, expect, it } from "vitest";

import { parseNetworkConfig } from "./network";

describe("parseNetworkConfig", () => {
  it("defaults to testnet with FastNEAR and dRPC", () => {
    const config = parseNetworkConfig({});
    expect(config.networkId).toBe("testnet");
    expect(config.rpcUrls[0]).toBe("https://test.rpc.fastnear.com");
    expect(config.factoryId).toBeUndefined();
  });

  it("refuses any network other than testnet", () => {
    expect(() => parseNetworkConfig({ NEXT_PUBLIC_NEAR_NETWORK: "mainnet" })).toThrow(/must be 'testnet'/);
  });

  it("validates the factory account and RPC URLs", () => {
    expect(() => parseNetworkConfig({ NEXT_PUBLIC_ORBIT_FACTORY: "Bad Account" })).toThrow();
    expect(() => parseNetworkConfig({ NEXT_PUBLIC_NEAR_RPC_URLS: "http://insecure.example" })).toThrow();
    expect(parseNetworkConfig({ NEXT_PUBLIC_ORBIT_FACTORY: "orbit-factory.testnet" }).factoryId).toBe(
      "orbit-factory.testnet",
    );
  });
});
