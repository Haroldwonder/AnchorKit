# AnchorKit Wallet Connector

A reusable, lightweight wallet connector for Stellar blockchain applications.

## Features

- ✅ Connect to Stellar wallets (Freighter, Albedo, Rabet)
- ✅ Disconnect wallet
- ✅ Display public key with copy functionality
- ✅ Sign transactions
- ✅ Event-driven architecture
- ✅ LocalStorage persistence
- ✅ Responsive UI component
- ✅ Zero dependencies

## Quick Start

### Option 1: Standalone HTML Component

Open `wallet-connector.html` in your browser for a ready-to-use UI component.

### Option 2: JavaScript Module

`wallet-connector.js` is a CommonJS module — it exposes `WalletConnector` via
`module.exports` and does not provide an ES-module `export`. Import it with
`require()` (or load it with a `<script>` tag, as shown in the Integration
Example below):

```javascript
// Import the module (CommonJS)
const WalletConnector = require('./wallet-connector.js');

// Create instance
const wallet = new WalletConnector();

// Connect wallet
try {
    const { publicKey, walletType } = await wallet.connect();
    console.log(`Connected to ${walletType}: ${publicKey}`);
} catch (error) {
    console.error('Connection failed:', error);
}

// Get public key
const publicKey = wallet.getPublicKey();

// Check connection status
if (wallet.isConnected()) {
    console.log('Wallet is connected');
}

// Disconnect
wallet.disconnect();
```

## API Reference

### Methods

#### `connect()`
Connects to an available Stellar wallet.

**Returns:** `Promise<{publicKey: string, walletType: string}>`

```javascript
const { publicKey, walletType } = await wallet.connect();
```

#### `disconnect()`
Disconnects the current wallet.

```javascript
wallet.disconnect();
```

#### `getPublicKey()`
Returns the connected wallet's public key.

**Returns:** `string | null`

```javascript
const publicKey = wallet.getPublicKey();
```

#### `isConnected()`
Checks if a wallet is currently connected.

**Returns:** `boolean`

```javascript
if (wallet.isConnected()) {
    // Wallet is connected
}
```

#### `getWalletType()`
Returns the type of connected wallet.

**Returns:** `string | null` - 'Freighter', 'Albedo', 'Rabet', or null

```javascript
const walletType = wallet.getWalletType();
```

#### `signTransaction(xdr, network)`
Signs a transaction with the connected wallet.

**Parameters:**
- `xdr` (string): Transaction XDR
- `network` (string): Network passphrase ('PUBLIC' or 'TESTNET')

**Returns:** `Promise<string>` - Signed transaction XDR

```javascript
const signedXdr = await wallet.signTransaction(transactionXdr, 'PUBLIC');
```

#### `checkExistingConnection()`
Checks localStorage for an existing connection.

**Returns:** `boolean`

```javascript
if (wallet.checkExistingConnection()) {
    console.log('Restored previous connection');
}
```

### Events

#### `on(event, callback)`
Adds an event listener.

**Events:**
- `connect`: Fired when wallet connects
- `disconnect`: Fired when wallet disconnects
- `error`: Fired when an error occurs

```javascript
wallet.on('connect', ({ publicKey, walletType }) => {
    console.log(`Connected: ${publicKey}`);
});

wallet.on('disconnect', () => {
    console.log('Wallet disconnected');
});

wallet.on('error', (error) => {
    console.error('Wallet error:', error);
});
```

#### `off(event, callback)`
Removes an event listener.

```javascript
const handler = (data) => console.log(data);
wallet.on('connect', handler);
wallet.off('connect', handler);
```

## Supported Wallets

- **Freighter**: Browser extension wallet for Stellar
- **Albedo**: Web-based Stellar wallet
- **Rabet**: Browser extension wallet for Stellar

## Integration Example

```html
<!DOCTYPE html>
<html>
<head>
    <title>My Stellar App</title>
</head>
<body>
    <button id="connectBtn">Connect Wallet</button>
    <div id="publicKey"></div>

    <script src="wallet-connector.js"></script>
    <script>
        const wallet = new WalletConnector();
        const connectBtn = document.getElementById('connectBtn');
        const publicKeyDiv = document.getElementById('publicKey');

        connectBtn.addEventListener('click', async () => {
            try {
                const { publicKey } = await wallet.connect();
                publicKeyDiv.textContent = `Connected: ${publicKey}`;
                connectBtn.textContent = 'Disconnect';
            } catch (error) {
                alert(error.message);
            }
        });

        wallet.on('connect', ({ publicKey }) => {
            console.log('Wallet connected:', publicKey);
        });

        wallet.on('disconnect', () => {
            publicKeyDiv.textContent = '';
            connectBtn.textContent = 'Connect Wallet';
        });
    </script>
</body>
</html>
```

## Browser Compatibility

- Chrome/Edge: ✅
- Firefox: ✅
- Safari: ✅
- Opera: ✅

## Security Notes

- Public keys are stored in localStorage for convenience
- Never store private keys or seeds
- Always verify transaction details before signing
- Use HTTPS in production

## License

MIT License - Part of AnchorKit
