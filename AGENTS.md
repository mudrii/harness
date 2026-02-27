# AGENTS.md

## Purpose

This file defines the agent execution contract for this repository.

## Canonical behavior source

- Command acceptance criteria are defined in `CONTRACTS.md`.
- Verification requirements are defined in `TEST_STRATEGY.md`.

## Agent operating rules

1. Keep command behavior deterministic for identical input/config.
2. Treat `apply` as safety-critical and enforce all preconditions.
3. Keep `analyze` and `suggest` read-only.
4. Return actionable errors with stable exit codes.
5. Add tests for behavior changes before completion.

## Definition of Done (DoD)

A task is only done when all are true:

1. Acceptance criteria in `CONTRACTS.md` are satisfied for changed behavior.
2. Required tests were added/updated per `TEST_STRATEGY.md`.
3. Existing and new tests pass.
4. Error-path behavior is covered for safety-critical changes.
5. Documentation is updated when command contract changes.
6. No build artifacts or local-only planning files are committed.
