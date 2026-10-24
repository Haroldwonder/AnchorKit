# Webhook Event Monitor

## Overview

The Webhook Event Monitor is a real-time debugging panel that displays incoming webhook events from anchor callbacks. It helps developers visualize and debug anchor integrations by showing event types, timestamps, and raw payloads.

## Features

- **Real-time Event Display** - Events appear instantly as they arrive
- **Event Type Filtering** - Filter by deposit, withdrawal, KYC, quote, transfer, session, or attestation
- **Collapsible JSON Payloads** - View raw event data with syntax highlighting
- **Live Statistics** - Track total events, last event time, and events per minute
- **Event History** - Maintains last 100 events with automatic cleanup
- **Visual Event Types** - Color-coded badges for quick identification

## Event Types

### Deposit
- **Color**: Green
- **Data**: Amount, asset, user address
- **Use Case**: Track incoming deposits from users

### Withdrawal
- **Color**: Orange
- **Data**: Amount, asset, user address, destination
- **Use Case**: Monitor withdrawal requests and processing

### KYC Update
- **Color**: Blue
- **Data**: User address, status, verification level
- **Use Case**: Track KYC verification status changes

### Quote
- **Color**: Purple
- **Data**: Asset pair, rate, quote ID
- **Use Case**: Monitor quote requests and responses

### Transfer
- **Color**: Pink
- **Data**: Transfer ID, from/to addresses, amount
- **Use Case**: Track transfer initiation and progress

### Session
- **Color**: Yellow
- **Data**: Session ID, initiator address
- **Use Case**: Monitor session creation and management

### Attestation
- **Color**: Gray
- **Data**: Attestation ID, subject, issuer
- **Use Case**: Track attestation submissions

## Usage

### Opening the Monitor

```bash
# Open in browser
open webhook_monitor.html
```

### Integration with Backend

Replace the simulation code with actual webhook handling:

```javascript
// WebSocket connection (recommended for real-time)
const ws = new WebSocket('ws://localhost:8080/webhooks');

ws.onmessage = (event) => {
    const webhookData = JSON.parse(event.data);
    addEvent({
        id: ++eventCounter,
        type: webhookData.type,
        timestamp: webhookData.timestamp,
        payload: webhookData.payload
    });
};

// OR Server-Sent Events (SSE)
const eventSource = new EventSource('/api/webhook-stream');

eventSource.onmessage = (event) => {
    const webhookData = JSON.parse(event.data);
    addEvent({
        id: ++eventCounter,
        type: webhookData.type,
        timestamp: webhookData.timestamp,
        payload: webhookData.payload
    });
};

// OR HTTP Polling (fallback)
setInterval(async () => {
    const response = await fetch('/api/webhooks/recent');
    const events = await response.json();
    events.forEach(event => addEvent(event));
}, 2000);
```

### Backend Webhook Endpoint Example

```rust
// Soroban contract event emission
pub fn emit_webhook_event(
    env: &Env,
    event_type: WebhookEventType,
    payload_hash: BytesN<32>
) {
    let event = WebhookEvent {
        event_id: generate_event_id(env),
        event_type,
        timestamp: env.ledger().timestamp(),
        payload_hash,
    };
    
    env.events().publish(
        (symbol_short!("webhook"), symbol_short!("event")),
        event,
    );
}
```

## Controls

### Start/Stop Monitoring
- Click "Stop" to pause event collection
- Click "Start" to resume monitoring
- Status indicator shows current state (green = listening, red = stopped)

### Clear Events
- Click "Clear" to remove all events from the display
- Confirmation dialog prevents accidental clearing
- Event counter resets to 0

### Filter Events
- Use the dropdown to filter by event type
- Select "All Events" to show everything
- Filter updates display immediately

## Statistics

### Total Events
- Shows cumulative count of all received events
- Resets when "Clear" is clicked
- Maximum of 100 events stored

### Last Event
- Shows time since last event received
- Updates every second
- Formats: "Just now", "Xs ago", "Xm ago"

### Events/Min
- Shows event rate over last 60 seconds
- Useful for monitoring traffic patterns
- Updates in real-time

## Event Details

Each event displays:
- **Event Type Badge** - Color-coded with icon
- **Event ID** - Unique sequential identifier
- **Timestamp** - Precise time with milliseconds
- **Key Details** - Type-specific information
- **Raw Payload** - Collapsible JSON view

## Payload View

Click "View Raw Payload" to expand/collapse the JSON payload:
- Syntax-highlighted JSON
- Properly formatted with 2-space indentation
- Dark theme for readability
- Scrollable for large payloads

## Security Considerations

### Production Deployment

1. **Authentication Required**
   - Protect the monitor with authentication
   - Use session-based or token-based auth
   - Never expose publicly without protection

2. **Data Sanitization**
   - Redact sensitive information (PII, credentials)
   - Hash or truncate addresses for privacy
   - Filter out internal system data

3. **Rate Limiting**
   - Implement rate limits on webhook endpoints
   - Prevent DoS attacks
   - Monitor for unusual traffic patterns

4. **HTTPS Only**
   - Always use HTTPS in production
   - Secure WebSocket connections (wss://)
   - Validate SSL certificates

### Example Sanitization

```javascript
function sanitizePayload(payload) {
    const sanitized = { ...payload };
    
    // Redact sensitive fields
    if (sanitized.email) {
        sanitized.email = sanitized.email.replace(/(.{2}).*(@.*)/, '$1***$2');
    }
    
    // Truncate addresses
    if (sanitized.user) {
        sanitized.user = sanitized.user.substring(0, 8) + '...' + 
                        sanitized.user.substring(sanitized.user.length - 3);
    }
    
    return sanitized;
}
```

## Performance

### Optimization Tips

1. **Event Limit** - Keeps only last 100 events to prevent memory issues
2. **Efficient Rendering** - Uses innerHTML for batch updates
3. **Debounced Updates** - Stats update at fixed intervals
4. **Lazy Payload Loading** - JSON only rendered when expanded

### Memory Management

```javascript
// Automatic cleanup
if (events.length > 100) {
    events.pop(); // Remove oldest event
}

// Clear old rate data
const oneMinuteAgo = Date.now() - 60000;
recentEvents = recentEvents.filter(time => time > oneMinuteAgo);
```

## Customization

### Adding New Event Types

1. Add to event type enum in `src/types.rs`:
```rust
pub enum WebhookEventType {
    // ... existing types
    CustomEvent = 9,
}
```

2. Add styling in HTML:
```css
.event-type.custom {
    background: #your-color;
    color: #your-text-color;
}
```

3. Add rendering logic:
```javascript
case 'custom':
    details.push(['Field', event.payload.field]);
    break;
```

### Changing Event Limit

```javascript
// Change from 100 to your desired limit
if (events.length > 200) {
    events.pop();
}
```

### Adjusting Simulation Rate

```javascript
// Change interval (in milliseconds)
setInterval(() => {
    if (isMonitoring && Math.random() > 0.3) {
        simulateWebhookEvent();
    }
}, 5000); // 5 seconds instead of 3
```

## Troubleshooting

### Events Not Appearing

1. Check browser console for errors
2. Verify WebSocket/SSE connection
3. Confirm webhook endpoint is accessible
4. Check CORS settings if cross-origin

### Performance Issues

1. Reduce event limit (< 100)
2. Increase update intervals
3. Disable animations for large volumes
4. Use pagination for historical events

### Connection Drops

1. Implement reconnection logic
2. Add connection status indicator
3. Buffer events during disconnection
4. Show reconnection attempts

## Integration Examples

### With Express.js

```javascript
const express = require('express');
const app = express();

app.post('/webhook', (req, res) => {
    const event = {
        id: Date.now(),
        type: req.body.type,
        timestamp: new Date().toISOString(),
        payload: req.body.data
    };
    
    // Broadcast to connected clients
    wss.clients.forEach(client => {
        client.send(JSON.stringify(event));
    });
    
    res.status(200).send('OK');
});
```

### With Stellar Horizon

```javascript
const StellarSdk = require('stellar-sdk');
const server = new StellarSdk.Server('https://horizon-testnet.stellar.org');

server.operations()
    .forAccount(accountId)
    .cursor('now')
    .stream({
        onmessage: (operation) => {
            const event = {
                id: operation.id,
                type: operation.type,
                timestamp: operation.created_at,
                payload: operation
            };
            addEvent(event);
        }
    });
```

## Best Practices

1. **Use WebSockets** - For true real-time updates
2. **Implement Filtering** - Server-side for large volumes
3. **Add Pagination** - For historical event browsing
4. **Export Functionality** - Allow downloading event logs
5. **Search Capability** - Find specific events quickly
6. **Timestamp Precision** - Include milliseconds for debugging
7. **Error Handling** - Show connection errors clearly
8. **Responsive Design** - Works on mobile devices

## Related Documentation

- [API_REFERENCE.md](../API_REFERENCE.md) - Event schemas and error codes
- [TRANSACTION_STATE_TRACKER.md](./TRANSACTION_STATE_TRACKER.md) - Session event tracking
- [STATUS_MONITOR.md](./STATUS_MONITOR.md) - Health check events
- [SDK_CONFIG.md](./SDK_CONFIG.md) - SDK configuration

## Future Enhancements

- Event search and filtering
- Export to CSV/JSON
- Event replay functionality
- Custom alert rules
- Event aggregation and analytics
- Multi-anchor monitoring
- Historical event browser
- Performance metrics dashboard
