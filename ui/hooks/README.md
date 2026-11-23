# useCopyToClipboard Hook

A clean, reusable React hook for copying text to clipboard with success feedback and utility functions for common copy operations.

## Features

- ✅ **Simple API** - One hook for all copy operations
- ✅ **Success Feedback** - Built-in copied state with configurable duration
- ✅ **Error Handling** - Graceful fallback with error callbacks
- ✅ **Utility Functions** - Format JSON, generate curl commands, install commands
- ✅ **TypeScript** - Full type safety
- ✅ **Zero Dependencies** - Uses native Clipboard API
- ✅ **Well Tested** - Comprehensive test coverage

## Installation

The hook is part of the AnchorKit UI components. No additional installation needed.

## Basic Usage

```tsx
import { useCopyToClipboard } from './hooks/useCopyToClipboard';

function MyComponent() {
  const { copy, isCopied } = useCopyToClipboard();

  return (
    <button onClick={() => copy('Hello World')}>
      {isCopied ? '✓ Copied!' : 'Copy Text'}
    </button>
  );
}
```

## API Reference

### `useCopyToClipboard(options?)`

#### Parameters

```typescript
interface CopyToClipboardOptions {
  successDuration?: number;  // Duration to show success state (default: 2000ms)
  onSuccess?: () => void;    // Callback when copy succeeds
  onError?: (error: Error) => void;  // Callback when copy fails
}
```

#### Returns

```typescript
interface CopyToClipboardResult {
  copiedText: string | null;  // Last copied text
  isCopied: boolean;          // Whether currently showing success state
  copy: (text: string) => Promise<boolean>;  // Copy function
  reset: () => void;          // Reset state manually
}
```

## Examples

### 1. Copy JSON Configuration

```tsx
import { useCopyToClipboard, formatJsonForCopy } from './hooks/useCopyToClipboard';

function JsonCopyButton() {
  const { copy, isCopied } = useCopyToClipboard();
  
  const config = {
    network: 'Testnet',
    anchor_domain: 'anchor.example.com',
    timeout_seconds: 30,
  };

  return (
    <button onClick={() => copy(formatJsonForCopy(config))}>
      {isCopied ? '✓ Copied!' : '📋 Copy JSON'}
    </button>
  );
}
```

### 2. Copy Curl Command

```tsx
import { useCopyToClipboard, generateCurlCommand } from './hooks/useCopyToClipboard';

function CurlCopyButton() {
  const { copy, isCopied } = useCopyToClipboard();
  
  const curlCommand = generateCurlCommand({
    url: 'https://api.example.com/data',
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer token',
    },
    body: { name: 'test' },
  });

  return (
    <button onClick={() => copy(curlCommand)}>
      {isCopied ? '✓ Copied!' : '📋 Copy Curl'}
    </button>
  );
}
```

### 3. Copy Install Command

```tsx
import { useCopyToClipboard, generateInstallCommand } from './hooks/useCopyToClipboard';

function InstallCopyButton() {
  const { copy, isCopied } = useCopyToClipboard();
  const [pm, setPm] = useState<'npm' | 'yarn' | 'pnpm'>('npm');
  
  const installCmd = generateInstallCommand('anchorkit-sdk', pm);

  return (
    <>
      <select value={pm} onChange={(e) => setPm(e.target.value as any)}>
        <option value="npm">npm</option>
        <option value="yarn">yarn</option>
        <option value="pnpm">pnpm</option>
      </select>
      <button onClick={() => copy(installCmd)}>
        {isCopied ? '✓ Copied!' : '📋 Copy'}
      </button>
    </>
  );
}
```

### 4. With Callbacks

```tsx
function CopyWithCallbacks() {
  const { copy, isCopied } = useCopyToClipboard({
    successDuration: 3000,
    onSuccess: () => console.log('Copied successfully!'),
    onError: (err) => alert(`Failed: ${err.message}`),
  });

  return (
    <button onClick={() => copy('text')}>
      {isCopied ? '✓ Copied!' : 'Copy'}
    </button>
  );
}
```

### 5. Multiple Copy Buttons

```tsx
function MultiCopyButtons() {
  const { copy: copyJson, isCopied: jsonCopied } = useCopyToClipboard();
  const { copy: copyCurl, isCopied: curlCopied } = useCopyToClipboard();

  return (
    <>
      <button onClick={() => copyJson('{"key": "value"}')}>
        {jsonCopied ? '✓' : '📋'} Copy JSON
      </button>
      <button onClick={() => copyCurl('curl https://api.example.com')}>
        {curlCopied ? '✓' : '📋'} Copy Curl
      </button>
    </>
  );
}
```

## Utility Functions

### `formatJsonForCopy(data, pretty?)`

Formats data as JSON string for copying.

```typescript
const json = formatJsonForCopy({ name: 'test' });
// Returns: "{\n  \"name\": \"test\"\n}"

const compact = formatJsonForCopy({ name: 'test' }, false);
// Returns: "{\"name\":\"test\"}"
```

### `generateCurlCommand(options)`

Generates a curl command string.

```typescript
const curl = generateCurlCommand({
  url: 'https://api.example.com/data',
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: { name: 'test' },
});
// Returns: curl -X POST \
//   "https://api.example.com/data" \
//   -H "Content-Type: application/json" \
//   -d '{"name":"test"}'
```

### `generateInstallCommand(packageName, packageManager?)`

Generates package install command.

```typescript
generateInstallCommand('anchorkit', 'npm');   // "npm install anchorkit"
generateInstallCommand('anchorkit', 'yarn');  // "yarn add anchorkit"
generateInstallCommand('anchorkit', 'pnpm');  // "pnpm add anchorkit"
```

## Browser Support

Uses the native [Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API):
- ✅ Chrome 63+
- ✅ Firefox 53+
- ✅ Safari 13.1+
- ✅ Edge 79+

**Note**: Requires HTTPS in production (or localhost for development).

## Error Handling

The hook gracefully handles errors:

```tsx
const { copy } = useCopyToClipboard({
  onError: (error) => {
    if (error.message.includes('not available')) {
      alert('Clipboard not supported in this browser');
    } else {
      alert('Failed to copy');
    }
  },
});
```

Common errors:
- Clipboard API not available (old browsers)
- User denied clipboard permission
- Document not focused (security restriction)

## Testing

The hook includes comprehensive tests:

```bash
npm test useCopyToClipboard
```

Test coverage:
- ✅ Successful copy operations
- ✅ Success state duration
- ✅ Callback invocations
- ✅ Error handling
- ✅ Manual reset
- ✅ Missing clipboard API
- ✅ Utility function outputs

## Best Practices

### 1. Use Separate Instances for Multiple Buttons

```tsx
// ✅ Good - Each button has its own state
const { copy: copy1, isCopied: copied1 } = useCopyToClipboard();
const { copy: copy2, isCopied: copied2 } = useCopyToClipboard();

// ❌ Bad - Shared state causes confusion
const { copy, isCopied } = useCopyToClipboard();
```

### 2. Provide Visual Feedback

```tsx
// ✅ Good - Clear feedback
<button onClick={() => copy(text)}>
  {isCopied ? '✓ Copied!' : '📋 Copy'}
</button>

// ❌ Bad - No feedback
<button onClick={() => copy(text)}>Copy</button>
```

### 3. Handle Errors Gracefully

```tsx
// ✅ Good - User-friendly error handling
const { copy } = useCopyToClipboard({
  onError: () => alert('Failed to copy. Please try again.'),
});

// ❌ Bad - Silent failure
const { copy } = useCopyToClipboard();
```

### 4. Use Appropriate Duration

```tsx
// ✅ Good - Reasonable duration
useCopyToClipboard({ successDuration: 2000 });  // 2 seconds

// ❌ Bad - Too short or too long
useCopyToClipboard({ successDuration: 100 });   // Too fast
useCopyToClipboard({ successDuration: 10000 }); // Too slow
```

## Integration with Existing Components

The hook is designed to work seamlessly with existing AnchorKit components:

```tsx
import { ApiRequestPanel } from '../components/ApiRequestPanel';
import { useCopyToClipboard } from '../hooks/useCopyToClipboard';

// Can be used alongside or within existing components
```

## Performance

- **Lightweight**: ~2KB minified
- **No re-renders**: Only updates when copy state changes
- **Memoized**: Callbacks are memoized with `useCallback`
- **Efficient**: Automatic cleanup of timers

## Security

- Uses native Clipboard API (secure)
- Requires user interaction (prevents abuse)
- HTTPS required in production
- No data sent to external services

## Related

- [ApiRequestPanel](../components/ApiRequestPanel.tsx) - Uses similar copy pattern
- [Webhook Monitor](../../webhook_monitor.html) - Could benefit from this hook
- [SDK Config Form](../../sdk_config_form.html) - Could use for JSON output

## License

Part of AnchorKit project.
