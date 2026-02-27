use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn list_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

pub fn read_to_string_if_exists(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub fn file_exists(path: &Path) -> bool {
    path.exists()
}
