use crate::analyze;
use crate::cli::{ApplyCommand, ApplyMode};
use crate::config;
use crate::error::{HarnessError, Result};
use crate::guardrails;
use crate::scan;
use crate::types::report::Risk;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeAction {
    Create,
    Modify,
}

impl ChangeAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Modify => "modify",
        }
    }
}

#[derive(Debug, Clone)]
struct PlannedChange {
    path: PathBuf,
    action: ChangeAction,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ApplyPlanFile {
    version: String,
    recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RollbackManifest {
    timestamp: String,
    harness_version: String,
    files: Vec<RollbackFile>,
}

#[derive(Debug, Serialize)]
struct RollbackFile {
    path: String,
    action: String,
    sha256: Option<String>,
}

pub fn execute_apply(cmd: &ApplyCommand) -> Result<()> {
    let loaded = config::load_config(&cmd.path)?;

    if !cmd.allow_dirty {
        check_clean_tree(&cmd.path, loaded.as_ref())?;
    }

    let recommendation_ids = resolve_plan(&cmd.path, cmd, loaded.as_ref())?;
    let changes = build_changes(&cmd.path, &recommendation_ids)?;
    guardrails::validate_with_config(&[], changes.len() as u32, loaded.as_ref())?;

    print_scope_summary(&cmd.path, &changes);
    if changes.is_empty() {
        println!("no-op: no changes required");
        return Ok(());
    }

    if matches!(cmd.apply_mode, ApplyMode::Preview) {
        println!("preview: no files were written");
        return Ok(());
    }

    if !cmd.yes && !confirm_apply()? {
        println!("apply cancelled");
        return Ok(());
    }

    let rollback_path = create_rollback_manifest(&cmd.path, &changes)?;
    println!("rollback manifest: {}", rollback_path.display());
    apply_changes(&changes)?;
    println!("apply complete: wrote {} file(s)", changes.len());
    Ok(())
}

pub fn check_clean_tree(
    root: &Path,
    config: Option<&crate::types::config::HarnessConfig>,
) -> Result<()> {
    let command_line = "git status --porcelain";
    guardrails::validate_with_config(&[command_line], 0, config)?;

    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .map_err(HarnessError::Io)?;

    if !output.status.success() {
        return Err(HarnessError::NotGitRepo(root.display().to_string()));
    }

    if String::from_utf8_lossy(&output.stdout).trim().is_empty() {
        Ok(())
    } else {
        Err(HarnessError::ConfigParse(
            "working tree is dirty; use --allow-dirty to override".to_string(),
        ))
    }
}

pub fn validate_plan_path(path: &str) -> Result<()> {
    let parsed = Path::new(path);
    if parsed.is_absolute() {
        return Err(HarnessError::ConfigParse(format!(
            "absolute plan path rejected: {path}"
        )));
    }
    if parsed
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(HarnessError::ConfigParse(format!(
            "path traversal rejected: {path}"
        )));
    }
    Ok(())
}

fn resolve_plan(
    root: &Path,
    cmd: &ApplyCommand,
    preloaded_config: Option<&crate::types::config::HarnessConfig>,
) -> Result<Vec<String>> {
    if cmd.plan_all {
        let model = scan::discover(root, preloaded_config);
        let report = analyze::analyze(&model, preloaded_config);
        let ids = report
            .recommendations
            .into_iter()
            .filter(|recommendation| matches!(recommendation.risk, Risk::Safe))
            .map(|recommendation| recommendation.id)
            .collect::<Vec<_>>();
        return Ok(ids);
    }

    let plan_file = cmd
        .plan_file
        .as_ref()
        .ok_or_else(|| HarnessError::ConfigParse("missing --plan-file value".to_string()))?;
    validate_plan_path(plan_file)?;
    let full_path = root.join(plan_file);
    if !full_path.exists() {
        return Err(HarnessError::PathNotFound(full_path.display().to_string()));
    }

    let raw = fs::read_to_string(&full_path).map_err(HarnessError::Io)?;
    let parsed: ApplyPlanFile = serde_json::from_str(&raw)?;
    if parsed.version != env!("CARGO_PKG_VERSION") {
        return Err(HarnessError::ConfigParse(format!(
            "plan version mismatch: expected {}, found {}",
            env!("CARGO_PKG_VERSION"),
            parsed.version
        )));
    }

    Ok(parsed.recommendations)
}

fn build_changes(root: &Path, recommendation_ids: &[String]) -> Result<Vec<PlannedChange>> {
    let mut changes = Vec::new();
    let mut seen = BTreeSet::new();
    for id in recommendation_ids {
        if !seen.insert(id.clone()) {
            continue;
        }

        match id.as_str() {
            "rec.context.index" => {
                maybe_add_context_index_change(root, &mut changes)?;
                maybe_add_agents_reference_change(root, &mut changes)?;
            }
            "rec.repo.scale" => {
                maybe_add_architecture_doc_change(root, &mut changes)?;
            }
            _ => {}
        }
    }
    Ok(changes)
}

fn maybe_add_context_index_change(root: &Path, changes: &mut Vec<PlannedChange>) -> Result<()> {
    let path = root.join("docs/context/INDEX.md");
    if !path.exists() {
        changes.push(PlannedChange {
            path,
            action: ChangeAction::Create,
            content: "# Generated by harness\n# Context Index\n\n- AGENTS.md\n".to_string(),
        });
    }
    Ok(())
}

fn maybe_add_agents_reference_change(root: &Path, changes: &mut Vec<PlannedChange>) -> Result<()> {
    let path = root.join("AGENTS.md");
    let link_line = "- Context index: docs/context/INDEX.md";
    if path.exists() {
        let existing = fs::read_to_string(&path).map_err(HarnessError::Io)?;
        if !existing.contains("docs/context/INDEX.md") {
            let mut updated = existing;
            if !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(link_line);
            updated.push('\n');
            changes.push(PlannedChange {
                path,
                action: ChangeAction::Modify,
                content: updated,
            });
        }
        return Ok(());
    }

    changes.push(PlannedChange {
        path,
        action: ChangeAction::Create,
        content: format!("# Generated by harness\n# Agents\n\n{link_line}\n"),
    });
    Ok(())
}

fn maybe_add_architecture_doc_change(root: &Path, changes: &mut Vec<PlannedChange>) -> Result<()> {
    let path = root.join("ARCHITECTURE.md");
    if !path.exists() {
        changes.push(PlannedChange {
            path,
            action: ChangeAction::Create,
            content: "# Generated by harness\n# Architecture\n\n## Overview\n\nTBD.\n".to_string(),
        });
    }
    Ok(())
}

fn print_scope_summary(root: &Path, changes: &[PlannedChange]) {
    let create_count = changes
        .iter()
        .filter(|change| change.action == ChangeAction::Create)
        .count();
    let modify_count = changes
        .iter()
        .filter(|change| change.action == ChangeAction::Modify)
        .count();

    println!("scope: create={create_count} modify={modify_count} delete=0");
    for change in changes {
        let display_path = change
            .path
            .strip_prefix(root)
            .unwrap_or(change.path.as_path())
            .display();
        println!("{}: {}", change.action.as_str(), display_path);
    }
}

fn confirm_apply() -> Result<bool> {
    print!("Apply these changes? [y/N]: ");
    io::stdout().flush().map_err(HarnessError::Io)?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(HarnessError::Io)?;
    let normalized = input.trim().to_ascii_lowercase();
    Ok(normalized == "y" || normalized == "yes")
}

fn create_rollback_manifest(root: &Path, changes: &[PlannedChange]) -> Result<PathBuf> {
    let timestamp = Utc::now();
    let timestamp_string = timestamp.to_rfc3339();
    let file_stamp = timestamp.format("%Y%m%dT%H%M%SZ").to_string();
    let rollback_dir = root.join(".harness/rollback");
    fs::create_dir_all(&rollback_dir).map_err(HarnessError::Io)?;

    let mut files = Vec::new();
    for change in changes {
        let relative = change
            .path
            .strip_prefix(root)
            .unwrap_or(change.path.as_path())
            .to_string_lossy()
            .to_string();
        let sha256 = if change.path.exists() {
            let bytes = fs::read(&change.path).map_err(HarnessError::Io)?;
            Some(sha256_hex(&bytes))
        } else {
            None
        };

        files.push(RollbackFile {
            path: relative,
            action: change.action.as_str().to_string(),
            sha256,
        });
    }

    let manifest = RollbackManifest {
        timestamp: timestamp_string,
        harness_version: env!("CARGO_PKG_VERSION").to_string(),
        files,
    };

    let out_path = rollback_dir.join(format!("{file_stamp}.json"));
    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&out_path, json).map_err(HarnessError::Io)?;
    Ok(out_path)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn apply_changes(changes: &[PlannedChange]) -> Result<()> {
    for change in changes {
        if let Some(parent) = change.path.parent() {
            fs::create_dir_all(parent).map_err(HarnessError::Io)?;
        }
        fs::write(&change.path, &change.content).map_err(HarnessError::Io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_git_repo(root: &Path) {
        let output = Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()
            .expect("git init should run");
        assert!(output.status.success(), "git init should succeed");
    }

    #[test]
    fn test_clean_tree_check_passes_on_clean_repo() {
        let tmp = TempDir::new().expect("temp dir should create");
        init_git_repo(tmp.path());
        assert!(check_clean_tree(tmp.path(), None).is_ok());
    }

    #[test]
    fn test_clean_tree_check_fails_on_dirty_repo() {
        let tmp = TempDir::new().expect("temp dir should create");
        init_git_repo(tmp.path());
        fs::write(tmp.path().join("dirty.txt"), "change").expect("dirty file should write");
        assert!(check_clean_tree(tmp.path(), None).is_err());
    }

    #[test]
    fn test_plan_file_rejects_path_traversal() {
        assert!(validate_plan_path("../../etc/passwd").is_err());
        assert!(validate_plan_path("plans/good.json").is_ok());
    }

    #[test]
    fn test_build_changes_for_context_index_recommendation() {
        let tmp = TempDir::new().expect("temp dir should create");
        fs::write(tmp.path().join("AGENTS.md"), "# Agents\n").expect("agents file should write");

        let changes = build_changes(tmp.path(), &[String::from("rec.context.index")])
            .expect("build changes should succeed");

        assert!(changes.iter().any(|change| {
            change.action == ChangeAction::Create
                && change.path == tmp.path().join("docs/context/INDEX.md")
        }));
        assert!(changes.iter().any(|change| {
            change.action == ChangeAction::Modify && change.path == tmp.path().join("AGENTS.md")
        }));
    }
}
