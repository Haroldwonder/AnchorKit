import type { Preview } from '@storybook/react';
import '../components/themes.css';

const preview: Preview = {
  parameters: {
    actions: { argTypesRegex: '^on[A-Z].*' },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    backgrounds: {
      default: 'light',
      values: [
        { name: 'light', value: '#eef2fa' },
        { name: 'dark', value: '#050810' },
        { name: 'surface', value: '#ffffff' },
      ],
    },
  },
};

export default preview;
