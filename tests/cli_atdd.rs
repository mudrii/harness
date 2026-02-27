#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

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
