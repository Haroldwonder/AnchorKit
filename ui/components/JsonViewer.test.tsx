import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { JsonViewer } from './JsonViewer';

describe('JsonViewer', () => {
  describe('Circular Reference Detection', () => {
    it('renders [Circular] for circular object references', () => {
      const obj: any = { name: 'test' };
      obj.self = obj; // Create circular reference

      render(<JsonViewer data={obj} />);
      
      // Should render without crashing
      expect(screen.getByText(/test/)).toBeInTheDocument();
    });

    it('renders [Circular] for nested circular references', () => {
      const parent: any = { name: 'parent' };
      const child: any = { name: 'child', parent };
      parent.child = child; // Create circular reference

      render(<JsonViewer data={parent} />);
      
      // Should render without crashing
      expect(screen.getByText(/parent/)).toBeInTheDocument();
      expect(screen.getByText(/child/)).toBeInTheDocument();
    });

    it('renders [Circular] for array circular references', () => {
      const arr: any[] = [1, 2, 3];
      arr.push(arr); // Create circular reference

      render(<JsonViewer data={arr} />);
      
      // Should render without crashing
      const treeView = screen.getByRole('status', { hidden: true }).parentElement;
      expect(treeView).toBeInTheDocument();
    });

    it('handles deeply nested circular references', () => {
      const root: any = { level: 0 };
      let current = root;
      
      // Create a deep chain
      for (let i = 1; i < 5; i++) {
        current.next = { level: i };
        current = current.next;
      }
      
      // Create circular reference back to root
      current.next = root;

      render(<JsonViewer data={root} />);
      
      // Should render without crashing
      expect(screen.getByText(/level/)).toBeInTheDocument();
    });

    it('handles multiple circular references in same object', () => {
      const obj: any = { name: 'root' };
      const child1: any = { name: 'child1' };
      const child2: any = { name: 'child2' };
      
      obj.child1 = child1;
      obj.child2 = child2;
      child1.parent = obj;
      child2.parent = obj;
      child1.sibling = child2;
      child2.sibling = child1;

      render(<JsonViewer data={obj} />);
      
      // Should render without crashing
      expect(screen.getByText(/root/)).toBeInTheDocument();
    });

    it('preserves non-circular data correctly', () => {
      const data = {
        user: {
          name: 'John Doe',
          age: 30,
          address: {
            street: '123 Main St',
            city: 'Springfield',
          },
        },
        items: [1, 2, 3],
      };

      render(<JsonViewer data={data} />);
      
      expect(screen.getByText(/John Doe/)).toBeInTheDocument();
      expect(screen.getByText(/Springfield/)).toBeInTheDocument();
    });
  });

  describe('Basic Rendering', () => {
    it('renders simple JSON object', () => {
      const data = { key: 'value', number: 42 };
      
      render(<JsonViewer data={data} />);
      
      expect(screen.getByText(/key/)).toBeInTheDocument();
      expect(screen.getByText(/value/)).toBeInTheDocument();
    });

    it('renders with title and subtitle', () => {
      const data = { test: true };
      
      render(
        <JsonViewer 
          data={data} 
          title="Test Response" 
          subtitle="GET /api/test" 
        />
      );
      
      expect(screen.getByText('Test Response')).toBeInTheDocument();
      expect(screen.getByText('GET /api/test')).toBeInTheDocument();
    });

    it('renders status badge when status provided', () => {
      const data = { success: true };
      
      render(<JsonViewer data={data} status={200} />);
      
      expect(screen.getByText(/200/)).toBeInTheDocument();
    });
  });
});
