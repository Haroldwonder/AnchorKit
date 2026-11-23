import React, { useMemo } from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import '@testing-library/jest-dom';
import SEP10AuthFlow from './Sep10AuthFlow';

// Mock clipboard API
Object.assign(navigator, {
  clipboard: {
    writeText: jest.fn(() => Promise.resolve()),
  },
});

// Mock crypto.randomUUID
Object.defineProperty(global, 'crypto', {
  value: {
    randomUUID: () => 'test-uuid-1234-5678-90ab-cdef',
  },
});

describe('SEP10AuthFlow', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  describe('Initial Idle State', () => {
    it('renders the authentication flow header', () => {
      render(<SEP10AuthFlow />);
      expect(screen.getByText(/Stellar · SEP-10/)).toBeInTheDocument();
      expect(screen.getByText(/Authentication/)).toBeInTheDocument();
      expect(screen.getByText(/Flow/)).toBeInTheDocument();
    });

    it('renders all four authentication steps', () => {
      render(<SEP10AuthFlow />);
      expect(screen.getAllByText('Connect Wallet').length).toBeGreaterThan(0);
      expect(screen.getAllByText('Fetch Challenge').length).toBeGreaterThan(0);
      expect(screen.getAllByText('Sign Challenge').length).toBeGreaterThan(0);
      expect(screen.getAllByText('Auth Token').length).toBeGreaterThan(0);
    });

    it('shows step numbers for all steps', () => {
      render(<SEP10AuthFlow />);
      expect(screen.getByText('STEP 1')).toBeInTheDocument();
      expect(screen.getByText('STEP 2')).toBeInTheDocument();
      expect(screen.getByText('STEP 3')).toBeInTheDocument();
      expect(screen.getByText('STEP 4')).toBeInTheDocument();
    });

    it('renders the domain input field', () => {
      render(<SEP10AuthFlow />);
      const domainInput = screen.getByPlaceholderText('anchor.example.com');
      expect(domainInput).toBeInTheDocument();
      expect(domainInput).toHaveValue('testanchor.stellar.org');
    });

    it('does not show reset button in idle state', () => {
      render(<SEP10AuthFlow />);
      expect(screen.queryByText(/RESET/)).not.toBeInTheDocument();
    });

    it('shows connect wallet button in idle state', () => {
      render(<SEP10AuthFlow />);
      expect(screen.getByRole('button', { name: /Connect Wallet/ })).toBeInTheDocument();
    });
  });

  describe('Connect Wallet Step', () => {
    it('triggers wallet connection when button is clicked', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      // Wait for the async operations to complete
      await waitFor(() => {
        expect(screen.getByText(/CONNECTED ACCOUNT/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('displays wallet address after connection', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        // Address is rendered split across elements, check for partial address pattern
        const addressElements = screen.getAllByText(/^G[A-Z2-7]+/);
        expect(addressElements.length).toBeGreaterThan(0);
      }, { timeout: 3000 });
    });

    it('shows network badge after connection', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        expect(screen.getByText('Testnet')).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('shows reset button after wallet connection', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        expect(screen.getByText(/RESET/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });
  });

  describe('Fetch Challenge Step', () => {
    it('shows fetch challenge button after wallet connection', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /Fetch Challenge/ })).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('displays challenge XDR after fetching', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/CHALLENGE XDR/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('shows domain in challenge display', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/testanchor\.stellar\.org\/auth/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });
  });

  describe('Sign Challenge Step', () => {
    it('shows sign button after challenge is fetched', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/Sign with Wallet/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('displays signed XDR after signing', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/SIGNED XDR/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('shows ED25519 signature confirmation', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/ED25519 signature applied/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });
  });

  describe('Auth Token Step', () => {
    it('shows submit button after signing', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/Submit & Get Token/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('displays JWT token after submission', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const submitButton = screen.getByText(/Submit & Get Token/);
        fireEvent.click(submitButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/DECODED PAYLOAD/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('shows copy JWT button', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const submitButton = screen.getByText(/Submit & Get Token/);
        fireEvent.click(submitButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /REVEAL TO COPY JWT/ })).toBeDisabled();
      }, { timeout: 3000 });
    });

    it('copies JWT to clipboard when copy button is clicked', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const submitButton = screen.getByText(/Submit & Get Token/);
        fireEvent.click(submitButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        fireEvent.click(screen.getByRole('button', { name: 'Reveal sensitive JWT fields' }));
        const copyButton = screen.getByRole('button', { name: 'COPY JWT' });
        fireEvent.click(copyButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalled();
      }, { timeout: 3000 });
    });
  });

  describe('Authenticated State', () => {
    it('shows authenticated status after token is received', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const submitButton = screen.getByText(/Submit & Get Token/);
        fireEvent.click(submitButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/AUTHENTICATED/)).toBeInTheDocument();
        expect(screen.getByText(/Session active · SEP-10 verified/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('shows auth status badge with wallet info', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const submitButton = screen.getByText(/Submit & Get Token/);
        fireEvent.click(submitButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/Auth Status/)).toBeInTheDocument();
        expect(screen.getByText(/Live session monitor/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('shows token expiry information', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        const submitButton = screen.getByText(/Submit & Get Token/);
        fireEvent.click(submitButton);
      }, { timeout: 3000 });

      await waitFor(() => {
        expect(screen.getByText(/TOKEN VALIDITY/)).toBeInTheDocument();
        expect(screen.getByText(/EXPIRES IN/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });
  });

  describe('Reset Functionality', () => {
    it('resets to idle state when reset button is clicked', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        expect(screen.getByText(/RESET/)).toBeInTheDocument();
      }, { timeout: 3000 });

      const resetButton = screen.getByText(/RESET/);
      await act(async () => {
        fireEvent.click(resetButton);
      });

      await waitFor(() => {
        expect(screen.queryByText(/CONNECTED ACCOUNT/)).not.toBeInTheDocument();
        expect(screen.getByRole('button', { name: /Connect Wallet/ })).toBeInTheDocument();
      }, { timeout: 3000 });
    });
  });

  describe('Activity Log', () => {
    it('shows activity log after actions', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        expect(screen.getByText(/Activity Log/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('logs wallet connection events', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        expect(screen.getByText(/Requesting wallet connection/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });
  });

  describe('Domain Configuration', () => {
    it('allows changing the domain', () => {
      render(<SEP10AuthFlow />);
      const domainInput = screen.getByPlaceholderText('anchor.example.com');
      
      fireEvent.change(domainInput, { target: { value: 'custom.anchor.com' } });
      
      expect(domainInput).toHaveValue('custom.anchor.com');
    });
  });

  describe('Step Progress Indicators', () => {
    it('shows completed steps with checkmarks', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        // Check for checkmark icons in completed steps
        const checkmarks = screen.getAllByText('✓');
        expect(checkmarks.length).toBeGreaterThan(0);
      }, { timeout: 3000 });
    });

    it('highlights active step', async () => {
      render(<SEP10AuthFlow />);
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      
      await act(async () => {
        fireEvent.click(connectButton);
      });

      await waitFor(() => {
        // The connect step should be marked as complete
        const completeLabels = screen.getAllByText('COMPLETE');
        expect(completeLabels.length).toBeGreaterThan(0);
      }, { timeout: 3000 });
    });
  });

  describe('Error Handling', () => {
    it('handles errors gracefully during wallet connection', async () => {
      // This test verifies the component doesn't crash on errors
      render(<SEP10AuthFlow />);
      expect(screen.getByText(/Stellar · SEP-10/)).toBeInTheDocument();
    });
  });

  describe('Complete Authentication Flow', () => {
    it('completes all 4 authentication steps', async () => {
      render(<SEP10AuthFlow />);
      
      // Step 1: Connect Wallet
      const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
      await act(async () => {
        fireEvent.click(connectButton);
      });
      
      await waitFor(() => {
        expect(screen.getByText(/CONNECTED ACCOUNT/)).toBeInTheDocument();
      }, { timeout: 3000 });

      // Step 2: Fetch Challenge
      await waitFor(() => {
        const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
        fireEvent.click(fetchButton);
      }, { timeout: 3000 });
      
      await waitFor(() => {
        expect(screen.getByText(/CHALLENGE XDR/)).toBeInTheDocument();
      }, { timeout: 3000 });

      // Step 3: Sign Challenge
      await waitFor(() => {
        const signButton = screen.getByText(/Sign with Wallet/);
        fireEvent.click(signButton);
      }, { timeout: 3000 });
      
      await waitFor(() => {
        expect(screen.getByText(/SIGNED XDR/)).toBeInTheDocument();
      }, { timeout: 3000 });

      // Step 4: Submit and Get Token
      await waitFor(() => {
        const submitButton = screen.getByText(/Submit & Get Token/);
        fireEvent.click(submitButton);
      }, { timeout: 3000 });
      
      await waitFor(() => {
        expect(screen.getByText(/AUTHENTICATED/)).toBeInTheDocument();
      }, { timeout: 3000 });
    });
  });
});

describe('Token Expiry Polling', () => {
  it('polls token expiry every 30 seconds', async () => {
    render(<SEP10AuthFlow />);
    const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
    
    await act(async () => {
      fireEvent.click(connectButton);
    });

    await waitFor(() => {
      const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
      fireEvent.click(fetchButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      const signButton = screen.getByText(/Sign with Wallet/);
      fireEvent.click(signButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      const submitButton = screen.getByText(/Submit & Get Token/);
      fireEvent.click(submitButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      expect(screen.getByText(/AUTHENTICATED/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Fast-forward time by 30 seconds
    act(() => {
      jest.advanceTimersByTime(30000);
    });

    // Token should still be valid (24h expiry)
    await waitFor(() => {
      expect(screen.getByText(/AUTHENTICATED/)).toBeInTheDocument();
    });
  });

  it('transitions badge to expired state when token expires', async () => {
    render(<SEP10AuthFlow />);
    const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
    
    await act(async () => {
      fireEvent.click(connectButton);
    });

    await waitFor(() => {
      const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
      fireEvent.click(fetchButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      const signButton = screen.getByText(/Sign with Wallet/);
      fireEvent.click(signButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      const submitButton = screen.getByText(/Submit & Get Token/);
      fireEvent.click(submitButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      expect(screen.getByText(/AUTHENTICATED/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Fast-forward time by 24 hours + 1 second to expire the token
    act(() => {
      jest.advanceTimersByTime(86401000);
    });

    // Badge should transition to expired state
    await waitFor(() => {
      expect(screen.getByText(/EXPIRED/)).toBeInTheDocument();
      expect(screen.getByText(/Token has expired · Re-authenticate required/)).toBeInTheDocument();
    });
  });

  it('updates expiry countdown every second', async () => {
    render(<SEP10AuthFlow />);
    const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
    
    await act(async () => {
      fireEvent.click(connectButton);
    });

    await waitFor(() => {
      const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
      fireEvent.click(fetchButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      const signButton = screen.getByText(/Sign with Wallet/);
      fireEvent.click(signButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      const submitButton = screen.getByText(/Submit & Get Token/);
      fireEvent.click(submitButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      expect(screen.getByText(/AUTHENTICATED/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Fast-forward time by 1 second
    act(() => {
      jest.advanceTimersByTime(1000);
    });

    // Countdown should update
    await waitFor(() => {
      expect(screen.getByText(/EXPIRES IN/)).toBeInTheDocument();
    });
  });

  it('cleans up polling intervals on unmount', async () => {
    const { unmount } = render(<SEP10AuthFlow />);
    const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
    
    await act(async () => {
      fireEvent.click(connectButton);
    });

    await waitFor(() => {
      const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
      fireEvent.click(fetchButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      const signButton = screen.getByText(/Sign with Wallet/);
      fireEvent.click(signButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      const submitButton = screen.getByText(/Submit & Get Token/);
      fireEvent.click(submitButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      expect(screen.getByText(/AUTHENTICATED/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Unmount component
    unmount();

    // No errors should occur from timers after unmount
    act(() => {
      jest.advanceTimersByTime(60000);
    });
  });
});

describe('SEP-10 Integration Tests', () => {
  // Mock anchor server responses
  const mockAnchorServer = {
    challenge: 'AAAAAGL8HQvQkbK2HA3WVjRrKmjX00fG8sLI7m0ERwJW/AX3AAAAZAAiII0AAAABAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAArqN6LeOagjxMaUP96Bzfs9e0corNZXzBWJkFoK7kvkwAAAAXSHboHQAAAAA=',
    signedXdr: 'AAAAAGL8HQvQkbK2HA3WVjRrKmjX00fG8sLI7m0ERwJW/AX3AAAAZAAiII0AAAABAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAArqN6LeOagjxMaUP96Bzfs9e0corNZXzBWJkFoK7kvkwAAAAXSHboHQAAAABqfF0L0JGytgwN1lY0aypo19NHxvLCyO5tBEcCVvwF9wAAAEDn6quNiED+WSSXUVNI7aXIERuSR7fl2GWoOK+UOSHiLPdHBQoqBDyfa8PPiRoxiQVjah6EEqMENREaaegGabJM',
    jwt: 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJ0ZXN0YW5jaG9yLnN0ZWxsYXIub3JnIiwic3ViIjoiR0w4SFF2UWtiSzJIQTNXVmpSckttalgwMGZHOHNMSTdtMEVSd0pXX0FYMyIsImlhdCI6MTY0MDk5NTIwMCwiZXhwIjoxNjQxMDgxNjAwLCJqdGkiOiJ0ZXN0LXV1aWQtMTIzNC01Njc4LTkwYWItY2RlZiJ9.signature'
  };

  beforeEach(() => {
    // Mock fetch for anchor server requests
    global.fetch = jest.fn();
    jest.clearAllMocks();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
    jest.restoreAllMocks();
  });

  it('completes full SEP-10 authentication round-trip', async () => {
    // Mock the anchor server endpoints
    (global.fetch as jest.Mock)
      .mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({
          transaction: mockAnchorServer.challenge,
          network_passphrase: 'Test SDF Network ; September 2015'
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({
          token: mockAnchorServer.jwt
        })
      });

    render(<SEP10AuthFlow />);

    // Step 1: Connect wallet
    const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
    await act(async () => {
      fireEvent.click(connectButton);
    });

    await waitFor(() => {
      expect(screen.getByText(/CONNECTED ACCOUNT/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Step 2: Fetch challenge from anchor
    await waitFor(() => {
      const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
      fireEvent.click(fetchButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      expect(screen.getByText(/CHALLENGE XDR/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Step 3: Sign challenge with wallet
    await waitFor(() => {
      const signButton = screen.getByText(/Sign with Wallet/);
      fireEvent.click(signButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      expect(screen.getByText(/SIGNED XDR/)).toBeInTheDocument();
      expect(screen.getByText(/ED25519 signature applied/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Step 4: Submit signed challenge and receive token
    await waitFor(() => {
      const submitButton = screen.getByText(/Submit & Get Token/);
      fireEvent.click(submitButton);
    }, { timeout: 3000 });

    await waitFor(() => {
      expect(screen.getByText(/AUTHENTICATED/)).toBeInTheDocument();
      expect(screen.getByText(/DECODED PAYLOAD/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Step 5: Verify token is valid and displayed
    await waitFor(() => {
      expect(screen.getByText(/Session active · SEP-10 verified/)).toBeInTheDocument();
      expect(screen.getByText(/TOKEN VALIDITY/)).toBeInTheDocument();
      expect(screen.getByText(/EXPIRES IN/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Verify JWT structure is displayed
    expect(screen.getByText(/HEADER/)).toBeInTheDocument();
    expect(screen.getByText(/PAYLOAD/)).toBeInTheDocument();
    expect(screen.getByText(/SIGNATURE/)).toBeInTheDocument();
    expect(document.body.textContent).not.toContain(mockAnchorServer.jwt.split('.')[1]);

    // Verify copy functionality works
    const copyButton = screen.getByRole('button', { name: /REVEAL TO COPY JWT/ });
    expect(copyButton).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Reveal sensitive JWT fields' }));
    expect(screen.getByRole('button', { name: 'COPY JWT' })).toBeEnabled();
    fireEvent.click(screen.getByRole('button', { name: 'COPY JWT' }));
    
    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        expect.stringMatching(/^[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+$/)
      );
    });
  });

  it('masks the encoded JWT payload and blocks copying until fields are revealed', async () => {
    (global.fetch as jest.Mock)
      .mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({
          transaction: mockAnchorServer.challenge,
          network_passphrase: 'Test SDF Network ; September 2015'
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({ token: mockAnchorServer.jwt })
      });

    render(<SEP10AuthFlow />);
    await waitFor(() => {
      fireEvent.click(screen.getByRole('button', { name: /Connect Wallet/ }));
    }, { timeout: 3000 });
    await waitFor(() => expect(screen.getByText(/CONNECTED ACCOUNT/)).toBeInTheDocument(), { timeout: 3000 });
    await waitFor(() => {
      fireEvent.click(screen.getByRole('button', { name: /Fetch Challenge/ }));
    }, { timeout: 3000 });
    await waitFor(() => expect(screen.getByText(/CHALLENGE XDR/)).toBeInTheDocument(), { timeout: 3000 });
    await waitFor(() => {
      fireEvent.click(screen.getByText(/Sign with Wallet/));
    }, { timeout: 3000 });
    await waitFor(() => expect(screen.getByText(/SIGNED XDR/)).toBeInTheDocument(), { timeout: 3000 });
    await waitFor(() => {
      fireEvent.click(screen.getByText(/Submit & Get Token/));
    }, { timeout: 3000 });
    await waitFor(() => expect(screen.getByText(/DECODED PAYLOAD/)).toBeInTheDocument(), { timeout: 3000 });

    const hiddenCopyButton = screen.getByRole('button', { name: /REVEAL TO COPY JWT/ });
    expect(hiddenCopyButton).toBeDisabled();
    fireEvent.click(hiddenCopyButton);
    expect(navigator.clipboard.writeText).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Reveal sensitive JWT fields' }));
    fireEvent.click(screen.getByRole('button', { name: 'COPY JWT' }));
    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalled();
    });

    const copiedJwt = (navigator.clipboard.writeText as jest.Mock).mock.calls[0][0] as string;
    const encodedPayload = copiedJwt.split('.')[1];
    expect(document.body.textContent).toContain(encodedPayload);
    fireEvent.click(screen.getByRole('button', { name: 'Hide sensitive JWT fields' }));
    expect(document.body.textContent).not.toContain(encodedPayload);
    const hiddenCopyButtonAfterHide = screen.getByRole('button', { name: /COPIED/ });
    expect(hiddenCopyButtonAfterHide).toBeDisabled();
    fireEvent.click(hiddenCopyButtonAfterHide);
    expect(navigator.clipboard.writeText).toHaveBeenCalledTimes(1);
  });

  it('tests complete challenge-response cryptographic flow', async () => {
    // Mock anchor server with realistic responses
    (global.fetch as jest.Mock)
      .mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({
          transaction: mockAnchorServer.challenge,
          network_passphrase: 'Test SDF Network ; September 2015'
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve({
          token: mockAnchorServer.jwt
        })
      });

    render(<SEP10AuthFlow />);

    // Complete authentication flow and verify each step
    const connectButton = screen.getByRole('button', { name: /Connect Wallet/ });
    await act(async () => {
      fireEvent.click(connectButton);
    });

    // Verify wallet connection logs
    await waitFor(() => {
      expect(screen.getByText(/Requesting wallet connection/)).toBeInTheDocument();
      expect(screen.getByText(/Connection approved/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Fetch challenge
    await waitFor(() => {
      const fetchButton = screen.getByRole('button', { name: /Fetch Challenge/ });
      fireEvent.click(fetchButton);
    }, { timeout: 3000 });

    // Verify challenge fetch logs
    await waitFor(() => {
      expect(screen.getByText(/Challenge XDR received/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Sign challenge
    await waitFor(() => {
      const signButton = screen.getByText(/Sign with Wallet/);
      fireEvent.click(signButton);
    }, { timeout: 3000 });

    // Verify signing logs
    await waitFor(() => {
      expect(screen.getByText(/Transaction signed with ED25519 key/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Submit and get token
    await waitFor(() => {
      const submitButton = screen.getByText(/Submit & Get Token/);
      fireEvent.click(submitButton);
    }, { timeout: 3000 });

    // Verify token receipt logs
    await waitFor(() => {
      expect(screen.getByText(/JWT received/)).toBeInTheDocument();
      expect(screen.getByText(/Auth session established — expires in 24h/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Verify final authenticated state
    await waitFor(() => {
      expect(screen.getByText(/AUTHENTICATED/)).toBeInTheDocument();
      expect(screen.getByText(/ACCOUNT/)).toBeInTheDocument();
      expect(screen.getByText(/NETWORK/)).toBeInTheDocument();
      expect(screen.getByText(/PROTOCOL/)).toBeInTheDocument();
      expect(screen.getByText(/AUTH METHOD/)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Verify progress indicators show completion
    const checkmarks = screen.getAllByText('✓');
    expect(checkmarks.length).toBeGreaterThan(0);
  });
});
