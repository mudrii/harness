# Harness CLI Simulation Guide

This guide defines a repeatable way to simulate real CLI usage, including positive and negative flows.

## 1. Why simulation tests

Unit tests validate internal logic, but CLI simulation validates:
- argument/flag contracts,
- exit codes,
- stdout/stderr expectations,
- filesystem side effects (plan files, scaffold artifacts),
- guardrail failures and error messages.

## 2. Fast local simulation runner

Use:

```bash
cd /Users/mudrii/src/harness
./scripts/simulate_cli_use_cases.sh
```

Optional:

```bash
HARNESS_BIN=/absolute/path/to/harness ./scripts/simulate_cli_use_cases.sh
```

If `HARNESS_BIN` is not set, the script uses `target/debug/harness` when available, otherwise falls back to `cargo run --quiet --`.

## 3. Covered scenarios

Current simulation matrix:

1. `analyze` fails outside a git repository (exit `3`)
2. `analyze --format json` succeeds for a valid harness repo (exit `0`)
3. `analyze` fails on malformed `harness.toml` (exit `3`)
4. `suggest --export-diff` creates `.harness/plans` output (exit `0`)
5. `apply --plan-all --apply-mode preview` works on clean git repo (exit `0`)
6. `apply --plan-all --apply-mode preview` fails on dirty worktree without override (exit `3`)
7. `lint` blocks deprecated tools configuration (exit `2`)

## 4. CI usage recommendation

Add this simulation runner as a separate CI job after build:

```bash
cargo build --all-targets
./scripts/simulate_cli_use_cases.sh
```

Keep existing Rust unit/integration/ATDD jobs in parallel.  
Simulation runner is complementary and catches contract regressions visible to end users.

## 5. Extending scenarios

Add a new scenario when introducing or changing:
- CLI flags/argument constraints,
- exit code semantics,
- config merge behavior,
- apply/guardrail workflow,
- report format output.

For each new scenario validate:
- expected exit code,
- one key stdout or stderr message,
- one side effect assertion if relevant.
