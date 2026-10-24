import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import { AnchorSelector, AnchorOption } from './AnchorSelector';

const anchors: AnchorOption[] = [
  { id: 'a', name: 'Anchor A', endpoint: 'https://a.example.com', healthScore: 90 },
  { id: 'b', name: 'Anchor B', endpoint: 'https://b.example.com', healthScore: 40 },
  { id: 'c', name: 'Anchor C', endpoint: 'https://c.example.com', healthScore: 70 },
];

describe('AnchorSelector', () => {
  describe('Empty state', () => {
    it('renders a message when the anchors list is empty', () => {
      const onChange = jest.fn();
      render(<AnchorSelector anchors={[]} onChange={onChange} />);
      expect(screen.getByText('No anchors available.')).toBeInTheDocument();
      expect(screen.getByRole('status')).toBeInTheDocument();
      expect(onChange).not.toHaveBeenCalled();
    });
  });

  describe('Eligibility / minHealthScore', () => {
    it('disables anchors below minHealthScore', () => {
      const onChange = jest.fn();
      render(<AnchorSelector anchors={anchors} onChange={onChange} minHealthScore={60} />);

      const optionB = screen.getByText('Anchor B').closest('[role="option"]');
      expect(optionB).toHaveAttribute('aria-disabled', 'true');
      expect(optionB).toHaveAttribute('tabIndex', '-1');
    });

    it('does not select a disabled anchor on click', () => {
      const onChange = jest.fn();
      render(<AnchorSelector anchors={anchors} onChange={onChange} minHealthScore={60} />);

      const optionB = screen.getByText('Anchor B').closest('[role="option"]')!;
      fireEvent.click(optionB);

      expect(optionB).toHaveAttribute('aria-selected', 'false');
      expect(onChange).not.toHaveBeenCalledWith(expect.objectContaining({ id: 'b' }));
    });
  });

  describe('Best badge', () => {
    it('badges the highest-scoring eligible anchor as Best', () => {
      const onChange = jest.fn();
      render(<AnchorSelector anchors={anchors} onChange={onChange} />);

      const optionA = screen.getByText('Anchor A').closest('[role="option"]')!;
      expect(optionA).toHaveTextContent('Best');

      const optionB = screen.getByText('Anchor B').closest('[role="option"]')!;
      const optionC = screen.getByText('Anchor C').closest('[role="option"]')!;
      expect(optionB).not.toHaveTextContent('Best');
      expect(optionC).not.toHaveTextContent('Best');
    });

    it('excludes ineligible anchors from Best consideration', () => {
      const onChange = jest.fn();
      const skewed: AnchorOption[] = [
        { id: 'a', name: 'Anchor A', healthScore: 95 },
        { id: 'b', name: 'Anchor B', healthScore: 65 },
      ];
      render(<AnchorSelector anchors={skewed} onChange={onChange} minHealthScore={100} />);

      // No anchor is eligible, so none should be badged Best.
      expect(screen.queryByText('Best')).not.toBeInTheDocument();
    });
  });

  describe('Auto-selection', () => {
    it('auto-selects the best anchor on mount and calls onChange', () => {
      const onChange = jest.fn();
      render(<AnchorSelector anchors={anchors} onChange={onChange} />);

      expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ id: 'a' }));
      const optionA = screen.getByText('Anchor A').closest('[role="option"]');
      expect(optionA).toHaveAttribute('aria-selected', 'true');
    });

    it('re-selects a new best anchor when the anchors list updates and the user has not manually chosen one', () => {
      const onChange = jest.fn();
      const { rerender } = render(<AnchorSelector anchors={anchors} onChange={onChange} />);
      expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'a' }));

      const updated: AnchorOption[] = [
        { id: 'a', name: 'Anchor A', healthScore: 30 },
        { id: 'b', name: 'Anchor B', healthScore: 40 },
        { id: 'c', name: 'Anchor C', healthScore: 99 },
      ];
      rerender(<AnchorSelector anchors={updated} onChange={onChange} />);

      expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'c' }));
      const optionC = screen.getByText('Anchor C').closest('[role="option"]');
      expect(optionC).toHaveAttribute('aria-selected', 'true');
    });

    it('preserves a manual selection across anchors updates instead of reverting to best', () => {
      const onChange = jest.fn();
      const { rerender } = render(<AnchorSelector anchors={anchors} onChange={onChange} />);

      const optionC = screen.getByText('Anchor C').closest('[role="option"]')!;
      fireEvent.click(optionC);
      expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'c' }));

      // Anchor A's score changes but Anchor C (manually picked) remains eligible.
      const updated: AnchorOption[] = [
        { id: 'a', name: 'Anchor A', healthScore: 99 },
        { id: 'b', name: 'Anchor B', healthScore: 40 },
        { id: 'c', name: 'Anchor C', healthScore: 70 },
      ];
      rerender(<AnchorSelector anchors={updated} onChange={onChange} />);

      expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'c', healthScore: 70 }));
    });

    it('notifies onChange with fresh data when the selected anchor is updated in place', () => {
      const onChange = jest.fn();
      const { rerender } = render(<AnchorSelector anchors={anchors} onChange={onChange} />);
      expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'a', healthScore: 90 }));

      const updated: AnchorOption[] = [
        { id: 'a', name: 'Anchor A (renamed)', healthScore: 92 },
        anchors[1],
        anchors[2],
      ];
      rerender(<AnchorSelector anchors={updated} onChange={onChange} />);

      expect(onChange).toHaveBeenLastCalledWith(
        expect.objectContaining({ id: 'a', name: 'Anchor A (renamed)', healthScore: 92 })
      );
    });
  });

  describe('Keyboard interaction', () => {
    it('selects an anchor on Enter', () => {
      const onChange = jest.fn();
      render(<AnchorSelector anchors={anchors} onChange={onChange} />);

      const optionC = screen.getByText('Anchor C').closest('[role="option"]')!;
      fireEvent.keyDown(optionC, { key: 'Enter' });

      expect(optionC).toHaveAttribute('aria-selected', 'true');
      expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'c' }));
    });

    it('selects an anchor on Space', () => {
      const onChange = jest.fn();
      render(<AnchorSelector anchors={anchors} onChange={onChange} />);

      const optionB = screen.getByText('Anchor B').closest('[role="option"]')!;
      fireEvent.keyDown(optionB, { key: ' ' });

      expect(optionB).toHaveAttribute('aria-selected', 'true');
      expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'b' }));
    });

    it('ignores Enter/Space on a disabled anchor', () => {
      const onChange = jest.fn();
      render(<AnchorSelector anchors={anchors} onChange={onChange} minHealthScore={60} />);

      const optionB = screen.getByText('Anchor B').closest('[role="option"]')!;
      fireEvent.keyDown(optionB, { key: 'Enter' });

      expect(optionB).toHaveAttribute('aria-selected', 'false');
    });
  });

  describe('Controlled selectedId', () => {
    it('respects an explicit selectedId over the auto-selected best', () => {
      const onChange = jest.fn();
      render(<AnchorSelector anchors={anchors} selectedId="c" onChange={onChange} />);

      const optionC = screen.getByText('Anchor C').closest('[role="option"]');
      expect(optionC).toHaveAttribute('aria-selected', 'true');
      expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'c' }));
    });

    it('follows updates to the selectedId prop', () => {
      const onChange = jest.fn();
      const { rerender } = render(<AnchorSelector anchors={anchors} selectedId="a" onChange={onChange} />);
      rerender(<AnchorSelector anchors={anchors} selectedId="b" onChange={onChange} />);

      const optionB = screen.getByText('Anchor B').closest('[role="option"]');
      expect(optionB).toHaveAttribute('aria-selected', 'true');
      expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'b' }));
    });
  });
});
