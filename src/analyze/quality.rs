use crate::scan::RepoModel;

pub fn repository_quality_score(model: &RepoModel) -> f32 {
    let mut score: f32 = 0.0;
    if model.quality.has_ci_workflow {
        score += 0.40;
    }
    if model.quality.has_tests {
        score += 0.30;
    }
    if model.quality.has_lint_config {
        score += 0.30;
    }
    score.clamp(0.0, 1.0)
}
