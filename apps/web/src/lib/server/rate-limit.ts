/**
 * Fixed-window, per-key rate limit held in memory. On serverless platforms
 * each instance has its own window, so this bounds abuse per instance rather
 * than globally — acceptable for issuing short-lived, size-capped upload URLs.
 */
const windows = new Map<string, { start: number; count: number }>();

export function allowRequest(key: string, limit: number, windowMs: number, now = Date.now()): boolean {
  const current = windows.get(key);
  if (!current || now - current.start >= windowMs) {
    windows.set(key, { start: now, count: 1 });
    if (windows.size > 10_000) windows.clear();
    return true;
  }
  current.count += 1;
  return current.count <= limit;
}
