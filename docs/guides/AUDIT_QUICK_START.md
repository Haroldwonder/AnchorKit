# Quick Start: Audit Commands

## Installation

Build the AnchorKit CLI with the new audit commands:

```bash
cd AnchorKit
cargo build --bin anchorkit --release
```

The binary will be available at `target/release/anchorkit`.

## Basic Usage

### Single Entry Lookup

```bash
# Fetch audit log entry by ID
anchorkit audit get 42
```

Expected output:
```
◈ Fetching audit log entry 42

  Log ID:          42
  Session ID:      5
  Actor:           GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
  Operation:       attest (index: 2)
  Timestamp:       14d 02h 30m 15s
  Status:          success
  Result:          attestation_id=100

✔ Entry retrieved successfully
```

### Session-Based Filtering

```bash
# List all audit logs for a session
anchorkit audit list --session 5

# With custom range
anchorkit audit list --session 5 --from 40 --to 50

# JSON output for processing
anchorkit audit list --session 5 --format json --pretty

# CSV for spreadsheet analysis
anchorkit audit list --session 5 --format csv > audit_export.csv
```

## Environment Setup

Required:
```bash
export ANCHORKIT_RPC_URL=https://soroban-testnet.stellar.org:443
export STELLAR_SECRET_KEY=SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

Or use Soroban identity:
```bash
soroban config identity generate --global my-identity
export SOROBAN_IDENTITY=my-identity
```

## Examples

### Export for Compliance

```bash
# Get all successful operations in a session
anchorkit audit list --session 5 --format json > session_5.json
jq '.[] | select(.status == "success")' session_5.json
```

### Monitor Specific Operations

```bash
# Find all attestation operations
anchorkit audit list --session 5 | grep -i attest

# Count operations by type
anchorkit audit list --session 5 --format json | jq -r '.[].operation_type' | sort | uniq -c
```

### Compare Sessions

```bash
# Export two sessions
anchorkit audit list --session 5 --format json > s5.json
anchorkit audit list --session 6 --format json > s6.json

# Find differences
diff <(jq '.' s5.json) <(jq '.' s6.json)
```

## Troubleshooting

### "Audit log entry not found"

- Check the log ID is valid: `anchorkit audit list --session <N>` shows valid range
- Old entries may have been pruned: check for more recent entries

### "No entries for session"

- Verify session ID exists: `anchorkit audit list --session 1`
- Sessions expire after 24 hours
- Check you're on the correct network (RPC endpoint matches wallet)

### Connection Errors

```bash
# Test RPC connection
curl -X POST $ANCHORKIT_RPC_URL -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getNetwork","params":{}}'

# Verify credentials
echo "✓ RPC:" $ANCHORKIT_RPC_URL
echo "✓ Wallet:" ${STELLAR_SECRET_KEY:0:10}...
```

## Next Steps

See full documentation in:
- User guide: `docs/guides/AUDIT_COMMANDS.md`
- Technical guide: `docs/internal/AUDIT_COMMANDS_IMPLEMENTATION.md`
