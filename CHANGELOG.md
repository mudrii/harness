# CHANGELOG.md

All notable changes to this project should be documented in this file.

## [Unreleased]

## [0.2.1] - 2026-02-28

### Changed

- Dedicated dead-code cleanup so strict linting passes:
  - `cargo clippy --all-targets -- -D warnings`

## [0.2.0] - 2026-02-28

### Added

- Lifecycle config validation for `tools.deprecated`:
  - rejects empty tool names
  - rejects duplicate tools across lifecycle stages
- CLI ATDD coverage for `observe` warning behavior.
- CLI ATDD and unit coverage proving `deprecated` tools remain apply-allowed.
- Phase 3 lifecycle tests for disabled-tool promotion behavior in apply mode and preview no-write semantics.

### Changed

- `init` scaffold now includes explicit `[tools.deprecated]` lifecycle fields.
- Documentation updated to align contracts/examples/strategy with implemented lifecycle behavior.
- `apply` now promotes `tools.deprecated.disabled` entries into `tools.baseline.forbidden` in `harness.toml` when running in apply mode.

## [0.1.0] - 2026-02-27

### Added

- Core CLI commands: `init`, `analyze`, `suggest`, `apply`, `optimize`, `bench`, `lint`.
- Deterministic recommendation and optimization behavior improvements.
- Tool lifecycle policy handling (`observe` -> `deprecated` -> `disabled`).
- Continuity logging and trace-informed optimization reporting.
- Installation script: `scripts/install.sh`.
- CLI simulation runner: `scripts/simulate_cli_use_cases.sh`.
- Documentation:
  - `ARCHITECTURE.md`
  - `docs/INSTALLATION.md`
  - `docs/CODE_STRUCTURE.md`
  - `docs/CLI_SIMULATION.md`
  - `AGENTS.md`
  - `CONTRACTS.md`
  - `TEST_STRATEGY.md`
  - `SKILLS.md`

### Changed

- Public roadmap simplified in `PLAN.md`.
- Detailed planning notes moved to local-only pattern.
- Binary release workflow added for Linux/macOS.

### Fixed

- Repository hygiene: stopped tracking build artifacts under `target/`.
