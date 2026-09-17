//! In-memory deterministic Virtual Filesystem (VFS) sandbox for Beejs.
//!
//! Provides an isolated, in-memory copy-on-write (COW) filesystem for executing
//! untrusted Agent-generated code without any side-effects on the host machine.

use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

static VFS_ENABLED: AtomicBool = AtomicBool::new(false);
static VFS_COW: AtomicBool = AtomicBool::new(true);

static VFS_FILES: Lazy<RwLock<HashMap<PathBuf, Vec<u8>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static VFS_DIRS: Lazy<RwLock<HashSet<PathBuf>>> = Lazy::new(|| RwLock::new(HashSet::new()));
static VFS_DELETED: Lazy<RwLock<HashSet<PathBuf>>> = Lazy::new(|| RwLock::new(HashSet::new()));

/// Returns true if virtual filesystem sandboxing is currently enabled
pub fn is_enabled() -> bool {
    VFS_ENABLED.load(Ordering::SeqCst)
}

/// Returns true if Copy-On-Write fallback to host disk is enabled
pub fn is_cow_enabled() -> bool {
    VFS_COW.load(Ordering::SeqCst)
}

/// Enables the in-memory virtual filesystem
pub fn enable(cow_mode: bool) {
    VFS_COW.store(cow_mode, Ordering::SeqCst);
    VFS_ENABLED.store(true, Ordering::SeqCst);
}

/// Disables the virtual filesystem and resets state
pub fn disable() {
    VFS_ENABLED.store(false, Ordering::SeqCst);
    reset();
}

/// Clears all files, directories, and deleted tracking in the virtual filesystem
pub fn reset() {
    if let Ok(mut files) = VFS_FILES.write() {
        files.clear();
    }
    if let Ok(mut dirs) = VFS_DIRS.write() {
        dirs.clear();
    }
    if let Ok(mut deleted) = VFS_DELETED.write() {
        deleted.clear();
    }
}

fn normalize(path: &Path) -> PathBuf {
    if let Ok(curr) = std::env::current_dir() {
        if path.is_relative() {
            return curr.join(path);
        }
    }
    path.to_path_buf()
}

/// Reads file bytes from Virtual Filesystem (with COW fallback if enabled)
pub fn vfs_read(path: &Path) -> io::Result<Vec<u8>> {
    let norm = normalize(path);

    // Check if explicitly marked deleted in COW mode
    if let Ok(deleted) = VFS_DELETED.read() {
        if deleted.contains(&norm) {
            return Err(io::Error::new(ErrorKind::NotFound, "File not found in VFS"));
        }
    }

    // Check in-memory store
    if let Ok(files) = VFS_FILES.read() {
        if let Some(bytes) = files.get(&norm) {
            return Ok(bytes.clone());
        }
    }

    // If COW mode enabled, read from host disk as base layer
    if is_cow_enabled() && norm.exists() && norm.is_file() {
        return std::fs::read(&norm);
    }

    Err(io::Error::new(ErrorKind::NotFound, "File not found in VFS"))
}

/// Reads UTF-8 string from Virtual Filesystem
pub fn vfs_read_to_string(path: &Path) -> io::Result<String> {
    let bytes = vfs_read(path)?;
    String::from_utf8(bytes).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
}

/// Writes file bytes into the in-memory Virtual Filesystem
pub fn vfs_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let norm = normalize(path);

    // Ensure parent directory is recorded
    if let Some(parent) = norm.parent() {
        let _ = vfs_create_dir_all(parent);
    }

    // Unmark from deleted set if re-creating
    if let Ok(mut deleted) = VFS_DELETED.write() {
        deleted.remove(&norm);
    }

    if let Ok(mut files) = VFS_FILES.write() {
        files.insert(norm, data.to_vec());
        Ok(())
    } else {
        Err(io::Error::new(ErrorKind::Other, "Lock poisoned"))
    }
}

/// Creates a directory in the Virtual Filesystem
pub fn vfs_create_dir(path: &Path) -> io::Result<()> {
    let norm = normalize(path);
    if let Ok(mut dirs) = VFS_DIRS.write() {
        dirs.insert(norm);
        Ok(())
    } else {
        Err(io::Error::new(ErrorKind::Other, "Lock poisoned"))
    }
}

/// Recursively creates directories in the Virtual Filesystem
pub fn vfs_create_dir_all(path: &Path) -> io::Result<()> {
    let norm = normalize(path);
    let mut current = PathBuf::new();
    for component in norm.components() {
        current.push(component);
        let _ = vfs_create_dir(&current);
    }
    Ok(())
}

/// Removes a file in the Virtual Filesystem
pub fn vfs_remove_file(path: &Path) -> io::Result<()> {
    let norm = normalize(path);
    let mut found = false;

    if let Ok(mut files) = VFS_FILES.write() {
        if files.remove(&norm).is_some() {
            found = true;
        }
    }

    if is_cow_enabled() {
        if let Ok(mut deleted) = VFS_DELETED.write() {
            deleted.insert(norm.clone());
            if norm.exists() {
                found = true;
            }
        }
    }

    if found {
        Ok(())
    } else {
        Err(io::Error::new(
            ErrorKind::NotFound,
            "File not found to remove",
        ))
    }
}

/// Removes directory and all children in the Virtual Filesystem
pub fn vfs_remove_dir_all(path: &Path) -> io::Result<()> {
    let norm = normalize(path);

    if let Ok(mut files) = VFS_FILES.write() {
        files.retain(|k, _| !k.starts_with(&norm));
    }
    if let Ok(mut dirs) = VFS_DIRS.write() {
        dirs.retain(|d| !d.starts_with(&norm));
    }
    if is_cow_enabled() {
        if let Ok(mut deleted) = VFS_DELETED.write() {
            deleted.insert(norm);
        }
    }
    Ok(())
}

/// Checks if a file or directory exists in the Virtual Filesystem
pub fn vfs_exists(path: &Path) -> bool {
    let norm = normalize(path);

    if let Ok(deleted) = VFS_DELETED.read() {
        if deleted.contains(&norm) {
            return false;
        }
    }

    if let Ok(files) = VFS_FILES.read() {
        if files.contains_key(&norm) {
            return true;
        }
    }

    if let Ok(dirs) = VFS_DIRS.read() {
        if dirs.contains(&norm) {
            return true;
        }
    }

    if is_cow_enabled() && norm.exists() {
        return true;
    }

    false
}

/// Metadata for VFS entries
#[derive(Debug, Clone, Copy)]
pub struct VfsMetadata {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
}

/// Stat a path in the VFS
pub fn vfs_metadata(path: &Path) -> io::Result<VfsMetadata> {
    let norm = normalize(path);

    if let Ok(deleted) = VFS_DELETED.read() {
        if deleted.contains(&norm) {
            return Err(io::Error::new(ErrorKind::NotFound, "Path deleted in VFS"));
        }
    }

    if let Ok(files) = VFS_FILES.read() {
        if let Some(bytes) = files.get(&norm) {
            return Ok(VfsMetadata {
                is_file: true,
                is_dir: false,
                len: bytes.len() as u64,
            });
        }
    }

    if let Ok(dirs) = VFS_DIRS.read() {
        if dirs.contains(&norm) {
            return Ok(VfsMetadata {
                is_file: false,
                is_dir: true,
                len: 0,
            });
        }
    }

    if is_cow_enabled() && norm.exists() {
        let meta = std::fs::metadata(&norm)?;
        return Ok(VfsMetadata {
            is_file: meta.is_file(),
            is_dir: meta.is_dir(),
            len: meta.len(),
        });
    }

    Err(io::Error::new(ErrorKind::NotFound, "Path not found in VFS"))
}

/// Reads directory entries in the VFS
pub fn vfs_read_dir(path: &Path) -> io::Result<Vec<String>> {
    let norm = normalize(path);

    if let Ok(deleted) = VFS_DELETED.read() {
        if deleted.contains(&norm) {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                "Directory deleted in VFS",
            ));
        }
    }

    let mut entries = std::collections::BTreeSet::new();
    let mut dir_found = false;

    if let Ok(dirs) = VFS_DIRS.read() {
        if dirs.contains(&norm) {
            dir_found = true;
        }
        for d in dirs.iter() {
            if let Ok(rel) = d.strip_prefix(&norm) {
                if let Some(first) = rel.components().next() {
                    let s = first.as_os_str().to_string_lossy();
                    if !s.is_empty() {
                        entries.insert(s.to_string());
                    }
                }
            }
        }
    }

    if let Ok(files) = VFS_FILES.read() {
        for f in files.keys() {
            if let Ok(rel) = f.strip_prefix(&norm) {
                dir_found = true;
                if let Some(first) = rel.components().next() {
                    let s = first.as_os_str().to_string_lossy();
                    if !s.is_empty() {
                        entries.insert(s.to_string());
                    }
                }
            }
        }
    }

    if is_cow_enabled() && norm.is_dir() {
        dir_found = true;
        if let Ok(rd) = std::fs::read_dir(&norm) {
            for entry in rd.flatten() {
                let p = entry.path();
                let deleted = if let Ok(del) = VFS_DELETED.read() {
                    del.contains(&p)
                } else {
                    false
                };
                if !deleted {
                    if let Some(name) = entry.file_name().to_str() {
                        entries.insert(name.to_string());
                    }
                }
            }
        }
    }

    if !dir_found && entries.is_empty() {
        return Err(io::Error::new(ErrorKind::NotFound, "Directory not found"));
    }

    Ok(entries.into_iter().collect())
}

/// Lists all files currently stored in memory
pub fn list_virtual_files() -> Vec<String> {
    if let Ok(files) = VFS_FILES.read() {
        files
            .keys()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    } else {
        Vec::new()
    }
}

/// Exports a JSON snapshot of the virtual filesystem
pub fn export_snapshot() -> Value {
    let mut map = serde_json::Map::new();
    if let Ok(files) = VFS_FILES.read() {
        for (path, bytes) in files.iter() {
            let s = String::from_utf8_lossy(bytes).to_string();
            map.insert(path.to_string_lossy().to_string(), json!(s));
        }
    }
    json!({
        "enabled": is_enabled(),
        "cow": is_cow_enabled(),
        "files": map
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs_read_write_and_isolation() {
        enable(true);
        let test_path = PathBuf::from("/virtual_sandbox/test_data.txt");

        assert!(!vfs_exists(&test_path));
        vfs_write(&test_path, b"Hello Virtual Sandbox").expect("write");
        assert!(vfs_exists(&test_path));

        let content = vfs_read_to_string(&test_path).expect("read");
        assert_eq!(content, "Hello Virtual Sandbox");

        // Verify host disk was NEVER created
        assert!(!test_path.exists());

        vfs_remove_file(&test_path).expect("remove");
        assert!(!vfs_exists(&test_path));

        disable();
    }
}
