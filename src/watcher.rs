// # Hot Reload Module for Amber
//
// This module provides file watching and hot reload capabilities for development.
// It monitors JavaScript/TypeScript files for changes and automatically re-executes them.

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

/// File change event types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeType {
    Created,
    Modified,
    Removed,
    Renamed,
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
        // Check extension
        if let Some(ext) = path.extension() {
            let ext_str: _ = ext.to_string_lossy().to_lowercase();
            if !self.config.extensions.iter().any(|e| e == &ext_str) {
                return false;
            }
        } else {
            return false;
        }
        // Check ignore directories
        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                let name_str: _ = name.to_string_lossy();
                if self.config.ignore_dirs.iter().any(|d| d == &*name_str) {
                    return false;
                }
            }
        }
        true
    }
    /// Start watching a directory for changes
    /// Returns a channel receiver for file change events
    pub fn watch(&mut self, path: impl AsRef<Path>) -> anyhow::Result<mpsc::Receiver<FileChange>> {
        let path: _ = path.as_ref().to_path_buf();
        check_fs_read_permission(&path)?;
        let (tx, rx) = mpsc::channel();
        let config: _ = self.config.clone();
        let stats: _ = self.stats.clone();
        let running: _ = self.running.clone();
        // Count initial files
        let mut file_count = 0u64;
        if path.is_dir() {
            for entry in walkdir::WalkDir::new(&path) {
                let entry =
                    entry.map_err(|e| anyhow::anyhow!("Failed to scan watched path: {}", e))?;
                if entry.file_type().is_file() && self.should_watch(entry.path()) {
                    check_fs_read_permission(entry.path())?;
                    file_count += 1;
                }
            }
        } else if self.should_watch(&path) {
            file_count = 1;
        }
        stats.files_watched.store(file_count, Ordering::SeqCst);
        running.store(true, Ordering::SeqCst);
        let debounce_duration: _ = Duration::from_millis(config.debounce_ms);
        std::thread::spawn(move || {
            let (notify_tx, notify_rx) = std::sync::mpsc::channel();
            let mut debouncer = match new_debouncer(debounce_duration, notify_tx) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[amberjs] Failed to create file watcher: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };
            let mode: _ = if config.recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            if let Err(e) = debouncer.watcher().watch(&path, mode) {
                eprintln!("[amberjs] Failed to watch path {:?}: {}", path, e);
                running.store(false, Ordering::SeqCst);
                return;
            }
            if config.show_notifications {
                println!(
                    "\n\x1b[36m[amberjs]\x1b[0m 👀 Watching for changes in {:?}",
                    path
                );
                println!("\x1b[36m[amberjs]\x1b[0m 📁 Watching {} files", file_count);
            }
            while running.load(Ordering::SeqCst) {
                match notify_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(events)) => {
                        for event in events {
                            let event_path: _ = event.path;
                            // Check if we should watch this file
                            let ext: _ = event_path
                                .extension()
                                .map(|e| e.to_string_lossy().to_lowercase());
                            let should_process: _ = ext
                                .map(|e| config.extensions.iter().any(|x| x == &*e))
                                .unwrap_or(false);
                            if !should_process {
                                continue;
                            }
                            // Check ignore directories
                            let mut ignored = false;
                            for component in event_path.components() {
                                if let std::path::Component::Normal(name) = component {
                                    let name_str: _ = name.to_string_lossy();
                                    if config.ignore_dirs.iter().any(|d| d == &*name_str) {
                                        ignored = true;
                                        break;
                                    }
                                }
                            }
                            if ignored {
                                continue;
                            }
                            if check_fs_read_permission(&event_path).is_err() {
                                continue;
                            }
                            let change_type: _ = match event.kind {
                                DebouncedEventKind::Any => FileChangeType::Modified,
                                DebouncedEventKind::AnyContinuous => FileChangeType::Modified,
                                _ => FileChangeType::Modified,
                            };
                            let change: _ = FileChange {
                                path: event_path,
                                change_type,
                                timestamp: SystemTime::now(),
                            };
                            if tx.send(change).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("[amberjs] Watcher error: {:?}", e);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        continue;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }
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
        let status: _ = if success {
            "\x1b[32m✓\x1b[0m"
        } else {
            "\x1b[31m✗\x1b[0m"
        };
        let filename: _ = path
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

    #[test]
    fn test_watcher_config_default() {
        let config: _ = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert!(config.extensions.contains(&"js".to_string()));
        assert!(config.extensions.contains(&"ts".to_string()));
        assert!(config.ignore_dirs.contains(&"node_modules".to_string()));
        assert!(config.recursive);
    }
    #[test]
    fn test_should_watch() {
        let reloader: _ = HotReloader::new();
        // Should watch JS/TS files
        assert!(reloader.should_watch(Path::new("test.js")));
        assert!(reloader.should_watch(Path::new("test.ts")));
        assert!(reloader.should_watch(Path::new("test.tsx")));
        // Should not watch non-JS files
        assert!(!reloader.should_watch(Path::new("test.txt")));
        assert!(!reloader.should_watch(Path::new("test.rs")));
        // Should not watch files in ignored directories
        assert!(!reloader.should_watch(Path::new("node_modules/test.js")));
        assert!(!reloader.should_watch(Path::new(".git/test.js")));
    }
    #[test]
    fn test_watcher_stats() {
        let stats: _ = WatcherStats::new();
        stats.record_reload(true, 50);
        stats.record_reload(true, 60);
        stats.record_reload(false, 100);
        let summary: _ = stats.get_summary();
        assert_eq!(summary.total_reloads, 3);
        assert_eq!(summary.successful_reloads, 2);
        assert_eq!(summary.failed_reloads, 1);
        assert_eq!(summary.last_reload_time_ms, 100);
    }
    #[test]
    fn test_config_builder() {
        let config: _ = WatcherConfigBuilder::new()
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
}
