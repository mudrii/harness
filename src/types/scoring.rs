pub type Score = f32;

#[derive(Debug, Clone)]
pub struct ScoreCard {
    pub context: Score,
    pub tools: Score,
    pub continuity: Score,
    pub verification: Score,
    pub repository_quality: Score,
    pub overall: Score,
}
