# Windows Setup Guide

This guide walks through setting up AnchorKit on Windows 10 or Windows 11, including Rust, the WASM target, Soroban CLI, and IDE configuration.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Install Rust](#install-rust)
3. [Add the WASM Target](#add-the-wasm-target)
4. [Install Soroban CLI](#install-soroban-cli)
5. [Install Stellar CLI (alternative)](#install-stellar-cli-alternative)
6. [Clone and Build AnchorKit](#clone-and-build-anchorkit)
7. [Run the Tests](#run-the-tests)
8. [IDE Setup](#ide-setup)
   - [VS Code](#vs-code)
   - [RustRover / IntelliJ](#rustrover--intellij)
9. [Configuration Validation](#configuration-validation)
10. [Troubleshooting](#troubleshooting)

---

## Prerequisites

- Windows 10 version 1903 or later, or Windows 11
- PowerShell 5.1+ (built-in) or [PowerShell 7+](https://aka.ms/powershell) (recommended)
- [Git for Windows](https://gitforwindows.org/) — includes Git Bash
- [Visual Studio Build Tools 2019 or 2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **"Desktop development with C++"** workload

> The C++ build tools are required by the Rust linker (`link.exe`). If you have a full Visual Studio installation the tools are already present.

---

## Install Rust

1. Download `rustup-init.exe` from <https://rustup.rs/>.
2. Run the installer. When prompted, select **option 1** (default installation).
3. Open a new PowerShell window and verify:

```powershell
rustc --version
cargo --version
```

You should see output similar to:

```
rustc 1.78.0 (9b00956e5 2024-04-29)
cargo 1.78.0 (54d8815d0 2024-04-18)
```

---

## Add the WASM Target

Soroban contracts compile to WebAssembly. Add the required target:

```powershell
rustup target add wasm32-unknown-unknown
```

Verify:

```powershell
rustup target list --installed
```

`wasm32-unknown-unknown` should appear in the list.

---

## Install Soroban CLI

The Soroban CLI lets you build, deploy, and invoke contracts locally and on testnet.

```powershell
cargo install --locked soroban-cli --features opt
```

After installation, confirm it is on your PATH:

```powershell
soroban --version
```

If the command is not found, add the Cargo bin directory to your `PATH`:

```powershell
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
# To make this permanent, add the line above to your PowerShell profile:
notepad $PROFILE
```

---

## Install Stellar CLI (alternative)

The Stellar CLI is the successor to the Soroban CLI and supports the same contract commands:

```powershell
cargo install --locked stellar-cli --features opt
```

```powershell
stellar --version
```

Both CLIs work with AnchorKit. This guide uses `soroban` in examples; substitute `stellar` if preferred.

---

## Clone and Build AnchorKit

```powershell
git clone https://github.com/Haroldwonder/AnchorKit.git
cd AnchorKit
```

Build the contract in release mode (produces an optimised WASM binary):

```powershell
cargo build --release --target wasm32-unknown-unknown
```

Build the host-side tooling (CLI, tests, examples):

```powershell
cargo build --release
```

---

## Run the Tests

```powershell
# Run the full test suite
cargo test

# Run with verbose output
cargo test --verbose

# Run a specific test module
cargo test contract_tests
```

To validate configuration files (PowerShell equivalents of the bash scripts):

```powershell
.\validate_all.ps1
.\pre_deploy_validate.ps1
```

---

## IDE Setup

### VS Code

1. Install [VS Code](https://code.visualstudio.com/).
2. Install the **rust-analyzer** extension (search for `rust-lang.rust-analyzer` in the Extensions panel).
3. Install the **Even Better TOML** extension for `Cargo.toml` editing.
4. Open the `AnchorKit` folder: **File → Open Folder**.

Recommended workspace settings (`.vscode/settings.json`):

```json
{
  "rust-analyzer.cargo.buildScripts.enable": true,
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

To run tests from the editor, install the **rust-analyzer** test runner integration — click the **▶ Run Test** code lens that appears above each `#[test]` function.

### RustRover / IntelliJ

1. Install [RustRover](https://www.jetbrains.com/rust/) (free for non-commercial use) or IntelliJ IDEA with the Rust plugin.
2. Open the `AnchorKit` directory as a project (**File → Open**).
3. RustRover auto-detects `Cargo.toml` and indexes the project. Wait for the initial indexing to complete.
4. Run tests with the green **▶** gutter icons next to each test function.

---

## Configuration Validation

AnchorKit ships PowerShell scripts for validating TOML configs and running the doctor check:

```powershell
# Validate all configuration files
.\validate_all.ps1

# Run pre-deployment checks
.\pre_deploy_validate.ps1

# Validate a specific config
.\validate_health_score.ps1
```

The CLI doctor command checks your environment end-to-end:

```powershell
cargo run --bin anchorkit -- doctor
```

See [`docs/guides/DOCTOR_COMMAND.md`](./docs/guides/DOCTOR_COMMAND.md) for what each check covers.

---

## Troubleshooting

### `error: linker 'link.exe' not found`

The Microsoft C++ linker is missing. Install or repair Visual Studio Build Tools and ensure the **"Desktop development with C++"** workload is selected.

### `error[E0463]: can't find crate for 'std'` when targeting wasm32

The WASM target is not installed. Run:

```powershell
rustup target add wasm32-unknown-unknown
```

### `soroban: command not found` after `cargo install`

The Cargo bin directory is not in your `PATH`. Add it:

```powershell
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

To persist across sessions, add the same line to your PowerShell profile (`$PROFILE`).

### Long compile times on Windows

Windows Defender can significantly slow Rust builds. Add an exclusion for the repository directory and your Cargo cache:

1. Open **Windows Security → Virus & threat protection → Manage settings**.
2. Under **Exclusions**, add the `AnchorKit` folder and `%USERPROFILE%\.cargo`.

### `CRLF` line ending warnings in Git

Run the following to configure Git to check out files with LF endings (required for shell scripts in the repo):

```powershell
git config --global core.autocrlf input
```

### PowerShell execution policy

If scripts fail with "running scripts is disabled on this system", enable script execution for the current session:

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

---

For further help, see the [QUICK_START.md](./QUICK_START.md) guide or open an issue on the [GitHub repository](https://github.com/Haroldwonder/AnchorKit/issues).
