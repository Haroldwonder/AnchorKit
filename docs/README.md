# Documentation

This directory contains project documentation.

## Contents

- [`performance.md`](./performance.md) — Gas/resource cost table for contract entry points (CPU instructions, memory, and read/write bytes per public function).

## Regenerating the performance table

`docs/performance.md` is generated — do not edit it by hand. Regenerate it with the
measurement script, which collects resource costs for each public contract entry
point using the test environment's budget/cost estimate (or
`stellar contract invoke --sim-only`):

```sh
./scripts/gen-performance.sh
```

The script writes the resulting table to `docs/performance.md`. Re-run it whenever
contract entry points or their resource usage change, and commit the regenerated
file alongside the change.
