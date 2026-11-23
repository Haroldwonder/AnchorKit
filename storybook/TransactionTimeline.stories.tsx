import type { Meta, StoryObj } from '@storybook/react';
import { TransactionTimeline } from '../ui/components/TransactionTimeline';
import type { TxEvent, TxStatus, TxType } from '../ui/components/TransactionTimeline';

const meta = {
  title: 'Components/TransactionTimeline',
  component: TransactionTimeline,
  tags: ['autodocs'],
  argTypes: {
    type: {
      control: 'radio',
      options: ['deposit', 'withdrawal'] as TxType[],
    },
    currentStatus: {
      control: 'select',
      options: ['initiated', 'pending', 'processing', 'completed', 'failed'] as TxStatus[],
    },
    amount: { control: 'text' },
    asset: { control: 'text' },
    id: { control: 'text' },
    onRetry: { action: 'retry clicked' },
    onClose: { action: 'close clicked' },
  },
  parameters: {
    layout: 'padded',
  },
} satisfies Meta<typeof TransactionTimeline>;

export default meta;
type Story = StoryObj<typeof meta>;

const depositEvents: TxEvent[] = [
  { status: 'initiated', timestamp: new Date(Date.now() - 4 * 60000).toISOString(), description: 'Deposit request received.' },
  { status: 'pending', timestamp: new Date(Date.now() - 3 * 60000).toISOString(), description: 'Awaiting your bank transfer.' },
  { status: 'processing', timestamp: new Date(Date.now() - 60000).toISOString(), description: 'Funds received — minting on Stellar.' },
];

const withdrawalEvents: TxEvent[] = [
  { status: 'initiated', timestamp: new Date(Date.now() - 6 * 60000).toISOString() },
  { status: 'pending', timestamp: new Date(Date.now() - 4 * 60000).toISOString() },
];

export const Processing: Story = {
  args: {
    type: 'deposit',
    amount: '250.00',
    asset: 'USDC',
    id: 'TXN-8841AA',
    currentStatus: 'processing',
    events: depositEvents,
    estimatedCompletionAt: Date.now() + 5 * 60000,
    onRetry: () => {},
    onClose: () => {},
  },
};

export const Completed: Story = {
  args: {
    type: 'deposit',
    amount: '1,000.00',
    asset: 'USDC',
    id: 'TXN-22FA91',
    currentStatus: 'completed',
    events: [
      ...depositEvents,
      {
        status: 'completed',
        timestamp: new Date().toISOString(),
        txHash: 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
        description: 'Assets delivered to your Stellar account.',
      },
    ],
    onClose: () => {},
  },
};

export const Failed: Story = {
  args: {
    type: 'withdrawal',
    amount: '500.00',
    asset: 'USDC',
    id: 'TXN-FAIL01',
    currentStatus: 'failed',
    events: [
      ...withdrawalEvents,
      { status: 'failed', timestamp: new Date().toISOString(), description: 'Bank rejected the transfer.' },
    ],
    onRetry: () => {},
    onClose: () => {},
  },
};

export const Pending: Story = {
  args: {
    type: 'withdrawal',
    amount: '75.00',
    asset: 'USDC',
    id: 'TXN-PEND55',
    currentStatus: 'pending',
    events: withdrawalEvents,
    estimatedCompletionAt: Date.now() + 10 * 60000,
    onRetry: () => {},
    onClose: () => {},
  },
};

export const Initiated: Story = {
  args: {
    type: 'deposit',
    amount: '50.00',
    asset: 'USDC',
    id: 'TXN-NEW00',
    currentStatus: 'initiated',
    events: [{ status: 'initiated', timestamp: new Date().toISOString() }],
    onClose: () => {},
  },
};
