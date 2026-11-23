/**
 * AnchorKit Wallet Connector
 * A reusable module for connecting to Stellar wallets
 * Supports: Freighter, Albedo, Rabet
 */

class WalletConnector {
    constructor() {
        this.connected = false;
        this.publicKey = null;
        this.walletType = null;
        this.listeners = {
            connect: [],
            disconnect: [],
            error: []
        };
    }

    /**
     * Connect to a Stellar wallet
     * @returns {Promise<{publicKey: string, walletType: string}>}
     */
    async connect() {
        try {
            // Try Freighter (most popular)
            if (window.freighterApi) {
                const { publicKey } = await window.freighterApi.getPublicKey();
                this.walletType = 'Freighter';
                this._setConnected(publicKey);
                return { publicKey, walletType: this.walletType };
            }

            // Try Albedo
            if (window.albedo) {
                const result = await window.albedo.publicKey();
                this.walletType = 'Albedo';
                this._setConnected(result.pubkey);
                return { publicKey: result.pubkey, walletType: this.walletType };
            }

            // Try Rabet
            if (window.rabet) {
                await window.rabet.connect();
                const publicKey = await window.rabet.getPublicKey();
                this.walletType = 'Rabet';
                this._setConnected(publicKey);
                return { publicKey, walletType: this.walletType };
            }

            throw new Error('No Stellar wallet detected. Please install Freighter, Albedo, or Rabet.');
        } catch (error) {
            this._emitError(error);
            throw error;
        }
    }

    /**
     * Disconnect from wallet
     */
    disconnect() {
        this.connected = false;
        this.publicKey = null;
        this.walletType = null;
        localStorage.removeItem('anchorkit_wallet_publicKey');
        this._emit('disconnect');
    }

    /**
     * Get current public key
     * @returns {string|null}
     */
    getPublicKey() {
        return this.publicKey;
    }

    /**
     * Check if wallet is connected
     * @returns {boolean}
     */
    isConnected() {
        return this.connected;
    }

    /**
     * Get wallet type
     * @returns {string|null}
     */
    getWalletType() {
        return this.walletType;
    }

    /**
     * Sign a transaction
     * @param {string} xdr - Transaction XDR
     * @param {string} network - Network passphrase
     * @returns {Promise<string>} Signed XDR
     */
    async signTransaction(xdr, network = 'PUBLIC') {
        if (!this.connected) {
            throw new Error('Wallet not connected');
        }

        try {
            switch (this.walletType) {
                case 'Freighter':
                    return await window.freighterApi.signTransaction(xdr, network);
                
                case 'Albedo':
                    const result = await window.albedo.tx({ xdr, network });
                    return result.signed_envelope_xdr;
                
                case 'Rabet':
                    const rabetResult = await window.rabet.sign(xdr, network);
                    return rabetResult.xdr;
                
                default:
                    throw new Error('Unknown wallet type');
            }
        } catch (error) {
            this._emitError(error);
            throw error;
        }
    }

    /**
     * Add event listener
     * @param {string} event - Event name (connect, disconnect, error)
     * @param {Function} callback - Callback function
     */
    on(event, callback) {
        if (this.listeners[event]) {
            this.listeners[event].push(callback);
        }
    }

    /**
     * Remove event listener
     * @param {string} event - Event name
     * @param {Function} callback - Callback function
     */
    off(event, callback) {
        if (this.listeners[event]) {
            this.listeners[event] = this.listeners[event].filter(cb => cb !== callback);
        }
    }

    /**
     * Check for existing connection
     * @returns {boolean}
     */
    checkExistingConnection() {
        const savedPublicKey = localStorage.getItem('anchorkit_wallet_publicKey');
        if (savedPublicKey) {
            this.publicKey = savedPublicKey;
            this.connected = true;
            return true;
        }
        return false;
    }

    // Private methods
    _setConnected(publicKey) {
        this.connected = true;
        this.publicKey = publicKey;
        localStorage.setItem('anchorkit_wallet_publicKey', publicKey);
        this._emit('connect', { publicKey, walletType: this.walletType });
    }

    _emit(event, data) {
        if (this.listeners[event]) {
            this.listeners[event].forEach(callback => callback(data));
        }
    }

    _emitError(error) {
        this._emit('error', error);
    }
}

// Export for different module systems
// ES module export (enables `import WalletConnector from './wallet-connector.js'`)
export default WalletConnector;

// CommonJS export (kept for existing consumers)
if (typeof module !== 'undefined' && module.exports) {
    module.exports = WalletConnector;
}

if (typeof window !== 'undefined') {
    window.WalletConnector = WalletConnector;
}
