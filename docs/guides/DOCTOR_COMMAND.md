# AnchorKit Doctor Command

The `doctor` command performs automated environment checks to help developers quickly identify and resolve setup issues.

## Usage

```bash
anchorkit doctor
```

## What It Checks

### 1. Rust Toolchain
- Verifies that `rustc` is installed and accessible
- **Fix**: Install from https://rustup.rs

### 2. WASM Target
- Checks if `wasm32-unknown-unknown` target is installed
- **Fix**: Run `rustup target add wasm32-unknown-unknown`

### 3. Wallet Configuration
- Looks for wallet/keypair configuration in:
  - Environment variables: `STELLAR_SECRET_KEY`, `SOROBAN_SECRET_KEY`, `ANCHORKIT_SECRET_KEY`
  - Soroban CLI identity directory: `~/.config/soroban/identity`
- **Fix**: Set one of the environment variables or configure soroban identity

### 4. RPC Endpoint
- Checks if RPC endpoint is configured via environment variables:
  - `ANCHORKIT_RPC_URL`
  - `SOROBAN_RPC_URL`
  - `STELLAR_RPC_URL`
- Validates URL format and tests connectivity
- **Fix**: Set one of the environment variables with a valid RPC URL

### 5. Config Files
- Verifies that `configs/` directory exists
- Checks for at least one `.json` or `.toml` config file
- Validates that config files are readable
- **Fix**: Create `configs/` directory and add anchor configuration files

### 6. Network Connectivity
- Tests connectivity to known Stellar endpoints
- **Fix**: Check internet connection and firewall settings

## Example Output

### All Checks Passing
```
🔍 Running AnchorKit diagnostics...

✔ Rust toolchain detected
✔ WASM target installed
✔ Wallet configured
✔ RPC endpoint reachable
✔ Config files valid (6 found)
✔ Network responding

⏱  Completed in 0.01s

✅ All checks passed! Your environment is ready.
```

### Some Checks Failing
```
🔍 Running AnchorKit diagnostics...

✔ Rust toolchain detected
✔ WASM target installed
✖ Wallet not configured → set STELLAR_SECRET_KEY or configure soroban identity
✖ RPC endpoint not configured → set ANCHORKIT_RPC_URL, SOROBAN_RPC_URL, or STELLAR_RPC_URL
✔ Config files valid (6 found)
✔ Network responding

⏱  Completed in 0.01s

⚠️  Some checks failed. Please address the issues above.
```

## Exit Codes

- `0`: All checks passed
- `1`: One or more checks failed

This makes the command useful in CI/CD pipelines and automation scripts.

## Performance

The doctor command is designed to complete in under 2 seconds, making it suitable for:
- Pre-deployment validation
- CI/CD health checks
- Quick environment verification
- Troubleshooting setup issues

## Safety

The doctor command is completely non-destructive:
- No files are modified
- No network state is changed
- No credentials are stored or transmitted
- Safe to run at any time

## Shell Completions

AnchorKit can generate completion scripts for bash, zsh, and fish. Run the
`completions` subcommand once and source the output from your shell's startup
file.

### bash

```bash
# Install system-wide (requires sudo)
anchorkit completions bash | sudo tee /etc/bash_completion.d/anchorkit > /dev/null

# Or install for the current user only
mkdir -p ~/.local/share/bash-completion/completions
anchorkit completions bash > ~/.local/share/bash-completion/completions/anchorkit

# Then reload your shell or run:
source ~/.local/share/bash-completion/completions/anchorkit
```

### zsh

```zsh
# Pick a directory on your $fpath (create one if needed)
mkdir -p ~/.zsh/completions
anchorkit completions zsh > ~/.zsh/completions/_anchorkit

# Add the directory to $fpath in ~/.zshrc if it isn't already:
#   fpath=(~/.zsh/completions $fpath)
#   autoload -Uz compinit && compinit

# Then reload completions:
autoload -Uz compinit && compinit
```

### fish

```fish
anchorkit completions fish > ~/.config/fish/completions/anchorkit.fish
# Completions are picked up automatically on the next shell session.
```

## Integration with CI/CD

Example GitHub Actions workflow:

```yaml
- name: Verify environment
  run: |
    cd AnchorKit
    cargo run --bin anchorkit -- doctor
```

Example pre-deployment script:

```bash
#!/bin/bash
set -e

echo "Checking environment..."
anchorkit doctor

echo "Environment OK, proceeding with deployment..."
anchorkit deploy --network mainnet
```
