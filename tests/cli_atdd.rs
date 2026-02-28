#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as ProcessCommand;
use tempfile::TempDir;

fn init_git_repo(path: &std::path::Path) {
    let output = ProcessCommand::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .expect("git init should run");
    assert!(output.status.success(), "git init should succeed");
}

#[test]
fn apply_requires_exactly_one_plan_selector() {
    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("apply")
        .arg(".")
        .arg("--apply-mode")
        .arg("preview")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn apply_rejects_both_plan_selectors() {
    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("apply")
        .arg(".")
        .arg("--plan-file")
        .arg("plan.json")
        .arg("--plan-all")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn apply_rejects_plan_file_path_traversal() {
    let repo = TempDir::new().expect("temp dir should be created");
    init_git_repo(repo.path());

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("apply")
        .arg(repo.path())
        .arg("--plan-file")
        .arg("../plan.json")
        .arg("--apply-mode")
        .arg("preview")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("path traversal rejected"));
}

#[test]
fn apply_plan_all_preview_prints_scope() {
    let repo = TempDir::new().expect("temp dir should be created");
    init_git_repo(repo.path());

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("apply")
        .arg(repo.path())
        .arg("--plan-all")
        .arg("--apply-mode")
        .arg("preview")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("scope:"))
        .stdout(predicate::str::contains("docs/context/INDEX.md"));
}

#[test]
fn apply_rejects_dirty_worktree_without_allow_dirty() {
    let repo = TempDir::new().expect("temp dir should be created");
    init_git_repo(repo.path());
    fs::write(repo.path().join("untracked.txt"), "dirty").expect("dirty file should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("apply")
        .arg(repo.path())
        .arg("--plan-all")
        .arg("--apply-mode")
        .arg("preview")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("working tree is dirty"));
}

#[test]
fn analyze_requires_git_repository() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"
"#,
    )
    .expect("repo config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("analyze")
        .arg(repo.path())
        .assert()
        .code(3)
        .stderr(predicate::str::contains("not a git repository"));
}

#[test]
fn analyze_returns_warning_when_git_repo_has_no_repo_config() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("analyze")
        .arg(repo.path())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no harness.toml found"));
}

#[test]
fn analyze_json_outputs_report_for_well_formed_repo() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git should create");
    fs::create_dir_all(repo.path().join("docs/context")).expect("context dir should create");
    fs::write(repo.path().join("AGENTS.md"), "# Agents\nmap").expect("agents should write");
    fs::write(
        repo.path().join("README.md"),
        "Architecture reference: ARCHITECTURE.md",
    )
    .expect("readme should write");
    fs::write(repo.path().join("ARCHITECTURE.md"), "# Architecture").expect("arch should write");
    fs::write(repo.path().join("docs/context/INDEX.md"), "index").expect("index should write");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[verification]
required = ["cargo check"]
pre_completion_required = true
loop_guard_enabled = true
"#,
    )
    .expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("analyze")
        .arg(repo.path())
        .arg("--format")
        .arg("json")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("\"overall_score\""));
}

#[test]
fn analyze_fails_on_malformed_repo_config() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git should create");
    fs::write(repo.path().join("harness.toml"), "[project").expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("analyze")
        .arg(repo.path())
        .assert()
        .code(3)
        .stderr(predicate::str::contains("config parse error"));
}

#[test]
fn analyze_fails_on_invalid_project_profile() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "ops"
"#,
    )
    .expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("analyze")
        .arg(repo.path())
        .assert()
        .code(3)
        .stderr(predicate::str::contains("unsupported project.profile"));
}

#[test]
fn analyze_uses_repo_config_over_invalid_global_profile() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[verification]
required = ["cargo check"]
pre_completion_required = true
loop_guard_enabled = true
"#,
    )
    .expect("repo config should write");

    let fake_home = TempDir::new().expect("temp home should be created");
    let global_config_dir = fake_home.path().join(".config/harness");
    fs::create_dir_all(&global_config_dir).expect("global config dir should create");
    fs::write(
        global_config_dir.join("config.toml"),
        r#"
[project]
name = "global"
profile = "invalid-profile"
"#,
    )
    .expect("global config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.env("HOME", fake_home.path())
        .arg("analyze")
        .arg(repo.path())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no harness.toml").not());
}

#[test]
fn analyze_uses_local_override_over_repo_profile() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"
"#,
    )
    .expect("repo config should write");
    fs::create_dir_all(repo.path().join(".harness")).expect("local dir should create");
    fs::write(
        repo.path().join(".harness/local.toml"),
        r#"
[project]
profile = "ops"
"#,
    )
    .expect("local override should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("analyze")
        .arg(repo.path())
        .assert()
        .code(3)
        .stderr(predicate::str::contains("unsupported project.profile"));
}

#[test]
fn lint_returns_warning_when_git_repo_has_no_repo_config() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("lint")
        .arg(repo.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("verification.missing_config"));
}

#[test]
fn lint_reports_non_blocking_when_tools_are_observed() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[verification]
required = ["cargo check"]
pre_completion_required = true
loop_guard_enabled = true

[tools.deprecated]
observe = ["grep"]
"#,
    )
    .expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("lint")
        .arg(repo.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("tools.observe"));
}

#[test]
fn lint_returns_blocking_when_verification_is_incomplete() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"
"#,
    )
    .expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("lint")
        .arg(repo.path())
        .assert()
        .code(2)
        .stdout(predicate::str::contains("verification.incomplete"));
}

#[test]
fn lint_reports_blocking_when_tools_are_deprecated() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[tools.deprecated]
deprecated = ["grep"]
"#,
    )
    .expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("lint")
        .arg(repo.path())
        .assert()
        .code(2)
        .stdout(predicate::str::contains("tools.deprecated"));
}

#[test]
fn suggest_outputs_ranked_recommendations() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("suggest")
        .arg(repo.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("suggestions:"))
        .stdout(predicate::str::contains("rec.context.index"));
}

#[test]
fn suggest_without_tool_pressure_does_not_emit_tools_prune() {
    let repo = TempDir::new().expect("temp dir should be created");
    init_git_repo(repo.path());

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("suggest")
        .arg(repo.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("rec.tools.prune").not());
}

#[test]
fn suggest_export_diff_writes_plan_file() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("suggest")
        .arg(repo.path())
        .arg("--export-diff")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("plan file:"));

    let plans_dir = repo.path().join(".harness/plans");
    let entries = fs::read_dir(plans_dir)
        .expect("plans directory should exist")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("plans entries should be readable");
    assert!(
        !entries.is_empty(),
        "at least one plan file should be written"
    );
}

#[test]
fn init_dry_run_does_not_write_files() {
    let repo = TempDir::new().expect("temp dir should be created");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("init")
        .arg(repo.path())
        .arg("--dry-run")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("dry-run: no files were written"));

    assert!(!repo.path().join("harness.toml").exists());
    assert!(!repo.path().join("AGENTS.md").exists());
}

#[test]
fn init_writes_baseline_files() {
    let repo = TempDir::new().expect("temp dir should be created");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("init").arg(repo.path()).assert().code(0);

    assert!(repo.path().join("harness.toml").exists());
    assert!(repo.path().join("AGENTS.md").exists());
    assert!(repo.path().join("docs/context/INDEX.md").exists());
}

#[test]
fn init_no_overwrite_preserves_existing_harness_toml() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::write(repo.path().join("harness.toml"), "custom=true").expect("file should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("init")
        .arg(repo.path())
        .arg("--no-overwrite")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("skip existing"));

    let content =
        fs::read_to_string(repo.path().join("harness.toml")).expect("file should be readable");
    assert_eq!(content, "custom=true");
}

#[test]
fn bench_writes_context_report_file() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("bench")
        .arg(repo.path())
        .arg("--runs")
        .arg("2")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("bench report:"));

    let reports = fs::read_dir(repo.path().join(".harness/bench"))
        .expect("bench dir should exist")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("entries should be readable");
    assert!(
        !reports.is_empty(),
        "bench should write at least one report"
    );
}

#[test]
fn bench_compare_rejects_incompatible_context_without_force() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::create_dir_all(repo.path().join(".harness/bench")).expect("bench dir should create");
    let baseline_path = repo.path().join(".harness/bench/baseline.json");
    fs::write(
        &baseline_path,
        r#"{
  "bench_context": {
    "os": "different-os",
    "toolchain": "rustc 1.77.0",
    "repo_ref": "abc",
    "repo_dirty": false,
    "harness_version": "0.1.0",
    "suite": "default",
    "timestamp": "2026-02-27T00:00:00Z"
  },
  "runs": [
    {"run": 1, "overall_score": 0.50}
  ]
}"#,
    )
    .expect("baseline report should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("bench")
        .arg(repo.path())
        .arg("--compare")
        .arg(&baseline_path)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("bench compare blocked"));
}

#[test]
fn bench_compare_allows_incompatible_context_with_force() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::create_dir_all(repo.path().join(".harness/bench")).expect("bench dir should create");
    let baseline_path = repo.path().join(".harness/bench/baseline.json");
    fs::write(
        &baseline_path,
        r#"{
  "bench_context": {
    "os": "different-os",
    "toolchain": "rustc 1.77.0",
    "repo_ref": "abc",
    "repo_dirty": false,
    "harness_version": "0.1.0",
    "suite": "default",
    "timestamp": "2026-02-27T00:00:00Z"
  },
  "runs": [
    {"run": 1, "overall_score": 0.50}
  ]
}"#,
    )
    .expect("baseline report should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("bench")
        .arg(repo.path())
        .arg("--compare")
        .arg(&baseline_path)
        .arg("--force-compare")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("bench compare:"));
}

#[test]
fn optimize_writes_report_file() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("optimize")
        .arg(repo.path())
        .assert()
        .code(0)
        .stdout(predicate::str::contains("optimize report:"));

    let reports = fs::read_dir(repo.path().join(".harness/optimize"))
        .expect("optimize dir should exist")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("entries should be readable");
    assert!(
        !reports.is_empty(),
        "optimize should write at least one report"
    );

    let first_report = reports
        .first()
        .expect("at least one optimize report should exist")
        .path();
    let report_content =
        fs::read_to_string(first_report).expect("optimize report should be readable");
    assert!(
        report_content.contains("insufficient data"),
        "optimize should gate recommendations when traces are below threshold"
    );
}

#[test]
fn apply_rejects_disabled_tool_policy() {
    let repo = TempDir::new().expect("temp dir should be created");
    init_git_repo(repo.path());
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[tools.deprecated]
disabled = ["apply_patch"]
"#,
    )
    .expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("apply")
        .arg(repo.path())
        .arg("--plan-all")
        .arg("--apply-mode")
        .arg("preview")
        .arg("--allow-dirty")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("forbidden tool access attempt"));
}

#[test]
fn optimize_with_sufficient_traces_renders_recommendations() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[optimization]
min_traces = 1
"#,
    )
    .expect("config should write");
    let trace_dir = repo.path().join("custom-traces");
    fs::create_dir_all(&trace_dir).expect("trace dir should create");
    let now = chrono::Utc::now().to_rfc3339();
    fs::write(
        trace_dir.join("run.jsonl"),
        format!(
            concat!(
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-1\",\"revision\":\"rev-a\",\"outcome\":\"success\",\"steps\":10,\"token_est\":100}}\n",
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-1\",\"revision\":\"rev-b\",\"outcome\":\"success\",\"steps\":10,\"token_est\":100}}\n"
            ),
            now
        ),
    )
    .expect("trace file should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("optimize")
        .arg(repo.path())
        .arg("--trace-dir")
        .arg(&trace_dir)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("optimize report:"));

    let reports = fs::read_dir(repo.path().join(".harness/optimize"))
        .expect("optimize dir should exist")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("entries should be readable");
    let first_report = reports
        .first()
        .expect("at least one optimize report should exist")
        .path();
    let report_content =
        fs::read_to_string(first_report).expect("optimize report should be readable");
    assert!(
        report_content.contains("## Top Recommendations"),
        "optimize should emit recommendations when trace threshold is met"
    );
    assert!(
        !report_content.contains("insufficient data"),
        "optimize should not emit insufficient data when threshold is met"
    );
}

#[test]
fn optimize_surfaces_malformed_trace_warning_without_failing() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[optimization]
min_traces = 1
"#,
    )
    .expect("config should write");
    let trace_dir = repo.path().join("traces");
    fs::create_dir_all(&trace_dir).expect("trace dir should create");
    fs::write(
        trace_dir.join("run.jsonl"),
        format!(
            "{{\"timestamp\":\"{}\"}}\nnot-json\n",
            chrono::Utc::now().to_rfc3339()
        ),
    )
    .expect("trace file should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("optimize")
        .arg(repo.path())
        .arg("--trace-dir")
        .arg(&trace_dir)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("optimize report:"));

    let reports = fs::read_dir(repo.path().join(".harness/optimize"))
        .expect("optimize dir should exist")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("entries should be readable");
    let first_report = reports
        .first()
        .expect("at least one optimize report should exist")
        .path();
    let report_content =
        fs::read_to_string(first_report).expect("optimize report should be readable");
    assert!(
        report_content.contains("ignored malformed trace records: 1"),
        "optimize should report malformed traces as a warning"
    );
}

#[test]
fn optimize_reports_improvement_when_deltas_exceed_thresholds() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[optimization]
min_traces = 1
min_uplift_abs = 0.05
min_uplift_rel = 0.10
task_overlap_threshold = 0.50
"#,
    )
    .expect("config should write");
    let trace_dir = repo.path().join("traces");
    fs::create_dir_all(&trace_dir).expect("trace dir should create");
    let now = chrono::Utc::now().to_rfc3339();
    fs::write(
        trace_dir.join("run.jsonl"),
        format!(
            concat!(
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-1\",\"revision\":\"rev-a\",\"outcome\":\"failure\",\"steps\":20,\"token_est\":200}}\n",
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-2\",\"revision\":\"rev-a\",\"outcome\":\"success\",\"steps\":20,\"token_est\":200}}\n",
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-1\",\"revision\":\"rev-b\",\"outcome\":\"success\",\"steps\":10,\"token_est\":100}}\n",
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-2\",\"revision\":\"rev-b\",\"outcome\":\"success\",\"steps\":10,\"token_est\":100}}\n"
            ),
            now
        ),
    )
    .expect("trace file should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("optimize")
        .arg(repo.path())
        .arg("--trace-dir")
        .arg(&trace_dir)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("optimize report:"));

    let reports = fs::read_dir(repo.path().join(".harness/optimize"))
        .expect("optimize dir should exist")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("entries should be readable");
    let first_report = reports
        .first()
        .expect("at least one optimize report should exist")
        .path();
    let report_content =
        fs::read_to_string(first_report).expect("optimize report should be readable");
    assert!(
        report_content.contains("Status: improvement detected."),
        "optimize should classify significant positive deltas as improvement"
    );
}

#[test]
fn optimize_reports_insufficient_comparative_data_when_overlap_is_low() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[optimization]
min_traces = 1
task_overlap_threshold = 0.80
"#,
    )
    .expect("config should write");
    let trace_dir = repo.path().join("traces");
    fs::create_dir_all(&trace_dir).expect("trace dir should create");
    let now = chrono::Utc::now().to_rfc3339();
    fs::write(
        trace_dir.join("run.jsonl"),
        format!(
            concat!(
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-a\",\"revision\":\"rev-a\",\"outcome\":\"success\",\"steps\":10,\"token_est\":100}}\n",
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-b\",\"revision\":\"rev-a\",\"outcome\":\"success\",\"steps\":10,\"token_est\":100}}\n",
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-c\",\"revision\":\"rev-b\",\"outcome\":\"success\",\"steps\":10,\"token_est\":100}}\n",
                "{{\"timestamp\":\"{0}\",\"task_id\":\"task-d\",\"revision\":\"rev-b\",\"outcome\":\"success\",\"steps\":10,\"token_est\":100}}\n"
            ),
            now
        ),
    )
    .expect("trace file should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("optimize")
        .arg(repo.path())
        .arg("--trace-dir")
        .arg(&trace_dir)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("optimize report:"));

    let reports = fs::read_dir(repo.path().join(".harness/optimize"))
        .expect("optimize dir should exist")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("entries should be readable");
    let first_report = reports
        .first()
        .expect("at least one optimize report should exist")
        .path();
    let report_content =
        fs::read_to_string(first_report).expect("optimize report should be readable");
    assert!(
        report_content.contains("Status: insufficient comparative data for optimize deltas."),
        "optimize should block comparisons with low task overlap"
    );
}

#[test]
fn analyze_writes_continuity_log_entries() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git should create");
    fs::create_dir_all(repo.path().join("docs/context")).expect("context dir should create");
    fs::write(repo.path().join("AGENTS.md"), "# Agents\nmap").expect("agents should write");
    fs::write(
        repo.path().join("README.md"),
        "Architecture reference: ARCHITECTURE.md",
    )
    .expect("readme should write");
    fs::write(repo.path().join("ARCHITECTURE.md"), "# Architecture").expect("arch should write");
    fs::write(repo.path().join("docs/context/INDEX.md"), "index").expect("index should write");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[verification]
required = ["cargo check"]
pre_completion_required = true
loop_guard_enabled = true

[continuity]
log_sampling = "all"
"#,
    )
    .expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("analyze").arg(repo.path()).assert().code(0);

    let progress_path = repo.path().join(".harness/progress.md");
    let content = fs::read_to_string(&progress_path).expect("progress log should be readable");
    assert!(content.contains("feature: analyze"));
    assert!(content.contains("action: complete"));
}

#[test]
fn suggest_logs_milestones_even_when_progress_sampling_none() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[continuity]
log_sampling = "none"
"#,
    )
    .expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("suggest").arg(repo.path()).assert().code(0);

    let progress_path = repo.path().join(".harness/progress.md");
    let content = fs::read_to_string(&progress_path).expect("progress log should be readable");
    assert!(content.contains("feature: suggest"));
    assert!(content.contains("action: complete"));
}

#[test]
fn analyze_fails_on_invalid_config_weights() {
    let repo = TempDir::new().expect("temp dir should be created");
    fs::create_dir_all(repo.path().join(".git")).expect(".git directory should create");
    fs::write(
        repo.path().join("harness.toml"),
        r#"
[project]
name = "sample"
profile = "general"

[metrics.weights]
context = 0.9
tools = 0.9
continuity = 0.1
verification = 0.1
repository_quality = 0.1
"#,
    )
    .expect("config should write");

    let mut cmd = Command::cargo_bin("harness").expect("binary should compile");
    cmd.arg("analyze")
        .arg(repo.path())
        .assert()
        .code(3)
        .stderr(predicate::str::contains("metrics.weights must sum to 1.0"));
}
