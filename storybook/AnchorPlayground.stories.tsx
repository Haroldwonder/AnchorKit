import type { Meta, StoryObj } from '@storybook/react';
import AnchorPlayground from '../ui/components/AnchorPlayground';

const meta = {
  title: 'Components/AnchorPlayground',
  component: AnchorPlayground,
  tags: ['autodocs'],
  parameters: {
    layout: 'fullscreen',
    docs: {
      description: {
        component:
          'Interactive SEP protocol playground. Explore SEP-1, SEP-10, SEP-24, and SEP-31 ' +
          'endpoints with a live request builder and response viewer. ' +
          'Self-contained stateful component — no props required.',
      },
    },
  },
} satisfies Meta<typeof AnchorPlayground>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  name: 'Interactive playground',
};
