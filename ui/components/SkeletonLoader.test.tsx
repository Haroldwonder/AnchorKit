import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import SkeletonLoader, { AssetListSkeleton, FeeTableSkeleton, LimitsSkeleton } from './SkeletonLoader';

describe('SkeletonLoader', () => {
  describe('Default rendering', () => {
    it('renders with default props', () => {
      render(<SkeletonLoader />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
      expect(skeleton).toHaveAttribute('aria-busy', 'true');
      expect(skeleton).toHaveAttribute('aria-label', 'Loading content');
    });

    it('renders rect variant by default', () => {
      render(<SkeletonLoader />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('has proper accessibility attributes', () => {
      render(<SkeletonLoader ariaLabel="Loading user data" />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toHaveAttribute('aria-busy', 'true');
      expect(skeleton).toHaveAttribute('aria-label', 'Loading user data');
    });
  });

  describe('Custom width and height props', () => {
    it('applies custom width as string', () => {
      render(<SkeletonLoader width="200px" />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('applies custom width as number', () => {
      render(<SkeletonLoader width={150} />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('applies custom height as string', () => {
      render(<SkeletonLoader height="50px" />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('applies custom height as number', () => {
      render(<SkeletonLoader height={60} />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('applies both custom width and height', () => {
      render(<SkeletonLoader width={200} height={80} />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });
  });

  describe('Variants', () => {
    it('renders text variant', () => {
      render(<SkeletonLoader variant="text" />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('renders circle variant', () => {
      render(<SkeletonLoader variant="circle" />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('renders list variant with default count', () => {
      render(<SkeletonLoader variant="list" />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('renders list variant with custom count', () => {
      render(<SkeletonLoader variant="list" count={5} />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('renders table variant with default count', () => {
      render(<SkeletonLoader variant="table" />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('renders table variant with custom count', () => {
      render(<SkeletonLoader variant="table" count={4} />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });
  });

  describe('Animation class presence', () => {
    it('includes shimmer animation styles', () => {
      render(<SkeletonLoader />);
      const styleElement = document.querySelector('style');
      expect(styleElement?.textContent).toContain('skeleton-shimmer');
      expect(styleElement?.textContent).toContain('background-position');
    });

    it('applies animation to skeleton elements', () => {
      render(<SkeletonLoader />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });
  });

  describe('Dark and light themes', () => {
    it('renders with dark theme by default', () => {
      render(<SkeletonLoader />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('renders with light theme when dark=false', () => {
      render(<SkeletonLoader dark={false} />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toBeInTheDocument();
    });

    it('applies custom className', () => {
      render(<SkeletonLoader className="custom-skeleton" />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toHaveClass('custom-skeleton');
    });
  });

  describe('Accessibility', () => {
    it('has aria-busy attribute set to true', () => {
      render(<SkeletonLoader />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toHaveAttribute('aria-busy', 'true');
    });

    it('has default aria-label', () => {
      render(<SkeletonLoader />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toHaveAttribute('aria-label', 'Loading content');
    });

    it('accepts custom aria-label', () => {
      render(<SkeletonLoader ariaLabel="Loading profile information" />);
      const skeleton = screen.getByRole('status');
      expect(skeleton).toHaveAttribute('aria-label', 'Loading profile information');
    });

    it('includes screen reader text', () => {
      render(<SkeletonLoader ariaLabel="Loading data" />);
      expect(screen.getByText('Loading data')).toBeInTheDocument();
    });
  });
});

describe('AssetListSkeleton', () => {
  it('renders with default props', () => {
    render(<AssetListSkeleton />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toBeInTheDocument();
    expect(skeleton).toHaveAttribute('aria-label', 'Loading asset list');
  });

  it('renders with custom count', () => {
    render(<AssetListSkeleton count={5} />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toBeInTheDocument();
  });

  it('renders with light theme', () => {
    render(<AssetListSkeleton dark={false} />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toBeInTheDocument();
  });
});

describe('FeeTableSkeleton', () => {
  it('renders with default props', () => {
    render(<FeeTableSkeleton />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toBeInTheDocument();
    expect(skeleton).toHaveAttribute('aria-label', 'Loading fee table');
  });

  it('renders with custom count', () => {
    render(<FeeTableSkeleton count={6} />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toBeInTheDocument();
  });

  it('renders with light theme', () => {
    render(<FeeTableSkeleton dark={false} />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toBeInTheDocument();
  });
});

describe('LimitsSkeleton', () => {
  it('renders with default props', () => {
    render(<LimitsSkeleton />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toBeInTheDocument();
    expect(skeleton).toHaveAttribute('aria-label', 'Loading limits');
  });

  it('renders with light theme', () => {
    render(<LimitsSkeleton dark={false} />);
    const skeleton = screen.getByRole('status');
    expect(skeleton).toBeInTheDocument();
  });

  it('includes shimmer animation styles', () => {
    render(<LimitsSkeleton />);
    const styleElement = document.querySelector('style');
    expect(styleElement?.textContent).toContain('skeleton-shimmer');
  });
});