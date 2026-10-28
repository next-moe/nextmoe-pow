import type { Progress } from '@nextmoe/pow-solver'

const EXPIRY_MARGIN_MS = 10_000
const MIN_REFRESH_MS = 30_000

export const progressFraction = ({ solved, k, attempts, expectedAttempts }: Progress) =>
  Math.max(solved / k, Math.min(0.95, attempts / expectedAttempts))

/** When to fetch a fresh challenge, from the server's clock if known, never sooner than MIN_REFRESH_MS. */
export const refreshDelay = (expiresAtSeconds: number, serverNowMs: number) =>
  Math.max(expiresAtSeconds * 1000 - serverNowMs - EXPIRY_MARGIN_MS, MIN_REFRESH_MS)
