import React, { useState } from 'react';
import { ApiRequestPanel } from './ApiRequestPanel';

/**
 * Example usage of ApiRequestPanel component
 * Demonstrates various use cases for AnchorKit API interactions
 */

export const ApiRequestPanelExamples: React.FC = () => {
  const [response, setResponse] = useState(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | undefined>();

  // Example 1: Submit Attestation
  const attestationExample = {
    endpoint: 'https://api.anchorkit.stellar.org/v1/attestations',
    method: 'POST' as const,
    requestBody: {
      issuer: 'GANCHOR123...',
      subject: 'GUSER456...',
      timestamp: 1708819200,
      payload_hash: 'abc123def456...',
      signature: 'sig789xyz...',
    },
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer <token>',
    },
  };

  // Example 2: Get Anchor Info
  const anchorInfoExample = {
    endpoint: 'https://api.anchorkit.stellar.org/v1/anchors/GANCHOR123',
    method: 'GET' as const,
    headers: {
      'Content-Type': 'application/json',
    },
  };

  // Example 3: Register Attestor
  const registerAttestorExample = {
    endpoint: 'https://api.anchorkit.stellar.org/v1/attestors',
    method: 'POST' as const,
    requestBody: {
      address: 'GANCHOR123...',
      services: ['deposits', 'withdrawals', 'kyc'],
      endpoint: 'https://anchor.example.com',
    },
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer <admin_token>',
    },
  };

  // Simulate API call
  const handleApiCall = async () => {
    setIsLoading(true);
    setError(undefined);
    
    try {
      // Simulate network delay
      await new Promise(resolve => setTimeout(resolve, 1500));
      
      // Mock response
      setResponse({
        success: true,
        attestation_id: 'att_123456',
        status: 'confirmed',
        timestamp: Date.now(),
      });
    } catch (err) {
      setError('Failed to submit attestation');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div style={{ padding: '24px', maxWidth: '1200px', margin: '0 auto' }}>
      <h1>API Request Panel Examples</h1>
      
      <section style={{ marginBottom: '48px' }}>
        <h2>Example 1: Submit Attestation</h2>
        <button 
          onClick={handleApiCall}
          style={{
            padding: '8px 16px',
            marginBottom: '16px',
            background: '#3b82f6',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          Test API Call
        </button>
        <ApiRequestPanel
          {...attestationExample}
          response={response}
          isLoading={isLoading}
          error={error}
        />
      </section>

      <section style={{ marginBottom: '48px' }}>
        <h2>Example 2: Get Anchor Info (GET Request)</h2>
        <ApiRequestPanel
          {...anchorInfoExample}
          response={{
            name: 'Example Anchor',
            address: 'GANCHOR123...',
            services: ['deposits', 'withdrawals'],
            health_status: 'healthy',
            last_check: 1708819200,
          }}
        />
      </section>

      <section style={{ marginBottom: '48px' }}>
        <h2>Example 3: Register Attestor</h2>
        <ApiRequestPanel
          {...registerAttestorExample}
          response={{
            success: true,
            attestor_id: 'GANCHOR123...',
            registered_at: 1708819200,
          }}
        />
      </section>

      <section style={{ marginBottom: '48px' }}>
        <h2>Example 4: Error State</h2>
        <ApiRequestPanel
          endpoint="https://api.anchorkit.stellar.org/v1/invalid"
          method="POST"
          requestBody={{ test: 'data' }}
          error="Network error: Unable to reach server"
        />
      </section>

      <section style={{ marginBottom: '48px' }}>
        <h2>Example 5: Loading State</h2>
        <ApiRequestPanel
          endpoint="https://api.anchorkit.stellar.org/v1/processing"
          method="POST"
          requestBody={{ operation: 'process' }}
          isLoading={true}
        />
      </section>
    </div>
  );
};

export default ApiRequestPanelExamples;
