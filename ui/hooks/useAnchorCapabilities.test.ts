import { renderHook, act, waitFor } from '@testing-library/react';
import {
  useAnchorCapabilities,
  clearCapabilitiesCache,
  AnchorServicesResult,
  SERVICE_DEPOSITS,
  SERVICE_WITHDRAWALS,
  SERVICE_QUOTES,
  SERVICE_KYC,
} from './useAnchorCapabilities';

// ── Fixtures ──────────────────────────────────────────────────────────────────

// Valid Stellar StrKey addresses (G-address: 'G' + 55 Base32 chars)
const ATTESTOR   = 'G' + 'A'.repeat(55);
const ATTESTOR_2 = 'G' + 'B'.repeat(55);

const BASE_CAPS: AnchorServicesResult = {
  anchor: ATTESTOR,
  services: [SERVICE_DEPOSITS, SERVICE_WITHDRAWALS],
};

const FULL_CAPS: AnchorServicesResult = {
  anchor: ATTESTOR,
  services: [SERVICE_DEPOSITS, SERVICE_WITHDRAWALS, SERVICE_QUOTES, SERVICE_KYC],
};

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeFetch(...results: AnchorServicesResult[]) {
  let i = 0;
  return jest.fn(async (_: string) => {
    const r = results[Math.min(i, results.length - 1)];
    i += 1;
    return r;
  });
}

// ── Successful fetch ──────────────────────────────────────────────────────────

describe('useAnchorCapabilities — successful fetch', () => {
  beforeEach(() => clearCapabilitiesCache());

  test('isLoading is true initially, false after fetch resolves', async () => {
    const fetch = jest.fn().mockResolvedValue(BASE_CAPS);
    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    expect(result.current.isLoading).toBe(true);
    expect(result.current.capabilities).toBeNull();
    expect(result.current.error).toBeNull();

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.capabilities).toEqual(BASE_CAPS);
    expect(result.current.error).toBeNull();
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledWith(ATTESTOR);
  });

  test('passes the attestor address to fetchCapabilities', async () => {
    const fetch = jest.fn().mockResolvedValue(BASE_CAPS);
    renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );
    await waitFor(() => expect(fetch).toHaveBeenCalledWith(ATTESTOR));
  });

  test('capabilities includes all returned service codes', async () => {
    const fetch = jest.fn().mockResolvedValue(FULL_CAPS);
    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );
    await waitFor(() => expect(result.current.capabilities).toEqual(FULL_CAPS));
    expect(result.current.capabilities!.services).toEqual([
      SERVICE_DEPOSITS, SERVICE_WITHDRAWALS, SERVICE_QUOTES, SERVICE_KYC,
    ]);
  });
});

// ── Caching behavior ──────────────────────────────────────────────────────────

describe('useAnchorCapabilities — caching', () => {
  beforeEach(() => clearCapabilitiesCache());

  test('second consumer receives cached data immediately without loading', async () => {
    const fetch = jest.fn().mockResolvedValue(BASE_CAPS);

    // First consumer populates the cache.
    const { result: r1 } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );
    await waitFor(() => expect(r1.current.capabilities).toEqual(BASE_CAPS));

    // Second consumer mounts after cache is warm.
    const { result: r2 } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    // Should have data immediately (no loading state).
    expect(r2.current.capabilities).toEqual(BASE_CAPS);
    expect(r2.current.isLoading).toBe(false);
    expect(r2.current.error).toBeNull();
  });

  test('deduplicates concurrent requests for the same attestor', async () => {
    let resolve!: (v: AnchorServicesResult) => void;
    const promise = new Promise<AnchorServicesResult>(r => { resolve = r; });
    const fetch = jest.fn().mockReturnValue(promise);

    // Both hooks mount before the fetch resolves.
    const { result: r1 } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );
    const { result: r2 } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    // Only one network request despite two consumers.
    expect(fetch).toHaveBeenCalledTimes(1);

    act(() => { resolve(BASE_CAPS); });

    await waitFor(() => expect(r1.current.capabilities).toEqual(BASE_CAPS));
    expect(r2.current.capabilities).toEqual(BASE_CAPS);
  });

  test('separate attestor keys result in independent fetches', async () => {
    const caps2: AnchorServicesResult = { anchor: ATTESTOR_2, services: [SERVICE_KYC] };
    const fetch = jest.fn()
      .mockImplementation(async (id: string) =>
        id === ATTESTOR ? BASE_CAPS : caps2
      );

    const { result: r1 } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );
    const { result: r2 } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR_2, { fetchCapabilities: fetch })
    );

    await waitFor(() => expect(r1.current.capabilities).toEqual(BASE_CAPS));
    await waitFor(() => expect(r2.current.capabilities).toEqual(caps2));

    expect(fetch).toHaveBeenCalledWith(ATTESTOR);
    expect(fetch).toHaveBeenCalledWith(ATTESTOR_2);
  });

  test('cross-component cache update: consumer B sees data fetched by consumer A', async () => {
    let resolveA!: (v: AnchorServicesResult) => void;
    const promiseA = new Promise<AnchorServicesResult>(r => { resolveA = r; });
    const fetch = jest.fn().mockReturnValue(promiseA);

    renderHook(() => useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch }));
    const { result: r2 } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    expect(r2.current.capabilities).toBeNull(); // not yet fetched

    act(() => { resolveA(BASE_CAPS); });

    await waitFor(() => expect(r2.current.capabilities).toEqual(BASE_CAPS));
  });

  test('evicts stale entries after the cache TTL and revalidates', async () => {
    jest.useFakeTimers();
    const fetch = jest.fn()
      .mockResolvedValueOnce(BASE_CAPS)
      .mockResolvedValueOnce(FULL_CAPS);

    const { result, unmount } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));
    unmount();

    act(() => {
      jest.setSystemTime(new Date(Date.now() + 31 * 60 * 1000));
    });

    const { result: refreshed } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    expect(refreshed.current.capabilities).toEqual(BASE_CAPS);
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(refreshed.current.capabilities).toEqual(FULL_CAPS));

    jest.useRealTimers();
  });
});

// ── SWR — stale-while-revalidate ──────────────────────────────────────────────

describe('useAnchorCapabilities — stale-while-revalidate', () => {
  beforeEach(() => clearCapabilitiesCache());

  test('returns stale data immediately on mount with warm cache', async () => {
    const fetch = makeFetch(BASE_CAPS, FULL_CAPS);

    // Populate the cache.
    const { unmount } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
    unmount();

    // New consumer: should see stale data before revalidation completes.
    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    expect(result.current.capabilities).toEqual(BASE_CAPS);
    expect(result.current.isLoading).toBe(false); // not blocking
  });

  test('updates to fresh data after background revalidation', async () => {
    const fetch = makeFetch(BASE_CAPS, FULL_CAPS);

    const { unmount } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
    unmount();

    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(FULL_CAPS));
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  test('does not set isLoading=true during background revalidation', async () => {
    const fetch = makeFetch(BASE_CAPS, FULL_CAPS);

    const { unmount } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
    unmount();

    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    // isLoading must remain false throughout — never flickers true
    const loadingValues: boolean[] = [result.current.isLoading];
    await waitFor(() => expect(result.current.capabilities).toEqual(FULL_CAPS));
    loadingValues.push(result.current.isLoading);

    expect(loadingValues.every(v => v === false)).toBe(true);
  });
});

// ── mutate ────────────────────────────────────────────────────────────────────

describe('useAnchorCapabilities — mutate', () => {
  beforeEach(() => clearCapabilitiesCache());

  test('mutate forces a fresh fetch', async () => {
    const fetch = makeFetch(BASE_CAPS, FULL_CAPS);
    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));

    act(() => { result.current.mutate(); });

    await waitFor(() => expect(result.current.capabilities).toEqual(FULL_CAPS));
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  test('mutate evicts the cache so isLoading becomes true', async () => {
    const fetch = makeFetch(BASE_CAPS, FULL_CAPS);
    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));

    act(() => { result.current.mutate(); });

    // Cache evicted: no data + fetching → isLoading=true
    expect(result.current.isLoading).toBe(true);
    expect(result.current.capabilities).toBeNull();
  });

  test('mutate clears a previous error and retries', async () => {
    const fetch = jest.fn()
      .mockRejectedValueOnce(new Error('network timeout'))
      .mockResolvedValueOnce(BASE_CAPS);

    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.error!.message).toBe('network timeout');

    act(() => { result.current.mutate(); });

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));
    expect(result.current.error).toBeNull();
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  test('mutate is a no-op when attestor is invalid', () => {
    const fetch = jest.fn();
    const { result } = renderHook(() =>
      useAnchorCapabilities(null, { fetchCapabilities: fetch })
    );

    act(() => { result.current.mutate(); });

    expect(fetch).not.toHaveBeenCalled();
  });
});

// ── Error handling ────────────────────────────────────────────────────────────

describe('useAnchorCapabilities — error handling', () => {
  beforeEach(() => clearCapabilitiesCache());

  test('sets error when fetchCapabilities rejects', async () => {
    const fetch = jest.fn().mockRejectedValue(new Error('contract error'));
    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.error!.message).toBe('contract error');
    expect(result.current.capabilities).toBeNull();
    expect(result.current.isLoading).toBe(false);
  });

  test('wraps non-Error rejections', async () => {
    const fetch = jest.fn().mockRejectedValue('ServicesNotConfigured');
    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    await waitFor(() => expect(result.current.error).toBeInstanceOf(Error));
    expect(result.current.error!.message).toBe('ServicesNotConfigured');
  });

  test('clears error on next successful fetch after background revalidation', async () => {
    // First mount: error
    const fetch = jest.fn()
      .mockRejectedValueOnce(new Error('rpc down'))
      .mockResolvedValue(BASE_CAPS);

    const { unmount } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
    unmount();

    // Second mount: should succeed and clear error
    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch })
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));
    expect(result.current.error).toBeNull();
  });

  test('preserves stale data when background revalidation fails', async () => {
    const fetch = makeFetch(BASE_CAPS);
    // fetchFn always returns BASE_CAPS for first call
    const fetchWithError = jest.fn()
      .mockResolvedValueOnce(BASE_CAPS)
      .mockRejectedValueOnce(new Error('revalidation failed'));

    const { unmount } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetchWithError })
    );
    await waitFor(() => expect(fetchWithError).toHaveBeenCalledTimes(1));
    unmount();

    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetchWithError })
    );

    // Returns stale data immediately
    expect(result.current.capabilities).toEqual(BASE_CAPS);

    // Revalidation fails — but stale data is preserved
    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.capabilities).toEqual(BASE_CAPS);
    expect(result.current.error!.message).toBe('revalidation failed');
  });
});

// ── Skip conditions ───────────────────────────────────────────────────────────

describe('useAnchorCapabilities — skip conditions', () => {
  beforeEach(() => clearCapabilitiesCache());

  test('does not fetch when attestor is null', () => {
    const fetch = jest.fn();
    const { result } = renderHook(() =>
      useAnchorCapabilities(null, { fetchCapabilities: fetch })
    );
    expect(fetch).not.toHaveBeenCalled();
    expect(result.current.capabilities).toBeNull();
    expect(result.current.isLoading).toBe(false);
  });

  test('does not fetch when attestor is undefined', () => {
    const fetch = jest.fn();
    renderHook(() => useAnchorCapabilities(undefined, { fetchCapabilities: fetch }));
    expect(fetch).not.toHaveBeenCalled();
  });

  test.each([
    ['empty string', ''],
    ['too short', 'GABC'],
    ['wrong leading char', 'X' + 'A'.repeat(55)],
    ['invalid base32 char', 'G' + 'A'.repeat(54) + '0'], // '0' is not in A-Z or 2-7
    ['too long', 'G' + 'A'.repeat(56)],
    ['lowercase', 'g' + 'a'.repeat(55)],
  ])('does not fetch for invalid attestor: %s', (_, addr) => {
    const fetch = jest.fn();
    renderHook(() => useAnchorCapabilities(addr, { fetchCapabilities: fetch }));
    expect(fetch).not.toHaveBeenCalled();
  });

  test('accepts a valid C-address (contract ID)', async () => {
    const contractAddr = 'C' + 'A'.repeat(55);
    const caps: AnchorServicesResult = { anchor: contractAddr, services: [SERVICE_DEPOSITS] };
    const fetch = jest.fn().mockResolvedValue(caps);

    const { result } = renderHook(() =>
      useAnchorCapabilities(contractAddr, { fetchCapabilities: fetch })
    );
    await waitFor(() => expect(result.current.capabilities).toEqual(caps));
    expect(fetch).toHaveBeenCalledWith(contractAddr);
  });

  test('does not fetch when enabled is false', () => {
    const fetch = jest.fn();
    const { result } = renderHook(() =>
      useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch, enabled: false })
    );
    expect(fetch).not.toHaveBeenCalled();
    expect(result.current.capabilities).toBeNull();
    expect(result.current.isLoading).toBe(false);
  });

  test('starts fetching when enabled toggles from false to true', async () => {
    const fetch = jest.fn().mockResolvedValue(BASE_CAPS);
    const { result, rerender } = renderHook(
      ({ enabled }: { enabled: boolean }) =>
        useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch, enabled }),
      { initialProps: { enabled: false } }
    );

    expect(fetch).not.toHaveBeenCalled();

    rerender({ enabled: true });

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  test('stops revalidating when enabled toggles from true to false', async () => {
    const fetch = jest.fn().mockResolvedValue(BASE_CAPS);
    const { rerender } = renderHook(
      ({ enabled }: { enabled: boolean }) =>
        useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fetch, enabled }),
      { initialProps: { enabled: true } }
    );

    await waitFor(() => expect(fetch).toHaveBeenCalled());
    const callsBefore = fetch.mock.calls.length;

    rerender({ enabled: false });

    // No further fetches should be triggered.
    await waitFor(() => expect(fetch.mock.calls.length).toBeGreaterThanOrEqual(callsBefore));
    expect(fetch.mock.calls.length).toBeLessThanOrEqual(callsBefore + 1); // at most the revalidation in progress
  });
});

// ── fetchCapabilities reference stability ─────────────────────────────────────

describe('useAnchorCapabilities — inline fetchCapabilities stability', () => {
  beforeEach(() => clearCapabilitiesCache());

  test('reference change does not restart the fetch or reset capabilities', async () => {
    const fetch1 = jest.fn().mockResolvedValue(BASE_CAPS);
    const fetch2 = jest.fn().mockResolvedValue(FULL_CAPS);

    const { result, rerender } = renderHook(
      ({ fn }: { fn: jest.Mock }) =>
        useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fn }),
      { initialProps: { fn: fetch1 } }
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));
    expect(fetch1).toHaveBeenCalledTimes(1);

    rerender({ fn: fetch2 });

    // Effect did not re-run (key unchanged) → fetch2 was NOT called.
    expect(fetch2).not.toHaveBeenCalled();
    // Capabilities remain from the first fetch.
    expect(result.current.capabilities).toEqual(BASE_CAPS);
  });

  test('when mutate is called after ref change, the new function is used', async () => {
    const fetch1 = jest.fn().mockResolvedValue(BASE_CAPS);
    const fetch2 = jest.fn().mockResolvedValue(FULL_CAPS);

    const { result, rerender } = renderHook(
      ({ fn }: { fn: jest.Mock }) =>
        useAnchorCapabilities(ATTESTOR, { fetchCapabilities: fn }),
      { initialProps: { fn: fetch1 } }
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));

    rerender({ fn: fetch2 });

    act(() => { result.current.mutate(); });

    await waitFor(() => expect(result.current.capabilities).toEqual(FULL_CAPS));
    expect(fetch2).toHaveBeenCalledTimes(1);
    expect(fetch1).toHaveBeenCalledTimes(1);
  });
});

// ── attestor key change ───────────────────────────────────────────────────────

describe('useAnchorCapabilities — attestor key change', () => {
  beforeEach(() => clearCapabilitiesCache());

  test('fetches new attestor when key changes', async () => {
    const caps2: AnchorServicesResult = { anchor: ATTESTOR_2, services: [SERVICE_KYC] };
    const fetch = jest.fn()
      .mockResolvedValueOnce(BASE_CAPS)
      .mockResolvedValueOnce(caps2);

    const { result, rerender } = renderHook(
      ({ addr }: { addr: string }) =>
        useAnchorCapabilities(addr, { fetchCapabilities: fetch }),
      { initialProps: { addr: ATTESTOR } }
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));

    rerender({ addr: ATTESTOR_2 });

    await waitFor(() => expect(result.current.capabilities).toEqual(caps2));
    expect(fetch).toHaveBeenNthCalledWith(1, ATTESTOR);
    expect(fetch).toHaveBeenNthCalledWith(2, ATTESTOR_2);
  });

  test('resets to loading state when switching to an uncached attestor', async () => {
    // Use a controlled promise for the second fetch so it doesn't resolve inside
    // rerender's act() boundary — this lets us observe the in-flight loading state.
    let resolveAttesor2!: (v: AnchorServicesResult) => void;
    const caps2: AnchorServicesResult = { anchor: ATTESTOR_2, services: [SERVICE_KYC] };
    const slowPromise = new Promise<AnchorServicesResult>(r => { resolveAttesor2 = r; });

    const fetch = jest.fn()
      .mockResolvedValueOnce(BASE_CAPS)
      .mockReturnValueOnce(slowPromise);

    const { result, rerender } = renderHook(
      ({ addr }: { addr: string }) =>
        useAnchorCapabilities(addr, { fetchCapabilities: fetch }),
      { initialProps: { addr: ATTESTOR } }
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));

    rerender({ addr: ATTESTOR_2 });

    // slowPromise hasn't resolved → in-flight → isLoading=true, capabilities=null.
    await waitFor(() => expect(result.current.isLoading).toBe(true));
    expect(result.current.capabilities).toBeNull();

    // Resolve and confirm final state.
    act(() => { resolveAttesor2(caps2); });
    await waitFor(() => expect(result.current.capabilities).toEqual(caps2));
  });

  test('clears state when switching to a null attestor', async () => {
    const fetch = jest.fn().mockResolvedValue(BASE_CAPS);
    const { result, rerender } = renderHook(
      ({ addr }: { addr: string | null }) =>
        useAnchorCapabilities(addr, { fetchCapabilities: fetch }),
      { initialProps: { addr: ATTESTOR as string | null } }
    );

    await waitFor(() => expect(result.current.capabilities).toEqual(BASE_CAPS));

    rerender({ addr: null });

    expect(result.current.capabilities).toBeNull();
    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeNull();
  });
});
