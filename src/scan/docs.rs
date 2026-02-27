use super::filesystem::{file_exists, read_to_string_if_exists};
use super::git_meta::doc_age_days;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct DocSignals {
    pub has_agents_md: bool,
    pub agents_has_section_header: bool,
    pub has_context_index: bool,
    pub has_architecture_doc: bool,
    pub readme_links_architecture: bool,
    pub docs_age_days: Option<i64>,
}

pub fn detect_docs(root: &Path) -> DocSignals {
    let agents_path = root.join("AGENTS.md");
    let context_index_path = root.join("docs/context/INDEX.md");
    let architecture_path = root.join("ARCHITECTURE.md");
    let docs_architecture_path = root.join("docs/ARCHITECTURE.md");
    let readme_path = root.join("README.md");

    let agents_content = read_to_string_if_exists(&agents_path).unwrap_or_default();
    let readme_content = read_to_string_if_exists(&readme_path).unwrap_or_default();

    let has_architecture_doc =
        file_exists(&architecture_path) || file_exists(&docs_architecture_path);
    let docs_age_days = doc_age_days(
        root,
        &[
            "AGENTS.md",
            "docs/context/INDEX.md",
            "ARCHITECTURE.md",
            "docs/ARCHITECTURE.md",
            "README.md",
        ],
    );

    DocSignals {
        has_agents_md: file_exists(&agents_path),
        agents_has_section_header: agents_content
            .lines()
            .any(|line| line.trim_start().starts_with('#')),
        has_context_index: file_exists(&context_index_path),
        has_architecture_doc,
        readme_links_architecture: readme_content.to_lowercase().contains("architecture"),
        docs_age_days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn detect_docs_picks_up_core_files() {
        let dir = TempDir::new().expect("temp dir should be created");
        fs::create_dir_all(dir.path().join("docs/context")).expect("docs context should create");
        fs::write(dir.path().join("AGENTS.md"), "# Agents\nmap").expect("agents file should write");
        fs::write(
            dir.path().join("README.md"),
            "See architecture in ARCHITECTURE.md",
        )
        .expect("readme should write");
        fs::write(dir.path().join("ARCHITECTURE.md"), "# Architecture").expect("arch should write");
        fs::write(dir.path().join("docs/context/INDEX.md"), "index").expect("index should write");

        let signals = detect_docs(dir.path());
        assert!(signals.has_agents_md);
        assert!(signals.agents_has_section_header);
        assert!(signals.has_context_index);
        assert!(signals.has_architecture_doc);
        assert!(signals.readme_links_architecture);
    }
}
