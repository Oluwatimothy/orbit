import { network } from "@/config/network";

const IPFS_PREFIX = "ipfs://";

/**
 * Turns a stored media reference into a URL the browser can load. Only
 * `ipfs://`, `https://` and inline image data URIs are ever rendered; anything
 * else returns null so callers show a fallback instead.
 */
export function resolveMedia(reference: string | null | undefined, baseUri?: string | null): string | null {
  if (!reference) return null;
  if (reference.startsWith(IPFS_PREFIX)) {
    return `${network.ipfsGateway}/${reference.slice(IPFS_PREFIX.length)}`;
  }
  if (reference.startsWith("https://")) return reference;
  if (/^data:image\/(png|jpeg|webp|gif|svg\+xml);base64,/.test(reference)) return reference;
  if (baseUri && /^[\w.-]+$/.test(reference)) {
    return resolveMedia(`${baseUri.replace(/\/$/, "")}/${reference}`);
  }
  return null;
}

/** Whether a reference is content-addressed (its bytes cannot change silently). */
export function isContentAddressed(reference: string): boolean {
  return (
    reference.startsWith(IPFS_PREFIX) ||
    reference.startsWith("data:") ||
    /\/ipfs\/[a-zA-Z0-9]+/.test(reference)
  );
}
