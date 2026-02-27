use crate::scan::RepoModel;

pub fn context_score(model: &RepoModel) -> f32 {
    let mut score: f32 = 0.0;
    if model.docs.has_agents_md && model.docs.agents_has_section_header {
        score += 0.35;
    }
    if model.docs.has_context_index {
        score += 0.20;
    }
    if model.docs.has_architecture_doc {
        score += 0.15;
    }
    if model.docs.readme_links_architecture {
        score += 0.10;
    }
    if model
        .docs
        .docs_age_days
        .map(|days| days < 90)
        .unwrap_or(false)
    {
        score += 0.20;
    }
    score.clamp(0.0, 1.0)
}
