# Architecture Decision Records

This folder records significant design decisions for AnchorKit, so the reasoning
does not live only in issue threads.

## Process

1. Copy [`0000-template.md`](./0000-template.md) to `NNNN-short-title.md` using the next number.
2. Fill in context, decision and consequences. Start with status `Proposed`.
3. Open a PR; once merged, set status to `Accepted`.
4. Never delete an ADR. If a decision changes, add a new ADR and mark the old one `Superseded by NNNN`.

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](./0001-cache-storage-tiers.md) | Cache storage tiers (persistent vs temporary) | Accepted |
| [0002](./0002-revocation-model.md) | Revocation model (soft-revoke) | Accepted |
