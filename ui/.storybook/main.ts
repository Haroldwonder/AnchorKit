import type { StorybookConfig } from '@storybook/react-vite';
import path from 'path';

const config: StorybookConfig = {
  stories: ['../../storybook/**/*.stories.@(ts|tsx)'],
  addons: [
    '@storybook/addon-essentials',
    '@storybook/addon-interactions',
  ],
  framework: {
    name: '@storybook/react-vite',
    options: {},
  },
  viteFinal: async (viteConfig) => {
    const uiNodeModules = path.resolve(__dirname, '../node_modules');
    return {
      ...viteConfig,
      server: {
        ...viteConfig.server,
        fs: {
          allow: ['..', '../..'],
        },
      },
      resolve: {
        ...viteConfig.resolve,
        alias: [
          ...(Array.isArray(viteConfig.resolve?.alias) ? viteConfig.resolve.alias : []),
          { find: 'react', replacement: path.join(uiNodeModules, 'react') },
          { find: 'react-dom', replacement: path.join(uiNodeModules, 'react-dom') },
          { find: 'react/jsx-runtime', replacement: path.join(uiNodeModules, 'react/jsx-runtime') },
        ],
      },
    };
  },
};

export default config;
