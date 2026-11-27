import React from 'react';
import { useAttestation } from '../hooks/useAttestation';
import { useCopyToClipboard } from '../hooks/useCopyToClipboard';

export interface AttestationCardProps {
  id: string;
  client: { fetchAttestation: (id: string) => Promise<any> };
}

export const AttestationCard: React.FC<AttestationCardProps> = ({ id, client }) => {
  const { attestation, isLoading, error } = useAttestation(id, client);
  const { copy } = useCopyToClipboard();
  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;
  if (!attestation) return null;
  const expired = attestation.expiresAt ? attestation.expiresAt < Date.now() : false;
  const revoked = attestation.revoked;
  return (
    <div style={{ border: '1px solid #ccc', padding: 12, borderRadius: 8 }}>
      <div>Issuer: {attestation.issuer}</div>
      <div>Subject: {attestation.subject}</div>
      <div>Timestamp: {new Date(attestation.timestamp).toLocaleString()}</div>
      <div style={{ color: revoked ? 'red' : expired ? 'orange' : 'green' }}>
        {revoked ? 'Revoked' : expired ? 'Expired' : 'Valid'}
      </div>
      <div>
        Hash: {attestation.payloadHash}
        <button onClick={() => copy(attestation.payloadHash)}>Copy</button>
      </div>
    </div>
  );
};
