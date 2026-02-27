use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct HarnessConfig {
    pub project: ProjectConfig,
    pub context: Option<ContextConfig>,
    pub tools: Option<ToolsConfig>,
    pub verification: Option<VerificationConfig>,
    pub continuity: Option<ContinuityConfig>,
    pub metrics: Option<MetricsConfig>,
    pub workflow: Option<WorkflowConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default = "default_profile")]
    pub profile: String,
    pub language: Option<String>,
    #[serde(default = "default_branch")]
    pub main_branch: String,
}

fn default_profile() -> String {
    "general".to_string()
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContextConfig {
    pub agents_map: Option<String>,
    pub context_index: Option<String>,
    #[serde(default)]
    pub doc_map_required: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsConfig {
    pub baseline: Option<ToolBaseline>,
    pub specialized: Option<ToolSpecialized>,
    pub deprecated: Option<ToolDeprecated>,
    pub aliases: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolBaseline {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub write: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolSpecialized {
    #[serde(default)]
    pub extra: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolDeprecated {
    #[serde(default)]
    pub observe: Vec<String>,
    #[serde(default)]
    pub deprecated: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VerificationConfig {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub pre_completion_required: bool,
    #[serde(default)]
    pub loop_guard_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogSampling {
    Milestones,
    All,
    None,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContinuityConfig {
    pub initializer: Option<String>,
    pub coding_prompt: Option<String>,
    pub progress_file: Option<String>,
    pub feature_state_file: Option<String>,
    pub state_schema_version: Option<u32>,
    pub log_sampling: Option<LogSampling>,
    pub batch_interval_secs: Option<u32>,
    pub max_log_size_kb: Option<u32>,
    pub retained_logs: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    pub weights: Option<HashMap<String, f32>>,
    pub max_risk_tolerance: Option<f32>,
    pub max_penalty_per_bucket: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowConfig {
    pub max_consecutive_failures: Option<u32>,
    pub max_idle_steps: Option<u32>,
    #[serde(default)]
    pub replan_on_loop: bool,
}

impl HarnessConfig {
    pub fn default_weights() -> [f32; 5] {
        [0.30, 0.25, 0.20, 0.15, 0.10]
    }

    pub fn weights(&self) -> [f32; 5] {
        match &self.metrics {
            Some(metrics) => match &metrics.weights {
                Some(weights) => [
                    *weights.get("context").unwrap_or(&0.30),
                    *weights.get("tools").unwrap_or(&0.25),
                    *weights.get("continuity").unwrap_or(&0.20),
                    *weights.get("verification").unwrap_or(&0.15),
                    *weights.get("repository_quality").unwrap_or(&0.10),
                ],
                None => Self::default_weights(),
            },
            None => Self::default_weights(),
        }
    }

    pub fn max_penalty_per_bucket(&self) -> f32 {
        self.metrics
            .as_ref()
            .and_then(|metrics| metrics.max_penalty_per_bucket)
            .unwrap_or(0.40)
    }
}

pub type Config = HarnessConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
[project]
name = "test-repo"
profile = "general"
language = "rust"
main_branch = "main"
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("minimal config should parse");
        assert_eq!(cfg.project.name, "test-repo");
        assert_eq!(cfg.project.profile, "general");
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
[project]
name = "test-repo"
profile = "agent"
language = "rust"
main_branch = "main"

[context]
agents_map = "AGENTS.md"
doc_map_required = true

[tools.baseline]
read = ["cat", "rg"]
write = ["apply_patch"]
forbidden = ["sudo"]

[verification]
required = ["cargo test"]
pre_completion_required = true
loop_guard_enabled = true

[metrics]
max_risk_tolerance = 0.35
max_penalty_per_bucket = 0.40
"#;

        let cfg: HarnessConfig = toml::from_str(toml_str).expect("full config should parse");
        assert_eq!(cfg.project.profile, "agent");
        assert_eq!(
            cfg.tools
                .as_ref()
                .and_then(|tools| tools.baseline.as_ref())
                .map(|baseline| baseline.read.clone())
                .unwrap_or_default(),
            vec!["cat".to_string(), "rg".to_string()]
        );
        assert!(cfg
            .verification
            .as_ref()
            .map(|verification| verification.pre_completion_required)
            .unwrap_or(false));
    }

    #[test]
    fn default_weights_sum_to_one() {
        let toml_str = r#"
[project]
name = "test"
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("defaults should parse");
        let weights = cfg.weights();
        assert!((weights.iter().sum::<f32>() - 1.0).abs() < 0.001);
    }
}
