use crate::types::report::HarnessReport;

pub fn to_markdown(report: &HarnessReport) -> String {
    let mut output = String::new();
    output.push_str("# Harness Report\n\n");
    output.push_str(&format!("Overall score: {:.3}\n\n", report.overall_score));
    output.push_str("## Category Scores\n\n");
    output.push_str(&format!(
        "- context: {:.3}\n- tools: {:.3}\n- continuity: {:.3}\n- verification: {:.3}\n- repository_quality: {:.3}\n\n",
        report.category_scores.context,
        report.category_scores.tools,
        report.category_scores.continuity,
        report.category_scores.verification,
        report.category_scores.repository_quality
    ));

    output.push_str("## Findings\n\n");
    if report.findings.is_empty() {
        output.push_str("- none\n\n");
    } else {
        for finding in &report.findings {
            output.push_str(&format!(
                "- [{}] {}: {}\n",
                if finding.blocking {
                    "blocking"
                } else {
                    "warning"
                },
                finding.title,
                finding.body
            ));
        }
        output.push('\n');
    }

    output.push_str("## Recommendations\n\n");
    if report.recommendations.is_empty() {
        output.push_str("- none\n");
    } else {
        for recommendation in &report.recommendations {
            output.push_str(&format!(
                "- {} ({:?}/{:?}, confidence {:.2}): {}\n",
                recommendation.title,
                recommendation.impact,
                recommendation.effort,
                recommendation.confidence,
                recommendation.summary
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::report::{Effort, Impact, Recommendation, Risk};
    use crate::types::scoring::ScoreCard;

    #[test]
    fn markdown_report_contains_sections() {
        let report = HarnessReport {
            overall_score: 0.5,
            category_scores: ScoreCard::new(0.1, 0.2, 0.3, 0.4, 0.5),
            findings: vec![],
            recommendations: vec![Recommendation::new(
                "id",
                "Title",
                "Summary",
                Impact::Medium,
                Effort::M,
                Risk::Medium,
                0.7,
            )],
        };

        let rendered = to_markdown(&report);
        assert!(rendered.contains("# Harness Report"));
        assert!(rendered.contains("## Category Scores"));
        assert!(rendered.contains("## Recommendations"));
    }
}
