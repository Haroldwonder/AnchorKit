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
});
