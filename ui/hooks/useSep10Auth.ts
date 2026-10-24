import { useState, useRef, useCallback, useEffect } from 'react';

// ── Public types ──────────────────────────────────────────────────────────────

/**
 * Injectable adapters for each step of the SEP-10 challenge-response protocol.
 * Keeping these injectable decouples the hook from any specific transport or
 * wallet library and makes the hook straightforward to unit-test.
 */
export interface Sep10AuthAdapters {
  /** GET /{domain}/auth?account={address} → base64-encoded challenge XDR */
  fetchChallenge: (domain: string, walletAddress: string) => Promise<string>;
  /** Sign the challenge XDR with the wallet private key → signed XDR */
  signChallenge: (challengeXdr: string) => Promise<string>;
  /** POST /{domain}/auth with signed XDR → JWT bearer token */
  submitChallenge: (domain: string, signedXdr: string) => Promise<string>;
}

export interface UseSep10AuthResult {
  /** JWT bearer token, or null before successful authentication */
  token: string | null;
  /** True when token is non-null (derived — does not check expiry) */
  isAuthenticated: boolean;
  /** True while authenticate() is running */
  isAuthenticating: boolean;
  /** Standardized loading flag for hook consumers. */
  isLoading: boolean;
  /** @deprecated Use isLoading instead. */
  loading: boolean;
  /** The most recent authentication error, or null if none. */
  error: Error | null;
  /** @deprecated Use error.message instead. */
  errorMessage: string | null;
  /**
   * Executes the full SEP-10 challenge-response flow:
   * fetchChallenge → signChallenge → submitChallenge.
   * Concurrent calls are deduplicated — a second call while one is in
   * progress returns immediately without starting a new flow.
   */
  authenticate: (walletAddress: string) => Promise<void>;
  /** Clears token, error, and in-flight guard so authentication can be retried */
  reset: () => void;
}

// ── Hook ─────────────────────────────────────────────────────────────────────

/**
 * Manages SEP-10 authentication against a single anchor domain.
 *
 * When `anchorDomain` changes, the hook automatically resets `token`, `error`,
 * and `isAuthenticating` so a JWT issued for the previous anchor is never
 * reported as valid for the new one. Callers do not need to invoke `reset()`
 * manually after switching anchors — `isAuthenticated` becomes `false` until
 * `authenticate()` succeeds against the new domain.
 */
export function useSep10Auth(
  anchorDomain: string,
  adapters: Sep10AuthAdapters,
): UseSep10AuthResult {
  const [token, setToken] = useState<string | null>(null);
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Keep refs so authenticate() always reads the latest domain and adapters
  // even when callers pass new inline objects on every render.
  const adaptersRef = useRef<Sep10AuthAdapters>(adapters);
  adaptersRef.current = adapters;

  const domainRef = useRef(anchorDomain);
  domainRef.current = anchorDomain;

  // In-flight guard: prevents duplicate concurrent SEP-10 flows.
  const inFlightRef = useRef(false);

  // Reset auth state whenever the anchor domain changes so a stale JWT from a
  // previous anchor is never treated as valid for the new domain.
  useEffect(() => {
    setToken(null);
    setError(null);
    setIsAuthenticating(false);
    inFlightRef.current = false;
  }, [anchorDomain]);

  const authenticate = useCallback(async (walletAddress: string): Promise<void> => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    setIsAuthenticating(true);
    setError(null);

    try {
      const domain = domainRef.current;
      const challengeXdr = await adaptersRef.current.fetchChallenge(domain, walletAddress);
      const signedXdr = await adaptersRef.current.signChallenge(challengeXdr);
      const jwt = await adaptersRef.current.submitChallenge(domain, signedXdr);
      setToken(jwt);
    } catch (err) {
      setError(
        err instanceof Error
          ? err
          : new Error('Authentication failed. Please try again.'),
      );
    } finally {
      setIsAuthenticating(false);
      inFlightRef.current = false;
    }
  }, []);

  const reset = useCallback(() => {
    setToken(null);
    setError(null);
    setIsAuthenticating(false);
    inFlightRef.current = false;
  }, []);

  return {
    token,
    isAuthenticated: token !== null,
    isAuthenticating,
    isLoading: isAuthenticating,
    loading: isAuthenticating,
    error,
    errorMessage: error?.message ?? null,
    authenticate,
    reset,
  };
}
