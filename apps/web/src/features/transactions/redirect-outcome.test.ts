import { describe, expect, it } from "vitest";

import { parseRedirect } from "./redirect-outcome";

describe("parseRedirect", () => {
  it("reads comma-separated hashes and ignores malformed ones", () => {
    const hash = "FPPeWoKzBna9YiGbenCVS7Fw3hFNLSRnEhbpbT9iGzYV";
    expect(parseRedirect(`?transactionHashes=${hash},not-a-hash`)).toEqual({ hashes: [hash], error: null });
  });

  it("surfaces wallet errors", () => {
    expect(parseRedirect("?errorCode=userRejected&errorMessage=User%20rejected").error).toBe("User rejected");
    expect(parseRedirect("").hashes).toEqual([]);
  });
});
