use crate::error::{HarnessError, Result};
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct SuggestPlan {
    pub version: String,
    pub generated_at: String,
    pub recommendations: Vec<String>,
}

impl SuggestPlan {
    pub fn new(recommendations: Vec<String>) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            generated_at: Utc::now().to_rfc3339(),
            recommendations,
        }
    }
}

pub fn write_plan(root: &Path, plan: &SuggestPlan) -> Result<PathBuf> {
    let dir = root.join(".harness/plans");
    fs::create_dir_all(&dir).map_err(HarnessError::Io)?;
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let out_path = dir.join(format!("plan-{stamp}.json"));
    let json = serde_json::to_string_pretty(plan)?;
    fs::write(&out_path, json).map_err(HarnessError::Io)?;
    Ok(out_path)
}
