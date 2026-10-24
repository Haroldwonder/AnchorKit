import type { Meta, StoryObj } from '@storybook/react';
import {
  SkeletonLoader,
  AssetListSkeleton,
  FeeTableSkeleton,
  LimitsSkeleton,
} from '../ui/components/SkeletonLoader';

const meta = {
  title: 'Components/SkeletonLoader',
  component: SkeletonLoader,
  tags: ['autodocs'],
  argTypes: {
    variant: {
      control: 'select',
      options: ['text', 'rect', 'circle', 'list', 'table'],
      description: 'Shape of the skeleton placeholder',
    },
    width: { control: 'text', description: 'CSS width value' },
    height: { control: 'text', description: 'CSS height value' },
    count: {
      control: { type: 'number', min: 1, max: 10 },
      description: 'Number of rows for list/table variants',
    },
    dark: { control: 'boolean', description: 'Use dark shimmer colors' },
    ariaLabel: { control: 'text' },
  },
  parameters: {
    layout: 'padded',
  },
} satisfies Meta<typeof SkeletonLoader>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Rect: Story = {
  args: {
    variant: 'rect',
    width: '100%',
    height: 80,
    dark: false,
  },
};

export const Text: Story = {
  args: {
    variant: 'text',
    width: '70%',
    dark: false,
  },
};

export const Circle: Story = {
  args: {
    variant: 'circle',
    width: 56,
    dark: false,
  },
};

export const List: Story = {
  args: {
    variant: 'list',
    count: 4,
    dark: false,
  },
};

export const Table: Story = {
  args: {
    variant: 'table',
    count: 4,
    dark: false,
  },
};

export const DarkRect: Story = {
  name: 'Rect (dark)',
  args: {
    variant: 'rect',
    width: '100%',
    height: 80,
    dark: true,
  },
  parameters: {
    backgrounds: { default: 'dark' },
  },
};

export const DarkList: Story = {
  name: 'List (dark)',
  args: {
    variant: 'list',
    count: 4,
    dark: true,
  },
  parameters: {
    backgrounds: { default: 'dark' },
  },
};

export const AssetList: Story = {
  name: 'Compound – AssetListSkeleton',
  render: (args) => <AssetListSkeleton dark={args.dark} count={args.count} />,
  args: { dark: false, count: 3 },
};

export const FeeTable: Story = {
  name: 'Compound – FeeTableSkeleton',
  render: (args) => <FeeTableSkeleton dark={args.dark} count={args.count} />,
  args: { dark: false, count: 3 },
};

export const Limits: Story = {
  name: 'Compound – LimitsSkeleton',
  render: (args) => <LimitsSkeleton dark={args.dark} />,
  args: { dark: false },
};
