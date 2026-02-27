// Integration tests for the harness CLI.
//
// These tests use assert_cmd to invoke the binary and verify
// exit codes, stdout/stderr output, and side effects.
//
// Prerequisites: tempfile, assert_cmd, predicates (dev-dependencies).

use assert_cmd::Command;
use predicates::prelude::*;

/// Helper to build a Command for the harness binary.
fn harness() -> Command {
    Command::cargo_bin("harness").expect("binary should exist")
}

#[test]
fn cli_version_flag() {
    harness()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("harness"));
}

#[test]
fn cli_help_flag() {
    harness()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI agent harness"));
}

#[test]
fn analyze_requires_path() {
    harness()
        .arg("analyze")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn init_requires_path() {
    harness()
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn apply_requires_plan_selector() {
    // apply needs either --plan-file or --plan-all
    harness()
        .args(["apply", "/tmp/test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn apply_rejects_both_selectors() {
    // --plan-file and --plan-all are mutually exclusive
    harness()
        .args(["apply", "/tmp/test", "--plan-file", "foo.json", "--plan-all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// Placeholder for Task 14 (git gate integration test):
// #[test]
// fn analyze_non_git_dir_exits_with_code_3() {
//     let tmp = tempfile::TempDir::new().unwrap();
//     harness()
//         .args(["analyze", tmp.path().to_str().unwrap()])
//         .assert()
//         .code(3)
//         .stderr(predicate::str::contains("not a git repository"));
// }
