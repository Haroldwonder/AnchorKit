import { useState, useEffect, useCallback, useRef } from 'react';
import {
  getRateLimitStatus as defaultGetStatus,
  type RateLimitStatusRaw,
} from './contractClient';

/** Stellar's target ledger close time in milliseconds. */
const LEDGER_CLOSE_TIME_MS = 5_000;

/**
 * How long a cached result is considered fresh.
 * After this period the next render will trigger a new RPC call.
 */
const CACHE_TTL_MS = 30_000;

// ── Types ─────────────────────────────────────────────────────────────────────

/** UI-friendly rate-limit snapshot for a single attestor. */
export interface RateLimitStatus {
  /** Submissions consumed in the current window. */
  used: number;
  /** Maximum submissions allowed per window. */
  limit: number;
  /** Wall-clock time when the current window expires and the counter resets. */
  windowResetsAt: Date;
  /** `true` when `used >= limit` — the attestor cannot submit until the window resets. */
  isThrottled: boolean;
}

export interface UseRateLimitStatusResult {
  /** The latest rate-limit snapshot, or `null` before the first successful fetch. */
  status: RateLimitStatus | null;
  /** `true` while an RPC call is in-flight. */
  isLoading: boolean;
  /** @deprecated Use isLoading instead. */
  loading: boolean;
  /** The last fetch error, or `null` on success / before the first fetch. */
  error: Error | null;
  /**
   * Bust the cache for the current attestor and re-fetch immediately.
   * Useful for post-submission refreshes.
   */
  refresh: () => void;
}

type GetStatusFn = (attestor: string) => Promise<RateLimitStatusRaw>;

export interface UseRateLimitStatusOptions {
  /**
   * Override the contract-layer function used to fetch rate-limit data.
   * Primarily for testing; defaults to `getRateLimitStatus` from contractClient.
   */
  getStatus?: GetStatusFn;
}

// ── Module-level cache ────────────────────────────────────────────────────────
// Keyed by attestor address. Shared across hook instances to avoid duplicate
// RPC calls when multiple components mount with the same attestor.
// Implements LRU (Least Recently Used) eviction to prevent unbounded growth.

const MAX_CACHE_SIZE = 50; // Maximum number of cached attestors

class LRUCache<K, V> {
  private map = new Map<K, { value: V; timestamp: number }>();
  private maxSize: number;

  constructor(maxSize: number = MAX_CACHE_SIZE) {
    this.maxSize = maxSize;
  }

  get(key: K): V | undefined {
    const entry = this.map.get(key);
    if (entry) {
      // Move to end (most recently used)
      this.map.delete(key);
      this.map.set(key, entry);
      return entry.value;
    }
    return undefined;
  }

  set(key: K, value: V): void {
    // Remove if exists to update position
    this.map.delete(key);

    // Add new entry at end
    this.map.set(key, { value, timestamp: Date.now() });

    // Evict least recently used if over capacity
    if (this.map.size > this.maxSize) {
      const firstKey = this.map.keys().next().value;
      this.map.delete(firstKey);
    }
  }

  delete(key: K): void {
    this.map.delete(key);
  }

  clear(): void {
    this.map.clear();
  }
}

const cache = new LRUCache<string, { status: RateLimitStatus; fetchedAt: number }>(MAX_CACHE_SIZE);

/** Clear the module-level cache.  Call in `afterEach` to isolate tests. */
export function clearRateLimitCache(): void {
  cache.clear();
}

// ── Internal helpers ──────────────────────────────────────────────────────────

function computeStatus(raw: RateLimitStatusRaw): RateLimitStatus {
  const used = raw.submissionCount;
  const limit = raw.maxSubmissions;
  const isThrottled = used >= limit;

  // Ledgers remaining until the window closes (clamped to 0 when already expired).
  const ledgersUntilReset = Math.max(
    0,
    raw.windowStartLedger + raw.windowLength - raw.currentLedger,
  );

  const windowResetsAt = new Date(
    raw.ledgerTimestamp * 1_000 + ledgersUntilReset * LEDGER_CLOSE_TIME_MS,
  );

  return { used, limit, windowResetsAt, isThrottled };
}

// ── Hook ──────────────────────────────────────────────────────────────────────

/**
 * Fetches and caches the rate-limit status for a single `attestor`.
 *
 * Behaviour contract:
 * - Returns `{ status: null, loading: false, error: null }` immediately when
 *   `attestor` is `null`, `undefined`, or an empty/whitespace string.
 * - Sets `loading: true` for the duration of the RPC call.
 * - Caches successful results for {@link CACHE_TTL_MS} ms; re-mounts with the
 *   same attestor hit the cache and skip the network.
 * - Re-fetches automatically when `attestor` changes.
 * - Discards stale responses when a newer request arrives (race-safe).
 * - Exposes `refresh()` to manually bust the cache and force a new fetch.
 *
 * @example
 * ```tsx
 * const { status, loading, error, refresh } = useRateLimitStatus(attestorAddress);
 * if (status?.isThrottled) {
 *   return <p>Rate limited — resets at {status.windowResetsAt.toLocaleTimeString()}</p>;
 * }
 * ```
 */
export function useRateLimitStatus(
  attestor: string | null | undefined,
  options?: UseRateLimitStatusOptions,
): UseRateLimitStatusResult {
  const [status, setStatus] = useState<RateLimitStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Monotonic counter used to discard responses from superseded requests.
  const requestIdRef = useRef(0);

  // Keep the getter in a ref so `fetchStatus` only re-creates when `attestor`
  // changes, not when the caller passes a new `options` object each render.
  const getStatusRef = useRef<GetStatusFn>(options?.getStatus ?? defaultGetStatus);
  getStatusRef.current = options?.getStatus ?? defaultGetStatus;

  const fetchStatus = useCallback(async () => {
    if (!attestor || attestor.trim() === '') {
      setStatus(null);
      setError(null);
      setLoading(false);
      return;
    }

    // Return cached data if it is still fresh.
    const cached = cache.get(attestor);
    if (cached && Date.now() - cached.fetchedAt < CACHE_TTL_MS) {
      setStatus(cached.status);
      setError(null);
      return;
    }

    const id = ++requestIdRef.current;
    setLoading(true);
    setError(null);

    try {
      const raw = await getStatusRef.current(attestor);
      if (id !== requestIdRef.current) return; // stale — newer request superseded this one
      const computed = computeStatus(raw);
      cache.set(attestor, { status: computed, fetchedAt: Date.now() });
      setStatus(computed);
    } catch (err) {
      if (id !== requestIdRef.current) return;
      setError(err instanceof Error ? err : new Error(String(err)));
      setStatus(null);
    } finally {
      if (id === requestIdRef.current) {
        setLoading(false);
      }
    }
  }, [attestor]); // attestor is the only real dependency; getStatus lives in a ref

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  const refresh = useCallback(() => {
    if (attestor) {
      cache.delete(attestor); // evict so fetchStatus makes an RPC call
    }
    fetchStatus();
  }, [attestor, fetchStatus]);

  return { status, isLoading: loading, loading, error, refresh };
}
