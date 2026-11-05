import { accountIdSchema } from "@/config/network";

const IMPLICIT_ACCOUNT = /^[0-9a-f]{64}$/;
const ETH_IMPLICIT_ACCOUNT = /^0x[0-9a-f]{40}$/;

export function isValidAccountId(value: string): boolean {
  return accountIdSchema.safeParse(value).success;
}

/**
 * Whether an account can exist on testnet. Named testnet accounts end in
 * `.testnet`; implicit accounts are network-agnostic hex IDs.
 */
export function isTestnetAccount(accountId: string): boolean {
  return (
    accountId.endsWith(".testnet") || IMPLICIT_ACCOUNT.test(accountId) || ETH_IMPLICIT_ACCOUNT.test(accountId)
  );
}
