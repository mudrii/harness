use serde::Serialize;

pub type Score = f32;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CategoryScoreBuilder {
    pub base: Score,
    pub bonuses: Score,
    pub penalties: Score,
}

#[allow(dead_code)]
impl CategoryScoreBuilder {
    pub fn new(base: Score) -> Self {
        Self {
            base,
            bonuses: 0.0,
            penalties: 0.0,
        }
    }

    pub fn add_bonus(&mut self, value: Score) {
        self.bonuses += value;
    }

    pub fn add_penalty(&mut self, value: Score) {
        self.penalties += value;
    }

    pub fn build(&self, max_penalty_per_bucket: Score) -> Score {
        let capped_penalty = self.penalties.min(max_penalty_per_bucket);
        (self.base + self.bonuses - capped_penalty).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreCard {
    pub context: Score,
    pub tools: Score,
    pub continuity: Score,
    pub verification: Score,
    pub repository_quality: Score,
    pub overall: Score,
}

impl ScoreCard {
    pub fn new(
        context: Score,
        tools: Score,
        continuity: Score,
        verification: Score,
        repository_quality: Score,
    ) -> Self {
        Self {
            context,
            tools,
            continuity,
            verification,
            repository_quality,
            overall: 0.0,
        }
    }

    pub fn clamped(&self) -> Self {
        Self {
            context: self.context.clamp(0.0, 1.0),
            tools: self.tools.clamp(0.0, 1.0),
            continuity: self.continuity.clamp(0.0, 1.0),
            verification: self.verification.clamp(0.0, 1.0),
            repository_quality: self.repository_quality.clamp(0.0, 1.0),
            overall: self.overall.clamp(0.0, 1.0),
        }
    }

    pub fn weighted_overall(&self, weights: &[Score; 5]) -> Score {
        let clamped = self.clamped();
        [
            clamped.context,
            clamped.tools,
            clamped.continuity,
            clamped.verification,
            clamped.repository_quality,
        ]
        .iter()
        .zip(weights.iter())
        .map(|(score, weight)| score * weight)
        .sum()
    }

    pub fn finalize(mut self, weights: &[Score; 5]) -> Self {
        self = self.clamped();
        self.overall = self.weighted_overall(weights);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_valid_scores() {
        let card = ScoreCard::new(0.5, 0.8, 0.3, 0.9, 0.7);
        let clamped = card.clamped();
        assert!((clamped.context - 0.5).abs() < 0.001);
    }

    #[test]
    fn clamp_caps_negative_and_overflow() {
        let card = ScoreCard {
            context: -0.2,
            tools: 1.5,
            continuity: 0.5,
            verification: 0.5,
            repository_quality: 0.5,
            overall: 1.2,
        };
        let clamped = card.clamped();
        assert!((clamped.context - 0.0).abs() < 0.001);
        assert!((clamped.tools - 1.0).abs() < 0.001);
        assert!((clamped.overall - 1.0).abs() < 0.001);
    }

    #[test]
    fn bucket_penalty_limit_enforced() {
        let mut builder = CategoryScoreBuilder::new(1.0);
        builder.add_penalty(0.60);
        let score = builder.build(0.40);
        assert!((score - 0.60).abs() < 0.001);
    }

    #[test]
    fn weighted_overall_sums_correctly() {
        let weights = [0.30, 0.25, 0.20, 0.15, 0.10];
        let card = ScoreCard::new(1.0, 1.0, 1.0, 1.0, 1.0);
        let overall = card.weighted_overall(&weights);
        assert!((overall - 1.0).abs() < 0.001);
    }

    #[test]
    fn weighted_overall_with_zeros() {
        let weights = [0.30, 0.25, 0.20, 0.15, 0.10];
        let card = ScoreCard::new(0.0, 0.0, 0.0, 0.0, 0.0);
        let overall = card.weighted_overall(&weights);
        assert!((overall - 0.0).abs() < 0.001);
    }
}
