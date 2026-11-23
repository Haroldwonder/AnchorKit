import React from 'react';
import {
  useCopyToClipboard,
  formatJsonForCopy,
  generateCurlCommand,
  generateInstallCommand,
} from './useCopyToClipboard';

/**
 * Example 1: Basic JSON Copy
 */
export const JsonCopyExample: React.FC = () => {
  const { copy, isCopied } = useCopyToClipboard();

  const sampleData = {
    network: 'Testnet',
    anchor_domain: 'anchor.example.com',
    timeout_seconds: 30,
  };

  return (
    <div style={{ padding: '20px' }}>
      <h3>Copy JSON Configuration</h3>
      <pre style={{ background: '#f5f5f5', padding: '10px', borderRadius: '4px' }}>
        {formatJsonForCopy(sampleData)}
      </pre>
      <button
        onClick={() => copy(formatJsonForCopy(sampleData))}
        style={{
          padding: '8px 16px',
          background: isCopied ? '#48bb78' : '#4299e1',
          color: 'white',
          border: 'none',
          borderRadius: '4px',
          cursor: 'pointer',
        }}
      >
        {isCopied ? '✓ Copied!' : '📋 Copy JSON'}
      </button>
    </div>
  );
};

/**
 * Example 2: Curl Command Copy
 */
export const CurlCopyExample: React.FC = () => {
  const { copy, isCopied } = useCopyToClipboard({ successDuration: 3000 });

  const curlCommand = generateCurlCommand({
    url: 'https://api.anchorkit.io/v1/quotes',
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer YOUR_TOKEN',
    },
    body: {
      base_asset: 'USD',
      quote_asset: 'USDC',
      amount: 1000,
    },
  });

  return (
    <div style={{ padding: '20px' }}>
      <h3>Copy Curl Command</h3>
      <pre
        style={{
          background: '#2d3748',
          color: '#e2e8f0',
          padding: '12px',
          borderRadius: '4px',
          overflow: 'auto',
        }}
      >
        {curlCommand}
      </pre>
      <button
        onClick={() => copy(curlCommand)}
        style={{
          padding: '8px 16px',
          background: isCopied ? '#48bb78' : '#4299e1',
          color: 'white',
          border: 'none',
          borderRadius: '4px',
          cursor: 'pointer',
        }}
      >
        {isCopied ? '✓ Copied!' : '📋 Copy Curl'}
      </button>
    </div>
  );
};

/**
 * Example 3: Install Command Copy
 */
export const InstallCommandExample: React.FC = () => {
  const { copy, isCopied } = useCopyToClipboard();
  const [packageManager, setPackageManager] = React.useState<'npm' | 'yarn' | 'pnpm'>('npm');

  const installCommand = generateInstallCommand('anchorkit-sdk', packageManager);

  return (
    <div style={{ padding: '20px' }}>
      <h3>Copy Install Command</h3>
      
      <div style={{ marginBottom: '10px' }}>
        <label style={{ marginRight: '10px' }}>Package Manager:</label>
        <select
          value={packageManager}
          onChange={(e) => setPackageManager(e.target.value as any)}
          style={{ padding: '4px 8px' }}
        >
          <option value="npm">npm</option>
          <option value="yarn">yarn</option>
          <option value="pnpm">pnpm</option>
        </select>
      </div>

      <pre
        style={{
          background: '#f5f5f5',
          padding: '10px',
          borderRadius: '4px',
          display: 'inline-block',
        }}
      >
        {installCommand}
      </pre>
      
      <button
        onClick={() => copy(installCommand)}
        style={{
          padding: '8px 16px',
          background: isCopied ? '#48bb78' : '#4299e1',
          color: 'white',
          border: 'none',
          borderRadius: '4px',
          cursor: 'pointer',
          marginLeft: '10px',
        }}
      >
        {isCopied ? '✓ Copied!' : '📋 Copy'}
      </button>
    </div>
  );
};

/**
 * Example 4: Multiple Copy Buttons with Callbacks
 */
export const MultiCopyExample: React.FC = () => {
  const [lastCopied, setLastCopied] = React.useState<string>('');

  const { copy: copyJson, isCopied: jsonCopied } = useCopyToClipboard({
    onSuccess: () => setLastCopied('JSON'),
    onError: (err) => alert(`Failed to copy: ${err.message}`),
  });

  const { copy: copyCurl, isCopied: curlCopied } = useCopyToClipboard({
    onSuccess: () => setLastCopied('Curl'),
  });

  const jsonData = { status: 'success', data: { id: 123 } };
  const curlCmd = generateCurlCommand({
    url: 'https://api.example.com/data',
    method: 'GET',
  });

  return (
    <div style={{ padding: '20px' }}>
      <h3>Multiple Copy Buttons</h3>
      
      {lastCopied && (
        <div
          style={{
            padding: '8px',
            background: '#c6f6d5',
            color: '#22543d',
            borderRadius: '4px',
            marginBottom: '10px',
          }}
        >
          Last copied: {lastCopied}
        </div>
      )}

      <div style={{ display: 'flex', gap: '10px' }}>
        <button
          onClick={() => copyJson(formatJsonForCopy(jsonData))}
          style={{
            padding: '8px 16px',
            background: jsonCopied ? '#48bb78' : '#4299e1',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          {jsonCopied ? '✓' : '📋'} Copy JSON
        </button>

        <button
          onClick={() => copyCurl(curlCmd)}
          style={{
            padding: '8px 16px',
            background: curlCopied ? '#48bb78' : '#4299e1',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          {curlCopied ? '✓' : '📋'} Copy Curl
        </button>
      </div>
    </div>
  );
};

/**
 * Example 5: Inline Copy Button Component
 */
export const InlineCopyButton: React.FC<{ text: string; label?: string }> = ({
  text,
  label = 'Copy',
}) => {
  const { copy, isCopied } = useCopyToClipboard();

  return (
    <button
      onClick={() => copy(text)}
      style={{
        padding: '4px 8px',
        fontSize: '12px',
        background: isCopied ? '#48bb78' : '#edf2f7',
        color: isCopied ? 'white' : '#2d3748',
        border: '1px solid #cbd5e0',
        borderRadius: '4px',
        cursor: 'pointer',
        transition: 'all 0.2s',
      }}
      title={`Copy ${label}`}
    >
      {isCopied ? '✓' : '📋'}
    </button>
  );
};

/**
 * Example 6: Complete Demo
 */
export const CopyToClipboardDemo: React.FC = () => {
  return (
    <div style={{ maxWidth: '800px', margin: '0 auto', padding: '20px' }}>
      <h1>Copy to Clipboard Hook Examples</h1>
      
      <div style={{ display: 'flex', flexDirection: 'column', gap: '30px' }}>
        <JsonCopyExample />
        <hr />
        <CurlCopyExample />
        <hr />
        <InstallCommandExample />
        <hr />
        <MultiCopyExample />
        <hr />
        
        <div style={{ padding: '20px' }}>
          <h3>Inline Copy Button</h3>
          <p>
            API Key: <code>sk_test_1234567890</code>{' '}
            <InlineCopyButton text="sk_test_1234567890" label="API Key" />
          </p>
        </div>
      </div>
    </div>
  );
};

export default CopyToClipboardDemo;
