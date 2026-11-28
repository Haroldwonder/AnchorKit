import React, { useState } from 'react';

export interface Quote {
  id: string;
  rate: number;
  fee: number;
  minAmount?: number;
  maxAmount?: number;
  validUntil: number;
  health?: string;
}

export interface QuoteComparisonProps {
  quotes: Quote[];
}

type SortKey = 'rate' | 'fee' | 'minAmount' | 'maxAmount' | 'validUntil' | 'health';

export const QuoteComparison: React.FC<QuoteComparisonProps> = ({ quotes }) => {
  const [sortKey, setSortKey] = useState<SortKey>('rate');
  const [asc, setAsc] = useState(true);
  const sorted = [...quotes].sort((a, b) => {
    const av = (a as any)[sortKey] ?? 0;
    const bv = (b as any)[sortKey] ?? 0;
    return asc ? (av > bv ? 1 : -1) : (av < bv ? 1 : -1);
  });
  const toggle = (k: SortKey) => {
    if (sortKey === k) setAsc(!asc); else { setSortKey(k); setAsc(true); }
  };
  return (
    <table>
      <thead>
        <tr>
          {(['rate','fee','minAmount','maxAmount','validUntil','health'] as SortKey[]).map((k) => (
            <th key={k} onClick={() => toggle(k)} style={{ cursor: 'pointer' }} tabIndex={0} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggle(k); }}>{k}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {sorted.map((q) => (
          <tr key={q.id}>
            <td>{q.rate}</td>
            <td>{q.fee}</td>
            <td>{q.minAmount}</td>
            <td>{q.maxAmount}</td>
            <td>{new Date(q.validUntil).toLocaleTimeString()}</td>
            <td>{q.health}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
};
