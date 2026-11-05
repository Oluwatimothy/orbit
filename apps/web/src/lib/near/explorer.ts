const EXPLORER = "https://testnet.nearblocks.io";

export function txUrl(hash: string): string {
  return `${EXPLORER}/txns/${encodeURIComponent(hash)}`;
}

export function accountUrl(accountId: string): string {
  return `${EXPLORER}/address/${encodeURIComponent(accountId)}`;
}

export function shortAccount(accountId: string, max = 24): string {
  if (accountId.length <= max) return accountId;
  // Implicit (64 hex) and long named accounts: keep both ends.
  return `${accountId.slice(0, 10)}…${accountId.slice(-8)}`;
}
