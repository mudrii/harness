# ONBOARDING.md

## Purpose

Fast-start guide for first-time contributors and AI agents.

## Read order (high signal first)

1. `README.md`
2. `SKILLS.md`
3. `AGENTS.md`
4. `CONTRACTS.md`
5. `TEST_STRATEGY.md`
6. `ARCHITECTURE.md`
7. `docs/CODE_STRUCTURE.md`

## First 5 commands

```bash
cd /Users/mudrii/src/harness
cargo check --all-targets
cargo test
./scripts/simulate_cli_use_cases.sh
cargo build --release
```

## CLI quick map

- `harness init <path>`
- `harness analyze <path> [--format json|md|sarif] [--min-impact safe|all]`
- `harness suggest <path> [--export-diff]`
- `harness apply <path> (--plan-file <file> | --plan-all) [--apply-mode preview|apply]`
- `harness optimize <path> [--trace-dir <path>]`
- `harness bench <path> [--suite <name>] [--runs <n>] [--compare <path>] [--force-compare]`
- `harness lint <path>`

## DoD and acceptance

- DoD: `AGENTS.md`
- Acceptance criteria: `CONTRACTS.md`
- Verification strategy: `TEST_STRATEGY.md`

## Safety notes

- `analyze` and `suggest` are read-only.
- `apply` is safety-critical and must keep precondition checks.
- Exit code contract is fixed: `0|1|2|3`.

## Repo hygiene

- Do not commit `target/` or `dist/`.
- Keep private plan notes local-only (`PLAN.local.md`, `docs/plans/local/`).
