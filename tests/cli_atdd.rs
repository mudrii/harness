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
