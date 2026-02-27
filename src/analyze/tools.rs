use crate::scan::RepoModel;

pub fn tools_score(model: &RepoModel) -> f32 {
    let mut score: f32 = 1.0;

    if model.tools.tool_names.len() > 12 {
        score -= 0.10;
    }
    score -= model.tools.risky_overlap_clusters as f32 * 0.05;
    score -= model.tools.unrestricted_destructive as f32 * 0.20;
    if model.tools.has_ambiguous_duplicates {
        score -= 0.15;
    }

    score.clamp(0.0, 1.0)
}
