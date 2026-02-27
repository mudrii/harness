use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct CommandPolicy {
    pub forbidden: Vec<String>,
    pub aliases: HashMap<String, String>,
}

impl Default for CommandPolicy {
    fn default() -> Self {
        Self {
            forbidden: vec![
                "git push --force".to_string(),
                "git reset --hard".to_string(),
                "rm -rf".to_string(),
                "sudo rm -rf".to_string(),
            ],
            aliases: HashMap::new(),
        }
    }
}

pub fn is_forbidden(cmd: &str) -> bool {
    is_forbidden_with_policy(cmd, &CommandPolicy::default())
}

pub fn is_forbidden_with_policy(cmd: &str, policy: &CommandPolicy) -> bool {
    let expanded = expand_aliases(normalize(cmd), &policy.aliases);
    if expanded.is_empty() {
        return false;
    }

    policy
        .forbidden
        .iter()
        .map(normalize)
        .any(|rule| command_matches(&expanded, &rule))
}

fn command_matches(command: &str, rule: &str) -> bool {
    let command_tokens: Vec<&str> = command.split_whitespace().collect();
    let rule_tokens: Vec<&str> = rule.split_whitespace().collect();
    if command_tokens.is_empty() || rule_tokens.is_empty() {
        return false;
    }

    starts_with_tokens(&command_tokens, &rule_tokens)
        || starts_with_tokens(&rule_tokens, &command_tokens)
}

fn starts_with_tokens(left: &[&str], right: &[&str]) -> bool {
    left.len() >= right.len() && left.iter().zip(right.iter()).all(|(a, b)| a == b)
}

fn expand_aliases(command: String, aliases: &HashMap<String, String>) -> String {
    let mut current = command;
    let mut seen = HashSet::new();

    for _ in 0..8 {
        let mut parts = current.split_whitespace();
        let head = match parts.next() {
            Some(token) => token.to_string(),
            None => return current,
        };
        let tail = parts.collect::<Vec<_>>().join(" ");

        if !seen.insert(head.clone()) {
            return current;
        }

        let Some(alias_target) = aliases.get(&head) else {
            return current;
        };
        current = if tail.is_empty() {
            normalize(alias_target)
        } else {
            normalize(format!("{alias_target} {tail}"))
        };
    }

    current
}

fn normalize(input: impl AsRef<str>) -> String {
    input
        .as_ref()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(forbidden: Vec<&str>, aliases: Vec<(&str, &str)>) -> CommandPolicy {
        CommandPolicy {
            forbidden: forbidden.into_iter().map(ToString::to_string).collect(),
            aliases: aliases
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn test_forbidden_match_blocks_exact_and_prefix() {
        let policy = policy(vec!["rm -rf"], vec![]);
        assert!(is_forbidden_with_policy("rm -rf /tmp/x", &policy));
        assert!(is_forbidden_with_policy("rm", &policy));
    }

    #[test]
    fn test_alias_expansion_blocks_forbidden_target() {
        let policy = policy(vec!["git push --force"], vec![("gpf", "git push --force")]);
        assert!(is_forbidden_with_policy("gpf origin main", &policy));
    }

    #[test]
    fn test_args_aware_matching_distinguishes_safe_and_forbidden_push() {
        let policy = policy(vec!["git push --force"], vec![]);
        assert!(!is_forbidden_with_policy("git push origin main", &policy));
        assert!(is_forbidden_with_policy(
            "git push --force origin main",
            &policy
        ));
    }

    #[test]
    fn test_safe_command_passes() {
        let policy = policy(vec!["rm -rf", "git push --force"], vec![]);
        assert!(!is_forbidden_with_policy("cargo test", &policy));
    }
}
