pub mod command_policy;
pub mod loop_guard;

use crate::error::HarnessError;
use crate::types::config::HarnessConfig;

#[cfg_attr(not(test), allow(dead_code))]
pub fn validate(commands: &[&str], planned_edits: u32) -> Result<(), HarnessError> {
    validate_with_config(commands, planned_edits, None)
}

pub fn validate_with_config(
    commands: &[&str],
    planned_edits: u32,
    config: Option<&HarnessConfig>,
) -> Result<(), HarnessError> {
    let policy = policy_from_config(config);

    if commands
        .iter()
        .any(|command| command_policy::is_forbidden_with_policy(command, &policy))
    {
        let forbidden = commands
            .iter()
            .find(|command| command_policy::is_forbidden_with_policy(command, &policy))
            .copied()
            .unwrap_or("unknown");
        return Err(HarnessError::ForbiddenToolAccess(forbidden.to_string()));
    }

    if loop_guard::detect_loop(planned_edits) {
        return Err(HarnessError::ConfigParse(
            "loop guard triggered: planned change count exceeds threshold".to_string(),
        ));
    }

    Ok(())
}

fn policy_from_config(config: Option<&HarnessConfig>) -> command_policy::CommandPolicy {
    let mut policy = command_policy::CommandPolicy::default();
    let Some(cfg) = config else {
        return policy;
    };

    if let Some(tools) = &cfg.tools {
        if let Some(baseline) = &tools.baseline {
            for command in &baseline.forbidden {
                if !command.is_empty() {
                    policy.forbidden.push(command.clone());
                }
            }
        }
        if let Some(deprecated) = &tools.deprecated {
            for command in &deprecated.disabled {
                if !command.is_empty() {
                    policy.forbidden.push(command.clone());
                }
            }
        }
        if let Some(aliases) = &tools.aliases {
            for (alias, target) in aliases {
                policy.aliases.insert(alias.clone(), target.clone());
            }
        }
    }

    policy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::config::HarnessConfig;

    #[test]
    fn test_validate_rejects_forbidden_commands() {
        let result = validate(&["git push --force origin main"], 0);
        assert!(matches!(result, Err(HarnessError::ForbiddenToolAccess(_))));
    }

    #[test]
    fn test_validate_rejects_looping_edit_counts() {
        let result = validate(&["git status --porcelain"], 25);
        assert!(matches!(result, Err(HarnessError::ConfigParse(_))));
    }

    #[test]
    fn test_validate_allows_safe_commands_and_edit_count() {
        let result = validate(&["git status --porcelain", "cargo test"], 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_config_rejects_alias_to_forbidden_command() {
        let cfg: HarnessConfig = toml::from_str(
            r#"
[project]
name = "sample"
profile = "general"

[tools]
aliases = { gpf = "git push --force" }

[tools.baseline]
forbidden = ["git push --force"]
"#,
        )
        .expect("config should parse");

        let result = validate_with_config(&["gpf origin main"], 0, Some(&cfg));
        assert!(matches!(result, Err(HarnessError::ForbiddenToolAccess(_))));
    }

    #[test]
    fn test_validate_with_config_rejects_disabled_tool_command() {
        let cfg: HarnessConfig = toml::from_str(
            r#"
[project]
name = "sample"
profile = "general"

[tools.deprecated]
disabled = ["apply_patch"]
"#,
        )
        .expect("config should parse");

        let result = validate_with_config(&["apply_patch"], 0, Some(&cfg));
        assert!(matches!(result, Err(HarnessError::ForbiddenToolAccess(_))));
    }

    #[test]
    fn test_validate_with_config_allows_deprecated_tool_command() {
        let cfg: HarnessConfig = toml::from_str(
            r#"
[project]
name = "sample"
profile = "general"

[tools.deprecated]
deprecated = ["apply_patch"]
"#,
        )
        .expect("config should parse");

        let result = validate_with_config(&["apply_patch"], 0, Some(&cfg));
        assert!(result.is_ok());
    }
}
