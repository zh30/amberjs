// V8 快照预热功能测试
// v0.3.232: 测试 warmup_builtins_internal 功能

use serial_test::serial;

fn reset_global_broker() {
    use amberjs::permissions::{global_resource_broker, ResourceBroker};

    *global_resource_broker()
        .write()
        .expect("resource broker lock should not be poisoned") = ResourceBroker::default();
}

struct BrokerResetGuard;

impl BrokerResetGuard {
    fn new() -> Self {
        reset_global_broker();
        Self
    }
}

impl Drop for BrokerResetGuard {
    fn drop(&mut self) {
        reset_global_broker();
    }
}

#[test]
#[serial]
fn test_snapshot_manager_warmup_stats() {
    use amberjs::v8_snapshot::{SnapshotConfig, SnapshotManager};

    let config = SnapshotConfig::default();
    let manager = SnapshotManager::new(config);

    // 获取统计信息
    let stats = manager.get_stats();

    // 初始状态：没有预热
    assert_eq!(stats.builtins_warmed, 0, "初始 builtin_warmed 应该为 0");
    assert_eq!(
        stats.snapshots_generated, 0,
        "初始 snapshots_generated 应该为 0"
    );
    assert_eq!(stats.snapshots_loaded, 0, "初始 snapshots_loaded 应该为 0");
}

#[test]
#[serial]
fn test_snapshot_manager_warmup_builtins() {
    use amberjs::v8_snapshot::{SnapshotConfig, SnapshotManager};

    let config = SnapshotConfig::default();
    let manager = SnapshotManager::new(config);

    // 执行内置对象预热
    let result = manager.warmup_builtins();
    assert!(result.is_ok(), "预热应该成功");

    // 验证统计更新
    let stats = manager.get_stats();
    assert_eq!(stats.builtins_warmed, 1, "预热后 builtins_warmed 应该为 1");
}

#[test]
#[serial]
fn test_snapshot_manager_creation() {
    use amberjs::v8_snapshot::{SnapshotConfig, SnapshotManager};

    let config = SnapshotConfig::default();
    let manager = SnapshotManager::new(config);

    // 验证管理器创建成功
    assert_eq!(manager.config.max_snapshots, 3);
    assert!(manager.config.builtin_warmup);
}

#[test]
#[serial]
fn test_snapshot_stats_hit_rate() {
    use amberjs::v8_snapshot::SnapshotStats;

    let stats = SnapshotStats::new();

    // 初始命中率应该是 0
    assert_eq!(stats.hit_rate(), 0.0);
    // hit_rate 测试通过 SnapshotStats 结构的默认值验证
}

#[test]
#[serial]
fn test_snapshot_config_default() {
    use amberjs::v8_snapshot::SnapshotConfig;

    let config = SnapshotConfig::default();

    assert_eq!(config.max_snapshots, 3);
    assert!(config.builtin_warmup);
    assert!(!config.enable_compression);
    assert_eq!(config.version, format!("v{}", env!("CARGO_PKG_VERSION")));
}

#[test]
#[serial]
fn test_snapshot_metadata() {
    use amberjs::v8_snapshot::SnapshotMetadata;
    use std::time::SystemTime;

    let metadata = SnapshotMetadata {
        version: "test-v1.0".to_string(),
        created_at: SystemTime::now(),
        size_bytes: 1024,
        is_compressed: false,
        builtin_warmup: true,
    };

    assert_eq!(metadata.version, "test-v1.0");
    assert_eq!(metadata.size_bytes, 1024);
    assert!(!metadata.is_compressed);
    assert!(metadata.builtin_warmup);
}

/// `generate_snapshot` must serialize a real V8 startup blob, not a marker
/// string labelled as a snapshot. The blob is fed back into `v8::Isolate` here
/// because that is the only assertion V8 itself has to agree with.
#[test]
#[serial]
fn test_generate_snapshot_produces_a_loadable_v8_startup_blob() {
    use amberjs::v8_snapshot::{SnapshotConfig, SnapshotManager};
    use rusty_v8 as v8;

    let manager = SnapshotManager::new(SnapshotConfig::default());
    let snapshot = manager
        .generate_snapshot()
        .expect("warmup context should serialize");

    assert!(snapshot.validate(), "generated snapshot should validate");
    assert!(
        !snapshot.snapshot_data.starts_with(b"BEEJS_WARMUP_V1\0"),
        "the blob must be V8 snapshot data, not a beejs marker"
    );

    amberjs::initialize_v8().unwrap();
    let startup_data = v8::StartupData::from(snapshot.snapshot_data.clone());
    let params = v8::Isolate::create_params().snapshot_blob(startup_data);
    let mut isolate = v8::Isolate::new(params);
    {
        v8::scope!(let scope, &mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let source = v8::String::new(scope, "1 + 1").unwrap();
        let script = v8::Script::compile(scope, source, None)
            .expect("an isolate restored from the snapshot should compile");
        let value = script.run(scope).expect("restored isolate should execute");
        assert_eq!(value.to_uint32(scope).unwrap().value(), 2);
    }

    let stats = manager.get_stats();
    assert_eq!(stats.snapshots_generated, 1);
}

#[test]
#[serial]
fn test_save_invalid_snapshot_rejects() {
    use amberjs::v8_snapshot::{SnapshotConfig, SnapshotManager, V8Snapshot};

    let dir = tempfile::tempdir().unwrap();
    let manager = SnapshotManager::new(SnapshotConfig::default());
    let snapshot = V8Snapshot::new(vec![], "invalid-empty".to_string(), false, true);

    let result = manager.save_snapshot_to_disk(&snapshot, dir.path());
    assert!(result.is_err(), "不能把空 snapshot 持久化为可信产物");
}

#[test]
#[serial]
fn snapshot_persistence_uses_global_file_broker() {
    use amberjs::permissions::{
        global_resource_broker, PermissionAction, PermissionKind, ResourceBroker, ResourceId,
    };
    use amberjs::v8_snapshot::{SnapshotConfig, SnapshotManager, V8Snapshot};

    let _guard = BrokerResetGuard::new();
    let dir = tempfile::tempdir().unwrap();
    let manager = SnapshotManager::new(SnapshotConfig::default());
    let snapshot = V8Snapshot::new(vec![1, 2, 3, 4], "broker-test".to_string(), false, true);
    let snapshot_dir = dir.path().join("snapshots");
    let snapshot_file = snapshot_dir.join("broker-test.bin");
    let metadata_file = snapshot_dir.join("broker-test.meta");

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Any,
        );
    }

    let denied_save = manager.save_snapshot_to_disk(&snapshot, dir.path());
    assert!(
        denied_save
            .unwrap_err()
            .to_string()
            .contains("permission denied"),
        "snapshot save must fail before writing when FileSystem/Write is denied"
    );
    assert!(
        !snapshot_dir.exists(),
        "denied save must not create the snapshots directory"
    );

    reset_global_broker();
    manager
        .save_snapshot_to_disk(&snapshot, dir.path())
        .unwrap();
    assert!(snapshot_file.exists());
    assert!(metadata_file.exists());

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Any,
        );
    }

    let denied_load = manager.load_snapshot_from_disk("broker-test", dir.path());
    assert!(
        denied_load
            .unwrap_err()
            .to_string()
            .contains("permission denied"),
        "snapshot load must fail before reading when FileSystem/Read is denied"
    );
    let denied_list = manager.list_persistent_snapshots(dir.path());
    assert!(
        denied_list
            .unwrap_err()
            .to_string()
            .contains("permission denied"),
        "snapshot listing must fail before scanning when FileSystem/Read is denied"
    );

    reset_global_broker();
    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Any,
        );
    }

    let denied_delete = manager.delete_persistent_snapshot("broker-test", dir.path());
    assert!(
        denied_delete
            .unwrap_err()
            .to_string()
            .contains("permission denied"),
        "snapshot delete must fail before removing files when FileSystem/Write is denied"
    );
    assert!(
        snapshot_file.exists() && metadata_file.exists(),
        "denied delete must leave snapshot files intact"
    );
}

#[test]
#[serial]
fn test_load_snapshot_not_found() {
    use amberjs::v8_snapshot::{SnapshotConfig, SnapshotManager};

    let config = SnapshotConfig::default();
    let manager = SnapshotManager::new(config);

    // 尝试加载不存在的快照
    let result = manager.load_snapshot("nonexistent");
    assert!(result.is_err(), "加载不存在的快照应该失败");
}
