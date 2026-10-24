import React from 'react';
import type { Meta, StoryObj } from '@storybook/react';
import { AnchorErrorBoundary } from '../ui/components/AnchorErrorBoundary';

const meta = {
  title: 'Components/AnchorErrorBoundary',
  component: AnchorErrorBoundary,
  tags: ['autodocs'],
  argTypes: {
    componentLabel: { control: 'text' },
    onError: { action: 'onError' },
  },
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component:
          'React error boundary that catches rendering errors and displays a fallback UI. ' +
          'Supports a custom fallback render prop and automatic reset via `resetKeys`.',
      },
    },
  },
} satisfies Meta<typeof AnchorErrorBoundary>;

export default meta;
type Story = StoryObj<typeof meta>;

function BrokenComponent(): React.ReactElement {
  throw new Error('Simulated render error: anchor feed failed to load.');
}

function WorkingComponent() {
  return (
    <div style={{ padding: 16, borderRadius: 8, background: '#ecfdf5', border: '1px solid #a7f3d0', color: '#065f46' }}>
      Component rendered successfully.
    </div>
  );
}

export const CaughtError: Story = {
  name: 'Caught error (default fallback)',
  args: {
    componentLabel: 'Anchor Feed',
    onError: () => {},
    children: <BrokenComponent />,
  },
};

export const NoError: Story = {
  name: 'No error (children render normally)',
  args: {
    componentLabel: 'Anchor Feed',
    children: <WorkingComponent />,
  },
};

export const CustomFallback: Story = {
  name: 'Custom fallback UI',
  args: {
    componentLabel: 'Price Widget',
    onError: () => {},
    fallback: (err, reset) => (
      <div
        style={{
          padding: 20,
          borderRadius: 8,
          background: '#fef2f2',
          border: '1px solid #fecaca',
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
        }}
      >
        <strong style={{ color: '#991b1b' }}>Custom Error UI</strong>
        <p style={{ color: '#7f1d1d', margin: 0, fontSize: 14 }}>{err.message}</p>
        <button onClick={reset} style={{ alignSelf: 'flex-start', padding: '4px 12px', cursor: 'pointer' }}>
          Retry
        </button>
      </div>
    ),
    children: <BrokenComponent />,
  },
};
