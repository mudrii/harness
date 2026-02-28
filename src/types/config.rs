use crate::error::HarnessError;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Deserialize)]
pub struct HarnessConfig {
    pub project: ProjectConfig,
    pub context: Option<ContextConfig>,
    pub tools: Option<ToolsConfig>,
    pub verification: Option<VerificationConfig>,
    pub continuity: Option<ContinuityConfig>,
    pub metrics: Option<MetricsConfig>,
    pub optimization: Option<OptimizationConfig>,
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
pub struct OptimizationConfig {
    pub min_traces: Option<u32>,
    pub min_uplift_abs: Option<f32>,
    pub min_uplift_rel: Option<f32>,
    pub trace_staleness_days: Option<u32>,
    pub task_overlap_threshold: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptimizationThresholds {
    pub min_traces: u32,
    pub min_uplift_abs: f32,
    pub min_uplift_rel: f32,
    pub trace_staleness_days: u32,
    pub task_overlap_threshold: f32,
}

impl Default for OptimizationThresholds {
    fn default() -> Self {
        Self {
            min_traces: 30,
            min_uplift_abs: 0.05,
            min_uplift_rel: 0.10,
            trace_staleness_days: 90,
            task_overlap_threshold: 0.50,
        }
    }
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

    pub fn optimization_thresholds(&self) -> OptimizationThresholds {
        let defaults = OptimizationThresholds::default();
        match &self.optimization {
            Some(optimization) => OptimizationThresholds {
                min_traces: optimization.min_traces.unwrap_or(defaults.min_traces),
                min_uplift_abs: optimization.min_uplift_abs.unwrap_or(defaults.min_uplift_abs),
                min_uplift_rel: optimization.min_uplift_rel.unwrap_or(defaults.min_uplift_rel),
                trace_staleness_days: optimization
                    .trace_staleness_days
                    .unwrap_or(defaults.trace_staleness_days),
                task_overlap_threshold: optimization
                    .task_overlap_threshold
                    .unwrap_or(defaults.task_overlap_threshold),
            },
            None => defaults,
        }
    }

    pub fn validate(&self) -> Result<(), HarnessError> {
        if !matches!(self.project.profile.as_str(), "general" | "agent") {
            return Err(HarnessError::ConfigParse(format!(
                "unsupported project.profile: {}",
                self.project.profile
            )));
        }

        let weights = self.weights();
        if weights.iter().any(|weight| !(0.0..=1.0).contains(weight)) {
            return Err(HarnessError::ConfigParse(
                "metrics.weights values must be between 0.0 and 1.0".to_string(),
            ));
        }
        let weight_sum: f32 = weights.iter().sum();
        if (weight_sum - 1.0).abs() > 0.001 {
            return Err(HarnessError::ConfigParse(format!(
                "metrics.weights must sum to 1.0 (found {:.3})",
                weight_sum
            )));
        }

        if let Some(metrics) = &self.metrics {
            if let Some(weights) = &metrics.weights {
                const ALLOWED_WEIGHT_KEYS: [&str; 5] = [
                    "context",
                    "tools",
                    "continuity",
                    "verification",
                    "repository_quality",
                ];
                let unknown = weights
                    .keys()
                    .filter(|key| !ALLOWED_WEIGHT_KEYS.contains(&key.as_str()))
                    .cloned()
                    .collect::<Vec<_>>();
                if !unknown.is_empty() {
                    return Err(HarnessError::ConfigParse(format!(
                        "metrics.weights contains unknown key(s): {}",
                        unknown.join(", ")
                    )));
                }
            }

            if let Some(max_risk_tolerance) = metrics.max_risk_tolerance {
                if !(0.0..=1.0).contains(&max_risk_tolerance) {
                    return Err(HarnessError::ConfigParse(
                        "metrics.max_risk_tolerance must be between 0.0 and 1.0".to_string(),
                    ));
                }
            }
            if let Some(max_penalty_per_bucket) = metrics.max_penalty_per_bucket {
                if !(0.0..=1.0).contains(&max_penalty_per_bucket) {
                    return Err(HarnessError::ConfigParse(
                        "metrics.max_penalty_per_bucket must be between 0.0 and 1.0".to_string(),
                    ));
                }
            }
        }

        if let Some(verification) = &self.verification {
            if verification.pre_completion_required && verification.required.is_empty() {
                return Err(HarnessError::ConfigParse(
                    "verification.required cannot be empty when pre_completion_required = true"
                        .to_string(),
                ));
            }
        }

        if let Some(deprecated) = self
            .tools
            .as_ref()
            .and_then(|tools| tools.deprecated.as_ref())
        {
            validate_tool_deprecation_lifecycle(deprecated)?;
        }

        if let Some(optimization) = &self.optimization {
            if let Some(min_traces) = optimization.min_traces {
                if min_traces == 0 {
                    return Err(HarnessError::ConfigParse(
                        "optimization.min_traces must be greater than 0".to_string(),
                    ));
                }
            }
            if let Some(min_uplift_abs) = optimization.min_uplift_abs {
                if !(0.0..=1.0).contains(&min_uplift_abs) {
                    return Err(HarnessError::ConfigParse(
                        "optimization.min_uplift_abs must be between 0.0 and 1.0".to_string(),
                    ));
                }
            }
            if let Some(min_uplift_rel) = optimization.min_uplift_rel {
                if !(0.0..=1.0).contains(&min_uplift_rel) {
                    return Err(HarnessError::ConfigParse(
                        "optimization.min_uplift_rel must be between 0.0 and 1.0".to_string(),
                    ));
                }
            }
            if let Some(trace_staleness_days) = optimization.trace_staleness_days {
                if trace_staleness_days == 0 {
                    return Err(HarnessError::ConfigParse(
                        "optimization.trace_staleness_days must be greater than 0".to_string(),
                    ));
                }
            }
            if let Some(task_overlap_threshold) = optimization.task_overlap_threshold {
                if !(0.0..=1.0).contains(&task_overlap_threshold) {
                    return Err(HarnessError::ConfigParse(
                        "optimization.task_overlap_threshold must be between 0.0 and 1.0"
                            .to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

fn validate_tool_deprecation_lifecycle(deprecated: &ToolDeprecated) -> Result<(), HarnessError> {
    let mut seen = HashMap::<String, &'static str>::new();
    for (stage, tools) in [
        ("observe", &deprecated.observe),
        ("deprecated", &deprecated.deprecated),
        ("disabled", &deprecated.disabled),
    ] {
        let mut stage_seen = HashSet::<String>::new();
        for tool in tools {
            let normalized = tool.trim();
            if normalized.is_empty() {
                return Err(HarnessError::ConfigParse(format!(
                    "tools.deprecated.{stage} entries must be non-empty command names"
                )));
            }
            if !stage_seen.insert(normalized.to_string()) {
                return Err(HarnessError::ConfigParse(format!(
                    "tools.deprecated.{stage} contains duplicate tool: {normalized}"
                )));
            }
            if let Some(existing_stage) = seen.get(normalized) {
                return Err(HarnessError::ConfigParse(format!(
                    "tool '{normalized}' cannot appear in both tools.deprecated.{existing_stage} and tools.deprecated.{stage}"
                )));
            }
            seen.insert(normalized.to_string(), stage);
        }
    }

    Ok(())
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

    #[test]
    fn validate_rejects_invalid_weight_sum() {
        let toml_str = r#"
[project]
name = "test"

[metrics.weights]
context = 0.9
tools = 0.9
continuity = 0.1
verification = 0.1
repository_quality = 0.1
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_pre_completion_without_required() {
        let toml_str = r#"
[project]
name = "test"

[verification]
pre_completion_required = true
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_accepts_valid_config() {
        let toml_str = r#"
[project]
name = "test"
profile = "general"

[metrics]
max_risk_tolerance = 0.5
max_penalty_per_bucket = 0.4

[verification]
required = ["cargo test"]
pre_completion_required = true
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn optimization_thresholds_default_when_missing() {
        let toml_str = r#"
[project]
name = "test"
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        let thresholds = cfg.optimization_thresholds();
        assert_eq!(
            thresholds,
            OptimizationThresholds {
                min_traces: 30,
                min_uplift_abs: 0.05,
                min_uplift_rel: 0.10,
                trace_staleness_days: 90,
                task_overlap_threshold: 0.50,
            }
        );
    }

    #[test]
    fn optimization_thresholds_parse_and_override_defaults() {
        let toml_str = r#"
[project]
name = "test"

[optimization]
min_traces = 50
min_uplift_abs = 0.08
min_uplift_rel = 0.12
trace_staleness_days = 30
task_overlap_threshold = 0.75
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        let thresholds = cfg.optimization_thresholds();
        assert_eq!(
            thresholds,
            OptimizationThresholds {
                min_traces: 50,
                min_uplift_abs: 0.08,
                min_uplift_rel: 0.12,
                trace_staleness_days: 30,
                task_overlap_threshold: 0.75,
            }
        );
    }

    #[test]
    fn validate_rejects_invalid_optimization_thresholds() {
        let toml_str = r#"
[project]
name = "test"

[optimization]
min_traces = 0
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        let err = cfg.validate().expect_err("validation should fail");
        assert!(
            err.to_string()
                .contains("optimization.min_traces must be greater than 0")
        );
    }

    #[test]
    fn validate_rejects_unknown_metrics_weight_keys() {
        let toml_str = r#"
[project]
name = "test"

[metrics.weights]
context = 0.30
tools = 0.25
continuity = 0.20
verification = 0.15
repository_quality = 0.10
unknown_bucket = 0.01
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        let err = cfg.validate().expect_err("validation should fail");
        assert!(err.to_string().contains("unknown key"));
        assert!(err.to_string().contains("unknown_bucket"));
    }

    #[test]
    fn validate_rejects_invalid_project_profile() {
        let toml_str = r#"
[project]
name = "test"
profile = "ops"
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        let err = cfg.validate().expect_err("validation should fail");
        assert!(err.to_string().contains("unsupported project.profile"));
    }

    #[test]
    fn validate_accepts_metric_boundaries() {
        let toml_str = r#"
[project]
name = "test"
profile = "general"

[metrics]
max_risk_tolerance = 1.0
max_penalty_per_bucket = 0.0
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_rejects_tool_in_multiple_deprecation_stages() {
        let toml_str = r#"
[project]
name = "test"
profile = "general"

[tools.deprecated]
observe = ["grep"]
deprecated = ["grep"]
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        let err = cfg.validate().expect_err("validation should fail");
        assert!(err.to_string().contains("cannot appear in both"));
        assert!(err.to_string().contains("tools.deprecated.observe"));
        assert!(err.to_string().contains("tools.deprecated.deprecated"));
    }

    #[test]
    fn validate_rejects_empty_deprecation_entry() {
        let toml_str = r#"
[project]
name = "test"
profile = "general"

[tools.deprecated]
observe = [" "]
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        let err = cfg.validate().expect_err("validation should fail");
        assert!(err.to_string().contains("must be non-empty command names"));
    }

    #[test]
    fn validate_accepts_distinct_deprecation_stages() {
        let toml_str = r#"
[project]
name = "test"
profile = "general"

[tools.deprecated]
observe = ["find"]
deprecated = ["grep"]
disabled = ["apply_patch"]
"#;
        let cfg: HarnessConfig = toml::from_str(toml_str).expect("config should parse");
        assert!(cfg.validate().is_ok());
    }
}
