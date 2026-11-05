"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";
import { Toaster } from "sonner";

import { RedirectOutcome } from "@/features/transactions/redirect-outcome";
import { WalletProvider } from "@/features/wallet/wallet-provider";

export function Providers({ children }: { children: ReactNode }) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: { staleTime: 15_000, retry: 2, refetchOnWindowFocus: false },
        },
      }),
  );
  return (
    <QueryClientProvider client={queryClient}>
      <WalletProvider>
        {children}
        <RedirectOutcome />
        <Toaster
          position="bottom-right"
          theme="dark"
          toastOptions={{
            classNames: {
              toast: "!bg-surface !border-line-strong !text-ink !rounded-md !font-sans",
              description: "!text-ink-muted",
              actionButton: "!bg-primary !text-ink-inverse",
            },
          }}
        />
      </WalletProvider>
    </QueryClientProvider>
  );
}
