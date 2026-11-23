import { renderHook, act, waitFor } from '@testing-library/react';
import { useSep10Auth, Sep10AuthAdapters } from './useSep10Auth';

// ── Fixtures ──────────────────────────────────────────────────────────────────

const DOMAIN = 'testanchor.stellar.org';
const ADDRESS = 'G' + 'A'.repeat(55);
const CHALLENGE_XDR = 'AAAAAGL8HQv_challenge_base64==';
const SIGNED_XDR = 'AAAAAGL8HQv_signed_base64==';
const JWT = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0In0.signature';

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeAdapters(overrides?: Partial<Sep10AuthAdapters>): Sep10AuthAdapters {
  return {
    fetchChallenge: jest.fn().mockResolvedValue(CHALLENGE_XDR),
    signChallenge: jest.fn().mockResolvedValue(SIGNED_XDR),
    submitChallenge: jest.fn().mockResolvedValue(JWT),
    ...overrides,
  };
}

// ── Successful authentication ─────────────────────────────────────────────────

describe('useSep10Auth — successful authentication', () => {
  test('initial state: not authenticated, not loading, no error, no token', () => {
    const { result } = renderHook(() => useSep10Auth(DOMAIN, makeAdapters()));
    expect(result.current.token).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.isAuthenticating).toBe(false);
    expect(result.current.error).toBeNull();
  });

  test('authenticate() runs full flow: fetchChallenge → signChallenge → submitChallenge', async () => {
    const adapters = makeAdapters();
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    expect(adapters.fetchChallenge).toHaveBeenCalledTimes(1);
    expect(adapters.fetchChallenge).toHaveBeenCalledWith(DOMAIN, ADDRESS);
    expect(adapters.signChallenge).toHaveBeenCalledTimes(1);
    expect(adapters.signChallenge).toHaveBeenCalledWith(CHALLENGE_XDR);
    expect(adapters.submitChallenge).toHaveBeenCalledTimes(1);
    expect(adapters.submitChallenge).toHaveBeenCalledWith(DOMAIN, SIGNED_XDR);
  });

  test('token is set after successful authenticate()', async () => {
    const { result } = renderHook(() => useSep10Auth(DOMAIN, makeAdapters()));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    expect(result.current.token).toBe(JWT);
    expect(result.current.isAuthenticated).toBe(true);
    expect(result.current.error).toBeNull();
  });

  test('isAuthenticating is true during the flow, false after', async () => {
    let resolveChallenge!: (v: string) => void;
    const slowChallenge = new Promise<string>(r => { resolveChallenge = r; });
    const adapters = makeAdapters({ fetchChallenge: jest.fn().mockReturnValue(slowChallenge) });

    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    // Start authenticate (don't await yet)
    act(() => { result.current.authenticate(ADDRESS); });

    await waitFor(() => expect(result.current.isAuthenticating).toBe(true));

    act(() => { resolveChallenge(CHALLENGE_XDR); });

    await waitFor(() => expect(result.current.isAuthenticating).toBe(false));
    expect(result.current.token).toBe(JWT);
  });

  test('token persists in state after authenticate() resolves', async () => {
    const { result } = renderHook(() => useSep10Auth(DOMAIN, makeAdapters()));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    // Simulate a re-render (ref should still have the token)
    expect(result.current.token).toBe(JWT);
    expect(result.current.isAuthenticated).toBe(true);
  });

  test('passes the correct domain from anchorDomain parameter', async () => {
    const adapters = makeAdapters();
    const { result } = renderHook(() => useSep10Auth('custom.anchor.com', adapters));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    expect(adapters.fetchChallenge).toHaveBeenCalledWith('custom.anchor.com', ADDRESS);
    expect(adapters.submitChallenge).toHaveBeenCalledWith('custom.anchor.com', SIGNED_XDR);
  });

  test('uses the latest domain when anchorDomain changes between renders', async () => {
    const adapters = makeAdapters();
    const { result, rerender } = renderHook(
      ({ domain }: { domain: string }) => useSep10Auth(domain, adapters),
      { initialProps: { domain: 'old.anchor.com' } }
    );

    rerender({ domain: 'new.anchor.com' });

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    expect(adapters.fetchChallenge).toHaveBeenCalledWith('new.anchor.com', ADDRESS);
    expect(adapters.submitChallenge).toHaveBeenCalledWith('new.anchor.com', SIGNED_XDR);
  });
});

// ── Failed authentication ─────────────────────────────────────────────────────

describe('useSep10Auth — failed authentication', () => {
  test('sets error when fetchChallenge rejects', async () => {
    const adapters = makeAdapters({
      fetchChallenge: jest.fn().mockRejectedValue(new Error('Network timeout')),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    expect(result.current.error).toBe('Network timeout');
    expect(result.current.token).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
    expect(adapters.signChallenge).not.toHaveBeenCalled();
    expect(adapters.submitChallenge).not.toHaveBeenCalled();
  });

  test('sets error when signChallenge rejects', async () => {
    const adapters = makeAdapters({
      signChallenge: jest.fn().mockRejectedValue(new Error('User rejected signing')),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    expect(result.current.error).toBe('User rejected signing');
    expect(result.current.token).toBeNull();
    expect(adapters.submitChallenge).not.toHaveBeenCalled();
  });

  test('sets error when submitChallenge rejects', async () => {
    const adapters = makeAdapters({
      submitChallenge: jest.fn().mockRejectedValue(new Error('Invalid signature')),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    expect(result.current.error).toBe('Invalid signature');
    expect(result.current.token).toBeNull();
  });

  test('wraps non-Error rejections in a generic message', async () => {
    const adapters = makeAdapters({
      fetchChallenge: jest.fn().mockRejectedValue('timeout'),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    expect(result.current.error).toBe('Authentication failed. Please try again.');
    expect(result.current.isAuthenticated).toBe(false);
  });

  test('isAuthenticating is false after a failed authenticate()', async () => {
    const adapters = makeAdapters({
      submitChallenge: jest.fn().mockRejectedValue(new Error('server error')),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    expect(result.current.isAuthenticating).toBe(false);
  });
});

// ── Error handling ────────────────────────────────────────────────────────────

describe('useSep10Auth — error handling', () => {
  test('error is cleared at the start of the next authenticate() call', async () => {
    const adapters = makeAdapters({
      fetchChallenge: jest.fn()
        .mockRejectedValueOnce(new Error('first failure'))
        .mockResolvedValue(CHALLENGE_XDR),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    // First call fails
    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });
    expect(result.current.error).toBe('first failure');

    // Second call clears error before running
    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });
    expect(result.current.error).toBeNull();
    expect(result.current.token).toBe(JWT);
  });

  test('retrying after an error sets the token on success', async () => {
    const adapters = makeAdapters({
      submitChallenge: jest.fn()
        .mockRejectedValueOnce(new Error('anchor unavailable'))
        .mockResolvedValue(JWT),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });
    expect(result.current.error).not.toBeNull();

    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });
    expect(result.current.token).toBe(JWT);
    expect(result.current.error).toBeNull();
  });
});

// ── Deduplication ─────────────────────────────────────────────────────────────

describe('useSep10Auth — deduplication', () => {
  test('concurrent calls to authenticate() are deduplicated to one network flow', async () => {
    let resolveChallenge!: (v: string) => void;
    const slowChallenge = new Promise<string>(r => { resolveChallenge = r; });
    const adapters = makeAdapters({ fetchChallenge: jest.fn().mockReturnValue(slowChallenge) });

    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    // Fire two concurrent authenticate calls before the first resolves
    act(() => {
      result.current.authenticate(ADDRESS);
      result.current.authenticate(ADDRESS);
    });

    act(() => { resolveChallenge(CHALLENGE_XDR); });

    await waitFor(() => expect(result.current.token).toBe(JWT));

    // Only one fetchChallenge request despite two authenticate() calls
    expect(adapters.fetchChallenge).toHaveBeenCalledTimes(1);
  });

  test('calling authenticate() while isAuthenticating returns immediately', async () => {
    let resolveSubmit!: (v: string) => void;
    const slowSubmit = new Promise<string>(r => { resolveSubmit = r; });
    const adapters = makeAdapters({ submitChallenge: jest.fn().mockReturnValue(slowSubmit) });

    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    act(() => { result.current.authenticate(ADDRESS); });

    await waitFor(() => expect(result.current.isAuthenticating).toBe(true));
    const callsBefore = (adapters.fetchChallenge as jest.Mock).mock.calls.length;

    // Second call while first is in-flight
    await act(async () => {
      await result.current.authenticate(ADDRESS);
    });

    // No additional fetchChallenge call
    expect(adapters.fetchChallenge).toHaveBeenCalledTimes(callsBefore);

    // Resolve the original and verify single token set
    act(() => { resolveSubmit(JWT); });
    await waitFor(() => expect(result.current.token).toBe(JWT));
    expect(adapters.submitChallenge).toHaveBeenCalledTimes(1);
  });

  test('authenticate() can be called again after a previous call completes', async () => {
    const jwt2 = 'second.jwt.token';
    const adapters = makeAdapters({
      submitChallenge: jest.fn()
        .mockResolvedValueOnce(JWT)
        .mockResolvedValueOnce(jwt2),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => { await result.current.authenticate(ADDRESS); });
    expect(result.current.token).toBe(JWT);

    await act(async () => { await result.current.authenticate(ADDRESS); });
    expect(result.current.token).toBe(jwt2);
    expect(adapters.submitChallenge).toHaveBeenCalledTimes(2);
  });
});

// ── Reset ─────────────────────────────────────────────────────────────────────

describe('useSep10Auth — reset', () => {
  test('reset() clears the token', async () => {
    const { result } = renderHook(() => useSep10Auth(DOMAIN, makeAdapters()));

    await act(async () => { await result.current.authenticate(ADDRESS); });
    expect(result.current.token).toBe(JWT);

    act(() => { result.current.reset(); });
    expect(result.current.token).toBeNull();
    expect(result.current.isAuthenticated).toBe(false);
  });

  test('reset() clears the error', async () => {
    const adapters = makeAdapters({
      fetchChallenge: jest.fn().mockRejectedValue(new Error('some error')),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => { await result.current.authenticate(ADDRESS); });
    expect(result.current.error).not.toBeNull();

    act(() => { result.current.reset(); });
    expect(result.current.error).toBeNull();
  });

  test('reset() clears isAuthenticating', async () => {
    let resolveChallenge!: (v: string) => void;
    const slowChallenge = new Promise<string>(r => { resolveChallenge = r; });
    const adapters = makeAdapters({ fetchChallenge: jest.fn().mockReturnValue(slowChallenge) });

    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    act(() => { result.current.authenticate(ADDRESS); });
    await waitFor(() => expect(result.current.isAuthenticating).toBe(true));

    act(() => { result.current.reset(); });
    expect(result.current.isAuthenticating).toBe(false);

    // Resolve the pending promise — no state update should occur (in-flight guard cleared)
    act(() => { resolveChallenge(CHALLENGE_XDR); });
  });

  test('authenticate() works after reset()', async () => {
    const adapters = makeAdapters();
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => { await result.current.authenticate(ADDRESS); });
    act(() => { result.current.reset(); });
    expect(result.current.token).toBeNull();

    await act(async () => { await result.current.authenticate(ADDRESS); });
    expect(result.current.token).toBe(JWT);
    expect(adapters.submitChallenge).toHaveBeenCalledTimes(2);
  });
});

// ── Adapter reference stability ───────────────────────────────────────────────

describe('useSep10Auth — adapter reference stability', () => {
  test('uses the latest adapter functions when authenticate() runs', async () => {
    const fetchV1 = jest.fn().mockResolvedValue(CHALLENGE_XDR);
    const fetchV2 = jest.fn().mockResolvedValue('updated_challenge==');

    const { result, rerender } = renderHook(
      ({ adapters }: { adapters: Sep10AuthAdapters }) => useSep10Auth(DOMAIN, adapters),
      { initialProps: { adapters: makeAdapters({ fetchChallenge: fetchV1 }) } }
    );

    // Swap the adapter before calling authenticate
    rerender({ adapters: makeAdapters({ fetchChallenge: fetchV2 }) });

    await act(async () => { await result.current.authenticate(ADDRESS); });

    // Should have called the updated adapter
    expect(fetchV2).toHaveBeenCalledTimes(1);
    expect(fetchV1).not.toHaveBeenCalled();
  });

  test('inline adapter functions do not cause the effect to restart', async () => {
    const callLog: string[] = [];
    const adapters: Sep10AuthAdapters = {
      fetchChallenge: jest.fn(async () => { callLog.push('fetch'); return CHALLENGE_XDR; }),
      signChallenge: jest.fn(async () => { callLog.push('sign'); return SIGNED_XDR; }),
      submitChallenge: jest.fn(async () => { callLog.push('submit'); return JWT; }),
    };

    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));

    await act(async () => { await result.current.authenticate(ADDRESS); });

    // Each step is called exactly once
    expect(callLog).toEqual(['fetch', 'sign', 'submit']);
  });
});

// ── isAuthenticated derivation ────────────────────────────────────────────────

describe('useSep10Auth — isAuthenticated', () => {
  test('isAuthenticated is false before authenticate()', () => {
    const { result } = renderHook(() => useSep10Auth(DOMAIN, makeAdapters()));
    expect(result.current.isAuthenticated).toBe(false);
  });

  test('isAuthenticated is true immediately after token is set', async () => {
    const { result } = renderHook(() => useSep10Auth(DOMAIN, makeAdapters()));
    await act(async () => { await result.current.authenticate(ADDRESS); });
    expect(result.current.isAuthenticated).toBe(true);
  });

  test('isAuthenticated is false after reset()', async () => {
    const { result } = renderHook(() => useSep10Auth(DOMAIN, makeAdapters()));
    await act(async () => { await result.current.authenticate(ADDRESS); });
    act(() => { result.current.reset(); });
    expect(result.current.isAuthenticated).toBe(false);
  });

  test('isAuthenticated is false after a failed authenticate()', async () => {
    const adapters = makeAdapters({
      submitChallenge: jest.fn().mockRejectedValue(new Error('fail')),
    });
    const { result } = renderHook(() => useSep10Auth(DOMAIN, adapters));
    await act(async () => { await result.current.authenticate(ADDRESS); });
    expect(result.current.isAuthenticated).toBe(false);
  });
});
