# SKILLS.md

## Purpose

Compact playbook for agents working in this repository.
Use this file to keep responses short, decisions consistent, and quality high.

## Canonical references

1. `AGENTS.md` for Definition of Done.
2. `CONTRACTS.md` for acceptance criteria and exit-code semantics.
3. `TEST_STRATEGY.md` for TDD/ATDD expectations and validation sequence.

If any instruction conflicts, follow this precedence:
`CONTRACTS.md` > `AGENTS.md` > `TEST_STRATEGY.md` > this file.

## Fast operating mode

1. Prefer minimal diffs and deterministic behavior.
2. Read only files required for the requested change.
3. Keep command output and error semantics stable.
4. Avoid broad exploratory scans unless required.
5. Do not propose speculative architecture changes unless asked.

## Quality gates by change type

## Docs-only changes

1. Validate internal consistency and path references.
2. No code/test run required unless user asks.

## Logic changes (non-CLI contract)

1. Add/update unit tests first (TDD).
2. Implement minimal code.
3. Run:
   - `cargo check --all-targets`
   - `cargo test`

## CLI contract or behavior changes

1. Update `CONTRACTS.md` if behavior expectations change.
2. Add/update unit tests + CLI ATDD coverage.
3. Run:
   - `cargo check --all-targets`
   - `cargo test`
   - `./scripts/simulate_cli_use_cases.sh`

## Release-impacting changes

1. Ensure all above checks pass.
2. Run strict lint gate:
   - `cargo clippy --all-targets -- -D warnings`

## Safety-critical checklist (mandatory)

Apply this checklist for `apply`, policy, config, and path-handling changes:

1. Negative-path tests included.
2. No silent error fallback.
3. Path traversal protections preserved.
4. Dirty-tree and policy preconditions preserved.
5. Exit codes remain stable (`0/1/2/3` contract).

## Token-efficiency rules for agent responses

1. Lead with outcome first.
2. Use short bullets and concrete commands.
3. Avoid repeating repository context already defined in docs.
4. Reference files directly instead of re-explaining full policy text.

## Repository hygiene rules

1. Never commit build artifacts (`target/`, `dist/`).
2. Keep private planning local-only (`PLAN.local.md`, `docs/plans/local/`).
3. Keep public roadmap concise in `PLAN.md`.

## Commit message style

Use scoped, action-oriented messages:

1. `Add ...`
2. `Fix ...`
3. `Refactor ...`
4. `Document ...`
5. `Enforce ...`

Examples:
- `Add CLI ATDD for apply dirty-worktree rejection`
- `Fix config validation for unknown metrics weights`
- `Document release asset workflow for Linux/macOS`

