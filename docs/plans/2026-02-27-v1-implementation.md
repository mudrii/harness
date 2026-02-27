# Harness v1.0 Implementation Plan

**Goal:** Implement the `harness analyze` command end-to-end as the first working vertical slice — config loading, scanning, scoring, and report rendering.

**Architecture:** Layered pipeline: CLI dispatches to config loader → scanner (builds RepoModel) → analyzer (produces ScoreCard) → reporter (renders JSON/Markdown). Each layer is pure/deterministic except the scanner which reads the filesystem and git.

**Tech Stack:** Rust 2021 edition, clap 4.5, serde/toml for config, walkdir for filesystem, std::process::Command for git, thiserror for errors, anyhow for CLI-level error propagation.

---

## Phase 1: Foundation (Config + Types + Error Handling)

### Task 1: Define Error Types

**Files:**
- Create: `src/error.rs`
- Modify: `src/main.rs` (add `mod error`)

**Step 1: Write the error type**

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HarnessError {
    #[error("not a git repository: {0}")]
    NotGitRepo(String),

    #[error("config file not found: {0}")]
    ConfigNotFound(String),

    #[error("config parse error: {0}")]
    ConfigParse(String),

    #[error("path does not exist: {0}")]
    PathNotFound(String),

    #[error("invalid profile target: {0}")]
    InvalidProfileTarget(String),

    #[error("bucket penalty exceeded maximum: {0}")]
    BucketPenaltyExceeded(String),

    #[error("forbidden tool access attempt: {0}")]
    ForbiddenToolAccess(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, HarnessError>;
```

**Step 2: Register module in main.rs**

Add `mod error;` after `mod config;` in `src/main.rs`.

**Step 3: Run build to verify**

Run: `cargo build 2>&1`
Expected: compiles with existing dead_code warnings only

**Step 4: Commit**

```
git add src/error.rs src/main.rs
git commit -m "feat: add error types with thiserror"
```

---

### Task 2: Expand Config Types for TOML Deserialization

**Files:**
- Modify: `src/types/config.rs`

**Step 1: Write failing test**

```rust
// at bottom of src/types/config.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
[project]
name = "test-repo"
profile = "general"
language = "rust"
main_branch = "main"
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.project.name, "test-repo");
        assert_eq!(cfg.project.profile, "general");
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
[project]
name = "test-repo"
profile = "agent"
language = "rust"
main_branch = "main"

[context]
agents_map = "AGENTS.md"
doc_map_required = true

[tools.baseline]
read = ["cat", "rg"]
write = ["apply_patch"]
forbidden = ["sudo"]

[verification]
required = ["cargo test"]
pre_completion_required = true
loop_guard_enabled = true

[metrics]
max_risk_tolerance = 0.35
max_penalty_per_bucket = 0.40
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.project.profile, "agent");
        assert_eq!(cfg.tools.unwrap().baseline.unwrap().read, vec!["cat", "rg"]);
        assert!(cfg.verification.unwrap().pre_completion_required);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib types::config::tests -v 2>&1`
Expected: FAIL — `HarnessConfig` not defined

**Step 3: Write implementation**

Replace entire `src/types/config.rs` with:

```rust
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct HarnessConfig {
    pub project: ProjectConfig,
    pub context: Option<ContextConfig>,
    pub tools: Option<ToolsConfig>,
    pub verification: Option<VerificationConfig>,
    pub continuity: Option<ContinuityConfig>,
    pub metrics: Option<MetricsConfig>,
    pub workflow: Option<WorkflowConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default = "default_profile")]
    pub profile: String,
    pub language: Option<String>,
    #[serde(default = "default_branch")]
    pub main_branch: String,
}

fn default_profile() -> String { "general".to_string() }
fn default_branch() -> String { "main".to_string() }

#[derive(Debug, Clone, Deserialize)]
pub struct ContextConfig {
    pub agents_map: Option<String>,
    pub context_index: Option<String>,
    #[serde(default)]
    pub doc_map_required: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsConfig {
    pub baseline: Option<ToolBaseline>,
    pub specialized: Option<ToolSpecialized>,
    pub deprecated: Option<ToolDeprecated>,
    pub aliases: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolBaseline {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub write: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolSpecialized {
    #[serde(default)]
    pub extra: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolDeprecated {
    #[serde(default)]
    pub observe: Vec<String>,
    #[serde(default)]
    pub deprecated: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VerificationConfig {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub pre_completion_required: bool,
    #[serde(default)]
    pub loop_guard_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContinuityConfig {
    pub initializer: Option<String>,
    pub coding_prompt: Option<String>,
    pub progress_file: Option<String>,
    pub feature_state_file: Option<String>,
    pub state_schema_version: Option<u32>,
    pub log_sampling: Option<String>,
    pub batch_interval_secs: Option<u32>,
    pub max_log_size_kb: Option<u32>,
    pub retained_logs: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    pub weights: Option<HashMap<String, f32>>,
    pub max_risk_tolerance: Option<f32>,
    pub max_penalty_per_bucket: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowConfig {
    pub max_consecutive_failures: Option<u32>,
    pub max_idle_steps: Option<u32>,
    #[serde(default)]
    pub replan_on_loop: bool,
}

impl HarnessConfig {
    pub fn default_weights() -> [f32; 5] {
        [0.30, 0.25, 0.20, 0.15, 0.10]
    }

    pub fn weights(&self) -> [f32; 5] {
        match &self.metrics {
            Some(m) => match &m.weights {
                Some(w) => [
                    *w.get("context").unwrap_or(&0.30),
                    *w.get("tools").unwrap_or(&0.25),
                    *w.get("continuity").unwrap_or(&0.20),
                    *w.get("verification").unwrap_or(&0.15),
                    *w.get("repository_quality").unwrap_or(&0.10),
                ],
                None => Self::default_weights(),
            },
            None => Self::default_weights(),
        }
    }

    pub fn max_penalty_per_bucket(&self) -> f32 {
        self.metrics
            .as_ref()
            .and_then(|m| m.max_penalty_per_bucket)
            .unwrap_or(0.40)
    }
}

// Keep backward compat alias
pub type Config = HarnessConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
[project]
name = "test-repo"
profile = "general"
language = "rust"
main_branch = "main"
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.project.name, "test-repo");
        assert_eq!(cfg.project.profile, "general");
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
[project]
name = "test-repo"
profile = "agent"
language = "rust"
main_branch = "main"

[context]
agents_map = "AGENTS.md"
doc_map_required = true

[tools.baseline]
read = ["cat", "rg"]
write = ["apply_patch"]
forbidden = ["sudo"]

[verification]
required = ["cargo test"]
pre_completion_required = true
loop_guard_enabled = true

[metrics]
max_risk_tolerance = 0.35
max_penalty_per_bucket = 0.40
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.project.profile, "agent");
        assert_eq!(cfg.tools.unwrap().baseline.unwrap().read, vec!["cat", "rg"]);
        assert!(cfg.verification.unwrap().pre_completion_required);
    }

    #[test]
    fn default_weights() {
        let toml_str = r#"
[project]
name = "test"
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).unwrap();
        let w = cfg.weights();
        assert!((w.iter().sum::<f32>() - 1.0).abs() < 0.01);
    }
}
```

**Step 4: Run tests**

Run: `cargo test --lib types::config 2>&1`
Expected: 3 tests PASS

**Step 5: Commit**

```
git add src/types/config.rs
git commit -m "feat: add HarnessConfig TOML deserialization with tests"
```

---

### Task 3: Implement Config Loading with Layered Merge

**Files:**
- Modify: `src/config.rs`

**Requirements:**
Implement the strict 3-tier config merge hierarchy: Global (`$HOME/.config/harness/config.toml`) -> Repo (`harness.toml`) -> Local (`.harness/local.toml`).

**Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_repo_config() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("harness.toml");
        fs::write(&config_path, r#"
[project]
name = "test-repo"
profile = "agent"
"#).unwrap();
        let cfg = load_config(dir.path()).unwrap();
        assert_eq!(cfg.project.name, "test-repo");
    }

    #[test]
    fn missing_config_returns_none() {
        let dir = TempDir::new().unwrap();
        let result = load_config(dir.path());
        assert!(result.is_none());
    }
}
```

Note: add `tempfile = "3"` to `[dev-dependencies]` in Cargo.toml.

**Step 2: Run test to verify it fails**

Run: `cargo test --lib config::tests 2>&1`
Expected: FAIL — `load_config` not defined

**Step 3: Write implementation**

```rust
// src/config.rs
use crate::types::config::HarnessConfig;
use std::path::Path;

pub const DEFAULT_CONFIG_FILE: &str = "harness.toml";
pub const DEFAULT_LOCAL_FILE: &str = ".harness/local.toml";

pub fn load_config(root: &Path) -> Option<HarnessConfig> {
    let config_path = root.join(DEFAULT_CONFIG_FILE);
    if !config_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&config_path).ok()?;
    toml::from_str(&content).ok()
}
```

**Step 4: Run tests**

Run: `cargo test --lib config::tests 2>&1`
Expected: 2 tests PASS

**Step 5: Commit**

```
git add src/config.rs Cargo.toml
git commit -m "feat: implement config loading from harness.toml"
```

---

### Task 4: Add ScoreCard Methods (Clamping + Weighted Overall)

**Files:**
- Modify: `src/types/scoring.rs`

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_valid_scores() {
        let card = ScoreCard::new(0.5, 0.8, 0.3, 0.9, 0.7);
        let clamped = card.clamped();
        assert!((clamped.context - 0.5).abs() < 0.001);
    }

    #[test]
    fn clamp_caps_negative_and_overflow() {
        let card = ScoreCard {
            context: -0.2,
            tools: 1.5,
            continuity: 0.5,
            verification: 0.5,
            repository_quality: 0.5,
            overall: 0.0,
        };
        let clamped = card.clamped();
        assert!((clamped.context - 0.0).abs() < 0.001);
        assert!((clamped.tools - 1.0).abs() < 0.001);
    }

    #[test]
    fn bucket_penalty_limit_enforced() {
        let mut builder = CategoryScoreBuilder::new(1.0);
        builder.add_penalty(0.60); // Exceeds max
        let score = builder.build(0.40);
        assert!((score - 0.60).abs() < 0.001); // 1.0 - 0.40 = 0.60
    }

    #[test]
    fn weighted_overall_sums_correctly() {
        let weights = [0.30, 0.25, 0.20, 0.15, 0.10];
        let card = ScoreCard::new(1.0, 1.0, 1.0, 1.0, 1.0);
        let overall = card.weighted_overall(&weights);
        assert!((overall - 1.0).abs() < 0.001);
    }

    #[test]
    fn weighted_overall_with_zeros() {
        let weights = [0.30, 0.25, 0.20, 0.15, 0.10];
        let card = ScoreCard::new(0.0, 0.0, 0.0, 0.0, 0.0);
        let overall = card.weighted_overall(&weights);
        assert!((overall - 0.0).abs() < 0.001);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib types::scoring::tests 2>&1`
Expected: FAIL — `ScoreCard::new` not defined

**Step 3: Write implementation**

```rust
// src/types/scoring.rs
pub type Score = f32;

#[derive(Debug, Clone)]
pub struct CategoryScoreBuilder {
    pub base: f32,
    pub bonuses: f32,
    pub penalties: f32,
}

impl CategoryScoreBuilder {
    pub fn new(base: f32) -> Self {
        Self { base, bonuses: 0.0, penalties: 0.0 }
    }
    pub fn add_bonus(&mut self, val: f32) { self.bonuses += val; }
    pub fn add_penalty(&mut self, val: f32) { self.penalties += val; }
    pub fn build(&self, max_penalty: f32) -> f32 {
        (self.base + self.bonuses - self.penalties.min(max_penalty)).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct ScoreCard {
    pub context: Score,
    pub tools: Score,
    pub continuity: Score,
    pub verification: Score,
    pub repository_quality: Score,
    pub overall: Score,
}

impl ScoreCard {
    pub fn new(context: f32, tools: f32, continuity: f32, verification: f32, quality: f32) -> Self {
        Self {
            context,
            tools,
            continuity,
            verification,
            repository_quality: quality,
            overall: 0.0,
        }
    }

    pub fn clamped(&self) -> Self {
        Self {
            context: self.context.clamp(0.0, 1.0),
            tools: self.tools.clamp(0.0, 1.0),
            continuity: self.continuity.clamp(0.0, 1.0),
            verification: self.verification.clamp(0.0, 1.0),
            repository_quality: self.repository_quality.clamp(0.0, 1.0),
            overall: self.overall,
        }
    }

    pub fn weighted_overall(&self, weights: &[f32; 5]) -> f32 {
        let clamped = self.clamped();
        let scores = [
            clamped.context,
            clamped.tools,
            clamped.continuity,
            clamped.verification,
            clamped.repository_quality,
        ];
        scores.iter().zip(weights.iter()).map(|(s, w)| s * w).sum()
    }

    pub fn finalize(mut self, weights: &[f32; 5]) -> Self {
        self = self.clamped();
        self.overall = self.weighted_overall(weights);
        self
    }
}
```

**Step 4: Run tests**

Run: `cargo test --lib types::scoring::tests 2>&1`
Expected: 4 tests PASS

**Step 5: Commit**

```
git add src/types/scoring.rs
git commit -m "feat: add ScoreCard clamping and weighted overall calculation"
```

---

### Task 5: Expand Report Types (HarnessReport, Finding, Impact/Effort Enums)

**Files:**
- Modify: `src/types/report.rs`

**Step 1: Write the types and tests**

```rust
// src/types/report.rs
use serde::Serialize;
use crate::types::scoring::ScoreCard;

#[derive(Debug, Clone, Serialize)]
pub struct HarnessReport {
    pub overall_score: f32,
    pub category_scores: CategoryScores,
    pub findings: Vec<Finding>,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryScores {
    pub context: f32,
    pub tools: f32,
    pub continuity: f32,
    pub verification: f32,
    pub repository_quality: f32,
}

impl From<&ScoreCard> for CategoryScores {
    fn from(card: &ScoreCard) -> Self {
        Self {
            context: card.context,
            tools: card.tools,
            continuity: card.continuity,
            verification: card.verification,
            repository_quality: card.repository_quality,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub category: String,
    pub message: String,
    pub severity: Severity,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Severity {
    Info,
    Warning,
    Blocking,
}

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub id: String,
    pub title: String,
    pub confidence: f32,
    pub impact: Impact,
    pub effort: Effort,
    pub risk: Risk,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Impact { Low, Medium, High }

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Effort { Xs, S, M, L }

#[derive(Debug, Clone, Serialize)]
pub enum Risk { Safe, Medium, High }

impl Recommendation {
    pub fn sort_key(&self) -> impl Ord + '_ {
        (std::cmp::Reverse(self.impact.clone()), self.effort.clone(), &self.id)
    }
}

impl HarnessReport {
    pub fn new(card: &ScoreCard, findings: Vec<Finding>, recommendations: Vec<Recommendation>) -> Self {
        Self {
            overall_score: card.overall,
            category_scores: CategoryScores::from(card),
            findings,
            recommendations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendation_sort_by_impact_then_effort() {
        let mut recs = vec![
            Recommendation {
                id: "R-002".into(), title: "low".into(),
                confidence: 0.8, impact: Impact::Low, effort: Effort::S, risk: Risk::Safe,
            },
            Recommendation {
                id: "R-001".into(), title: "high".into(),
                confidence: 0.9, impact: Impact::High, effort: Effort::Xs, risk: Risk::Safe,
            },
        ];
        recs.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        assert_eq!(recs[0].id, "R-001");
    }
}
```

**Step 2: Run test**

Run: `cargo test --lib types::report::tests 2>&1`
Expected: PASS

**Step 3: Commit**

```
git add src/types/report.rs
git commit -m "feat: add HarnessReport, Finding, and recommendation sorting"
```

---

## Phase 2: Scanner Module

### Task 6: Implement RepoModel and Scanner Entry Point

**Files:**
- Modify: `src/scan/mod.rs`

**Step 1: Define RepoModel and write test**

```rust
// src/scan/mod.rs
pub mod filesystem;
pub mod docs;
pub mod tools;
pub mod git_meta;

use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RepoModel {
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub has_agents_md: bool,
    pub has_context_index: bool,
    pub has_architecture_docs: bool,
    pub readme_links_arch: bool,
    pub docs_age_days: Option<u64>,
    pub tool_names: Vec<String>,
    pub tool_overlap_clusters: usize,
    pub has_initializer_prompt: bool,
    pub has_coding_prompt: bool,
    pub has_progress_file: bool,
    pub has_feature_list: bool,
    pub verification_commands: Vec<String>,
    pub has_pre_completion_gate: bool,
    pub has_loop_guard: bool,
    pub has_ci_config: bool,
    pub has_test_directory: bool,
    pub has_lint_config: bool,
    pub has_harness_config: bool,
}

pub fn discover(root: &Path) -> RepoModel {
    let files = filesystem::list_files(root);
    let doc_signals = docs::detect_docs(root, &files);
    let tool_signals = tools::detect_tools(root);
    let git_signals = git_meta::git_metadata(root);

    RepoModel {
        root: root.to_path_buf(),
        files: files.clone(),
        has_agents_md: doc_signals.has_agents_md,
        has_context_index: doc_signals.has_context_index,
        has_architecture_docs: doc_signals.has_architecture_docs,
        readme_links_arch: doc_signals.readme_links_arch,
        docs_age_days: git_signals.docs_age_days,
        tool_names: tool_signals.tool_names,
        tool_overlap_clusters: tool_signals.overlap_clusters,
        has_initializer_prompt: doc_signals.has_initializer_prompt,
        has_coding_prompt: doc_signals.has_coding_prompt,
        has_progress_file: doc_signals.has_progress_file,
        has_feature_list: doc_signals.has_feature_list,
        verification_commands: tool_signals.verification_commands,
        has_pre_completion_gate: tool_signals.has_pre_completion_gate,
        has_loop_guard: tool_signals.has_loop_guard,
        has_ci_config: doc_signals.has_ci_config,
        has_test_directory: doc_signals.has_test_directory,
        has_lint_config: doc_signals.has_lint_config,
        has_harness_config: doc_signals.has_harness_config,
    }
}
```

Note: the sub-module signatures need updating to return structured types. Tasks 7-10 implement each scanner sub-module.

**Step 2: Commit after sub-modules are done (Task 10)**

---

### Task 7: Implement Filesystem Scanner

**Files:**
- Modify: `src/scan/filesystem.rs`

Use `walkdir` to enumerate files, filtering out `.git/` and `target/`.

```rust
// src/scan/filesystem.rs
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn list_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != "target" && name != "node_modules"
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn lists_files_excluding_git() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join("README.md"), "# test").unwrap();
        fs::write(dir.path().join(".git/config"), "").unwrap();
        let files = list_files(dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("README.md"));
    }
}
```

**Run:** `cargo test --lib scan::filesystem::tests 2>&1`
Expected: PASS

**Commit:** `git commit -m "feat: implement filesystem scanner with walkdir"`

---

### Task 8: Implement Docs Scanner

**Files:**
- Modify: `src/scan/docs.rs`

```rust
// src/scan/docs.rs
use std::path::{Path, PathBuf};

pub struct DocSignals {
    pub has_agents_md: bool,
    pub has_context_index: bool,
    pub has_architecture_docs: bool,
    pub readme_links_arch: bool,
    pub has_initializer_prompt: bool,
    pub has_coding_prompt: bool,
    pub has_progress_file: bool,
    pub has_feature_list: bool,
    pub has_ci_config: bool,
    pub has_test_directory: bool,
    pub has_lint_config: bool,
    pub has_harness_config: bool,
}

pub fn detect_docs(root: &Path, files: &[PathBuf]) -> DocSignals {
    let rel = |p: &Path| p.strip_prefix(root).unwrap_or(p);
    let has = |name: &str| files.iter().any(|f| {
        rel(f).to_string_lossy().to_lowercase().contains(&name.to_lowercase())
    });
    let has_exact = |name: &str| files.iter().any(|f| {
        rel(f).to_string_lossy() == name
    });

    let readme_links_arch = files.iter()
        .find(|f| rel(f).to_string_lossy() == "README.md")
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|c| c.contains("ARCHITECTURE") || c.contains("architecture"))
        .unwrap_or(false);

    DocSignals {
        has_agents_md: has_exact("AGENTS.md"),
        has_context_index: has("docs/context/INDEX.md") || has("docs/context/index.md"),
        has_architecture_docs: has_exact("ARCHITECTURE.md") || has("docs/architecture"),
        readme_links_arch,
        has_initializer_prompt: has(".harness/initializer.prompt.md"),
        has_coding_prompt: has(".harness/coding.prompt.md"),
        has_progress_file: has(".harness/progress.md"),
        has_feature_list: has(".harness/feature_list.json"),
        has_ci_config: has(".github/workflows") || has(".gitlab-ci") || has("Jenkinsfile"),
        has_test_directory: has("tests/") || has("test/") || has("spec/"),
        has_lint_config: has(".eslintrc") || has("rustfmt.toml") || has(".prettierrc") || has("clippy.toml"),
        has_harness_config: has_exact("harness.toml"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn detects_agents_md() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "# Agents").unwrap();
        let files = vec![dir.path().join("AGENTS.md")];
        let signals = detect_docs(dir.path(), &files);
        assert!(signals.has_agents_md);
    }

    #[test]
    fn detects_missing_agents_md() {
        let dir = TempDir::new().unwrap();
        let signals = detect_docs(dir.path(), &[]);
        assert!(!signals.has_agents_md);
    }
}
```

**Run:** `cargo test --lib scan::docs::tests 2>&1`
Expected: PASS

**Commit:** `git commit -m "feat: implement doc signal detection"`

---

### Task 9: Implement Git Metadata Scanner

**Files:**
- Modify: `src/scan/git_meta.rs`

```rust
// src/scan/git_meta.rs
use std::path::Path;
use std::process::Command;

pub struct GitSignals {
    pub docs_age_days: Option<u64>,
}

pub fn git_metadata(root: &Path) -> GitSignals {
    let docs_age_days = doc_age_days(root);
    GitSignals { docs_age_days }
}

fn doc_age_days(root: &Path) -> Option<u64> {
    // Get last commit timestamp for any .md file
    let output = Command::new("git")
        .args(["log", "-1", "--format=%ct", "--", "*.md"])
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let timestamp: i64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .ok()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;

    Some(((now - timestamp) / 86400) as u64)
}
```

Note: Git metadata tests require a real git repo — use integration tests (Task 14) rather than unit tests here.

**Commit:** `git commit -m "feat: implement git metadata scanner for doc age"`

---

### Task 10: Implement Tools Scanner

**Files:**
- Modify: `src/scan/tools.rs`

```rust
// src/scan/tools.rs
use std::path::Path;
use crate::types::config::HarnessConfig;

pub struct ToolSignals {
    pub tool_names: Vec<String>,
    pub overlap_clusters: usize,
    pub verification_commands: Vec<String>,
    pub has_pre_completion_gate: bool,
    pub has_loop_guard: bool,
}

pub fn detect_tools(root: &Path) -> ToolSignals {
    let config_path = root.join("harness.toml");
    let config: Option<HarnessConfig> = std::fs::read_to_string(&config_path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok());

    match config {
        Some(cfg) => {
            let mut tool_names = Vec::new();
            if let Some(tools) = &cfg.tools {
                if let Some(baseline) = &tools.baseline {
                    tool_names.extend(baseline.read.clone());
                    tool_names.extend(baseline.write.clone());
                }
            }
            let overlap_clusters = count_overlaps(&tool_names);
            let verification_commands = cfg.verification
                .as_ref()
                .map(|v| v.required.clone())
                .unwrap_or_default();
            let has_pre_completion_gate = cfg.verification
                .as_ref()
                .map(|v| v.pre_completion_required)
                .unwrap_or(false);
            let has_loop_guard = cfg.verification
                .as_ref()
                .map(|v| v.loop_guard_enabled)
                .unwrap_or(false);

            ToolSignals {
                tool_names,
                overlap_clusters,
                verification_commands,
                has_pre_completion_gate,
                has_loop_guard,
            }
        }
        None => ToolSignals {
            tool_names: Vec::new(),
            overlap_clusters: 0,
            verification_commands: Vec::new(),
            has_pre_completion_gate: false,
            has_loop_guard: false,
        },
    }
}

fn count_overlaps(tools: &[String]) -> usize {
    let grep_like = ["grep", "rg", "ag", "ack"];
    let find_like = ["find", "fd", "locate"];

    let mut clusters = 0;
    let grep_count = tools.iter().filter(|t| grep_like.iter().any(|g| t.contains(g))).count();
    if grep_count > 1 { clusters += 1; }
    let find_count = tools.iter().filter(|t| find_like.iter().any(|f| t.contains(f))).count();
    if find_count > 1 { clusters += 1; }
    clusters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_grep_overlap() {
        let tools = vec!["grep".into(), "rg".into(), "cat".into()];
        assert_eq!(count_overlaps(&tools), 1);
    }

    #[test]
    fn no_overlap() {
        let tools = vec!["rg".into(), "cat".into()];
        assert_eq!(count_overlaps(&tools), 0);
    }
}
```

**Run:** `cargo test --lib scan::tools::tests 2>&1`
Expected: PASS

**Commit after wiring scan/mod.rs:**

```
git add src/scan/
git commit -m "feat: implement scanner module (filesystem, docs, tools, git)"
```

---

## Phase 3: Analyzer Module

### Task 11: Implement All 5 Scoring Functions

**Files:**
- Modify: `src/analyze/context.rs`, `tools.rs`, `continuity.rs`, `verification.rs`, `quality.rs`
- Modify: `src/analyze/mod.rs`

Each scoring function takes `&RepoModel` and returns `f32`, implementing the rules from PLAN.md Section 7.2. The `mod.rs` wires them into a `ScoreCard`.

This is where the user's domain knowledge matters most — the scoring rules encode the value judgments of what makes a "good" harness.

**Implementation:** Each function applies additive bonuses/penalties per PLAN.md §7.2, then the caller clamps via `ScoreCard::finalize()`.

**Tests:** Each scorer gets 2-3 unit tests with mock RepoModel data.

**Commit:** `git commit -m "feat: implement 5-category scoring engine"`

---

### Task 12: Wire Analyze Command End-to-End

**Files:**
- Modify: `src/main.rs`
- Modify: `src/analyze/mod.rs`

Replace the `println` stub in `main.rs` for the `Analyze` command with the real pipeline:

```rust
cli::Commands::Analyze(cmd) => {
    let root = &cmd.path;
    if !root.exists() {
        eprintln!("error: path does not exist: {}", root.display());
        std::process::exit(3);
    }
    let config = config::load_config(root);
    let model = scan::discover(root);
    let mut report = analyze::analyze(&model, &config);
    
    // Filter recommendations by min-impact
    if let Some(min) = &cmd.min_impact {
        report.recommendations.retain(|r| r.impact >= *min);
    }

    match cmd.format {
        cli::ReportFormat::Json => println!("{}", report::json::to_json(&report)),
        cli::ReportFormat::Md => println!("{}", report::md::to_markdown(&report)),
        cli::ReportFormat::Sarif => println!("{}", report::sarif::to_sarif(&report)),
    }
    if report.findings.iter().any(|f| matches!(f.severity, types::report::Severity::Blocking)) {
        std::process::exit(2);
    } else if !report.findings.is_empty() {
        std::process::exit(1);
    }
}
```

**Commit:** `git commit -m "feat: wire analyze command to scanner/scorer/reporter pipeline"`

---

## Phase 4: Report Renderers

### Task 13: Implement JSON and Markdown Report Renderers

**Files:**
- Modify: `src/report/json.rs`, `src/report/md.rs`

**JSON renderer:** Use `serde_json::to_string_pretty(&report)`.

**Markdown renderer:** Format scores as a table, findings as a list, recommendations as a ranked list.

**Tests:** Compare rendered output against expected strings for a known report.

**Commit:** `git commit -m "feat: implement JSON and Markdown report renderers"`

---

## Phase 5: Git Requirement Gate

### Task 14: Add Git Presence Check

**Files:**
- Modify: `src/main.rs`

Before dispatching any command, check for `.git`:

```rust
let root = match &cli.command {
    cli::Commands::Analyze(cmd) => &cmd.path,
    cli::Commands::Init(cmd) => &cmd.path,
    // ... all commands
};
if !root.join(".git").exists() {
    eprintln!("error: {} is not a git repository. harness v1 requires git.", root.display());
    std::process::exit(3);
}
```

**Test:** Integration test in `tests/integration.rs` that runs the binary against a non-git temp dir and asserts exit code 3.

Add to Cargo.toml dev-deps: `assert_cmd = "2"` and `predicates = "3"`.

**Commit:** `git commit -m "feat: require git repository, reject non-git paths"`

---

## Phase 6: Command Policy (P2-5)

### Task 15: Implement Command Policy with Alias Expansion

**Files:**
- Modify: `src/guardrails/command_policy.rs`

```rust
use std::collections::HashMap;

pub enum ToolStatus {
    Allowed,
    Observe,
    Deprecated,
    Disabled,
}

pub struct CommandPolicy {
    pub observe: Vec<String>,
    pub deprecated: Vec<String>,
    pub disabled: Vec<String>,
    pub aliases: HashMap<String, String>,
}

impl CommandPolicy {
    pub fn check_tool(&self, cmd: &str) -> ToolStatus {
        let expanded = self.expand_aliases(cmd);
        if self.disabled.iter().any(|f| expanded.starts_with(f)) {
            return ToolStatus::Disabled;
        }
        if self.deprecated.iter().any(|f| expanded.starts_with(f)) {
            return ToolStatus::Deprecated;
        }
        if self.observe.iter().any(|f| expanded.starts_with(f)) {
            return ToolStatus::Observe;
        }
        ToolStatus::Allowed
    }

    fn expand_aliases(&self, cmd: &str) -> String {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        let base = parts[0];
        match self.aliases.get(base) {
            Some(resolved) => {
                if parts.len() > 1 {
                    format!("{} {}", resolved, parts[1])
                } else {
                    resolved.clone()
                }
            }
            None => cmd.to_string(),
        }
    }
}
```

**Tests:** 4-5 tests covering forbidden matching, alias expansion, argument-aware matching.

**Commit:** `git commit -m "feat: implement command policy with alias expansion"`

---

---

## Phase 7: Scaffold and Write Mode (`init` and `apply`)

### Task 16: Implement CLI Profile Validation Enum

**Files:**
- Modify: `src/cli.rs`

Replace `String` profile types with a `clap::ValueEnum`.

```rust
use clap::ValueEnum;

#[derive(Clone, ValueEnum, Debug)]
pub enum Profile {
    General,
    Agent,
}
```
Update all config generation and validation paths to depend on this strict profile type.

### Task 17: Implement Init & Apply Commands (V1 Scope)

**Files:**
- Modify: `src/generator/writer.rs`
- Modify: `src/cli.rs`

Implement the strict 5 preconditions for the `apply` command:
1. Check clean working tree (unless `--allow-dirty`).
2. Plan file validation: must exist, parse correctly, reject path traversal attempts, and match current harness version constraints.
3. Rollback manifest generation (`.harness/rollback/<timestamp>.json` with file list + SHA256 hashes).
4. Change scope summary: print detailed modification manifest to stdout before prompting.
5. Explicit `y/N` confirmation prompts.

Additionally, handle File writing logic:
- Audit Headers: Every newly created markdown/configuration file must start with a `# Generated by harness` tracking header.
- Log Extents: When modifying `.harness/progress.md`, verify `max_log_size_kb` triggers logs to rotate to `progress-<date>.md` while reaping oldest bounds to match `retained_logs`.

These are confirmed to be strictly within the v1 scope.

### Task 18: Implement Linting Core & CLI

**Files:**
- Create: `src/analyze/lint.rs`
- Modify: `src/main.rs`

Implement the `harness lint` command to check policy conformance:
1. Verify required foundational files exist as specified by the active profile.
2. Confirm active tool configurations do not violate hard profile strictures (e.g., `forbidden` commands).
3. Distinguish between blocking (returns exit code 2) and non-blocking warnings in the report stream.

### Task 19: Global Exit Code Propagation

**Files:**
- Modify: `src/main.rs`

Clean up the mock exits and propagate real command validation results:
- Exit `1` explicitly mapped only for `Analyze` when non-blocking warnings surface.
- Exit `2` mapped strictly to `Lint` findings or blocked preconditions.
- Exit `3` tied rigorously to `HarnessError` bubbled up `Result` aborts.

---

## Module Readiness Tracker

| Module | Status | Task | Dependencies |
|--------|--------|------|-------------|
| error | pending | Task 1 | none |
| types/config | pending | Task 2 | none |
| config | pending | Task 3 | Task 2 |
| types/scoring | pending | Task 4 | none |
| types/report | pending | Task 5 | Task 4 |
| scan/mod | pending | Task 6 | Tasks 7-10 |
| scan/filesystem | pending | Task 7 | none |
| scan/docs | pending | Task 8 | none |
| scan/git_meta | pending | Task 9 | none |
| scan/tools | pending | Task 10 | Task 2 |
| analyze/* | pending | Task 11 | Tasks 4, 6 |
| main (wiring) | pending | Task 12 | Tasks 11, 13 |
| report/json,md | pending | Task 13 | Task 5 |
| git gate | pending | Task 14 | Task 12 |
| guardrails/policy | pending | Task 15 | Task 2 |
| cli (enums) | done | Task 16 | none |
| apply preconditions | pending | Task 17 | Task 3 |
| analyze/lint | pending | Task 18 | Task 15 |
| main (exit codes) | in-progress | Task 19 | none |

Status values: pending | in-progress | done | blocked

---

## Deferred to later v1.0 milestones

The following are correctly in v1 scope but purposefully excluded from this *first vertical slice implementation plan*. They will be covered in subsequent implementation plans following milestone M1:
- `harness optimize` command (trace module)
- `harness bench` command (benchmark module)
- `harness suggest` command (recommendation engine)

## Deferred to v1.1

The following are explicitly outside the v1 release scope:
- SARIF report format
- `ops` and `strict` profiles
- Namespace-based tool classification
- CI/CD pipeline

---

## Mandatory: TDD, ATDD, Unit Testing, and Linting

Every task in this plan MUST follow strict TDD and ATDD methodology. This is non-negotiable.

### TDD Rules (Unit Level)

1. **Red-Green-Refactor cycle is mandatory:**
   - Write a failing test first that defines the expected behavior
   - Run the test, confirm it fails for the right reason
   - Write the minimum code to make the test pass
   - Run the test, confirm it passes
   - Refactor if needed, re-run tests

2. **Minimum test coverage per module:**

   | Module | Required Tests |
   |--------|---------------|
   | `error.rs` | 1: error variant display messages |
   | `types/config.rs` | 3: minimal parse, full parse, default weights |
   | `config.rs` | 2: load existing config, missing config returns None |
   | `types/scoring.rs` | 4: clamp valid, clamp overflow, weighted sum, finalize |
   | `types/report.rs` | 1: recommendation sorting |
   | `scan/filesystem.rs` | 1: list files excluding .git/target |
   | `scan/docs.rs` | 2: detect present signals, detect absent signals |
   | `scan/tools.rs` | 2: count overlap clusters, no overlap |
   | `scan/git_meta.rs` | (integration test only — requires git repo) |
   | `analyze/*.rs` | 2-3 per scorer: full marks, zero marks, partial |
   | `report/json.rs` | 1: valid JSON output |
   | `report/md.rs` | 1: markdown contains expected sections |
   | `guardrails/command_policy.rs` | 4: forbidden match, alias expansion, args-aware, safe pass |
   | `cli.rs` | 3: valid enum parse, invalid enum rejection, default values |

3. **Test naming convention:** `test_<behavior_under_test>`, e.g., `test_clamp_caps_negative_scores`

4. **No implementation code without a corresponding test.** If you can't write a test first, you don't understand the requirement well enough.

### ATDD Rules (Acceptance Level)

Each phase has an end-to-end acceptance test that verifies user-visible behavior:

| Phase | Acceptance Test |
|-------|----------------|
| Phase 1 (Foundation) | `cargo test` passes, all config types deserialize, ScoreCard math is correct |
| Phase 2 (Scanner) | `scan::discover()` returns a populated RepoModel for a real git repo |
| Phase 3 (Analyzer) | `analyze::analyze()` returns a report with 5 non-zero scores |
| Phase 4 (Reports) | `harness analyze . --format json` produces valid JSON with correct structure |
| Phase 5 (Git Gate) | `harness analyze /tmp/non-git-dir` exits with code 3 |
| Phase 6 (Policy) | Command policy correctly blocks aliased forbidden commands |

Integration tests live in `tests/`:
- `tests/integration.rs` — binary-level tests using `assert_cmd` + `predicates`
- `tests/fixtures/` — test data (minimal git repos, sample configs)

### Linting Rules (Mandatory)

1. **`cargo clippy` must pass with no warnings** before any commit:
   - Run: `cargo clippy -- -D warnings`
   - Fix all warnings before committing
   - Common clippy issues to watch for: `needless_return`, `redundant_clone`, `unused_imports`

2. **`cargo fmt --check` must pass** before any commit:
   - All code must be formatted with `rustfmt`
   - Run: `cargo fmt` to auto-format, then `cargo fmt --check` to verify

3. **Pre-commit verification sequence** (run in this order):
   ```
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   cargo build
   ```

4. **No `#[allow(dead_code)]` without justification** — stub modules may have dead code during development, but each `allow` must be removed when the module is implemented.

### Test Infrastructure

Dev dependencies (add to Cargo.toml):
```toml
[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

Test organization:
- Unit tests: `#[cfg(test)] mod tests` at bottom of each source file
- Integration tests: `tests/integration.rs`
- Test fixtures: `tests/fixtures/` (created by test setup, not committed)

### Testing Anti-Patterns to Avoid

- No `#[ignore]` tests without a documented reason
- No tests that depend on external network access
- No tests that depend on specific filesystem paths outside tempdir
- No tests that mutate global state
- No assertion-free tests (every test must assert something meaningful)

---

## Verification

After all tasks are complete:

1. `cargo build` — must compile with no errors (warnings OK for unused modules)
2. `cargo test` — all unit tests pass
3. `cargo run -- analyze /path/to/any/git/repo` — produces a real JSON or Markdown report with 5 scored categories
4. `cargo run -- analyze /path/to/any/git/repo --format json | python3 -m json.tool` — valid JSON
5. `cargo run -- analyze /tmp/empty-non-git-dir` — exits with code 3 and error message
6. Run against the harness repo itself: `cargo run -- analyze .` — should produce meaningful scores
