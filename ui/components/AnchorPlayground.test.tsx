import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import AnchorPlayground from './AnchorPlayground';

// Mock fetch
global.fetch = jest.fn();

describe('AnchorPlayground', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('Skeleton Loaders', () => {
    it('displays skeleton loaders while data is loading', async () => {
      // Mock a delayed response
      (global.fetch as jest.Mock).mockImplementation(
        () =>
          new Promise((resolve) =>
            setTimeout(
              () =>
                resolve({
                  ok: true,
                  status: 200,
                  headers: new Headers({ 'content-type': 'application/json' }),
                  json: async () => ({ data: 'test' }),
                }),
              1000,
            ),
          ),
      );

      render(<AnchorPlayground />);

      // Click send request button
      const sendButton = screen.getByRole('button', { name: /Send Request/i });
      fireEvent.click(sendButton);

      // Check for skeleton loaders
      await waitFor(() => {
        expect(screen.getByRole('status')).toBeInTheDocument();
        expect(screen.getByRole('status')).toHaveAttribute('aria-busy', 'true');
        expect(screen.getByRole('status')).toHaveAttribute('aria-label', 'Loading anchor data');
      });

      // Check for loading text
      expect(screen.getByText('Loading Assets...')).toBeInTheDocument();
      expect(screen.getByText('Loading Fees...')).toBeInTheDocument();
      expect(screen.getByText('Loading Limits...')).toBeInTheDocument();
    });

    it('has accessible skeleton loader with aria-busy attribute', async () => {
      (global.fetch as jest.Mock).mockImplementation(
        () =>
          new Promise((resolve) =>
            setTimeout(
              () =>
                resolve({
                  ok: true,
                  status: 200,
                  headers: new Headers({ 'content-type': 'application/json' }),
                  json: async () => ({ data: 'test' }),
                }),
              1000,
            ),
          ),
      );

      render(<AnchorPlayground />);

      const sendButton = screen.getByRole('button', { name: /Send Request/i });
      fireEvent.click(sendButton);

      await waitFor(() => {
        const skeleton = screen.getByRole('status');
        expect(skeleton).toHaveAttribute('aria-busy', 'true');
        expect(skeleton).toHaveAttribute('aria-label', 'Loading anchor data');
      });
    });

    it('displays asset list skeleton with 3 items', async () => {
      (global.fetch as jest.Mock).mockImplementation(
        () =>
          new Promise((resolve) =>
            setTimeout(
              () =>
                resolve({
                  ok: true,
                  status: 200,
                  headers: new Headers({ 'content-type': 'application/json' }),
                  json: async () => ({ data: 'test' }),
                }),
              1000,
            ),
          ),
      );

      render(<AnchorPlayground />);

      const sendButton = screen.getByRole('button', { name: /Send Request/i });
      fireEvent.click(sendButton);

      await waitFor(() => {
        expect(screen.getByText('Loading Assets...')).toBeInTheDocument();
      });
    });

    it('displays fee table skeleton', async () => {
      (global.fetch as jest.Mock).mockImplementation(
        () =>
          new Promise((resolve) =>
            setTimeout(
              () =>
                resolve({
                  ok: true,
                  status: 200,
                  headers: new Headers({ 'content-type': 'application/json' }),
                  json: async () => ({ data: 'test' }),
                }),
              1000,
            ),
          ),
      );

      render(<AnchorPlayground />);

      const sendButton = screen.getByRole('button', { name: /Send Request/i });
      fireEvent.click(sendButton);

      await waitFor(() => {
        expect(screen.getByText('Loading Fees...')).toBeInTheDocument();
      });
    });

    it('displays limits skeleton', async () => {
      (global.fetch as jest.Mock).mockImplementation(
        () =>
          new Promise((resolve) =>
            setTimeout(
              () =>
                resolve({
                  ok: true,
                  status: 200,
                  headers: new Headers({ 'content-type': 'application/json' }),
                  json: async () => ({ data: 'test' }),
                }),
              1000,
            ),
          ),
      );

      render(<AnchorPlayground />);

      const sendButton = screen.getByRole('button', { name: /Send Request/i });
      fireEvent.click(sendButton);

      await waitFor(() => {
        expect(screen.getByText('Loading Limits...')).toBeInTheDocument();
      });
    });

    it('hides skeleton loaders when data loads', async () => {
      (global.fetch as jest.Mock).mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ success: true, data: 'test response' }),
      });

      render(<AnchorPlayground />);

      const sendButton = screen.getByRole('button', { name: /Send Request/i });
      fireEvent.click(sendButton);

      // Wait for response to load
      await waitFor(() => {
        expect(screen.queryByText('Loading Assets...')).not.toBeInTheDocument();
        expect(screen.queryByText('Loading Fees...')).not.toBeInTheDocument();
        expect(screen.queryByText('Loading Limits...')).not.toBeInTheDocument();
      });
    });

    it('matches snapshot for skeleton loader state', async () => {
      (global.fetch as jest.Mock).mockImplementation(
        () =>
          new Promise((resolve) =>
            setTimeout(
              () =>
                resolve({
                  ok: true,
                  status: 200,
                  headers: new Headers({ 'content-type': 'application/json' }),
                  json: async () => ({ data: 'test' }),
                }),
              1000,
            ),
          ),
      );

      const { container } = render(<AnchorPlayground />);

      const sendButton = screen.getByRole('button', { name: /Send Request/i });
      fireEvent.click(sendButton);

      await waitFor(() => {
        expect(screen.getByRole('status')).toBeInTheDocument();
      });

      expect(container.querySelector('[role="status"]')).toMatchSnapshot();
    });
  });

  describe('Basic Rendering', () => {
    it('renders the playground header', () => {
      render(<AnchorPlayground />);
      expect(screen.getByText(/Anchor \/\/ Playground/i)).toBeInTheDocument();
    });

    it('renders domain input field', () => {
      render(<AnchorPlayground />);
      const domainInput = screen.getByPlaceholderText('testanchor.stellar.org');
      expect(domainInput).toBeInTheDocument();
    });

    it('renders SEP protocol selector', () => {
      render(<AnchorPlayground />);
      expect(screen.getByText(/SEP Protocol/i)).toBeInTheDocument();
    });

    it('renders dark/light mode toggle', () => {
      render(<AnchorPlayground />);
      const toggleButton = screen.getByRole('button', { name: /Light Mode|Dark Mode/i });
      expect(toggleButton).toBeInTheDocument();
    });
  });

  describe('Request Functionality', () => {
    it('sends request when send button is clicked', async () => {
      (global.fetch as jest.Mock).mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ success: true }),
      });

      render(<AnchorPlayground />);

      const sendButton = screen.getByRole('button', { name: /Send Request/i });
      fireEvent.click(sendButton);

      await waitFor(() => {
        expect(global.fetch).toHaveBeenCalled();
      });
    });

    it('displays response after successful request', async () => {
      (global.fetch as jest.Mock).mockResolvedValue({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({ success: true, message: 'Test response' }),
      });

      render(<AnchorPlayground />);

      const sendButton = screen.getByRole('button', { name: /Send Request/i });
      fireEvent.click(sendButton);

      await waitFor(() => {
        expect(screen.getByText(/200/)).toBeInTheDocument();
      });
    });
  });
});
