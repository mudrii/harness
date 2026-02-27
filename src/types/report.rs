#[derive(Debug, Clone)]
pub struct Recommendation {
    pub id: String,
    pub title: String,
    pub confidence: f32,
    pub risk: String,
}
