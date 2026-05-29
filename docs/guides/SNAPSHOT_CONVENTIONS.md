# Snapshot Directory Conventions

AnchorKit uses the Soroban SDK's built-in snapshot mechanism to record contract
ledger state during tests. This document explains how snapshot paths are derived,
how to keep the `test_snapshots/` directory clean, and what went wrong in issue
#528.

## How Soroban derives snapshot paths

When a test runs, the Soroban test environment records the full ledger state
(auth records, storage entries, ledger metadata) to a JSON file named after the
test. The file path is built from the Rust **module hierarchy**:

```
test_snapshots/<outer-module>/<inner-module>/test_name.1.json
```

For a test file `src/foo_tests.rs` declared in `lib.rs` as:

```rust
#[cfg(test)]
mod foo_tests;
```

…and whose contents open an inner module:

```rust
// src/foo_tests.rs
mod foo_tests {
    #[test]
    fn test_example() { ... }
}
```

Soroban writes the snapshot to:

```
test_snapshots/foo_tests/foo_tests/test_example.1.json
```

The outer directory name comes from the `mod foo_tests;` declaration in
`lib.rs`; the inner directory name comes from the `mod foo_tests { }` block
inside the file itself.

## The anchor_info_discovery_tests layout

`src/anchor_info_discovery_tests.rs` is declared in `lib.rs` as:

```rust
#[cfg(test)]
mod anchor_info_discovery_tests;
```

Inside the file, all tests live in:

```rust
mod anchor_info_discovery_tests {
    #[test]
    fn test_fetch_and_cache_toml() { ... }
    // ...
}
```

The resulting canonical snapshot directory is therefore:

```
test_snapshots/anchor_info_discovery_tests/anchor_info_discovery_tests/
```

## Issue #528 — what happened and what was removed

An older version of the test file used a different inner module name (`tests`
instead of `anchor_info_discovery_tests`), which caused Soroban to write
snapshots to:

```
test_snapshots/anchor_info_discovery/tests/   ← stale, removed
```

After the module was renamed the old directory was never cleaned up, leaving two
trees side-by-side for the same tests:

| Directory | Files | Status |
|---|---|---|
| `test_snapshots/anchor_info_discovery/tests/` | 16 | **Removed** — stale, predates current test suite |
| `test_snapshots/anchor_info_discovery_tests/anchor_info_discovery_tests/` | 29 | **Kept** — canonical, covers all current tests |

The stale tree had smaller files (incomplete auth data) and was missing 13 tests
added after the rename, confirming it was never regenerated.

## Rules for contributors

1. **Never create snapshot directories by hand.** Let the test runner generate
   them. The path is always derived from the module hierarchy.

2. **If you rename a test module**, delete the old snapshot directory for that
   module. Stale snapshots are not automatically removed and will accumulate.

3. **Inner module names matter.** If your test file wraps tests in
   `mod my_feature_tests { }`, the snapshot path will include that name twice:
   `test_snapshots/my_feature_tests/my_feature_tests/`.

4. **Verify snapshot count matches test count.** If
   `test_snapshots/<module>/<module>/` has fewer `.json` files than `#[test]`
   functions in the corresponding source file, some snapshots may be stale or
   missing.

## Directory structure reference

```
test_snapshots/
├── anchor_info_discovery_tests/
│   └── anchor_info_discovery_tests/   ← tests in src/anchor_info_discovery_tests.rs
│       ├── test_fetch_and_cache_toml.1.json
│       ├── test_xlm_native_asset.1.json
│       └── ...  (29 files total)
├── session_tests/
│   └── session_tests/                 ← tests in src/session_tests.rs
├── capability_detection_tests/
│   └── capability_detection_tests/    ← tests in src/capability_detection_tests.rs
└── ...
```
