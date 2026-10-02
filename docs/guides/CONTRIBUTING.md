# Contributing to AnchorKit

Thank you for your interest in contributing to AnchorKit! This document provides guidelines and instructions for contributing to this project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Environment Setup](#development-environment-setup)
- [Makefile Shortcuts](#makefile-shortcuts)
- [Running Tests](#running-tests)
- [Code Style Guidelines](#code-style-guidelines)
- [Branch Naming Conventions](#branch-naming-conventions)
- [Pull Request Process](#pull-request-process)
- [Issue and PR Templates](#issue-and-pr-templates)
- [Documentation](#documentation)

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please be respectful and constructive in all interactions.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/AnchorKit.git
   cd AnchorKit
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/Haroldwonder/AnchorKit.git
   ```
4. **Create a feature branch** (see [Branch Naming Conventions](#branch-naming-conventions))

## Development Environment Setup

### Rust + Soroban Toolchain

AnchorKit is built using Rust and the Soroban SDK for Stellar smart contracts.

1. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

2. **Install Soroban CLI**:
   ```bash
   cargo install --locked soroban-cli
   ```

3. **Add WASM target**:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

4. **Verify installation**:
   ```bash
   rustc --version
   soroban --version
   ```

### Node.js for UI Components

The UI components are built with React and TypeScript.

1. **Install Node.js** (v18 or later recommended):
   ```bash
   # Using nvm (recommended)
   curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
   nvm install 18
   nvm use 18
   ```

2. **Install UI dependencies**:
   ```bash
   cd ui
   npm install
   ```

3. **Verify installation**:
   ```bash
   node --version
   npm --version
   ```

## Makefile Shortcuts

A `Makefile` is provided at the project root for common tasks. It defines
these eleven targets:

```bash
# Build & test
make build          # cargo build --release
make build-wasm     # cargo build --release --target wasm32-unknown-unknown
make test           # cargo test
make bench          # cargo bench (criterion benchmarks for contract hot paths)
make coverage       # Rust (tarpaulin) + UI coverage, best-effort

# Lint & format
make lint           # cargo clippy -- -D warnings
make fmt            # cargo fmt

# UI
make storybook      # cd ui && npm run storybook

# Validation & deploy
make validate       # ./validate_all.sh (repo config + Rust validation)
make deploy-testnet # deploy to Stellar testnet (depends on build-wasm)
make clean          # cargo clean

make help           # list all targets
```

`make build-wasm` produces the WASM artifact that `make deploy-testnet`
uploads, so you can build and inspect the artifact without deploying.
`make validate` runs the repository's config and Rust validation scripts and is
the quickest way to check a change before opening a PR.

> **Note**: Requires `make` (available by default on Linux and macOS).

## Running Tests

### Rust Tests

Run the complete test suite for the Rust crate:

```bash
# Run all tests
cargo test

# Run tests with verbose output
cargo test --verbose

# Run specific test module
cargo test domain_validator

# Run tests with output
cargo test -- --nocapture
```

### UI Tests

Run the test suite for the UI components:

```bash
cd ui

# Run all tests
npm test

# Run tests in watch mode
npm run test:watch

# Run tests with coverage
npm run test:coverage
```

### Configuration Validation

Validate configuration files:

```bash
# Linux/macOS
./validate_all.sh

# Windows
.\validate_all.ps1
```

## Code Style Guidelines

### Rust

- **Formatting**: Use `rustfmt` to format your code
  ```bash
  cargo fmt
  ```
- **Linting**: Use `clippy` to catch common issues
  ```bash
  cargo clippy -- -D warnings
  ```
- **Documentation**: Add doc comments for public APIs
- **Error Handling**: Use the `AnchorKitError` type for error handling

### TypeScript/React (UI)

- **Linting**: Use ESLint with the project configuration
  ```bash
  cd ui
  npm run lint
  ```
- **Type Checking**: Ensure TypeScript compilation passes
  ```bash
  cd ui
  npm run type-check
  ```
- **Formatting**: Follow the project's ESLint configuration

### General Guidelines

- Write clear, descriptive commit messages
- Keep commits focused and atomic
- Add tests for new functionality
- Update documentation as needed
- Follow existing code patterns and conventions

## Branch Naming Conventions

Use descriptive branch names with the following prefixes:

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions or modifications
- `chore/` - Maintenance tasks

Examples:
- `feature/add-session-management`
- `fix/domain-validation-edge-case`
- `docs/update-api-specification`

## Pull Request Process

1. **Update your branch** with the latest upstream changes:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Ensure all tests pass**:
   ```bash
   cargo test
   cd ui && npm test
   ```

3. **Run linters**:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cd ui && npm run lint
   ```

4. **Commit your changes**:
   ```bash
   git add .
   git commit -m "feat: descriptive commit message"
   ```

5. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Create a Pull Request** on GitHub

7. **Fill out the PR template** with:
   - Description of changes
   - Related issue numbers
   - Testing performed
   - Screenshots (if applicable)

### PR Review Process

- All PRs require at least one approval
- Address review feedback promptly
- Keep PRs focused and reasonably sized
- Update documentation if needed
- Ensure CI checks pass

## Issue and PR Templates

### Issue Template

When creating an issue, please include:

- **Description**: Clear description of the issue or feature
- **Steps to Reproduce**: For bugs, provide steps to reproduce
- **Expected Behavior**: What should happen
- **Actual Behavior**: What actually happens
- **Environment**: OS, Rust version, Node version
- **Screenshots**: If applicable

### PR Template

When creating a PR, please include:

- **Description**: What does this PR do?
- **Related Issues**: Link to related issues (e.g., "Closes #123")
- **Type of Change**: Bug fix, feature, documentation, etc.
- **Testing**: How was this tested?
- **Checklist**:
  - [ ] Tests added/updated
  - [ ] Documentation updated
  - [ ] Code formatted with rustfmt/prettier
  - [ ] Linter passes

## Documentation

### Existing Documentation

- **[QUICK_START.md](../../QUICK_START.md)** - Quick reference guide with examples
- **[README.md](../../README.md)** - Main project documentation
- **[API_REFERENCE.md](../API_REFERENCE.md)** - API specification and reference
- **[ERROR_IMPLEMENTATION_GUIDE.md](./ERROR_IMPLEMENTATION_GUIDE.md)** - Technical implementation details

### Writing Documentation

- Use clear, concise language
- Include code examples
- Keep documentation up-to-date with code changes
- Use Markdown formatting

## Questions or Issues?

If you have questions or encounter issues:

1. Check the [documentation](#existing-documentation)
2. Search [existing issues](https://github.com/Haroldwonder/AnchorKit/issues)
3. Create a new issue if needed

Thank you for contributing to AnchorKit!
