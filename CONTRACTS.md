# CONTRACTS.md

## Global acceptance criteria

1. Exit code semantics are stable:
   - `0` success
   - `1` success with warnings
   - `2` blocking policy/conformance violations
   - `3` runtime/fatal errors
2. Behavior is deterministic for identical inputs and config.
3. Read-only commands never mutate repository files.
4. Safety checks run before any write path.

## Command acceptance criteria

## `harness init <path>`

1. Creates baseline scaffold files when missing.
2. `--dry-run` produces no writes.
3. `--no-overwrite` preserves existing files.

## `harness analyze <path>`

1. Read-only execution only.
2. Supports `--format {json,md,sarif}`.
3. Returns diagnostics, scores, and recommendations.
4. Non-git repository is rejected with exit code `3`.

## `harness suggest <path>`

1. Read-only recommendation generation.
2. `--export-diff` writes plan artifact(s) under `.harness/plans/`.

## `harness apply <path>`

1. Exactly one selector is required:
   - `--plan-file <path>` xor `--plan-all`
2. Enforces write safety preconditions:
   - clean working tree unless explicitly allowed
   - valid plan input and path traversal rejection
   - policy checks before write
3. Supports preview/apply mode behavior with stable output and exit codes.

## `harness optimize <path>`

1. Consumes trace evidence and emits optimization guidance.
2. Handles malformed traces without crashing and reports warnings.

## `harness bench <path>`

1. Produces run metrics for configured suites/runs.
2. Compare mode rejects incompatible contexts unless force flag is used.

## `harness lint <path>`

1. Enforces profile and policy conformance.
2. Blocking violations return exit code `2`.
3. Warning-only states return exit code `1`.

## Tool deprecation lifecycle contract

1. `tools.deprecated.observe` emits warning finding `tools.observe` and is non-blocking.
2. `tools.deprecated.deprecated` emits blocking finding `tools.deprecated` in `lint` (exit `2`).
3. `tools.deprecated.disabled` is promoted into `tools.baseline.forbidden` on `apply --apply-mode apply`.
4. `tools.deprecated.disabled` is treated as forbidden in guardrails/apply (runtime rejection with exit `3` when violated).
5. `apply --apply-mode preview` remains no-write and does not persist lifecycle promotion.
6. The same tool name cannot be configured in multiple lifecycle stages.
