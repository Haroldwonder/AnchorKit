import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { ApiRequestPanel } from './ApiRequestPanel';

describe('ApiRequestPanel', () => {
  const mockEndpoint = 'https://api.anchorkit.stellar.org/v1/test';
  
  beforeEach(() => {
    // Mock clipboard API
    Object.assign(navigator, {
      clipboard: {
        writeText: jest.fn(() => Promise.resolve()),
      },
    });
  });

  describe('Endpoint Display', () => {
    it('renders endpoint URL', () => {
      render(<ApiRequestPanel endpoint={mockEndpoint} />);
      expect(screen.getByText(mockEndpoint)).toBeInTheDocument();
    });

    it('displays correct method badge', () => {
      const { rerender } = render(
        <ApiRequestPanel endpoint={mockEndpoint} method="GET" />
      );
      expect(screen.getByText('GET')).toBeInTheDocument();

      rerender(<ApiRequestPanel endpoint={mockEndpoint} method="POST" />);
      expect(screen.getByText('POST')).toBeInTheDocument();
    });

    it('defaults to POST method', () => {
      render(<ApiRequestPanel endpoint={mockEndpoint} />);
      expect(screen.getByText('POST')).toBeInTheDocument();
    });
  });

  describe('Request Body', () => {
    it('renders request body when provided', () => {
      const requestBody = { test: 'data', value: 123 };
      render(
        <ApiRequestPanel endpoint={mockEndpoint} requestBody={requestBody} />
      );
      // JSON is rendered in a code block; check the container text
      const pre = document.querySelector('pre');
      expect(pre?.textContent).toContain('test');
      expect(pre?.textContent).toContain('data');
    });

    it('formats JSON request body', () => {
      const requestBody = { key: 'value' };
      render(
        <ApiRequestPanel endpoint={mockEndpoint} requestBody={requestBody} />
      );
      const pre = document.querySelector('pre');
      expect(pre?.textContent).toContain('key');
      expect(pre?.textContent).toContain('value');
    });

    it('handles string request body', () => {
      const requestBody = 'plain text body';
      render(
        <ApiRequestPanel endpoint={mockEndpoint} requestBody={requestBody} />
      );
      expect(screen.getByText(requestBody)).toBeInTheDocument();
    });

    it('does not render request section when no body provided', () => {
      render(<ApiRequestPanel endpoint={mockEndpoint} />);
      expect(screen.queryByText('Request Body')).not.toBeInTheDocument();
    });
  });

  describe('Response Display', () => {
    it('renders response when provided', () => {
      const response = { success: true, id: '123' };
      render(
        <ApiRequestPanel endpoint={mockEndpoint} response={response} />
      );
      expect(screen.getByText(/success/)).toBeInTheDocument();
    });

    it('shows loading state', () => {
      render(<ApiRequestPanel endpoint={mockEndpoint} isLoading={true} />);
      expect(screen.getByText('Response')).toBeInTheDocument();
      const skeletonLines = document.querySelectorAll('.skeleton-line');
      expect(skeletonLines.length).toBeGreaterThan(0);
    });

    it('shows error state', () => {
      const errorMessage = 'Network error occurred';
      render(
        <ApiRequestPanel endpoint={mockEndpoint} error={errorMessage} />
      );
      expect(screen.getByText(errorMessage)).toBeInTheDocument();
      expect(screen.getByText('⚠️')).toBeInTheDocument();
    });

    it('shows empty state when no response', () => {
      render(<ApiRequestPanel endpoint={mockEndpoint} />);
      expect(screen.getByText('No response yet')).toBeInTheDocument();
    });
  });

  describe('cURL Generation', () => {
    it('generates basic cURL command', () => {
      render(
        <ApiRequestPanel endpoint={mockEndpoint} method="GET" />
      );
      expect(screen.getByText(/curl -X GET/)).toBeInTheDocument();
    });

    it('includes headers in cURL', () => {
      const headers = {
        'Content-Type': 'application/json',
        'Authorization': 'Bearer token123',
      };
      render(
        <ApiRequestPanel
          endpoint={mockEndpoint}
          method="POST"
          headers={headers}
        />
      );
      expect(screen.getByText(/Content-Type: application\/json/)).toBeInTheDocument();
      expect(screen.getByText(/Authorization: Bearer token123/)).toBeInTheDocument();
    });

    it('includes request body in cURL for POST', () => {
      const requestBody = { test: 'data' };
      render(
        <ApiRequestPanel
          endpoint={mockEndpoint}
          method="POST"
          requestBody={requestBody}
        />
      );
      expect(screen.getByText(/-d/)).toBeInTheDocument();
    });

    it('excludes request body in cURL for GET', () => {
      const requestBody = { test: 'data' };
      const { container } = render(
        <ApiRequestPanel
          endpoint={mockEndpoint}
          method="GET"
          requestBody={requestBody}
        />
      );
      const curlSection = container.querySelector('.curl-section');
      expect(curlSection?.textContent).not.toContain('-d');
    });
  });

  describe('Copy to Clipboard', () => {
    it('copies endpoint to clipboard', async () => {
      render(<ApiRequestPanel endpoint={mockEndpoint} />);
      const copyButtons = screen.getAllByRole('button');
      
      fireEvent.click(copyButtons[0]); // First copy button (endpoint)
      
      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith(mockEndpoint);
      });
    });

    it('copies request body to clipboard', async () => {
      const requestBody = { test: 'data' };
      render(
        <ApiRequestPanel endpoint={mockEndpoint} requestBody={requestBody} />
      );
      
      const requestCopyButton = screen.getAllByRole('button')[1];
      fireEvent.click(requestCopyButton);
      
      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
          JSON.stringify(requestBody, null, 2)
        );
      });
    });

    it('copies response to clipboard', async () => {
      const response = { success: true };
      render(
        <ApiRequestPanel endpoint={mockEndpoint} response={response} />
      );
      
      const responseCopyButton = screen.getAllByRole('button')[1];
      fireEvent.click(responseCopyButton);
      
      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
          JSON.stringify(response, null, 2)
        );
      });
    });

    it('copies cURL command to clipboard', async () => {
      render(
        <ApiRequestPanel endpoint={mockEndpoint} method="POST" />
      );
      
      const curlCopyButton = screen.getAllByRole('button').pop();
      fireEvent.click(curlCopyButton!);
      
      await waitFor(() => {
        expect(navigator.clipboard.writeText).toHaveBeenCalled();
        const callArg = (navigator.clipboard.writeText as jest.Mock).mock.calls[0][0];
        expect(callArg).toContain('curl -X POST');
      });
    });

    it('shows checkmark after successful copy', async () => {
      render(<ApiRequestPanel endpoint={mockEndpoint} />);
      const copyButton = screen.getAllByRole('button')[0];
      
      fireEvent.click(copyButton);
      
      await waitFor(() => {
        expect(screen.getByText('✓')).toBeInTheDocument();
      });
    });

    it('resets checkmark after 2 seconds', async () => {
      jest.useFakeTimers();
      render(<ApiRequestPanel endpoint={mockEndpoint} />);
      const copyButton = screen.getAllByRole('button')[0];
      
      fireEvent.click(copyButton);
      
      await waitFor(() => {
        expect(screen.getByText('✓')).toBeInTheDocument();
      });
      
      jest.advanceTimersByTime(2000);
      
      await waitFor(() => {
        expect(screen.queryByText('✓')).not.toBeInTheDocument();
      });
      
      jest.useRealTimers();
    });
  });

  describe('HTTP Methods', () => {
    const methods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'] as const;

    methods.forEach(method => {
      it(`renders ${method} method correctly`, () => {
        render(<ApiRequestPanel endpoint={mockEndpoint} method={method} />);
        const badge = screen.getByText(method);
        expect(badge).toBeInTheDocument();
        expect(badge).toHaveAttribute('data-method', method);
      });
    });
  });

  describe('Accessibility', () => {
    it('has proper button titles', () => {
      render(<ApiRequestPanel endpoint={mockEndpoint} />);
      const buttons = screen.getAllByRole('button');
      buttons.forEach(button => {
        expect(button).toHaveAttribute('title');
      });
    });

    it('uses semantic HTML', () => {
      const { container } = render(
        <ApiRequestPanel
          endpoint={mockEndpoint}
          requestBody={{ test: 'data' }}
          response={{ result: 'success' }}
        />
      );
      
      expect(container.querySelector('code')).toBeInTheDocument();
      expect(container.querySelector('pre')).toBeInTheDocument();
    });
  });

  describe('Edge Cases', () => {
    it('handles empty headers object', () => {
      render(
        <ApiRequestPanel endpoint={mockEndpoint} headers={{}} />
      );
      expect(screen.getByText(mockEndpoint)).toBeInTheDocument();
    });

    it('handles null response', () => {
      render(
        <ApiRequestPanel endpoint={mockEndpoint} response={null as any} />
      );
      expect(screen.getByText('No response yet')).toBeInTheDocument();
    });

    it('handles undefined error', () => {
      render(
        <ApiRequestPanel endpoint={mockEndpoint} error={undefined} />
      );
      expect(screen.queryByText('⚠️')).not.toBeInTheDocument();
    });

    it('handles very long endpoint URLs', () => {
      const longEndpoint = 'https://api.example.com/' + 'a'.repeat(200);
      render(<ApiRequestPanel endpoint={longEndpoint} />);
      expect(screen.getByText(longEndpoint)).toBeInTheDocument();
    });

    it('handles complex nested JSON', () => {
      const complexBody = {
        level1: {
          level2: {
            level3: {
              data: [1, 2, 3],
              nested: { key: 'value' },
            },
          },
        },
      };
      render(
        <ApiRequestPanel endpoint={mockEndpoint} requestBody={complexBody} />
      );
      const pre = document.querySelector('pre');
      expect(pre?.textContent).toContain('level1');
    });
  });
});
