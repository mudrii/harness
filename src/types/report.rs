use crate::types::scoring::ScoreCard;
use serde::Serialize;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Impact {
    Low,
    Medium,
    High,
}

impl Impact {
    fn priority(self) -> u8 {
        match self {
            Self::High => 3,
            Self::Medium => 2,
            Self::Low => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Xs,
    S,
    M,
    L,
}

impl Effort {
    fn rank(self) -> u8 {
        match self {
            Self::Xs => 1,
            Self::S => 2,
            Self::M => 3,
            Self::L => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Safe,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub body: String,
    pub blocking: bool,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub impact: Impact,
    pub effort: Effort,
    pub risk: Risk,
    pub confidence: f32,
}

impl Recommendation {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        impact: Impact,
        effort: Effort,
        risk: Risk,
        confidence: f32,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: summary.into(),
            impact,
            effort,
            risk,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HarnessReport {
    pub overall_score: f32,
    pub category_scores: ScoreCard,
    pub findings: Vec<Finding>,
    pub recommendations: Vec<Recommendation>,
}

impl HarnessReport {
    pub fn sort_recommendations(&mut self) {
        self.recommendations.sort_by(|a, b| {
            b.impact
                .priority()
                .cmp(&a.impact.priority())
                .then_with(|| a.effort.rank().cmp(&b.effort.rank()))
                .then_with(|| alphabetical_cmp(&a.id, &b.id))
        });
    }
}

fn alphabetical_cmp(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::scoring::ScoreCard;

    #[test]
    fn recommendation_confidence_is_clamped() {
        let recommendation = Recommendation::new(
            "id-1",
            "Title",
            "Summary",
            Impact::Medium,
            Effort::S,
            Risk::Medium,
            1.5,
        );

        assert!((recommendation.confidence - 1.0).abs() < 0.001);
    }

    #[test]
    fn recommendation_sorting_uses_impact_effort_and_id() {
        let mut report = HarnessReport {
            overall_score: 0.0,
            category_scores: ScoreCard::new(0.0, 0.0, 0.0, 0.0, 0.0),
            findings: vec![],
            recommendations: vec![
                Recommendation::new(
                    "b",
                    "Low effort medium impact",
                    "x",
                    Impact::Medium,
                    Effort::Xs,
                    Risk::Safe,
                    0.8,
                ),
                Recommendation::new(
                    "a",
                    "High impact",
                    "x",
                    Impact::High,
                    Effort::M,
                    Risk::High,
                    0.9,
                ),
                Recommendation::new(
                    "c",
                    "Medium impact higher effort",
                    "x",
                    Impact::Medium,
                    Effort::L,
                    Risk::Medium,
                    0.7,
                ),
            ],
        };

        report.sort_recommendations();

        let ids: Vec<String> = report
            .recommendations
            .iter()
            .map(|recommendation| recommendation.id.clone())
            .collect();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }
}
