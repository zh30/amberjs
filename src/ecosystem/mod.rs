// Amber 生态系统模块
// Stage 80 - 生态系统完善
// Stage 86 - 生态完善 (插件系统增强)
/// Stage 91 Phase 3 - 生态系统集成
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hasher;
use std::hash::Hash;
pub mod package;
pub mod marketplace;
pub mod marketplace_core;
pub mod devtools;
pub mod community;
pub mod analytics;
pub mod plugin_engine;
// Stage 91 Phase 3.1 - 包管理器集成
pub mod package_managers;
// Stage 91 Phase 3.2 - 开发工具支持
pub mod type_generator;
pub mod ts_type_analyzer;
pub mod dts_emitter;
pub mod symbol_resolver;

pub use package::*;
pub use plugin_engine::*;
pub use marketplace_core::*;
pub use package_managers::*;
pub use type_generator::*;
pub use ts_type_analyzer::*;
pub use dts_emitter::*;
pub use symbol_resolver::*;
use serde::{Serialize, Deserialize};
use chrono::Utc;
pub use marketplace::*;
pub use devtools::*;
pub use community::*;
pub use analytics::*;
// 共享类型定义
pub mod types {

use std::collections::{BTreeMap};
    /// 版本号
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub struct Version {
        pub major: u64,
        pub minor: u64,
        pub patch: u64,
        pub pre_release: Option<String>,
    }
    impl Version {
        pub fn parse(s: &str) -> Result<Self, ParseError> {
            let parts: Vec<&str> = s.split('.').collect();
            if parts.len() < 3 {
                return Err(ParseError::InvalidFormat);
            }
            Ok(Self {
                major: parts[0].parse().map_err(|_| ParseError::InvalidNumber)?,
                minor: parts[1].parse().map_err(|_| ParseError::InvalidNumber)?,
                patch: parts[2].parse().map_err(|_| ParseError::InvalidNumber)?,
                pre_release: None,
            })
        }
        pub fn to_string(&self) -> String {
            if let Some(ref pre) = self.pre_release {
                format!("{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
            } else {
                format!("{}.{}.{}", self.major, self.minor, self.patch)
            }
        }
    }
    impl std::fmt::Display for Version {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.to_string())
        }
    }
    /// 版本约束
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct VersionConstraint {
        pub comparator: VersionComparator,
        pub version: Version,
    }
    impl std::hash::Hash for VersionConstraint {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.comparator.hash(state);
            self.version.hash(state);
        }
    }
    impl VersionConstraint {
        pub fn parse(s: &str) -> Result<Self, ParseError> {
            let (comp, ver) = s.split_at(1);
            Ok(Self {
                comparator: match comp {
                    "^" => VersionComparator::Compatible,
                    "~" => VersionComparator::Approximate,
                    ">=" => VersionComparator::GreaterEqual,
                    _ => VersionComparator::Exact,
                },
                version: Version::parse(ver)?,
            })
        }
        pub fn matches(&self, version: &Version) -> bool {
            match self.comparator {
                VersionComparator::Exact => version == &self.version,
                VersionComparator::Compatible => {
                    version.major == self.version.major
                        && version.minor >= self.version.minor
                }
                VersionComparator::Approximate => {
                    version.major == self.version.major
                        && version.minor == self.version.minor
                        && version.patch >= self.version.patch
                }
                VersionComparator::GreaterEqual => {
                    version.major > self.version.major
                        || (version.major == self.version.major
                            && version.minor > self.version.minor)
                        || (version.major == self.version.major
                            && version.minor == self.version.minor
                            && version.patch >= self.version.patch)
                }
            }
        }
    }
    /// 版本比较器
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum VersionComparator {
        Exact,
        Compatible,
        Approximate,
        GreaterEqual,
    }
    impl std::hash::Hash for VersionComparator {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            match self {
                VersionComparator::Exact => "Exact".hash(state),
                VersionComparator::Compatible => "Compatible".hash(state),
                VersionComparator::Approximate => "Approximate".hash(state),
                VersionComparator::GreaterEqual => "GreaterEqual".hash(state),
            }
        }
    }
    /// 包清单
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PackageManifest {
        pub name: String,
        pub version: Version,
        pub dependencies: HashMap<String, VersionConstraint>,
        pub dev_dependencies: HashMap<String, VersionConstraint>,
    }
    /// 包 ID
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub struct PackageId {
        pub name: String,
        pub version: Version,
    }
    /// 包信息
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PackageInfo {
        pub name: String,
        pub version: Version,
        pub download_url: String,
        pub checksum: String,
        pub available_versions: Vec<Version>,
        pub manifest: PackageManifest,
    }
    /// 包
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Package {
        pub id: PackageId,
        pub manifest: PackageManifest,
        pub tarball: Vec<u8>,
    }
    /// 依赖图
    #[derive(Debug, Clone)]
    pub struct DependencyGraph {
        pub nodes: HashMap<String, Version>,
        pub edges: HashMap<String, HashSet<String>>,
        pub has_circular: bool,
        pub conflicts_resolved: bool,
    }
    impl DependencyGraph {
        pub fn new() -> Self {
            Self {
                nodes: HashMap::new(),
                edges: HashMap::new(),
                has_circular: false,
                conflicts_resolved: true,
            }
        }
        pub fn add_node(&mut self, name: String, version: Version) {
            self.nodes.insert(name.clone(), version);
            self.edges.entry(name).or_insert_with(HashSet::new);
        }
        pub fn add_edge(&mut self, from: String, to: String) {
            self.edges.entry(from).or_insert_with(HashSet::new).insert(to);
        }
        pub fn contains(&self, name: &str) -> bool {
            self.nodes.contains_key(name)
        }
        pub fn has_circular_dependency(&self) -> bool {
            self.has_circular
        }
    }
    /// 版本选择结果
    #[derive(Debug, Clone)]
    pub struct VersionSelection {
        pub selected_version: Version,
        pub is_compatible: bool,
        pub resolution_conflicts: bool,
    }
    /// 下载结果
    #[derive(Debug, Clone)]
    pub struct DownloadResult {
        pub package_name: String,
        pub success: bool,
        pub downloaded_at: chrono::DateTime<chrono::Utc>,
        pub error: Option<String>,
    }
    /// 版本约束集合
    #[derive(Debug, Clone)]
    pub struct VersionConstraints {
        pub package_name: String,
        pub constraints: Vec<VersionConstraint>,
    }
    /// 预热结果
    #[derive(Debug, Clone)]
    pub struct PrefetchResult {
        pub prefetched_count: u64,
        pub cache_hit_rate: f64,
    }
    /// 解析错误
    #[derive(Debug)]
    pub enum ParseError {
        InvalidFormat,
        InvalidNumber,
    }
    impl std::fmt::Display for ParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ParseError::InvalidFormat => write!(f, "Invalid version format"),
                ParseError::InvalidNumber => write!(f, "Invalid version number"),
            }
        }
    }
    impl std::error::Error for ParseError {}
}