import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { AnchorCapabilityCard, AnchorCapabilityCardProps, KYCLevel, OperationType } from './AnchorCapabilityCard';

// Mock data for testing
const mockAsset = {
  code: 'USDC',
  name: 'USD Coin',
  icon: '💵',
  operationTypes: ['both'] as OperationType[],
  depositEnabled: true,
  withdrawalEnabled: true,
  fees: {
    deposit: {
      type: 'flat' as const,
      flatAmount: 1.99,
      currency: 'USD'
    },
    withdrawal: {
      type: 'percent' as const,
      percent: 0.5,
      currency: 'USD'
    }
  },
  limits: {
    minDeposit: 10,
    maxDeposit: 10000,
    minWithdrawal: 10,
    maxWithdrawal: 5000,
    dailyLimit: 10000,
    monthlyLimit: 50000,
    currency: 'USD'
  },
  kyc: {
    level: 'basic' as KYCLevel,
    fields: [
      { name: 'first_name', label: 'First Name', required: true },
      { name: 'email', label: 'Email Address', required: true },
      { name: 'phone', label: 'Phone Number', required: false }
    ],
    estimatedTime: '< 2 minutes'
  },
  networks: ['ACH', 'Wire'],
  countries: ['US', 'CA']
};

const mockProps: AnchorCapabilityCardProps = {
  anchorName: 'Test Anchor',
  domain: 'test.stellar.org',
  logoInitials: 'TA',
  accentColor: '#3b82f6',
  description: 'Test anchor for unit testing',
  assets: [mockAsset]
};

describe('AnchorCapabilityCard', () => {
  describe('Rendering with all service types', () => {
    it('renders anchor name and domain', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      expect(screen.getByText('Test Anchor')).toBeInTheDocument();
      expect(screen.getByText('test.stellar.org')).toBeInTheDocument();
    });

    it('renders description when provided', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      expect(screen.getByText('Test anchor for unit testing')).toBeInTheDocument();
    });

    it('renders logo initials', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      expect(screen.getByText('TA')).toBeInTheDocument();
    });

    it('displays asset count', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      expect(screen.getByText('1 asset')).toBeInTheDocument();
    });

    it('renders all supported assets', () => {
      const multiAssetProps = {
        ...mockProps,
        assets: [
          mockAsset,
          {
            ...mockAsset,
            code: 'XLM',
            name: 'Stellar Lumens',
            icon: '⭐'
          }
        ]
      };
      render(<AnchorCapabilityCard {...multiAssetProps} />);
      expect(screen.getByText('USDC')).toBeInTheDocument();
      expect(screen.getByText('XLM')).toBeInTheDocument();
      expect(screen.getByText('2 assets')).toBeInTheDocument();
    });

    it('shows operation badges for deposit and withdrawal', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      expect(screen.getByText('Deposit')).toBeInTheDocument();
      expect(screen.getByText('Withdraw')).toBeInTheDocument();
    });

    it('displays network badges', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      expect(screen.getByText('ACH')).toBeInTheDocument();
      expect(screen.getByText('Wire')).toBeInTheDocument();
    });
  });

  describe('Rendering with no services', () => {
    it('handles empty assets array', () => {
      const emptyProps = { ...mockProps, assets: [] };
      render(<AnchorCapabilityCard {...emptyProps} />);
      expect(screen.getByText('Test Anchor')).toBeInTheDocument();
      expect(screen.getByText('0 assets')).toBeInTheDocument();
    });
  });

  describe('Disabled state', () => {
    it('renders disabled asset correctly', () => {
      const disabledAsset = {
        ...mockAsset,
        depositEnabled: false,
        withdrawalEnabled: false
      };
      const disabledProps = { ...mockProps, assets: [disabledAsset] };
      render(<AnchorCapabilityCard {...disabledProps} />);
      expect(screen.getByText('USDC')).toBeInTheDocument();
    });
  });

  describe('Click handlers', () => {
    it('switches between tabs when clicked', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      
      // Click on Fees tab
      fireEvent.click(screen.getByText('Fees'));
      expect(screen.getByText('Deposit Fees')).toBeInTheDocument();
      
      // Click on Limits tab
      fireEvent.click(screen.getByText('Limits'));
      expect(screen.getByText('All Limits')).toBeInTheDocument();
      
      // Click on KYC tab
      fireEvent.click(screen.getByText('KYC'));
      expect(screen.getByText('Basic KYC')).toBeInTheDocument();
    });

    it('selects different assets when clicked', () => {
      const multiAssetProps = {
        ...mockProps,
        assets: [
          mockAsset,
          {
            ...mockAsset,
            code: 'XLM',
            name: 'Stellar Lumens',
            icon: '⭐'
          }
        ]
      };
      render(<AnchorCapabilityCard {...multiAssetProps} />);
      
      // Click on XLM asset
      fireEvent.click(screen.getByText('XLM'));
      
      // Should switch to fees tab and show XLM data
      expect(screen.getByText('Fees')).toBeInTheDocument();
    });
  });

  describe('Accessibility attributes', () => {
    it('has proper ARIA labels for interactive elements', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      
      // Tab buttons should be accessible
      const assetsTab = screen.getByRole('button', { name: /Assets/i });
      expect(assetsTab).toBeInTheDocument();
      
      const feesTab = screen.getByRole('button', { name: /Fees/i });
      expect(feesTab).toBeInTheDocument();
    });

    it('has proper document type accessibility in KYC section', () => {
      const kycAsset = {
        ...mockAsset,
        kyc: {
          ...mockAsset.kyc,
          documentTypes: ['passport', 'drivers_license']
        }
      };
      const kycProps = { ...mockProps, assets: [kycAsset] };
      render(<AnchorCapabilityCard {...kycProps} />);
      
      // Switch to KYC tab
      fireEvent.click(screen.getByText('KYC'));
      
      // Check for document list accessibility
      expect(screen.getByRole('list', { name: /Required document types/i })).toBeInTheDocument();
    });

    it('provides accessible labels for form elements', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      
      // Asset selection buttons should be accessible
      const assetButton = screen.getByRole('button');
      expect(assetButton).toBeInTheDocument();
    });
  });

  describe('Fee display', () => {
    it('displays flat fees correctly', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      fireEvent.click(screen.getByText('Fees'));
      expect(screen.getByText('USD 1.99')).toBeInTheDocument();
    });

    it('handles flat fees with omitted amounts without crashing', () => {
      const incompleteFeeAsset = {
        ...mockAsset,
        fees: {
          deposit: { type: 'flat' as const, currency: 'USD' },
          withdrawal: { type: 'flat' as const, currency: 'USD' },
        },
      };
      render(<AnchorCapabilityCard {...mockProps} assets={[incompleteFeeAsset]} />);
      fireEvent.click(screen.getByText('Fees'));
      expect(screen.getAllByText('USD 0.00')).toHaveLength(2);
    });

    it('displays percentage fees correctly', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      fireEvent.click(screen.getByText('Fees'));
      expect(screen.getByText('0.50% USD')).toBeInTheDocument();
    });

    it('displays tiered fees correctly', () => {
      const tieredAsset = {
        ...mockAsset,
        fees: {
          deposit: {
            type: 'tiered' as const,
            currency: 'USD',
            tiers: [
              { upTo: 100, fee: '$1.00' },
              { upTo: null, fee: '0.5%' }
            ]
          }
        }
      };
      const tieredProps = { ...mockProps, assets: [tieredAsset] };
      render(<AnchorCapabilityCard {...tieredProps} />);
      
      fireEvent.click(screen.getByText('Fees'));
      expect(screen.getByText('Tiers')).toBeInTheDocument();
      expect(screen.getByText('$1.00')).toBeInTheDocument();
      expect(screen.getByText('0.5%')).toBeInTheDocument();
    });
  });

  describe('Limits display', () => {
    it('displays all limit types', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      fireEvent.click(screen.getByText('Limits'));
      
      expect(screen.getByText('Min Deposit')).toBeInTheDocument();
      expect(screen.getByText('Max Deposit')).toBeInTheDocument();
      expect(screen.getByText('Min Withdrawal')).toBeInTheDocument();
      expect(screen.getByText('Max Withdrawal')).toBeInTheDocument();
      expect(screen.getByText('Daily Limit')).toBeInTheDocument();
      expect(screen.getByText('Monthly Limit')).toBeInTheDocument();
    });

    it('displays deposit range visualization', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      fireEvent.click(screen.getByText('Limits'));
      expect(screen.getByText('Deposit Range')).toBeInTheDocument();
      const depositBar = screen.getByRole('progressbar', { name: 'Deposit limit range' });
      expect(depositBar).toHaveAttribute('aria-valuenow', '0.1');
      expect(depositBar.firstChild).toHaveStyle({ width: '0.1%' });
      const withdrawalBar = screen.getByRole('progressbar', { name: 'Withdrawal limit range' });
      expect(withdrawalBar).toHaveAttribute('aria-valuenow', '0.2');
      expect(withdrawalBar.firstChild).toHaveStyle({ width: '0.2%' });
    });

    it('shows country availability when provided', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      fireEvent.click(screen.getByText('Limits'));
      expect(screen.getByText('Available In')).toBeInTheDocument();
      expect(screen.getByText('US')).toBeInTheDocument();
      expect(screen.getByText('CA')).toBeInTheDocument();
    });
  });

  describe('KYC information', () => {
    it('displays KYC level badge', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      fireEvent.click(screen.getByText('KYC'));
      expect(screen.getByText('Basic KYC')).toBeInTheDocument();
    });

    it('shows required fields', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      fireEvent.click(screen.getByText('KYC'));
      expect(screen.getByText('Required Fields')).toBeInTheDocument();
      expect(screen.getByText('First Name')).toBeInTheDocument();
      expect(screen.getByText('Email Address')).toBeInTheDocument();
    });

    it('shows optional fields', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      fireEvent.click(screen.getByText('KYC'));
      expect(screen.getByText('Optional Fields')).toBeInTheDocument();
      expect(screen.getByText('Phone Number')).toBeInTheDocument();
    });

    it('displays estimated time when provided', () => {
      render(<AnchorCapabilityCard {...mockProps} />);
      fireEvent.click(screen.getByText('KYC'));
      expect(screen.getByText('Estimated: < 2 minutes')).toBeInTheDocument();
    });

    it('handles no KYC level correctly', () => {
      const noKycAsset = {
        ...mockAsset,
        kyc: {
          level: 'none' as KYCLevel,
          fields: []
        }
      };
      const noKycProps = { ...mockProps, assets: [noKycAsset] };
      render(<AnchorCapabilityCard {...noKycProps} />);
      
      fireEvent.click(screen.getByText('KYC'));
      expect(screen.getByText('No verification needed')).toBeInTheDocument();
    });
  });

  describe('Service health status', () => {
    it('does not display stale health for a disabled service', () => {
      render(
        <AnchorCapabilityCard
          {...mockProps}
          services={{ deposits: true, kyc: false }}
          healthStatus={{ deposits: 'degraded', kyc: 'unavailable' }}
        />,
      );

      expect(screen.getByText('◐ Deposits')).toBeInTheDocument();
      expect(screen.queryByText('○ Kyc')).not.toBeInTheDocument();
    });
  });

  describe('Theme integration', () => {
    it('applies custom accent color', () => {
      render(<AnchorCapabilityCard {...mockProps} accentColor="#ff0000" />);
      expect(screen.getByText('Test Anchor')).toBeInTheDocument();
    });

    it('works without accent color (uses default)', () => {
      const { accentColor, ...propsWithoutColor } = mockProps;
      render(<AnchorCapabilityCard {...propsWithoutColor} />);
      expect(screen.getByText('Test Anchor')).toBeInTheDocument();
    });
  });

  describe('Edge cases', () => {
    it('handles missing optional props gracefully', () => {
      const minimalProps = {
        anchorName: 'Minimal Anchor',
        domain: 'minimal.stellar.org',
        assets: [mockAsset]
      };
      render(<AnchorCapabilityCard {...minimalProps} />);
      expect(screen.getByText('Minimal Anchor')).toBeInTheDocument();
    });

    it('handles assets without icons', () => {
      const noIconAsset = { ...mockAsset, icon: undefined };
      const noIconProps = { ...mockProps, assets: [noIconAsset] };
      render(<AnchorCapabilityCard {...noIconProps} />);
      expect(screen.getByText('USDC')).toBeInTheDocument();
    });

    it('handles assets without networks', () => {
      const noNetworkAsset = { ...mockAsset, networks: undefined };
      const noNetworkProps = { ...mockProps, assets: [noNetworkAsset] };
      render(<AnchorCapabilityCard {...noNetworkProps} />);
      expect(screen.getByText('USDC')).toBeInTheDocument();
    });

    it('handles assets without countries', () => {
      const noCountryAsset = { ...mockAsset, countries: undefined };
      const noCountryProps = { ...mockProps, assets: [noCountryAsset] };
      render(<AnchorCapabilityCard {...noCountryProps} />);
      fireEvent.click(screen.getByText('Limits'));
      expect(screen.getByText('All Limits')).toBeInTheDocument();
    });
  });
});
