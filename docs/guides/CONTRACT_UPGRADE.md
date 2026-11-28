# Contract Upgrade Procedure

AnchorKit can replace its own WASM in place with the admin-only
`upgrade(new_wasm_hash: BytesN<32>)` entry point. The contract address, the
admin, and all storage (attestors, attestations, sessions, audit logs, ...)
stay the same, so integrators don't need to move to a new contract ID.

`upgrade` is a code upgrade. It sits alongside `migrate(migration_name)`, which
handles data migrations: use `upgrade` to change the code, then `migrate` if
the new code expects a different storage layout.

## Entry points

| Function | Access | Description |
|---|---|---|
| `upgrade(new_wasm_hash)` | Admin | Calls `env.deployer().update_current_contract_wasm(new_wasm_hash)`, bumps the contract version and emits an `upgraded` event |
| `get_contract_version()` | Public | Returns the current code version. It is `1` before the first upgrade and goes up by 1 with each upgrade |

### `upgraded` event

Topics: `("admin", "upgraded")`

```rust
pub struct ContractUpgraded {
    pub old_version: u32,
    pub new_version: u32,       // old_version + 1
    pub new_wasm_hash: BytesN<32>,
    pub timestamp: u64,
}
```

## Procedure

1. **Build the new WASM**

   ```bash
   anchorkit build --release
   # or: cargo build --target wasm32-unknown-unknown --release
   ```

2. **Upload (install) the WASM** to the network. This only stores the code and
   prints its hash. It does not change the running contract yet.

   ```bash
   stellar contract upload \
     --wasm target/wasm32-unknown-unknown/release/anchorkit.wasm \
     --source <ADMIN_ACCOUNT> \
     --network testnet
   # => <NEW_WASM_HASH> (64 hex characters)
   ```

3. **Check the current version** so you can confirm the upgrade later:

   ```bash
   stellar contract invoke --id <CONTRACT_ID> --source <ADMIN_ACCOUNT> \
     --network testnet -- get_contract_version
   ```

4. **Upgrade the contract.** The admin account must sign:

   ```bash
   stellar contract invoke --id <CONTRACT_ID> --source <ADMIN_ACCOUNT> \
     --network testnet -- upgrade --new_wasm_hash <NEW_WASM_HASH>
   ```

   The new code takes effect after this invocation completes. Calls from
   accounts other than the admin fail authorization and leave the code unchanged.

5. **Run data migrations** if the release notes for the new version call for
   them:

   ```bash
   stellar contract invoke --id <CONTRACT_ID> --source <ADMIN_ACCOUNT> \
     --network testnet -- migrate --migration_name <MIGRATION_NAME>
   ```

6. **Verify.** `get_contract_version` should return the previous value + 1, and
   an `("admin", "upgraded")` event with the old and new version should appear
   in the contract's events.

## Notes

- Always rehearse the upgrade on testnet (or futurenet) first.
- `upgrade` also works while the contract is paused. This lets you pause, ship
  a fix, and then unpause.
- Keep the new code's storage keys compatible with the old ones, or ship a
  `migrate` step that converts them. Otherwise existing data can become
  unreadable.
- If the upload in step 2 failed or used a different hash, the upgrade fails
  and the contract keeps running its current code.
