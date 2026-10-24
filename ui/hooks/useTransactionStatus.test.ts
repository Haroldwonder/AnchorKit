import { renderHook, act, waitFor } from '@testing-library/react';
import { useTransactionStatus } from './useTransactionStatus';
import { TransactionStateTracker, isTerminalStatus } from './TransactionStateTracker';
import { TxStatus } from '../components/TransactionTimeline';

// ─── TransactionStateTracker unit tests ───────────────────────────────────────

describe('TransactionStateTracker', () => {
  test('records the first transition from null', async () => {
    const fetch = jest.fn().mockResolvedValue('pending' as TxStatus);
    const tracker = new TransactionStateTracker(fetch);

    const snap = await tracker.poll('tx1');

    expect(snap.status).toBe('pending');
    expect(snap.transitions).toHaveLength(1);
    expect(snap.transitions[0]).toMatchObject({ from: null, to: 'pending' });
    expect(typeof snap.transitions[0].at).toBe('number');
  });

  test('records a transition when status changes', async () => {
    const fetch = jest
      .fn()
      .mockResolvedValueOnce('pending' as TxStatus)
      .mockResolvedValueOnce('processing' as TxStatus);
    const tracker = new TransactionStateTracker(fetch);

    await tracker.poll('tx1');
    const snap = await tracker.poll('tx1');

    expect(snap.status).toBe('processing');
    expect(snap.transitions).toHaveLength(2);
    expect(snap.transitions[1]).toMatchObject({ from: 'pending', to: 'processing' });
  });

  test('does not add a transition when status is unchanged', async () => {
    const fetch = jest.fn().mockResolvedValue('pending' as TxStatus);
    const tracker = new TransactionStateTracker(fetch);

    await tracker.poll('tx1');
    const snap = await tracker.poll('tx1');

    expect(snap.transitions).toHaveLength(1);
  });

  test('getSnapshot throws before first poll', () => {
    const tracker = new TransactionStateTracker(jest.fn());
    expect(() => tracker.getSnapshot()).toThrow('No status polled yet');
  });

  test('reset clears status and transitions', async () => {
    const fetch = jest.fn().mockResolvedValue('processing' as TxStatus);
    const tracker = new TransactionStateTracker(fetch);

    await tracker.poll('tx1');
    tracker.reset();

    expect(() => tracker.getSnapshot()).toThrow();
  });

  test.each<[TxStatus, boolean]>([
    ['initiated', false],
    ['pending', false],
    ['processing', false],
    ['completed', true],
    ['failed', true],
    ['refunded', true],
  ])('isTerminalStatus(%s) === %s', (status, expected) => {
    expect(isTerminalStatus(status)).toBe(expected);
  });
});

// ─── useTransactionStatus hook tests ─────────────────────────────────────────

describe('useTransactionStatus', () => {
  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
    jest.clearAllMocks();
  });

  // ── Pending state ────────────────────────────────────────────────────────

  test('returns null status before first poll resolves', () => {
    const fetchStatus = jest.fn(() => new Promise<TxStatus>(() => {})); // never resolves
    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus })
    );

    expect(result.current.status).toBeNull();
    expect(result.current.transitions).toEqual([]);
    expect(result.current.isTerminal).toBe(false);
    expect(result.current.error).toBeNull();
  });

  test('sets status after first poll resolves', async () => {
    const fetchStatus = jest.fn().mockResolvedValue('pending' as TxStatus);

    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus })
    );

    await waitFor(() => expect(result.current.status).toBe('pending'));
    expect(result.current.isTerminal).toBe(false);
    expect(result.current.transitions).toHaveLength(1);
    expect(result.current.transitions[0]).toMatchObject({ from: null, to: 'pending' });
  });

  // ── Transitioning ────────────────────────────────────────────────────────

  test('records transitions as status advances', async () => {
    const fetchStatus = jest
      .fn()
      .mockResolvedValueOnce('pending' as TxStatus)
      .mockResolvedValueOnce('processing' as TxStatus)
      .mockResolvedValue('completed' as TxStatus);

    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, interval: 100 })
    );

    await waitFor(() => expect(result.current.status).toBe('pending'));

    act(() => { jest.advanceTimersByTime(100); });
    await waitFor(() => expect(result.current.status).toBe('processing'));

    act(() => { jest.advanceTimersByTime(100); });
    await waitFor(() => expect(result.current.status).toBe('completed'));

    expect(result.current.transitions).toHaveLength(3);
    expect(result.current.transitions.map(t => t.to)).toEqual([
      'pending', 'processing', 'completed',
    ]);
  });

  // ── Terminal state ───────────────────────────────────────────────────────

  test('stops polling when completed is reached', async () => {
    const fetchStatus = jest.fn().mockResolvedValue('completed' as TxStatus);

    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, interval: 100 })
    );

    await waitFor(() => expect(result.current.isTerminal).toBe(true));
    expect(result.current.status).toBe('completed');

    const callsBefore = fetchStatus.mock.calls.length;
    act(() => { jest.advanceTimersByTime(500); });
    expect(fetchStatus.mock.calls.length).toBe(callsBefore);
  });

  test('stops polling when failed is reached', async () => {
    const fetchStatus = jest.fn().mockResolvedValue('failed' as TxStatus);

    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, interval: 100 })
    );

    await waitFor(() => expect(result.current.isTerminal).toBe(true));
    expect(result.current.status).toBe('failed');

    const callsBefore = fetchStatus.mock.calls.length;
    act(() => { jest.advanceTimersByTime(500); });
    expect(fetchStatus.mock.calls.length).toBe(callsBefore);
  });

  test('stops polling when refunded is reached', async () => {
    const fetchStatus = jest.fn().mockResolvedValue('refunded' as TxStatus);

    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, interval: 100 })
    );

    await waitFor(() => expect(result.current.isTerminal).toBe(true));
    expect(result.current.status).toBe('refunded');

    const callsBefore = fetchStatus.mock.calls.length;
    act(() => { jest.advanceTimersByTime(500); });
    expect(fetchStatus.mock.calls.length).toBe(callsBefore);
  });

  // ── Error scenarios ──────────────────────────────────────────────────────

  test('sets error when fetchStatus rejects', async () => {
    const fetchStatus = jest.fn().mockRejectedValue(new Error('Network error'));

    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, interval: 100 })
    );

    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.error?.message).toBe('Network error');
    expect(result.current.status).toBeNull();
  });

  test('retries after an error', async () => {
    const fetchStatus = jest
      .fn()
      .mockRejectedValueOnce(new Error('timeout'))
      .mockResolvedValue('processing' as TxStatus);

    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, interval: 100 })
    );

    await waitFor(() => expect(result.current.error).not.toBeNull());

    act(() => { jest.advanceTimersByTime(100); });
    await waitFor(() => expect(result.current.status).toBe('processing'));
    expect(result.current.error).toBeNull();
  });

  test('wraps non-Error rejections in an Error', async () => {
    const fetchStatus = jest.fn().mockRejectedValue('string error');

    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, interval: 100 })
    );

    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe('string error');
  });

  // ── Cleanup on unmount ───────────────────────────────────────────────────

  test('clears pending timer and does not update state after unmount', async () => {
    const fetchStatus = jest
      .fn()
      .mockResolvedValue('pending' as TxStatus);

    const { result, unmount } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, interval: 100 })
    );

    await waitFor(() => expect(result.current.status).toBe('pending'));

    unmount();

    const callsBefore = fetchStatus.mock.calls.length;
    act(() => { jest.advanceTimersByTime(300); });
    expect(fetchStatus.mock.calls.length).toBe(callsBefore);
  });

  // ── Options: enabled flag ────────────────────────────────────────────────

  test('does not poll when enabled is false', () => {
    const fetchStatus = jest.fn().mockResolvedValue('pending' as TxStatus);

    const { result } = renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, enabled: false })
    );

    act(() => { jest.advanceTimersByTime(1000); });

    expect(fetchStatus).not.toHaveBeenCalled();
    expect(result.current.status).toBeNull();
  });

  test('starts polling when enabled transitions from false to true', async () => {
    const fetchStatus = jest.fn().mockResolvedValue('processing' as TxStatus);

    const { result, rerender } = renderHook(
      ({ enabled }: { enabled: boolean }) =>
        useTransactionStatus('tx1', { fetchStatus, enabled }),
      { initialProps: { enabled: false } }
    );

    expect(fetchStatus).not.toHaveBeenCalled();

    rerender({ enabled: true });

    await waitFor(() => expect(result.current.status).toBe('processing'));
  });

  test('stops polling when enabled transitions from true to false', async () => {
    const fetchStatus = jest.fn().mockResolvedValue('pending' as TxStatus);

    const { rerender } = renderHook(
      ({ enabled }: { enabled: boolean }) =>
        useTransactionStatus('tx1', { fetchStatus, enabled, interval: 100 }),
      { initialProps: { enabled: true } }
    );

    await waitFor(() => expect(fetchStatus).toHaveBeenCalled());

    rerender({ enabled: false });

    const callsBefore = fetchStatus.mock.calls.length;
    act(() => { jest.advanceTimersByTime(300); });
    expect(fetchStatus.mock.calls.length).toBe(callsBefore);
  });

  // ── Options: interval ────────────────────────────────────────────────────

  test('respects custom polling interval', async () => {
    const fetchStatus = jest.fn().mockResolvedValue('pending' as TxStatus);

    renderHook(() =>
      useTransactionStatus('tx1', { fetchStatus, interval: 1000 })
    );

    await waitFor(() => expect(fetchStatus).toHaveBeenCalledTimes(1));

    act(() => { jest.advanceTimersByTime(999); });
    expect(fetchStatus).toHaveBeenCalledTimes(1);

    act(() => { jest.advanceTimersByTime(1); });
    await waitFor(() => expect(fetchStatus).toHaveBeenCalledTimes(2));
  });

  // ── Invalid / null txId ──────────────────────────────────────────────────

  test('does not poll when txId is null', () => {
    const fetchStatus = jest.fn();

    renderHook(() =>
      useTransactionStatus(null, { fetchStatus })
    );

    act(() => { jest.advanceTimersByTime(1000); });
    expect(fetchStatus).not.toHaveBeenCalled();
  });

  test('does not poll when txId is undefined', () => {
    const fetchStatus = jest.fn();

    renderHook(() =>
      useTransactionStatus(undefined, { fetchStatus })
    );

    act(() => { jest.advanceTimersByTime(1000); });
    expect(fetchStatus).not.toHaveBeenCalled();
  });

  test('resets state when txId changes', async () => {
    const fetchStatus = jest
      .fn()
      .mockResolvedValueOnce('completed' as TxStatus)
      .mockResolvedValue('pending' as TxStatus);

    const { result, rerender } = renderHook(
      ({ txId }: { txId: string }) =>
        useTransactionStatus(txId, { fetchStatus, interval: 100 }),
      { initialProps: { txId: 'tx1' } }
    );

    await waitFor(() => expect(result.current.isTerminal).toBe(true));

    rerender({ txId: 'tx2' });

    await waitFor(() => expect(result.current.status).toBe('pending'));
    expect(result.current.isTerminal).toBe(false);
    expect(result.current.transitions[0]).toMatchObject({ from: null, to: 'pending' });
  });

  // ── Inline fetchStatus stability ─────────────────────────────────────────

  test('does not restart polling when fetchStatus reference changes', async () => {
    let callCount = 0;
    const fetchStatus = jest.fn(async () => {
      callCount++;
      return 'pending' as TxStatus;
    });

    const { rerender } = renderHook(
      ({ fn }: { fn: typeof fetchStatus }) =>
        useTransactionStatus('tx1', { fetchStatus: fn, interval: 500 }),
      { initialProps: { fn: fetchStatus } }
    );

    await waitFor(() => expect(callCount).toBeGreaterThan(0));
    const snapshot = callCount;

    // Pass a new function reference — should NOT restart the effect / reset transitions
    const fetchStatus2 = jest.fn(async () => 'pending' as TxStatus);
    rerender({ fn: fetchStatus2 });

    act(() => { jest.advanceTimersByTime(0); });
    // The effect should not have restarted (no extra calls from a fresh poll)
    expect(callCount).toBe(snapshot);
  });
  test('exposes retry exhaustion separately from a terminal transaction status', async () => {
    const fetchStatus = jest.fn().mockRejectedValue(new Error('unavailable'));
    const { result } = renderHook(() =>
      useTransactionStatus('tx1', {
        fetchStatus,
        interval: 100,
        maxRetries: 2,
      })
    );

    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.isPolling).toBe(true);
    expect(result.current.retriesExhausted).toBe(false);

    act(() => { jest.advanceTimersByTime(100); });
    await waitFor(() => expect(result.current.retriesExhausted).toBe(true));
    expect(result.current.isPolling).toBe(false);
    expect(result.current.isTerminal).toBe(false);
    expect(fetchStatus).toHaveBeenCalledTimes(2);
  });

});
