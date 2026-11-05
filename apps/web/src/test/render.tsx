import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ReactElement } from "react";
import { vi } from "vitest";

import { WalletContextForTesting } from "@/features/wallet/wallet-provider";

type WalletValue = NonNullable<React.ContextType<typeof WalletContextForTesting>>;

export function walletState(overrides: Partial<WalletValue> = {}): WalletValue {
  return {
    status: "disconnected",
    accountId: null,
    networkMismatch: false,
    connect: vi.fn(async () => {}),
    disconnect: vi.fn(async () => {}),
    signAndSend: vi.fn(async () => {
      throw new Error("not used");
    }),
    ...overrides,
  };
}

/** Renders with a fixed wallet state and a fresh query cache; no network access. */
export function renderWithWallet(ui: ReactElement, wallet: WalletValue = walletState()) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, enabled: false } } });
  return render(
    <QueryClientProvider client={client}>
      <WalletContextForTesting.Provider value={wallet}>{ui}</WalletContextForTesting.Provider>
    </QueryClientProvider>,
  );
}
