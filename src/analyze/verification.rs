use crate::types::config::HarnessConfig;

pub fn verification_score(config: Option<&HarnessConfig>) -> f32 {
    let mut score: f32 = 0.0;
    if let Some(verification) = config.and_then(|cfg| cfg.verification.as_ref()) {
        if !verification.required.is_empty() {
            score += 0.50;
        }
        if verification.pre_completion_required {
            score += 0.30;
        }
        if verification.loop_guard_enabled {
            score += 0.20;
        }
    }
    score.clamp(0.0, 1.0)
}
