pub mod docs;
pub mod filesystem;
pub mod git_meta;
pub mod tools;

use crate::types::config::HarnessConfig;
use docs::DocSignals;
use filesystem::{file_exists, list_files, read_to_string_if_exists};
use std::path::{Path, PathBuf};
use tools::ToolSignals;

#[derive(Debug, Clone, Default)]
pub struct ContinuitySignals {
    pub has_initializer_prompt: bool,
    pub has_coding_prompt: bool,
    pub has_progress_file: bool,
    pub has_feature_state_file: bool,
    pub has_progress_summary: bool,
}

#[derive(Debug, Clone, Default)]
pub struct QualitySignals {
    pub has_ci_workflow: bool,
    pub has_tests: bool,
    pub has_lint_config: bool,
}

#[derive(Debug, Clone)]
pub struct RepoModel {
    #[allow(dead_code)]
    pub root: PathBuf,
    pub file_count: usize,
    pub docs: DocSignals,
    pub tools: ToolSignals,
    pub continuity: ContinuitySignals,
    pub quality: QualitySignals,
}

pub fn discover(root: &Path, config: Option<&HarnessConfig>) -> RepoModel {
    let files = list_files(root);
    let docs = docs::detect_docs(root);
    let tools = tools::detect_tools(config);
    let continuity = detect_continuity(root, config);
    let quality = detect_quality(root, &files);

    RepoModel {
        root: root.to_path_buf(),
        file_count: files.len(),
        docs,
        tools,
        continuity,
        quality,
    }
}

fn detect_continuity(root: &Path, config: Option<&HarnessConfig>) -> ContinuitySignals {
    let initializer = config
        .and_then(|cfg| cfg.continuity.as_ref())
        .and_then(|continuity| continuity.initializer.as_ref())
        .map(|path| root.join(path))
        .unwrap_or_else(|| root.join(".harness/initializer.prompt.md"));

    let coding_prompt = config
        .and_then(|cfg| cfg.continuity.as_ref())
        .and_then(|continuity| continuity.coding_prompt.as_ref())
        .map(|path| root.join(path))
        .unwrap_or_else(|| root.join(".harness/coding.prompt.md"));

    let progress_file = config
        .and_then(|cfg| cfg.continuity.as_ref())
        .and_then(|continuity| continuity.progress_file.as_ref())
        .map(|path| root.join(path))
        .unwrap_or_else(|| root.join(".harness/progress.md"));

    let feature_state = config
        .and_then(|cfg| cfg.continuity.as_ref())
        .and_then(|continuity| continuity.feature_state_file.as_ref())
        .map(|path| root.join(path))
        .unwrap_or_else(|| root.join(".harness/feature_list.json"));

    let progress_content = read_to_string_if_exists(&progress_file).unwrap_or_default();

    ContinuitySignals {
        has_initializer_prompt: file_exists(&initializer),
        has_coding_prompt: file_exists(&coding_prompt),
        has_progress_file: file_exists(&progress_file),
        has_feature_state_file: file_exists(&feature_state),
        has_progress_summary: progress_content.to_lowercase().contains("summary"),
    }
}

fn detect_quality(root: &Path, files: &[PathBuf]) -> QualitySignals {
    let has_ci_workflow = file_exists(&root.join(".github/workflows"))
        && files
            .iter()
            .any(|path| path.to_string_lossy().contains(".github/workflows/"));

    let has_tests = files.iter().any(|path| {
        let name = path
            .file_name()
            .and_then(|file| file.to_str())
            .unwrap_or_default();
        name.ends_with("_test.rs")
            || name.ends_with("_spec.rs")
            || path.to_string_lossy().contains("/tests/")
    });

    let has_lint_config = file_exists(&root.join("rustfmt.toml"))
        || file_exists(&root.join(".clippy.toml"))
        || file_exists(&root.join("clippy.toml"));

    QualitySignals {
        has_ci_workflow,
        has_tests,
        has_lint_config,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn discover_collects_quality_and_continuity_signals() {
        let dir = TempDir::new().expect("temp dir should be created");
        fs::create_dir_all(dir.path().join(".harness")).expect("harness dir should be created");
        fs::create_dir_all(dir.path().join(".github/workflows"))
            .expect("workflow dir should be created");
        fs::create_dir_all(dir.path().join("tests")).expect("tests dir should be created");
        fs::write(
            dir.path().join(".harness/initializer.prompt.md"),
            "initializer",
        )
        .expect("initializer should write");
        fs::write(dir.path().join(".harness/coding.prompt.md"), "coding").expect("coding write");
        fs::write(
            dir.path().join(".harness/progress.md"),
            "summary: checkpoint",
        )
        .expect("progress write");
        fs::write(
            dir.path().join(".github/workflows/ci.yml"),
            "name: ci\non: [push]",
        )
        .expect("ci workflow should write");
        fs::write(dir.path().join("tests/sample_test.rs"), "#[test] fn t() {}")
            .expect("test file should write");
        fs::write(dir.path().join("rustfmt.toml"), "edition = \"2021\"")
            .expect("lint config should write");

        let model = discover(dir.path(), None);
        assert!(model.continuity.has_initializer_prompt);
        assert!(model.quality.has_ci_workflow);
        assert!(model.quality.has_tests);
        assert!(model.quality.has_lint_config);
    }
}
