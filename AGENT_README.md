# AGENT_README.md

## Purpose

Single first-read entrypoint for coding agents working in this repository.

## Mission

Implement requested changes with minimal, deterministic diffs while preserving CLI contracts, safety behavior, and test quality.

## Mandatory context pack (read first)

1. `AGENT_README.md`
2. `CONTRACTS.md`
3. `TEST_STRATEGY.md`
4. `REPO_MAP.json`
5. `ERROR_CATALOG.json`

## Optional / on-demand context

- `README.md`
- `ARCHITECTURE.md`
- `docs/CODE_STRUCTURE.md`
- `AGENTS.md`
- `SKILLS.md`
- `ONBOARDING.md`
- `COMMAND_EXAMPLES.md`
- `TEST_MATRIX.md`
- `src/cli.rs`
- `src/main.rs`

## Non-negotiable constraints

1. Preserve exit-code contract: `0`, `1`, `2`, `3`.
2. Keep `analyze` and `suggest` read-only.
3. Treat `apply` as safety-critical and do not bypass preconditions.
4. Do not commit `target/`, `dist/`, or local-only planning files.
5. Do not change command contracts unless explicitly requested.
6. Keep lifecycle semantics stable:
   - `observe` = warning only
   - `deprecated` = blocking lint finding
   - `disabled` = promoted to baseline forbidden on apply mode, and forbidden on apply/guardrails

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

## Harness truth sources

- `tests/cli_atdd.rs` (CLI behavior truth)
- `scripts/simulate_cli_use_cases.sh` (scenario truth)

## Delivery format

1. Short plan (max 6 bullets)
2. Files changed
3. Contract impact (yes/no; if yes, what changed)
4. Tests added/updated
5. Commands run and key results
6. Risks / follow-up suggestions
