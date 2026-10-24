/**
 * Webhook Monitor Integration Example
 * 
 * This file shows how to integrate the webhook monitor with your backend.
 * Choose one of the three methods below based on your infrastructure.
 */

// ============================================================================
// STANDALONE BOOTSTRAP
// When this file runs outside webhook_monitor.html (e.g. in a Node.js backend
// test or as a standalone <script>), eventCounter and addEvent must exist.
// The HTML page declares its own `let eventCounter` and `function addEvent`
// which will shadow these module-level versions for browser embeds — so this
// is a safe, non-breaking fallback only.
// ============================================================================

/** @type {number} Running sequence counter used to stamp each event with a unique id. */
let eventCounter = 0;

/**
 * Default addEvent implementation used when the file runs outside
 * webhook_monitor.html (e.g. in a Node.js process or a unit-test harness).
 * In the browser page, the page-local addEvent shadows this function.
 *
 * @param {{ id: number, type: string, timestamp: string, payload: object }} event
 */
/* eslint-disable no-unused-vars */
function addEvent(event) {
    // Fallback: emit a structured log line so events are not silently dropped
    // when running outside the monitor UI.
    console.log(
        `[WebhookMonitor] event #${event.id} type=${event.type} ts=${event.timestamp}`,
        event.payload
    );
}
/* eslint-enable no-unused-vars */

// ============================================================================
// METHOD 1: WebSocket (Recommended for Real-Time)
// ============================================================================

class WebhookMonitorWebSocket {
    /**
     * @param {string} wsUrl - WebSocket server URL
     * @param {object} [options]
     * @param {number} [options.maxReconnectAttempts=10] - Max reconnect attempts before giving up
     */
    constructor(wsUrl, options = {}) {
        this.wsUrl = wsUrl;
        this.ws = null;
        this.reconnectCount = 0;
        this.maxReconnectAttempts = options.maxReconnectAttempts ?? 10;
        this._listeners = {};
    }

    // Minimal EventEmitter interface (avoids a Node.js-only dependency in browser contexts)
    on(event, listener) {
        if (!this._listeners[event]) this._listeners[event] = [];
        this._listeners[event].push(listener);
        return this;
    }

    emit(event, ...args) {
        (this._listeners[event] || []).forEach(fn => fn(...args));
    }

    connect() {
        this.ws = new WebSocket(this.wsUrl);

        this.ws.onopen = () => {
            console.log('✅ WebSocket connected');
            this.reconnectCount = 0; // reset budget on every successful connection
        };

        this.ws.onmessage = (event) => {
            try {
                const webhookData = JSON.parse(event.data);
                this.handleWebhookEvent(webhookData);
            } catch (error) {
                console.error('Failed to parse webhook event:', error);
            }
        };

        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };

        this.ws.onclose = () => {
            console.log('WebSocket closed');
            this.attemptReconnect();
        };
    }

    attemptReconnect() {
        if (this.reconnectCount >= this.maxReconnectAttempts) {
            // Only emit the event on the first time we hit the limit
            if (this.reconnectCount === this.maxReconnectAttempts) {
                console.error(
                    `WebSocket reconnect limit reached (${this.maxReconnectAttempts} attempts). Giving up.`
                );
                this.emit('max_reconnects_exceeded', { attempts: this.reconnectCount });
                // Sentinel: increment past the limit so subsequent onclose calls are silent no-ops
                this.reconnectCount++;
            }
            return;
        }

        this.reconnectCount++;
        const delay = Math.min(1000 * Math.pow(2, this.reconnectCount), 30000);
        console.log(`Reconnecting in ${delay}ms... (attempt ${this.reconnectCount}/${this.maxReconnectAttempts})`);
        setTimeout(() => this.connect(), delay);
    }

    handleWebhookEvent(webhookData) {
        // eventCounter and addEvent are declared at module scope (see top of file).
        // When embedded in webhook_monitor.html the page-local versions take over.
        addEvent({
            id: ++eventCounter,
            type: webhookData.type,
            timestamp: webhookData.timestamp || new Date().toISOString(),
            payload: webhookData.payload
        });
    }

    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }
}

// Usage:
// const monitor = new WebhookMonitorWebSocket('ws://localhost:8080/webhooks');
// monitor.connect();

// ============================================================================
// METHOD 2: Server-Sent Events (SSE)
// ============================================================================

class WebhookMonitorSSE {
    constructor(sseUrl) {
        this.sseUrl = sseUrl;
        this.eventSource = null;
    }

    connect() {
        this.eventSource = new EventSource(this.sseUrl);

        this.eventSource.onopen = () => {
            console.log('✅ SSE connected');
        };

        this.eventSource.onmessage = (event) => {
            try {
                const webhookData = JSON.parse(event.data);
                this.handleWebhookEvent(webhookData);
            } catch (error) {
                console.error('Failed to parse SSE event:', error);
            }
        };

        this.eventSource.onerror = (error) => {
            console.error('SSE error:', error);
            // SSE automatically reconnects
        };
    }

    handleWebhookEvent(webhookData) {
        addEvent({
            id: ++eventCounter,
            type: webhookData.type,
            timestamp: webhookData.timestamp || new Date().toISOString(),
            payload: webhookData.payload
        });
    }

    disconnect() {
        if (this.eventSource) {
            this.eventSource.close();
            this.eventSource = null;
        }
    }
}

// Usage:
// const monitor = new WebhookMonitorSSE('/api/webhook-stream');
// monitor.connect();

// ============================================================================
// METHOD 3: HTTP Polling (Fallback)
// ============================================================================

class WebhookMonitorPolling {
    /**
     * @param {string} apiUrl
     * @param {object} [options]
     * @param {number} [options.baseIntervalMs=2000] - Base polling interval
     * @param {number} [options.maxIntervalMs=30000] - Maximum backoff interval
     */
    constructor(apiUrl, options = {}) {
        // Support legacy positional signature: new WebhookMonitorPolling(url, 3000)
        if (typeof options === 'number') {
            options = { baseIntervalMs: options };
        }
        this.apiUrl = apiUrl;
        this.baseIntervalMs = options.baseIntervalMs ?? 2000;
        this.maxIntervalMs = options.maxIntervalMs ?? 30000;
        this._timeoutId = null;
        this._running = false;
        this._failures = 0;
        this.lastEventId = 0;
    }

    start() {
        this._running = true;
        this.poll();
    }

    async poll() {
        if (!this._running) return;

        try {
            const response = await fetch(`${this.apiUrl}?since=${this.lastEventId}`);

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const events = await response.json();
            this._failures = 0; // reset backoff on success

            events.forEach(webhookData => {
                this.handleWebhookEvent(webhookData);
                if (webhookData.id > this.lastEventId) {
                    this.lastEventId = webhookData.id;
                }
            });
        } catch (error) {
            this._failures++;
            console.error('Polling error:', error);
        }

        if (this._running) {
            const delay = Math.min(
                this.baseIntervalMs * Math.pow(2, this._failures),
                this.maxIntervalMs
            );
            this._timeoutId = setTimeout(() => this.poll(), delay);
        }
    }

    handleWebhookEvent(webhookData) {
        addEvent({
            id: ++eventCounter,
            type: webhookData.type,
            timestamp: webhookData.timestamp || new Date().toISOString(),
            payload: webhookData.payload
        });
    }

    stop() {
        this._running = false;
        if (this._timeoutId) {
            clearTimeout(this._timeoutId);
            this._timeoutId = null;
        }
    }
}

// Usage:
// const monitor = new WebhookMonitorPolling('/api/webhooks/recent', 3000);
// monitor.start();

// ============================================================================
// BACKEND EXAMPLES
// ============================================================================

// Express.js + WebSocket Example
/*
const express = require('express');
const WebSocket = require('ws');
const app = express();

const wss = new WebSocket.Server({ port: 8080 });

// Webhook endpoint
app.post('/webhook', express.json(), (req, res) => {
    const event = {
        id: Date.now(),
        type: req.body.type,
        timestamp: new Date().toISOString(),
        payload: req.body.data
    };
    
    // Broadcast to all connected clients
    wss.clients.forEach(client => {
        if (client.readyState === WebSocket.OPEN) {
            client.send(JSON.stringify(event));
        }
    });
    
    res.status(200).json({ success: true });
});

app.listen(3000, () => console.log('Server running on port 3000'));
*/

// Express.js + SSE Example
/*
const express = require('express');
const app = express();

const clients = [];

// SSE endpoint
app.get('/api/webhook-stream', (req, res) => {
    res.setHeader('Content-Type', 'text/event-stream');
    res.setHeader('Cache-Control', 'no-cache');
    res.setHeader('Connection', 'keep-alive');
    
    clients.push(res);
    
    req.on('close', () => {
        clients.splice(clients.indexOf(res), 1);
    });
});

// Webhook endpoint
app.post('/webhook', express.json(), (req, res) => {
    const event = {
        id: Date.now(),
        type: req.body.type,
        timestamp: new Date().toISOString(),
        payload: req.body.data
    };
    
    // Send to all SSE clients
    clients.forEach(client => {
        client.write(`data: ${JSON.stringify(event)}\n\n`);
    });
    
    res.status(200).json({ success: true });
});

app.listen(3000, () => console.log('Server running on port 3000'));
*/

// ============================================================================
// SECURITY MIDDLEWARE EXAMPLE
// ============================================================================

// Allowlist of known, expected fields in a webhook payload.
// Only these fields will pass through to downstream logic.
const WEBHOOK_PAYLOAD_ALLOWLIST = new Set([
    'id',
    'type',
    'timestamp',
    'status',
    'amount',
    'asset',
    'user',
    'email',
    'memo',
    'transaction_id',
    'account',
    'network',
    'fee',
    'message',
]);

function sanitizeWebhookPayload(payload) {
    if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
        return {};
    }

    const sanitized = {};

    for (const key of Object.keys(payload)) {
        if (!WEBHOOK_PAYLOAD_ALLOWLIST.has(key)) {
            // Log field name only — never log the value to avoid leaking sensitive data
            console.warn(`WARN: Unknown field detected in webhook payload: "${key}"`);
            continue;
        }

        let value = payload[key];

        // Redact email addresses
        if (key === 'email' && typeof value === 'string' && value.includes('@')) {
            const [local, domain] = value.split('@');
            value = `${local.substring(0, 2)}***@${domain}`;
        }

        // Truncate Stellar account addresses
        if (key === 'user' && typeof value === 'string' && value.length > 11) {
            value = `${value.substring(0, 8)}...${value.substring(value.length - 3)}`;
        }

        sanitized[key] = value;
    }

    return sanitized;
}

// ============================================================================
// STELLAR HORIZON INTEGRATION EXAMPLE
// ============================================================================

/*
const StellarSdk = require('stellar-sdk');

function monitorStellarAccount(accountId) {
    const server = new StellarSdk.Server('https://horizon-testnet.stellar.org');
    
    server.operations()
        .forAccount(accountId)
        .cursor('now')
        .stream({
            onmessage: (operation) => {
                const event = {
                    id: operation.id,
                    type: mapOperationType(operation.type),
                    timestamp: operation.created_at,
                    payload: {
                        operation_type: operation.type,
                        from: operation.from,
                        to: operation.to,
                        amount: operation.amount,
                        asset: operation.asset_type
                    }
                };
                
                // Send to webhook monitor
                broadcastEvent(event);
            },
            onerror: (error) => {
                console.error('Stellar stream error:', error);
            }
        });
}

function mapOperationType(stellarType) {
    const typeMap = {
        'payment': 'transfer',
        'create_account': 'deposit',
        'path_payment_strict_receive': 'transfer',
        'path_payment_strict_send': 'transfer'
    };
    return typeMap[stellarType] || 'other';
}
*/

// ============================================================================
// USAGE IN webhook_monitor.html
// ============================================================================

/*
Replace the simulation code in webhook_monitor.html with:

<script>
    // Choose your integration method
    const monitor = new WebhookMonitorWebSocket('ws://localhost:8080/webhooks');
    // OR
    // const monitor = new WebhookMonitorSSE('/api/webhook-stream');
    // OR
    // const monitor = new WebhookMonitorPolling('/api/webhooks/recent', 3000);
    
    // Start monitoring
    monitor.connect(); // or monitor.start() for polling
    
    // Clean up on page unload
    window.addEventListener('beforeunload', () => {
        monitor.disconnect(); // or monitor.stop() for polling
    });
</script>
*/

// ============================================================================
// MODULE EXPORTS
// Guard with typeof check so this file can still be loaded as a plain browser
// <script> without errors (browsers have no `module` global).
// ============================================================================

/* istanbul ignore next */
if (typeof module !== 'undefined' && typeof module.exports !== 'undefined') {
    module.exports = {
        // Security / sanitization
        WEBHOOK_PAYLOAD_ALLOWLIST,
        sanitizeWebhookPayload,
        // Monitor classes
        WebhookMonitorWebSocket,
        WebhookMonitorSSE,
        WebhookMonitorPolling,
        // Standalone bootstrap (useful for test harnesses that want to spy on addEvent)
        addEvent,
    };
}
