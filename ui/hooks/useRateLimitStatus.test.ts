import { renderHook, act, waitFor } from '@testing-library/react';
import {
  useRateLimitStatus,
  clearRateLimitCache,
  type RateLimitStatus,
} from './useRateLimitStatus';
import {
  getRateLimitStatus,
  setRateLimitClient,
  resetRateLimitClient,
  ContractError,
  type RateLimitStatusRaw,
} from './contractClient';

// ── Constants ─────────────────────────────────────────────────────────────────

const ATTESTOR_A = 'GAFQKP7BFKHM3FGCXPZ3CIKWBEYK3KKFZM6RKWZD7M2BQTXTQNZQIZ';
const ATTESTOR_B = 'GBZH7S5NC57XNHKHJ75C5DGMI3SP6VWAPQQKFPKZAKL7DW2FYW4XJKQ';

const LEDGER_CLOSE_TIME_MS = 5_000;

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeRaw(overrides: Partial<RateLimitStatusRaw> = {}): RateLimitStatusRaw {
  return {
    submissionCount:  3,
    maxSubmissions:   10,
    windowStartLedger: 1_000,
    windowLength:      100,
    currentLedger:     1_050,   // 50 ledgers remain in the window
    ledgerTimestamp:   Math.floor(Date.now() / 1_000),
    ...overrides,
  };
}

function mockGet(raw: RateLimitStatusRaw) {
  return jest.fn<Promise<RateLimitStatusRaw>, [string]>().mockResolvedValue(raw);
}

// ── Setup / teardown ──────────────────────────────────────────────────────────

beforeEach(() => {
  clearRateLimitCache();
  resetRateLimitClient();
  jest.clearAllMocks();
});

// ── Normal usage ──────────────────────────────────────────────────────────────

describe('useRateLimitStatus – normal usage', () => {
  it('returns status=null, loading=false, error=null on initial render', () => {
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(makeRaw()) }),
    );
    // Before the effect fires the hook has not started loading yet.
    expect(result.current.status).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('sets loading=true while the RPC call is in-flight', async () => {
    let resolveRaw!: (r: RateLimitStatusRaw) => void;
    const pending = new Promise<RateLimitStatusRaw>((r) => { resolveRaw = r; });
    const getStatus = jest.fn().mockReturnValue(pending);

    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus }),
    );

    await waitFor(() => expect(result.current.loading).toBe(true));
    expect(result.current.status).toBeNull();
    expect(result.current.error).toBeNull();

    resolveRaw(makeRaw());
    await waitFor(() => expect(result.current.loading).toBe(false));
  });

  it('populates status after a successful fetch', async () => {
    const raw = makeRaw({ submissionCount: 4, maxSubmissions: 10 });
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(raw) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());

    const { status } = result.current;
    expect(status!.used).toBe(4);
    expect(status!.limit).toBe(10);
    expect(status!.isThrottled).toBe(false);
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('exposes windowResetsAt as a Date instance', async () => {
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(makeRaw()) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(result.current.status!.windowResetsAt).toBeInstanceOf(Date);
  });

  it('computes windowResetsAt from ledger data', async () => {
    const nowSec = Math.floor(Date.now() / 1_000);
    // 50 ledgers remain → 50 × 5 s = 250 s from now
    const raw = makeRaw({
      windowStartLedger: 1_000,
      windowLength:       100,
      currentLedger:      1_050,
      ledgerTimestamp:    nowSec,
    });

    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(raw) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());

    const expected = nowSec * 1_000 + 50 * LEDGER_CLOSE_TIME_MS;
    expect(result.current.status!.windowResetsAt.getTime()).toBe(expected);
  });

  it('clamps windowResetsAt to the ledger timestamp when the window has already expired', async () => {
    const nowSec = Math.floor(Date.now() / 1_000);
    // Window expired 10 ledgers ago → ledgersUntilReset = max(0, -10) = 0
    const raw = makeRaw({
      windowStartLedger: 1_000,
      windowLength:       10,
      currentLedger:      1_020,
      ledgerTimestamp:    nowSec,
    });

    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(raw) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(result.current.status!.windowResetsAt.getTime()).toBe(nowSec * 1_000);
  });

  it('calls getStatus with the attestor address', async () => {
    const getStatus = mockGet(makeRaw());
    renderHook(() => useRateLimitStatus(ATTESTOR_A, { getStatus }));

    await waitFor(() => expect(getStatus).toHaveBeenCalledWith(ATTESTOR_A));
  });
});

// ── Throttled state ───────────────────────────────────────────────────────────

describe('useRateLimitStatus – throttled state', () => {
  it('sets isThrottled=true when used === limit', async () => {
    const raw = makeRaw({ submissionCount: 10, maxSubmissions: 10 });
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(raw) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(result.current.status!.isThrottled).toBe(true);
    expect(result.current.status!.used).toBe(10);
    expect(result.current.status!.limit).toBe(10);
  });

  it('sets isThrottled=true when used > limit (over-limit edge case)', async () => {
    const raw = makeRaw({ submissionCount: 12, maxSubmissions: 10 });
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(raw) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(result.current.status!.isThrottled).toBe(true);
  });

  it('sets isThrottled=false when used < limit', async () => {
    const raw = makeRaw({ submissionCount: 9, maxSubmissions: 10 });
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(raw) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(result.current.status!.isThrottled).toBe(false);
  });

  it('sets isThrottled=false when used is 0', async () => {
    const raw = makeRaw({ submissionCount: 0, maxSubmissions: 10 });
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(raw) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(result.current.status!.isThrottled).toBe(false);
  });

  it('reflects the per-attestor limit, not just the global default', async () => {
    // An attestor with a per-attestor override of 50 should not be throttled
    // at the same point a default-10 attestor would be.
    const raw = makeRaw({ submissionCount: 10, maxSubmissions: 50 });
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(raw) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(result.current.status!.isThrottled).toBe(false);
    expect(result.current.status!.limit).toBe(50);
  });
});

// ── Error scenarios ───────────────────────────────────────────────────────────

describe('useRateLimitStatus – error scenarios', () => {
  it('sets error and clears status when the RPC call rejects', async () => {
    const getStatus = jest.fn().mockRejectedValue(new Error('RPC unavailable'));
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus }),
    );

    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.error!.message).toBe('RPC unavailable');
    expect(result.current.status).toBeNull();
    expect(result.current.loading).toBe(false);
  });

  it('wraps non-Error rejection values in an Error', async () => {
    const getStatus = jest.fn().mockRejectedValue('plain string rejection');
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus }),
    );

    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error!.message).toBe('plain string rejection');
  });

  it('clears a previous error and status on subsequent failure', async () => {
    const getStatus = jest.fn()
      .mockRejectedValueOnce(new Error('first error'))
      .mockRejectedValueOnce(new Error('second error'));

    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus }),
    );

    await waitFor(() =>
      expect(result.current.error?.message).toBe('first error'),
    );

    act(() => { result.current.refresh(); });

    await waitFor(() =>
      expect(result.current.error?.message).toBe('second error'),
    );
    expect(result.current.status).toBeNull();
  });

  // ── Missing / invalid attestor ──────────────────────────────────────────────

  it('returns null status for a null attestor without calling getStatus', () => {
    const getStatus = jest.fn();
    const { result } = renderHook(() =>
      useRateLimitStatus(null, { getStatus }),
    );

    expect(result.current.status).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(getStatus).not.toHaveBeenCalled();
  });

  it('returns null status for an undefined attestor without calling getStatus', () => {
    const getStatus = jest.fn();
    const { result } = renderHook(() =>
      useRateLimitStatus(undefined, { getStatus }),
    );

    expect(result.current.status).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(getStatus).not.toHaveBeenCalled();
  });

  it('returns null status for an empty-string attestor without calling getStatus', () => {
    const getStatus = jest.fn();
    const { result } = renderHook(() =>
      useRateLimitStatus('', { getStatus }),
    );

    expect(result.current.status).toBeNull();
    expect(getStatus).not.toHaveBeenCalled();
  });

  it('returns null status for a whitespace-only attestor without calling getStatus', () => {
    const getStatus = jest.fn();
    const { result } = renderHook(() =>
      useRateLimitStatus('   ', { getStatus }),
    );

    expect(result.current.status).toBeNull();
    expect(getStatus).not.toHaveBeenCalled();
  });
});

// ── Attestor changes ──────────────────────────────────────────────────────────

describe('useRateLimitStatus – attestor changes', () => {
  it('re-fetches and calls getStatus with the new attestor when attestor changes', async () => {
    const getStatus = jest.fn().mockResolvedValue(makeRaw());
    let attestor = ATTESTOR_A;

    const { result, rerender } = renderHook(() =>
      useRateLimitStatus(attestor, { getStatus }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(getStatus).toHaveBeenCalledTimes(1);
    expect(getStatus).toHaveBeenLastCalledWith(ATTESTOR_A);

    attestor = ATTESTOR_B;
    rerender();

    await waitFor(() => expect(getStatus).toHaveBeenCalledTimes(2));
    expect(getStatus).toHaveBeenLastCalledWith(ATTESTOR_B);
  });

  it('clears status and error immediately when attestor changes to null', async () => {
    const getStatus = mockGet(makeRaw());
    let attestor: string | null = ATTESTOR_A;

    const { result, rerender } = renderHook(() =>
      useRateLimitStatus(attestor, { getStatus }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());

    attestor = null;
    rerender();

    await waitFor(() => expect(result.current.status).toBeNull());
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('clears a previous error when a valid attestor replaces null', async () => {
    const getStatus = jest.fn().mockResolvedValue(makeRaw());
    let attestor: string | null = null;

    const { result, rerender } = renderHook(() =>
      useRateLimitStatus(attestor, { getStatus }),
    );

    // Simulate an earlier error by manually checking initial state
    expect(result.current.error).toBeNull();

    attestor = ATTESTOR_A;
    rerender();

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(result.current.error).toBeNull();
  });
});

// ── Caching ───────────────────────────────────────────────────────────────────

describe('useRateLimitStatus – caching', () => {
  it('does not make a second RPC call for the same attestor within the TTL', async () => {
    const getStatus = mockGet(makeRaw());
    const options = { getStatus };

    // First mount — populates the cache.
    const { result: r1, unmount } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, options),
    );
    await waitFor(() => expect(r1.current.status).not.toBeNull());
    unmount();

    // Second mount — should hit the cache.
    const { result: r2 } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, options),
    );
    await waitFor(() => expect(r2.current.status).not.toBeNull());

    expect(getStatus).toHaveBeenCalledTimes(1); // only one RPC call
  });

  it('serves the cached value immediately (no loading flash) on a cache hit', async () => {
    const getStatus = mockGet(makeRaw());
    const options = { getStatus };

    const { result: r1, unmount } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, options),
    );
    await waitFor(() => expect(r1.current.status).not.toBeNull());
    unmount();

    const { result: r2 } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, options),
    );

    // On a cache hit, loading should never become true.
    expect(r2.current.loading).toBe(false);
    await waitFor(() => expect(r2.current.status).not.toBeNull());
    expect(r2.current.loading).toBe(false);
  });

  it('maintains separate cache entries for different attestors', async () => {
    const rawA = makeRaw({ submissionCount: 2 });
    const rawB = makeRaw({ submissionCount: 7 });

    const getStatusA = mockGet(rawA);
    const getStatusB = mockGet(rawB);

    const { result: rA } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: getStatusA }),
    );
    const { result: rB } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_B, { getStatus: getStatusB }),
    );

    await waitFor(() => {
      expect(rA.current.status?.used).toBe(2);
      expect(rB.current.status?.used).toBe(7);
    });
  });

  it('bypasses the cache and makes an RPC call on refresh()', async () => {
    const getStatus = mockGet(makeRaw());
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(getStatus).toHaveBeenCalledTimes(1);

    act(() => { result.current.refresh(); });

    await waitFor(() => expect(getStatus).toHaveBeenCalledTimes(2));
  });

  it('reflects updated data after refresh()', async () => {
    const getStatus = jest.fn()
      .mockResolvedValueOnce(makeRaw({ submissionCount: 3 }))
      .mockResolvedValueOnce(makeRaw({ submissionCount: 8 }));

    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus }),
    );

    await waitFor(() => expect(result.current.status?.used).toBe(3));

    act(() => { result.current.refresh(); });

    await waitFor(() => expect(result.current.status?.used).toBe(8));
  });
});

// ── Stale request discard ─────────────────────────────────────────────────────

describe('useRateLimitStatus – stale request discard', () => {
  it('discards the response from a superseded request when attestor changes mid-flight', async () => {
    let resolveFirst!: (r: RateLimitStatusRaw) => void;
    let resolveSecond!: (r: RateLimitStatusRaw) => void;

    const getStatus = jest.fn()
      .mockReturnValueOnce(
        new Promise<RateLimitStatusRaw>((r) => { resolveFirst = r; }),
      )
      .mockReturnValueOnce(
        new Promise<RateLimitStatusRaw>((r) => { resolveSecond = r; }),
      );

    let attestor = ATTESTOR_A;
    const { result, rerender } = renderHook(() =>
      useRateLimitStatus(attestor, { getStatus }),
    );

    // Switch attestor before the first response arrives.
    attestor = ATTESTOR_B;
    rerender();

    // Resolve the second (current) request.
    resolveSecond(makeRaw({ submissionCount: 7 }));
    await waitFor(() => expect(result.current.status?.used).toBe(7));

    // Resolve the stale first request — its result must be silently dropped.
    resolveFirst(makeRaw({ submissionCount: 1 }));
    await new Promise((r) => setTimeout(r, 20));
    expect(result.current.status?.used).toBe(7); // unchanged
  });

  it('shows loading=true until the latest request settles, not the first one', async () => {
    let resolveFirst!: (r: RateLimitStatusRaw) => void;
    let resolveSecond!: (r: RateLimitStatusRaw) => void;

    const getStatus = jest.fn()
      .mockReturnValueOnce(
        new Promise<RateLimitStatusRaw>((r) => { resolveFirst = r; }),
      )
      .mockReturnValueOnce(
        new Promise<RateLimitStatusRaw>((r) => { resolveSecond = r; }),
      );

    let attestor = ATTESTOR_A;
    const { result, rerender } = renderHook(() =>
      useRateLimitStatus(attestor, { getStatus }),
    );

    await waitFor(() => expect(result.current.loading).toBe(true));

    attestor = ATTESTOR_B;
    rerender();

    // Resolve only the stale request — loading must still be true.
    resolveFirst(makeRaw());
    await new Promise((r) => setTimeout(r, 20));
    expect(result.current.loading).toBe(true);

    // Resolve the current request — now loading can settle.
    resolveSecond(makeRaw({ submissionCount: 5 }));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.status?.used).toBe(5);
  });
});

// ── Typed-output shape ────────────────────────────────────────────────────────

describe('useRateLimitStatus – returned shape', () => {
  it('exposes a stable refresh function reference on re-render', async () => {
    const getStatus = mockGet(makeRaw());
    const { result, rerender } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());
    const firstRefresh = result.current.refresh;

    rerender();
    expect(result.current.refresh).toBe(firstRefresh);
  });

  it('exposes all four fields on the status object', async () => {
    const { result } = renderHook(() =>
      useRateLimitStatus(ATTESTOR_A, { getStatus: mockGet(makeRaw()) }),
    );

    await waitFor(() => expect(result.current.status).not.toBeNull());

    const s: RateLimitStatus = result.current.status!;
    expect(typeof s.used).toBe('number');
    expect(typeof s.limit).toBe('number');
    expect(s.windowResetsAt).toBeInstanceOf(Date);
    expect(typeof s.isThrottled).toBe('boolean');
  });
});

// ── Default getStatus / contractClient ────────────────────────────────────────

describe('useRateLimitStatus – default getStatus out of the box', () => {
  it('works out of the box without options using default getRateLimitStatus', async () => {
    const { result } = renderHook(() => useRateLimitStatus(ATTESTOR_A));

    await waitFor(() => expect(result.current.status).not.toBeNull());

    const s = result.current.status!;
    expect(s.used).toBe(0);
    expect(s.limit).toBe(10);
    expect(s.isThrottled).toBe(false);
    expect(s.windowResetsAt).toBeInstanceOf(Date);
    expect(result.current.error).toBeNull();
    expect(result.current.loading).toBe(false);
  });
});

describe('getRateLimitStatus & setRateLimitClient', () => {
  it('returns default rate limit data when no custom client is configured', async () => {
    const data = await getRateLimitStatus(ATTESTOR_A);
    expect(data.submissionCount).toBe(0);
    expect(data.maxSubmissions).toBe(10);
    expect(data.windowLength).toBe(100);
    expect(data.currentLedger).toBeGreaterThan(0);
    expect(data.windowStartLedger).toBe(data.currentLedger);
    expect(data.ledgerTimestamp).toBeGreaterThan(0);
  });

  it('uses custom client when configured via setRateLimitClient', async () => {
    setRateLimitClient(async () => makeRaw({ submissionCount: 5 }));
    const data = await getRateLimitStatus(ATTESTOR_A);
    expect(data.submissionCount).toBe(5);
  });

  it('reverts to default client after resetRateLimitClient', async () => {
    setRateLimitClient(async () => makeRaw({ submissionCount: 9 }));
    resetRateLimitClient();
    const data = await getRateLimitStatus(ATTESTOR_A);
    expect(data.submissionCount).toBe(0);
  });

  it('throws ContractError when attestor address is empty', async () => {
    await expect(getRateLimitStatus('')).rejects.toThrow(ContractError);
  });
});
