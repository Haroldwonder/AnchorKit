import { renderHook, act, waitFor } from '@testing-library/react';
import { useAnchorHealth, isValidAttestor, GetHealthScoreFn } from './useAnchorHealth';

// ── Fixtures ───────────────────────────────────────────────────────────────────

const ATTESTOR = 'G' + 'A'.repeat(55);
const CONTRACT = 'C' + 'A'.repeat(55);
const INTERVAL = 5_000;

function makeGetHealthScore(score = 85): jest.MockedFunction<GetHealthScoreFn> {
  return jest.fn().mockResolvedValue(score);
}

// ── isValidAttestor ────────────────────────────────────────────────────────────

describe('isValidAttestor', () => {
  test('accepts a valid ed25519 G-address', () => {
    expect(isValidAttestor(ATTESTOR)).toBe(true);
  });

  test('accepts a valid contract C-address', () => {
    expect(isValidAttestor(CONTRACT)).toBe(true);
  });

  test('rejects null', () => {
    expect(isValidAttestor(null)).toBe(false);
  });

  test('rejects undefined', () => {
    expect(isValidAttestor(undefined)).toBe(false);
  });

  test('rejects empty string', () => {
    expect(isValidAttestor('')).toBe(false);
  });

  test('rejects wrong starting character', () => {
    expect(isValidAttestor('A' + 'A'.repeat(55))).toBe(false);
  });

  test('rejects address that is too short', () => {
    expect(isValidAttestor('G' + 'A'.repeat(54))).toBe(false);
  });

  test('rejects address that is too long', () => {
    expect(isValidAttestor('G' + 'A'.repeat(56))).toBe(false);
  });

  test('rejects address with invalid base32 characters', () => {
    expect(isValidAttestor('G' + '0'.repeat(55))).toBe(false); // '0' invalid in base32
  });
});

// ── Initial state ──────────────────────────────────────────────────────────────

describe('useAnchorHealth — initial state', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => { jest.runOnlyPendingTimers(); jest.useRealTimers(); });

  test('null attestor: all fields are idle (no loading, no score)', () => {
    const { result } = renderHook(() =>
      useAnchorHealth(null, INTERVAL, makeGetHealthScore()),
    );
    expect(result.current.score).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.lastUpdated).toBeNull();
  });

  test('undefined attestor: all fields are idle', () => {
    const { result } = renderHook(() =>
      useAnchorHealth(undefined, INTERVAL, makeGetHealthScore()),
    );
    expect(result.current.score).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.lastUpdated).toBeNull();
  });

  test('invalid attestor: all fields are idle', () => {
    const { result } = renderHook(() =>
      useAnchorHealth('not-an-address', INTERVAL, makeGetHealthScore()),
    );
    expect(result.current.score).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.lastUpdated).toBeNull();
  });

  test('valid attestor: loading is true before first poll completes', () => {
    // Hold the mock unresolved so the initial fetch is still in-flight.
    const mockFn = jest.fn().mockReturnValue(new Promise(() => {}));
    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );
    expect(result.current.loading).toBe(true);
    expect(result.current.score).toBeNull();
    expect(result.current.error).toBeNull();
  });

  test('no polling starts when attestor is absent', () => {
    const mockFn = makeGetHealthScore();
    renderHook(() => useAnchorHealth(null, INTERVAL, mockFn));
    act(() => jest.advanceTimersByTime(INTERVAL * 5));
    expect(mockFn).not.toHaveBeenCalled();
  });
});

// ── Successful polling ─────────────────────────────────────────────────────────

describe('useAnchorHealth — successful polling', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => { jest.runOnlyPendingTimers(); jest.useRealTimers(); });

  test('first poll resolves: loading→false, score set, lastUpdated set', async () => {
    const mockFn = makeGetHealthScore(72);
    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    expect(result.current.loading).toBe(true);

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.score).toBe(72);
    expect(result.current.error).toBeNull();
    expect(result.current.lastUpdated).toBeInstanceOf(Date);
  });

  test('getHealthScore is called with the attestor address', async () => {
    const mockFn = makeGetHealthScore(50);
    renderHook(() => useAnchorHealth(ATTESTOR, INTERVAL, mockFn));

    await act(async () => { await Promise.resolve(); });

    expect(mockFn).toHaveBeenCalledTimes(1);
    expect(mockFn).toHaveBeenCalledWith(ATTESTOR);
  });

  test('getHealthScore is called with a contract C-address', async () => {
    const mockFn = makeGetHealthScore(90);
    renderHook(() => useAnchorHealth(CONTRACT, INTERVAL, mockFn));

    await act(async () => { await Promise.resolve(); });

    expect(mockFn).toHaveBeenCalledWith(CONTRACT);
  });

  test('score 0 is stored correctly (not treated as falsy)', async () => {
    const mockFn = makeGetHealthScore(0);
    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    await act(async () => { await Promise.resolve(); });

    expect(result.current.score).toBe(0);
    expect(result.current.loading).toBe(false);
  });

  test('score 100 is stored correctly', async () => {
    const mockFn = makeGetHealthScore(100);
    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    await act(async () => { await Promise.resolve(); });

    expect(result.current.score).toBe(100);
  });
});

// ── Score updates ──────────────────────────────────────────────────────────────

describe('useAnchorHealth — score updates on subsequent polls', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => { jest.runOnlyPendingTimers(); jest.useRealTimers(); });

  test('second poll updates the score', async () => {
    const mockFn = jest.fn()
      .mockResolvedValueOnce(60)
      .mockResolvedValueOnce(75);

    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    await act(async () => { await Promise.resolve(); });
    expect(result.current.score).toBe(60);

    await act(async () => {
      jest.advanceTimersByTime(INTERVAL);
      await Promise.resolve();
    });

    expect(result.current.score).toBe(75);
    expect(mockFn).toHaveBeenCalledTimes(2);
  });

  test('lastUpdated advances with each successful poll', async () => {
    const times: Date[] = [];
    const mockFn = jest.fn().mockImplementation(() => {
      times.push(new Date());
      return Promise.resolve(50);
    });

    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    await act(async () => { await Promise.resolve(); });
    const first = result.current.lastUpdated!;

    // Advance real time inside fake-timer world by making the mock capture
    // a later Date on the second call.
    jest.setSystemTime(jest.getRealSystemTime() + INTERVAL);

    await act(async () => {
      jest.advanceTimersByTime(INTERVAL);
      await Promise.resolve();
    });

    const second = result.current.lastUpdated!;
    expect(second.getTime()).toBeGreaterThanOrEqual(first.getTime());
  });

  test('loading stays false during background polls after initial load', async () => {
    const mockFn = makeGetHealthScore(80);
    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    await act(async () => { await Promise.resolve(); });
    expect(result.current.loading).toBe(false);

    await act(async () => {
      jest.advanceTimersByTime(INTERVAL);
      await Promise.resolve();
    });

    expect(result.current.loading).toBe(false);
  });

  test('polls fire at the requested interval', async () => {
    const mockFn = makeGetHealthScore(88);
    renderHook(() => useAnchorHealth(ATTESTOR, INTERVAL, mockFn));

    // Flush initial poll
    await act(async () => { await Promise.resolve(); });
    expect(mockFn).toHaveBeenCalledTimes(1);

    // Three more ticks
    for (let i = 0; i < 3; i++) {
      await act(async () => {
        jest.advanceTimersByTime(INTERVAL);
        await Promise.resolve();
      });
    }

    expect(mockFn).toHaveBeenCalledTimes(4);
  });
});

// ── Error handling ─────────────────────────────────────────────────────────────

describe('useAnchorHealth — error handling', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => { jest.runOnlyPendingTimers(); jest.useRealTimers(); });

  test('initial poll failure: error set, loading false, score null', async () => {
    const mockFn = jest.fn().mockRejectedValue(new Error('Contract call failed'));
    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    expect(result.current.loading).toBe(true);

    await act(async () => { await Promise.resolve(); });

    expect(result.current.loading).toBe(false);
    expect(result.current.score).toBeNull();
    expect(result.current.error).toBe('Contract call failed');
    expect(result.current.lastUpdated).toBeNull();
  });

  test('non-Error rejection produces a generic fallback message', async () => {
    const mockFn = jest.fn().mockRejectedValue('string error');
    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    await act(async () => { await Promise.resolve(); });

    expect(result.current.error).toBe(
      'Failed to fetch health score. Please try again.',
    );
  });

  test('subsequent poll failure keeps the last known score alongside the error', async () => {
    const mockFn = jest.fn()
      .mockResolvedValueOnce(65)
      .mockRejectedValueOnce(new Error('Network timeout'));

    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    await act(async () => { await Promise.resolve(); });
    expect(result.current.score).toBe(65);
    expect(result.current.error).toBeNull();

    await act(async () => {
      jest.advanceTimersByTime(INTERVAL);
      await Promise.resolve();
    });

    expect(result.current.score).toBe(65);       // stale score preserved
    expect(result.current.error).toBe('Network timeout');
  });

  test('successful poll after an error clears the error', async () => {
    const mockFn = jest.fn()
      .mockRejectedValueOnce(new Error('Timeout'))
      .mockResolvedValueOnce(90);

    const { result } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    await act(async () => { await Promise.resolve(); });
    expect(result.current.error).toBe('Timeout');

    await act(async () => {
      jest.advanceTimersByTime(INTERVAL);
      await Promise.resolve();
    });

    expect(result.current.score).toBe(90);
    expect(result.current.error).toBeNull();
  });

  test('error is cleared when attestor changes to a new valid address', async () => {
    const mockFn = jest.fn()
      .mockRejectedValueOnce(new Error('Bad attestor'))
      .mockResolvedValue(55);

    const { result, rerender } = renderHook(
      ({ a }: { a: string }) => useAnchorHealth(a, INTERVAL, mockFn),
      { initialProps: { a: ATTESTOR } },
    );

    await act(async () => { await Promise.resolve(); });
    expect(result.current.error).toBe('Bad attestor');

    const OTHER = 'G' + 'B'.repeat(55);
    rerender({ a: OTHER });
    expect(result.current.error).toBeNull();
    expect(result.current.score).toBeNull();
  });
});

// ── Interval changes ───────────────────────────────────────────────────────────

describe('useAnchorHealth — interval changes', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => { jest.runOnlyPendingTimers(); jest.useRealTimers(); });

  test('shortening the interval causes polling to fire more frequently', async () => {
    const mockFn = makeGetHealthScore(77);
    const { rerender } = renderHook(
      ({ iv }: { iv: number }) => useAnchorHealth(ATTESTOR, iv, mockFn),
      { initialProps: { iv: 10_000 } },
    );

    // Flush initial poll
    await act(async () => { await Promise.resolve(); });

    // Switch to faster interval — this triggers an immediate re-poll too
    rerender({ iv: 1_000 });
    await act(async () => { await Promise.resolve(); });
    const callsAfterRerender = mockFn.mock.calls.length; // initial + immediate re-poll

    // Advance 3 separate ticks, flushing async promises after each so
    // in-flight deduplication does not suppress subsequent ticks.
    for (let i = 0; i < 3; i++) {
      await act(async () => {
        jest.advanceTimersByTime(1_000);
        await Promise.resolve();
      });
    }

    expect(mockFn.mock.calls.length).toBe(callsAfterRerender + 3);
  });

  test('changing interval restarts the timer (old interval no longer fires)', async () => {
    const mockFn = makeGetHealthScore(55);
    const { rerender } = renderHook(
      ({ iv }: { iv: number }) => useAnchorHealth(ATTESTOR, iv, mockFn),
      { initialProps: { iv: 10_000 } },
    );

    await act(async () => { await Promise.resolve(); });

    // Switch to a much longer interval; flush the immediate re-poll that
    // fires when the effect restarts.
    rerender({ iv: 30_000 });
    await act(async () => { await Promise.resolve(); });
    const callsAfterRerender = mockFn.mock.calls.length;

    // Advance 10 000 ms — the OLD 10 s timer would have fired here, but it
    // was cancelled.  The new 30 s timer has not fired yet.
    await act(async () => {
      jest.advanceTimersByTime(10_000);
      await Promise.resolve();
    });

    expect(mockFn.mock.calls.length).toBe(callsAfterRerender);
  });

  test('existing score is preserved when only the interval changes', async () => {
    const mockFn = makeGetHealthScore(63);
    const { result, rerender } = renderHook(
      ({ iv }: { iv: number }) => useAnchorHealth(ATTESTOR, iv, mockFn),
      { initialProps: { iv: 5_000 } },
    );

    await act(async () => { await Promise.resolve(); });
    expect(result.current.score).toBe(63);

    rerender({ iv: 10_000 });

    // Score should be preserved immediately after interval change
    expect(result.current.score).toBe(63);
    expect(result.current.loading).toBe(false);
  });
});

// ── Attestor changes ───────────────────────────────────────────────────────────

describe('useAnchorHealth — attestor changes', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => { jest.runOnlyPendingTimers(); jest.useRealTimers(); });

  test('changing attestor resets score to null and sets loading true', async () => {
    const mockFn = makeGetHealthScore(70);
    const { result, rerender } = renderHook(
      ({ a }: { a: string }) => useAnchorHealth(a, INTERVAL, mockFn),
      { initialProps: { a: ATTESTOR } },
    );

    await act(async () => { await Promise.resolve(); });
    expect(result.current.score).toBe(70);

    const OTHER = 'G' + 'B'.repeat(55);
    rerender({ a: OTHER });

    expect(result.current.score).toBeNull();
    expect(result.current.loading).toBe(true);
  });

  test('new attestor is used in subsequent polls', async () => {
    const mockFn = makeGetHealthScore(80);
    const OTHER = 'G' + 'B'.repeat(55);

    const { rerender } = renderHook(
      ({ a }: { a: string }) => useAnchorHealth(a, INTERVAL, mockFn),
      { initialProps: { a: ATTESTOR } },
    );

    await act(async () => { await Promise.resolve(); });
    expect(mockFn).toHaveBeenLastCalledWith(ATTESTOR);

    rerender({ a: OTHER });
    await act(async () => { await Promise.resolve(); });
    expect(mockFn).toHaveBeenLastCalledWith(OTHER);
  });

  test('switching to null attestor stops polling and resets state', async () => {
    const mockFn = makeGetHealthScore(90);
    const { result, rerender } = renderHook(
      ({ a }: { a: string | null }) => useAnchorHealth(a, INTERVAL, mockFn),
      { initialProps: { a: ATTESTOR as string | null } },
    );

    await act(async () => { await Promise.resolve(); });
    expect(result.current.score).toBe(90);

    rerender({ a: null });

    expect(result.current.score).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();

    const callsBefore = mockFn.mock.calls.length;
    act(() => jest.advanceTimersByTime(INTERVAL * 3));
    expect(mockFn.mock.calls.length).toBe(callsBefore); // no new calls
  });

  test('switching from invalid to valid attestor starts polling', async () => {
    const mockFn = makeGetHealthScore(45);
    const { result, rerender } = renderHook(
      ({ a }: { a: string | null }) => useAnchorHealth(a, INTERVAL, mockFn),
      { initialProps: { a: null as string | null } },
    );

    expect(result.current.loading).toBe(false);
    expect(mockFn).not.toHaveBeenCalled();

    rerender({ a: ATTESTOR });
    expect(result.current.loading).toBe(true);

    await act(async () => { await Promise.resolve(); });
    expect(result.current.score).toBe(45);
  });
});

// ── Cleanup behaviour ──────────────────────────────────────────────────────────

describe('useAnchorHealth — cleanup behaviour', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => { jest.runOnlyPendingTimers(); jest.useRealTimers(); });

  test('unmount cancels the polling interval', async () => {
    const mockFn = makeGetHealthScore(50);
    const { unmount } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    await act(async () => { await Promise.resolve(); });
    const callsAtUnmount = mockFn.mock.calls.length;

    unmount();

    act(() => jest.advanceTimersByTime(INTERVAL * 5));
    expect(mockFn.mock.calls.length).toBe(callsAtUnmount); // no new calls
  });

  test('in-flight request on unmount does not update state', async () => {
    let resolveHealth!: (v: number) => void;
    const mockFn = jest.fn().mockReturnValue(
      new Promise<number>((r) => { resolveHealth = r; }),
    );

    const { result, unmount } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, mockFn),
    );

    // loading=true, poll in-flight (not yet resolved)
    expect(result.current.loading).toBe(true);

    // Unmount while poll is still pending
    act(() => unmount());

    // Now resolve the in-flight request — should be a no-op
    await act(async () => {
      resolveHealth(99);
      await Promise.resolve();
    });

    // State should not have been updated after unmount
    expect(result.current.score).toBeNull();
    expect(result.current.loading).toBe(true); // last rendered value — unchanged
  });

  test('no interval is leaked: clearing all timers after unmount shows no pending timers', () => {
    const { unmount } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, makeGetHealthScore()),
    );
    unmount();
    // jest.runOnlyPendingTimers() should not throw or call the mock
    expect(() => act(() => jest.runOnlyPendingTimers())).not.toThrow();
  });
});

// ── Overlapping request prevention ────────────────────────────────────────────

describe('useAnchorHealth — overlapping request prevention', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => { jest.runOnlyPendingTimers(); jest.useRealTimers(); });

  test('timer firing while a poll is in-flight is silently skipped', async () => {
    // First call hangs; second call resolves immediately.
    let resolveFirst!: (v: number) => void;
    const mockFn = jest.fn()
      .mockReturnValueOnce(new Promise<number>((r) => { resolveFirst = r; }))
      .mockResolvedValue(42);

    renderHook(() => useAnchorHealth(ATTESTOR, INTERVAL, mockFn));

    // Initial poll is in-flight (unresolved).
    expect(mockFn).toHaveBeenCalledTimes(1);

    // Timer fires while first poll is still pending — should be skipped.
    act(() => jest.advanceTimersByTime(INTERVAL));
    expect(mockFn).toHaveBeenCalledTimes(1); // no second call yet

    // Resolve first poll, then let the next tick run normally.
    await act(async () => {
      resolveFirst(37);
      await Promise.resolve();
    });

    await act(async () => {
      jest.advanceTimersByTime(INTERVAL);
      await Promise.resolve();
    });

    expect(mockFn).toHaveBeenCalledTimes(2); // second call allowed once first finished
  });
});

// ── Adapter reference stability ────────────────────────────────────────────────

describe('useAnchorHealth — adapter reference stability', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => { jest.runOnlyPendingTimers(); jest.useRealTimers(); });

  test('passing a new inline function reference on each render does not restart the timer', async () => {
    let renderCount = 0;
    const innerFn = makeGetHealthScore(33);

    const { result, rerender } = renderHook(() => {
      renderCount++;
      // Fresh arrow function every render — timer must not restart
      return useAnchorHealth(ATTESTOR, INTERVAL, () => innerFn(ATTESTOR));
    });

    await act(async () => { await Promise.resolve(); });
    const callsAfterInit = innerFn.mock.calls.length;

    // Force extra renders without changing deps
    rerender();
    rerender();

    // Advancing time by less than one interval: no extra calls expected
    act(() => jest.advanceTimersByTime(INTERVAL - 1));
    expect(innerFn.mock.calls.length).toBe(callsAfterInit);
  });

  test('the most recent getHealthScore function is used on each tick', async () => {
    const v1 = jest.fn().mockResolvedValue(10);
    const v2 = jest.fn().mockResolvedValue(20);
    let currentFn: GetHealthScoreFn = v1;

    const { result, rerender } = renderHook(() =>
      useAnchorHealth(ATTESTOR, INTERVAL, currentFn),
    );

    await act(async () => { await Promise.resolve(); });
    expect(result.current.score).toBe(10);

    // Swap the implementation
    currentFn = v2;
    rerender();

    await act(async () => {
      jest.advanceTimersByTime(INTERVAL);
      await Promise.resolve();
    });

    expect(result.current.score).toBe(20);
    expect(v2).toHaveBeenCalledTimes(1);
  });
});
