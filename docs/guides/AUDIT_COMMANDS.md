# AnchorKit Audit Commands

Audit logs in AnchorKit are stored on-chain in the Soroban smart contract. The `audit` CLI commands provide tools to fetch, filter, and display these audit log entries.

## Overview

Audit logs track all significant operations performed on the AnchorKit contract, including:
- Session creation
- Attestations submitted
- Quote submissions
- Attestor registration/revocation
- Authorization events
- Compliance checks

Each audit log entry contains:
- **Log ID**: Unique monotonic identifier
- **Session ID**: Associated session context
- **Actor**: Address performing the operation
- **Operation Type**: Kind of operation (e.g., "attest", "register_attestor")
- **Operation Index**: Order within the session
- **Timestamp**: When the operation occurred
- **Status**: Success/failure status
- **Result**: Operation outcome summary (e.g., "attestation_id=42")

## Commands

### `anchorkit audit get`

Fetch and display a single audit log entry by ID.

#### Syntax

```bash
anchorkit audit get <LOG_ID>
```

#### Parameters

- `<LOG_ID>`: The audit log entry ID (numeric)

#### Environment Variables

- `ANCHORKIT_RPC_URL`: Soroban RPC endpoint (required)
- `STELLAR_SECRET_KEY` or `SOROBAN_IDENTITY`: Wallet for signing transactions (required)

#### Examples

```bash
# Fetch audit log entry #42
anchorkit audit get 42

# Output:
#   Log ID:          42
#   Session ID:      5
#   Actor:           GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
#   Operation:       attest (index: 2)
#   Timestamp:       1234567890 (14d 02h 30m 15s)
#   Status:          success
#   Result:          attestation_id=100
```

#### Exit Codes

- `0`: Success, entry found and displayed
- `1`: Entry not found or network error

---

### `anchorkit audit list`

List and filter audit log entries by session ID.

#### Syntax

```bash
anchorkit audit list --session <SESSION_ID> [OPTIONS]
```

#### Parameters

- `--session <SESSION_ID>`: Filter by session ID (required)
- `--from <LOG_ID>`: Start from this log ID (optional, defaults to 0)
- `--to <LOG_ID>`: End at this log ID (optional, defaults to latest)
- `--format <FORMAT>`: Output format: `text` (default), `json`, or `csv`
- `--pretty`: Pretty-print JSON output (only with `--format json`)

#### Output Formats

##### Text Format (Default)

Human-readable table format with box drawing characters:

```bash
anchorkit audit list --session 5
```

Output:
```
◈ Fetching audit logs for session 5

┌─ Audit Log Entries (Session 5) ──────────────────────────
  │ Log ID:     40 │ Op: initialize │ Status: success
  │ Session:    5 │ Actor: GXXXXXX...
  │ Timestamp:  14d 02h 30m 15s (op_index: 0)
  │ Result:     contract_initialized
├───────────────────────────────────────────────────────────
  │ Log ID:     41 │ Op: register_attestor │ Status: success
  │ Session:    5 │ Actor: GXXXXXX...
  │ Timestamp:  14d 02h 30m 20s (op_index: 1)
  │ Result:     attestor_registered
├───────────────────────────────────────────────────────────
  │ Log ID:     42 │ Op: attest │ Status: success
  │ Session:    5 │ Actor: GXXXXXX...
  │ Timestamp:  14d 02h 30m 25s (op_index: 2)
  │ Result:     attestation_id=100
└───────────────────────────────────────────────────────────

✔ Retrieved 3 audit log entries
```

##### JSON Format

Structured JSON output suitable for processing:

```bash
anchorkit audit list --session 5 --format json --pretty
```

Output:
```json
[
  {
    "log_id": 40,
    "session_id": 5,
    "actor": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "operation_type": "initialize",
    "operation_index": 0,
    "timestamp": 1234567890,
    "status": "success",
    "result": "contract_initialized"
  },
  {
    "log_id": 41,
    "session_id": 5,
    "actor": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "operation_type": "register_attestor",
    "operation_index": 1,
    "timestamp": 1234567895,
    "status": "success",
    "result": "attestor_registered"
  }
]
```

##### CSV Format

Comma-separated values for spreadsheet analysis:

```bash
anchorkit audit list --session 5 --format csv > session_5_audit.csv
```

Output:
```csv
log_id,session_id,actor,operation_type,operation_index,timestamp,status,result
40,5,GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX,initialize,0,1234567890,success,contract_initialized
41,5,GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX,register_attestor,1,1234567895,success,attestor_registered
42,5,GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX,attest,2,1234567900,success,attestation_id=100
```

#### Environment Variables

- `ANCHORKIT_RPC_URL`: Soroban RPC endpoint (required)
- `STELLAR_SECRET_KEY` or `SOROBAN_IDENTITY`: Wallet for signing transactions (required)

#### Examples

```bash
# List all audit entries for session 5
anchorkit audit list --session 5

# List entries 100-150 for session 5 in JSON format
anchorkit audit list --session 5 --from 100 --to 150 --format json --pretty

# Export to CSV for analysis
anchorkit audit list --session 5 --format csv > audit_export.csv
```

#### Exit Codes

- `0`: Success, entries retrieved
- `1`: Invalid format or network error

---

## Implementation Details

### On-Chain Contract Methods

The audit commands use these contract methods:

#### `get_audit_log(log_id: u64) -> AuditLog`

Retrieves a single audit log entry by its ID.

**Returns**: `AuditLog` struct with:
- `log_id: u64` - Unique entry ID
- `session_id: u64` - Associated session
- `actor: Address` - Address that performed the operation
- `operation: OperationContext` - Operation details

**Errors**: Returns error if entry not found

#### `get_audit_log_range(from_id: u64, to_id: u64) -> Vec<AuditLog>`

Retrieves audit logs in a range, capped at 100 entries per call.

**Parameters**:
- `from_id: u64` - Start ID (inclusive)
- `to_id: u64` - End ID (inclusive)

**Returns**: `Vec<AuditLog>` with entries in [from_id, to_id], max 100 entries

**Notes**:
- If range exceeds 100 entries, only first 100 are returned
- Entries with no log are silently skipped
- Useful for pagination when fetching large audit logs

### RPC Integration

The CLI commands communicate with the Soroban contract via the RPC endpoint specified in environment variables. The general flow is:

1. **Build Transaction**: Construct a contract invoke transaction for the desired method
2. **Sign**: Sign with the provided wallet credentials
3. **Submit**: Send to RPC endpoint's `/simulateTransaction` endpoint
4. **Parse**: Extract and decode the returned `AuditLog` structures
5. **Format**: Convert to display format (text/JSON/CSV)

### Pagination Strategy

When fetching large audit logs:

```
for page in 0..total_pages:
    batch = contract.get_audit_log_range(
        from_id = page * 100,
        to_id = (page + 1) * 100 - 1
    )
    if batch.len() < 100:
        break  # reached end
    accumulate batch results
```

This minimizes RPC calls while handling large datasets efficiently.

### Session Filtering

The `--session` flag filters entries client-side:

1. Fetch audit logs in ranges (100 per call)
2. Keep only entries where `entry.session_id == --session value`
3. Continue until all entries fetched or limit reached

This allows quick filtering without requiring contract-side session index queries.

---

## Troubleshooting

### "Audit log entry N not found"

**Cause**: The log ID doesn't exist or has been pruned.

**Solution**:
1. Check if the log ID is within the current range (pruning removes old entries)
2. Use `audit list --session X` to find valid entries in a session
3. Verify the log ID value (should be positive integer)

### "No audit log entries found for session N"

**Cause**: The session ID is invalid or has no operations.

**Solution**:
1. Verify the session ID: `anchorkit audit list --session <ID>`
2. Check if session exists: sessions have 24-hour TTL
3. Confirm you're connecting to correct RPC endpoint

### Network Connection Errors

**Cause**: RPC endpoint unreachable or misconfigured.

**Solution**:
1. Check RPC URL: `echo $ANCHORKIT_RPC_URL`
2. Test connectivity: `curl -X POST $ANCHORKIT_RPC_URL/...`
3. Verify firewall/proxy settings
4. Try alternative RPC endpoint if available

### "Transaction signature verification failed"

**Cause**: Invalid or missing wallet credentials.

**Solution**:
1. Check wallet is configured: `anchorkit doctor`
2. Verify secret key: `echo $STELLAR_SECRET_KEY | wc -c` (should be ~56 chars)
3. Re-authenticate: `soroban config identity show default`
4. Use correct network: ensure RPC endpoint matches wallet's network

---

## Performance Considerations

### Query Limits

- Single `get_audit_log()`: ~100ms RPC latency
- Range query `get_audit_log_range()`: ~200-500ms RPC latency (100 entries)
- Pagination (1000 entries): ~10 RPC calls, ~2-5 seconds total

### Best Practices

1. **Use `--format json` for automation** - Faster parsing than text
2. **Filter by session early** - Reduces number of entries to fetch
3. **Batch requests** - Use `--from` and `--to` to control page size
4. **Cache results** - Store audit data locally if querying repeatedly
5. **Monitor rate limits** - Some RPC endpoints limit concurrent requests

---

## Examples

### Audit all operations in a session

```bash
anchorkit audit list --session 5 --format json --pretty > session_audit.json
```

### Check most recent operations

```bash
# Get latest 100 audit entries (requires knowing the latest log_id)
anchorkit audit list --session 5 --from 900 --to 999 --format text
```

### Monitor specific operation

```bash
# Watch for "attest" operations
anchorkit audit list --session 5 | grep -i attest
```

### Export for compliance

```bash
# Export all audit logs for a session as CSV
anchorkit audit list --session 5 --format csv | \
  awk -F, '$7=="success" {print}' > successful_ops.csv
```

### Compare sessions

```bash
# Export two sessions for comparison
anchorkit audit list --session 5 --format json > session_5.json
anchorkit audit list --session 6 --format json > session_6.json
# Use diff or jq to compare
jq -s '.[0] - .[1]' session_5.json session_6.json
```
