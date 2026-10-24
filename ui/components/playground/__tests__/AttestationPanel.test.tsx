import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import { AttestationPanel, AttestationEntry } from '../AttestationPanel';

// ── fixtures ──────────────────────────────────────────────────────────────────

const VALID_HASH = 'a'.repeat(64);
const OTHER_HASH = 'b'.repeat(64);

const sampleAttestations: AttestationEntry[] = [
  {
    id: 'att-1',
    subject: 'GABC1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ01234',
    payloadHash: VALID_HASH,
    timestamp: 1700000000,
    valid: true,
  },
  {
    id: 'att-2',
    subject: 'GXYZ9876543210ZYXWVUTSRQPONMLKJIHGFEDCBA98765',
    payloadHash: OTHER_HASH,
    timestamp: 1700001000,
    valid: false,
  },
];

// ── rendering ─────────────────────────────────────────────────────────────────

describe('AttestationPanel – rendering', () => {
  it('renders without crashing', () => {
    render(<AttestationPanel attestations={[]} />);
    expect(screen.getByRole('region', { name: /attestation panel/i })).toBeInTheDocument();
  });

  it('shows an empty-state message when there are no attestations', () => {
    render(<AttestationPanel attestations={[]} />);
    expect(screen.getByRole('status')).toHaveTextContent(/no attestations recorded yet/i);
  });

  it('renders the correct number of list items', () => {
    render(<AttestationPanel attestations={sampleAttestations} />);
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(sampleAttestations.length);
  });

  it('shows the attestation count badge', () => {
    render(<AttestationPanel attestations={sampleAttestations} />);
    expect(screen.getByText(String(sampleAttestations.length))).toBeInTheDocument();
  });

  it('marks valid attestations with the "Valid" label', () => {
    render(<AttestationPanel attestations={sampleAttestations} />);
    expect(screen.getByText('Valid')).toBeInTheDocument();
  });

  it('marks revoked attestations with the "Revoked" label', () => {
    render(<AttestationPanel attestations={sampleAttestations} />);
    expect(screen.getByText('Revoked')).toBeInTheDocument();
  });
});

// ── submit form ───────────────────────────────────────────────────────────────

describe('AttestationPanel – submit form', () => {
  it('does not render the submit form when onSubmit is not provided', () => {
    render(<AttestationPanel attestations={[]} />);
    expect(screen.queryByRole('form', { name: /submit attestation/i })).not.toBeInTheDocument();
  });

  it('renders the submit form when onSubmit is provided', () => {
    render(<AttestationPanel attestations={[]} onSubmit={jest.fn()} />);
    expect(screen.getByRole('form', { name: /submit attestation/i })).toBeInTheDocument();
  });

  it('renders subject address and payload hash inputs', () => {
    render(<AttestationPanel attestations={[]} onSubmit={jest.fn()} />);
    expect(screen.getByLabelText(/subject address/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/payload hash/i)).toBeInTheDocument();
  });

  it('calls onSubmit with trimmed subject and lowercased hash on valid submission', () => {
    const onSubmit = jest.fn();
    render(<AttestationPanel attestations={[]} onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText(/subject address/i), {
      target: { value: '  GABC123  ' },
    });
    fireEvent.change(screen.getByLabelText(/payload hash/i), {
      target: { value: VALID_HASH.toUpperCase() },
    });
    fireEvent.click(screen.getByRole('button', { name: /submit/i }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit).toHaveBeenCalledWith('GABC123', VALID_HASH.toLowerCase());
  });

  it('clears the fields after a successful submission', () => {
    render(<AttestationPanel attestations={[]} onSubmit={jest.fn()} />);

    const subjectInput = screen.getByLabelText(/subject address/i);
    const hashInput = screen.getByLabelText(/payload hash/i);

    fireEvent.change(subjectInput, { target: { value: 'GABC123' } });
    fireEvent.change(hashInput, { target: { value: VALID_HASH } });
    fireEvent.click(screen.getByRole('button', { name: /submit/i }));

    expect(subjectInput).toHaveValue('');
    expect(hashInput).toHaveValue('');
  });

  it('shows an error and does not call onSubmit when subject is empty', () => {
    const onSubmit = jest.fn();
    render(<AttestationPanel attestations={[]} onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText(/payload hash/i), {
      target: { value: VALID_HASH },
    });
    fireEvent.click(screen.getByRole('button', { name: /submit/i }));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByRole('alert')).toHaveTextContent(/subject address is required/i);
  });

  it('shows an error when payload hash is not 64 hex characters', () => {
    const onSubmit = jest.fn();
    render(<AttestationPanel attestations={[]} onSubmit={onSubmit} />);

    fireEvent.change(screen.getByLabelText(/subject address/i), {
      target: { value: 'GABC123' },
    });
    fireEvent.change(screen.getByLabelText(/payload hash/i), {
      target: { value: 'tooshort' },
    });
    fireEvent.click(screen.getByRole('button', { name: /submit/i }));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByRole('alert')).toHaveTextContent(/64-character hex/i);
  });
});

// ── export surface ────────────────────────────────────────────────────────────

describe('AttestationPanel – module export', () => {
  it('is exported from the playground directory', async () => {
    // Dynamic import verifies the module exists and the named export is a function
    const mod = await import('../AttestationPanel');
    expect(typeof mod.AttestationPanel).toBe('function');
  });

  it('is re-exported from the top-level components index', async () => {
    const mod = await import('../../index');
    expect(typeof mod.AttestationPanel).toBe('function');
  });
});
