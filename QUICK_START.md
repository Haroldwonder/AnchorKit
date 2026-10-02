# AnchorKit Quick Start

Get up and running with AnchorKit on Stellar testnet in minutes.

## Prerequisites

- Rust toolchain (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Soroban CLI (`cargo install --locked soroban-cli`)

## 1. Build

```bash
cargo build --release
```

## 2. Start from the testnet example config

The fastest way to get started is with the minimal reference config:

```bash
cp configs/testnet-example.json configs/my-anchor.json
```

Open `configs/my-anchor.json` and replace the placeholder values:

| Field | What to change |
|---|---|
| `contract.name` | Your anchor name (lowercase, hyphens only) |
| `attestors[0].address` | Your attestor's Stellar public key (starts with `G`) |
| `attestors[0].endpoint` | Your attestor's HTTPS endpoint URL |

Contract interactions are performed through Soroban transactions, not direct JavaScript session helpers. Use the Soroban CLI or the Stellar JavaScript SDK's contract invocation APIs when you need to call the contract.

## 3. Validate your config

```bash
cargo run --bin anchorkit -- validate configs/my-anchor.json
```

Expected output:
```
✔ configs/my-anchor.json: valid JSON
```

For strict schema validation (field lengths, patterns, enums):

```bash
python validate_config.py
```

## 4. Run environment diagnostics

```bash
cargo run --bin anchorkit -- doctor
```

## 5. Register an attestor

```bash
cargo run --bin anchorkit -- register \
  --address GYOUR_ATTESTOR_ADDRESS_HERE \
  --services deposits,withdrawals,kyc \
  --endpoint https://your-anchor.example.com
```

## Config reference

The config requires three top-level sections — `contract`, `attestors`, and
`sessions` (per `config_schema.json`'s root `required` array). A fourth
section, `deployment`, is defined by the schema and shown below, but it is
**optional**: a config with only the three required sections validates
successfully.

```json
{
  "contract": {
    "name": "my-anchor",        // lowercase alphanumeric + hyphens
    "version": "1.0.0",         // semver
    "network": "stellar-testnet" // stellar-testnet | stellar-mainnet | stellar-futurenet
  },
  "attestors": {
    "registry": [
      {
        "name": "kyc-provider",
        "address": "G...",       // 56-char Stellar public key
        "endpoint": "https://...",
        "role": "kyc-issuer",   // see valid roles below
        "enabled": true
      }
    ]
  },
  "sessions": {
    "enable_session_tracking": true,
    "session_timeout_seconds": 3600,
    "operations_per_session": 100,
    "audit_log_retention_days": 30
  },
  "deployment": {                   // optional — not in the schema's required list
    "admin_key": "G...",        // 56-char Stellar public key
    "rpc_endpoint": "https://soroban-testnet.stellar.org",
    "network": "stellar-testnet"
  }
}
```

Valid attestor roles: `kyc-issuer`, `transfer-verifier`, `compliance-approver`, `rate-provider`, `attestor`

See `configs/testnet-example.json` for the annotated minimal config, or the more complete examples:

- `configs/fiat-on-off-ramp.json` — fiat deposit/withdrawal with KYC
- `configs/remittance-anchor.json` — cross-border remittance with AML
- `configs/stablecoin-issuer.json` — stablecoin issuance with reserve audits

## Further reading

- [Architecture overview](docs/ARCHITECTURE.md)
- [CLI doctor command](docs/guides/DOCTOR_COMMAND.md)
- [SEP-10 authentication](docs/features/SEP10_AUTH.md)
- [Error codes reference](docs/features/ERROR_CODES_REFERENCE.md)
