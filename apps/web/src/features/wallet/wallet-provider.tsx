"use client";

import type { EventMap, NearConnector, NearWalletBase } from "@hot-labs/near-connect";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { network } from "@/config/network";
import { isTestnetAccount } from "@/lib/near/accounts";
import type { OutcomeLike } from "@/lib/near/outcome";

export interface FunctionCall {
  methodName: string;
  args: Record<string, unknown>;
  /** Gas in gas units, as a decimal string. */
  gas: string;
  /** Deposit in yoctoNEAR, as a decimal string. */
  deposit: string;
}

export interface TransactionRequest {
  receiverId: string;
  actions: FunctionCall[];
}

type WalletStatus = "initializing" | "disconnected" | "connected";

interface WalletContextValue {
  status: WalletStatus;
  accountId: string | null;
  /** The connected account cannot exist on testnet (e.g. a `.near` account). */
  networkMismatch: boolean;
  connect: () => Promise<void>;
  disconnect: () => Promise<void>;
  /** Signs and sends; resolves with the wallet's (possibly partial) outcome. */
  signAndSend: (request: TransactionRequest) => Promise<OutcomeLike>;
}

const WalletContext = createContext<WalletContextValue | null>(null);

export class WalletRejectedError extends Error {
  constructor() {
    super("The request was rejected in the wallet.");
  }
}

const REJECTION_PATTERN = /reject|cancel|denied|closed|user abort/i;

export function WalletProvider({ children }: { children: ReactNode }) {
  // Created in an effect: NEAR Connect needs browser APIs, and pages should still render on the server.
  const [connector, setConnector] = useState<NearConnector | null>(null);
  const [wallet, setWallet] = useState<NearWalletBase | null>(null);
  const [accountId, setAccountId] = useState<string | null>(null);
  const [status, setStatus] = useState<WalletStatus>("initializing");

  useEffect(() => {
    let cancelled = false;
    import("@hot-labs/near-connect")
      .then(({ NearConnector: Connector }) => {
        if (!cancelled) {
          setConnector(
            new Connector({ network: network.networkId, providers: { testnet: network.rpcUrls } }),
          );
        }
      })
      .catch((error: unknown) => {
        console.error("Wallet connector failed to load", error);
        if (!cancelled) setStatus("disconnected");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!connector) return;
    let cancelled = false;
    const onSignIn = (payload: EventMap["wallet:signIn"]) => {
      if (cancelled) return;
      setWallet(payload.wallet);
      setAccountId(payload.accounts[0]?.accountId ?? null);
      setStatus("connected");
    };
    const onSignOut = () => {
      if (cancelled) return;
      setWallet(null);
      setAccountId(null);
      setStatus("disconnected");
    };
    connector.on("wallet:signIn", onSignIn);
    connector.on("wallet:signOut", onSignOut);

    connector
      .getConnectedWallet()
      .then(({ wallet: restored, accounts }) => {
        if (cancelled) return;
        setWallet(restored);
        setAccountId(accounts[0]?.accountId ?? null);
        setStatus(accounts[0] ? "connected" : "disconnected");
      })
      // No previously connected wallet is the normal first-visit state.
      .catch(() => !cancelled && setStatus("disconnected"));

    return () => {
      cancelled = true;
      connector.off("wallet:signIn", onSignIn);
      connector.off("wallet:signOut", onSignOut);
    };
  }, [connector]);

  const connect = useCallback(async () => {
    if (!connector) throw new Error("The wallet connector has not loaded. Check your connection and reload.");
    try {
      const connected = await connector.connect();
      const accounts = await connected.getAccounts();
      setWallet(connected);
      setAccountId(accounts[0]?.accountId ?? null);
      setStatus(accounts[0] ? "connected" : "disconnected");
    } catch (error) {
      if (REJECTION_PATTERN.test(String(error))) return;
      throw error;
    }
  }, [connector]);

  const disconnect = useCallback(async () => {
    if (!connector) return;
    await connector.disconnect(wallet ?? undefined);
    setWallet(null);
    setAccountId(null);
    setStatus("disconnected");
  }, [connector, wallet]);

  const networkMismatch = accountId !== null && !isTestnetAccount(accountId);

  const signAndSend = useCallback(
    async (request: TransactionRequest): Promise<OutcomeLike> => {
      if (!wallet || !accountId) throw new Error("Connect a wallet first.");
      if (networkMismatch) throw new Error(`${accountId} is not a testnet account.`);
      try {
        const outcome = await wallet.signAndSendTransaction({
          network: network.networkId,
          signerId: accountId,
          receiverId: request.receiverId,
          actions: request.actions.map((params) => ({ type: "FunctionCall" as const, params })),
        });
        return outcome as unknown as OutcomeLike;
      } catch (error) {
        if (REJECTION_PATTERN.test(String(error))) throw new WalletRejectedError();
        throw error;
      }
    },
    [wallet, accountId, networkMismatch],
  );

  const value = useMemo(
    () => ({ status, accountId, networkMismatch, connect, disconnect, signAndSend }),
    [status, accountId, networkMismatch, connect, disconnect, signAndSend],
  );
  return <WalletContext.Provider value={value}>{children}</WalletContext.Provider>;
}

export function useWallet(): WalletContextValue {
  const context = useContext(WalletContext);
  if (!context) throw new Error("useWallet must be used inside WalletProvider");
  return context;
}

/** For tests and stories: provide a fixed wallet state without NEAR Connect. */
export const WalletContextForTesting = WalletContext;
