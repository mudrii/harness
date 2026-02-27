use crate::error::HarnessError;
use crate::types::config::{HarnessConfig, LogSampling};
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SamplingMode {
    Milestones,
    All,
    None,
}

#[derive(Debug, Clone)]
struct ContinuitySettings {
    progress_file: PathBuf,
    sampling_mode: SamplingMode,
    batch_interval_secs: u32,
    max_log_size_kb: u64,
    retained_logs: usize,
}

#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: String,
    feature: String,
    action: String,
    evidence: Vec<String>,
    next_state: String,
}

pub struct ContinuityLogger {
    settings: ContinuitySettings,
    pending: Vec<LogEntry>,
    last_flush: chrono::DateTime<chrono::Utc>,
}

impl ContinuityLogger {
    pub fn new(root: &Path, cfg: Option<&HarnessConfig>) -> Self {
        let settings = resolve_settings(root, cfg);
        Self {
            settings,
            pending: Vec::new(),
            last_flush: Utc::now(),
        }
    }

    pub fn record_milestone(
        &mut self,
        feature: &str,
        action: &str,
        evidence: &[String],
        next_state: &str,
    ) -> Result<(), HarnessError> {
        self.push_entry(feature, action, evidence, next_state);
        self.flush()
    }

    pub fn record_progress(
        &mut self,
        feature: &str,
        action: &str,
        evidence: &[String],
        next_state: &str,
    ) -> Result<(), HarnessError> {
        if !matches!(self.settings.sampling_mode, SamplingMode::All) {
            return Ok(());
        }
        self.push_entry(feature, action, evidence, next_state);
        if Utc::now()
            .signed_duration_since(self.last_flush)
            .num_seconds()
            >= i64::from(self.settings.batch_interval_secs)
        {
            self.flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), HarnessError> {
        if self.pending.is_empty() {
            return Ok(());
        }

        if let Some(parent) = self.settings.progress_file.parent() {
            std::fs::create_dir_all(parent).map_err(HarnessError::Io)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.settings.progress_file)
            .map_err(HarnessError::Io)?;

        for entry in &self.pending {
            let evidence = if entry.evidence.is_empty() {
                "-".to_string()
            } else {
                entry.evidence.join(", ")
            };
            writeln!(
                file,
                "- timestamp: {} | feature: {} | action: {} | evidence: {} | next_state: {}",
                entry.timestamp, entry.feature, entry.action, evidence, entry.next_state
            )
            .map_err(HarnessError::Io)?;
        }
        file.flush().map_err(HarnessError::Io)?;

        self.pending.clear();
        self.last_flush = Utc::now();
        self.rotate_if_needed()
    }

    fn push_entry(&mut self, feature: &str, action: &str, evidence: &[String], next_state: &str) {
        self.pending.push(LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            feature: feature.to_string(),
            action: action.to_string(),
            evidence: evidence.to_vec(),
            next_state: next_state.to_string(),
        });
    }

    fn rotate_if_needed(&self) -> Result<(), HarnessError> {
        let progress_path = &self.settings.progress_file;
        let max_bytes = self.settings.max_log_size_kb * 1024;
        let metadata = match std::fs::metadata(progress_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(HarnessError::Io(error)),
        };

        if metadata.len() <= max_bytes {
            return Ok(());
        }

        let parent = progress_path.parent().unwrap_or_else(|| Path::new("."));
        let stem = progress_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("progress");
        let extension = progress_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("md");
        let stamp = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let rotated = parent.join(format!("{stem}-{stamp}.{extension}"));
        std::fs::rename(progress_path, &rotated).map_err(HarnessError::Io)?;
        std::fs::write(progress_path, "").map_err(HarnessError::Io)?;
        self.prune_rotated_logs(parent, stem, extension)
    }

    fn prune_rotated_logs(
        &self,
        parent: &Path,
        stem: &str,
        extension: &str,
    ) -> Result<(), HarnessError> {
        let prefix = format!("{stem}-");
        let suffix = format!(".{extension}");
        let mut rotated = std::fs::read_dir(parent)
            .map_err(HarnessError::Io)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .map(|name| name.starts_with(&prefix) && name.ends_with(&suffix))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        rotated.sort();
        while rotated.len() > self.settings.retained_logs {
            let stale = rotated.remove(0);
            std::fs::remove_file(stale).map_err(HarnessError::Io)?;
        }
        Ok(())
    }
}

fn resolve_settings(root: &Path, cfg: Option<&HarnessConfig>) -> ContinuitySettings {
    let continuity = cfg.and_then(|value| value.continuity.as_ref());
    let progress_rel = continuity
        .and_then(|value| value.progress_file.as_ref())
        .map_or(".harness/progress.md", String::as_str);
    let progress_file = resolve_path(root, progress_rel);
    let sampling_mode = match continuity.and_then(|value| value.log_sampling.as_ref()) {
        Some(LogSampling::All) => SamplingMode::All,
        Some(LogSampling::None) => SamplingMode::None,
        _ => SamplingMode::Milestones,
    };
    let batch_interval_secs = continuity
        .and_then(|value| value.batch_interval_secs)
        .unwrap_or(60)
        .max(1);
    let max_log_size_kb = continuity
        .and_then(|value| value.max_log_size_kb)
        .unwrap_or(100)
        .max(1) as u64;
    let retained_logs = continuity
        .and_then(|value| value.retained_logs)
        .unwrap_or(3)
        .max(1) as usize;

    ContinuitySettings {
        progress_file,
        sampling_mode,
        batch_interval_secs,
        max_log_size_kb,
        retained_logs,
    }
}

fn resolve_path(root: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_config(toml_str: &str) -> HarnessConfig {
        toml::from_str(toml_str).expect("config should parse")
    }

    #[test]
    fn milestone_logs_even_when_sampling_none() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let config = parse_config(
            r#"
[project]
name = "sample"
profile = "general"

[continuity]
log_sampling = "none"
"#,
        );
        let mut logger = ContinuityLogger::new(dir.path(), Some(&config));
        logger
            .record_milestone(
                "analyze",
                "start",
                &["path=repo".to_string()],
                "running",
            )
            .expect("milestone should be logged");

        let content = std::fs::read_to_string(dir.path().join(".harness/progress.md"))
            .expect("progress file should be readable");
        assert!(content.contains("feature: analyze"));
        assert!(content.contains("action: start"));
    }

    #[test]
    fn progress_is_skipped_when_sampling_milestones() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let config = parse_config(
            r#"
[project]
name = "sample"
profile = "general"

[continuity]
log_sampling = "milestones"
"#,
        );
        let mut logger = ContinuityLogger::new(dir.path(), Some(&config));
        logger
            .record_progress(
                "analyze",
                "scan",
                &["signals=ok".to_string()],
                "running",
            )
            .expect("progress record should not fail");
        logger.flush().expect("flush should succeed");

        assert!(
            !dir.path().join(".harness/progress.md").exists(),
            "progress-only event should not create a file in milestones mode"
        );
    }

    #[test]
    fn progress_is_logged_when_sampling_all() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let config = parse_config(
            r#"
[project]
name = "sample"
profile = "general"

[continuity]
log_sampling = "all"
"#,
        );
        let mut logger = ContinuityLogger::new(dir.path(), Some(&config));
        logger
            .record_progress(
                "analyze",
                "scan",
                &["signals=ok".to_string()],
                "running",
            )
            .expect("progress record should succeed");
        logger.flush().expect("flush should succeed");

        let content = std::fs::read_to_string(dir.path().join(".harness/progress.md"))
            .expect("progress file should be readable");
        assert!(content.contains("feature: analyze"));
        assert!(content.contains("action: scan"));
    }

    #[test]
    fn rotation_prunes_old_logs_using_retained_limit() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let config = parse_config(
            r#"
[project]
name = "sample"
profile = "general"

[continuity]
log_sampling = "all"
max_log_size_kb = 1
retained_logs = 2
"#,
        );
        let mut logger = ContinuityLogger::new(dir.path(), Some(&config));
        let payload = "x".repeat(1600);
        for _ in 0..4 {
            logger
                .record_milestone("bench", "checkpoint", std::slice::from_ref(&payload), "running")
                .expect("milestone log should succeed");
        }

        let rotated = std::fs::read_dir(dir.path().join(".harness"))
            .expect("harness dir should exist")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .map(|name| name.starts_with("progress-") && name.ends_with(".md"))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        assert!(
            rotated.len() <= 2,
            "rotated logs should respect retained limit"
        );
    }
}
