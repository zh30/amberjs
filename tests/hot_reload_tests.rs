// Hot Reload Tests for Beejs Runtime

use amberjs::permissions::{
    global_resource_broker, PermissionAction, PermissionKind, ResourceBroker, ResourceId,
};
use amberjs::watcher::{FileChangeType, HotReloader, WatcherConfig, WatcherConfigBuilder};
use serial_test::serial;
use std::path::Path;
use tempfile::TempDir;

fn reset_global_broker() {
    *global_resource_broker()
        .write()
        .expect("resource broker lock should not be poisoned") = ResourceBroker::default();
}

/// Test 1: Verify watcher configuration defaults
#[test]
fn test_watcher_config_defaults() {
    let config = WatcherConfig::default();

    // Check debounce default
    assert_eq!(config.debounce_ms, 100);

    // Check default extensions
    assert!(config.extensions.contains(&"js".to_string()));
    assert!(config.extensions.contains(&"ts".to_string()));
    assert!(config.extensions.contains(&"tsx".to_string()));
    assert!(config.extensions.contains(&"jsx".to_string()));

    // Check default ignore dirs
    assert!(config.ignore_dirs.contains(&"node_modules".to_string()));
    assert!(config.ignore_dirs.contains(&".git".to_string()));

    // Check other defaults
    assert!(config.recursive);
    assert!(config.clear_console);
    assert!(config.show_notifications);
}

/// Test 2: Verify config builder pattern
#[test]
fn test_watcher_config_builder() {
    let config = WatcherConfigBuilder::new()
        .debounce_ms(200)
        .add_extension("vue")
        .add_extension("svelte")
        .add_ignore_dir("vendor")
        .recursive(false)
        .clear_console(false)
        .show_notifications(false)
        .build();

    assert_eq!(config.debounce_ms, 200);
    assert!(config.extensions.contains(&"vue".to_string()));
    assert!(config.extensions.contains(&"svelte".to_string()));
    assert!(config.ignore_dirs.contains(&"vendor".to_string()));
    assert!(!config.recursive);
    assert!(!config.clear_console);
    assert!(!config.show_notifications);
}

/// Test 3: Test file filtering by extension
#[test]
fn test_should_watch_by_extension() {
    let reloader = HotReloader::new();

    // Should watch JavaScript/TypeScript files
    assert!(reloader.should_watch(Path::new("app.js")));
    assert!(reloader.should_watch(Path::new("index.ts")));
    assert!(reloader.should_watch(Path::new("Component.tsx")));
    assert!(reloader.should_watch(Path::new("Button.jsx")));
    assert!(reloader.should_watch(Path::new("utils.mjs")));
    assert!(reloader.should_watch(Path::new("config.cjs")));

    // Should NOT watch non-JS files
    assert!(!reloader.should_watch(Path::new("styles.css")));
    assert!(!reloader.should_watch(Path::new("image.png")));
    assert!(!reloader.should_watch(Path::new("data.json")));
    assert!(!reloader.should_watch(Path::new("readme.md")));
    assert!(!reloader.should_watch(Path::new("main.rs")));
    assert!(!reloader.should_watch(Path::new("Cargo.toml")));
}

/// Test 4: Test file filtering by path (ignore directories)
#[test]
fn test_should_watch_ignore_dirs() {
    let reloader = HotReloader::new();

    // Should NOT watch files in ignored directories
    assert!(!reloader.should_watch(Path::new("node_modules/lodash/index.js")));
    assert!(!reloader.should_watch(Path::new(".git/hooks/pre-commit.js")));
    assert!(!reloader.should_watch(Path::new("dist/bundle.js")));
    assert!(!reloader.should_watch(Path::new("build/app.js")));
    assert!(!reloader.should_watch(Path::new(".beejs-cache/compiled.js")));

    // Should watch files in normal directories
    assert!(reloader.should_watch(Path::new("src/index.js")));
    assert!(reloader.should_watch(Path::new("lib/utils.ts")));
    assert!(reloader.should_watch(Path::new("tests/app.test.js")));
}

/// Test 5: Test watcher statistics tracking
#[test]
fn test_watcher_stats_tracking() {
    let reloader = HotReloader::new();

    // Initial stats should be zero
    let stats = reloader.get_stats();
    assert_eq!(stats.total_reloads, 0);
    assert_eq!(stats.successful_reloads, 0);
    assert_eq!(stats.failed_reloads, 0);

    // Record some successful reloads
    reloader.record_reload(true, 50);
    reloader.record_reload(true, 75);

    let stats = reloader.get_stats();
    assert_eq!(stats.total_reloads, 2);
    assert_eq!(stats.successful_reloads, 2);
    assert_eq!(stats.failed_reloads, 0);
    assert_eq!(stats.last_reload_time_ms, 75);

    // Record a failed reload
    reloader.record_reload(false, 100);

    let stats = reloader.get_stats();
    assert_eq!(stats.total_reloads, 3);
    assert_eq!(stats.successful_reloads, 2);
    assert_eq!(stats.failed_reloads, 1);
    assert_eq!(stats.last_reload_time_ms, 100);
}

/// Test 6: Test custom extension configuration
#[test]
fn test_custom_extension_config() {
    let config = WatcherConfigBuilder::new()
        .extensions(vec!["vue".to_string(), "svelte".to_string()])
        .build();

    let reloader = HotReloader::with_config(config);

    // Should watch Vue/Svelte files
    assert!(reloader.should_watch(Path::new("App.vue")));
    assert!(reloader.should_watch(Path::new("Component.svelte")));

    // Should NOT watch JS/TS files (not in custom config)
    assert!(!reloader.should_watch(Path::new("app.js")));
    assert!(!reloader.should_watch(Path::new("index.ts")));
}

/// Test 7: Test HotReloader creation with defaults
#[test]
fn test_hot_reloader_creation() {
    let reloader = HotReloader::new();

    // Should not be running initially
    assert!(!reloader.is_running());

    // Stats should be empty
    let stats = reloader.get_stats();
    assert_eq!(stats.total_reloads, 0);
}

/// Test 8: Test file change type enum
#[test]
fn test_file_change_type() {
    let created = FileChangeType::Created;
    let modified = FileChangeType::Modified;
    let removed = FileChangeType::Removed;
    let renamed = FileChangeType::Renamed;

    // Test equality
    assert_eq!(created, FileChangeType::Created);
    assert_eq!(modified, FileChangeType::Modified);
    assert_ne!(created, modified);
    assert_ne!(removed, renamed);
}

/// Test 9: Test watcher stop functionality
#[test]
fn test_watcher_stop() {
    let reloader = HotReloader::new();

    // Initially not running
    assert!(!reloader.is_running());

    // Stop should be safe even when not running
    reloader.stop();
    assert!(!reloader.is_running());
}

#[test]
#[serial]
fn watch_uses_global_file_read_broker_before_scanning_or_starting() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("app.js"), "console.log('hot');").unwrap();
    let mut reloader = HotReloader::with_config(
        WatcherConfigBuilder::new()
            .show_notifications(false)
            .build(),
    );

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(temp_dir.path().to_path_buf()),
        );
    }

    let result = reloader.watch(temp_dir.path());
    reloader.stop();
    reset_global_broker();

    let error = result.expect_err("denied watch path must fail before starting watcher");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        !reloader.is_running(),
        "denied watch must not leave the watcher running"
    );
    assert_eq!(reloader.get_stats().files_watched, 0);
}

#[test]
#[serial]
fn watch_uses_global_file_read_broker_for_initial_file_count() {
    let temp_dir = TempDir::new().unwrap();
    let watched_file = temp_dir.path().join("app.js");
    std::fs::write(&watched_file, "console.log('hot');").unwrap();
    let mut reloader = HotReloader::with_config(
        WatcherConfigBuilder::new()
            .show_notifications(false)
            .build(),
    );

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(watched_file.clone()),
        );
    }

    let result = reloader.watch(temp_dir.path());
    reloader.stop();
    reset_global_broker();

    let error = result.expect_err("denied watched file must fail before counting files");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        !reloader.is_running(),
        "denied initial scan must not leave the watcher running"
    );
    assert_eq!(reloader.get_stats().files_watched, 0);
}
