import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TransactionTimeline, TxStatus, TxType, TxEvent } from './TransactionTimeline';
import '@testing-library/jest-dom';

const user = userEvent.setup();

const baseProps = {
  type: 'deposit' as TxType,
  amount: '250.00',
  asset: 'USDC',
  events: [] as TxEvent[],
  currentStatus: 'initiated' as TxStatus,
  onRetry: jest.fn(),
  onClose: jest.fn(),
};

describe('TransactionTimeline', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });
  test('renders deposit header correctly', () => {
    render(<TransactionTimeline {...baseProps} />);
    expect(screen.getByText('↓ Deposit')).toBeInTheDocument();
    expect(screen.getByText('250.00')).toHaveTextContent('250.00');
    expect(screen.getByText('USDC')).toBeInTheDocument();
    expect(screen.getAllByText('Initiated').length).toBeGreaterThan(0);
  });

  test('renders withdrawal header correctly', () => {
    render(<TransactionTimeline {...baseProps} type="withdrawal" />);
    expect(screen.getByText('↑ Withdrawal')).toBeInTheDocument();
  });

  test('renders status badge with correct color class', () => {
    render(<TransactionTimeline {...baseProps} currentStatus="processing" />);
    const badges = screen.getAllByText('Processing');
    expect(badges[0]).toBeInTheDocument();
    expect(badges[0]).toHaveStyle({ color: '#0284c7' });
  });

  test('renders all TxStatus icons and labels', () => {
    const statuses: TxStatus[] = ['initiated', 'pending', 'processing', 'completed', 'failed'];
    statuses.forEach(status => {
      render(<TransactionTimeline {...baseProps} currentStatus={status} />);
      const label = screen.getAllByText(new RegExp(status.charAt(0).toUpperCase() + status.slice(1), 'i'))[0];
      expect(label).toBeInTheDocument();
    });
  });

  test('shows txHash link when completed with txHash', () => {
    const events: TxEvent[] = [{
      status: 'completed' as TxStatus,
      txHash: 'abc123def456',
    }];
    render(<TransactionTimeline {...baseProps} currentStatus="completed" events={events} />);
    // truncateHash: 8 chars + … + 8 chars; 'abc123def456' is 12 chars so shown as-is (≤16)
    expect(screen.getByText(/abc123def456/)).toBeInTheDocument();
    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', expect.stringContaining('abc123def456'));
  });

  test('shows Retry button on failed status and calls onRetry on click', async () => {
    render(<TransactionTimeline {...baseProps} currentStatus="failed" />);
    const retryButton = screen.getByRole('button', { name: /retry/i });
    expect(retryButton).toBeInTheDocument();
    await user.click(retryButton);
    expect(baseProps.onRetry).toHaveBeenCalledTimes(1);
  });

  test('shows Close button and calls onClose on click', async () => {
    render(<TransactionTimeline {...baseProps} currentStatus="pending" />);
    const closeButton = screen.getByRole('button', { name: /close/i });
    expect(closeButton).toBeInTheDocument();
    await user.click(closeButton);
    expect(baseProps.onClose).toHaveBeenCalledTimes(1);
  });

  test('shows Done button on completed and calls onClose', async () => {
    render(<TransactionTimeline {...baseProps} currentStatus="completed" />);
    const doneButton = screen.getByRole('button', { name: /done/i });
    expect(doneButton).toBeInTheDocument();
    await user.click(doneButton);
    expect(baseProps.onClose).toHaveBeenCalledTimes(1);
  });

  test('renders event timestamps and details', () => {
    const events: TxEvent[] = [{
      status: 'pending' as TxStatus,
      timestamp: '2024-01-15T10:30:00Z',
      detail: 'via ACH',
    }];
    render(<TransactionTimeline {...baseProps} events={events} currentStatus="pending" />);
    // formatTs uses toLocaleString — find any element containing "Jan" and "15"
    const tsEls = screen.getAllByText((_, el) =>
      !!el?.textContent?.includes('Jan') && !!el?.textContent?.includes('15')
    );
    expect(tsEls.length).toBeGreaterThan(0);
    expect(screen.getByText('via ACH')).toBeInTheDocument();
  });

  test('renders failed state with custom label and description', () => {
    const events: TxEvent[] = [{
      status: 'failed' as TxStatus,
      label: 'Bank Error',
      description: 'Account details mismatch',
    }];
    render(<TransactionTimeline {...baseProps} events={events} currentStatus="failed" />);
    expect(screen.getByText('Bank Error')).toBeInTheDocument();
    expect(screen.getByText('Account details mismatch')).toBeInTheDocument();
    expect(screen.getByText('✕')).toBeInTheDocument();
  });

  test('does not show Retry button when not failed', () => {
    render(<TransactionTimeline {...baseProps} currentStatus="completed" />);
    expect(screen.queryByRole('button', { name: /retry/i })).not.toBeInTheDocument();
  });

  test('renders countdown when estimatedCompletionAt is provided', () => {
    const future = Date.now() + 120000; // 2 minutes from now
    render(<TransactionTimeline {...baseProps} estimatedCompletionAt={future} />);
    expect(screen.getByText(/Est. Completion/i)).toBeInTheDocument();
    expect(screen.getByText(/2:00/)).toBeInTheDocument();
  });

  describe('Error State Rendering', () => {
    test('renders failed transaction with error message', () => {
      const events: TxEvent[] = [
        { status: 'initiated', timestamp: '2024-01-15T10:00:00Z' },
        { status: 'pending', timestamp: '2024-01-15T10:05:00Z' },
        {
          status: 'failed',
          timestamp: '2024-01-15T10:10:00Z',
          label: 'Payment Failed',
          description: 'Insufficient funds in source account',
        },
      ];
      render(<TransactionTimeline {...baseProps} currentStatus="failed" events={events} />);
      
      expect(screen.getByText('Payment Failed')).toBeInTheDocument();
      expect(screen.getByText('Insufficient funds in source account')).toBeInTheDocument();
      expect(screen.getByText('✕')).toBeInTheDocument();
    });

    test('renders failed state with default description when not provided', () => {
      render(<TransactionTimeline {...baseProps} type="deposit" currentStatus="failed" events={[]} />);
      
      expect(screen.getByText(/Deposit could not be completed/i)).toBeInTheDocument();
    });

    test('renders failed state for withdrawal with default description', () => {
      render(<TransactionTimeline {...baseProps} type="withdrawal" currentStatus="failed" events={[]} />);
      
      expect(screen.getByText(/Withdrawal could not be completed/i)).toBeInTheDocument();
    });

    test('renders very long error message without breaking layout', () => {
      const longError = 'A'.repeat(500);
      const events: TxEvent[] = [{
        status: 'failed',
        description: longError,
      }];
      render(<TransactionTimeline {...baseProps} currentStatus="failed" events={events} />);
      
      expect(screen.getByText(longError)).toBeInTheDocument();
    });

    test('shows retry button only on failed status', () => {
      const { rerender } = render(<TransactionTimeline {...baseProps} currentStatus="failed" />);
      expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
      
      rerender(<TransactionTimeline {...baseProps} currentStatus="completed" />);
      expect(screen.queryByRole('button', { name: /retry/i })).not.toBeInTheDocument();
    });
  });

  describe('Unknown and Edge States', () => {
    test('renders with empty history array', () => {
      render(<TransactionTimeline {...baseProps} events={[]} currentStatus="initiated" />);
      
      expect(screen.getByText('Initiated')).toBeInTheDocument();
    });

    test('handles missing optional fields gracefully', () => {
      const minimalProps = {
        type: 'deposit' as TxType,
        amount: '100',
        asset: 'USDC',
        events: [] as TxEvent[],
        currentStatus: 'pending' as TxStatus,
      };
      render(<TransactionTimeline {...minimalProps} />);
      
      expect(screen.getByText('100')).toBeInTheDocument();
      expect(screen.getByText('USDC')).toBeInTheDocument();
    });

    test('renders transaction with no ID', () => {
      const propsWithoutId = { ...baseProps };
      delete (propsWithoutId as any).id;
      render(<TransactionTimeline {...propsWithoutId} />);
      
      expect(screen.getByText('250.00')).toBeInTheDocument();
    });

    test('renders with multiple events for same status', () => {
      const events: TxEvent[] = [
        { status: 'initiated', timestamp: '2024-01-15T10:00:00Z', detail: 'First attempt' },
        { status: 'initiated', timestamp: '2024-01-15T10:01:00Z', detail: 'Retry' },
      ];
      render(<TransactionTimeline {...baseProps} events={events} currentStatus="initiated" />);
      
      expect(screen.getByText('First attempt')).toBeInTheDocument();
    });
  });

  describe('Timestamp and Detail Rendering', () => {
    test('formats ISO timestamps correctly', () => {
      const events: TxEvent[] = [{
        status: 'completed',
        timestamp: '2024-01-15T14:30:00Z',
      }];
      render(<TransactionTimeline {...baseProps} currentStatus="completed" events={events} />);
      
      const tsElements = screen.getAllByText((_, el) =>
        !!el?.textContent?.includes('Jan') && !!el?.textContent?.includes('15')
      );
      expect(tsElements.length).toBeGreaterThan(0);
    });

    test('handles invalid timestamp gracefully', () => {
      const events: TxEvent[] = [{
        status: 'pending',
        timestamp: 'invalid-date',
      }];
      render(<TransactionTimeline {...baseProps} events={events} currentStatus="pending" />);
      
      expect(screen.getByText('invalid-date')).toBeInTheDocument();
    });

    test('renders event detail field', () => {
      const events: TxEvent[] = [{
        status: 'processing',
        detail: 'Processing via SEPA',
      }];
      render(<TransactionTimeline {...baseProps} events={events} currentStatus="processing" />);
      
      expect(screen.getByText('Processing via SEPA')).toBeInTheDocument();
    });
  });

  describe('Transaction Hash Links', () => {
    test('truncates long transaction hashes', () => {
      const longHash = 'a'.repeat(100);
      const events: TxEvent[] = [{
        status: 'completed',
        txHash: longHash,
      }];
      render(<TransactionTimeline {...baseProps} currentStatus="completed" events={events} />);
      
      const link = screen.getByRole('link');
      expect(link).toHaveAttribute('href', expect.stringContaining(longHash));
      // Should show truncated version (8 chars + … + 8 chars)
      expect(link.textContent).toContain('…');
    });

    test('does not truncate short hashes', () => {
      const shortHash = 'abc123';
      const events: TxEvent[] = [{
        status: 'completed',
        txHash: shortHash,
      }];
      render(<TransactionTimeline {...baseProps} currentStatus="completed" events={events} />);
      
      expect(screen.getByText(shortHash)).toBeInTheDocument();
    });
  });

  describe('Countdown Timer', () => {
    test('does not show countdown when transaction is completed', () => {
      const future = Date.now() + 60000;
      render(<TransactionTimeline {...baseProps} currentStatus="completed" estimatedCompletionAt={future} />);
      
      expect(screen.queryByText(/Est. Completion/i)).not.toBeInTheDocument();
    });

    test('does not show countdown when transaction is failed', () => {
      const future = Date.now() + 60000;
      render(<TransactionTimeline {...baseProps} currentStatus="failed" estimatedCompletionAt={future} />);
      
      expect(screen.queryByText(/Est. Completion/i)).not.toBeInTheDocument();
    });

    test('updates countdown timer', async () => {
      jest.useFakeTimers();
      const future = Date.now() + 65000; // 1:05
      render(<TransactionTimeline {...baseProps} currentStatus="processing" estimatedCompletionAt={future} />);
      
      expect(screen.getByText(/1:0[45]/)).toBeInTheDocument();
      
      jest.advanceTimersByTime(10000); // Advance 10 seconds
      
      await waitFor(() => {
        expect(screen.getByText(/0:5[0-9]/)).toBeInTheDocument();
      });
      
      jest.useRealTimers();
    });
  });
});

describe('Expandable Timeline Entries', () => {
  const eventWithAll: TxEvent = {
    status: 'pending',
    timestamp: '2024-06-01T12:00:00Z',
    errorMessage: 'Rate limit exceeded',
    rawApiResponse: { status: 'pending', code: 429, message: 'Too many requests' },
  };

  const eventMinimal: TxEvent = {
    status: 'initiated',
  };

  describe('Expand / collapse behavior', () => {
    test('More button is present for a step that has an event', () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      expect(screen.getByRole('button', { name: /▼ More/i })).toBeInTheDocument();
    });

    test('no More button for steps without an event', () => {
      // Only one event supplied (pending); initiated step has no event
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      // There is exactly one toggle button
      const buttons = screen.getAllByRole('button', { name: /▼ More|▲ Less/i });
      expect(buttons).toHaveLength(1);
    });

    test('clicking More expands the details panel', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      const toggle = screen.getByRole('button', { name: /▼ More/i });
      await userEvent.click(toggle);
      expect(screen.getByRole('region')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /▲ Less/i })).toBeInTheDocument();
    });

    test('clicking Less collapses the details panel', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      const toggle = screen.getByRole('button', { name: /▼ More/i });
      await userEvent.click(toggle);
      await userEvent.click(screen.getByRole('button', { name: /▲ Less/i }));
      expect(screen.queryByRole('region')).not.toBeInTheDocument();
    });

    test('expanding one step collapses another', async () => {
      const events: TxEvent[] = [
        { status: 'initiated', rawApiResponse: { step: 'init' } },
        { status: 'pending',   rawApiResponse: { step: 'pend' } },
      ];
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={events}
        />
      );
      const [firstToggle, secondToggle] = screen.getAllByRole('button', { name: /▼ More/i });
      await userEvent.click(firstToggle);
      expect(screen.getAllByRole('region')).toHaveLength(1);

      await userEvent.click(secondToggle);
      // Only second is now open
      expect(screen.getAllByRole('region')).toHaveLength(1);
      expect(screen.getByText(/"step": "pend"/)).toBeInTheDocument();
      expect(screen.queryByText(/"step": "init"/)).not.toBeInTheDocument();
    });
  });

  describe('Accessible semantics', () => {
    test('toggle button has aria-expanded=false when collapsed', () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      const btn = screen.getByRole('button', { name: /▼ More/i });
      expect(btn).toHaveAttribute('aria-expanded', 'false');
    });

    test('toggle button has aria-expanded=true when expanded', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.getByRole('button', { name: /▲ Less/i })).toHaveAttribute('aria-expanded', 'true');
    });

    test('toggle button has aria-controls pointing to panel id', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      const btn = screen.getByRole('button', { name: /▼ More/i });
      const controls = btn.getAttribute('aria-controls');
      expect(controls).toBe('step-details-pending');

      await userEvent.click(btn);
      const panel = document.getElementById('step-details-pending');
      expect(panel).not.toBeNull();
    });

    test('expanded panel has role=region', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.getByRole('region')).toBeInTheDocument();
    });
  });

  describe('Keyboard interaction', () => {
    test('Enter key expands the step', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      const btn = screen.getByRole('button', { name: /▼ More/i });
      btn.focus();
      fireEvent.keyDown(btn, { key: 'Enter' });
      expect(screen.getByRole('region')).toBeInTheDocument();
    });

    test('Space key expands the step', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      const btn = screen.getByRole('button', { name: /▼ More/i });
      btn.focus();
      fireEvent.keyDown(btn, { key: ' ' });
      expect(screen.getByRole('region')).toBeInTheDocument();
    });

    test('Enter key collapses when already expanded', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      const btn = screen.getByRole('button', { name: /▼ More/i });
      await userEvent.click(btn);
      const lessBtn = screen.getByRole('button', { name: /▲ Less/i });
      lessBtn.focus();
      fireEvent.keyDown(lessBtn, { key: 'Enter' });
      expect(screen.queryByRole('region')).not.toBeInTheDocument();
    });
  });

  describe('Raw API response rendering', () => {
    test('renders stringified JSON when rawApiResponse is an object', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.getByText(/"code": 429/)).toBeInTheDocument();
    });

    test('renders string rawApiResponse as-is', async () => {
      const event: TxEvent = { status: 'pending', rawApiResponse: 'raw string response' };
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[event]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.getByText('raw string response')).toBeInTheDocument();
    });

    test('shows placeholder when rawApiResponse is absent', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="initiated"
          events={[eventMinimal]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.getByText('No response data available.')).toBeInTheDocument();
    });
  });

  describe('Timestamp in expanded panel', () => {
    test('shows formatted timestamp when present', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      const tsEls = screen.getAllByText((_, el) =>
        !!el?.textContent?.includes('Jun') && !!el?.textContent?.includes('1')
      );
      expect(tsEls.length).toBeGreaterThan(0);
    });

    test('shows placeholder dash when timestamp is absent', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="initiated"
          events={[eventMinimal]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.getByText('—')).toBeInTheDocument();
    });
  });

  describe('Error message in expanded panel', () => {
    test('shows error message when errorMessage is present', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[eventWithAll]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.getByText('Rate limit exceeded')).toBeInTheDocument();
    });

    test('shows fallback when errorMessage is empty string', async () => {
      const event: TxEvent = { status: 'pending', errorMessage: '' };
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[event]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.getByText('No error message provided.')).toBeInTheDocument();
    });

    test('does not render error section when errorMessage is not set', async () => {
      const event: TxEvent = { status: 'pending', rawApiResponse: { ok: true } };
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="pending"
          events={[event]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.queryByText(/No error message provided\./)).not.toBeInTheDocument();
    });
  });

  describe('Incomplete data handling', () => {
    test('step with no event does not render a toggle button', () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="initiated"
          events={[]}
        />
      );
      expect(screen.queryByRole('button', { name: /▼ More|▲ Less/i })).not.toBeInTheDocument();
    });

    test('handles event with only status field (all optional fields absent)', async () => {
      render(
        <TransactionTimeline
          {...baseProps}
          currentStatus="initiated"
          events={[{ status: 'initiated' }]}
        />
      );
      await userEvent.click(screen.getByRole('button', { name: /▼ More/i }));
      expect(screen.getByText('—')).toBeInTheDocument();
      expect(screen.getByText('No response data available.')).toBeInTheDocument();
      expect(screen.queryByText(/No error message provided\./)).not.toBeInTheDocument();
    });
  });
});
