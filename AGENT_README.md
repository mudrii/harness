# AGENT_README.md

## Purpose

Single first-read entrypoint for coding agents working in this repository.

## Mission

Implement requested changes with minimal, deterministic diffs while preserving CLI contracts, safety behavior, and test quality.

## Mandatory read order

1. `ONBOARDING.md`
2. `CLAUDE.md`
3. `SKILLS.md`
4. `AGENTS.md`
5. `CONTRACTS.md`
6. `TEST_STRATEGY.md`
7. `COMMAND_EXAMPLES.md`
8. `TEST_MATRIX.md`
9. `ERROR_CATALOG.json`
10. `ARCHITECTURE.md`
11. `docs/CODE_STRUCTURE.md`
12. `src/cli.rs`
13. `src/main.rs`
14. `tests/cli_atdd.rs`
15. `scripts/simulate_cli_use_cases.sh`

## Non-negotiable constraints

1. Preserve exit-code contract: `0`, `1`, `2`, `3`.
2. Keep `analyze` and `suggest` read-only.
3. Treat `apply` as safety-critical and do not bypass preconditions.
4. Do not commit `target/`, `dist/`, or local-only planning files.
5. Do not change command contracts unless explicitly requested.

## Implementation policy

1. TDD first for logic changes.
2. Add/update CLI ATDD for user-visible behavior changes.
3. Include positive and negative tests for safety-critical paths.
4. If behavior contract changes, update:
   - `CONTRACTS.md`
   - `COMMAND_EXAMPLES.md`
   - `TEST_MATRIX.md` (if needed)
   - `ERROR_CATALOG.json` (if error surface changes)

## Default validation sequence

1. `cargo check --all-targets`
2. `cargo test`
3. `./scripts/simulate_cli_use_cases.sh`

## Delivery format

1. Short plan (max 6 bullets)
2. Files changed
3. Contract impact (yes/no; if yes, what changed)
4. Tests added/updated
5. Commands run and key results
6. Risks / follow-up suggestions
