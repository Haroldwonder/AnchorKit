import type { Meta, StoryObj } from '@storybook/react';
import { JsonViewer } from '../ui/components/JsonViewer';
import type { ViewerTheme, ViewerMode } from '../ui/components/JsonViewer';

const sampleSep1Response = {
  VERSION: '2.0.0',
  NETWORK_PASSPHRASE: 'Test SDF Network ; September 2015',
  ACCOUNTS: ['GCZJM35NKGVK47BB4SPBDV25477PZYIYPVVG453LPYFNXLS3FGHDXOCM'],
  DOCUMENTATION: {
    ORG_NAME: 'TestAnchor',
    ORG_URL: 'https://testanchor.stellar.org',
    ORG_LOGO: 'https://testanchor.stellar.org/logo.png',
  },
  CURRENCIES: [
    { code: 'USDC', issuer: 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5', status: 'live' },
    { code: 'USDT', issuer: 'GCQTGZQQ5G4PTM2GL7CDIFKUBIPEC52BROAQIAPW53XBRJVN6ZJVTG6V', status: 'live' },
  ],
};

const sampleErrorResponse = {
  error: 'Not Found',
  status: 404,
  message: 'The requested resource could not be found.',
  request_id: 'req_8a2bfc391e4d',
};

const meta = {
  title: 'Components/JsonViewer',
  component: JsonViewer,
  tags: ['autodocs'],
  argTypes: {
    theme: {
      control: 'radio',
      options: ['ember', 'arctic', 'forest'] as ViewerTheme[],
    },
    defaultMode: {
      control: 'radio',
      options: ['tree', 'raw'] as ViewerMode[],
    },
    status: { control: { type: 'number', min: 100, max: 599 } },
    responseTime: { control: { type: 'number', min: 0, max: 5000 } },
    defaultExpandDepth: { control: { type: 'number', min: 0, max: 5 } },
    searchable: { control: 'boolean' },
    title: { control: 'text' },
    subtitle: { control: 'text' },
  },
  parameters: {
    layout: 'padded',
    backgrounds: { default: 'dark' },
  },
} satisfies Meta<typeof JsonViewer>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Ember: Story = {
  args: {
    data: sampleSep1Response,
    title: 'GET /stellar.toml',
    subtitle: 'testanchor.stellar.org',
    status: 200,
    responseTime: 142,
    theme: 'ember',
    defaultMode: 'tree',
    defaultExpandDepth: 2,
    searchable: true,
  },
};

export const Arctic: Story = {
  args: {
    data: sampleSep1Response,
    title: 'GET /stellar.toml',
    status: 200,
    responseTime: 98,
    theme: 'arctic',
    defaultMode: 'tree',
    defaultExpandDepth: 2,
    searchable: true,
  },
};

export const Forest: Story = {
  args: {
    data: sampleSep1Response,
    title: 'GET /stellar.toml',
    status: 200,
    responseTime: 210,
    theme: 'forest',
    defaultMode: 'tree',
    searchable: true,
  },
};

export const ErrorResponse: Story = {
  name: '404 Error Response',
  args: {
    data: sampleErrorResponse,
    title: 'GET /info',
    status: 404,
    responseTime: 55,
    theme: 'ember',
    defaultMode: 'tree',
    searchable: false,
  },
};

export const RawMode: Story = {
  args: {
    data: sampleSep1Response,
    title: 'GET /stellar.toml',
    status: 200,
    responseTime: 142,
    theme: 'ember',
    defaultMode: 'raw',
    searchable: true,
  },
};

export const DeepNested: Story = {
  name: 'Deep Nested Object',
  args: {
    data: {
      level1: {
        level2: {
          level3: {
            level4: { value: 'deep', items: [1, 2, 3] },
          },
        },
      },
      array: [{ a: 1 }, { b: 2 }, { c: 3 }],
      primitive: 42,
      flag: true,
      empty: null,
    },
    title: 'Nested data',
    theme: 'arctic',
    defaultExpandDepth: 3,
    searchable: true,
  },
};
