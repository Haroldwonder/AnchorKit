import type { Meta, StoryObj } from '@storybook/react';
import { AnchorCapabilityCard } from '../ui/components/AnchorCapabilityCard';
import type { SupportedAsset } from '../ui/components/AnchorCapabilityCard';

const usdcAsset: SupportedAsset = {
  code: 'USDC',
  issuer: 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5',
  name: 'USD Coin',
  icon: '💵',
  operationTypes: ['both'],
  depositEnabled: true,
  withdrawalEnabled: true,
  fees: {
    deposit: { type: 'flat', flatAmount: 1.0, currency: 'USD' },
    withdrawal: { type: 'percent', percent: 0.5, currency: 'USD' },
  },
  limits: {
    minDeposit: 10,
    maxDeposit: 50000,
    minWithdrawal: 10,
    maxWithdrawal: 25000,
    dailyLimit: 10000,
    monthlyLimit: 100000,
    currency: 'USD',
  },
  kyc: {
    level: 'basic',
    fields: [
      { name: 'first_name', label: 'First Name', required: true },
      { name: 'last_name', label: 'Last Name', required: true },
      { name: 'email', label: 'Email Address', required: true },
    ],
    estimatedTime: '< 5 minutes',
  },
  countries: ['US', 'CA', 'GB', 'DE', 'AU'],
  networks: ['ACH', 'SEPA', 'WIRE'],
};

const xlmAsset: SupportedAsset = {
  code: 'XLM',
  name: 'Stellar Lumens',
  icon: '⭐',
  operationTypes: ['deposit'],
  depositEnabled: true,
  withdrawalEnabled: false,
  fees: {
    deposit: { type: 'flat', flatAmount: 0, currency: 'XLM' },
  },
  limits: {
    minDeposit: 1,
    maxDeposit: 1000000,
    currency: 'XLM',
  },
  kyc: {
    level: 'none',
    fields: [],
    description: 'No KYC required for XLM deposits.',
  },
  countries: ['US', 'GB', 'DE', 'JP', 'SG', 'AU', 'BR'],
  networks: ['Stellar'],
};

const eurcAsset: SupportedAsset = {
  code: 'EURC',
  name: 'Euro Coin',
  icon: '💶',
  operationTypes: ['both'],
  depositEnabled: true,
  withdrawalEnabled: true,
  fees: {
    deposit: { type: 'flat', flatAmount: 1.5, currency: 'EUR' },
    withdrawal: { type: 'percent', percent: 0.3, currency: 'EUR' },
  },
  limits: {
    minDeposit: 20,
    maxDeposit: 100000,
    minWithdrawal: 20,
    maxWithdrawal: 50000,
    currency: 'EUR',
  },
  kyc: {
    level: 'full',
    fields: [
      { name: 'first_name', label: 'First Name', required: true },
      { name: 'last_name', label: 'Last Name', required: true },
      { name: 'id_type', label: 'ID Type', required: true },
      { name: 'id_number', label: 'ID Number', required: true },
    ],
    documentTypes: ['passport', 'drivers_license', 'national_id'],
    estimatedTime: '1-2 business days',
  },
  countries: ['DE', 'FR', 'ES', 'IT', 'NL', 'BE'],
  networks: ['SEPA', 'SWIFT'],
};

const meta = {
  title: 'Components/AnchorCapabilityCard',
  component: AnchorCapabilityCard,
  tags: ['autodocs'],
  argTypes: {
    anchorName: { control: 'text' },
    domain: { control: 'text' },
    logoInitials: { control: 'text' },
    accentColor: { control: 'color' },
    description: { control: 'text' },
  },
  parameters: {
    layout: 'padded',
  },
} satisfies Meta<typeof AnchorCapabilityCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const MultiAsset: Story = {
  name: 'Multi-asset anchor',
  args: {
    anchorName: 'TestAnchor',
    domain: 'testanchor.stellar.org',
    logoInitials: 'TA',
    accentColor: '#6366f1',
    description: 'A fully-featured testnet anchor supporting USDC, EURC, and XLM with competitive fees and fast settlements.',
    assets: [usdcAsset, eurcAsset, xlmAsset],
  },
};

export const SingleAsset: Story = {
  name: 'Single-asset anchor',
  args: {
    anchorName: 'CryptoRamp',
    domain: 'cryptoramp.example.com',
    logoInitials: 'CR',
    accentColor: '#10b981',
    description: 'Fast USDC on/off ramp via ACH and wire.',
    assets: [usdcAsset],
  },
};

export const NoKYC: Story = {
  name: 'No-KYC anchor (XLM only)',
  args: {
    anchorName: 'QuickLumens',
    domain: 'quicklumens.example.io',
    logoInitials: 'QL',
    accentColor: '#f59e0b',
    assets: [xlmAsset],
  },
};

export const EnhancedKYC: Story = {
  name: 'Enhanced KYC anchor',
  args: {
    anchorName: 'ComplianceAnchor',
    domain: 'compliance.example.com',
    logoInitials: 'CA',
    accentColor: '#8b5cf6',
    description: 'Institutional-grade anchor with enhanced KYC and source-of-funds verification.',
    assets: [
      {
        ...usdcAsset,
        kyc: {
          level: 'enhanced',
          fields: [
            { name: 'first_name', label: 'First Name', required: true },
            { name: 'last_name', label: 'Last Name', required: true },
            { name: 'id_number', label: 'ID Number', required: true },
            { name: 'source_of_funds', label: 'Source of Funds', required: true },
          ],
          documentTypes: ['passport', 'utility_bill', 'bank_statement'],
          estimatedTime: '3-5 business days',
          description: 'Full KYC plus source-of-funds documentation required.',
        },
      },
    ],
  },
};
