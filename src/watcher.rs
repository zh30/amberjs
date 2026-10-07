// Hot reload file watcher for Amber.
//
// `notify`'s recommended backend reports create/modify/remove/rename. Events are
// filtered, coalesced for `debounce_ms`, and only then handed to the caller.
// Access events are ignored so a read does not reload.

use notify::event::ModifyKind;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

/// File change event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeType {
    Created,
    Modified,
    Removed,
    Renamed,
}

impl FileChangeType {
    /// Stable label used by the CLI and the WebSocket payload.
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Modified => "modified",
            Self::Removed => "removed",
            Self::Renamed => "renamed",
        }
    }
}

/// Represents a file change event
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: FileChangeType,
    pub timestamp: SystemTime,
}

/// Configuration for the hot reload watcher
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce duration to prevent rapid-fire events
    pub debounce_ms: u64,
    /// File extensions to watch (e.g., ["js", "ts", "mjs"])
    pub extensions: Vec<String>,
    /// Directories to ignore (e.g., ["node_modules", ".git"])
    pub ignore_dirs: Vec<String>,
    /// Whether to watch recursively
    pub recursive: bool,
    /// Whether to clear console on reload
    pub clear_console: bool,
    /// Whether to show reload notifications
    pub show_notifications: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            extensions: vec![
                "js".to_string(),
                "ts".to_string(),
                "mjs".to_string(),
                "cjs".to_string(),
                "jsx".to_string(),
                "tsx".to_string(),
            ],
            ignore_dirs: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
                "target".to_string(),
                ".amberjs-cache".to_string(),
            ],
            recursive: true,
            clear_console: true,
            show_notifications: true,
        }
    }
}

/// Statistics for the hot reload watcher
#[derive(Debug, Default)]
pub struct WatcherStats {
    pub total_reloads: AtomicU64,
    pub successful_reloads: AtomicU64,
    pub failed_reloads: AtomicU64,
    pub last_reload_time_ms: AtomicU64,
    pub files_watched: AtomicU64,
}

impl WatcherStats {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn record_reload(&self, success: bool, duration_ms: u64) {
        self.total_reloads.fetch_add(1, Ordering::SeqCst);
        if success {
            self.successful_reloads.fetch_add(1, Ordering::SeqCst);
        } else {
            self.failed_reloads.fetch_add(1, Ordering::SeqCst);
        }
        self.last_reload_time_ms
            .store(duration_ms, Ordering::SeqCst);
    }
    pub fn get_summary(&self) -> WatcherStatsSummary {
        WatcherStatsSummary {
            total_reloads: self.total_reloads.load(Ordering::SeqCst),
            successful_reloads: self.successful_reloads.load(Ordering::SeqCst),
            failed_reloads: self.failed_reloads.load(Ordering::SeqCst),
            last_reload_time_ms: self.last_reload_time_ms.load(Ordering::SeqCst),
            files_watched: self.files_watched.load(Ordering::SeqCst),
        }
    }
}

/// Summary of watcher statistics
#[derive(Debug, Clone)]
pub struct WatcherStatsSummary {
    pub total_reloads: u64,
    pub successful_reloads: u64,
    pub failed_reloads: u64,
    pub last_reload_time_ms: u64,
    pub files_watched: u64,
}

fn check_fs_read_permission(path: &Path) -> anyhow::Result<()> {
    crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::FileSystem,
        crate::permissions::PermissionAction::Read,
        crate::permissions::ResourceId::Path(path.to_path_buf()),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))
}

fn path_is_watched(config: &WatcherConfig, path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    let ext = ext.to_string_lossy().to_lowercase();
    if !config.extensions.iter().any(|candidate| candidate == &ext) {
        return false;
    }
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            let name = name.to_string_lossy();
            if config.ignore_dirs.iter().any(|dir| dir == name.as_ref()) {
                return false;
            }
        }
    }
    true
}

/// Map a notify event kind onto a reload change.
///
/// `Access` and `Other` are ignored. `Any` is treated as a content change.
pub fn file_change_type_from_notify_kind(kind: EventKind) -> Option<FileChangeType> {
    match kind {
        EventKind::Create(_) => Some(FileChangeType::Created),
        EventKind::Remove(_) => Some(FileChangeType::Removed),
        EventKind::Modify(ModifyKind::Name(_)) => Some(FileChangeType::Renamed),
        EventKind::Modify(_) | EventKind::Any => Some(FileChangeType::Modified),
        EventKind::Access(_) | EventKind::Other => None,
    }
}

fn merge_change(previous: FileChangeType, next: FileChangeType) -> FileChangeType {
    match (previous, next) {
        (FileChangeType::Created, FileChangeType::Modified) => FileChangeType::Created,
        (FileChangeType::Removed, FileChangeType::Created) => FileChangeType::Modified,
        (_, next) => next,
    }
}

fn count_watched_files(config: &WatcherConfig, path: &Path) -> anyhow::Result<u64> {
    if path.is_file() {
        if path_is_watched(config, path) {
            check_fs_read_permission(path)?;
            return Ok(1);
        }
        return Ok(0);
    }
    if !path.is_dir() {
        return Ok(0);
    }

    let mut count = 0u64;
    if config.recursive {
        for entry in walkdir::WalkDir::new(path) {
            let entry = entry.map_err(|e| anyhow::anyhow!("Failed to scan watched path: {e}"))?;
            if entry.file_type().is_file() && path_is_watched(config, entry.path()) {
                check_fs_read_permission(entry.path())?;
                count += 1;
            }
        }
    } else {
        for entry in std::fs::read_dir(path)
            .map_err(|e| anyhow::anyhow!("Failed to scan watched path: {e}"))?
        {
            let entry = entry.map_err(|e| anyhow::anyhow!("Failed to scan watched path: {e}"))?;
            let file_path = entry.path();
            if file_path.is_file() && path_is_watched(config, &file_path) {
                check_fs_read_permission(&file_path)?;
                count += 1;
            }
        }
    }
    Ok(count)
}

fn ingest_event(
    config: &WatcherConfig,
    event: Event,
    pending: &mut BTreeMap<PathBuf, FileChangeType>,
    rescan: &mut bool,
) {
    if event.need_rescan() {
        *rescan = true;
    }
    let Some(change_type) = file_change_type_from_notify_kind(event.kind) else {
        return;
    };
    for event_path in event.paths {
        if !path_is_watched(config, &event_path) {
            continue;
        }
        if check_fs_read_permission(&event_path).is_err() {
            continue;
        }
        pending
            .entry(event_path)
            .and_modify(|existing| *existing = merge_change(*existing, change_type))
            .or_insert(change_type);
    }
}

fn flush_pending(
    tx: &mpsc::Sender<FileChange>,
    pending: &mut BTreeMap<PathBuf, FileChangeType>,
    rescan: &mut bool,
    watch_root: &Path,
) -> bool {
    if *rescan && pending.is_empty() {
        let change = FileChange {
            path: watch_root.to_path_buf(),
            change_type: FileChangeType::Modified,
            timestamp: SystemTime::now(),
        };
        if tx.send(change).is_err() {
            return false;
        }
    }
    *rescan = false;
    for (path, change_type) in std::mem::take(pending) {
        let change = FileChange {
            path,
            change_type,
            timestamp: SystemTime::now(),
        };
        if tx.send(change).is_err() {
            return false;
        }
    }
    true
}

/// Hot reload watcher for Amber runtime
pub struct HotReloader {
    config: WatcherConfig,
    stats: Arc<WatcherStats>,
    running: Arc<AtomicBool>,
}

impl HotReloader {
    /// Create a new hot reloader with default configuration
    pub fn new() -> Self {
        Self::with_config(WatcherConfig::default())
    }
    /// Create a new hot reloader with custom configuration
    pub fn with_config(config: WatcherConfig) -> Self {
        Self {
            config,
            stats: Arc::new(WatcherStats::new()),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
    /// Check if a file should be watched based on extension and path
    pub fn should_watch(&self, path: &Path) -> bool {
        path_is_watched(&self.config, path)
    }
    /// Start watching a directory or file for changes.
    ///
    /// The notify watcher is created on this thread, so a missing path or a
    /// backend failure returns `Err` and leaves `is_running()` false.
    /// Returns a channel receiver for file change events.
    pub fn watch(&mut self, path: impl AsRef<Path>) -> anyhow::Result<mpsc::Receiver<FileChange>> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "watch path does not exist: {}",
                path.display()
            ));
        }
        check_fs_read_permission(&path)?;
        let file_count = count_watched_files(&self.config, &path)?;

        let (tx, rx) = mpsc::channel();
        let (notify_tx, notify_rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(notify_tx, notify::Config::default())
            .map_err(|e| anyhow::anyhow!("Failed to create file watcher: {e}"))?;
        let mode = if self.config.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        watcher
            .watch(&path, mode)
            .map_err(|e| anyhow::anyhow!("Failed to watch path {}: {e}", path.display()))?;

        self.stats.files_watched.store(file_count, Ordering::SeqCst);
        self.running.store(true, Ordering::SeqCst);

        let config = self.config.clone();
        let running = self.running.clone();
        let debounce = Duration::from_millis(config.debounce_ms);
        std::thread::spawn(move || {
            let _watcher: RecommendedWatcher = watcher;
            if config.show_notifications {
                println!(
                    "\n\x1b[36m[amberjs]\x1b[0m 👀 Watching for changes in {:?}",
                    path
                );
                println!("\x1b[36m[amberjs]\x1b[0m 📁 Watching {} files", file_count);
            }

            let mut pending: BTreeMap<PathBuf, FileChangeType> = BTreeMap::new();
            let mut rescan = false;
            let mut last_event = Instant::now();
            while running.load(Ordering::SeqCst) {
                let wait = if pending.is_empty() && !rescan {
                    Duration::from_millis(100)
                } else {
                    debounce.saturating_sub(last_event.elapsed())
                };
                match notify_rx.recv_timeout(wait) {
                    Ok(Ok(event)) => {
                        ingest_event(&config, event, &mut pending, &mut rescan);
                        last_event = Instant::now();
                    }
                    Ok(Err(error)) => {
                        eprintln!("[amberjs] Watcher error: {error:?}");
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if (pending.is_empty() && !rescan) || last_event.elapsed() < debounce {
                            continue;
                        }
                        if !flush_pending(&tx, &mut pending, &mut rescan, &path) {
                            break;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            running.store(false, Ordering::SeqCst);
        });
        Ok(rx)
    }
    /// Stop watching for changes
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
    /// Check if the watcher is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    /// Get watcher statistics
    pub fn get_stats(&self) -> WatcherStatsSummary {
        self.stats.get_summary()
    }
    /// Record a reload event
    pub fn record_reload(&self, success: bool, duration_ms: u64) {
        self.stats.record_reload(success, duration_ms);
    }
    /// Clear the console (platform-independent)
    pub fn clear_console(&self) {
        if self.config.clear_console {
            print!("\x1B[2J\x1B[1;1H");
        }
    }
    /// Print reload notification
    pub fn notify_reload(&self, path: &Path, success: bool, duration_ms: u64) {
        if !self.config.show_notifications {
            return;
        }
        let status = if success {
            "\x1b[32m✓\x1b[0m"
        } else {
            "\x1b[31m✗\x1b[0m"
        };
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        println!(
            "\x1b[36m[amberjs]\x1b[0m {} Reloaded {} in {}ms",
            status, filename, duration_ms
        );
    }
}

impl Default for HotReloader {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for WatcherConfig
#[derive(Debug, Default)]
pub struct WatcherConfigBuilder {
    config: WatcherConfig,
}

impl WatcherConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: WatcherConfig::default(),
        }
    }
    pub fn debounce_ms(mut self, ms: u64) -> Self {
        self.config.debounce_ms = ms;
        self
    }
    pub fn extensions(mut self, extensions: Vec<String>) -> Self {
        self.config.extensions = extensions;
        self
    }
    pub fn add_extension(mut self, ext: impl Into<String>) -> Self {
        self.config.extensions.push(ext.into());
        self
    }
    pub fn ignore_dirs(mut self, dirs: Vec<String>) -> Self {
        self.config.ignore_dirs = dirs;
        self
    }
    pub fn add_ignore_dir(mut self, dir: impl Into<String>) -> Self {
        self.config.ignore_dirs.push(dir.into());
        self
    }
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.config.recursive = recursive;
        self
    }
    pub fn clear_console(mut self, clear: bool) -> Self {
        self.config.clear_console = clear;
        self
    }
    pub fn show_notifications(mut self, show: bool) -> Self {
        self.config.show_notifications = show;
        self
    }
    pub fn build(self) -> WatcherConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, RenameMode};

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert!(config.extensions.contains(&"js".to_string()));
        assert!(config.extensions.contains(&"ts".to_string()));
        assert!(config.ignore_dirs.contains(&"node_modules".to_string()));
        assert!(config.ignore_dirs.contains(&"target".to_string()));
        assert!(config.recursive);
    }
    #[test]
    fn test_should_watch() {
        let reloader = HotReloader::new();
        assert!(reloader.should_watch(Path::new("test.js")));
        assert!(reloader.should_watch(Path::new("test.ts")));
        assert!(reloader.should_watch(Path::new("test.tsx")));
        assert!(!reloader.should_watch(Path::new("test.txt")));
        assert!(!reloader.should_watch(Path::new("test.rs")));
        assert!(!reloader.should_watch(Path::new("node_modules/test.js")));
        assert!(!reloader.should_watch(Path::new(".git/test.js")));
        assert!(!reloader.should_watch(Path::new("target/debug/app.js")));
    }
    #[test]
    fn test_watcher_stats() {
        let stats = WatcherStats::new();
        stats.record_reload(true, 50);
        stats.record_reload(true, 60);
        stats.record_reload(false, 100);
        let summary = stats.get_summary();
        assert_eq!(summary.total_reloads, 3);
        assert_eq!(summary.successful_reloads, 2);
        assert_eq!(summary.failed_reloads, 1);
        assert_eq!(summary.last_reload_time_ms, 100);
    }
    #[test]
    fn test_config_builder() {
        let config = WatcherConfigBuilder::new()
            .debounce_ms(200)
            .add_extension("vue")
            .add_ignore_dir("vendor")
            .recursive(false)
            .build();
        assert_eq!(config.debounce_ms, 200);
        assert!(config.extensions.contains(&"vue".to_string()));
        assert!(config.ignore_dirs.contains(&"vendor".to_string()));
        assert!(!config.recursive);
    }

    #[test]
    fn notify_kind_maps_to_stable_change_labels() {
        assert_eq!(
            file_change_type_from_notify_kind(EventKind::Create(notify::event::CreateKind::File)),
            Some(FileChangeType::Created)
        );
        assert_eq!(
            file_change_type_from_notify_kind(EventKind::Remove(notify::event::RemoveKind::File)),
            Some(FileChangeType::Removed)
        );
        assert_eq!(
            file_change_type_from_notify_kind(EventKind::Modify(ModifyKind::Name(
                RenameMode::Both
            ))),
            Some(FileChangeType::Renamed)
        );
        assert_eq!(
            file_change_type_from_notify_kind(EventKind::Modify(ModifyKind::Any)),
            Some(FileChangeType::Modified)
        );
        assert_eq!(
            file_change_type_from_notify_kind(EventKind::Access(AccessKind::Read)),
            None
        );
        assert_eq!(FileChangeType::Created.as_label(), "created");
        assert_eq!(FileChangeType::Renamed.as_label(), "renamed");
    }

    #[test]
    fn merge_keeps_create_across_a_following_write() {
        assert_eq!(
            merge_change(FileChangeType::Created, FileChangeType::Modified),
            FileChangeType::Created
        );
        assert_eq!(
            merge_change(FileChangeType::Removed, FileChangeType::Created),
            FileChangeType::Modified
        );
    }
}
