import { useState, useEffect, useCallback, useRef } from 'react';

export interface Quote {
  id: string;
  baseAsset: string;
  quoteAsset: string;
  rate: number;
  fee: number;
  minAmount?: number;
  maxAmount?: number;
  validUntil: number;
}

export interface UseQuotesOptions {
  baseAsset: string;
  quoteAsset: string;
  amount: number;
  client: { fetchQuotes: (p: { baseAsset: string; quoteAsset: string; amount: number }) => Promise<Quote[]> };
  refreshBufferMs?: number;
}

export interface UseQuotesResult {
  quotes: Quote[];
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export function useQuotes({ baseAsset, quoteAsset, amount, client, refreshBufferMs = 5000 }: UseQuotesOptions): UseQuotesResult {
  const [quotes, setQuotes] = useState<Quote[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fetchQuotes = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await client.fetchQuotes({ baseAsset, quoteAsset, amount });
      const now = Date.now();
      const valid = data.filter((q) => q.validUntil > now);
      setQuotes(valid);
      const earliest = valid.reduce((m, q) => Math.min(m, q.validUntil), Infinity);
      if (timerRef.current) clearTimeout(timerRef.current);
      if (isFinite(earliest)) {
        timerRef.current = setTimeout(fetchQuotes, Math.max(0, earliest - Date.now() - refreshBufferMs));
      }
    } catch (e) {
      setError(e instanceof Error ? e : new Error(String(e)));
    } finally {
      setIsLoading(false);
    }
  }, [baseAsset, quoteAsset, amount, client, refreshBufferMs]);

  useEffect(() => {
    fetchQuotes();
    return () => { if (timerRef.current) clearTimeout(timerRef.current); };
  }, [fetchQuotes]);

  return { quotes, isLoading, error, refetch: fetchQuotes };
}
