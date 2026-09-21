// Amber Package Manager
// 高性能包管理器，支持 npm/yarn 兼容
//
// 主要功能：
// - package.json 解析和验证
// - npm registry 集成
// - 依赖解析和版本管理
// - 包下载和缓存
// - node_modules 结构管理

#[allow(unused)]
use anyhow::{anyhow, Result};
#[allow(unused)]
use flate2::read::GzDecoder;
#[allow(unused)]
use serde::{Deserialize, Serialize};
#[allow(unused)]
use std::collections::{HashMap, HashSet};
#[allow(unused)]
use std::fs;
#[allow(unused)]
use std::hash::Hash;
#[allow(unused)]
use std::io::Write;
#[allow(unused)]
use std::path::{Component, Path, PathBuf};
#[allow(unused)]
use std::process::Command;
#[allow(unused)]
use tar::Archive;
#[allow(unused)]
use tempfile::{NamedTempFile, TempDir};

#[allow(unused_imports)]
/// Package manager configuration
#[derive(Debug, Clone)]
pub struct PackageManagerConfig {
    pub registry_url: String,
    pub cache_dir: PathBuf,
    pub node_modules_dir: PathBuf,
    pub timeout_secs: u64,
}
impl Default for PackageManagerConfig {
    fn default() -> Self {
        Self {
            registry_url: "https://registry.npmjs.org/".to_string(),
            cache_dir: PathBuf::from(".amberjs_cache"),
            node_modules_dir: PathBuf::from("node_modules"),
            timeout_secs: 30,
        }
    }
}
/// Package.json structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub main: Option<String>,
    pub scripts: Option<HashMap<String, String>>,
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(
        rename = "devDependencies",
        alias = "dev_dependencies",
        skip_serializing_if = "Option::is_none"
    )]
    pub dev_dependencies: Option<HashMap<String, String>>,
    #[serde(
        rename = "peerDependencies",
        alias = "peer_dependencies",
        skip_serializing_if = "Option::is_none"
    )]
    pub peer_dependencies: Option<HashMap<String, String>>,
    #[serde(
        rename = "optionalDependencies",
        alias = "optional_dependencies",
        skip_serializing_if = "Option::is_none"
    )]
    pub optional_dependencies: Option<HashMap<String, String>>,
    #[serde(default)]
    pub author: Option<serde_json::Value>,
    pub license: Option<String>,
    #[serde(default)]
    pub repository: Option<serde_json::Value>,
}
/// Package information from registry
#[derive(Debug, Clone, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub dist: PackageDist,
    pub dependencies: Option<HashMap<String, String>>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct PackageDist {
    pub tarball: String,
    #[serde(default)]
    pub shasum: String,
    #[serde(default)]
    pub integrity: Option<String>,
}
/// Package version
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageVersion {
    pub name: String,
    pub version: String,
}
/// Package resolution result
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub package: PackageVersion,
    pub path: PathBuf,
    pub resolved: bool,
    pub integrity: Option<String>,
    pub tarball_url: Option<String>,
}

struct DownloadedPackage {
    path: PathBuf,
    integrity: Option<String>,
    tarball_url: String,
}

/// Parse `name`, `name@version`, `@scope/name`, or `@scope/name@version`.
pub fn parse_npm_package_spec(spec: &str) -> (String, String) {
    let spec = spec.trim();
    if spec.starts_with('@') {
        if let Some(slash) = spec.find('/') {
            let rest = &spec[slash + 1..];
            if let Some(at) = rest.find('@') {
                return (
                    spec[..slash + 1 + at].to_string(),
                    rest[at + 1..].to_string(),
                );
            }
        }
        return (spec.to_string(), "latest".to_string());
    }
    if let Some((name, version)) = spec.split_once('@') {
        if !name.is_empty() && !version.is_empty() {
            return (name.to_string(), version.to_string());
        }
    }
    (spec.to_string(), "latest".to_string())
}
/// High-performance package manager
pub struct PackageManager {
    config: PackageManagerConfig,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SemverTriple {
    major: u64,
    minor: u64,
    patch: u64,
}

/// Resolve caret range (^1.2.3 -> >=1.2.3 <2.0.0)
fn resolve_caret_range(versions: Vec<String>, base: &str) -> String {
    // Simple implementation - return latest compatible version
    let parsed: Vec<&str> = base.split('.').collect();
    if parsed.len() >= 1 {
        let major: u32 = parsed[0].parse().unwrap_or(0);
        let latest_major: Vec<String> = versions
            .iter()
            .filter(|v| {
                let parts: Vec<&str> = v.split('.').collect();
                parts.get(0).map(|p| p.parse::<u32>().unwrap_or(0)) == Some(major)
            })
            .cloned()
            .collect();
        latest_major
            .last()
            .map(|s| s.to_string())
            .unwrap_or_else(|| base.to_string())
    } else {
        base.to_string()
    }
}

/// Resolve tilde range (~1.2.3 -> >=1.2.3 <1.3.0)
fn resolve_tilde_range(versions: Vec<String>, base: &str) -> String {
    let parsed: Vec<&str> = base.split('.').collect();
    if parsed.len() >= 2 {
        let major: u32 = parsed[0].parse().unwrap_or(0);
        let minor: u32 = parsed[1].parse().unwrap_or(0);
        let latest: Vec<String> = versions
            .iter()
            .filter(|v| {
                let parts: Vec<&str> = v.split('.').collect();
                parts.get(0).map(|p| p.parse::<u32>().unwrap_or(0)) == Some(major)
                    && parts.get(1).map(|p| p.parse::<u32>().unwrap_or(0)) == Some(minor)
            })
            .cloned()
            .collect();
        latest
            .last()
            .map(|s| s.to_string())
            .unwrap_or_else(|| base.to_string())
    } else {
        base.to_string()
    }
}

/// Resolve greater than version
fn resolve_greater_than(versions: Vec<String>, min: &str) -> String {
    let min_parsed: Vec<u32> = min.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let latest: Vec<String> = versions
        .iter()
        .filter(|v| {
            let parts: Vec<u32> = v.split('.').map(|p| p.parse().unwrap_or(0)).collect();
            parts >= min_parsed
        })
        .cloned()
        .collect();
    latest
        .last()
        .map(|s| s.to_string())
        .unwrap_or_else(|| min.to_string())
}

/// Resolve less than version
fn resolve_less_than(versions: Vec<String>, max: &str) -> String {
    let max_parsed: Vec<u32> = max
        .split('.')
        .map(|p| p.parse().unwrap_or(u32::MAX))
        .collect();
    let latest: Vec<String> = versions
        .iter()
        .filter(|v| {
            let parts: Vec<u32> = v
                .split('.')
                .map(|p| p.parse().unwrap_or(u32::MAX))
                .collect();
            parts <= max_parsed
        })
        .cloned()
        .collect();
    latest
        .last()
        .map(|s| s.to_string())
        .unwrap_or_else(|| max.to_string())
}

fn dependency_request_matches_locked_version(requested: &str, locked: &str) -> bool {
    let requested = requested.trim();
    let locked = locked.trim();
    if requested == "*" || requested.eq_ignore_ascii_case("latest") {
        return true;
    }

    if requested == locked {
        return true;
    }

    if let Some(base) = requested.strip_prefix('^') {
        return semver_caret_matches(base, locked);
    }
    if let Some(base) = requested.strip_prefix('~') {
        return semver_tilde_matches(base, locked);
    }
    if let Some(base) = requested.strip_prefix('=') {
        return locked == base.trim();
    }

    false
}

fn semver_caret_matches(base: &str, locked: &str) -> bool {
    let Some(base) = parse_semver_triplet(base) else {
        return false;
    };
    let Some(locked) = parse_semver_triplet(locked) else {
        return false;
    };

    if locked < base {
        return false;
    }
    if base.major > 0 {
        return locked.major == base.major;
    }
    if base.minor > 0 {
        return locked.major == 0 && locked.minor == base.minor;
    }
    locked.major == 0 && locked.minor == 0 && locked.patch == base.patch
}

fn semver_tilde_matches(base: &str, locked: &str) -> bool {
    let Some(base) = parse_semver_triplet(base) else {
        return false;
    };
    let Some(locked) = parse_semver_triplet(locked) else {
        return false;
    };

    locked >= base && locked.major == base.major && locked.minor == base.minor
}

fn parse_semver_triplet(version: &str) -> Option<SemverTriple> {
    let core = version
        .trim()
        .trim_start_matches('v')
        .split(['-', '+'])
        .next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some(SemverTriple {
        major,
        minor,
        patch,
    })
}

fn validate_package_archive_path(path: &Path, entry_type: tar::EntryType) -> Result<PathBuf> {
    if path.is_absolute() {
        return Err(anyhow!(
            "Unsafe package archive entry uses absolute path: {}",
            path.display()
        ));
    }

    if !(entry_type.is_file() || entry_type.is_dir()) {
        return Err(anyhow!(
            "Unsupported package archive entry type {:?} for {}",
            entry_type,
            path.display()
        ));
    }

    let mut components = path.components();
    match components.next() {
        Some(Component::Normal(prefix)) if prefix == std::ffi::OsStr::new("package") => {}
        _ => {
            return Err(anyhow!(
                "Package archive entry is outside package/ prefix: {}",
                path.display()
            ));
        }
    }

    let mut relative_path = PathBuf::new();
    for component in components {
        match component {
            Component::Normal(part) => relative_path.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!(
                    "Unsafe package archive entry path escapes package directory: {}",
                    path.display()
                ));
            }
        }
    }

    Ok(relative_path)
}

fn verify_package_tarball(
    tarball_path: &Path,
    integrity: Option<&str>,
    shasum: Option<&str>,
) -> Result<()> {
    if let Some(integrity) = integrity {
        if !integrity.trim().is_empty() {
            return verify_sri_integrity(tarball_path, integrity);
        }
    }

    if let Some(shasum) = shasum {
        if !shasum.trim().is_empty() {
            return verify_sha1_shasum(tarball_path, shasum);
        }
    }

    Err(anyhow!(
        "Package metadata missing integrity or shasum; refusing untrusted tarball"
    ))
}

fn verify_sri_integrity(tarball_path: &Path, integrity: &str) -> Result<()> {
    use base64::Engine as _;
    use sha2::Digest as _;

    let bytes = fs::read(tarball_path)
        .map_err(|e| anyhow!("Failed to read tarball for integrity verification: {}", e))?;
    let mut saw_supported_algorithm = false;

    for token in integrity.split_whitespace() {
        let Some((algorithm, expected_b64)) = token.split_once('-') else {
            continue;
        };

        let actual = match algorithm {
            "sha512" => {
                saw_supported_algorithm = true;
                sha2::Sha512::digest(&bytes).to_vec()
            }
            "sha384" => {
                saw_supported_algorithm = true;
                sha2::Sha384::digest(&bytes).to_vec()
            }
            "sha256" => {
                saw_supported_algorithm = true;
                sha2::Sha256::digest(&bytes).to_vec()
            }
            "sha1" => {
                saw_supported_algorithm = true;
                sha1::Sha1::digest(&bytes).to_vec()
            }
            _ => continue,
        };

        let expected = base64::engine::general_purpose::STANDARD
            .decode(expected_b64)
            .map_err(|e| anyhow!("Failed to decode package integrity: {}", e))?;

        if actual == expected {
            return Ok(());
        }
    }

    if saw_supported_algorithm {
        Err(anyhow!("Package integrity mismatch for {:?}", tarball_path))
    } else {
        Err(anyhow!(
            "Unsupported package integrity algorithm(s): {}",
            integrity
        ))
    }
}

fn verify_sha1_shasum(tarball_path: &Path, shasum: &str) -> Result<()> {
    use sha2::Digest as _;

    let bytes = fs::read(tarball_path)
        .map_err(|e| anyhow!("Failed to read tarball for shasum verification: {}", e))?;
    let actual = hex::encode(sha1::Sha1::digest(&bytes));

    if actual.eq_ignore_ascii_case(shasum.trim()) {
        Ok(())
    } else {
        Err(anyhow!("Package shasum mismatch for {:?}", tarball_path))
    }
}

fn validate_locked_dependency_dist(
    name: &str,
    version: &str,
    locked: Option<&LockedDependency>,
    registry_tarball_url: &str,
    registry_integrity: Option<&str>,
) -> Result<()> {
    let Some(locked) = locked else {
        return Ok(());
    };

    if locked.version.trim() != version {
        return Err(anyhow!(
            "package-lock.json version mismatch for package '{}': install resolved '{}' but lockfile pins '{}'",
            name,
            version,
            locked.version
        ));
    }

    if let Some(locked_resolved) = locked
        .resolved
        .as_deref()
        .filter(|resolved| !resolved.trim().is_empty())
    {
        if locked_resolved.trim() != registry_tarball_url {
            return Err(anyhow!(
                "package-lock.json resolved mismatch for package '{}': lockfile pins '{}' but registry metadata reports '{}'",
                name,
                locked_resolved,
                registry_tarball_url
            ));
        }
    }

    if let Some(locked_integrity) = locked
        .integrity
        .as_deref()
        .filter(|integrity| !integrity.trim().is_empty())
    {
        if let Some(registry_integrity) =
            registry_integrity.filter(|integrity| !integrity.trim().is_empty())
        {
            if locked_integrity.trim() != registry_integrity.trim() {
                return Err(anyhow!(
                    "package-lock.json integrity mismatch for package '{}': lockfile pins '{}' but registry metadata reports '{}'",
                    name,
                    locked_integrity,
                    registry_integrity
                ));
            }
        }
    }

    Ok(())
}

fn check_fs_read_permission(path: &Path) -> Result<()> {
    crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::FileSystem,
        crate::permissions::PermissionAction::Read,
        crate::permissions::ResourceId::Path(path.to_path_buf()),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

fn check_fs_write_permission(path: &Path) -> Result<()> {
    crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::FileSystem,
        crate::permissions::PermissionAction::Write,
        crate::permissions::ResourceId::Path(path.to_path_buf()),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

fn check_network_connect_permission(url: &str) -> Result<()> {
    crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Network,
        crate::permissions::PermissionAction::Connect,
        crate::permissions::ResourceId::Url(url.to_string()),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

fn check_process_execute_permission(command: &str) -> Result<()> {
    crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Process,
        crate::permissions::PermissionAction::Execute,
        crate::permissions::ResourceId::Name(command.to_string()),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

impl PackageManager {
    /// Create a new package manager instance
    pub fn new(config: PackageManagerConfig) -> Result<Self> {
        check_fs_write_permission(&config.cache_dir)?;
        check_fs_write_permission(&config.node_modules_dir)?;

        // Create cache directory if it doesn't exist
        if !config.cache_dir.exists() {
            fs::create_dir_all(&config.cache_dir)
                .map_err(|e| anyhow!("Failed to create cache directory: {}", e))?;
        }
        // Create node_modules directory if it doesn't exist
        if !config.node_modules_dir.exists() {
            fs::create_dir_all(&config.node_modules_dir)
                .map_err(|e| anyhow!("Failed to create node_modules directory: {}", e))?;
        }
        Ok(PackageManager { config })
    }

    /// Fetch package information from npm registry
    pub fn fetch_package_info(&self, name: &str) -> Result<serde_json::Value> {
        let url = format!(
            "{}/{}",
            self.config.registry_url.trim_end_matches('/'),
            name
        );
        check_network_connect_permission(&url)?;
        check_process_execute_permission("curl")?;

        // Use curl to fetch package info
        let output = Command::new("curl")
            .args(&[
                "-sL",
                "--max-time",
                &self.config.timeout_secs.to_string(),
                &url,
            ])
            .output()
            .map_err(|e| anyhow!("Failed to execute curl: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to fetch package info: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response = String::from_utf8_lossy(&output.stdout);
        let info: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| anyhow!("Failed to parse package info: {}", e))?;

        Ok(info)
    }

    /// Download package tarball from npm registry
    pub fn download_package(&self, name: &str, version: &str) -> Result<PathBuf> {
        self.download_package_with_locked_dependency(name, version, None)
            .map(|downloaded| downloaded.path)
    }

    fn download_package_with_locked_dependency(
        &self,
        name: &str,
        version: &str,
        locked: Option<&LockedDependency>,
    ) -> Result<DownloadedPackage> {
        let cached_path = self
            .config
            .cache_dir
            .join(name)
            .join(format!("{}.tgz", version));

        // Fetch package info to get tarball URL
        let info = self.fetch_package_info(name)?;
        let versions = info.get("versions").ok_or(anyhow!("No versions found"))?;

        let version_info = versions.get(version).ok_or(anyhow!(
            "Version {} not found for package {}",
            version,
            name
        ))?;

        let dist = version_info
            .get("dist")
            .ok_or(anyhow!("No dist metadata found"))?;

        let tarball_url = dist
            .get("tarball")
            .and_then(|t| t.as_str())
            .ok_or(anyhow!("No tarball URL found"))?
            .to_string();
        check_network_connect_permission(&tarball_url)?;
        check_process_execute_permission("curl")?;

        let integrity = dist
            .get("integrity")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string());
        let shasum = dist
            .get("shasum")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        validate_locked_dependency_dist(name, version, locked, &tarball_url, integrity.as_deref())?;
        let locked_integrity = locked
            .and_then(|locked| locked.integrity.as_deref())
            .filter(|integrity| !integrity.trim().is_empty());
        let verification_integrity = locked_integrity.or(integrity.as_deref());
        let verification_shasum = if locked_integrity.is_some() {
            None
        } else {
            shasum.as_deref()
        };

        if cached_path.exists() {
            match verify_package_tarball(&cached_path, verification_integrity, verification_shasum)
            {
                Ok(()) => {
                    return Ok(DownloadedPackage {
                        path: cached_path,
                        integrity,
                        tarball_url,
                    })
                }
                Err(e) => {
                    let _ = fs::remove_file(&cached_path);
                    tracing::warn!(
                        "Discarded cached package {}@{} after verification failure: {}",
                        name,
                        version,
                        e
                    );
                }
            }
        }

        // Create cache directory
        let package_cache_dir = self.config.cache_dir.join(name);
        check_fs_write_permission(&package_cache_dir)?;
        if !package_cache_dir.exists() {
            fs::create_dir_all(&package_cache_dir)
                .map_err(|e| anyhow!("Failed to create cache directory: {}", e))?;
        }

        // Download tarball
        let tarball_path = package_cache_dir.join(format!("{}.tgz", version));
        check_fs_write_permission(&tarball_path)?;
        let output = Command::new("curl")
            .args(&[
                "-sL",
                "--max-time",
                &self.config.timeout_secs.to_string(),
                "-o",
                tarball_path.to_str().ok_or(anyhow!("Invalid path"))?,
                &tarball_url,
            ])
            .output()
            .map_err(|e| anyhow!("Failed to download tarball: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to download tarball: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        if let Err(e) =
            verify_package_tarball(&tarball_path, verification_integrity, verification_shasum)
        {
            let _ = fs::remove_file(&tarball_path);
            return Err(e);
        }

        Ok(DownloadedPackage {
            path: tarball_path,
            integrity,
            tarball_url,
        })
    }

    /// Extract tarball to node_modules
    pub fn extract_package(&self, tarball_path: &Path, package_name: &str) -> Result<PathBuf> {
        check_fs_read_permission(tarball_path)?;
        let target_dir = self.config.node_modules_dir.join(package_name);
        check_fs_write_permission(&target_dir)?;

        // Create parent directory
        if let Some(parent) = target_dir.parent() {
            check_fs_write_permission(parent)?;
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| anyhow!("Failed to create parent directory: {}", e))?;
            }
        }
        let target_parent = target_dir
            .parent()
            .ok_or_else(|| anyhow!("Package target has no parent: {}", target_dir.display()))?;
        let staging_root = tempfile::Builder::new()
            .prefix(".amberjs-package-")
            .tempdir_in(target_parent)
            .map_err(|e| anyhow!("Failed to create package staging directory: {}", e))?;
        let staging_dir = staging_root.path().join("package");
        fs::create_dir_all(&staging_dir)
            .map_err(|e| anyhow!("Failed to create package staging root: {}", e))?;

        // Extract tarball
        let tarball_file =
            fs::File::open(tarball_path).map_err(|e| anyhow!("Failed to open tarball: {}", e))?;
        let decoder = GzDecoder::new(tarball_file);
        let mut archive = Archive::new(decoder);

        for entry in archive
            .entries()
            .map_err(|e| anyhow!("Failed to read archive: {}", e))?
        {
            let mut entry = entry.map_err(|e| anyhow!("Failed to read entry: {}", e))?;
            let path = entry.path()?.into_owned();
            let entry_type = entry.header().entry_type();

            let stripped_path = validate_package_archive_path(&path, entry_type)?;

            let target_path = staging_dir.join(&stripped_path);

            if entry_type.is_dir() {
                fs::create_dir_all(&target_path)
                    .map_err(|e| anyhow!("Failed to create directory: {}", e))?;
            } else {
                if let Some(parent) = target_path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)
                            .map_err(|e| anyhow!("Failed to create parent: {}", e))?;
                    }
                }
                entry
                    .unpack(&target_path)
                    .map_err(|e| anyhow!("Failed to unpack entry: {}", e))?;
            }
        }

        // Only replace the installed package after every archive entry has been
        // validated and unpacked into the staging directory.
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)
                .map_err(|e| anyhow!("Failed to remove existing package: {}", e))?;
        }
        fs::rename(&staging_dir, &target_dir)
            .map_err(|e| anyhow!("Failed to move verified package into place: {}", e))?;

        Ok(target_dir)
    }

    /// Parse version range and return exact version
    pub fn resolve_version(&self, name: &str, version_range: &str) -> Result<String> {
        let info = self.fetch_package_info(name)?;

        // Handle "latest" special tag
        if version_range == "latest" {
            let dist_tags = info
                .get("dist-tags")
                .ok_or(anyhow!("No dist-tags found"))?
                .as_object()
                .ok_or(anyhow!("Invalid dist-tags format"))?;

            let latest_version = dist_tags
                .get("latest")
                .ok_or(anyhow!("No 'latest' tag found"))?
                .as_str()
                .ok_or(anyhow!("Invalid latest tag format"))?
                .to_string();

            return Ok(latest_version);
        }

        let versions = info
            .get("versions")
            .ok_or(anyhow!("No versions found"))?
            .as_object()
            .ok_or(anyhow!("Invalid versions format"))?;

        let all_versions: Vec<String> = versions.keys().cloned().collect();

        // Parse version range
        let exact_version = if version_range.starts_with('^') {
            // Caret range: ^1.2.3 -> >=1.2.3 <2.0.0
            let base = &version_range[1..];
            resolve_caret_range(all_versions, base)
        } else if version_range.starts_with('~') {
            // Tilde range: ~1.2.3 -> >=1.2.3 <1.3.0
            let base = &version_range[1..];
            resolve_tilde_range(all_versions, base)
        } else if version_range.starts_with(">=") {
            // Greater than or equal
            let min = &version_range[2..];
            resolve_greater_than(all_versions, min)
        } else if version_range.starts_with('>') {
            // Greater than
            let min = &version_range[1..];
            resolve_greater_than(all_versions, min)
        } else if version_range.starts_with("<=") {
            // Less than or equal
            let max = &version_range[2..];
            resolve_less_than(all_versions, max)
        } else if version_range.starts_with('<') {
            // Less than
            let max = &version_range[1..];
            resolve_less_than(all_versions, max)
        } else {
            // Exact version
            version_range.to_string()
        };

        Ok(exact_version)
    }

    /// Install a single package
    pub fn install_package(&self, name: &str, version_range: &str) -> Result<ResolutionResult> {
        self.install_package_with_locked_dependency(name, version_range, None)
    }

    fn install_package_with_locked_dependency(
        &self,
        name: &str,
        version_range: &str,
        locked: Option<&LockedDependency>,
    ) -> Result<ResolutionResult> {
        let mut seen = HashSet::new();
        self.install_package_recursive(name, version_range, locked, &mut seen)
    }

    fn install_package_recursive(
        &self,
        name: &str,
        version_range: &str,
        locked: Option<&LockedDependency>,
        seen: &mut HashSet<String>,
    ) -> Result<ResolutionResult> {
        if !seen.insert(name.to_string()) {
            return Ok(ResolutionResult {
                package: PackageVersion {
                    name: name.to_string(),
                    version: version_range.to_string(),
                },
                path: self.config.node_modules_dir.join(name),
                resolved: true,
                integrity: None,
                tarball_url: None,
            });
        }

        self.check_package_install_write_permissions(name)?;

        let version = if let Some(locked) = locked {
            if !dependency_request_matches_locked_version(version_range, &locked.version) {
                return Err(anyhow!(
                    "package-lock.json version mismatch for package '{}': package.json requests '{}' but lockfile pins '{}'",
                    name,
                    version_range,
                    locked.version
                ));
            }
            locked.version.clone()
        } else {
            self.resolve_version(name, version_range)?
        };

        // Download tarball
        let downloaded = self.download_package_with_locked_dependency(name, &version, locked)?;

        // Extract to node_modules
        let installed_path = self.extract_package(&downloaded.path, name)?;
        self.write_package_integrity_meta(
            &installed_path,
            downloaded.integrity.as_deref(),
            Some(&downloaded.tarball_url),
        )?;

        self.install_package_dependency_tree(&installed_path, seen)?;

        Ok(ResolutionResult {
            package: PackageVersion {
                name: name.to_string(),
                version,
            },
            path: installed_path,
            resolved: true,
            integrity: downloaded.integrity,
            tarball_url: Some(downloaded.tarball_url),
        })
    }

    fn write_package_integrity_meta(
        &self,
        package_dir: &Path,
        integrity: Option<&str>,
        tarball_url: Option<&str>,
    ) -> Result<()> {
        let meta_path = package_dir.join(".amberjs-integrity.json");
        check_fs_write_permission(&meta_path)?;
        let meta = serde_json::json!({
            "integrity": integrity,
            "resolved": tarball_url,
        });
        fs::write(
            &meta_path,
            serde_json::to_string_pretty(&meta)
                .map_err(|e| anyhow!("Failed to serialize integrity metadata: {}", e))?,
        )
        .map_err(|e| anyhow!("Failed to write integrity metadata: {}", e))?;
        Ok(())
    }

    fn read_package_integrity_meta(&self, package_dir: &Path) -> (Option<String>, Option<String>) {
        let meta_path = package_dir.join(".amberjs-integrity.json");
        let Ok(content) = fs::read_to_string(&meta_path) else {
            return (None, None);
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            return (None, None);
        };
        (
            value
                .get("integrity")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            value
                .get("resolved")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        )
    }

    fn install_package_dependency_tree(
        &self,
        package_dir: &Path,
        seen: &mut HashSet<String>,
    ) -> Result<()> {
        let package_json_path = package_dir.join("package.json");
        if !package_json_path.exists() {
            return Ok(());
        }
        let package = self.parse_package_json(&package_json_path)?;
        let Some(dependencies) = package.dependencies else {
            return Ok(());
        };
        for (dep_name, dep_range) in dependencies {
            self.install_package_recursive(&dep_name, &dep_range, None, seen)?;
        }
        Ok(())
    }
    /// Parse package.json file
    pub fn parse_package_json(&self, path: &Path) -> Result<PackageJson> {
        check_fs_read_permission(path)?;
        let content =
            fs::read_to_string(path).map_err(|e| anyhow!("Failed to read package.json: {}", e))?;
        let package: PackageJson = serde_json::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse package.json: {}", e))?;
        Ok(package)
    }
    /// Initialize a new package.json
    pub fn init_package_json(&self, name: &str, version: &str) -> Result<PackageJson> {
        let package: _ = PackageJson {
            name: name.to_string(),
            version: version.to_string(),
            description: None,
            main: Some("index.js".to_string()),
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            author: None,
            license: Some("MIT".to_string()),
            repository: None,
        };
        // Write package.json
        let path: _ = PathBuf::from("package.json");
        check_fs_write_permission(&path)?;
        let content: _ = serde_json::to_string_pretty(&package)
            .map_err(|e| anyhow!("Failed to serialize package.json: {}", e))?;
        fs::write(&path, content).map_err(|e| anyhow!("Failed to write package.json: {}", e))?;
        Ok(package)
    }
    /// Install dependencies from package.json (with actual npm registry download)
    pub fn install_dependencies(
        &self,
        package_json: &PackageJson,
    ) -> Result<Vec<ResolutionResult>> {
        let mut results = Vec::new();
        let package_lock = self.read_existing_package_lock()?;
        let locked_deps = package_lock
            .as_ref()
            .and_then(|lock| lock.dependencies.as_ref());

        // Install regular dependencies
        if let Some(deps) = &package_json.dependencies {
            for (name, version) in deps {
                let locked = locked_deps.and_then(|deps| deps.get(name));
                let resolution = self
                    .install_package_with_locked_dependency(name, version, locked)
                    .map_err(|e| anyhow!("Failed to install {}@{}: {}", name, version, e))?;
                results.push(resolution);
            }
        }
        // Install dev dependencies
        if let Some(deps) = &package_json.dev_dependencies {
            for (name, version) in deps {
                let locked = locked_deps.and_then(|deps| deps.get(name));
                let resolution = self
                    .install_package_with_locked_dependency(name, version, locked)
                    .map_err(|e| anyhow!("Failed to install dev {}@{}: {}", name, version, e))?;
                results.push(resolution);
            }
        }
        // Install optional dependencies
        if let Some(deps) = &package_json.optional_dependencies {
            for (name, version) in deps {
                let locked = locked_deps.and_then(|deps| deps.get(name));
                match self.install_package_with_locked_dependency(name, version, locked) {
                    Ok(resolution) => results.push(resolution),
                    Err(e) => {
                        tracing::debug!("Failed to install optional {}@{}: {}", name, version, e)
                    }
                }
            }
        }
        Ok(results)
    }
    /// Resolve a package to a specific version
    pub fn resolve_package(&self, name: &str, version: &str) -> Result<ResolutionResult> {
        // For now, implement basic resolution
        // In a full implementation, this would:
        // 1. Query npm registry
        // 2. Parse version range (^, ~, >, etc.)
        // 3. Resolve to exact version
        // 4. Check for conflicts
        let package_version: _ = PackageVersion {
            name: name.to_string(),
            version: version.to_string(),
        };
        let path: _ = self.config.node_modules_dir.join(name);
        Ok(ResolutionResult {
            package: package_version,
            path,
            resolved: true,
            integrity: None,
            tarball_url: None,
        })
    }
    /// Add a dependency
    pub fn add_dependency(
        &self,
        package_json: &mut PackageJson,
        name: &str,
        version: &str,
    ) -> Result<()> {
        if package_json.dependencies.is_none() {
            package_json.dependencies = Some(HashMap::new());
        }
        if let Some(deps) = &mut package_json.dependencies {
            deps.insert(name.to_string(), version.to_string());
        }
        Ok(())
    }
    /// Remove a dependency
    pub fn remove_dependency(&self, package_json: &mut PackageJson, name: &str) -> Result<()> {
        if let Some(deps) = &mut package_json.dependencies {
            deps.remove(name);
        }
        Ok(())
    }
    /// Get installed packages
    pub fn get_installed_packages(&self) -> Result<Vec<PackageVersion>> {
        let mut packages = Vec::new();
        if self.config.node_modules_dir.exists() {
            check_fs_read_permission(&self.config.node_modules_dir)?;
            for entry in fs::read_dir(&self.config.node_modules_dir)
                .map_err(|e| anyhow!("Failed to read node_modules: {}", e))?
            {
                let entry: _ =
                    entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
                let path: _ = entry.path();
                if path.is_dir() {
                    let _name: _ = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    // Check for package.json
                    let package_json_path: _ = path.join("package.json");
                    if package_json_path.exists() {
                        if let Ok(package) = self.parse_package_json(&package_json_path) {
                            packages.push(PackageVersion {
                                name: package.name,
                                version: package.version,
                            });
                        }
                    }
                }
            }
        }
        Ok(packages)
    }
    /// Clean cache
    pub fn clean_cache(&self) -> Result<()> {
        if self.config.cache_dir.exists() {
            check_fs_write_permission(&self.config.cache_dir)?;
            fs::remove_dir_all(&self.config.cache_dir)
                .map_err(|e| anyhow!("Failed to clean cache: {}", e))?;
            check_fs_write_permission(&self.config.cache_dir)?;
            fs::create_dir_all(&self.config.cache_dir)
                .map_err(|e| anyhow!("Failed to recreate cache directory: {}", e))?;
        }
        Ok(())
    }
    /// Get configuration
    pub fn config(&self) -> &PackageManagerConfig {
        &self.config
    }

    fn check_package_install_write_permissions(&self, package_name: &str) -> Result<()> {
        check_fs_write_permission(&self.config.node_modules_dir)?;

        let target_dir = self.config.node_modules_dir.join(package_name);
        if let Some(parent) = target_dir.parent() {
            check_fs_write_permission(parent)?;
        }
        check_fs_write_permission(&target_dir)?;

        Ok(())
    }
}

// ============================================================================
// Package-lock.json Support (v0.3.226)
// ============================================================================

/// Package-lock.json structure (npm lockfile v3 format)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageLock {
    pub name: String,
    pub version: String,
    #[serde(rename = "lockfileVersion", default)]
    pub lockfile_version: u32,
    #[serde(default)]
    pub requires: bool,
    #[serde(default)]
    pub dependencies: Option<HashMap<String, LockedDependency>>,
}

/// Locked dependency entry in package-lock.json
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LockedDependency {
    pub version: String,
    #[serde(default)]
    pub resolved: Option<String>,
    #[serde(default)]
    pub integrity: Option<String>,
    #[serde(default)]
    pub dev: Option<bool>,
    #[serde(default)]
    pub dependencies: Option<HashMap<String, LockedDependency>>,
}

/// Represents an installed package for lock file generation
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub resolved: Option<String>,
    pub integrity: Option<String>,
    pub dev: bool,
    pub dependencies: Vec<InstalledPackage>,
}

impl PackageManager {
    /// Read and parse existing package-lock.json
    pub fn read_package_lock(&self) -> Result<PackageLock> {
        let lock_path = self.package_lock_read_path()?;

        self.read_package_lock_at(&lock_path)
    }

    fn read_existing_package_lock(&self) -> Result<Option<PackageLock>> {
        for candidate in self.package_lock_read_candidates() {
            if candidate.exists() {
                return self.read_package_lock_at(&candidate).map(Some);
            }
        }

        Ok(None)
    }

    fn read_package_lock_at(&self, lock_path: &Path) -> Result<PackageLock> {
        check_fs_read_permission(lock_path)?;
        let content = fs::read_to_string(&lock_path)
            .map_err(|e| anyhow!("Failed to read package-lock.json: {}", e))?;

        let lock: PackageLock = serde_json::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse package-lock.json: {}", e))?;

        // Validate lockfile version
        if lock.lockfile_version < 2 || lock.lockfile_version > 3 {
            tracing::warn!(
                "Unsupported lockfile version: {}, expected 2 or 3",
                lock.lockfile_version
            );
        }

        Ok(lock)
    }

    fn package_lock_read_candidates(&self) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if self
            .config
            .node_modules_dir
            .file_name()
            .is_some_and(|name| name == "node_modules")
        {
            if let Some(project_dir) = self.config.node_modules_dir.parent() {
                candidates.push(project_dir.join("package-lock.json"));
            }
        }

        let legacy_path = self.config.node_modules_dir.join("package-lock.json");
        if !candidates.iter().any(|path| path == &legacy_path) {
            candidates.push(legacy_path);
        }

        candidates
    }

    fn package_lock_read_path(&self) -> Result<PathBuf> {
        let candidates = self.package_lock_read_candidates();

        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }

        let searched = candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Err(anyhow!("package-lock.json not found at {}", searched))
    }

    /// Generate package-lock.json from installed packages
    pub fn generate_package_lock(
        &self,
        lock_path: &Path,
        project_name: &str,
        project_version: &str,
    ) -> Result<()> {
        let mut dependencies = HashMap::new();
        check_fs_write_permission(lock_path)?;

        // Scan installed packages
        if self.config.node_modules_dir.exists() {
            check_fs_read_permission(&self.config.node_modules_dir)?;
            for entry in fs::read_dir(&self.config.node_modules_dir)
                .map_err(|e| anyhow!("Failed to read node_modules: {}", e))?
            {
                let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
                let path = entry.path();

                if path.is_dir()
                    && path.file_name().map(|n| n.to_str()) == Some(Some("node_modules"))
                {
                    continue; // Skip the node_modules directory itself
                }

                if path.is_dir() {
                    let scoped = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with('@'))
                        .unwrap_or(false);
                    if scoped {
                        for scoped_entry in fs::read_dir(&path)
                            .map_err(|e| anyhow!("Failed to read scoped node_modules: {}", e))?
                        {
                            let scoped_entry = scoped_entry.map_err(|e| {
                                anyhow!("Failed to read scoped directory entry: {}", e)
                            })?;
                            if let Some(pkg) = self.scan_installed_package(&scoped_entry.path())? {
                                dependencies.insert(
                                    pkg.name.clone(),
                                    LockedDependency {
                                        version: pkg.version.clone(),
                                        resolved: pkg.resolved.clone(),
                                        integrity: pkg.integrity.clone(),
                                        dev: Some(pkg.dev),
                                        dependencies: None,
                                    },
                                );
                            }
                        }
                        continue;
                    }
                    if let Some(pkg) = self.scan_installed_package(&path)? {
                        let nested_deps: HashMap<String, LockedDependency> = pkg
                            .dependencies
                            .iter()
                            .map(|d| {
                                (
                                    d.name.clone(),
                                    LockedDependency {
                                        version: d.version.clone(),
                                        resolved: d.resolved.clone(),
                                        integrity: d.integrity.clone(),
                                        dev: Some(d.dev),
                                        dependencies: None, // Simplified: no recursive nesting for now
                                    },
                                )
                            })
                            .collect();

                        dependencies.insert(
                            pkg.name.clone(),
                            LockedDependency {
                                version: pkg.version.clone(),
                                resolved: pkg.resolved.clone(),
                                integrity: pkg.integrity.clone(),
                                dev: Some(pkg.dev),
                                dependencies: Some(nested_deps),
                            },
                        );
                    }
                }
            }
        }

        let lock = PackageLock {
            name: project_name.to_string(),
            version: project_version.to_string(),
            lockfile_version: 3,
            requires: true,
            dependencies: Some(dependencies),
        };

        let content = serde_json::to_string_pretty(&lock)
            .map_err(|e| anyhow!("Failed to serialize package-lock.json: {}", e))?;

        fs::write(lock_path, content)
            .map_err(|e| anyhow!("Failed to write package-lock.json: {}", e))?;

        tracing::info!("Generated package-lock.json at {:?}", lock_path);
        Ok(())
    }

    /// Update existing package-lock.json with new dependencies
    pub fn update_package_lock(
        &self,
        lock_path: &Path,
        project_name: &str,
        project_version: &str,
        updated_deps: Vec<(String, LockedDependency)>,
    ) -> Result<()> {
        let mut lock = if lock_path.exists() {
            check_fs_read_permission(lock_path)?;
            let content = fs::read_to_string(lock_path)
                .map_err(|e| anyhow!("Failed to read package-lock.json: {}", e))?;
            serde_json::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse package-lock.json: {}", e))?
        } else {
            PackageLock {
                name: project_name.to_string(),
                version: project_version.to_string(),
                lockfile_version: 3,
                requires: true,
                dependencies: Some(HashMap::new()),
            }
        };

        // Update dependencies
        if lock.dependencies.is_none() {
            lock.dependencies = Some(HashMap::new());
        }
        let deps = lock.dependencies.as_mut().unwrap();

        for (name, dep) in updated_deps {
            deps.insert(name, dep);
        }

        let content = serde_json::to_string_pretty(&lock)
            .map_err(|e| anyhow!("Failed to serialize package-lock.json: {}", e))?;

        check_fs_write_permission(lock_path)?;
        fs::write(lock_path, content)
            .map_err(|e| anyhow!("Failed to write package-lock.json: {}", e))?;

        Ok(())
    }

    /// Scan an installed package and return its info
    fn scan_installed_package(&self, path: &Path) -> Result<Option<InstalledPackage>> {
        let package_json_path = path.join("package.json");
        if !package_json_path.exists() {
            return Ok(None);
        }

        check_fs_read_permission(&package_json_path)?;
        let content = fs::read_to_string(&package_json_path)
            .map_err(|e| anyhow!("Failed to read package.json: {}", e))?;

        let package: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse package.json: {}", e))?;

        let name = package["name"]
            .as_str()
            .ok_or(anyhow!("Package missing name field"))?
            .to_string();

        let version = package["version"]
            .as_str()
            .ok_or(anyhow!("Package {} missing version field", name))?
            .to_string();

        // Check if this is a dev dependency (would be in devDependencies of root)
        let is_dev = false; // Simplified - in full impl, check parent context

        // Collect nested dependencies
        let mut nested_deps = Vec::new();
        let nested_node_modules = path.join("node_modules");
        if nested_node_modules.exists() {
            check_fs_read_permission(&nested_node_modules)?;
            for entry in fs::read_dir(&nested_node_modules)
                .map_err(|e| anyhow!("Failed to read nested node_modules: {}", e))?
            {
                let entry =
                    entry.map_err(|e| anyhow!("Failed to read nested directory entry: {}", e))?;
                if let Some(pkg) = self.scan_installed_package(&entry.path())? {
                    nested_deps.push(pkg);
                }
            }
        }

        let (integrity, resolved) = self.read_package_integrity_meta(path);

        Ok(Some(InstalledPackage {
            name,
            version,
            resolved,
            integrity,
            dev: is_dev,
            dependencies: nested_deps,
        }))
    }

    /// Install a package with exact version (--save-exact behavior)
    pub fn install_package_exact(&self, name: &str, version: &str) -> Result<ResolutionResult> {
        let package_json_path = PathBuf::from("package.json");
        if package_json_path.exists() {
            check_fs_read_permission(&package_json_path)?;
            check_fs_write_permission(&package_json_path)?;
        }
        self.check_package_install_write_permissions(name)?;

        // Resolve to exact version first
        let exact_version = self.resolve_version(name, version)?;

        // Download and extract
        let tarball_path = self.download_package(name, &exact_version)?;
        self.extract_package(&tarball_path, name)?;

        // If package.json exists in current directory, update it with exact version
        if package_json_path.exists() {
            let mut package = self.parse_package_json(&package_json_path)?;

            // Update the dependency in the correct section
            let version_str = format!("{}", exact_version);

            if let Some(deps) = &mut package.dependencies {
                if deps.contains_key(name) {
                    deps.insert(name.to_string(), version_str.clone());
                }
            }
            if let Some(deps) = &mut package.dev_dependencies {
                if deps.contains_key(name) {
                    deps.insert(name.to_string(), version_str);
                }
            }

            // Write back with exact version
            let content = serde_json::to_string_pretty(&package)
                .map_err(|e| anyhow!("Failed to serialize package.json: {}", e))?;
            check_fs_write_permission(&package_json_path)?;
            fs::write(&package_json_path, content)
                .map_err(|e| anyhow!("Failed to write package.json: {}", e))?;
        }

        Ok(ResolutionResult {
            package: PackageVersion {
                name: name.to_string(),
                version: exact_version,
            },
            path: self.config.node_modules_dir.join(name),
            resolved: true,
            integrity: None,
            tarball_url: None,
        })
    }

    /// Generate a lock file for a single package (for bunx command)
    pub fn generate_lock_for_package(
        &self,
        package_name: &str,
        package_version: &str,
    ) -> Result<PackageLock> {
        Err(anyhow!(
            "Cannot generate trusted lock entry for {}@{} without registry integrity metadata",
            package_name,
            package_version
        ))
    }

    /// Generate a lock file for a single package once verified registry metadata is available.
    pub fn generate_lock_for_verified_package(
        &self,
        package_name: &str,
        package_version: &str,
        resolved: &str,
        integrity: &str,
    ) -> Result<PackageLock> {
        if integrity.trim().is_empty() {
            return Err(anyhow!(
                "Cannot generate trusted lock entry for {}@{} with empty integrity",
                package_name,
                package_version
            ));
        }

        let lock = PackageLock {
            name: format!("@amberjs/temp-{}", package_name),
            version: "0.0.0".to_string(),
            lockfile_version: 3,
            requires: true,
            dependencies: Some(
                vec![(
                    package_name.to_string(),
                    LockedDependency {
                        version: package_version.to_string(),
                        resolved: Some(resolved.to_string()),
                        integrity: Some(integrity.to_string()),
                        dev: Some(false),
                        dependencies: None,
                    },
                )]
                .into_iter()
                .collect(),
            ),
        };

        Ok(lock)
    }

    /// Prune unused dependencies from node_modules
    /// Removes packages that are not declared in package.json
    pub fn prune(&self, package_json: &PackageJson) -> Result<Vec<String>> {
        let mut removed = Vec::new();

        // Collect all declared dependencies (owned Strings for easier handling)
        let mut declared_deps: std::collections::HashSet<String> = std::collections::HashSet::new();

        if let Some(deps) = &package_json.dependencies {
            for name in deps.keys() {
                declared_deps.insert(name.clone());
            }
        }
        if let Some(deps) = &package_json.dev_dependencies {
            for name in deps.keys() {
                declared_deps.insert(name.clone());
            }
        }
        if let Some(deps) = &package_json.optional_dependencies {
            for name in deps.keys() {
                declared_deps.insert(name.clone());
            }
        }

        // Also add packages from package-lock.json if it exists
        if let Ok(lock) = self.read_package_lock() {
            if let Some(deps) = &lock.dependencies {
                for name in deps.keys() {
                    declared_deps.insert(name.clone());
                }
            }
        }

        // Scan node_modules and remove undeclared packages
        if self.config.node_modules_dir.exists() {
            check_fs_read_permission(&self.config.node_modules_dir)?;
            for entry in fs::read_dir(&self.config.node_modules_dir)
                .map_err(|e| anyhow!("Failed to read node_modules: {}", e))?
            {
                let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string());

                // Skip special directories
                if name == Some(".bin".to_string()) || name == Some(".cache".to_string()) {
                    continue;
                }

                // Check if package is in node_modules (not nested)
                if path.is_dir() {
                    if let Some(pkg_name) = &name {
                        // Check if this package is declared
                        if !declared_deps.contains(pkg_name.as_str()) {
                            // Also check if it's a nested dependency (inside @scope)
                            let is_scope = pkg_name.starts_with('@');
                            if !is_scope {
                                tracing::info!("Removing undeclared package: {}", pkg_name);

                                // Remove the package directory
                                check_fs_write_permission(&path)?;
                                fs::remove_dir_all(&path)
                                    .map_err(|e| anyhow!("Failed to remove {}: {}", pkg_name, e))?;

                                removed.push(pkg_name.clone());
                            }
                        }
                    }
                }
            }

            // Handle scoped packages (@org/pkg) - check if parent @org is declared
            check_fs_read_permission(&self.config.node_modules_dir)?;
            let mut org_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();
            for entry in fs::read_dir(&self.config.node_modules_dir)
                .map_err(|e| anyhow!("Failed to read node_modules: {}", e))?
            {
                let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
                let name = entry.file_name().to_str().map(|s| s.to_string());
                if let Some(name) = name {
                    if name.starts_with('@') && entry.path().is_dir() {
                        org_dirs.insert(name);
                    }
                }
            }

            for org in org_dirs {
                // Check if this org/pkg should exist
                let org_path = self.config.node_modules_dir.join(&org);
                let mut org_has_valid_pkgs = false;

                check_fs_read_permission(&org_path)?;
                for pkg_entry in fs::read_dir(&org_path)
                    .map_err(|e| anyhow!("Failed to read scoped package directory: {}", e))?
                {
                    let pkg_entry = pkg_entry
                        .map_err(|e| anyhow!("Failed to read scoped package entry: {}", e))?;
                    let pkg_path = pkg_entry.path();
                    if pkg_path.is_dir() {
                        let pkg_name = pkg_entry.file_name().to_str().unwrap_or("").to_string();
                        let full_name = format!("{}/{}", org, pkg_name);

                        if declared_deps.contains(&full_name) {
                            org_has_valid_pkgs = true;
                        } else {
                            // Remove this package
                            tracing::info!("Removing undeclared package: {}", full_name);
                            check_fs_write_permission(&pkg_path)?;
                            fs::remove_dir_all(&pkg_path)
                                .map_err(|e| anyhow!("Failed to remove {}: {}", full_name, e))?;
                            removed.push(full_name);
                        }
                    }
                }

                // Remove empty @org directory
                if !org_has_valid_pkgs {
                    check_fs_read_permission(&org_path)?;
                    let entries = fs::read_dir(&org_path)
                        .map_err(|e| anyhow!("Failed to read scoped package directory: {}", e))?;
                    if entries.count() == 0 {
                        tracing::info!("Removing empty scope directory: {}", org);
                        check_fs_write_permission(&org_path)?;
                        fs::remove_dir(&org_path)
                            .map_err(|e| anyhow!("Failed to remove empty scope {}: {}", org, e))?;
                    }
                }
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_package_manager_creation() {
        let temp_dir: _ = TempDir::new().unwrap();
        let config: _ = PackageManagerConfig {
            cache_dir: temp_dir.path().join("cache"),
            node_modules_dir: temp_dir.path().join("node_modules"),
            ..Default::default()
        };
        let pm: _ = PackageManager::new(config).unwrap();
        assert!(pm.config.cache_dir.exists());
        assert!(pm.config.node_modules_dir.exists());
    }
    #[test]
    fn test_parse_package_json() {
        let temp_dir: _ = TempDir::new().unwrap();
        let config: _ = PackageManagerConfig {
            cache_dir: temp_dir.path().join("cache"),
            node_modules_dir: temp_dir.path().join("node_modules"),
            ..Default::default()
        };
        let pm: _ = PackageManager::new(config).unwrap();
        // Create a test package.json
        let mut package_json = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(
            package_json,
            r#"{{
            "name": "test-package",
            "version": "1.0.0",
            "main": "index.js",
            "dependencies": {{
                "lodash": "^4.17.0"
            }}
        }}"#
        )
        .unwrap();
        let package: _ = pm.parse_package_json(package_json.path()).unwrap();
        assert_eq!(package.name, "test-package");
        assert_eq!(package.version, "1.0.0");
        assert!(package.dependencies.is_some());
    }
    #[test]
    fn test_init_package_json() {
        let temp_dir: _ = TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        let config: _ = PackageManagerConfig {
            cache_dir: PathBuf::from(".amberjs_cache"),
            node_modules_dir: PathBuf::from("node_modules"),
            ..Default::default()
        };
        let pm: _ = PackageManager::new(config).unwrap();
        let package: _ = pm.init_package_json("my-package", "1.0.0").unwrap();
        assert_eq!(package.name, "my-package");
        assert_eq!(package.version, "1.0.0");
        assert!(Path::new("package.json").exists());
    }
    #[test]
    fn test_add_remove_dependency() {
        let temp_dir: _ = TempDir::new().unwrap();
        let config: _ = PackageManagerConfig {
            cache_dir: temp_dir.path().join("cache"),
            node_modules_dir: temp_dir.path().join("node_modules"),
            ..Default::default()
        };
        let pm: _ = PackageManager::new(config).unwrap();
        let mut package = PackageJson {
            name: "test".to_string(),
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
        };
        pm.add_dependency(&mut package, "lodash", "^4.17.0")
            .unwrap();
        assert!(package.dependencies.is_some());
        if let Some(deps) = &package.dependencies {
            assert!(deps.contains_key("lodash"));
        }
        pm.remove_dependency(&mut package, "lodash").unwrap();
        if let Some(deps) = &package.dependencies {
            assert!(!deps.contains_key("lodash"));
        }
    }

    #[test]
    fn parse_npm_package_spec_handles_scoped_and_plain_names() {
        assert_eq!(
            parse_npm_package_spec("lodash"),
            ("lodash".to_string(), "latest".to_string())
        );
        assert_eq!(
            parse_npm_package_spec("lodash@4.17.21"),
            ("lodash".to_string(), "4.17.21".to_string())
        );
        assert_eq!(
            parse_npm_package_spec("@scope/name"),
            ("@scope/name".to_string(), "latest".to_string())
        );
        assert_eq!(
            parse_npm_package_spec("@scope/name@1.2.3"),
            ("@scope/name".to_string(), "1.2.3".to_string())
        );
    }
}
