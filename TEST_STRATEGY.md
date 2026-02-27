# TEST_STRATEGY.md

## Purpose

Define how acceptance criteria are verified with TDD and ATDD.

## Target test distribution

1. Unit tests (TDD): 60-70%
2. CLI ATDD tests: 20-30%
3. Integration tests: 5-15%

## Required test policy

1. TDD first for logic-level changes.
2. ATDD required for user-visible CLI behavior changes.
3. Negative-path tests are mandatory for safety-critical paths.

## Mandatory negative-path coverage

1. Malformed config parse failures.
2. Invalid config validation failures.
3. Non-git repo rejection.
4. Apply selector misuse (`--plan-file` xor `--plan-all`).
5. Path traversal rejection for plan files.
6. Dirty worktree rejection when not allowed.
7. Forbidden/disabled policy violations.

## Acceptance-to-test mapping

1. `CONTRACTS.md` global/exit criteria -> unit + CLI ATDD assertions.
2. Read-only command guarantees -> CLI ATDD side-effect checks.
3. Apply safety preconditions -> unit policy tests + CLI ATDD failure-path tests.
4. Compare/optimize guards -> integration + CLI ATDD checks.

## Standard validation sequence

1. `cargo check --all-targets`
2. `cargo test`
3. `./scripts/simulate_cli_use_cases.sh`
