# Gap Resolution Design

Date: 2026-02-27
Status: Approved

## Context

Review of PLAN.md, ARCHITECTURE.md, and source code identified 8 specification gaps and 2 unresolved assumptions. This document captures the approved design for each.

## Resolved Assumptions

### A1: Git Requirement
**Decision:** Git is a hard requirement in v1. The scanner depends on git metadata for doc-age signals, commit history, and working-tree checks. Non-git VCS support is out of scope for v1.

**Implementation:** `harness analyze` and `harness init` check for `.git` presence early and abort with a clear error if missing.

### A2: Config Format
**Decision:** TOML only in v1. The config file is `harness.toml` (repo-level) and `$HOME/.config/harness/config.toml` (global). `.harness.yaml` and `serde_yaml` dependency are dropped from v1 scope.

**Implementation:** Remove `serde_yaml` from Cargo.toml. Update PLAN.md Section 5.1 to remove YAML references.

---

## P1 Gap Fixes

### P1-1: Profile Validation

**Problem:** `cli.rs` accepts arbitrary `--profile` strings. Invalid profiles silently pass through.

**Design:** Replace `String` with a `clap::ValueEnum` enum.

```rust
#[derive(Clone, ValueEnum)]
pub enum Profile {
    General,
    Agent,
}
```

`InitCommand.profile` becomes `Profile` type. Clap rejects unknown values at parse time with a helpful error message. `presets.rs` maps `Profile` variants to profile-specific config structs containing tool baselines, verification requirements, and risk tolerances.

**Files affected:** `src/cli.rs`, `src/optimization/presets.rs`, `src/main.rs`

### P1-2: Tool Deprecation Lifecycle

**Problem:** No staged process for removing tools. Immediate removals can break active workflows.

**Design:** 3-phase lifecycle managed through config:

```toml
[tools.deprecated]
observe = ["find", "ag"]       # Phase 1: allowed, non-blocking warning
deprecated = ["grep"]          # Phase 2: allowed, blocking finding in report
disabled = []                  # Phase 3: moved to forbidden on next apply
```

Phase behavior:
- **observe**: tool works, `analyze` emits non-blocking warning, usage tracked in traces
- **deprecated**: tool works, `analyze` emits blocking finding, `lint` reports violation
- **disabled**: tool treated as forbidden on next `apply`, `lint` fails hard

Rollback trigger: if trace data (when available) shows >10% completion rate drop after a tool deprecation, the recommendation engine suggests reverting.

**Files affected:** PLAN.md Section 5.2 (manifest schema), `src/guardrails/command_policy.rs` (implementation)

### P1-3: Trace Optimization Statistical Rigor

**Problem:** Trace-driven recommendations lack statistical validity guardrails.

**Design:** Statistical gates added to the optimization contract:

```toml
[optimization]
min_traces = 30                  # minimum traces per revision before recommendations
min_uplift_abs = 0.05            # completion rate delta threshold (5pp)
min_uplift_rel = 0.10            # token/step delta threshold (10%)
trace_staleness_days = 90        # exclude traces older than this
task_overlap_threshold = 0.50    # minimum task overlap for paired comparison
```

Behavior:
- Below `min_traces`: `optimize` returns "insufficient data" instead of noisy recommendations
- Below `min_uplift_*`: changes are not recommended
- Below `task_overlap_threshold`: comparisons are rejected as incomparable
- Beyond `trace_staleness_days`: traces are excluded from the current optimization cycle

Implementation is deferred to v1.1 (trace module scope). This design documents the contract.

Direction semantics (added 2026-02-27):
- Completion rate: Δ > 0 is improvement (higher is better).
- Token/step count: Δ < 0 is improvement (lower is better).
- The |Δ| gate triggers on magnitude in either direction.
- Positive changes produce "improvement" recommendations.
- Negative changes produce "regression" warnings.

**Files affected:** PLAN.md Sections 5.2, 10

---

## P2 Gap Fixes

### P2-4: Score Normalization and Clamping

**Problem:** Additive scoring rules can produce sub-scores outside [0,1] or negative values. No tie-breaking for equal-confidence recommendations.

**Design:**

1. Clamp each sub-score to `[0.0, 1.0]` after all bonuses/penalties
2. Cap any single penalty bucket at `max_penalty_per_bucket` (default 0.40, configurable)
3. Overall score: `sum(weight_i * clamp(score_i))` is guaranteed in [0,1]
4. Recommendation tie-breaking: impact (high > medium > low), then effort (xs < s < m < l), then alphabetical ID

```rust
impl ScoreCard {
    pub fn clamped(&self) -> Self {
        ScoreCard {
            context: self.context.clamp(0.0, 1.0),
            tools: self.tools.clamp(0.0, 1.0),
            continuity: self.continuity.clamp(0.0, 1.0),
            verification: self.verification.clamp(0.0, 1.0),
            repository_quality: self.repository_quality.clamp(0.0, 1.0),
            overall: self.weighted_overall(),
        }
    }
}
```

Config:
```toml
[metrics]
max_penalty_per_bucket = 0.40
```

**Files affected:** `src/types/scoring.rs`, PLAN.md Section 7

### P2-5: Tool Classification Model

**Problem:** Flat command lists are too coarse. Aliases and wrappers can bypass policy.

**Design for v1:** Keep flat lists but add alias expansion and argument-aware matching:

```toml
[tools.baseline]
read = ["cat", "rg", "git"]
write = ["apply_patch"]
forbidden = ["sudo", "ssh", "nc", "mkfs"]

[tools.aliases]
grep = "rg"
find = "fd"
```

The command policy checker:
1. Expands aliases before matching
2. Matches command + arguments (not just command name)
3. Checks both exact and prefix matches for compound commands (e.g., `git push` matches forbidden `git push --force`)

Constraint alignment:
- Config lists use executable names only.
- Argument restrictions are policy rules, not manifest command strings.

Namespace-based classification (fs.read/fs.write/net/exec) is designed for v1.1.

**Files affected:** PLAN.md Section 5.2, `src/guardrails/command_policy.rs`

### P2-6: Apply Safety Preconditions

**Problem:** `harness apply` lacks explicit precondition checks before writing files.

**Design:** 5 preconditions executed in order before any file writes:

1. **Clean working tree** (unless `--allow-dirty`): run `git status --porcelain`, abort if non-empty
2. **Plan file validity**: file exists, parses, no path traversal, version matches harness version
3. **Rollback manifest**: write `.harness/rollback/<timestamp>.json` with file list and SHA256 hashes before any modifications
4. **Change scope summary**: print file counts (modified/created/deleted) and file names
5. **Confirmation**: require `y/N` input in apply mode (skipped in preview mode and `--yes` flag)

New CLI flag on `ApplyCommand`:
```rust
#[arg(long)]
pub allow_dirty: bool,
#[arg(long, short)]
pub yes: bool,
```

Selector requirement:
- Exactly one of `--plan-file <path>` or `--plan-all` must be provided.
- Enforced in CLI parser via mutual exclusion + required-unless-present.

Rollback manifest format:
```json
{
  "timestamp": "2026-02-27T10:00:00Z",
  "harness_version": "0.1.0",
  "files": [
    {"path": "harness.toml", "action": "modify", "sha256": "abc..."},
    {"path": ".harness/progress.md", "action": "create", "sha256": null}
  ]
}
```

**Files affected:** `src/cli.rs` (new flags), `src/generator/writer.rs`, PLAN.md Section 4.4

---

## P3 Gap Fixes (Spec Only, Implementation Deferred)

### P3-7: Benchmark Reproducibility Context

**Problem:** Benchmark metrics stored without environment context make cross-run comparisons unreliable.

**Design:** Each benchmark run stores a `bench_context` alongside metrics:

```json
{
  "bench_context": {
    "os": "darwin-aarch64",
    "toolchain": "rustc 1.77.0",
    "repo_ref": "abc123f",
    "repo_dirty": false,
    "harness_version": "0.1.0",
    "harness_config_hash": "sha256:...",
    "env_vars": ["RUST_LOG=debug"],
    "command": "cargo test --release",
    "seed": null,
    "timestamp": "2026-02-27T10:00:00Z",
    "wall_ms": 4523
  }
}
```

Comparison guard: `harness bench --compare` refuses mismatched `os`, `toolchain`, or `repo_dirty=true` unless `--force-compare` is passed.

**Files affected:** PLAN.md Sections 4.6, 10

### P3-8: Continuity Log Sampling

**Problem:** Logging every command run creates noise and write amplification.

**Design:**

Event classification:
- **Milestone events** (always logged): task start, task complete, verification pass/fail, error
- **Progress events** (sampled per config): individual command runs, file reads/writes

Batching: progress events accumulate in memory and flush on milestone events or every `batch_interval_secs` (default 60).

Rotation: when `progress.md` exceeds `max_log_size_kb` (default 100), rotate to `progress-<date>.md`. Keep last `retained_logs` (default 3) rotated files.

```toml
[continuity]
log_sampling = "milestones"      # milestones|all|none
batch_interval_secs = 60
max_log_size_kb = 100
retained_logs = 3
```

**Files affected:** PLAN.md Sections 5.2, 9.3

---

## Summary of Changes to Planning Docs

| Document | Sections to Update |
|---|---|
| PLAN.md | 4.4 (apply preconditions), 4.6 (bench context), 5.1 (TOML only), 5.2 (manifest: profiles, deprecated tools, optimization, continuity, metrics), 7 (clamping rules), 9.3 (log sampling), 10 (statistical gates), 11 (git requirement) |
| ARCHITECTURE.md | Section 8 (security model: tool lifecycle), Section 5 (apply preconditions) |
| Cargo.toml | Remove `serde_yaml` |
| src/cli.rs | Profile ValueEnum, ApplyCommand flags |
| src/optimization/presets.rs | Profile-keyed config structs |
| src/types/scoring.rs | Clamping, weighted_overall |
