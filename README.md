# Harness

`harness` is a Rust CLI for engineering agent harnesses: it scans a repository, scores it, and recommends or generates changes to improve AI-agent reliability, continuity, and cost efficiency.

## Why this exists

Current AI-agent workflows often fail not due to model weakness, but due to harness design:
- too many overlapping tools,
- unclear agent instructions,
- weak verification gates,
- poor continuity for long-running sessions.

`harness` optimizes the control plane around agents.

## Install

```bash
git clone git@github.com:mudrii/harness.git
cd harness
cargo build --release
```

## Quick start

```bash
# analyze an existing repository
harness analyze /path/to/repo

# show ranked recommendations
harness suggest /path/to/repo

# initialize harness scaffold
harness init /path/to/repo --profile agent --dry-run

# apply generated changes
harness apply /path/to/repo --plan-file .harness/plan.json
```

## Command overview

- `harness init` — bootstrap scaffold and baseline prompts.
- `harness analyze` — read-only health report (JSON/Markdown/SARIF).
- `harness suggest` — rank safe-to-apply changes.
- `harness apply` — preview/apply scaffold and patches.
- `harness optimize` — use traces to generate next-gen recommendations.
- `harness bench` — benchmark before/after revisions.
- `harness lint` — validate profile conformance.

## Suggested repository layout

```text
path/to/repo/
  AGENTS.md
  docs/context/
  .harness/
    initializer.prompt.md
    coding.prompt.md
    progress.md
    feature_list.json
  harness.toml
```

## Example output

`harness analyze` returns:
- weighted overall score,
- category scores,
- actionable recommendations,
- confidence and risk labels,
- optional diff preview.

## Contributing

Contributions should align with the architecture in `ARCHITECTURE.md`. Use small, deterministic changes and avoid adding speculative behavior without corresponding trace evidence.
