import { useState, useEffect, useRef } from 'react';
import { isDocumentHidden, subscribeToVisibility } from './visibility';

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
  /** Number of polls that have failed in a row; reset to 0 on success. */
  consecutiveErrors: number;
}

export interface UseAnchorHealthOptions {
  /**
   * Pause polling while the browser tab is hidden and refresh immediately when
   * it becomes visible again. Set to `false` to keep polling in the background.
   * Defaults to `true`.
   */
  pauseWhenHidden?: boolean;
  /**
   * Upper bound (ms) for the exponential backoff applied after consecutive
   * errors. Defaults to {@link DEFAULT_MAX_BACKOFF_MS}.
   */
  maxBackoff?: number;
}

// --- Validation ---

/**
 * Accepts a checksum-valid Stellar StrKey starting with G (ed25519) or C
 * (contract). Re-exported from the shared `strkey` validator.
 */
export { isValidAttestor };

/**
 * Minimum allowed polling interval (ms). Prevents accidental 0/negative values
 * from causing a near-max-rate polling loop against the RPC/contract endpoint.
 */
export const MIN_POLL_INTERVAL_MS = 1000;

/** Default cap (ms) for the error backoff delay. */
export const DEFAULT_MAX_BACKOFF_MS = 60_000;

/**
 * Delay before the next poll after `consecutiveErrors` failures in a row:
 * `interval * 2^(consecutiveErrors - 1)`, capped at `maxBackoff` (but never
 * shorter than `interval`). Returns `interval` when there are no errors.
 */
export function computeBackoffDelay(
  interval: number,
  consecutiveErrors: number,
  maxBackoff: number = DEFAULT_MAX_BACKOFF_MS,
): number {
  if (consecutiveErrors <= 0) return interval;
  const delay = interval * Math.pow(2, consecutiveErrors - 1);
  return Math.max(interval, Math.min(delay, maxBackoff));
}

function isDevelopment(): boolean {
  return typeof process === 'undefined' || process.env.NODE_ENV !== 'production';
}

// --- Hook ---

/**
 * Polls `get_anchor_health_score` at `interval` milliseconds via the supplied
 * `getHealthScore` adapter and returns live health data.
 *
 * Design notes:
 * - `getHealthScore` is held in a ref so callers can pass inline functions
 *   without restarting the timer on every render.
 * - Overlapping requests are skipped: the next poll is only scheduled once the
 *   previous one has settled, and a poll requested while one is in-flight is
 *   a no-op.
 * - After consecutive errors the delay grows exponentially
 *   (`interval * 2^(errors - 1)`, capped at `options.maxBackoff`) and resets
 *   to `interval` after the next success. The failure count is exposed as
 *   `consecutiveErrors`.
 * - By default polling pauses while `document.visibilityState === 'hidden'`
 *   and resumes with an immediate refresh when the tab becomes visible again.
 *   Pass `{ pauseWhenHidden: false }` to opt out.
 * - When `attestor` changes the timer is cancelled, all state is reset, and a
 *   fresh polling cycle begins.  When only `interval` changes the timer is *
 *   restarted but state is NOT reset (the latest score remains visible).
 * - `loading` is only `true` during the very first fetch after mounting or
 *   after the attestor changes; background polls run silently.
 * - The `interval` is validated: if it is not a positive finite number, or if
 *   it is less than `MIN_POLL_INTERVAL_MS`, it is clamped to that minimum and
 *   a warning is emitted once per hook instance, outside production builds.
 */
export function useAnchorHealth(
  attestor: string | null | undefined,
  interval: number,
  getHealthScore: GetHealthScoreFn,
  options: UseAnchorHealthOptions = {},
): UseAnchorHealthResult {
  const { pauseWhenHidden = true, maxBackoff = DEFAULT_MAX_BACKOFF_MS } = options;

  const [score, setScore] = useState<number | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [consecutiveErrors, setConsecutiveErrors] = useState<number>(0);

  // Ensures the invalid-interval warning is logged at most once per instance.
  const warnedIntervalRef = useRef(false);

  // Failure streak shared across effect re-runs so an interval-only change
  // keeps the current backoff instead of silently resetting it.
  const errorCountRef = useRef(0);

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
        errorCountRef.current = 0;
        setConsecutiveErrors(0);
      }
      return;
    }

    // Reset state for a new attestor; preserve it for an interval-only change.
    if (attestorChanged) {
      setScore(null);
      setLastUpdated(null);
      setError(null);
      setLoading(true);
      errorCountRef.current = 0;
      setConsecutiveErrors(0);
    }

    // Normalize the interval, enforcing a sane minimum to prevent runaway polling.
    const normalizedInterval =
      Number.isFinite(interval) && interval > 0
        ? Math.max(interval, MIN_POLL_INTERVAL_MS)
        : MIN_POLL_INTERVAL_MS;

    if (normalizedInterval !== interval && !warnedIntervalRef.current && isDevelopment()) {
      warnedIntervalRef.current = true;
      console.warn('[useAnchorHealth] Invalid poll interval ' + String(interval) + 'ms. Using ' + MIN_POLL_INTERVAL_MS + 'ms instead.');
    }

    let cancelled = false;
    // Per-closure in-flight flag prevents overlapping requests.
    let inFlight = false;
    let timerId: ReturnType<typeof setTimeout> | null = null;

    const clearTimer = () => {
      if (timerId !== null) {
        clearTimeout(timerId);
        timerId = null;
      }
    };

    const scheduleNext = () => {
      clearTimer();
      if (cancelled) return;
      // While hidden, stay idle; the visibility listener resumes polling.
      if (pauseWhenHidden && isDocumentHidden()) return;
      timerId = setTimeout(poll, computeBackoffDelay(normalizedInterval, errorCountRef.current, maxBackoff));
    };

    const poll = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const newScore = await getHealthScoreRef.current(attestor);
        if (!cancelled) {
          errorCountRef.current = 0;
          setScore(newScore);
          setLastUpdated(new Date());
          setError(null);
          setLoading(false);
          setConsecutiveErrors(0);
        }
      } catch (err) {
        if (!cancelled) {
          errorCountRef.current += 1;
          setError(
            err instanceof Error
              ? err
              : new Error('Failed to fetch health score. Please try again.'),
          );
          setLoading(false);
          setConsecutiveErrors(errorCountRef.current);
        }
      } finally {
        inFlight = false;
        scheduleNext();
      }
    };

    const unsubscribe = pauseWhenHidden
      ? subscribeToVisibility((hidden) => {
          if (cancelled) return;
          if (hidden) {
            clearTimer();
          } else {
            // Resume with an immediate refresh; poll() reschedules itself.
            clearTimer();
            poll();
          }
        })
      : () => {};

    poll();

    return () => {
      cancelled = true;
      clearTimer();
      unsubscribe();
    };
  }, [attestor, interval, pauseWhenHidden, maxBackoff]); // getHealthScore intentionally excluded - kept in ref
  return { score, loading, error, lastUpdated, consecutiveErrors };
}
