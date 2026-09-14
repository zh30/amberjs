// V8 快照预热系统模块
// 提供 V8 快照生成、加载和预热功能
pub mod config;
pub mod manager;
pub mod snapshot;
pub use config::*;
pub use manager::*;
pub use snapshot::*;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

const SNAPSHOT_MAGIC: &[u8; 8] = b"BEEJS_V2";
static CLI_STARTUP_SNAPSHOT: AtomicBool = AtomicBool::new(false);

/// Enable startup snapshot for `bee run` (no-op when disabled via env).
pub fn enable_startup_snapshot_for_cli() {
    CLI_STARTUP_SNAPSHOT.store(true, Ordering::SeqCst);
}

/// Returns the snapshot file name tied to the current package version and build profile.
pub fn startup_snapshot_name() -> String {
    let mode = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!(
        "beejs-startup-v{}-{}-rusty_v8-0.32-warmup-v2.bin",
        env!("CARGO_PKG_VERSION"),
        mode
    )
}

/// Directory used for snapshot cache storage.
pub fn startup_cache_dir() -> std::path::PathBuf {
    std::env::var_os("BEEJS_SNAPSHOT_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| std::path::PathBuf::from(home).join(".cache").join("beejs"))
        })
        .unwrap_or_else(std::env::temp_dir)
}

/// Returns the absolute path to the startup snapshot file.
pub fn startup_blob_path() -> std::path::PathBuf {
    startup_cache_dir().join(startup_snapshot_name())
}

/// Information about current snapshot status.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SnapshotStatusInfo {
    pub enabled: bool,
    pub path: std::path::PathBuf,
    pub exists: bool,
    pub size_bytes: u64,
    pub version: &'static str,
}

/// Query current snapshot status.
pub fn startup_blob_status() -> SnapshotStatusInfo {
    let path = startup_blob_path();
    let (exists, size_bytes) = match std::fs::metadata(&path) {
        Ok(m) => (true, m.len()),
        Err(_) => (false, 0),
    };
    let enabled = !std::env::var_os("BEEJS_DISABLE_STARTUP_SNAPSHOT").is_some()
        && (CLI_STARTUP_SNAPSHOT.load(Ordering::SeqCst)
            || std::env::var_os("BEEJS_STARTUP_SNAPSHOT").is_some());
    SnapshotStatusInfo {
        enabled,
        path,
        exists,
        size_bytes,
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// Clear startup snapshot cache on disk.
pub fn clear_startup_blob_cache() -> std::io::Result<bool> {
    let path = startup_blob_path();
    if path.exists() {
        std::fs::remove_file(path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Explicitly rebuild the startup snapshot and write to disk.
pub fn rebuild_startup_blob() -> anyhow::Result<usize> {
    let path = startup_blob_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let manager = SnapshotManager::new(SnapshotConfig::default());
    let snapshot = manager.generate_snapshot()?;
    let payload = wrap_snapshot_payload(&snapshot.snapshot_data);
    std::fs::write(&path, &payload)?;
    Ok(snapshot.snapshot_data.len())
}

fn wrap_snapshot_payload(data: &[u8]) -> Vec<u8> {
    let version_bytes = env!("CARGO_PKG_VERSION").as_bytes();
    let mut payload = Vec::with_capacity(8 + 2 + version_bytes.len() + 8 + data.len());
    payload.extend_from_slice(SNAPSHOT_MAGIC);
    payload.extend_from_slice(&(version_bytes.len() as u16).to_le_bytes());
    payload.extend_from_slice(version_bytes);
    payload.extend_from_slice(&(data.len() as u64).to_le_bytes());
    payload.extend_from_slice(data);
    payload
}

fn unwrap_snapshot_payload(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 8 + 2 + 8 {
        return None;
    }
    if &bytes[0..8] != SNAPSHOT_MAGIC {
        return None;
    }
    let vlen = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    if bytes.len() < 10 + vlen + 8 {
        return None;
    }
    let version = std::str::from_utf8(&bytes[10..10 + vlen]).ok()?;
    if version != env!("CARGO_PKG_VERSION") {
        return None;
    }
    let dlen_offset = 10 + vlen;
    let mut dlen_bytes = [0u8; 8];
    dlen_bytes.copy_from_slice(&bytes[dlen_offset..dlen_offset + 8]);
    let dlen = u64::from_le_bytes(dlen_bytes) as usize;
    let data_offset = dlen_offset + 8;
    if bytes.len() < data_offset + dlen {
        return None;
    }
    Some(bytes[data_offset..data_offset + dlen].to_vec())
}

/// Process-wide warmup blob for `CreateParams::snapshot_blob`.
/// Opt in with `enable_startup_snapshot_for_cli()` or `BEEJS_STARTUP_SNAPSHOT=1`.
/// Disable with `BEEJS_DISABLE_STARTUP_SNAPSHOT=1`.
pub fn cached_startup_blob() -> Option<&'static [u8]> {
    if std::env::var_os("BEEJS_DISABLE_STARTUP_SNAPSHOT").is_some() {
        return None;
    }
    let enabled = CLI_STARTUP_SNAPSHOT.load(Ordering::SeqCst)
        || std::env::var_os("BEEJS_STARTUP_SNAPSHOT").is_some();
    if !enabled {
        return None;
    }
    static BLOB: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    BLOB.get_or_init(load_or_create_startup_blob).as_deref()
}

fn load_or_create_startup_blob() -> Option<Vec<u8>> {
    let path = startup_blob_path();
    if let Ok(file) = std::fs::File::open(&path) {
        if let Ok(mmap) = unsafe { memmap2::Mmap::map(&file) } {
            if let Some(valid_blob) = unwrap_snapshot_payload(&mmap) {
                return Some(valid_blob);
            } else {
                // Invalid or outdated snapshot detected: remove it so it heals cleanly
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    let manager = SnapshotManager::new(SnapshotConfig::default());
    let snapshot = match manager.generate_snapshot() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: failed to generate V8 startup snapshot: {e}");
            return None;
        }
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let payload = wrap_snapshot_payload(&snapshot.snapshot_data);
    let _ = std::fs::write(&path, &payload);
    Some(snapshot.snapshot_data)
}
