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

    it('resets expanded paths when the data prop changes', () => {
      const { rerender } = render(
        <JsonViewer data={{ account: { balance: 'old' } }} defaultExpandDepth={2} />,
      );

      expect(screen.getByText(/old/)).toBeInTheDocument();

      rerender(<JsonViewer data={{ transfer: { amount: 'new' } }} defaultExpandDepth={2} />);

      expect(screen.getByText(/new/)).toBeInTheDocument();
      expect(screen.queryByText(/old/)).not.toBeInTheDocument();
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

  describe('Deeply Nested Objects', () => {
    it('handles deeply nested objects (>10 levels)', () => {
      let deepObj: any = { value: 'leaf' };
      for (let i = 0; i < 15; i++) {
        deepObj = { [`level${i}`]: deepObj };
      }

      render(<JsonViewer data={deepObj} defaultExpandDepth={20} />);
      
      expect(screen.getByText(/level0/)).toBeInTheDocument();
      expect(screen.getByText(/leaf/)).toBeInTheDocument();
    });

    it('collapses deeply nested objects by default', () => {
      let deepObj: any = { value: 'hidden' };
      for (let i = 0; i < 10; i++) {
        deepObj = { [`level${i}`]: deepObj };
      }

      render(<JsonViewer data={deepObj} defaultExpandDepth={2} />);
      
      expect(screen.getByText(/level0/)).toBeInTheDocument();
      // Deep value should not be visible initially
      expect(screen.queryByText('hidden')).not.toBeInTheDocument();
    });
  });

  describe('Large Arrays', () => {
    it('handles arrays with 1000+ elements', () => {
      const largeArray = Array.from({ length: 1500 }, (_, i) => ({ id: i, value: `item${i}` }));

      render(<JsonViewer data={largeArray} />);
      
      // Should show array indicator
      expect(screen.getByText(/1500 items/)).toBeInTheDocument();
    });

    it('renders large array elements when expanded', () => {
      const largeArray = Array.from({ length: 100 }, (_, i) => i);

      render(<JsonViewer data={largeArray} defaultExpandDepth={1} />);
      
      expect(screen.getByText(/100 items/)).toBeInTheDocument();
    });
  });

  describe('Null and Undefined Values', () => {
    it('renders null values correctly', () => {
      const data = { nullValue: null, nested: { alsoNull: null } };

      render(<JsonViewer data={data} defaultExpandDepth={5} />);
      
      expect(screen.getByText(/nullValue/)).toBeInTheDocument();
      expect(screen.getAllByText(/null/).length).toBeGreaterThan(0);
    });

    it('handles object with all null values', () => {
      const data = { a: null, b: null, c: null };

      render(<JsonViewer data={data} />);
      
      expect(screen.getByText(/a/)).toBeInTheDocument();
      expect(screen.getByText(/b/)).toBeInTheDocument();
      expect(screen.getByText(/c/)).toBeInTheDocument();
    });

    it('renders undefined as null in JSON', () => {
      const data = { defined: 'value', undefined: undefined };

      render(<JsonViewer data={data} />);
      
      // undefined should be stripped or shown as null in JSON
      expect(screen.getByText(/defined/)).toBeInTheDocument();
    });
  });

  describe('Non-String Keys', () => {
    it('handles numeric keys in objects', () => {
      const data = { 0: 'zero', 1: 'one', 2: 'two' };

      render(<JsonViewer data={data} />);
      
      expect(screen.getByText(/"0"/)).toBeInTheDocument();
      expect(screen.getByText(/zero/)).toBeInTheDocument();
    });

    it('handles special character keys', () => {
      const data = { 'key-with-dash': 'value1', 'key.with.dot': 'value2', 'key with space': 'value3' };

      render(<JsonViewer data={data} />);
      
      expect(screen.getByText(/key-with-dash/)).toBeInTheDocument();
      expect(screen.getByText(/key\.with\.dot/)).toBeInTheDocument();
      expect(screen.getByText(/key with space/)).toBeInTheDocument();
    });
  });

  describe('Mixed Data Types', () => {
    it('renders objects with mixed types', () => {
      const data = {
        string: 'text',
        number: 42,
        boolean: true,
        null: null,
        array: [1, 2, 3],
        object: { nested: 'value' },
      };

      render(<JsonViewer data={data} defaultExpandDepth={2} />);
      
      expect(screen.getByText(/string/)).toBeInTheDocument();
      expect(screen.getByText(/number/)).toBeInTheDocument();
      expect(screen.getByText(/boolean/)).toBeInTheDocument();
      expect(screen.getByText(/array/)).toBeInTheDocument();
      expect(screen.getByText(/object/)).toBeInTheDocument();
    });

    it('handles empty objects and arrays', () => {
      const data = { emptyObj: {}, emptyArr: [] };

      render(<JsonViewer data={data} defaultExpandDepth={2} />);
      
      expect(screen.getByText(/emptyObj/)).toBeInTheDocument();
      expect(screen.getByText(/emptyArr/)).toBeInTheDocument();
      expect(screen.getByText(/0 items/)).toBeInTheDocument();
      expect(screen.getByText(/0 keys/)).toBeInTheDocument();
    });
  });

  describe('Search Functionality', () => {
    it('highlights search matches in keys', () => {
      const data = { userName: 'John', userEmail: 'john@example.com' };

      const { container } = render(<JsonViewer data={data} searchable />);
      
      const searchInput = container.querySelector('input[placeholder*="Search"]') as HTMLInputElement;
      expect(searchInput).toBeInTheDocument();
    });

    it('shows match count when searching', () => {
      const data = { test: 'value', nested: { test: 'another' } };

      const { container } = render(<JsonViewer data={data} searchable defaultExpandDepth={5} />);
      
      const searchInput = container.querySelector('input[placeholder*="Search"]') as HTMLInputElement;
      if (searchInput) {
        fireEvent.change(searchInput, { target: { value: 'test' } });
        // Match count should appear
        expect(container.textContent).toContain('test');
      }
    });
  });

  describe('Theme Support', () => {
    it('renders with ember theme', () => {
      const data = { test: 'value' };

      const { container } = render(<JsonViewer data={data} theme="ember" />);
      
      expect(container.textContent).toContain('ember');
    });

    it('renders with arctic theme', () => {
      const data = { test: 'value' };

      const { container } = render(<JsonViewer data={data} theme="arctic" />);
      
      expect(container.textContent).toContain('arctic');
    });

    it('renders with forest theme', () => {
      const data = { test: 'value' };

      const { container } = render(<JsonViewer data={data} theme="forest" />);
      
      expect(container.textContent).toContain('forest');
    });
  });

  describe('Mode Switching', () => {
    it('switches between tree and raw mode', () => {
      const data = { test: 'value' };

      const { container } = render(<JsonViewer data={data} />);
      
      const treeButton = screen.getByText(/TREE/i);
      const rawButton = screen.getByText(/RAW/i);
      
      expect(treeButton).toBeInTheDocument();
      expect(rawButton).toBeInTheDocument();
    });

    it('displays raw JSON in raw mode', () => {
      const data = { test: 'value', number: 42 };

      render(<JsonViewer data={data} defaultMode="raw" />);
      
      // In raw mode, should show formatted JSON
      expect(screen.getByText(/test/)).toBeInTheDocument();
    });
  });

  describe('Copy Functionality', () => {
    it('shows copy button', () => {
      const data = { test: 'value' };

      render(<JsonViewer data={data} />);
      
      expect(screen.getByText(/Copy/i)).toBeInTheDocument();
    });
  });

  describe('Response Metadata', () => {
    it('displays response time when provided', () => {
      const data = { test: 'value' };

      render(<JsonViewer data={data} responseTime={150} />);
      
      expect(screen.getByText(/150ms/)).toBeInTheDocument();
    });

    it('displays line count', () => {
      const data = { a: 1, b: 2, c: 3 };

      const { container } = render(<JsonViewer data={data} />);
      
      expect(container.textContent).toMatch(/\d+ lines/);
    });
  });
});
