use crate::types::config::HarnessConfig;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct ToolSignals {
    pub tool_names: Vec<String>,
    pub risky_overlap_clusters: usize,
    pub unrestricted_destructive: usize,
    pub has_ambiguous_duplicates: bool,
}

pub fn detect_tools(config: Option<&HarnessConfig>) -> ToolSignals {
    let mut tool_names = if let Some(config) = config {
        collect_config_tools(config)
    } else {
        default_tool_names()
    };
    if tool_names.is_empty() {
        tool_names = default_tool_names();
    }

    normalize_tool_list(&mut tool_names);
    let risky_overlap_clusters = count_overlap_clusters(&tool_names);
    let unrestricted_destructive = count_unrestricted_destructive(&tool_names);
    let has_ambiguous_duplicates = has_duplicates(&tool_names);

    ToolSignals {
        tool_names,
        risky_overlap_clusters,
        unrestricted_destructive,
        has_ambiguous_duplicates,
    }
}

fn default_tool_names() -> Vec<String> {
    vec![
        "bash".to_string(),
        "ls".to_string(),
        "find".to_string(),
        "cat".to_string(),
        "rg".to_string(),
        "git".to_string(),
    ]
}

fn collect_config_tools(config: &HarnessConfig) -> Vec<String> {
    let mut collected = Vec::new();
    if let Some(tools) = &config.tools {
        if let Some(baseline) = &tools.baseline {
            collected.extend(baseline.read.clone());
            collected.extend(baseline.write.clone());
        }
        if let Some(specialized) = &tools.specialized {
            collected.extend(specialized.extra.clone());
        }
    }
    collected
}

fn normalize_tool_list(tools: &mut Vec<String>) {
    tools.retain(|tool| !tool.trim().is_empty());
    for tool in tools.iter_mut() {
        *tool = tool.trim().to_lowercase();
    }
    tools.sort();
}

fn has_duplicates(tools: &[String]) -> bool {
    let mut unique = HashSet::new();
    for tool in tools {
        if !unique.insert(tool) {
            return true;
        }
    }
    false
}

fn count_overlap_clusters(tools: &[String]) -> usize {
    let grep_cluster = ["grep", "rg", "ag", "ack"];
    let find_cluster = ["find", "fd"];
    let mut count = 0;
    if grep_cluster
        .iter()
        .filter(|tool| tools.contains(&tool.to_string()))
        .count()
        > 1
    {
        count += 1;
    }
    if find_cluster
        .iter()
        .filter(|tool| tools.contains(&tool.to_string()))
        .count()
        > 1
    {
        count += 1;
    }
    count
}

fn count_unrestricted_destructive(tools: &[String]) -> usize {
    let dangerous = ["sudo", "mkfs", "fdisk", "rm", "shutdown"];
    tools
        .iter()
        .filter(|tool| dangerous.contains(&tool.as_str()))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_tools_uses_defaults_when_missing_config() {
        let signals = detect_tools(None);
        assert!(signals.tool_names.contains(&"bash".to_string()));
        assert!(!signals.tool_names.is_empty());
    }
}
