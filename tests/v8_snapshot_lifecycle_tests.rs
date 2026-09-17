// V8 快照生命周期、版本感知与自愈测试
use amberjs::v8_snapshot::*;
use serial_test::serial;
use std::fs;

#[test]
#[serial]
fn test_snapshot_name_contains_package_version() {
    let name = startup_snapshot_name();
    assert!(
        name.contains(env!("CARGO_PKG_VERSION")),
        "Snapshot name `{name}` must contain current CARGO_PKG_VERSION"
    );
    assert!(
        name.ends_with(".bin"),
        "Snapshot name must end with .bin extension"
    );
}

#[test]
#[serial]
fn test_snapshot_status_query() {
    let status = startup_blob_status();
    assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
    assert!(status.path.to_string_lossy().contains(status.version));
}

#[test]
#[serial]
fn test_snapshot_rebuild_and_clear_lifecycle() {
    // 1. Rebuild snapshot explicitly
    let size = rebuild_startup_blob().expect("Rebuild startup blob should succeed");
    assert!(size > 1000, "Rebuilt snapshot size must be non-trivial");

    let status_after_rebuild = startup_blob_status();
    assert!(
        status_after_rebuild.exists,
        "Snapshot file must exist after rebuild"
    );
    assert!(status_after_rebuild.size_bytes > 0);

    // 2. Clear snapshot cache
    let cleared = clear_startup_blob_cache().expect("Clear cache should succeed");
    assert!(cleared, "Clear should report true for existing file");

    let status_after_clear = startup_blob_status();
    assert!(
        !status_after_clear.exists,
        "Snapshot file must not exist after clear"
    );

    // 3. Second clear on non-existing file returns false
    let cleared_again = clear_startup_blob_cache().expect("Clear cache should succeed");
    assert!(
        !cleared_again,
        "Clear should return false if file was already gone"
    );
}

#[test]
#[serial]
fn test_snapshot_corrupted_file_self_heals() {
    let path = startup_blob_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Write corrupted garbage bytes to simulate dirty / corrupted cache
    fs::write(&path, b"CORRUPTED_GARBAGE_PAYLOAD_NOT_AMBER_HEADER").expect("Write corrupted file");
    assert!(path.exists());

    // Calling rebuild_startup_blob should overwrite with valid wrapped snapshot
    let size = rebuild_startup_blob().expect("Self healing rebuild should succeed");
    assert!(size > 0);

    let content = fs::read(&path).expect("Read healed file");
    assert!(
        content.starts_with(SNAPSHOT_MAGIC),
        "Healed snapshot file must have valid SNAPSHOT_MAGIC header"
    );

    // Clean up
    let _ = clear_startup_blob_cache();
}
