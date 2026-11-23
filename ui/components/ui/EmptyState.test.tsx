import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { EmptyState } from './EmptyState';

describe('EmptyState', () => {
  it('renders the heading', () => {
    render(<EmptyState heading="No players found" />);
    expect(screen.getByText('No players found')).toBeInTheDocument();
  });

  it('renders subtext when provided', () => {
    render(
      <EmptyState
        heading="No players found"
        subtext="Try adjusting your region, position, or level filter."
      />,
    );
    expect(
      screen.getByText('Try adjusting your region, position, or level filter.'),
    ).toBeInTheDocument();
  });

  it('does not render subtext when omitted', () => {
    render(<EmptyState heading="No players found" />);
    expect(screen.queryByRole('paragraph')).not.toBeInTheDocument();
  });

  it('renders the icon when provided', () => {
    render(<EmptyState heading="No players found" icon="🔍" />);
    expect(screen.getByText('🔍')).toBeInTheDocument();
  });

  it('does not render the icon wrapper when icon is omitted', () => {
    const { container } = render(<EmptyState heading="No players found" />);
    expect(container.querySelector('.empty-state__icon')).not.toBeInTheDocument();
  });

  it('renders an action element when provided', () => {
    render(
      <EmptyState
        heading="No players found"
        action={<button>Clear Filters</button>}
      />,
    );
    expect(screen.getByRole('button', { name: 'Clear Filters' })).toBeInTheDocument();
  });

  it('does not render the action wrapper when action is omitted', () => {
    const { container } = render(<EmptyState heading="No players found" />);
    expect(container.querySelector('.empty-state__action')).not.toBeInTheDocument();
  });

  it('sets role="status" and aria-label to the heading text', () => {
    render(<EmptyState heading="No players found" />);
    expect(
      screen.getByRole('status', { name: 'No players found' }),
    ).toBeInTheDocument();
  });

  it('renders all props together without error', () => {
    render(
      <EmptyState
        icon="🔍"
        heading="No players found"
        subtext="Try adjusting your region, position, or level filter."
        action={<button>Clear Filters</button>}
      />,
    );
    expect(screen.getByText('🔍')).toBeInTheDocument();
    expect(screen.getByText('No players found')).toBeInTheDocument();
    expect(
      screen.getByText('Try adjusting your region, position, or level filter.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Clear Filters' })).toBeInTheDocument();
  });
});
