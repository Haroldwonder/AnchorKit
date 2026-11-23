import type { Meta, StoryObj } from '@storybook/react';
import { ApiRequestPanel } from '../ui/components/ApiRequestPanel';

const meta = {
  title: 'Components/ApiRequestPanel',
  component: ApiRequestPanel,
  tags: ['autodocs'],
  argTypes: {
    method: {
      control: 'select',
      options: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'],
    },
    endpoint: { control: 'text' },
    isLoading: { control: 'boolean' },
    editable: { control: 'boolean' },
    error: { control: 'text' },
    onSubmit: { action: 'onSubmit' },
  },
  parameters: {
    layout: 'padded',
  },
} satisfies Meta<typeof ApiRequestPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const GetRequest: Story = {
  name: 'GET – stellar.toml discovery',
  args: {
    endpoint: 'https://testanchor.stellar.org/.well-known/stellar.toml',
    method: 'GET',
    headers: { Accept: 'application/json' },
    response: {
      VERSION: '2.0.0',
      NETWORK_PASSPHRASE: 'Test SDF Network ; September 2015',
      CURRENCIES: [
        { code: 'USDC', status: 'live' },
        { code: 'USDT', status: 'live' },
      ],
    },
    isLoading: false,
    editable: false,
  },
};

export const PostRequest: Story = {
  name: 'POST – SEP-10 auth token',
  args: {
    endpoint: 'https://testanchor.stellar.org/auth',
    method: 'POST',
    requestBody: {
      transaction: 'AAAAAgAAAADSbCY8VWFKJ...',
    },
    response: {
      token: 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...',
    },
    headers: { 'Content-Type': 'application/json' },
    isLoading: false,
    editable: true,
    onSubmit: () => {},
  },
};

export const Loading: Story = {
  name: 'Loading state',
  args: {
    endpoint: 'https://testanchor.stellar.org/sep24/transactions',
    method: 'GET',
    isLoading: true,
    editable: false,
  },
};

export const ErrorState: Story = {
  name: 'Error response',
  args: {
    endpoint: 'https://testanchor.stellar.org/sep24/deposit',
    method: 'POST',
    requestBody: { asset_code: 'USDC', account: 'GINVALID' },
    error: '400 Bad Request – Invalid Stellar account address.',
    isLoading: false,
    editable: true,
    onSubmit: () => {},
  },
};

export const EditableBody: Story = {
  name: 'Editable request body',
  args: {
    endpoint: 'https://testanchor.stellar.org/sep31/send',
    method: 'POST',
    requestBody: {
      asset_code: 'USDC',
      asset_issuer: 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5',
      amount: '100.00',
      fields: {
        transaction: { routing_number: '026009593', account_number: '123456789' },
      },
    },
    headers: {
      Authorization: 'Bearer eyJhbGc...',
      'Content-Type': 'application/json',
    },
    isLoading: false,
    editable: true,
    onSubmit: () => {},
  },
};

export const NoResponse: Story = {
  name: 'No response yet',
  args: {
    endpoint: 'https://testanchor.stellar.org/sep24/info',
    method: 'GET',
    isLoading: false,
    editable: false,
  },
};
