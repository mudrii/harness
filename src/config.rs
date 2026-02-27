use crate::error::{HarnessError, Result};
use crate::types::config::HarnessConfig;
use std::path::{Path, PathBuf};
use toml::map::Map;
use toml::Value;

pub const DEFAULT_CONFIG_FILE: &str = "harness.toml";
pub const DEFAULT_LOCAL_FILE: &str = ".harness/local.toml";
pub const DEFAULT_GLOBAL_CONFIG_FILE: &str = ".config/harness/config.toml";

pub fn load_config(root: &Path) -> Result<Option<HarnessConfig>> {
    let global = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(DEFAULT_GLOBAL_CONFIG_FILE));
    load_config_with_global(root, global.as_deref())
}

pub(crate) fn load_config_with_global(
    root: &Path,
    global_path: Option<&Path>,
) -> Result<Option<HarnessConfig>> {
    let repo_path = root.join(DEFAULT_CONFIG_FILE);
    if !repo_path.exists() {
        return Ok(None);
    }

    let mut merged = Value::Table(Map::new());
    if let Some(path) = global_path {
        merge_file_if_exists(&mut merged, path)?;
    }
    merge_file_if_exists(&mut merged, &repo_path)?;
    merge_file_if_exists(&mut merged, &root.join(DEFAULT_LOCAL_FILE))?;

    let cfg: HarnessConfig = merged
        .try_into()
        .map_err(|e: toml::de::Error| HarnessError::ConfigParse(e.to_string()))?;
    Ok(Some(cfg))
}

fn merge_file_if_exists(merged: &mut Value, path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let value = read_toml_value(path)?;
    merge_toml(merged, value);
    Ok(())
}

fn read_toml_value(path: &Path) -> Result<Value> {
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content)
        .map_err(|e| HarnessError::ConfigParse(format!("{}: {}", path.display(), e)))
}

fn merge_toml(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Table(base_table), Value::Table(overlay_table)) => {
            for (key, value) in overlay_table {
                match base_table.get_mut(&key) {
                    Some(existing) => merge_toml(existing, value),
                    None => {
                        base_table.insert(key, value);
                    }
                }
            }
        }
        (slot, value) => {
            *slot = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_config_returns_none_when_repo_file_missing() {
        let dir = TempDir::new().expect("temp dir should be created");
        let cfg = load_config_with_global(dir.path(), None).expect("load should not fail");
        assert!(cfg.is_none());
    }

    #[test]
    fn load_config_merges_global_repo_and_local_in_order() {
        let root = TempDir::new().expect("root temp dir should be created");
        let global_root = TempDir::new().expect("global temp dir should be created");
        let global_path = global_root.path().join("config.toml");

        fs::write(
            &global_path,
            r#"
[context]
agents_map = "GLOBAL_AGENTS.md"

[metrics]
max_risk_tolerance = 0.20
"#,
        )
        .expect("global config should write");

        fs::write(
            root.path().join(DEFAULT_CONFIG_FILE),
            r#"
[project]
name = "repo"
profile = "general"
main_branch = "main"

[context]
agents_map = "AGENTS.md"
context_index = "docs/context/INDEX.md"
"#,
        )
        .expect("repo config should write");

        fs::create_dir_all(root.path().join(".harness")).expect("local harness dir should create");
        fs::write(
            root.path().join(DEFAULT_LOCAL_FILE),
            r#"
[project]
profile = "agent"
"#,
        )
        .expect("local override should write");

        let cfg = load_config_with_global(root.path(), Some(&global_path))
            .expect("load should succeed")
            .expect("merged config should exist");

        assert_eq!(cfg.project.name, "repo");
        assert_eq!(cfg.project.profile, "agent");
        assert_eq!(
            cfg.context
                .as_ref()
                .and_then(|context| context.agents_map.as_ref())
                .map(String::as_str),
            Some("AGENTS.md")
        );
        assert_eq!(
            cfg.metrics
                .as_ref()
                .and_then(|metrics| metrics.max_risk_tolerance),
            Some(0.20)
        );
    }
}
