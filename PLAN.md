# Harness CLI Detailed Plan (v1.1)

## 0) Scope and assumptions

This document defines a practical plan for a Rust-based CLI named `harness` that analyzes a folder/repository and generates a concrete harness configuration to improve AI-agent productivity, reliability, cost efficiency, and long-run continuity.

Assumptions:
- The user runs AI agents in local or remote repo environments.
- The CLI does not replace the model provider or orchestration platform.
- Repository structure and files are the primary source of truth for optimization.
- Tool use should be safe by default, with explicit opt-in to high-risk operations.
- Git is required in v1. Non-git VCS support is deferred.
- Configuration uses TOML only in v1. YAML support is dropped.

## 1) Reference-source synthesis and interpretation

### 1.1 Vercel (tool minimization)

Key observation: broad tool reductions can increase reliability and reduce cognitive/operational overhead.

Implication in `harness`:
- Start from a minimal baseline toolset.
- Add only tool capabilities with measurable gain.
- Detect redundant tools and conflict overlap.
- Recommend removing or deprecating underused/high-risk tools before adding new ones.

### 1.2 OpenAI (agent legibility)

Key observation: agents act best on versioned, well-structured repository artifacts.

Implication in `harness`:
- The scanner must score and enforce `AGENTS.md` + docs map quality.
- Keep top-level instructions short; push deep guidance into versioned files.
- Provide repo-specific harness files that are explicitly linked from `AGENTS.md`.

### 1.3 LangChain (harness dominates outcomes)

Key observation: with strong middleware, prompt constraints, and workflow policies, large gains are possible without changing model.

Implication in `harness`:
- Evaluate harness parameters as first-class tuning knobs.
- Build feedback loop from traces to plan recommendations.
- Add loop prevention and verification gates by default where risk is high.

### 1.4 Anthropic (long-running agent continuity)

Key observation: long-running agents fail due to session discontinuity unless explicit state transfer exists.

Implication in `harness`:
- Introduce initializer/coding separation.
- Persist feature state, progress snapshots, and end-state summaries in repository files.
- Enforce resumability checks when resuming partial work.

## 2) Detailed problem statement

### 2.1 Failure modes this CLI should reduce

1. **Tool bloat and confusion**
- Too many overlapping tools increase ambiguity and error modes.

2. **Context drift and context-loss**
- Instructions live outside repository or are scattered/obsolete.

3. **Missing verification**
- Agents mark work as complete despite failing build/test expectations.

4. **Session amnesia**
- Multi-day or long tasks lose state across session boundaries.

5. **No measurable harness evolution**
- Recommendations are static and not tied to run outcomes.

### 2.2 Desired outcomes

- Higher autonomous completion rate.
- Lower tool calls, especially unnecessary/redundant calls.
- Better reproducibility of successful agent runs.
- Faster re-entry after interruptions.
- Safer changes via predictable policy profiles.

## 3) Product scope (v1)

### 3.1 Included

- Folder/repo scanning and classification.
- Quantified scores for harness health and risk.
- Recommendation generation with confidence and risk tags.
- Scaffold and write mode for harness files.
- Trace ingestion and optimization suggestions from historical runs.

### 3.2 Excluded (v1)

- Native LLM API routing/orchestration layer.
- Custom code execution engine.
- GUI interface.
- Any cloud-hosted SaaS backend.

## 4) Command surface and contracts

### 4.1 `harness init <path>`

Goal: bootstrap harness baseline if absent.

Behavior:
- Detect project language (heuristic from files).
- Emit `.harness/` scaffold.
- Create `harness.toml` with conservative defaults.
- Add baseline prompts.
- Add docs index and progress template only if missing.

Required flags:
- `--profile <id>` (default: `general`)
- `--dry-run`
- `--no-overwrite`

### 4.2 `harness analyze <path>`

Goal: read-only scoring and diagnostics.

Output:
- JSON/Markdown report.
- Category scores + issue list.
- Suggested actions with severity ranking.

Required flags:
- `--format {json,md,sarif}`
- `--min-impact {safe|all}` (validated by `MinImpact` enum via `clap::ValueEnum`)

### 4.3 `harness suggest <path>`

Goal: rank recommended changes without mutating files.

Output:
- Ordered recommendation plan.
- Optional `--export-diff` to show exact patch snippets.

### 4.4 `harness apply <path>`

Goal: apply selected plan safely.

Behavior:
- Supports interactive and machine mode.
- Writes changed files only after explicit confirmation.
- Idempotent application and conflict awareness.

Preconditions (checked in order before any writes):
1. Clean working tree required (abort if dirty, unless `--allow-dirty`).
2. Plan file must exist, parse, have no path traversal, and match harness version.
3. Rollback manifest written to `.harness/rollback/<timestamp>.json` (file list + SHA256 hashes).
4. Change scope summary printed (files modified/created/deleted with names).
5. Explicit `y/N` confirmation in apply mode (skipped in preview mode or with `--yes`).

Required flags:
- `--plan-file path` or `--plan-all`.
- `--apply-mode {preview,apply}` (validated by `ApplyMode` enum via `clap::ValueEnum`)
- `--allow-dirty` (skip clean working tree check)
- `--yes` / `-y` (skip confirmation prompt)

### 4.5 `harness optimize <path>`

Goal: use traces to produce harness evolution recommendations.

Behavior:
- Parse `trace-dir`, evaluate deltas vs baseline.
- Identify top offenders by repeated failure pattern.
- Propose concrete harness edits.

### 4.6 `harness bench <path>`

Goal: benchmark and compare before/after harness revisions.

Behavior:
- Execute benchmark suite commands.
- Persist run metrics (`success_rate`, `median_steps`, `tool_calls`, `token_est`, `wall_ms`).
- Persist `bench_context` per run: OS, toolchain, repo ref, dirty state, harness version, config hash, env vars, command, seed, timestamp.
- Compare variant runs and identify regressions.
- Comparison guard: refuse to compare runs with different OS, toolchain, or dirty state unless `--force-compare`.

### 4.7 `harness lint <path>`

Goal: policy conformance check.

Checks:
- Required files exist.
- Tool policy matches profile.
- No forbidden operations allowed by policy.
- Report blocking/non-blocking issues.

## 5) Data model and manifest schema

### 5.1 Configuration hierarchy

1. Global config: `$HOME/.config/harness/config.toml`
2. Repo config: `harness.toml`
3. Repo local override: `.harness/local.toml`

Note: YAML config (`.harness.yaml`) is not supported in v1. TOML is the only format.

Merge order: global < repo < local override.

### 5.2 Core manifest (annotated)

```toml
[project]
name = "repo-name"
profile = "agent" # agent|general (v1); ops|strict deferred to v1.1
language = "rust"
main_branch = "main"

[context]
agents_map = "AGENTS.md"
context_index = "docs/context/INDEX.md"
doc_map_required = true

[tools.baseline]
read = ["bash", "ls", "find", "cat", "rg", "git"]
write = ["apply_patch", "tee"] # exclusively executable names, no shell redirects or args
forbidden = ["sudo", "ssh", "nc", "mkfs", "fdisk"]

[tools.specialized]
extra = [] # strictly single executable names (e.g. "deploy")

[tools.deprecated]
observe = []      # Phase 1: allowed, non-blocking warning in report
deprecated = []   # Phase 2: allowed, blocking finding in report, lint violation
disabled = []     # Phase 3: moved to forbidden on next apply

[tools.aliases]
# Known aliases that inherit parent tool classification
# grep = "rg"
# find = "fd"

[verification]
required = ["cargo check", "cargo test", "cargo fmt --check"]
pre_completion_required = true
loop_guard_enabled = true

[continuity]
initializer = ".harness/initializer.prompt.md"
coding_prompt = ".harness/coding.prompt.md"
progress_file = ".harness/progress.md"
feature_state_file = ".harness/feature_list.json"
state_schema_version = 1
log_sampling = "milestones"      # milestones|all|none
batch_interval_secs = 60
max_log_size_kb = 100
retained_logs = 3

[optimization]
min_traces = 30                  # minimum traces per revision before recommendations
min_uplift_abs = 0.05            # completion rate delta threshold (5pp)
min_uplift_rel = 0.10            # token/step delta threshold (10%)
trace_staleness_days = 90        # exclude traces older than this
task_overlap_threshold = 0.50    # minimum task overlap for paired comparison

[metrics]
weights = { context = 0.30, tools = 0.25, continuity = 0.20, verification = 0.15, repository_quality = 0.10 }
max_risk_tolerance = 0.35
max_penalty_per_bucket = 0.40  # no single penalty class can reduce a sub-score by more than this

[workflow]
max_consecutive_failures = 2
max_idle_steps = 8
replan_on_loop = true
```

### 5.3 Feature progress schema (optional but recommended)

```json
{
  "version": 1,
  "feature": "feature-id",
  "items": [
    {
      "id": "F-001",
      "goal": "Refactor parser",
      "status": "in_progress|done|blocked",
      "owner": "agent|human",
      "evidence": ["path/to/file.rs"]
    }
  ]
}
```

State transitions (v1, schema version 1):
- in_progress → done (evidence provided)
- in_progress → blocked (blocker identified)
- blocked → in_progress (blocker resolved)

Note: A 5-state model (idle → planning → coding → verifying → done/blocked)
is designed for v1.1 (schema version 2) when trace-driven state detection is available.

## 6) Scanner design (what gets analyzed)

### 6.1 Context signals

- `AGENTS.md` size and structure.
- Presence of context index in `docs/context/`.
- Cross-reference quality (named links, ownership, run instructions).
- Churn and age metadata (stale docs warning).

### 6.2 Tool signals

- Count of agent tools (if existing harness config exists).
- Overlap detection:
  - grep-like commands clustering (`grep`, `rg`, `ag`, `find` variants).
  - read/write command overlaps.
- Risk detection: write/delete/network commands exposed.

### 6.3 Verification signals

- Is verification defined?
- Is verification required before session end?
- Are required checks realistic and deterministic (e.g., no random, no interactive prompts)?

### 6.4 Continuity signals

- initializer and coding prompt artifacts present.
- progress file includes timestamps and checkpoint markers.
- feature list includes state transitions and evidence.

### 6.5 Repo hygiene signals

- Clear module boundaries?
- CI config present.
- Test location conventions.
- Lint/format enforcement files.

## 7) Scoring and scoring math

### 7.1 Formula

`overall_score = Σ(weight_i * clamp(score_i, 0.0, 1.0))`

- context: 0.30
- tools: 0.25
- continuity: 0.20
- verification: 0.15
- repository_quality: 0.10

`score_i ∈ [0.0, 1.0]` (clamped after all bonuses/penalties)

Normalization rules:
- Each sub-score is clamped to [0.0, 1.0] after all additive rules.
- No single penalty bucket can reduce a sub-score by more than `max_penalty_per_bucket` (default 0.40).
- Overall score is guaranteed in [0.0, 1.0] since weights sum to 1.0 and each clamped score is in [0,1].
- Recommendation tie-breaking: impact (high > medium > low), then effort (xs < s < m < l), then alphabetical ID.

### 7.2 Rule-based subscore examples

#### context_score
- `+0.35` if `AGENTS.md` exists and includes at least one section header.
- `+0.20` if docs map exists.
- `+0.15` if architecture/test guide exists.
- `+0.10` if README links architecture docs.
- `+0.20` if docs age < 90 days (based on git metadata).

#### tools_score
- Start at 1.0.
- `-0.10` if baseline tool count > 12.
- `-0.05` per risky overlap cluster.
- `-0.20` for each unrestricted destructive command.
- `-0.15` if tool names contain ambiguous duplicates.

#### continuity_score
- `+0.40` if initializer + coding prompt present.
- `+0.25` if progress log exists and updated in last N commits.
- `+0.20` if feature list schema present for multi-feature folders.
- `+0.15` if each session has completion summary convention.

#### verification_score
- `+0.50` if required checks exist.
- `+0.30` if pre-completion gate exists.
- `+0.20` if loop guard exists.

Repository quality: small bounded score from test placement, CI consistency, and build reproducibility.

### 7.3 Confidence scoring

Each recommendation includes:
- `confidence` in [0.0, 1.0]
- `impact` in `{low, medium, high}`
- `effort` in `{xs, s, m, l}`

## 8) Recommendation taxonomy

### 8.1 Safe recommendations (auto-apply)
- Add missing context index.
- Add `harness.toml` base scaffold from profile.
- Add `.harness/progress.md` skeleton.

### 8.2 Medium risk
- Reduce tool overlap clusters.
- Make verification required for completion.
- Introduce loop guard limits.

### 8.3 High risk (require explicit approval)
- Remove existing specialized tools.
- Restrict network/system-level commands.
- Rework profile-specific workflow order.

## 9) Template strategy

### 9.1 Prompt templates

- Top-level map (short): `AGENTS.md`
- Initializer role prompt:
  - load context index,
  - set success constraints,
  - define state artifact names.
- Coding role prompt:
  - pick one feature/task,
  - include verify-first discipline,
  - report evidence before completion.

### 9.2 Middleware hooks

- pre-command hook: check forbidden command and context.
- post-command hook: log command class.
- pre-completion hook: run required verification set.
- fail fast rule: if repeated no-progress edits detected.

### 9.3 Continuity defaults

Events are classified and logged selectively:
- **Milestone events** (always logged): task start, task complete, verification pass/fail, error.
- **Progress events** (sampled per `log_sampling` config): individual command runs, file reads/writes.

Progress events accumulate in memory and flush on milestone events or every `batch_interval_secs` (default 60s).

Each logged entry in `.harness/progress.md` includes:
  - timestamp,
  - feature,
  - action,
  - evidence,
  - next state.

Log rotation: when `progress.md` exceeds `max_log_size_kb` (default 100KB), rotate to `progress-<date>.md`. Keep last `retained_logs` (default 3) rotated files.

## 10) Trace-driven improvement loop

1. Collect traces (command calls, failures, success flags, duration).
2. Aggregate by harness revision.
3. Compute deltas:
   - success delta,
   - token estimate delta,
   - tool call churn.
4. Detect repeated failure signature clusters.
5. Propose rule adjustments with priority ordering.
6. Run `bench` again to validate improvements.

Statistical gates (all configurable via `[optimization]` manifest section):
- Minimum `min_traces` (default 30) per revision before generating recommendations. Below threshold: "insufficient data".
- Completion rate recommendations require `|Δ| ≥ min_uplift_abs` (default 0.05, i.e., 5pp).
- Token/step recommendations require `|Δ| ≥ min_uplift_rel` (default 0.10, i.e., 10%).
- Paired comparisons require `task_overlap_threshold` (default 0.50) overlap on the same task set.
- Traces older than `trace_staleness_days` (default 90) are excluded.

Direction semantics:
- Completion rate: Δ > 0 is improvement (higher is better).
- Token/step count: Δ < 0 is improvement (lower is better).
- The |Δ| gate triggers on magnitude in either direction.
- Positive changes produce "improvement" recommendations.
- Negative changes produce "regression" warnings.

Trace record schema (v1.1):
- task_id: string (format: "<feature>/<subtask>", e.g., "auth/login-form")
- revision: string (harness config hash at time of trace)
- outcome: "success" | "failure" | "timeout"
- steps: u32
- tool_calls: u32
- token_est: u64
- wall_ms: u64
- timestamp: ISO 8601

Task overlap is computed as |intersection(tasks_A, tasks_B)| / |union(tasks_A, tasks_B)|.

Acceptance criterion for every change:
- no negative regression in completion rate for top 10 representative tasks.

## 11) Detailed implementation plan (engineering)

### 11.1 Rust stack and dependencies

- CLI: `clap`
- Serialization: `serde`, `serde_json`, `toml`
- Filesystem walk: `walkdir`
- Markdown handling: simple parser (v1); `pulldown-cmark` deferred to v1.1
- Template rendering: string interpolation (v1); `handlebars` deferred to v1.1
- Tracing/logging: `tracing`, `tracing-subscriber`
- Error handling: `thiserror`

### 11.2 Internal API contracts (proposed)

- `scan::discover(path: &Path) -> RepoModel`
- `analyze::score(model: &RepoModel, cfg: &Config) -> HarnessReport`
- `optimization::plan(report: &HarnessReport, policy: &Policy) -> Vec<Change>`
- `generator::render(plan: &[Change], mode: RenderMode) -> Vec<FilePatch>`
- `writer::apply(patches: &[FilePatch], options: ApplyOptions) -> ApplyResult`
- `trace::load_runs(path: &Path) -> TraceSummary`

### 11.3 Module responsibilities and boundaries

- `scan`: pure discovery, no policy mutation.
- `analyze`: deterministic scoring.
- `optimization`: deterministic ranking, no side effects.
- `generator`: deterministic templates and deterministic IDs.
- `writer`: side-effect layer with backups/rollbacks.
- `report`: output formatting and exit code mapping.

### 11.4 Failure handling

- Missing file artifacts -> warning + recovery suggestions.
- Parse failures -> degrade to partial results with explicit diagnostics.
- Unsafe apply -> abort and print remediation guide.
- In all cases, command must never destroy uncommitted files without explicit `--force`.

### 11.5 CLI exit-code rules

- `0`: success and no blocking issues.
- `1`: completed with warnings.
- `2`: blocking issues detected.
- `3`: runtime failure or parse failure.

## 12) Detailed repo structure recommendation

```
src/
  main.rs
  cli.rs
  config.rs
  types/
    mod.rs
    config.rs
    scoring.rs
    report.rs
  scan/
    mod.rs
    filesystem.rs
    docs.rs
    tools.rs
    git_meta.rs
  analyze/
    mod.rs
    context.rs
    tools.rs
    continuity.rs
    verification.rs
    quality.rs
  optimization/
    mod.rs
    scoring.rs
    recommender.rs
    rules.rs
    presets.rs
  generator/
    mod.rs
    templates.rs
    manifest.rs
    writer.rs
  trace/
    mod.rs
    parser.rs
    aggregation.rs
  report/
    mod.rs
    json.rs
    md.rs
    sarif.rs
  guardrails/
    mod.rs
    loop_guard.rs
    command_policy.rs
```

## 13) Security and governance

- Default denylist for destructive commands.
- No implicit network commands in baseline.
- Clear audit trail: every generated file starts with metadata header.
- Optional signature policy for generated files (hash in manifest).
- Optional org-level profile restrictions (no overrides for critical commands).
- Tool deprecation lifecycle: 3-phase (observe -> deprecated -> disabled) with rollback triggers when trace data shows >10% completion rate drop.
- Command policy uses alias expansion and argument-aware matching to prevent bypass via wrappers. Namespace-based classification (fs/net/vcs/exec) designed for v1.1.

## 14) Business plan and adoption model

### 14.1 Why this is a useful product

- Teams can improve agent quality without changing model stacks.
- Measurable savings in retries, tokens, and operator overhead.
- Lower risk through deterministic harness artifacts and continuity discipline.

### 14.2 Delivery model

- Open-source core CLI.
- Paid add-ons:
  - policy packs,
  - enterprise reporting,
  - trace retention/analysis extension,
  - SSO and org governance integrations.

### 14.3 Go-to-market target customers

- Platform teams,
- internal AI tool teams,
- teams with heavy AI coding operations,
- data/ops teams with long-running automated agents.

## 15) Risks and mitigation matrix

- Over-pruning risk → staged rollout + rollback plan.
- False positives in recommendation engine → confidence score + explainability.
- Policy mismatch across repos → profile inheritance and per-repo override.
- Trace drift over time → periodic recalibration jobs.
- Human fatigue with reports → compact mode + actionable summaries.

## 16) KPI and acceptance metrics

- Completion success rate lift: baseline vs after apply.
- Verification pass consistency.
- Tokens/step and average step count.
- Continuity recovery time between interrupted sessions.
- Percentage of suggestions with high confidence executed without overrides.

## 17) Milestones (practical)

- Week 1-2: **[M1]** scanner + `analyze` command. **[M2 start]** recommendation engine + `suggest` dry-run.
- Week 3-4: **[M2 complete]** `suggest` finalized. **[M3]** `init` and safe `apply` with patch preview.
- Week 5-6: **[M4]** trace ingestion, `optimize`, and `bench` loop.
- Week 7: policy packs and team profiles. Stability hardening begins.
- Week 8: v1 release candidate, docs, and integration testing.

See ARCHITECTURE.md Section 10 for milestone definitions (M1-M4).

## 18) Immediate next technical tasks

1. Bootstrap Rust workspace and CLI parser.
2. Implement scanner module for context/tool/verification/continuity signals.
3. Implement weighted scoring and recommendation engine.
4. Add report renderers.
5. Add apply mode with dry-run.
6. Add initial docs: quick-start and profile examples.
