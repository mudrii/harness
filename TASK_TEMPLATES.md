# TASK_TEMPLATES.md

## Purpose

Reusable templates for common agent tasks in this repository.

## 1) Add a CLI flag

### Input template

- Command:
- New flag:
- Expected behavior:
- Exit-code impact:
- Docs to update:

### Required steps

1. Update clap definitions in `src/cli.rs`.
2. Implement behavior in `src/main.rs` (or relevant module).
3. Add unit tests for parsing/logic where relevant.
4. Add CLI ATDD for success and failure cases.
5. Update `CONTRACTS.md` if acceptance behavior changed.

## 2) Add policy/guardrail rule

### Input template

- Rule goal:
- Allowed/forbidden patterns:
- Alias behavior:
- Blocking vs warning:

### Required steps

1. Implement in `src/guardrails/*`.
2. Add unit tests for direct and alias-expanded behavior.
3. Add CLI ATDD for blocking/warning output and exit code.
4. Update `TOOLS.md` and/or `CONTRACTS.md` as needed.

## 3) Add report field

### Input template

- Field name:
- Source module:
- Output formats impacted:
- Backward compatibility note:

### Required steps

1. Update types in `src/types/*`.
2. Update renderers in `src/report/*`.
3. Add unit/integration coverage for new field.
4. Update `COMMAND_EXAMPLES.md` if visible to users.

## 4) Add/modify config key

### Input template

- Config path:
- Default value:
- Validation constraints:
- Affected commands:

### Required steps

1. Update schema in `src/types/config.rs`.
2. Update merge/load behavior in `src/config.rs` if needed.
3. Add positive and negative validation tests.
4. Add CLI ATDD for malformed and valid config paths.
5. Update `ERROR_CATALOG.json` if new error surface is introduced.

## 5) Release prep

### Checklist

1. `cargo check --all-targets`
2. `cargo test`
3. `./scripts/simulate_cli_use_cases.sh`
4. `cargo clippy --all-targets -- -D warnings`
5. `cargo build --release`
6. Update `CHANGELOG.md`
7. Tag and publish release assets/checksums
