import { useState, useEffect, useRef } from 'react';
import { TxStatus } from '../components/TransactionTimeline';
import {
  TransactionStateTracker,
  TransactionSnapshot,
  TransactionTransition,
  FetchStatusFn,
} from './TransactionStateTracker';
import { isDocumentHidden, subscribeToVisibility } from './visibility';

export type { FetchStatusFn, TransactionTransition, TransactionSnapshot };

export interface UseTransactionStatusOptions {
  /** Function that fetches the current status for a given transaction ID. */
  fetchStatus: FetchStatusFn;
  /** Polling interval in milliseconds. Defaults to 5000. */
  interval?: number;
  /** Set to false to pause polling without unmounting. Defaults to true. */
  enabled?: boolean;
  /** Maximum number of consecutive retries before giving up. Defaults to Infinity. */
  maxRetries?: number;
  /** Maximum backoff delay in milliseconds. Defaults to 60000 (1 minute). */
  maxBackoff?: number;
  /**
   * Pause polling while the browser tab is hidden and refresh immediately when
   * it becomes visible again. Set to false to keep polling in the background.
   * Defaults to true.
   */
  pauseWhenHidden?: boolean;
}

export interface UseTransactionStatusResult {
  /** Current transaction status, or null before the first poll completes. */
  status: TxStatus | null;
  /** Ordered list of recorded status transitions. */
  transitions: TransactionTransition[];
  /** True when status is terminal (completed, failed, or refunded). */
  isTerminal: boolean;
  /** True while polling is active and the transaction is not terminal. */
  isPolling: boolean;
  /** True when maxRetries was reached before a terminal transaction status. */
  retriesExhausted: boolean;
  /** True while the initial fetch is still waiting on the first successful poll. */
  isLoading: boolean;
  /** @deprecated Use isLoading instead. */
  loading: boolean;
  /** Last fetch error, if any. Cleared on the next successful poll. */
  error: Error | null;
}

/**
 * Polls a TransactionStateTracker for the given txId and exposes a consistent
 * status interface. Polling starts automatically when txId is valid and
 * enabled is true, stops once a terminal state is reached, and cleans up
 * timers on unmount.
 *
 * @example
 * const { status, transitions, isTerminal } = useTransactionStatus(txId, {
 *   fetchStatus: (id) => api.getTransactionStatus(id),
 *   interval: 3000,
 * });
 */
export function useTransactionStatus(
  txId: string | null | undefined,
  options: UseTransactionStatusOptions
): UseTransactionStatusResult {
  const { 
    interval = 5000, 
    enabled = true,
    maxRetries = Infinity,
    maxBackoff = 60000,
    pauseWhenHidden = true,
  } = options;

  // Ref so that an inline fetchStatus function doesn't restart the polling loop
  const fetchStatusRef = useRef<FetchStatusFn>(options.fetchStatus);
  fetchStatusRef.current = options.fetchStatus;

  const [snapshot, setSnapshot] = useState<TransactionSnapshot | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [isPolling, setIsPolling] = useState(false);
  const [retriesExhausted, setRetriesExhausted] = useState(false);

  useEffect(() => {
    if (!txId || !enabled) {
      setSnapshot(null);
      setError(null);
      setIsPolling(false);
      setRetriesExhausted(false);
      return;
    }

    setIsPolling(true);
    setRetriesExhausted(false);

    const tracker = new TransactionStateTracker((id) => fetchStatusRef.current(id));
    let cancelled = false;
    let timerId: ReturnType<typeof setTimeout> | null = null;
    let consecutiveErrors = 0;
    let inFlight = false;
    // Set when a poll is due but deferred because the tab is hidden.
    let pendingWhileHidden = false;

    const schedule = (delay: number) => {
      if (pauseWhenHidden && isDocumentHidden()) {
        pendingWhileHidden = true;
        return;
      }
      timerId = setTimeout(poll, delay);
    };

    const poll = async () => {
      if (inFlight) return;
      inFlight = true;
      timerId = null;
      try {
        const snap = await tracker.poll(txId);
        if (cancelled) return;
        setSnapshot(snap);
        setError(null);
        setRetriesExhausted(false);
        consecutiveErrors = 0; // reset on success
        if (!snap.isTerminal) {
          schedule(interval);
        } else {
          setIsPolling(false);
        }
      } catch (err) {
        if (cancelled) return;
        consecutiveErrors++;
        setError(err instanceof Error ? err : new Error(String(err)));
        
        // Stop retrying if max attempts reached
        if (consecutiveErrors >= maxRetries) {
          setIsPolling(false);
          setRetriesExhausted(true);
          return;
        }
        
        // Exponential backoff with cap: min(interval * 2^attempts, maxBackoff)
        const backoffDelay = Math.min(
          interval * Math.pow(2, consecutiveErrors - 1),
          maxBackoff
        );
        schedule(backoffDelay);
      } finally {
        inFlight = false;
      }
    };

    const unsubscribe = pauseWhenHidden
      ? subscribeToVisibility((hidden) => {
          if (cancelled) return;
          if (hidden) {
            if (timerId !== null) {
              clearTimeout(timerId);
              timerId = null;
              pendingWhileHidden = true;
            }
          } else if (pendingWhileHidden) {
            // Resume with an immediate refresh.
            pendingWhileHidden = false;
            poll();
          }
        })
      : () => {};

    poll();

    return () => {
      cancelled = true;
      if (timerId !== null) clearTimeout(timerId);
      unsubscribe();
      tracker.reset();
    };
  }, [txId, enabled, interval, maxRetries, maxBackoff, pauseWhenHidden]);

  const isLoading = snapshot === null && error === null;

  return {
    status: snapshot?.status ?? null,
    transitions: snapshot?.transitions ?? [],
    isTerminal: snapshot?.isTerminal ?? false,
    isPolling,
    retriesExhausted,
    isLoading,
    loading: isLoading,
    error,
  };
}
