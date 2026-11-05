import { describe, expect, it } from "vitest";

import { isContentAddressed, resolveMedia } from "./media";

describe("resolveMedia", () => {
  it("maps ipfs references through the configured gateway", () => {
    expect(resolveMedia("ipfs://bafy123/1.png")).toBe("https://gateway.pinata.cloud/ipfs/bafy123/1.png");
  });

  it("resolves NEP-177 relative media against base_uri", () => {
    expect(resolveMedia("7.png", "https://gw.example/ipfs/bafydir")).toBe(
      "https://gw.example/ipfs/bafydir/7.png",
    );
    expect(resolveMedia("../secret", "https://gw.example")).toBeNull();
  });

  it("never renders unsafe schemes", () => {
    expect(resolveMedia("javascript:alert(1)")).toBeNull();
    expect(resolveMedia("http://insecure.example/a.png")).toBeNull();
    expect(resolveMedia("data:text/html;base64,PGgxPg==")).toBeNull();
    expect(resolveMedia(null)).toBeNull();
  });

  it("distinguishes content-addressed references", () => {
    expect(isContentAddressed("ipfs://bafy")).toBe(true);
    expect(isContentAddressed("https://cdn.example/a.png")).toBe(false);
  });
});
