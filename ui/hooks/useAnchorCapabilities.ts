import { useState, useEffect, useCallback, useRef } from 'react';

// ── Service constants ─────────────────────────────────────────────────────────
// Mirror of src/types.rs SERVICE_* constants kept in sync with the on-chain values.

export const SERVICE_DEPOSITS    = 1;
export const SERVICE_WITHDRAWALS = 2;
export const SERVICE_QUOTES      = 3;
export const SERVICE_KYC         = 4;

// ── Public types ──────────────────────────────────────────────────────────────

/** TypeScript mirror of the Soroban AnchorServices contract type. */
export interface AnchorServicesResult {
  anchor: string;
  services: number[];
}

/**
 * Function that calls `get_supported_services(attestor)` on the contract layer
 * and resolves with the structured result.
 */
export type FetchCapabilitiesFn = (attestor: string) => Promise<AnchorServicesResult>;

export interface UseAnchorCapabilitiesOptions {
  /** Function that calls `get_supported_services` via the contract layer. */
  fetchCapabilities: FetchCapabilitiesFn;
  /** Set to false to skip fetching (e.g. while waiting for auth). Defaults to true. */
  enabled?: boolean;
}

export interface UseAnchorCapabilitiesResult {
  /** Resolved AnchorServices, or null while loading / on error / when attestor is absent. */
  capabilities: AnchorServicesResult | null;
  /** True only during the initial (no-cache) fetch; false during background revalidation. */
  isLoading: boolean;
  /** Last fetch error. Cleared on the next successful fetch. */
  error: Error | null;
  /** Evict the cached entry and force an immediate refetch. */
  mutate: () => void;
}

// ── Module-level SWR cache ────────────────────────────────────────────────────
// Shared across all hook instances so multiple components using the same attestor
// key share one in-flight request and one cached result.

interface CacheEntry {
  data: AnchorServicesResult;
  fetchedAt: number;
}

const CACHE_TTL_MS = 30_000;
const MAX_CACHE_ENTRIES = 128;

const cache       = new Map<string, CacheEntry>();
const inFlight    = new Map<string, Promise<AnchorServicesResult>>();
const subscribers = new Map<string, Set<() => void>>();

function evictExpiredEntries(): void {
  const now = Date.now();
  for (const [key, entry] of cache.entries()) {
    if (now - entry.fetchedAt > CACHE_TTL_MS) {
      cache.delete(key);
      inFlight.delete(key);
      subscribers.delete(key);
    }
  }
}

function pruneCache(): void {
  while (cache.size > MAX_CACHE_ENTRIES) {
    const oldestKey = cache.keys().next().value as string | undefined;
    if (oldestKey === undefined) break;
    cache.delete(oldestKey);
    inFlight.delete(oldestKey);
    subscribers.delete(oldestKey);
  }
}

function subscribe(key: string, listener: () => void): () => void {
  if (!subscribers.has(key)) subscribers.set(key, new Set());
  subscribers.get(key)!.add(listener);
  return () => {
    const set = subscribers.get(key);
    if (!set) return;
    set.delete(listener);
    if (set.size === 0) subscribers.delete(key);
  };
}

function notify(key: string): void {
  // React 18 automatically batches state updates from promises, timeouts, and
  // event handlers, so explicit batching is no longer needed.
  subscribers.get(key)?.forEach(fn => fn());
}

/**
 * Validates a Stellar StrKey address.
 * Accepts G-addresses (Ed25519 public keys) and C-addresses (contract IDs):
 * both are 56 characters of Base32 (A-Z, 2-7).
 */
function isValidAttestor(v: string | null | undefined): v is string {
  if (!v || typeof v !== 'string') return false;
  return v.length === 56 && /^[GC][A-Z2-7]{55}$/.test(v);
}

async function fetchAndCache(
  key: string,
  fetchFn: FetchCapabilitiesFn,
): Promise<AnchorServicesResult> {
  // Deduplicate: if there is already an in-flight request for this key, reuse it.
  const existing = inFlight.get(key);
  if (existing) return existing;

  const promise = fetchFn(key)
    .then(data => {
      evictExpiredEntries();
      cache.set(key, { data, fetchedAt: Date.now() });
      pruneCache();
      inFlight.delete(key);
      notify(key); // wake all cross-component subscribers
      return data;
    })
    .catch(err => {
      inFlight.delete(key);
      throw err;
    });

  inFlight.set(key, promise);
  return promise;
}

/**
 * Evict all cached entries and cancel deduplication state.
 * Intended for use in tests between cases to ensure isolation.
 */
export function clearCapabilitiesCache(): void {
  cache.clear();
  inFlight.clear();
  subscribers.clear();
}

// ── Hook ──────────────────────────────────────────────────────────────────────

interface LocalState {
  isFetching: boolean;
  error: Error | null;
  /** Incremented to force a re-read from the module-level cache map. */
  tick: number;
}

/**
 * Calls `get_supported_services(attestor)` via the provided `fetchCapabilities`
 * function and caches the result using a stale-while-revalidate strategy.
 *
 * - Results are shared across all consumers of the same `attestor` key.
 * - Only one in-flight request is made even when multiple components mount
 *   simultaneously with the same attestor.
 * - Mounting with a cached entry returns stale data immediately and revalidates
 *   in the background without showing a loading state.
 * - Skips all requests when `attestor` is null/undefined or fails StrKey
 *   validation, and when `options.enabled` is false.
 *
 * @example
 * const { capabilities, isLoading, error, mutate } = useAnchorCapabilities(attestorAddr, {
 *   fetchCapabilities: (id) => contract.get_supported_services(id),
 * });
 */
export function useAnchorCapabilities(
  attestor: string | null | undefined,
  options: UseAnchorCapabilitiesOptions,
): UseAnchorCapabilitiesResult {
  const { enabled = true } = options;

  // Ref so that callers passing inline arrow functions don't restart the effect.
  const fetchFnRef = useRef<FetchCapabilitiesFn>(options.fetchCapabilities);
  fetchFnRef.current = options.fetchCapabilities;

  const key = isValidAttestor(attestor) && enabled ? attestor : null;

  // Initialise isFetching=true when there is a valid key but no cached entry,
  // so the very first render already reports isLoading=true.
  const [localState, setLocalState] = useState<LocalState>(() => ({
    isFetching: key !== null && !cache.has(key),
    error: null,
    tick: 0,
  }));

  useEffect(() => {
    if (!key) {
      setLocalState({ isFetching: false, error: null, tick: 0 });
      return;
    }

    let cancelled = false;

    // Subscribe to cache updates pushed by other components' fetches.
    const unsubscribe = subscribe(key, () => {
      if (!cancelled) setLocalState(s => ({ ...s, tick: s.tick + 1 }));
    });

    // SWR: if no cache entry, mark as loading; otherwise revalidate silently.
    if (!cache.has(key)) {
      setLocalState(s => ({ ...s, isFetching: true, error: null }));
    }

    fetchAndCache(key, id => fetchFnRef.current(id))
      .then(() => {
        if (cancelled) return;
        setLocalState(s => ({ ...s, isFetching: false, error: null, tick: s.tick + 1 }));
      })
      .catch(err => {
        if (cancelled) return;
        setLocalState(s => ({
          ...s,
          isFetching: false,
          error: err instanceof Error ? err : new Error(String(err)),
        }));
      });

    return () => {
      cancelled = true;
      unsubscribe();
    };
  }, [key]);

  // Derive from cache on every render; tick changes force re-reads after updates.
  const capabilities = key ? (cache.get(key)?.data ?? null) : null;

  // isLoading is only true when we have no data yet — background revalidation
  // (which always has capabilities !== null) does not set isLoading.
  const isLoading = localState.isFetching && capabilities === null;

  const mutate = useCallback(() => {
    if (!key) return;
    cache.delete(key);
    inFlight.delete(key);
    setLocalState(s => ({ ...s, isFetching: true, error: null }));
    fetchAndCache(key, id => fetchFnRef.current(id))
      .then(() =>
        setLocalState(s => ({ ...s, isFetching: false, tick: s.tick + 1 }))
      )
      .catch(err =>
        setLocalState(s => ({
          ...s,
          isFetching: false,
          error: err instanceof Error ? err : new Error(String(err)),
        }))
      );
  }, [key]);

  return { capabilities, isLoading, error: localState.error, mutate };
}
