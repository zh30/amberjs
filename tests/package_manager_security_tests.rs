use amberjs::package_manager::{PackageJson, PackageManager, PackageManagerConfig};
use amberjs::permissions::{
    global_resource_broker, PermissionAction, PermissionKind, ResourceBroker, ResourceId,
};
use flate2::write::GzEncoder;
use flate2::Compression;
use serial_test::serial;
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tar::{Builder, EntryType, Header};
use tempfile::TempDir;

fn package_manager(temp_dir: &TempDir) -> PackageManager {
    PackageManager::new(PackageManagerConfig {
        cache_dir: temp_dir.path().join("cache"),
        node_modules_dir: temp_dir.path().join("node_modules"),
        ..Default::default()
    })
    .unwrap()
}

fn reset_global_broker() {
    *global_resource_broker()
        .write()
        .expect("resource broker lock should not be poisoned") = ResourceBroker::default();
}

fn empty_package_json() -> PackageJson {
    PackageJson {
        name: "fixture".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        main: None,
        scripts: None,
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        author: None,
        license: None,
        repository: None,
    }
}

fn finish_archive(builder: Builder<GzEncoder<fs::File>>) {
    let encoder = builder.into_inner().unwrap();
    encoder.finish().unwrap();
}

fn append_file(builder: &mut Builder<GzEncoder<fs::File>>, path: &str, content: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Regular);
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, path, Cursor::new(content))
        .unwrap();
}

fn append_raw_path_file(builder: &mut Builder<GzEncoder<fs::File>>, path: &str, content: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Regular);
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    let header_bytes = header.as_mut_bytes();
    header_bytes[..path.len()].copy_from_slice(path.as_bytes());
    header.set_cksum();
    builder.append(&header, Cursor::new(content)).unwrap();
}

fn append_link(
    builder: &mut Builder<GzEncoder<fs::File>>,
    entry_type: EntryType,
    path: &str,
    target: &Path,
) {
    let mut header = Header::new_gnu();
    header.set_entry_type(entry_type);
    header.set_size(0);
    header.set_mode(0o777);
    header.set_cksum();
    builder.append_link(&mut header, path, target).unwrap();
}

fn append_fifo(builder: &mut Builder<GzEncoder<fs::File>>, path: &str) {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Fifo);
    header.set_size(0);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, path, std::io::empty())
        .unwrap();
}

fn create_tgz<F>(path: &Path, write_entries: F)
where
    F: FnOnce(&mut Builder<GzEncoder<fs::File>>),
{
    let file = fs::File::create(path).unwrap();
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);
    write_entries(&mut builder);
    builder.finish().unwrap();
    finish_archive(builder);
}

fn file_url(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn sha1_shasum(path: &Path) -> String {
    use sha1::Digest as _;

    let bytes = fs::read(path).unwrap();
    hex::encode(sha1::Sha1::digest(&bytes))
}

fn sha512_integrity(path: &Path) -> String {
    use base64::Engine as _;
    use sha2::Digest as _;

    let bytes = fs::read(path).unwrap();
    let digest = sha2::Sha512::digest(&bytes);
    format!(
        "sha512-{}",
        base64::engine::general_purpose::STANDARD.encode(digest)
    )
}

struct CurrentDirGuard {
    original: std::path::PathBuf,
}

impl CurrentDirGuard {
    fn switch_to(path: &Path) -> Self {
        let original = std::env::current_dir().expect("failed to read current directory");
        std::env::set_current_dir(path).expect("failed to switch current directory");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

#[test]
#[serial]
fn package_manager_new_respects_cache_and_node_modules_write_permissions() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    let node_modules_dir = temp_dir.path().join("node_modules");

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(cache_dir.clone()),
        );
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(node_modules_dir.clone()),
        );
    }

    let result = PackageManager::new(PackageManagerConfig {
        cache_dir: cache_dir.clone(),
        node_modules_dir: node_modules_dir.clone(),
        ..Default::default()
    });

    reset_global_broker();

    let error = match result {
        Ok(_) => panic!("denied package manager dirs must fail construction"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        !cache_dir.exists(),
        "denied cache dir creation must not write {}",
        cache_dir.display()
    );
    assert!(
        !node_modules_dir.exists(),
        "denied node_modules dir creation must not write {}",
        node_modules_dir.display()
    );
}

#[test]
#[serial]
fn fetch_package_info_respects_network_connect_permission() {
    let temp_dir = TempDir::new().unwrap();
    let registry_dir = temp_dir.path().join("registry");
    fs::create_dir_all(&registry_dir).unwrap();
    fs::write(registry_dir.join("pkg"), r#"{"versions":{}}"#).unwrap();
    let registry_url = file_url(&registry_dir);
    let package_url = format!("{}/pkg", registry_url.trim_end_matches('/'));
    let pm = PackageManager::new(PackageManagerConfig {
        registry_url,
        cache_dir: temp_dir.path().join("cache"),
        node_modules_dir: temp_dir.path().join("node_modules"),
        ..Default::default()
    })
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Url(package_url),
        );
    }

    let result = pm.fetch_package_info("pkg");

    reset_global_broker();

    let error = result.expect_err("denied network fetch must fail before curl");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
}

#[test]
#[serial]
fn fetch_package_info_respects_process_execute_permission_for_curl() {
    let temp_dir = TempDir::new().unwrap();
    let registry_dir = temp_dir.path().join("registry");
    fs::create_dir_all(&registry_dir).unwrap();
    fs::write(registry_dir.join("pkg"), r#"{"versions":{}}"#).unwrap();
    let pm = PackageManager::new(PackageManagerConfig {
        registry_url: file_url(&registry_dir),
        cache_dir: temp_dir.path().join("cache"),
        node_modules_dir: temp_dir.path().join("node_modules"),
        ..Default::default()
    })
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Name("curl".to_string()),
        );
    }

    let result = pm.fetch_package_info("pkg");

    reset_global_broker();

    let error = result.expect_err("denied curl execution must fail before spawning curl");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
}

#[test]
#[serial]
fn parse_package_json_uses_global_file_read_broker() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let package_json_path = temp_dir.path().join("package.json");
    fs::write(
        &package_json_path,
        r#"{"name":"blocked-read","version":"1.0.0"}"#,
    )
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(package_json_path.clone()),
        );
    }

    let result = pm.parse_package_json(&package_json_path);

    reset_global_broker();

    let error = result.expect_err("denied package.json read must fail");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
}

#[test]
#[serial]
fn extract_package_uses_global_file_write_broker() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let archive_path = temp_dir.path().join("safe.tgz");
    let package_dir = temp_dir.path().join("node_modules").join("blocked");
    create_tgz(&archive_path, |builder| {
        append_file(builder, "package/index.js", b"module.exports = 1;\n");
    });

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(package_dir.clone()),
        );
    }

    let result = pm.extract_package(&archive_path, "blocked");

    reset_global_broker();

    let error = result.expect_err("denied package extraction write must fail");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        !package_dir.exists(),
        "denied extraction must not create {}",
        package_dir.display()
    );
}

#[test]
#[serial]
fn read_package_lock_uses_global_file_read_broker() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let lock_path = temp_dir
        .path()
        .join("node_modules")
        .join("package-lock.json");
    fs::write(
        &lock_path,
        r#"{"name":"blocked","version":"1.0.0","lockfileVersion":3}"#,
    )
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(lock_path.clone()),
        );
    }

    let result = pm.read_package_lock();

    reset_global_broker();

    let error = result.expect_err("denied package-lock read must fail");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
}

#[test]
#[serial]
fn generate_package_lock_uses_global_file_write_broker() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let lock_path = temp_dir.path().join("package-lock.json");

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(lock_path.clone()),
        );
    }

    let result = pm.generate_package_lock(&lock_path, "blocked", "1.0.0");

    reset_global_broker();

    let error = result.expect_err("denied package-lock write must fail");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        !lock_path.exists(),
        "denied package-lock generation must not create {}",
        lock_path.display()
    );
}

#[test]
#[serial]
fn get_installed_packages_uses_global_file_read_broker_before_scanning_node_modules() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let node_modules_dir = temp_dir.path().join("node_modules");
    let package_dir = node_modules_dir.join("installed");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"installed","version":"1.0.0"}"#,
    )
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(node_modules_dir.clone()),
        );
    }

    let result = pm.get_installed_packages();

    reset_global_broker();

    let error = result.expect_err("denied installed package scan must fail");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
}

#[test]
#[serial]
fn clean_cache_uses_global_file_write_broker_before_removing_cache() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let cache_dir = temp_dir.path().join("cache");
    let cached_file = cache_dir.join("kept.txt");
    fs::write(&cached_file, "cached").unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(cache_dir.clone()),
        );
    }

    let result = pm.clean_cache();

    reset_global_broker();

    let error = result.expect_err("denied cache clean must fail before deleting cache");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        cached_file.exists(),
        "denied cache clean must not delete {}",
        cached_file.display()
    );
}

#[test]
#[serial]
fn generate_package_lock_uses_global_file_read_broker_for_nested_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let package_dir = temp_dir.path().join("node_modules").join("parent");
    let nested_node_modules = package_dir.join("node_modules");
    let nested_package = nested_node_modules.join("child");
    let lock_path = temp_dir.path().join("package-lock.json");
    fs::create_dir_all(&nested_package).unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"parent","version":"1.0.0"}"#,
    )
    .unwrap();
    fs::write(
        nested_package.join("package.json"),
        r#"{"name":"child","version":"1.0.0"}"#,
    )
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(nested_node_modules.clone()),
        );
    }

    let result = pm.generate_package_lock(&lock_path, "fixture", "1.0.0");

    reset_global_broker();

    let error = result.expect_err("denied nested package scan must fail before lock write");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        !lock_path.exists(),
        "denied nested scan must not write {}",
        lock_path.display()
    );
}

#[test]
#[serial]
fn prune_uses_global_file_read_broker_before_scanning_node_modules() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let node_modules_dir = temp_dir.path().join("node_modules");
    let unused_package = node_modules_dir.join("unused");
    fs::create_dir_all(&unused_package).unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(node_modules_dir.clone()),
        );
    }

    let result = pm.prune(&empty_package_json());

    reset_global_broker();

    let error = result.expect_err("denied prune scan must fail before reading node_modules");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        unused_package.exists(),
        "denied prune scan must not remove {}",
        unused_package.display()
    );
}

#[test]
#[serial]
fn prune_uses_global_file_write_broker_before_removing_package() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let unused_package = temp_dir.path().join("node_modules").join("unused");
    fs::create_dir_all(&unused_package).unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(unused_package.clone()),
        );
    }

    let result = pm.prune(&empty_package_json());

    reset_global_broker();

    let error = result.expect_err("denied prune removal must fail before deleting package");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        unused_package.exists(),
        "denied prune removal must not delete {}",
        unused_package.display()
    );
}

#[test]
#[serial]
fn install_package_exact_checks_package_json_write_before_registry_access() {
    let temp_dir = TempDir::new().unwrap();
    let _cwd = CurrentDirGuard::switch_to(temp_dir.path());
    let package_json_path = temp_dir.path().join("package.json");
    fs::write(
        &package_json_path,
        r#"{"name":"exact-policy-fixture","version":"1.0.0","dependencies":{"pkg":"1.0.0"}}"#,
    )
    .unwrap();
    let original_package_json = fs::read_to_string(&package_json_path).unwrap();
    let pm = PackageManager::new(PackageManagerConfig {
        registry_url: file_url(&temp_dir.path().join("missing-registry")),
        cache_dir: temp_dir.path().join("cache"),
        node_modules_dir: temp_dir.path().join("node_modules"),
        ..Default::default()
    })
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(package_json_path.clone()),
        );
    }

    let result = pm.install_package_exact("pkg", "1.0.0");

    reset_global_broker();

    let error = result.expect_err("denied package.json write must fail before registry access");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert_eq!(
        fs::read_to_string(&package_json_path).unwrap(),
        original_package_json,
        "denied exact install must not rewrite package.json"
    );
    assert!(
        !temp_dir.path().join("node_modules").join("pkg").exists(),
        "denied exact install must not extract package before package.json write is allowed"
    );
}

#[test]
#[serial]
fn install_package_checks_target_write_before_download_side_effects() {
    let temp_dir = TempDir::new().unwrap();
    let registry_dir = temp_dir.path().join("registry");
    fs::create_dir_all(&registry_dir).unwrap();
    let archive_path = temp_dir.path().join("pkg.tgz");
    create_tgz(&archive_path, |builder| {
        append_file(builder, "package/index.js", b"module.exports = 1;\n");
    });

    fs::write(
        registry_dir.join("pkg"),
        format!(
            r#"{{
                "versions": {{
                    "1.0.0": {{
                        "dist": {{
                            "tarball": "{}",
                            "shasum": "{}"
                        }}
                    }}
                }}
            }}"#,
            file_url(&archive_path),
            sha1_shasum(&archive_path)
        ),
    )
    .unwrap();

    let cache_tarball = temp_dir.path().join("cache").join("pkg").join("1.0.0.tgz");
    let package_dir = temp_dir.path().join("node_modules").join("pkg");
    let pm = PackageManager::new(PackageManagerConfig {
        registry_url: file_url(&registry_dir),
        cache_dir: temp_dir.path().join("cache"),
        node_modules_dir: temp_dir.path().join("node_modules"),
        ..Default::default()
    })
    .unwrap();

    {
        let mut broker = global_resource_broker()
            .write()
            .expect("resource broker lock should not be poisoned");
        *broker = ResourceBroker::default();
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(package_dir.clone()),
        );
    }

    let result = pm.install_package("pkg", "1.0.0");

    reset_global_broker();

    let error = result.expect_err("denied target write must fail before download");
    assert!(
        error.to_string().contains("permission denied"),
        "expected permission denied, got: {error}"
    );
    assert!(
        !cache_tarball.exists(),
        "denied target write must not download package into cache before failing"
    );
    assert!(
        !package_dir.exists(),
        "denied target write must not create {}",
        package_dir.display()
    );
}

#[test]
fn parses_npm_camel_case_dependency_sections() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let package_json_path = temp_dir.path().join("package.json");

    fs::write(
        &package_json_path,
        r#"{
            "name": "camel-case-package",
            "version": "1.0.0",
            "devDependencies": {
                "typescript": "^5.0.0"
            },
            "peerDependencies": {
                "react": "^18.0.0"
            },
            "optionalDependencies": {
                "fsevents": "^2.3.0"
            }
        }"#,
    )
    .unwrap();

    let package = pm.parse_package_json(&package_json_path).unwrap();

    assert_eq!(
        package.dev_dependencies.unwrap().get("typescript"),
        Some(&"^5.0.0".to_string())
    );
    assert_eq!(
        package.peer_dependencies.unwrap().get("react"),
        Some(&"^18.0.0".to_string())
    );
    assert_eq!(
        package.optional_dependencies.unwrap().get("fsevents"),
        Some(&"^2.3.0".to_string())
    );
}

#[test]
fn extract_package_rejects_parent_directory_entries_without_writing_outside_package() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let archive_path = temp_dir.path().join("parent-escape.tgz");
    let outside_path = temp_dir.path().join("outside.txt");

    create_tgz(&archive_path, |builder| {
        append_raw_path_file(builder, "package/../../outside.txt", b"escaped");
    });

    let result = pm.extract_package(&archive_path, "evil");

    assert!(result.is_err(), "parent-directory entry must be rejected");
    assert!(
        !outside_path.exists(),
        "malicious archive wrote outside package dir at {}",
        outside_path.display()
    );
}

#[test]
fn extract_package_cleans_partial_package_when_late_entry_is_unsafe() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let archive_path = temp_dir.path().join("late-escape.tgz");
    let package_dir = temp_dir.path().join("node_modules").join("evil");
    let outside_path = temp_dir.path().join("outside.txt");

    create_tgz(&archive_path, |builder| {
        append_file(builder, "package/index.js", b"module.exports = 1;\n");
        append_raw_path_file(builder, "package/../../outside.txt", b"escaped");
    });

    let result = pm.extract_package(&archive_path, "evil");

    assert!(result.is_err(), "late unsafe entry must reject archive");
    assert!(
        !outside_path.exists(),
        "malicious archive wrote outside package dir at {}",
        outside_path.display()
    );
    assert!(
        !package_dir.exists(),
        "failed extraction must not leave a partial package at {}",
        package_dir.display()
    );
}

#[test]
fn extract_package_rejects_absolute_path_entries() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let archive_path = temp_dir.path().join("absolute.tgz");

    create_tgz(&archive_path, |builder| {
        append_raw_path_file(builder, "/package/absolute.txt", b"absolute");
    });

    let result = pm.extract_package(&archive_path, "evil");

    assert!(result.is_err(), "absolute tar entry must be rejected");
}

#[test]
fn extract_package_rejects_symlink_escape_entries() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let archive_path = temp_dir.path().join("symlink-escape.tgz");
    let link_path = temp_dir
        .path()
        .join("node_modules")
        .join("evil")
        .join("link");

    create_tgz(&archive_path, |builder| {
        append_link(
            builder,
            EntryType::Symlink,
            "package/link",
            Path::new("../../outside.txt"),
        );
    });

    let result = pm.extract_package(&archive_path, "evil");

    assert!(result.is_err(), "escaping symlink must be rejected");
    assert!(
        fs::symlink_metadata(&link_path).is_err(),
        "escaping symlink should not be materialized"
    );
}

#[test]
fn extract_package_rejects_hardlink_escape_entries() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let archive_path = temp_dir.path().join("hardlink-escape.tgz");
    let outside_source = temp_dir.path().join("outside-source.txt");
    let hardlink_path = temp_dir
        .path()
        .join("node_modules")
        .join("evil")
        .join("hardlink");
    fs::write(&outside_source, "outside").unwrap();

    create_tgz(&archive_path, |builder| {
        append_link(
            builder,
            EntryType::Link,
            "package/hardlink",
            outside_source.as_path(),
        );
    });

    let result = pm.extract_package(&archive_path, "evil");

    assert!(result.is_err(), "escaping hardlink must be rejected");
    assert!(
        !hardlink_path.exists(),
        "escaping hardlink should not be materialized"
    );
}

#[test]
fn extract_package_rejects_special_file_entries() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);
    let archive_path = temp_dir.path().join("special-file.tgz");
    let fifo_path = temp_dir
        .path()
        .join("node_modules")
        .join("evil")
        .join("fifo");

    create_tgz(&archive_path, |builder| {
        append_fifo(builder, "package/fifo");
    });

    let result = pm.extract_package(&archive_path, "evil");

    assert!(result.is_err(), "special file entry must be rejected");
    assert!(
        !fifo_path.exists(),
        "special file should not be materialized"
    );
}

#[test]
#[serial]
fn download_package_rejects_integrity_mismatch_and_does_not_keep_cache_file() {
    let temp_dir = TempDir::new().unwrap();
    let registry_dir = temp_dir.path().join("registry");
    fs::create_dir_all(&registry_dir).unwrap();
    let archive_path = temp_dir.path().join("pkg.tgz");
    create_tgz(&archive_path, |builder| {
        append_file(builder, "package/index.js", b"module.exports = 1;\n");
    });

    fs::write(
        registry_dir.join("pkg"),
        format!(
            r#"{{
                "versions": {{
                    "1.0.0": {{
                        "dist": {{
                            "tarball": "{}",
                            "integrity": "sha512-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=="
                        }}
                    }}
                }}
            }}"#,
            file_url(&archive_path)
        ),
    )
    .unwrap();

    let pm = PackageManager::new(PackageManagerConfig {
        registry_url: file_url(&registry_dir),
        cache_dir: temp_dir.path().join("cache"),
        node_modules_dir: temp_dir.path().join("node_modules"),
        ..Default::default()
    })
    .unwrap();

    let result = pm.download_package("pkg", "1.0.0");

    assert!(result.is_err(), "integrity mismatch must reject download");
    assert!(
        !temp_dir
            .path()
            .join("cache")
            .join("pkg")
            .join("1.0.0.tgz")
            .exists(),
        "mismatched tarball must not remain in cache"
    );
}

#[test]
#[serial]
fn download_package_rejects_registry_entry_without_integrity_or_shasum() {
    let temp_dir = TempDir::new().unwrap();
    let registry_dir = temp_dir.path().join("registry");
    fs::create_dir_all(&registry_dir).unwrap();
    let archive_path = temp_dir.path().join("pkg.tgz");
    create_tgz(&archive_path, |builder| {
        append_file(builder, "package/index.js", b"module.exports = 1;\n");
    });

    fs::write(
        registry_dir.join("pkg"),
        format!(
            r#"{{
                "versions": {{
                    "1.0.0": {{
                        "dist": {{
                            "tarball": "{}"
                        }}
                    }}
                }}
            }}"#,
            file_url(&archive_path)
        ),
    )
    .unwrap();

    let pm = PackageManager::new(PackageManagerConfig {
        registry_url: file_url(&registry_dir),
        cache_dir: temp_dir.path().join("cache"),
        node_modules_dir: temp_dir.path().join("node_modules"),
        ..Default::default()
    })
    .unwrap();

    let result = pm.download_package("pkg", "1.0.0");

    assert!(
        result.is_err(),
        "download without integrity or shasum must not be treated as trusted"
    );
}

#[test]
#[serial]
fn install_dependencies_rejects_registry_tarball_that_does_not_match_lock_integrity() {
    let temp_dir = TempDir::new().unwrap();
    let registry_dir = temp_dir.path().join("registry");
    fs::create_dir_all(&registry_dir).unwrap();

    let good_archive = temp_dir.path().join("good.tgz");
    create_tgz(&good_archive, |builder| {
        append_file(builder, "package/index.js", b"module.exports = 'good';\n");
    });
    let bad_archive = temp_dir.path().join("bad.tgz");
    create_tgz(&bad_archive, |builder| {
        append_file(builder, "package/index.js", b"module.exports = 'bad';\n");
    });

    fs::write(
        temp_dir.path().join("package-lock.json"),
        format!(
            r#"{{
                "name": "root",
                "version": "1.0.0",
                "lockfileVersion": 3,
                "requires": true,
                "dependencies": {{
                    "pkg": {{
                        "version": "1.0.0",
                        "resolved": "{}",
                        "integrity": "{}",
                        "dev": false
                    }}
                }}
            }}"#,
            file_url(&bad_archive),
            sha512_integrity(&good_archive)
        ),
    )
    .unwrap();

    fs::write(
        registry_dir.join("pkg"),
        format!(
            r#"{{
                "versions": {{
                    "1.0.0": {{
                        "dist": {{
                            "tarball": "{}",
                            "integrity": "{}"
                        }}
                    }}
                }}
            }}"#,
            file_url(&bad_archive),
            sha512_integrity(&bad_archive)
        ),
    )
    .unwrap();

    let pm = PackageManager::new(PackageManagerConfig {
        registry_url: file_url(&registry_dir),
        cache_dir: temp_dir.path().join("cache"),
        node_modules_dir: temp_dir.path().join("node_modules"),
        ..Default::default()
    })
    .unwrap();
    let mut dependencies = HashMap::new();
    dependencies.insert("pkg".to_string(), "1.0.0".to_string());
    let package_json = PackageJson {
        name: "root".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        main: None,
        scripts: None,
        dependencies: Some(dependencies),
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        author: None,
        license: None,
        repository: None,
    };

    let result = pm.install_dependencies(&package_json);

    let error = result.expect_err("lockfile integrity mismatch must reject install");
    let error = error.to_string();
    assert!(
        error.contains("lockfile") && error.contains("integrity"),
        "lockfile integrity error should be explicit, got: {error}"
    );
    assert!(
        !temp_dir.path().join("node_modules").join("pkg").exists(),
        "lockfile integrity mismatch must not leave installed package behind"
    );
}

#[test]
fn generate_lock_for_package_refuses_missing_integrity_material() {
    let temp_dir = TempDir::new().unwrap();
    let pm = package_manager(&temp_dir);

    let result = pm.generate_lock_for_package("pkg", "1.0.0");

    assert!(
        result.is_err(),
        "lock generation must not mark a package trusted without integrity"
    );
}

#[test]
#[serial]
fn install_dependencies_propagates_required_dependency_failure() {
    let temp_dir = TempDir::new().unwrap();
    let pm = PackageManager::new(PackageManagerConfig {
        registry_url: file_url(&temp_dir.path().join("empty-registry")),
        cache_dir: temp_dir.path().join("cache"),
        node_modules_dir: temp_dir.path().join("node_modules"),
        ..Default::default()
    })
    .unwrap();
    let mut dependencies = HashMap::new();
    dependencies.insert("missing".to_string(), "1.0.0".to_string());
    let package_json = PackageJson {
        name: "root".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        main: None,
        scripts: None,
        dependencies: Some(dependencies),
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        author: None,
        license: None,
        repository: None,
    };

    let result = pm.install_dependencies(&package_json);

    assert!(
        result.is_err(),
        "required dependency installation failures must propagate"
    );
}
