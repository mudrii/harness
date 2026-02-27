# Harness Public Roadmap

This is the open-source roadmap for `harness`.

## Mission

Build a reliable Rust CLI that helps teams harden AI-agent workflows by improving:
- tool safety and minimalism,
- continuity for long-running tasks,
- verification before completion,
- deterministic and auditable execution.

## Current state

Implemented command surface:
- `init`
- `analyze`
- `suggest`
- `apply`
- `optimize`
- `bench`
- `lint`

Cross-platform binary release workflow is enabled for Linux and macOS.

## Near-term priorities

1. Stabilize public report/schema contracts.
2. Reduce warning-level technical debt and tighten lint standards.
3. Improve benchmark compare confidence and reporting clarity.
4. Expand simulation and ATDD coverage for edge-case guardrails.
5. Improve onboarding docs for external contributors.

## Next milestones

### M1: Core reliability hardening
- finalize error-path consistency and exit code semantics,
- eliminate known false-positive/false-negative policy checks,
- improve rollback manifest validation and recovery docs.

### M2: Trace-driven optimization maturity
- strengthen trace parsing tolerance,
- improve recommendation prioritization from evidence quality,
- add clearer optimize report diagnostics for weak statistical signal.

### M3: Contribution and ecosystem readiness
- contributor guide + issue templates,
- stable release notes/checklist,
- broader CI coverage across release and simulation flows.

## Out of scope (for now)

- hosted orchestration SaaS,
- GUI frontend,
- provider-specific LLM routing layer.

## Internal planning

Detailed planning and private working notes are intentionally kept local-only and are not part of the public repository contract.
