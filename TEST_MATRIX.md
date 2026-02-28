# TEST_MATRIX.md

## Purpose

Map command acceptance criteria to verification layers.

## Matrix

| Area | Acceptance focus | Unit tests | CLI ATDD | Integration | Simulation |
|---|---|---|---|---|---|
| `init` | scaffold creation, dry-run/no-overwrite behavior | Yes | Yes | Optional | Optional |
| `analyze` | read-only scoring/reporting, non-git rejection | Yes | Yes | Yes | Yes |
| `suggest` | recommendation output, plan export | Yes | Yes | Optional | Yes |
| `apply` | selector contract, safety preconditions, policy checks | Yes | Yes | Optional | Yes |
| `optimize` | trace parsing tolerance, recommendation deltas | Yes | Yes | Optional | Optional |
| `bench` | run metrics and compare guards | Yes | Yes | Optional | Optional |
| `lint` | conformance severities and exit-code mapping | Yes | Yes | Optional | Yes |

## Required negative-path checks

1. Invalid plan selector combinations.
2. Path traversal rejection.
3. Dirty working tree rejection without override.
4. Non-git repository rejection.
5. Malformed config parse failure.
6. Invalid config validation failure.
7. Disabled/forbidden policy rejection.
8. Lifecycle stage semantics:
   - `observe` warning-only
   - `deprecated` lint-blocking but apply-allowed
   - `disabled` promoted to baseline forbidden on apply mode; apply-blocking when violated

## Default verification sequence

```bash
cargo check --all-targets
cargo test
./scripts/simulate_cli_use_cases.sh
```
