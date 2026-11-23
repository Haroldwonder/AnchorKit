import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import { AnchorErrorBoundary } from './AnchorErrorBoundary';

// Suppress React's console.error output for expected error boundary throws
beforeEach(() => jest.spyOn(console, 'error').mockImplementation(() => {}));
afterEach(() => (console.error as jest.Mock).mockRestore());

// A component that throws on demand
function Bomb({ shouldThrow }: { shouldThrow: boolean }) {
  if (shouldThrow) throw new Error('boom');
  return <div>OK</div>;
}

describe('AnchorErrorBoundary – DefaultFallback', () => {
  test('renders children when no error', () => {
    render(
      <AnchorErrorBoundary>
        <Bomb shouldThrow={false} />
      </AnchorErrorBoundary>
    );
    expect(screen.getByText('OK')).toBeInTheDocument();
  });

  test('renders error message when child throws', () => {
    render(
      <AnchorErrorBoundary>
        <Bomb shouldThrow={true} />
      </AnchorErrorBoundary>
    );
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(/boom/i)).toBeInTheDocument();
  });

  test('Try Again button is present and accessible', () => {
    render(
      <AnchorErrorBoundary>
        <Bomb shouldThrow={true} />
      </AnchorErrorBoundary>
    );
    const btn = screen.getByRole('button', { name: /try again/i });
    expect(btn).toBeInTheDocument();
  });

  test('clicking Try Again calls reset and re-renders children', () => {
    const { rerender } = render(
      <AnchorErrorBoundary>
        <Bomb shouldThrow={true} />
      </AnchorErrorBoundary>
    );

    // Error state shown
    expect(screen.getByRole('alert')).toBeInTheDocument();

    // Click Try Again
    fireEvent.click(screen.getByRole('button', { name: /try again/i }));

    // Re-render with non-throwing child to confirm reset worked
    rerender(
      <AnchorErrorBoundary>
        <Bomb shouldThrow={false} />
      </AnchorErrorBoundary>
    );
    expect(screen.getByText('OK')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  test('calls onError prop when child throws', () => {
    const onError = jest.fn();
    render(
      <AnchorErrorBoundary onError={onError}>
        <Bomb shouldThrow={true} />
      </AnchorErrorBoundary>
    );
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onError.mock.calls[0][0]).toMatchObject({ message: 'boom' });
  });

  test('uses custom fallback when provided', () => {
    const fallback = jest.fn(() => <div>custom fallback</div>);
    render(
      <AnchorErrorBoundary fallback={fallback}>
        <Bomb shouldThrow={true} />
      </AnchorErrorBoundary>
    );
    expect(screen.getByText('custom fallback')).toBeInTheDocument();
    expect(fallback).toHaveBeenCalledWith(
      expect.objectContaining({ message: 'boom' }),
      expect.any(Function)
    );
  });
});
