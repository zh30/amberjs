// 包管理器集成模块
// Stage 91 Phase 3.1 - 包管理器集成
//
// 提供 npm、Yarn、pnpm 的完整兼容性支持
pub mod npm;
pub mod yarn;
pub mod pnpm;
pub mod lockfile;
pub mod registry;
pub mod auth;
pub use npm::*;
pub use yarn::*;
pub use pnpm::*;
pub use lockfile::*;
pub use registry::*;
pub use auth::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::collections::{BTreeMap};
/// 包管理器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageManagerType {
    Npm,
    Yarn,
    Pnpm,
}
/// 包规范
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageSpec {
    /// 包名 (latest)
    Name(String),
    /// 包名@版本
    NameVersion(String, String),
    /// 包名@版本范围
    NameRange(String, String),
    /// Git URL
    Git(String),
    /// 本地路径
    Local(PathBuf),
}
/// 构建结果
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub success: bool,
    pub output_dir: PathBuf,
    pub build_time_ms: u64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}
/// 包管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagerConfig {
    pub manager_type: PackageManagerType,
    pub registry_url: String,
    pub cache_dir: PathBuf,
    pub timeout_ms: u64,
    pub retry_count: u32,
    pub auth_token: Option<String>,
    pub ca_file: Option<PathBuf>,
    pub strict_ssl: bool,
}
impl Default for PackageManagerConfig {
    fn default() -> Self {
        Self {
            manager_type: PackageManagerType::Npm,
            registry_url: "https://registry.npmjs.org/".to_string(),
            cache_dir: PathBuf::from(".amberjs_cache"),
            timeout_ms: 30000,
            retry_count: 3,
            auth_token: None,
            ca_file: None,
            strict_ssl: true,
        }
    }
}
/// 包解析结果
#[derive(Debug, Clone)]
pub struct PackageResolution {
    pub package_name: String,
    pub version: String,
    pub resolved_url: String,
    pub integrity: String,
    pub dependencies: HashMap<String, String>,
    pub peer_dependencies: HashMap<String, String>,
    pub optional_dependencies: HashMap<String, String>,
    pub bins: HashMap<String, PathBuf>,
    pub main: String,
    pub types: Option<String>,
    pub exports: Option<serde_json::Value>,
}
/// 包安装选项
#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub production: bool,
    pub dev: bool,
    pub optional: bool,
    pub global: bool,
    pub save: bool,
    pub save_dev: bool,
    pub save_exact: bool,
    pub ignore_scripts: bool,
    pub legacy_peer_deps: bool,
}
impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            production: false,
            dev: false,
            optional: true,
            global: false,
            save: true,
            save_dev: false,
            save_exact: false,
            ignore_scripts: false,
            legacy_peer_deps: false,
        }
    }
}
/// 依赖类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    Regular,
    Dev,
    Peer,
    Optional,
    Bundle,
}
/// 脚本定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub name: String,
    pub command: String,
    pub description: Option<String>,
}
/// 导出配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportsConfig {
    pub main: Option<String>,
    pub types: Option<String>,
    pub module: Option<String>,
    pub browser: Option<String>,
    pub node: Option<String>,
    pub import: Option<String>,
    pub require: Option<String>,
    pub default: Option<String>,
}
/// 文件模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesConfig {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}
/// 生命周期钩子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleHooks {
    pub pre_install: Vec<String>,
    pub post_install: Vec<String>,
    pub pre_publish: Vec<String>,
    pub post_publish: Vec<String>,
}
/// 语义化版本范围
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionRange {
    Exact(String),
    GreaterThan(String),
    GreaterThanOrEqual(String),
    LessThan(String),
    LessThanOrEqual(String),
    Compatible(String),      // ^1.2.3
    Approximate(String),     // ~1.2.3
    Range(String, String),   // >=1.0.0 <2.0.0
    Or(Vec<VersionRange>),
    Wildcard,               // *
}
impl VersionRange {
    /// 解析版本范围字符串
    pub fn parse(s: &str) -> Result<Self, String> {
        if s == "*" {
            return Ok(VersionRange::Wildcard);
        }
        if let Some(stripped) = s.strip_prefix('^') {
            return Ok(VersionRange::Compatible(stripped.to_string());
        }
        if let Some(stripped) = s.strip_prefix('~') {
            return Ok(VersionRange::Approximate(stripped.to_string());
        }
        if s.contains("||") {
            let parts: Vec<VersionRange> = s
                .split("||")
                .map(|p| VersionRange::parse(p.trim())
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(VersionRange::Or(parts));
        }
        if s.contains(" - ") {
            let parts: Vec<&str> = s.split(" - ").collect();
            if parts.len() == 2 {
                return Ok(VersionRange::Range(parts[0].to_string(), parts[1].to_string());
            }
        }
        if let Some(stripped) = s.strip_prefix('>') {
            if stripped.starts_with('=') {
                return Ok(VersionRange::GreaterThanOrEqual(stripped[1..].to_string());
            }
            return Ok(VersionRange::GreaterThan(stripped.to_string());
        }
        if let Some(stripped) = s.strip_prefix('<') {
            if stripped.starts_with('=') {
                return Ok(VersionRange::LessThanOrEqual(stripped[1..].to_string());
            }
            return Ok(VersionRange::LessThan(stripped.to_string());
        }
        Ok(VersionRange::Exact(s.to_string())
    }
    /// 检查版本是否匹配此范围
    pub fn matches(&self, version: &str) -> bool {
        match self {
            VersionRange::Exact(v) => version == v,
            VersionRange::Wildcard => true,
            VersionRange::Compatible(v) => {
                // 兼容版本：主版本号相同，更高级版本允许
                if let Some((major, _)) = version.split('.').next() {
                    if let Some((req_major, _)) = v.split('.').next() {
                        return major == req_major;
                    }
                }
                false
            }
            VersionRange::Approximate(v) => {
                // 近似版本：主版本和次版本相同，补丁版本允许更高
                let v_parts: Vec<&str> = v.split('.').collect();
                let ver_parts: Vec<&str> = version.split('.').collect();
                if v_parts.len() >= 2 && ver_parts.len() >= 2 {
                    return v_parts[0] == ver_parts[0] && v_parts[1] == ver_parts[1];
                }
                false
            }
            VersionRange::GreaterThan(v) => version > v,
            VersionRange::GreaterThanOrEqual(v) => version >= v,
            VersionRange::LessThan(v) => version < v,
            VersionRange::LessThanOrEqual(v) => version <= v,
            VersionRange::Range(start, end) => version >= start && version <= end,
            VersionRange::Or(ranges) => ranges.iter().any(|r| r.matches(version)),
        }
    }
}
impl std::fmt::Display for VersionRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionRange::Exact(v) => write!(f, "{}", v),
            VersionRange::Wildcard => write!(f, "*"),
            VersionRange::Compatible(v) => write!(f, "^{}", v),
            VersionRange::Approximate(v) => write!(f, "~{}", v),
            VersionRange::GreaterThan(v) => write!(f, ">{}", v),
            VersionRange::GreaterThanOrEqual(v) => write!(f, ">={}", v),
            VersionRange::LessThan(v) => write!(f, "<{}", v),
            VersionRange::LessThanOrEqual(v) => write!(f, "<={}", v),
            VersionRange::Range(start, end) => write!(f, "{} - {}", start, end),
            VersionRange::Or(ranges) => {
                let strs: Vec<String> = ranges.iter().map(|r| r.to_string()).collect();
                write!(f, "{}", strs.join(" || "))
            }
        }
    }
}