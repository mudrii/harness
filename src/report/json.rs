use crate::types::report::HarnessReport;

pub fn to_json(report: &HarnessReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::report::{Effort, Impact, Risk};
    use crate::types::report::{HarnessReport, Recommendation};
    use crate::types::scoring::ScoreCard;

    #[test]
    fn json_report_contains_overall_score() {
        let report = HarnessReport {
            overall_score: 0.8,
            category_scores: ScoreCard::new(0.8, 0.7, 0.6, 0.9, 0.7),
            findings: vec![],
            recommendations: vec![Recommendation::new(
                "id",
                "title",
                "summary",
                Impact::High,
                Effort::S,
                Risk::Safe,
                0.9,
            )],
        };

        let rendered = to_json(&report).expect("json should serialize");
        assert!(rendered.contains("\"overall_score\": 0.8"));
    }
}
