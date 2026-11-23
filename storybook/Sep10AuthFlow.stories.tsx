import type { Meta, StoryObj } from '@storybook/react';
import SEP10AuthFlow from '../ui/components/Sep10AuthFlow';

const meta = {
  title: 'Components/SEP10AuthFlow',
  component: SEP10AuthFlow,
  tags: ['autodocs'],
  parameters: {
    layout: 'fullscreen',
    backgrounds: { default: 'dark' },
    docs: {
      description: {
        component:
          'Interactive SEP-10 challenge-response authentication flow. ' +
          'Self-contained stateful demo — no props required. ' +
          'Walk through wallet connect → challenge fetch → sign → JWT issuance.',
      },
    },
  },
} satisfies Meta<typeof SEP10AuthFlow>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  name: 'Idle (initial state)',
};
