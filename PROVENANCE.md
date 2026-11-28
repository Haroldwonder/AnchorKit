# Release provenance

GitHub Actions generates Sigstore attestations for the release artifacts and npm
adds publication provenance to the UI package.

## Contract WASM

Download the artifact from the GitHub release, then verify it against this
repository and the release workflow:

```bash
gh release download "<tag>" --repo bitstarkbridge/AnchorKit --pattern anchorkit.wasm
gh attestation verify anchorkit.wasm \
  --repo bitstarkbridge/AnchorKit \
  --format json
```

The verification is successful only when the attested subject digest matches
`anchorkit.wasm` and the attestation was produced for `bitstarkbridge/AnchorKit`.

## CLI binaries

CLI binaries named `anchorkit-cli*` are released from `cli/dist/` and use the
same attestation workflow:

```bash
gh release download "<tag>" --repo bitstarkbridge/AnchorKit --pattern "anchorkit-cli*"
gh attestation verify anchorkit-cli \
  --repo bitstarkbridge/AnchorKit \
  --format json
```

For a platform-suffixed binary, verify the exact downloaded filename instead of
`anchorkit-cli`.

## UI npm package

The UI package is published with `npm publish --provenance`. After installing
the package, verify its signatures and provenance metadata:

```bash
npm audit signatures
npm view "<ui-package-name>" dist.attestations --json
```

Npm’s provenance view is also available on the package page for the exact
published version.
