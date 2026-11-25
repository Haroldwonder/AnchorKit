# Contract Entry Point Resource Costs

This document records rough resource costs (CPU instructions, memory, and
read/write bytes) for each public contract entry point. It is **generated** by
`scripts/gen-performance.sh` — do not edit it by hand. Re-run the script after
changing contract code and commit the regenerated file.

## How costs are measured

Costs are collected from the Soroban test environment's budget/cost estimate
(`Env::cost_estimate()`), which reports the same CPU instruction and memory
figures the network charges for. For a quick manual check you can also use the
simulation-only invocation:

```sh
stellar contract invoke --sim-only --id <CONTRACT_ID> --fn <entry_point> -- <args>
```

## Regenerating this table

```sh
./scripts/gen-performance.sh
```

The script runs the contract test suite with the `cost-report` feature enabled,
which prints one line per entry point in the form:

```
COST <entry_point> cpu=<n> mem=<n> read_bytes=<n> write_bytes=<n>
```

It then rewrites the table below. Values are indicative and will vary with
input size and ledger state.

## Cost table

| Entry point | CPU instructions | Memory (bytes) | Read bytes | Write bytes |
| ----------- | ---------------- | -------------- | ---------- | ----------- |
| _run `./scripts/gen-performance.sh` to populate_ | — | — | — | — |

> Note: the table is intentionally empty until the generator is run against a
> built contract. The script is the single source of truth for these numbers.
