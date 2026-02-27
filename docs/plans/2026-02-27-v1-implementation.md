# Harness v1.0 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

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
    let report = analyze::analyze(&model, &config);
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

pub struct CommandPolicy {
    pub forbidden: Vec<String>,
    pub aliases: HashMap<String, String>,
}

impl CommandPolicy {
    pub fn is_forbidden(&self, cmd: &str) -> bool {
        let expanded = self.expand_aliases(cmd);
        self.forbidden.iter().any(|f| expanded.starts_with(f))
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

## Deferred to v1.1

The following are explicitly out of scope for this plan:
- `harness optimize` command (trace module)
- `harness bench` command (benchmark module)
- `harness init` command implementation (generator/writer)
- `harness apply` command implementation (generator/writer + rollback)
- `harness suggest` command (recommendation engine)
- SARIF report format
- `ops` and `strict` profiles
- Namespace-based tool classification
- CI/CD pipeline

---

## Verification

After all tasks are complete:

1. `cargo build` — must compile with no errors (warnings OK for unused modules)
2. `cargo test` — all unit tests pass
3. `cargo run -- analyze /path/to/any/git/repo` — produces a real JSON or Markdown report with 5 scored categories
4. `cargo run -- analyze /path/to/any/git/repo --format json | python3 -m json.tool` — valid JSON
5. `cargo run -- analyze /tmp/empty-non-git-dir` — exits with code 3 and error message
6. Run against the harness repo itself: `cargo run -- analyze .` — should produce meaningful scores
