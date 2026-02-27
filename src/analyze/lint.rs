use crate::scan::RepoModel;
use crate::types::config::HarnessConfig;
use crate::types::report::Finding;

pub fn lint_findings(model: &RepoModel, config: Option<&HarnessConfig>) -> Vec<Finding> {
    super::analyze(model, config).findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{docs::DocSignals, tools::ToolSignals, ContinuitySignals, QualitySignals};
    use std::path::PathBuf;

    fn base_model() -> RepoModel {
        RepoModel {
            root: PathBuf::from("."),
            file_count: 100,
            docs: DocSignals {
                has_agents_md: true,
                agents_has_section_header: true,
                has_context_index: true,
                has_architecture_doc: true,
                readme_links_architecture: true,
                docs_age_days: Some(1),
            },
            tools: ToolSignals::default(),
            continuity: ContinuitySignals::default(),
            quality: QualitySignals::default(),
        }
    }

    #[test]
    fn test_lint_includes_missing_context_findings() {
        let mut model = base_model();
        model.docs.has_agents_md = false;
        model.docs.has_context_index = false;

        let findings = lint_findings(&model, None);
        assert!(findings
            .iter()
            .any(|finding| finding.id == "context.missing_agents"));
        assert!(findings
            .iter()
            .any(|finding| finding.id == "context.missing_index"));
    }

    #[test]
    fn test_lint_reports_blocking_for_destructive_tools() {
        let mut model = base_model();
        model.tools.unrestricted_destructive = 1;

        let findings = lint_findings(&model, None);
        assert!(findings
            .iter()
            .any(|finding| finding.id == "tools.destructive_exposed" && finding.blocking));
    }
}
