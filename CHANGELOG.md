# CHANGELOG.md

All notable changes to this project should be documented in this file.

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

