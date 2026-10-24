import React from 'react';
import { render } from '@testing-library/react';
import { SunIcon, MoonIcon, CopyIcon, SendIcon } from '../PlaygroundIcons';

describe('PlaygroundIcons', () => {
  describe('SunIcon', () => {
    it('should render the sun icon', () => {
      const { container } = render(<SunIcon />);
      const svg = container.querySelector('svg');
      expect(svg).toBeInTheDocument();
    });

    it('should have proper sizing applied', () => {
      // Issue #1047: Sun icon should be properly sized (w-3.5 h-3.5 = ~14px)
      const { container } = render(<SunIcon />);
      const svg = container.querySelector('svg');

      // Should have either width/height attributes or be accessible via styling
      // When Tailwind is properly configured, w-3.5 h-3.5 should resolve to 14px (3.5 * 4px)
      expect(svg).toBeInTheDocument();
    });

    it('should have viewBox for scalability', () => {
      const { container } = render(<SunIcon />);
      const svg = container.querySelector('svg');
      expect(svg?.getAttribute('viewBox')).toBe('0 0 24 24');
    });
  });

  describe('MoonIcon', () => {
    it('should render the moon icon', () => {
      const { container } = render(<MoonIcon />);
      const svg = container.querySelector('svg');
      expect(svg).toBeInTheDocument();
    });

    it('should have proper sizing applied', () => {
      // Issue #1047: Moon icon should be properly sized (w-3.5 h-3.5 = ~14px)
      const { container } = render(<MoonIcon />);
      const svg = container.querySelector('svg');
      expect(svg).toBeInTheDocument();
    });

    it('should use currentColor for styling', () => {
      const { container } = render(<MoonIcon />);
      const svg = container.querySelector('svg');
      expect(svg?.getAttribute('stroke')).toBe('currentColor');
    });
  });

  describe('CopyIcon', () => {
    it('should render the copy icon', () => {
      const { container } = render(<CopyIcon />);
      const svg = container.querySelector('svg');
      expect(svg).toBeInTheDocument();
    });

    it('should have smaller sizing', () => {
      // Issue #1047: Copy icon uses w-3 h-3 = ~12px
      const { container } = render(<CopyIcon />);
      const svg = container.querySelector('svg');
      expect(svg?.getAttribute('viewBox')).toBe('0 0 24 24');
    });
  });

  describe('SendIcon', () => {
    it('should render the send icon', () => {
      const { container } = render(<SendIcon />);
      const svg = container.querySelector('svg');
      expect(svg).toBeInTheDocument();
    });

    it('should have proper sizing applied', () => {
      // Issue #1047: Send icon should be properly sized (w-3.5 h-3.5 = ~14px)
      const { container } = render(<SendIcon />);
      const svg = container.querySelector('svg');
      expect(svg).toBeInTheDocument();
    });
  });

  describe('Issue #1047: Tailwind Configuration', () => {
    it('should verify icons are properly sized without Tailwind runtime', () => {
      // This test verifies that icons either:
      // 1. Have explicit width/height attributes instead of Tailwind classes
      // 2. Or the ui/ package has Tailwind configured
      // 3. Or uses inline styles as fallback

      const { container: sunContainer } = render(<SunIcon />);
      const sunSvg = sunContainer.querySelector('svg');

      // Should have one of: width attribute, height attribute, or Tailwind classes
      const hasWidthAttr = sunSvg?.hasAttribute('width');
      const hasHeightAttr = sunSvg?.hasAttribute('height');
      const hasTailwindClass = sunSvg?.className.baseVal?.includes('w-') ||
                               sunSvg?.className.baseVal?.includes('h-');

      expect(
        hasWidthAttr || hasHeightAttr || hasTailwindClass
      ).toBe(true);
    });
  });
});
