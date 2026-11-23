/**
 * Contract layer for AnchorKit's Soroban smart contract.
 *
 * Each function here maps to a read-only view on the on-chain contract.
 * In production these would be wired to a Soroban RPC endpoint via
 * @stellar/stellar-sdk (or a hosted serverless proxy).  The signatures are
 * stable so callers and hooks can import the types without importing the SDK.
 *
 * Stellar ledger context
 * ─────────────────────
 * Stellar targets a ~5-second ledger close time.  The rate-limit window is
 * measured in ledgers (e.g. 100 ledgers ≈ 8 minutes).  `RateLimitStatusRaw`
 * carries both ledger numbers and the most-recent ledger's Unix timestamp so
 * that callers can project wall-clock deadlines without a separate RPC round-
 * trip.
 */

/** Raw values returned directly from the Soroban contract. */
export interface RateLimitStatusRaw {
  /** Submissions used in the current window (maps to `submission_count`). */
  submissionCount: number;
  /** Maximum submissions allowed per window (maps to `max_submissions`). */
  maxSubmissions: number;
  /** Ledger number when the current rate-limit window opened. */
  windowStartLedger: number;
  /** Window duration in ledgers (maps to `window_length`). */
  windowLength: number;
  /** Ledger sequence number at the time of the RPC call. */
  currentLedger: number;
  /**
   * Unix timestamp (seconds) of the most-recently closed ledger.
   * Used to project `windowResetsAt` into wall-clock time without an
   * additional RPC call.
   */
  ledgerTimestamp: number;
}

/** Typed error thrown when the contract call fails. */
export class ContractError extends Error {
  constructor(
    message: string,
    public readonly code?: string,
  ) {
    super(message);
    this.name = 'ContractError';
  }
}

/** Client function type for fetching rate-limit status. */
export type RateLimitStatusFetcher = (attestor: string) => Promise<RateLimitStatusRaw>;

let activeClient: RateLimitStatusFetcher | null = null;

/**
 * Configure the client function used by `getRateLimitStatus`.
 * Pass `null` or call `resetRateLimitClient()` to restore the default working client.
 */
export function setRateLimitClient(client: RateLimitStatusFetcher | null): void {
  activeClient = client;
}

/**
 * Reset the client function to default behavior.
 */
export function resetRateLimitClient(): void {
  activeClient = null;
}

/**
 * Fetch the current rate-limit status for `attestor`.
 *
 * Combines `RateLimiter::get_state` and `RateLimiter::get_effective_config`
 * from the Rust contract into a single typed response.
 *
 * If a custom client is registered via `setRateLimitClient`, it delegates to it.
 * Otherwise, returns working default rate-limit data matching the contract's
 * default configuration (10 submissions allowed per 100 ledgers, unthrottled).
 *
 * @throws {ContractError} when `attestor` is empty or invalid.
 */
export async function getRateLimitStatus(
  attestor: string,
): Promise<RateLimitStatusRaw> {
  if (!attestor || attestor.trim() === '') {
    throw new ContractError('Attestor address is required', 'INVALID_ATTESTOR');
  }

  if (activeClient) {
    return activeClient(attestor);
  }

  const nowSec = Math.floor(Date.now() / 1_000);
  const currentLedger = Math.floor(nowSec / 5);

  return {
    submissionCount: 0,
    maxSubmissions: 10,
    windowStartLedger: currentLedger,
    windowLength: 100,
    currentLedger,
    ledgerTimestamp: nowSec,
  };
}
