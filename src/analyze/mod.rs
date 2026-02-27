pub mod context;
pub mod continuity;
pub mod lint;
pub mod quality;
pub mod tools;
pub mod verification;

use crate::scan::RepoModel;
use crate::types::config::HarnessConfig;
use crate::types::report::{Effort, Finding, HarnessReport, Impact, Recommendation, Risk};
use crate::types::scoring::ScoreCard;

pub fn analyze(model: &RepoModel, config: Option<&HarnessConfig>) -> HarnessReport {
    let context = context::context_score(model);
    let tools = tools::tools_score(model);
    let continuity = continuity::continuity_score(model);
    let verification = verification::verification_score(config);
    let repository_quality = quality::repository_quality_score(model);

    let weights = config
        .map(|cfg| cfg.weights())
        .unwrap_or_else(HarnessConfig::default_weights);
    let category_scores =
        ScoreCard::new(context, tools, continuity, verification, repository_quality)
            .finalize(&weights);

    let mut findings = Vec::new();
    if !model.docs.has_agents_md {
        findings.push(Finding {
            id: "context.missing_agents".to_string(),
            title: "Missing AGENTS.md".to_string(),
            body: "Repository is missing AGENTS.md; agent legibility is reduced.".to_string(),
            blocking: false,
            file: Some("AGENTS.md".to_string()),
        });
    }
    if !model.docs.has_context_index {
        findings.push(Finding {
            id: "context.missing_index".to_string(),
            title: "Missing docs context index".to_string(),
            body: "docs/context/INDEX.md is missing, reducing navigability for agents.".to_string(),
            blocking: false,
            file: Some("docs/context/INDEX.md".to_string()),
        });
    }
    if model.tools.unrestricted_destructive > 0 {
        findings.push(Finding {
            id: "tools.destructive_exposed".to_string(),
            title: "Potentially destructive tools exposed".to_string(),
            body: "Detected unrestricted destructive commands in tool inventory.".to_string(),
            blocking: true,
            file: Some("harness.toml".to_string()),
        });
    }
    if let Some(deprecated) = config
        .and_then(|cfg| cfg.tools.as_ref())
        .and_then(|tools| tools.deprecated.as_ref())
    {
        if !deprecated.observe.is_empty() {
            findings.push(Finding {
                id: "tools.observe".to_string(),
                title: "Observed tools scheduled for deprecation".to_string(),
                body: format!(
                    "Observed tools are still allowed but tracked: {}.",
                    deprecated.observe.join(", ")
                ),
                blocking: false,
                file: Some("harness.toml".to_string()),
            });
        }
        if !deprecated.deprecated.is_empty() {
            findings.push(Finding {
                id: "tools.deprecated".to_string(),
                title: "Deprecated tools still enabled".to_string(),
                body: format!(
                    "Deprecated tools should be migrated off active workflows: {}.",
                    deprecated.deprecated.join(", ")
                ),
                blocking: true,
                file: Some("harness.toml".to_string()),
            });
        }
        if !deprecated.disabled.is_empty() {
            findings.push(Finding {
                id: "tools.disabled".to_string(),
                title: "Disabled tools are configured".to_string(),
                body: format!(
                    "Disabled tools are forbidden on apply and must not be used: {}.",
                    deprecated.disabled.join(", ")
                ),
                blocking: true,
                file: Some("harness.toml".to_string()),
            });
        }
    }
    if config.is_some() && verification < 0.5 {
        findings.push(Finding {
            id: "verification.incomplete".to_string(),
            title: "Verification policy incomplete".to_string(),
            body: "Verification requirements are incomplete or missing pre-completion checks."
                .to_string(),
            blocking: true,
            file: Some("harness.toml".to_string()),
        });
    } else if config.is_none() {
        findings.push(Finding {
            id: "verification.missing_config".to_string(),
            title: "Verification policy unavailable".to_string(),
            body: "Verification checks cannot be evaluated because harness.toml is missing."
                .to_string(),
            blocking: false,
            file: Some("harness.toml".to_string()),
        });
    }

    let mut recommendations = vec![
        Recommendation::new(
            "rec.context.index",
            "Add Context Index",
            "Create docs/context/INDEX.md and link it from AGENTS.md.",
            Impact::High,
            Effort::S,
            Risk::Safe,
            0.92,
        ),
        Recommendation::new(
            "rec.verification.gate",
            "Enable Verification Gate",
            "Set pre_completion_required and provide required verification commands.",
            Impact::High,
            Effort::S,
            Risk::Medium,
            0.88,
        ),
        Recommendation::new(
            "rec.tools.prune",
            "Prune Redundant Tools",
            "Reduce overlap in grep/find-style tool clusters and remove risky commands.",
            Impact::Medium,
            Effort::M,
            Risk::Medium,
            0.84,
        ),
    ];

    let mut report = HarnessReport {
        overall_score: category_scores.overall,
        category_scores,
        findings,
        recommendations: Vec::new(),
    };

    if model.file_count < 20 {
        recommendations.push(Recommendation::new(
            "rec.repo.scale",
            "Document Repository Scale",
            "Add lightweight architecture notes to support agent understanding in small repos.",
            Impact::Low,
            Effort::Xs,
            Risk::Safe,
            0.60,
        ));
    }

    report.recommendations = recommendations;
    report.sort_recommendations();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{docs::DocSignals, tools::ToolSignals};
    use crate::scan::{ContinuitySignals, QualitySignals};
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
    fn analyze_generates_warning_when_config_is_missing() {
        let model = base_model();

        let report = analyze(&model, None);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "verification.missing_config" && !finding.blocking));
        assert!((0.0..=1.0).contains(&report.overall_score));
    }

    #[test]
    fn analyze_emits_deprecation_lifecycle_findings() {
        let model = base_model();
        let config: HarnessConfig = toml::from_str(
            r#"
[project]
name = "sample"
profile = "general"

[tools.deprecated]
observe = ["find"]
deprecated = ["grep"]
disabled = ["apply_patch"]
"#,
        )
        .expect("config should parse");

        let report = analyze(&model, Some(&config));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "tools.observe" && !finding.blocking));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "tools.deprecated" && finding.blocking));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "tools.disabled" && finding.blocking));
    }
}
