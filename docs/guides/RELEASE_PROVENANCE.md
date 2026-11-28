# Release Artifact Provenance

Release assets published by this repository are built in GitHub Actions and
signed with build-provenance attestations.

## Contract WASM and CLI

When a GitHub Release is published, the release workflow builds:

- `anchorkit.wasm` - contract WASM
- `anchorkit-linux-x86_64` - Linux CLI binary
- `SHA256SUMS` - SHA-256 checksums for the binary artifacts

Before verifying an artifact, download it from the GitHub Release. GitHub CLI
`2.49.0` or newer can verify the attestation:

```bash
gh attestation verify anchorkit.wasm \
  --repo joshkirojohn212-jpg/AnchorKit

gh attestation verify anchorkit-linux-x86_64 \
  --repo joshkirojohn212-jpg/AnchorKit
```

The successful output identifies the repository, workflow, commit, and artifact
subject that produced the attestation. Verify the downloaded files' checksums
before using them:

```bash
sha256sum -c SHA256SUMS
```

## npm UI package

The `@anchorkit/ui-components` package is published with npm provenance during
the GitHub Release workflow. Install the package, then inspect or verify its
published provenance:

```bash
npm view @anchorkit/ui-components dist.attestations
npm audit signatures
```

`npm audit signatures` verifies registry signatures for the installed dependency
tree and reports provenance where available. A release created outside the
configured GitHub Actions workflow should not include a repository provenance
attestation.
