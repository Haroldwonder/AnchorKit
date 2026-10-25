import { useState, useEffect, useRef } from 'react';
import { TxStatus } from '../components/TransactionTimeline';
import {
  TransactionStateTracker,
  TransactionSnapshot,
  TransactionTransition,
  FetchStatusFn,
} from './TransactionStateTracker';

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
    maxBackoff = 60000
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

    const poll = async () => {
      try {
        const snap = await tracker.poll(txId);
        if (cancelled) return;
        setSnapshot(snap);
        setError(null);
        setRetriesExhausted(false);
        consecutiveErrors = 0; // reset on success
        if (!snap.isTerminal) {
          timerId = setTimeout(poll, interval);
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
        timerId = setTimeout(poll, backoffDelay);
      }
    };

    poll();

    return () => {
      cancelled = true;
      if (timerId !== null) clearTimeout(timerId);
      tracker.reset();
    };
  }, [txId, enabled, interval, maxRetries, maxBackoff]);

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
