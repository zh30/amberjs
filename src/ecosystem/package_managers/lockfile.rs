// Lockfile 管理器
// Stage 91 Phase 3.1 - Lockfile 解析和管理
//
/// 支持 package-lock.json、yarn.lock、pnpm-lock.yaml
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio;
use std::collections::{BTreeMap};
/// Lockfile 类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockfileType {
    PackageLock,
    YarnLock,
    PnpmLock,
}
/// Lockfile 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileEntry {
    pub version: String,
    pub resolved: String,
    pub integrity: String,
    pub requires: HashMap<String, String>,
    pub dependencies: HashMap<String, String>,
    pub dev: bool,
    pub optional: bool,
    pub bundled: bool,
    pub license: Option<String>,
    pub bin: Option<HashMap<String, String>>,
    pub engines: Option<HashMap<String, String>>,
    pub os: Option<Vec<String>>,
    pub cpu: Option<Vec<String>>,
}
/// Lockfile 管理器
#[derive(Debug)]
pub struct LockfileManager {
    lockfile_type: LockfileType,
    entries: HashMap<String, LockfileEntry>,
}
impl LockfileManager {
    /// 创建新的 lockfile 管理器
    pub fn new() -> Self {
        let lockfile_type: _ = if PathBuf::from("yarn.lock").exists() {
            LockfileType::YarnLock
        } else if PathBuf::from("pnpm-lock.yaml").exists() {
            LockfileType::PnpmLock
        } else {
            LockfileType::PackageLock
        };
        Self {
            lockfile_type,
            entries: HashMap::new(),
        }
    }
    /// 从文件加载 lockfile
    pub async fn load_from_file(&mut self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content: _ = tokio::fs::read_to_string(path).await?;
        match self.lockfile_type {
            LockfileType::PackageLock => {
                self.load_package_lock(&content)?;
            }
            LockfileType::YarnLock => {
                self.load_yarn_lock(&content)?;
            }
            LockfileType::PnpmLock => {
                self.load_pnpm_lock(&content)?;
            }
        }
        Ok(())
    }
    /// 加载 package-lock.json
    fn load_package_lock(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let package_lock: serde_json::Value = serde_json::from_str(content)?;
        if let Some(packages) = package_lock.get("packages") {
            for (key, value) in packages.as_object().unwrap() {
                if key.starts_with("node_modules/") {
                    let package_name: _ = key.strip_prefix("node_modules/").unwrap();
                    let entry: _ = LockfileEntry {
                        version: value.get("version").unwrap().as_str().unwrap_or("").to_string(),
                        resolved: value.get("resolved").unwrap().as_str().unwrap_or("").to_string(),
                        integrity: value.get("integrity").unwrap().as_str().unwrap_or("").to_string(),
                        requires: value.get("requires")
                            .and_then(|r| r.as_object())
                            .map(|m| m.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())
                                .collect())
                            .unwrap_or_default(),
                        dependencies: value.get("dependencies")
                            .and_then(|d| d.as_object())
                            .map(|m| m.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())
                                .collect())
                            .unwrap_or_default(),
                        dev: value.get("dev").unwrap().as_bool().unwrap_or(false),
                        optional: value.get("optional").unwrap().as_bool().unwrap_or(false),
                        bundled: value.get("bundled").unwrap().as_bool().unwrap_or(false),
                        license: value.get("license").and_then(|l| l.as_str()).map(|s| s.to_string()),
                        bin: value.get("bin").and_then(|b| b.as_object())
                            .map(|m| m.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())
                                .collect()),
                        engines: value.get("engines").and_then(|e| e.as_object())
                            .map(|m| m.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())
                                .collect()),
                        os: value.get("os").and_then(|o| o.as_array())
                            .map(|arr| arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string())
                                .collect()),
                        cpu: value.get("cpu").and_then(|c| c.as_array())
                            .map(|arr| arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string())
                                .collect()),
                    };
                    self.entries.insert(package_name.to_string(), entry);
                }
            }
        }
        Ok(())
    }
    /// 加载 yarn.lock
    fn load_yarn_lock(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 简化的 yarn.lock 解析
        // 实际实现需要完整的 Yarn v1 或 v2 解析器
        let lines: Vec<&str> = content.lines().collect();
        let mut current_package = None;
        let mut current_entry = None;
        for line in lines {
            if line.starts_with('"') {
                // 包定义开始
                if let Some((name, version)) = self.parse_yarn_lock_key(line) {
                    if let Some(entry) = current_entry.take() {
                        if let Some(pkg_name) = current_package.take() {
                            self.entries.insert(pkg_name, entry);
                        }
                    }
                    current_package = Some(format!("{}@{}", name, version));
                    current_entry = Some(LockfileEntry {
                        version,
                        resolved: "".to_string(),
                        integrity: "".to_string(),
                        requires: HashMap::new(),
                        dependencies: HashMap::new(),
                        dev: false,
                        optional: false,
                        bundled: false,
                        license: None,
                        bin: None,
                        engines: None,
                        os: None,
                        cpu: None,
                    });
                }
            } else if line.trim().starts_with("version") {
                if let Some(ref mut entry) = current_entry {
                    entry.version = line.split('"').nth(1).unwrap_or("").to_string();
                }
            } else if line.trim().starts_with("resolved") {
                if let Some(ref mut entry) = current_entry {
                    entry.resolved = line.split('"').nth(1).unwrap_or("").to_string();
                }
            } else if line.trim().starts_with("integrity") {
                if let Some(ref mut entry) = current_entry {
                    entry.integrity = line.split('"').nth(1).unwrap_or("").to_string();
                }
            }
        }
        // 处理最后一个包
        if let Some(entry) = current_entry.take() {
            if let Some(pkg_name) = current_package.take() {
                self.entries.insert(pkg_name, entry);
            }
        }
        Ok(())
    }
    /// 解析 yarn.lock 键
    fn parse_yarn_lock_key(&self, key: &str) -> Option<(String, String)> {
        let key: _ = key.trim().trim_matches('"');
        if let Some(at_pos) = key.rfind('@') {
            let (name_part, version_part) = key.split_at(at_pos);
            let name: _ = if name_part.starts_with("@scope/") {
                name_part.to_string()
            } else {
                name_part.trim_start_matches('@').to_string()
            };
            let version: _ = version_part.trim_start_matches('@').to_string();
            Some((name, version))
        } else {
            None
        }
    }
    /// 加载 pnpm-lock.yaml
    fn load_pnpm_lock(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 简化的 pnpm-lock.yaml 解析
        // 实际实现需要完整的 YAML 解析
        let lines: Vec<&str> = content.lines().collect();
        let mut current_package = None;
        let mut current_version = None;
        for line in lines {
            if line.starts_with("  ") && line.contains(":") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let key: _ = parts[0].trim();
                    let value: _ = parts[1].trim();
                    match key {
                        "version" => {
                            current_version = Some(value.to_string());
                        }
                        _ => {}
                    }
                }
            } else if !line.starts_with(' ') && line.contains('/') {
                // 包路径
                let parts: Vec<&str> = line.split('/').collect();
                if parts.len() >= 2 {
                    let name: _ = if parts[0].starts_with("@") {
                        format!("{}/{}", parts[0], parts[1])
                    } else {
                        parts[0].to_string()
                    };
                    if let Some(version) = current_version.take() {
                        self.entries.insert(
                            name,
                            LockfileEntry {
                                version,
                                resolved: line.trim().to_string(),
                                integrity: "".to_string(),
                                requires: HashMap::new(),
                                dependencies: HashMap::new(),
                                dev: false,
                                optional: false,
                                bundled: false,
                                license: None,
                                bin: None,
                                engines: None,
                                os: None,
                                cpu: None,
                            },
                        );
                    }
                }
            }
        }
        Ok(())
    }
    /// 保存 lockfile
    pub async fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        match self.lockfile_type {
            LockfileType::PackageLock => {
                self.save_package_lock(path).await?;
            }
            LockfileType::YarnLock => {
                self.save_yarn_lock(path).await?;
            }
            LockfileType::PnpmLock => {
                self.save_pnpm_lock(path).await?;
            }
        }
        Ok(())
    }
    /// 保存 package-lock.json
    async fn save_package_lock(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut package_lock = serde_json::Map::new();
        package_lock.insert("name".to_string(), serde_json::Value::String("amberjs-project".to_string());
        package_lock.insert("version".to_string(), serde_json::Value::String("1.0.0".to_string());
        package_lock.insert("lockfileVersion".to_string(), serde_json::Value::Number(serde_json::Number::from(3));
        let mut packages = serde_json::Map::new();
        packages.insert("".to_string(), serde_json::Value::Object(serde_json::Map::new());
        for (name, entry) in &self.entries {
            let mut package_obj = serde_json::Map::new();
            package_obj.insert("version".to_string(), serde_json::Value::String(entry.version.clone());
            package_obj.insert("resolved".to_string(), serde_json::Value::String(entry.resolved.clone());
            package_obj.insert("integrity".to_string(), serde_json::Value::String(entry.integrity.clone());
            package_obj.insert("dev".to_string(), serde_json::Value::Bool(entry.dev));
            package_obj.insert("optional".to_string(), serde_json::Value::Bool(entry.optional));
            if !entry.requires.is_empty() {
                let requires: _ = serde_json::Value::Object(
                    entry.requires.iter()
                        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())
                        .collect(),
                );
                package_obj.insert("requires".to_string(), requires);
            }
            if !entry.dependencies.is_empty() {
                let dependencies: _ = serde_json::Value::Object(
                    entry.dependencies.iter()
                        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())
                        .collect(),
                );
                package_obj.insert("dependencies".to_string(), dependencies);
            }
            let package_key: _ = format!("node_modules/{}", name));
            packages.insert(package_key, serde_json::Value::Object(package_obj));
        }
        package_lock.insert("packages".to_string(), serde_json::Value::Object(packages));
        let content: _ = serde_json::to_string_pretty(&serde_json::Value::Object(package_lock))?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }
    /// 保存 yarn.lock
    async fn save_yarn_lock(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut content = String::new();
        content.push_str("# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\n");
        content.push_str(&format!("# yarn lockfile v1\n\n"));
        for (name, entry) in &self.entries {
            let (name_part, version_part) = if name.starts_with("@") {
                let parts: Vec<&str> = name.split('/').collect();
                if parts.len() >= 2 {
                    (format!("@{}", parts[1]), parts[0].to_string())
                } else {
                    (name.clone(), "".to_string())
                }
            } else {
                (name.clone(), "".to_string())
            };
            content.push_str(&format!("\"{}@{}\":\n", name_part, version_part));
            content.push_str(&format!("  version \"{}\"\n", entry.version));
            content.push_str(&format!("  resolved \"{}\"\n", entry.resolved));
            content.push_str(&format!("  integrity {}\n", entry.integrity));
            content.push('\n');
        }
        tokio::fs::write(path, content).await?;
        Ok(())
    }
    /// 保存 pnpm-lock.yaml
    async fn save_pnpm_lock(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut content = String::new();
        content.push_str("lockfileVersion: 6.0\n");
        content.push_str("settings:\n  autoInstallPeers: true\n  excludeLinksFromLockfile: false\n\n");
        content.push_str("importers:\n  .:\n    devDependencies:\n\n");
        content.push_str("packages:\n");
        for (name, entry) in &self.entries {
            let package_path: _ = if name.starts_with("@") {
                format!("node_modules/{}", name)
            } else {
                format!("node_modules/{}", name)
            };
            content.push_str(&format!("  {}:\n", package_path));
            content.push_str(&format!("    version: {}\n", entry.version));
            if !entry.requires.is_empty() {
                content.push_str(&format!("    requires:\n"));
                for (dep, version) in &entry.requires {
                    content.push_str(&format!("      {}: {}\n", dep, version));
                }
            }
            content.push('\n');
        }
        tokio::fs::write(path, content).await?;
        Ok(())
    }
    /// 更新 lockfile
    pub async fn update_lockfile(
        &mut self,
        resolutions: &HashMap<String, PackageResolution>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (name, resolution) in resolutions {
            let entry: _ = LockfileEntry {
                version: resolution.version.clone(),
                resolved: resolution.resolved_url.clone(),
                integrity: resolution.integrity.clone(),
                requires: resolution.dependencies.clone(),
                dependencies: HashMap::new(),
                dev: false,
                optional: false,
                bundled: false,
                license: None,
                bin: Some(resolution.bins.clone()),
                engines: None,
                os: None,
                cpu: None,
            };
            self.entries.insert(name.clone(), entry);
        }
        Ok(())
    }
    /// 获取所有条目
    pub fn get_entries(&self) -> &HashMap<String, LockfileEntry> {
        &self.entries
    }
    /// 查找包
    pub fn find_package(&self, name: &str) -> Option<&LockfileEntry> {
        self.entries.get(name)
    }
    /// 验证 lockfile 完整性
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 检查是否有缺失的包
        for (name, entry) in &self.entries {
            if entry.version.is_empty() {
                return Err(format!("Package {} missing version", name).into());
            }
            if entry.resolved.is_empty() {
                return Err(format!("Package {} missing resolved URL", name).into());
            }
        }
        Ok(())
    }
}