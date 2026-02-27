use crate::scan::RepoModel;

pub fn continuity_score(model: &RepoModel) -> f32 {
    let mut score: f32 = 0.0;
    if model.continuity.has_initializer_prompt && model.continuity.has_coding_prompt {
        score += 0.40;
    }
    if model.continuity.has_progress_file {
        score += 0.25;
    }
    if model.continuity.has_feature_state_file {
        score += 0.20;
    }
    if model.continuity.has_progress_summary {
        score += 0.15;
    }
    score.clamp(0.0, 1.0)
}
