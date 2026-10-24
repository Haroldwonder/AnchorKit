import type { Meta, StoryObj } from '@storybook/react';
import { AnchorHealthBadge } from '../ui/components/AnchorHealthBadge';

const meta = {
  title: 'Components/AnchorHealthBadge',
  component: AnchorHealthBadge,
  tags: ['autodocs'],
  argTypes: {
    score: { control: { type: 'range', min: 0, max: 100 } },
    showScore: { control: 'boolean' },
  },
  parameters: { layout: 'centered' },
} satisfies Meta<typeof AnchorHealthBadge>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Healthy: Story = {
  name: 'Healthy (≥80)',
  args: { score: 95, showScore: true },
};

export const Fair: Story = {
  name: 'Fair (60–79)',
  args: { score: 68, showScore: true },
};

export const Poor: Story = {
  name: 'Poor (<60)',
  args: { score: 32, showScore: true },
};

export const NoScore: Story = {
  name: 'Label only (showScore=false)',
  args: { score: 85, showScore: false },
};

export const AllTiers: Story = {
  name: 'All tiers side by side',
  render: () => (
    <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
      <AnchorHealthBadge score={90} />
      <AnchorHealthBadge score={65} />
      <AnchorHealthBadge score={40} />
      <AnchorHealthBadge score={0} />
    </div>
  ),
};
