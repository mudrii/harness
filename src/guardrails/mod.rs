pub mod command_policy;
pub mod loop_guard;

use crate::error::HarnessError;

pub fn validate(commands: &[&str], planned_edits: u32) -> Result<(), HarnessError> {
    if commands
        .iter()
        .any(|command| command_policy::is_forbidden(command))
    {
        let forbidden = commands
            .iter()
            .find(|command| command_policy::is_forbidden(command))
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
