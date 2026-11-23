# ApiRequestPanel - Usage Guide

## Overview

The `ApiRequestPanel` component displays API request/response information with optional editable JSON validation.

## Props

```typescript
interface ApiRequestPanelProps {
  endpoint: string;                                    // Required: API endpoint URL
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH'; // HTTP method (default: 'POST')
  requestBody?: Record<string, any> | string;          // Request body to display/edit
  response?: Record<string, any> | string;             // API response to display
  headers?: Record<string, string>;                    // HTTP headers
  isLoading?: boolean;                                 // Loading state
  error?: string;                                      // Error message
  editable?: boolean;                                  // Enable editable mode (NEW)
  onSubmit?: (body: Record<string, any> | string) => void; // Submit callback (NEW)
}
```

## Basic Usage (Display Only)

```tsx
import { ApiRequestPanel } from './components/ApiRequestPanel';

function MyComponent() {
  return (
    <ApiRequestPanel
      endpoint="https://api.example.com/users"
      method="POST"
      requestBody={{ name: "John", email: "john@example.com" }}
      response={{ id: 123, success: true }}
    />
  );
}
```

## Editable Mode with JSON Validation (NEW)

```tsx
import { ApiRequestPanel } from './components/ApiRequestPanel';

function MyComponent() {
  const [response, setResponse] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  const handleSubmit = async (body) => {
    setLoading(true);
    setError(null);
    
    try {
      const res = await fetch('https://api.example.com/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
      });
      const data = await res.json();
      setResponse(data);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <ApiRequestPanel
      endpoint="https://api.example.com/users"
      method="POST"
      editable={true}
      onSubmit={handleSubmit}
      isLoading={loading}
      response={response}
      error={error}
    />
  );
}
```

## Features

### Display Mode (Default)
- Shows endpoint URL with method badge
- Displays formatted request body (read-only)
- Shows response or loading state
- Generates cURL command
- Copy-to-clipboard for all sections

### Editable Mode (New)
- ✅ **Real-time JSON validation** - Validates as you type
- ✅ **Inline error messages** - Shows specific JSON syntax errors
- ✅ **Smart submit button** - Disabled when JSON is invalid or loading
- ✅ **Accessibility** - Proper ARIA attributes for screen readers
- ✅ **Empty body support** - Treats empty input as `{}`
- ✅ **Initial value** - Pre-fills with `requestBody` if provided

## JSON Validation Behavior

### Valid JSON
- ✅ Submit button is enabled
- ✅ Green success indicator (if content present)
- ✅ No error messages

### Invalid JSON
- ❌ Submit button is disabled
- ❌ Red border on textarea
- ❌ Inline error message with details
- ❌ ARIA attributes indicate error state

### Edge Cases
- Empty string → Valid (submits as `{}`)
- Whitespace only → Valid (submits as `{}`)
- Loading state → Submit disabled regardless of JSON validity

## Examples

### Example 1: API Testing Tool
```tsx
<ApiRequestPanel
  endpoint="https://api.stellar.org/accounts"
  method="GET"
  editable={true}
  onSubmit={handleApiTest}
/>
```

### Example 2: Form Builder
```tsx
<ApiRequestPanel
  endpoint="https://api.example.com/submit"
  method="POST"
  requestBody={{ template: "contact-form" }}
  editable={true}
  onSubmit={handleFormSubmit}
  headers={{ 'Authorization': 'Bearer token123' }}
/>
```

### Example 3: Response Viewer
```tsx
<ApiRequestPanel
  endpoint="https://api.example.com/data"
  method="GET"
  response={apiResponse}
  isLoading={isLoading}
  error={errorMessage}
/>
```

## Styling

The component uses CSS custom properties for theming:

```css
--ak-surface: Background color
--ak-border: Border color
--ak-accent: Primary action color
--ak-error-bg: Error background
--ak-error-border: Error border
--ak-error-text: Error text color
```

## Accessibility

- Textarea has `aria-label="Request body JSON"`
- Invalid state uses `aria-invalid="true"`
- Error message linked via `aria-describedby`
- Error container has `role="alert"`
- Submit button has descriptive `title` attribute

## Testing

Run the test suite:
```bash
npm test -- ApiRequestPanel.test.tsx
```

Or use the standalone tests:
```bash
node test-json-validation.js
node test-component-integration.js
```

Open the interactive demo:
```bash
# Open test-api-request-panel.html in your browser
```

## Migration Guide

### Upgrading from Previous Version

The component is backward compatible. Existing usage continues to work:

```tsx
// Old usage - still works
<ApiRequestPanel
  endpoint="https://api.example.com"
  requestBody={{ data: "value" }}
/>

// New usage - opt-in to editable mode
<ApiRequestPanel
  endpoint="https://api.example.com"
  editable={true}
  onSubmit={handleSubmit}
/>
```

## Common Patterns

### Pattern 1: Try It Out Button
```tsx
const [mode, setMode] = useState('display');

<ApiRequestPanel
  endpoint={endpoint}
  requestBody={defaultBody}
  editable={mode === 'edit'}
  onSubmit={mode === 'edit' ? handleSubmit : undefined}
/>
<button onClick={() => setMode(mode === 'edit' ? 'display' : 'edit')}>
  {mode === 'edit' ? 'Cancel' : 'Try It Out'}
</button>
```

### Pattern 2: Validation Feedback
```tsx
const [validationState, setValidationState] = useState(null);

<ApiRequestPanel
  endpoint={endpoint}
  editable={true}
  onSubmit={(body) => {
    setValidationState('valid');
    handleSubmit(body);
  }}
/>
```

### Pattern 3: Multi-step Form
```tsx
const [step, setStep] = useState(1);

{step === 1 && (
  <ApiRequestPanel
    endpoint="/api/step1"
    editable={true}
    onSubmit={(body) => {
      saveStep1(body);
      setStep(2);
    }}
  />
)}
```

## Troubleshooting

### Submit button stays disabled
- Check if JSON is valid
- Verify `onSubmit` prop is provided
- Check if `isLoading` is false

### Error message not showing
- Ensure JSON is actually invalid
- Check browser console for errors
- Verify CSS is loaded

### Textarea not editable
- Confirm `editable={true}` is set
- Check if component is in loading state
- Verify no CSS conflicts

## Support

For issues or questions:
- Check the test files for examples
- Review the interactive demo
- See the main README.md
