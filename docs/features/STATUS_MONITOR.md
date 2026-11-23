# Status Monitor

A standalone web-based monitoring tool for tracking anchor endpoint health in real-time.

## Overview

The status monitor provides visual indicators for anchor service reliability without requiring any integration with the AnchorKit smart contract. It's a pure client-side tool for operational monitoring.

## Features

- **Visual Status Indicators**
  - 🟢 **Online**: Endpoint responding normally (HTTP 200-299)
  - 🟡 **Degraded**: Slow response or client errors (timeout >5s or HTTP 400-499)
  - 🔴 **Offline**: Service unavailable (network error or HTTP 500+)

- **Monitored Endpoints**
  - `/info` - Anchor information endpoint
  - `/auth` - Authentication endpoint
  - `/transactions` - Transaction processing endpoint

- **Auto-refresh**: Checks all endpoints every 30 seconds
- **Configurable**: Adjustable base URL for different anchor services
- **Timestamps**: Shows last check time for each endpoint

## Usage

### Production Monitoring

1. Open `status-monitor.html` in a web browser
2. Enter your anchor base URL (e.g., `https://anchor.example.com`)
3. Monitor the status indicators

### Local Testing

1. Start the mock anchor server:
```bash
python3 mock-server.py
```

2. In another terminal, serve the status monitor:
```bash
python3 -m http.server 8000
```

3. Open `http://localhost:8000/status-monitor.html`

4. Use the default URL `http://localhost:8080` to test against the mock server

## Files

- `status-monitor.html` - Status monitoring dashboard
- `mock-server.py` - Mock anchor server for testing

## Technical Details

- **No dependencies**: Pure HTML/CSS/JavaScript
- **CORS-enabled**: Works with cross-origin anchor services
- **Timeout**: 5-second request timeout to detect degraded performance
- **Standalone**: Does not interact with AnchorKit smart contract

## Note on CI/CD

This monitoring tool is independent of the AnchorKit smart contract codebase. Any test failures in the Rust contract tests are unrelated to this monitoring functionality.
