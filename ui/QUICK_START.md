# Quick Start - API Request Panel

Get started with the API Request Panel component in 5 minutes.

## Installation

### Option 1: Copy Files (Recommended for now)

```bash
# Copy component files to your React project
cp ui/components/ApiRequestPanel.tsx src/components/
cp ui/components/ApiRequestPanel.css src/components/
```

### Option 2: Install as Package (Coming Soon)

```bash
npm install @anchorkit/ui-components
```

## Basic Usage

### 1. Import the Component

```tsx
import { ApiRequestPanel } from './components/ApiRequestPanel';
import './components/ApiRequestPanel.css';
```

### 2. Use in Your Component

```tsx
function MyApp() {
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

That's it! You now have a fully functional API request panel.

## Common Patterns

### Pattern 1: With API Call

```tsx
function SubmitAttestation() {
  const [response, setResponse] = useState(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);

  const handleSubmit = async () => {
    setIsLoading(true);
    setError(null);
    
    try {
      const result = await fetch('https://api.anchorkit.stellar.org/v1/attestations', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestData),
      });
      setResponse(await result.json());
    } catch (err) {
      setError(err.message);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <>
      <button onClick={handleSubmit}>Submit</button>
      <ApiRequestPanel
        endpoint="https://api.anchorkit.stellar.org/v1/attestations"
        method="POST"
        requestBody={requestData}
        response={response}
        isLoading={isLoading}
        error={error}
      />
    </>
  );
}
```

### Pattern 2: With AnchorKit Contract

```tsx
function AnchorOperation() {
  const [response, setResponse] = useState(null);
  const [isLoading, setIsLoading] = useState(false);

  const submitAttestation = async () => {
    setIsLoading(true);
    const result = await contract.submit_attestation(
      issuer,
      subject,
      timestamp,
      payloadHash,
      signature
    );
    setResponse(result);
    setIsLoading(false);
  };

  return (
    <ApiRequestPanel
      endpoint="https://api.anchorkit.stellar.org/v1/attestations"
      method="POST"
      requestBody={{ issuer, subject, timestamp }}
      response={response}
      isLoading={isLoading}
    />
  );
}
```

### Pattern 3: Multiple Requests

```tsx
function ApiDashboard() {
  return (
    <div>
      <h2>Submit Attestation</h2>
      <ApiRequestPanel
        endpoint="/v1/attestations"
        method="POST"
        requestBody={attestationData}
        response={attestationResponse}
      />

      <h2>Get Anchor Info</h2>
      <ApiRequestPanel
        endpoint="/v1/anchors/GANCHOR123"
        method="GET"
        response={anchorInfo}
      />

      <h2>Register Attestor</h2>
      <ApiRequestPanel
        endpoint="/v1/attestors"
        method="POST"
        requestBody={attestorData}
        response={registerResponse}
      />
    </div>
  );
}
```

## Props Reference

| Prop | Type | Required | Description |
|------|------|----------|-------------|
| `endpoint` | `string` | ✅ | API endpoint URL |
| `method` | `'GET' \| 'POST' \| 'PUT' \| 'DELETE' \| 'PATCH'` | ❌ | HTTP method (default: 'POST') |
| `requestBody` | `object \| string` | ❌ | Request payload |
| `response` | `object \| string` | ❌ | API response |
| `headers` | `object` | ❌ | HTTP headers |
| `isLoading` | `boolean` | ❌ | Loading state |
| `error` | `string` | ❌ | Error message |

## Features

- ✅ Display endpoint with method badge
- ✅ Show formatted request body
- ✅ Display response with states (loading/error/success)
- ✅ Generate and copy cURL command
- ✅ Copy any section to clipboard
- ✅ Dark mode support
- ✅ Responsive design
- ✅ Accessible

## Styling

The component uses its own CSS file. To customize:

```css
/* Override in your global CSS */
.api-request-panel {
  --primary-color: #your-color;
  --border-radius: 12px;
}
```

## TypeScript

Full TypeScript support included:

```tsx
import { ApiRequestPanel, ApiRequestPanelProps } from './components/ApiRequestPanel';

const props: ApiRequestPanelProps = {
  endpoint: 'https://api.example.com',
  method: 'POST',
  // ... TypeScript will validate all props
};
```

## Testing

```bash
cd ui
npm install
npm test
```

## Examples

See complete examples in:
- `ui/components/ApiRequestPanel.example.tsx`
- `ui/components/README.md`

## Next Steps

1. ✅ Copy component to your project
2. ✅ Import and use in your app
3. 📖 Read full documentation: `ui/components/README.md`
4. 🧪 Run tests: `npm test`
5. 🎨 Customize styling as needed

## Need Help?

- 📖 Full docs: `ui/components/README.md`
- 💡 Examples: `ui/components/ApiRequestPanel.example.tsx`
- 🧪 Tests: `ui/components/ApiRequestPanel.test.tsx`
- 🐛 Issues: GitHub Issues

## What's Next?

Try these enhancements:
- Add request history
- Implement response formatting options
- Add authentication helpers
- Create request builder UI
- Add response time display

---

**You're ready to go!** 🚀

Start using the API Request Panel in your AnchorKit application.
