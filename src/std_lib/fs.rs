//! High-level filesystem extensions (`amber:std/fs`).

use anyhow::Result;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub is_file: bool,
    pub is_dir: bool,
    pub size: u64,
}

/// Recursively walk a directory, returning a list of file entries
pub fn walk_dir_sync(
    dir: &Path,
    max_depth: Option<usize>,
    exts: Option<&[String]>,
) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let mut walker = WalkDir::new(dir);
    if let Some(depth) = max_depth {
        walker = walker.max_depth(depth);
    }

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path == dir {
            continue;
        }

        if let Some(extensions) = exts {
            if !entry.file_type().is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !extensions.iter().any(|e| e.to_lowercase() == ext) {
                continue;
            }
        }

        let metadata = entry.metadata().ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let is_file = entry.file_type().is_file();
        let is_dir = entry.file_type().is_dir();
        let name = entry.file_name().to_string_lossy().to_string();

        entries.push(FileEntry {
            path: path.to_string_lossy().to_string(),
            name,
            is_file,
            is_dir,
            size,
        });
    }

    Ok(entries)
}

/// Recursively copy a directory to destination
pub fn copy_dir_sync(src: &Path, dest: &Path) -> Result<()> {
    if !dest.exists() {
        fs::create_dir_all(dest)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_sync(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

/// Remove all files and directories inside a folder without deleting the folder itself
pub fn empty_dir_sync(dir: &Path) -> Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}
