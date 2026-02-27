use chrono::Utc;
use std::path::Path;
use std::process::Command;

pub fn doc_age_days(root: &Path, tracked_paths: &[&str]) -> Option<i64> {
    tracked_paths
        .iter()
        .filter_map(|path| last_commit_unix(root, path))
        .max()
        .map(|ts| {
            let now = Utc::now().timestamp();
            ((now - ts).max(0)) / 86_400
        })
}

fn last_commit_unix(root: &Path, relative_path: &str) -> Option<i64> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("log")
        .arg("-1")
        .arg("--format=%ct")
        .arg("--")
        .arg(relative_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    stdout.trim().parse::<i64>().ok()
}
