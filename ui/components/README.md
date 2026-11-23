# API Request Panel Component

A reusable React component for displaying API requests and responses in AnchorKit applications.

## Features

- ✅ Display endpoint with HTTP method badge
- ✅ Show formatted request body (JSON)
- ✅ Display response with loading and error states
- ✅ Generate and display cURL command
- ✅ Copy to clipboard functionality for all sections
- ✅ Skeleton loader for async operations
- ✅ Error handling with visual feedback
- ✅ Dark mode support
- ✅ Responsive design
- ✅ Follows AnchorKit 8pt grid system

## Installation

```bash
# Copy the component files to your project
cp ui/components/ApiRequestPanel.tsx src/components/
cp ui/components/ApiRequestPanel.css src/components/
```

## Usage

### Basic Example

```tsx
import { ApiRequestPanel } from './components/ApiRequestPanel';

function MyComponent() {
  return (
    <ApiRequestPanel
      endpoint="https://api.anchorkit.stellar.org/v1/attestations"
      method="POST"
      requestBody={{
        issuer: 'GANCHOR123...',
        subject: 'GUSER456...',
        timestamp: 1708819200,
      }}
      response={{
        success: true,
        attestation_id: 'att_123456',
      }}
    />
  );
}
```

### With Loading State

```tsx
function SubmitAttestation() {
  const [response, setResponse] = useState(null);
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async () => {
    setIsLoading(true);
    const result = await contract.submit_attestation(...);
    setResponse(result);
    setIsLoading(false);
  };

  return (
    <>
      <button onClick={handleSubmit}>Submit</button>
      <ApiRequestPanel
        endpoint="https://api.anchorkit.stellar.org/v1/attestations"
        method="POST"
        requestBody={{ /* ... */ }}
        response={response}
        isLoading={isLoading}
      />
    </>
  );
}
```

### With Error Handling

```tsx
function ApiCallWithError() {
  const [error, setError] = useState<string>();

  const makeRequest = async () => {
    try {
      const result = await fetch(...);
      // handle success
    } catch (err) {
      setError(err.message);
    }
  };

  return (
    <ApiRequestPanel
      endpoint="https://api.anchorkit.stellar.org/v1/endpoint"
      method="POST"
      requestBody={{ /* ... */ }}
      error={error}
    />
  );
}
```

## Props

| Prop | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `endpoint` | `string` | ✅ | - | The API endpoint URL |
| `method` | `'GET' \| 'POST' \| 'PUT' \| 'DELETE' \| 'PATCH'` | ❌ | `'POST'` | HTTP method |
| `requestBody` | `Record<string, any> \| string` | ❌ | - | Request payload |
| `response` | `Record<string, any> \| string` | ❌ | - | API response data |
| `headers` | `Record<string, string>` | ❌ | `{}` | HTTP headers |
| `isLoading` | `boolean` | ❌ | `false` | Loading state |
| `error` | `string` | ❌ | - | Error message |

## Features in Detail

### 1. Method Badges

HTTP methods are displayed with color-coded badges:
- **GET**: Blue
- **POST**: Green
- **PUT**: Yellow
- **DELETE**: Red
- **PATCH**: Purple

### 2. Copy to Clipboard

Each section has a copy button:
- Endpoint URL
- Request body (formatted JSON)
- Response (formatted JSON)
- cURL command (complete with headers and body)

Visual feedback shows when content is copied (✓ checkmark for 2 seconds).

### 3. cURL Generation

Automatically generates a complete cURL command including:
- HTTP method
- Endpoint URL
- All headers
- Request body (for POST/PUT/PATCH)

Example output:
```bash
curl -X POST \
  "https://api.anchorkit.stellar.org/v1/attestations" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
  "issuer": "GANCHOR123...",
  "subject": "GUSER456..."
}'
```

### 4. Loading States

Uses skeleton loaders that match AnchorKit's design system:
- Animated gradient effect
- Multiple lines with varying widths
- Smooth transitions

### 5. Error Handling

Displays errors with:
- Warning icon (⚠️)
- Red background
- Clear error message
- Accessible color contrast

### 6. Responsive Design

- Mobile-friendly layout
- Stacks elements vertically on small screens
- Touch-friendly button sizes
- Horizontal scrolling for long code blocks

### 7. Dark Mode

Automatically adapts to system preferences:
- Dark backgrounds
- Adjusted text colors
- Maintained contrast ratios
- Smooth transitions

## Styling

The component uses CSS custom properties for easy theming. Override these in your global styles:

```css
.api-request-panel {
  --primary-color: #3b82f6;
  --success-color: #10b981;
  --error-color: #ef4444;
  --border-color: #e5e7eb;
  --background: #ffffff;
}
```

## Integration with AnchorKit

### With Skeleton Loaders

```tsx
import { ApiRequestPanel } from './components/ApiRequestPanel';

function AnchorOperation({ anchorAddress }) {
  const [skeleton, setSkeleton] = useState(null);
  const [response, setResponse] = useState(null);

  useEffect(() => {
    async function load() {
      const skel = await contract.get_anchor_info_skeleton(anchorAddress);
      setSkeleton(skel);
      
      if (!skel.is_loading && !skel.has_error) {
        const data = await contract.get_anchor_metadata(anchorAddress);
        setResponse(data);
      }
    }
    load();
  }, [anchorAddress]);

  return (
    <ApiRequestPanel
      endpoint={`https://api.anchorkit.stellar.org/v1/anchors/${anchorAddress}`}
      method="GET"
      response={response}
      isLoading={skeleton?.is_loading}
      error={skeleton?.error_message}
    />
  );
}
```

### With Session Tracking

```tsx
function SessionOperation({ sessionId }) {
  const [response, setResponse] = useState(null);

  const submitWithSession = async () => {
    const result = await contract.submit_attestation_with_session(
      sessionId,
      issuer,
      subject,
      timestamp,
      payloadHash,
      signature
    );
    setResponse(result);
  };

  return (
    <ApiRequestPanel
      endpoint="https://api.anchorkit.stellar.org/v1/attestations"
      method="POST"
      requestBody={{
        session_id: sessionId,
        issuer,
        subject,
        timestamp,
      }}
      response={response}
      headers={{
        'X-Session-ID': sessionId,
      }}
    />
  );
}
```

## Accessibility

- Semantic HTML structure
- ARIA labels on interactive elements
- Keyboard navigation support
- Screen reader friendly
- High contrast mode support

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Mobile browsers (iOS Safari, Chrome Mobile)

## Performance

- Minimal re-renders with React.memo
- Efficient clipboard API usage
- Lazy loading of large responses
- Optimized animations

## Testing

```tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { ApiRequestPanel } from './ApiRequestPanel';

test('displays endpoint', () => {
  render(
    <ApiRequestPanel
      endpoint="https://api.example.com/test"
      method="GET"
    />
  );
  expect(screen.getByText('https://api.example.com/test')).toBeInTheDocument();
});

test('copies cURL to clipboard', async () => {
  const writeText = jest.fn();
  Object.assign(navigator, {
    clipboard: { writeText },
  });

  render(
    <ApiRequestPanel
      endpoint="https://api.example.com/test"
      method="POST"
    />
  );

  const copyButton = screen.getAllByRole('button')[3]; // cURL copy button
  fireEvent.click(copyButton);

  expect(writeText).toHaveBeenCalled();
});
```

## Contributing

When contributing to this component:

1. Follow the AnchorKit design system (8pt grid)
2. Maintain accessibility standards
3. Add tests for new features
4. Update documentation
5. Ensure responsive design works

## License

Part of the AnchorKit project.
