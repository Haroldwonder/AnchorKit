import { useState, useEffect } from 'react';

export interface Attestation {
  id: string;
  issuer: string;
  subject: string;
  timestamp: number;
  payloadHash: string;
  revoked: boolean;
  expiresAt?: number;
}

export interface UseAttestationResult {
  attestation: Attestation | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export function useAttestation(id: string, client: { fetchAttestation: (id: string) => Promise<Attestation> }): UseAttestationResult {
  const [attestation, setAttestation] = useState<Attestation | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refetch = () => {
    setIsLoading(true);
    setError(null);
    client.fetchAttestation(id)
      .then(setAttestation)
      .catch((e) => setError(e instanceof Error ? e : new Error(String(e))))
      .finally(() => setIsLoading(false));
  };

  useEffect(() => { refetch(); }, [id]);
  return { attestation, isLoading, error, refetch };
}
