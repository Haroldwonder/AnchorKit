# Security Policy

## Supported Versions

The table below shows which versions of AnchorKit currently receive security patches. If your version is not listed as supported, upgrade to the latest release before filing a report.

| Version | Supported          | Notes                                      |
|---------|--------------------|--------------------------------------------|
| 0.1.x   | ✅ Yes             | Current stable release — actively patched  |
| < 0.1   | ❌ No              | Pre-release; no further security fixes     |

Only the **latest patch release** within a supported minor version receives security fixes. For example, if `0.1.3` is the latest patch, `0.1.0`–`0.1.2` will not receive backported fixes. We recommend always running the latest published version.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

If you discover a security vulnerability in AnchorKit, please report it responsibly using one of the following channels:

### GitHub Private Vulnerability Reporting (Preferred)

Use GitHub's built-in private reporting feature:

1. Go to the [Security tab](https://github.com/Haroldwonder/AnchorKit/security) of this repository.
2. Click **"Report a vulnerability"**.
3. Fill in the details and submit.

This keeps the report confidential until a fix is ready and allows coordinated disclosure.

### Email

If you prefer email, send your report to: **security@anchorkit.dev**

Please include the following in your report:

- A clear description of the vulnerability
- Steps to reproduce or a proof-of-concept
- The potential impact (e.g., unauthorized access, replay attack bypass, data exposure)
- Any suggested mitigations, if you have them

## Response Process

1. **Acknowledgement** — We will acknowledge receipt of your report within **72 hours**.
2. **Assessment** — We will assess severity and scope within **7 days**.
3. **Fix & Disclosure** — We aim to release a fix within **30 days** of confirmation. For critical issues we will expedite this timeline.
4. **Credit** — With your permission, we will credit you in the release notes and/or `CHANGELOG.md`.

## Disclosure Policy

We follow a **coordinated disclosure** model:

- Please give us reasonable time to investigate and patch before any public disclosure.
- We will notify you when a fix is released and coordinate a disclosure date with you.
- We will not take legal action against researchers who report vulnerabilities in good faith and follow this policy.

## Scope

The following are in scope for vulnerability reports:

- Smart contract logic in `src/` (authorization bypasses, replay attack vectors, storage manipulation)
- SEP-10 JWT verification (`src/sep10_jwt.rs`)
- Cryptographic operations (payload hashing, signature verification)
- Dependency vulnerabilities that directly affect AnchorKit

The following are **out of scope**:

- Vulnerabilities in the Stellar/Soroban platform itself (report those to [Stellar's security team](https://www.stellar.org/bug-bounty-program))
- Issues in example scripts or documentation only
- Theoretical vulnerabilities without a realistic attack path

## Security Design Notes

For an overview of AnchorKit's authorization model, access control tiers, and replay protection mechanisms, see:

- **[docs/features/AUTHORIZATION_MODEL.md](./docs/features/AUTHORIZATION_MODEL.md)** — Admin-only, self-only, and public function access tiers
- **[docs/features/SEP10_AUTH.md](./docs/features/SEP10_AUTH.md)** — SEP-10 authentication flow
- **[docs/features/ERROR_CODES_REFERENCE.md](./docs/features/ERROR_CODES_REFERENCE.md)** — Stable error codes including auth failures
