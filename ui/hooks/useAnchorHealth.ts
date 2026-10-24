import { useState, useEffect, useRef } from 'react';

// --- Public types ---

/** Contract call signature for fetching the on-chain health score. */
export type GetHealthScoreFn = (attestor: string) => Promise<number>;

export interface UseAnchorHealthResult {
  /** Latest health score (0-100), or null before the first successful poll. */
  score: number | null;
  /** True only during the initial fetch before any score or error has arrived. */
  isLoading: boolean;
  /** @deprecated Use isLoading instead. */
  loading: boolean;
  /** The most recent failed poll, or null when healthy. */
  error: Error | null;
  /** @deprecated Use error.message instead. */
  errorMessage: string | null;
  /** Timestamp of the most recent successful score fetch; null until first success. */
  lastUpdated: Date | null;
}

// --- Validation ---

/** Accepts 56-char Stellar StrKey starting with G (ed25519) or C (contract). */
export function isValidAttestor(addr: string | null | undefined): addr is string {
  if (!addr) return false;
  return /^[GC][A-Z2-7]{55}$/.test(addr);
}

/**
 * Minimum allowed polling interval (ms). Prevents accidental 0/negative values
 * from causing a near-max-rate polling loop against the RPC/contract endpoint.
 */
export const MIN_POLL_INTERVAL_MS = 1000;

// --- Hook ---

/**
 * Polls `get_anchor_health_score` at `interval` milliseconds via the supplied
 * `getHealthScore` adapter and returns live health data.
 *
 * Design notes:
 * - `getHealthScore` is held in a ref so callers can pass inline functions
 *   without restarting the timer on every render.
 * - Overlapping requests are skipped: if the previous poll is still in-flight
 *   when the interval fires, the new tick is a no-op.
 * - When `attestor` changes the timer is cancelled, all state is reset, and a
 *   fresh polling cycle begins.  When only `interval` changes the timer is *
 *   restarted but state is NOT reset (the latest score remains visible).
 * - `loading` is only `true` during the very first fetch after mounting or
 *   after the attestor changes; background polls run silently.
 * - The `interval` is validated: if it is not a positive finite number, or if
 *   it is less than `MIN_POLL_INTERVAL_MS`, it is clamped to that minimum and
 *   a dev-mode warning is emitted.
 */
export function useAnchorHealth(
  attestor: string | null | undefined,
  interval: number,
  getHealthScore: GetHealthScoreFn,
): UseAnchorHealthResult {
  const [score, setScore] = useState<number | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  // Keeps the latest function reference without adding it to effect deps,
  // preventing timer restarts when the caller passes a new inline function.
  const getHealthScoreRef = useRef<GetHealthScoreFn>(getHealthScore);
  getHealthScoreRef.current = getHealthScore;

  // Track previous attestor so we can distinguish "attestor changed" from
  // "only interval changed" and avoid resetting visible state unnecessarily.
  const prevAttestorRef = useRef<string | null | undefined>(undefined);

  useEffect(() => {
    const attestorChanged = prevAttestorRef.current !== attestor;
    prevAttestorRef.current = attestor;

    if (!isValidAttestor(attestor)) {
      if (attestorChanged) {
        setScore(null);
        setLastUpdated(null);
        setError(null);
        setLoading(false);
      }
      return;
    }

    // Reset state for a new attestor; preserve it for an interval-only change.
    if (attestorChanged) {
      setScore(null);
      setLastUpdated(null);
      setError(null);
      setLoading(true);
    }

    // Normalize the interval, enforcing a sane minimum to prevent runaway polling.
    const normalizedInterval =
      Number.isFinite(interval) && interval > 0
        ? Math.max(interval, MIN_POLL_INTERVAL_MS)
        : MIN_POLL_INTERVAL_MS;

    if (normalizedInterval !== interval) {
      console.warn('[useAnchorHealth] Invalid poll interval ' + String(interval) + 'ms. Using ' + MIN_POLL_INTERVAL_MS + 'ms instead.');
    }

    let cancelled = false;
    // Per-closure in-flight flag prevents overlapping requests.
    let inFlight = false;

    const poll = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const newScore = await getHealthScoreRef.current(attestor);
        if (!cancelled) {
          setScore(newScore);
          setLastUpdated(new Date());
          setError(null);
          setLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          setError(
            err instanceof Error
              ? err
              : new Error('Failed to fetch health score. Please try again.'),
          );
          setLoading(false);
        }
      } finally {
        inFlight = false;
      }
    };

    poll();
    const timerId = setInterval(poll, normalizedInterval);

    return () => {
      cancelled = true;
      clearInterval(timerId);
    };
  }, [attestor, interval]); // getHealthScore intentionally excluded - kept in ref
  return { score, loading, error, lastUpdated };
}
