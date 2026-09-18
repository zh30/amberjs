use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use once_cell::sync::Lazy;

#[derive(Clone, Hash, Eq, PartialEq)]
struct ResolutionCacheKey {
    parent_dir: PathBuf,
    specifier: String,
    is_esm: bool,
}

static MODULE_RESOLUTION_CACHE: Lazy<
    RwLock<HashMap<ResolutionCacheKey, Result<ResolvedModule, CommonJsResolveError>>>,
> = Lazy::new(|| RwLock::new(HashMap::new()));

static PACKAGE_JSON_CACHE: Lazy<RwLock<HashMap<PathBuf, Option<serde_json::Value>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// 清空模块解析缓存（用于热重载或测试隔离）
pub fn clear_module_resolution_cache() {
    if let Ok(mut cache) = MODULE_RESOLUTION_CACHE.write() {
        cache.clear();
    }
    if let Ok(mut cache) = PACKAGE_JSON_CACHE.write() {
        cache.clear();
    }
}

const JS_EXTENSIONS: &[&str] = &["js", "json", "ts", "mjs", "cjs", "tsx"];
const COMMONJS_EXPORT_CONDITIONS: &[&str] = &[
    "require", "wintercg", "wintertc", "node", "amber", "amberjs", "default",
];
const ESM_EXPORT_CONDITIONS: &[&str] = &[
    "import", "wintercg", "wintertc", "node", "amber", "amberjs", "default",
];
const BUILTIN_MODULES: &[&str] = &[
    "ai",
    "amber:ai",
    "amber:db",
    "amber:ffi",
    "amber:pool",
    "amber:wasm",
    "amber:replay",
    "amber:weights",
    "amber:security",
    "amber:permissions",
    "amber:kv",
    "amber:tools",
    "amber:bus",
    "amber:grammar",
    "amber:checkpoint",
    "amber:sockets",
    "amber:sqlite",
    "amber:vector",
    "amber:std",
    "amber:std/dotenv",
    "amber:std/cli",
    "amber:std/fs",
    "amber:std/crypto",
    "amber:std/assert",
    "amber:mcp",
    "amber:sandbox",
    "amber:vfs",
    "amber:test",
    "assert",
    "assert/strict",
    "async_hooks",
    "db",
    "ffi",
    "pool",
    "wasm",
    "replay",
    "weights",
    "security",
    "permissions",
    "kv",
    "tools",
    "bus",
    "grammar",
    "checkpoint",
    "sockets",
    "wintertc:sockets",
    "sqlite",
    "vector",
    "std",
    "std/dotenv",
    "std/cli",
    "std/fs",
    "std/crypto",
    "std/assert",
    "mcp",
    "sandbox",
    "vfs",
    "buffer",
    "child_process",
    "crypto",
    "diagnostics_channel",
    "dns",
    "events",
    "fs",
    "fs/promises",
    "http",
    "http2",
    "https",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "performance",
    "process",
    "querystring",
    "readline",
    "stream",
    "stream/promises",
    "string_decoder",
    "tcp_async",
    "timers",
    "timers/promises",
    "tls",
    "tty",
    "url",
    "util",
    "vm",
    "worker_threads",
    "zlib",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolvedModule {
    Builtin(String),
    File(PathBuf),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommonJsModuleFormat {
    CommonJs,
    EsModule,
    Json,
    TypeScript,
    TypeScriptJsx,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PackageSpecifier {
    name: String,
    subpath: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PackageExportResolution {
    Resolved(PathBuf),
    NotFound,
    Blocked,
    InvalidTarget(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PackageImportResolution {
    Resolved(ResolvedModule),
    NotFound,
    Blocked,
    InvalidTarget(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommonJsResolveError {
    specifier: String,
    from: PathBuf,
    reason: Option<String>,
}

impl CommonJsResolveError {
    pub fn new(specifier: impl Into<String>, from: impl Into<PathBuf>) -> Self {
        Self {
            specifier: specifier.into(),
            from: from.into(),
            reason: None,
        }
    }

    fn with_reason(
        specifier: impl Into<String>,
        from: impl Into<PathBuf>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            specifier: specifier.into(),
            from: from.into(),
            reason: Some(reason.into()),
        }
    }

    fn package_path_not_exported(package_root: &Path, export_key: &str) -> Self {
        let package_json_path = package_root.join("package.json");
        Self::with_reason(
            export_key,
            package_root,
            format!(
                "ERR_PACKAGE_PATH_NOT_EXPORTED: Package subpath '{}' is not defined by \"exports\" in {}",
                export_key,
                package_json_path.display()
            ),
        )
    }

    fn package_import_not_defined(package_root: &Path, specifier: &str) -> Self {
        let package_json_path = package_root.join("package.json");
        Self::with_reason(
            specifier,
            package_root,
            format!(
                "ERR_PACKAGE_IMPORT_NOT_DEFINED: Package import specifier '{}' is not defined in {}",
                specifier,
                package_json_path.display()
            ),
        )
    }

    fn invalid_package_target(package_root: &Path, export_key: &str, target: &str) -> Self {
        let package_json_path = package_root.join("package.json");
        Self::with_reason(
            export_key,
            package_root,
            format!(
                "ERR_INVALID_PACKAGE_TARGET: Invalid package target '{}' for '{}' in {}",
                target,
                export_key,
                package_json_path.display()
            ),
        )
    }

    fn invalid_module_specifier(package_root: &Path, export_key: &str) -> Self {
        Self::with_reason(
            export_key,
            package_root,
            format!(
                "ERR_INVALID_MODULE_SPECIFIER: Invalid module specifier '{}'",
                export_key
            ),
        )
    }

    fn invalid_package_config(package_root: &Path, reason: impl Into<String>) -> Self {
        let package_json_path = package_root.join("package.json");
        Self::with_reason(
            package_json_path.to_string_lossy(),
            package_root,
            format!(
                "ERR_INVALID_PACKAGE_CONFIG: Invalid package configuration in {}: {}",
                package_json_path.display(),
                reason.into()
            ),
        )
    }
}

impl fmt::Display for CommonJsResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(reason) = &self.reason {
            return write!(formatter, "{reason}");
        }

        write!(
            formatter,
            "Cannot find module '{}' from '{}'",
            self.specifier,
            self.from.display()
        )
    }
}

impl std::error::Error for CommonJsResolveError {}

pub fn is_builtin_module(specifier: &str) -> bool {
    normalize_builtin_specifier(specifier).is_some()
}

pub fn resolve_commonjs_module(
    specifier: &str,
    parent_dir: &Path,
) -> Result<ResolvedModule, CommonJsResolveError> {
    let remapped =
        crate::tooling::import_map::resolve_from_global_import_map(specifier, Some(parent_dir));
    let effective_specifier = remapped.as_deref().unwrap_or(specifier);

    let key = ResolutionCacheKey {
        parent_dir: parent_dir.to_path_buf(),
        specifier: effective_specifier.to_string(),
        is_esm: false,
    };
    if let Ok(cache) = MODULE_RESOLUTION_CACHE.read() {
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }
    }
    let res =
        resolve_module_with_conditions(effective_specifier, parent_dir, COMMONJS_EXPORT_CONDITIONS);
    if let Ok(mut cache) = MODULE_RESOLUTION_CACHE.write() {
        cache.insert(key, res.clone());
    }
    res
}

pub fn resolve_esm_module(
    specifier: &str,
    parent_dir: &Path,
) -> Result<ResolvedModule, CommonJsResolveError> {
    let remapped =
        crate::tooling::import_map::resolve_from_global_import_map(specifier, Some(parent_dir));
    let effective_specifier = remapped.as_deref().unwrap_or(specifier);

    let key = ResolutionCacheKey {
        parent_dir: parent_dir.to_path_buf(),
        specifier: effective_specifier.to_string(),
        is_esm: true,
    };
    if let Ok(cache) = MODULE_RESOLUTION_CACHE.read() {
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }
    }
    let res =
        resolve_module_with_conditions(effective_specifier, parent_dir, ESM_EXPORT_CONDITIONS);
    if let Ok(mut cache) = MODULE_RESOLUTION_CACHE.write() {
        cache.insert(key, res.clone());
    }
    res
}

fn resolve_module_with_conditions(
    specifier: &str,
    parent_dir: &Path,
    conditions: &[&str],
) -> Result<ResolvedModule, CommonJsResolveError> {
    if let Some(builtin_name) = normalize_builtin_specifier(specifier) {
        return Ok(ResolvedModule::Builtin(builtin_name.to_string()));
    }

    if specifier.starts_with("node:") {
        return Err(CommonJsResolveError::new(specifier, parent_dir));
    }

    if specifier == "."
        || specifier == ".."
        || specifier.starts_with("./")
        || specifier.starts_with("../")
        || Path::new(specifier).is_absolute()
    {
        let candidate = if Path::new(specifier).is_absolute() {
            PathBuf::from(specifier)
        } else {
            parent_dir.join(specifier)
        };
        return resolve_path_candidate(&candidate, conditions)?
            .map(ResolvedModule::File)
            .ok_or_else(|| CommonJsResolveError::new(specifier, parent_dir));
    }

    if specifier.starts_with('#') {
        return resolve_package_import(specifier, parent_dir, conditions);
    }

    if let Some(resolved) = resolve_package_self_reference(specifier, parent_dir, conditions)? {
        return Ok(ResolvedModule::File(resolved));
    }

    resolve_node_modules(specifier, parent_dir, conditions)?
        .map(ResolvedModule::File)
        .ok_or_else(|| CommonJsResolveError::new(specifier, parent_dir))
}

pub fn classify_commonjs_file(path: &Path) -> Result<CommonJsModuleFormat, CommonJsResolveError> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => Ok(CommonJsModuleFormat::Json),
        Some("ts") => Ok(CommonJsModuleFormat::TypeScript),
        Some("tsx") | Some("jsx") => Ok(CommonJsModuleFormat::TypeScriptJsx),
        Some("mjs") => Ok(CommonJsModuleFormat::EsModule),
        Some("cjs") => Ok(CommonJsModuleFormat::CommonJs),
        Some("js") => {
            if nearest_package_type(path)?.as_deref() == Some("module") {
                Ok(CommonJsModuleFormat::EsModule)
            } else {
                Ok(CommonJsModuleFormat::CommonJs)
            }
        }
        _ => Ok(CommonJsModuleFormat::CommonJs),
    }
}

fn normalize_builtin_specifier(specifier: &str) -> Option<&str> {
    if BUILTIN_MODULES.contains(&specifier) {
        return Some(specifier);
    }
    let without_node = specifier.strip_prefix("node:").unwrap_or(specifier);
    if BUILTIN_MODULES.contains(&without_node) {
        return Some(without_node);
    }
    let without_amber = specifier.strip_prefix("amber:").unwrap_or(specifier);
    if BUILTIN_MODULES.contains(&without_amber) {
        return Some(without_amber);
    }
    let without_wintertc = specifier.strip_prefix("wintertc:").unwrap_or(specifier);
    if BUILTIN_MODULES.contains(&without_wintertc) {
        return Some(without_wintertc);
    }
    None
}

fn nearest_package_type(path: &Path) -> Result<Option<String>, CommonJsResolveError> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if let Some(package_json) = read_package_json(dir)? {
            return Ok(package_json
                .get("type")
                .and_then(|value| value.as_str())
                .map(|package_type| package_type.to_string()));
        }
        current = dir.parent();
    }

    Ok(None)
}

fn is_export_condition(condition: &str, conditions: &[&str]) -> bool {
    conditions.contains(&condition)
}

fn resolve_node_modules(
    specifier: &str,
    parent_dir: &Path,
    conditions: &[&str],
) -> Result<Option<PathBuf>, CommonJsResolveError> {
    let package = parse_package_specifier(specifier);
    let mut current = Some(parent_dir);
    while let Some(dir) = current {
        let package_root = dir.join("node_modules").join(&package.name);
        let resolved = if let Some(subpath) = &package.subpath {
            resolve_package_subpath(&package_root, subpath, conditions)?
        } else {
            resolve_path_candidate(&package_root, conditions)?
        };

        if let Some(resolved) = resolved {
            return Ok(Some(resolved));
        }
        current = dir.parent();
    }
    Ok(None)
}

fn parse_package_specifier(specifier: &str) -> PackageSpecifier {
    if specifier.starts_with('@') {
        let mut parts = specifier.splitn(3, '/');
        let scope = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        let subpath = parts.next().map(|path| path.to_string());

        if !scope.is_empty() && !name.is_empty() {
            return PackageSpecifier {
                name: format!("{scope}/{name}"),
                subpath,
            };
        }
    }

    let mut parts = specifier.splitn(2, '/');
    let name = parts.next().unwrap_or_default().to_string();
    let subpath = parts.next().map(|path| path.to_string());
    PackageSpecifier { name, subpath }
}

fn resolve_package_self_reference(
    specifier: &str,
    parent_dir: &Path,
    conditions: &[&str],
) -> Result<Option<PathBuf>, CommonJsResolveError> {
    let package = parse_package_specifier(specifier);
    if package.name.is_empty() {
        return Ok(None);
    }

    let mut current = Some(parent_dir);
    while let Some(dir) = current {
        let Some(package_json) = read_package_json(dir)? else {
            current = dir.parent();
            continue;
        };

        let Some(package_name) = package_json.get("name").and_then(|value| value.as_str()) else {
            current = dir.parent();
            continue;
        };
        if package_name != package.name {
            current = dir.parent();
            continue;
        }

        let Some(exports) = package_json.get("exports") else {
            return Ok(None);
        };

        if let Some(subpath) = &package.subpath {
            return resolve_package_subpath(dir, subpath, conditions);
        }

        return match resolve_package_root_exports(dir, exports, conditions)? {
            Some(PackageExportResolution::Resolved(resolved)) => Ok(Some(resolved)),
            Some(PackageExportResolution::InvalidTarget(target)) => Err(
                CommonJsResolveError::invalid_package_target(dir, ".", &target),
            ),
            Some(PackageExportResolution::NotFound | PackageExportResolution::Blocked) => {
                Err(CommonJsResolveError::package_path_not_exported(dir, "."))
            }
            None => Ok(None),
        };
    }

    Ok(None)
}

fn resolve_package_import(
    specifier: &str,
    parent_dir: &Path,
    conditions: &[&str],
) -> Result<ResolvedModule, CommonJsResolveError> {
    if specifier == "#" || specifier.ends_with('/') {
        return Err(CommonJsResolveError::invalid_module_specifier(
            parent_dir, specifier,
        ));
    }

    let mut current = Some(parent_dir);
    while let Some(dir) = current {
        if dir.file_name() == Some(OsStr::new("node_modules")) {
            break;
        }

        let Some(package_json) = read_package_json(dir)? else {
            current = dir.parent();
            continue;
        };

        if let Some(imports_obj) = package_json
            .get("imports")
            .and_then(|value| value.as_object())
        {
            if let Some(target) = imports_obj.get(specifier) {
                return resolve_package_import_value(dir, specifier, target, conditions);
            }

            return resolve_package_import_resolution(
                dir,
                specifier,
                resolve_package_pattern_import(dir, imports_obj, specifier, conditions)?,
            );
        }

        return Err(CommonJsResolveError::package_import_not_defined(
            dir, specifier,
        ));
    }

    Err(CommonJsResolveError::package_import_not_defined(
        parent_dir, specifier,
    ))
}

fn resolve_package_import_resolution(
    package_root: &Path,
    specifier: &str,
    resolution: PackageImportResolution,
) -> Result<ResolvedModule, CommonJsResolveError> {
    match resolution {
        PackageImportResolution::Resolved(resolved) => Ok(resolved),
        PackageImportResolution::InvalidTarget(target) => Err(
            CommonJsResolveError::invalid_package_target(package_root, specifier, &target),
        ),
        PackageImportResolution::NotFound | PackageImportResolution::Blocked => Err(
            CommonJsResolveError::package_import_not_defined(package_root, specifier),
        ),
    }
}

fn resolve_package_import_value(
    package_root: &Path,
    specifier: &str,
    target: &serde_json::Value,
    conditions: &[&str],
) -> Result<ResolvedModule, CommonJsResolveError> {
    let resolution =
        resolve_package_import_value_with_pattern(package_root, target, None, conditions)?;
    resolve_package_import_resolution(package_root, specifier, resolution)
}

fn resolve_package_import_value_with_pattern(
    package_root: &Path,
    target: &serde_json::Value,
    pattern_capture: Option<&str>,
    conditions: &[&str],
) -> Result<PackageImportResolution, CommonJsResolveError> {
    if target.is_null() {
        return Ok(PackageImportResolution::Blocked);
    }

    if let Some(target) = target.as_str() {
        let target = if let Some(pattern_capture) = pattern_capture {
            target.replace('*', pattern_capture)
        } else {
            target.to_string()
        };
        return resolve_package_import_string_target(package_root, &target, conditions);
    }

    if let Some(targets) = target.as_array() {
        if targets.is_empty() {
            return Ok(PackageImportResolution::Blocked);
        }

        let mut last_fallback = PackageImportResolution::NotFound;
        for target in targets {
            match resolve_package_import_value_with_pattern(
                package_root,
                target,
                pattern_capture,
                conditions,
            )? {
                PackageImportResolution::Resolved(resolved) => {
                    return Ok(PackageImportResolution::Resolved(resolved));
                }
                PackageImportResolution::InvalidTarget(target) => {
                    last_fallback = PackageImportResolution::InvalidTarget(target);
                }
                PackageImportResolution::Blocked => {
                    return Ok(PackageImportResolution::Blocked);
                }
                PackageImportResolution::NotFound => {
                    if !matches!(last_fallback, PackageImportResolution::InvalidTarget(_)) {
                        last_fallback = PackageImportResolution::NotFound;
                    }
                }
            }
        }
        return Ok(last_fallback);
    }

    if let Some(target_obj) = target.as_object() {
        for (condition, condition_target) in target_obj {
            if is_export_condition(condition, conditions) {
                match resolve_package_import_value_with_pattern(
                    package_root,
                    condition_target,
                    pattern_capture,
                    conditions,
                )? {
                    PackageImportResolution::Resolved(resolved) => {
                        return Ok(PackageImportResolution::Resolved(resolved));
                    }
                    PackageImportResolution::Blocked => {
                        return Ok(PackageImportResolution::Blocked);
                    }
                    PackageImportResolution::InvalidTarget(target) => {
                        return Ok(PackageImportResolution::InvalidTarget(target));
                    }
                    PackageImportResolution::NotFound => {}
                }
            }
        }
        return Ok(PackageImportResolution::NotFound);
    }

    Ok(PackageImportResolution::InvalidTarget(target.to_string()))
}

fn resolve_package_import_string_target(
    package_root: &Path,
    target: &str,
    conditions: &[&str],
) -> Result<PackageImportResolution, CommonJsResolveError> {
    if target.is_empty() {
        return Ok(PackageImportResolution::InvalidTarget(target.to_string()));
    }

    if target.starts_with("./") {
        return Ok(
            match resolve_package_string_target(package_root, target, conditions)? {
                PackageExportResolution::Resolved(resolved) => {
                    PackageImportResolution::Resolved(ResolvedModule::File(resolved))
                }
                PackageExportResolution::NotFound => PackageImportResolution::NotFound,
                PackageExportResolution::Blocked => PackageImportResolution::Blocked,
                PackageExportResolution::InvalidTarget(target) => {
                    PackageImportResolution::InvalidTarget(target)
                }
            },
        );
    }

    if target.starts_with("../") || target.starts_with('/') || url::Url::parse(target).is_ok() {
        return Ok(PackageImportResolution::InvalidTarget(target.to_string()));
    }

    resolve_module_with_conditions(target, package_root, conditions)
        .map(PackageImportResolution::Resolved)
}

fn resolve_path_candidate(
    candidate: &Path,
    conditions: &[&str],
) -> Result<Option<PathBuf>, CommonJsResolveError> {
    if let Some(file) = resolve_as_file(candidate) {
        return Ok(Some(file));
    }

    resolve_as_directory(candidate, conditions)
}

fn resolve_as_file(candidate: &Path) -> Option<PathBuf> {
    if candidate.is_file() {
        return canonical_file(candidate);
    }

    let candidate_str = candidate.to_string_lossy();
    for extension in JS_EXTENSIONS {
        let appended = PathBuf::from(format!("{candidate_str}.{extension}"));
        if appended.is_file() {
            return canonical_file(&appended);
        }
        let with_extension = candidate.with_extension(extension);
        if with_extension.is_file() {
            return canonical_file(&with_extension);
        }
    }

    None
}

fn resolve_as_directory(
    candidate: &Path,
    conditions: &[&str],
) -> Result<Option<PathBuf>, CommonJsResolveError> {
    if !candidate.is_dir() {
        return Ok(None);
    }

    if let Some(main_path) = resolve_package_main(candidate, conditions)? {
        return Ok(Some(main_path));
    }

    for extension in JS_EXTENSIONS {
        let index_path = candidate.join(format!("index.{extension}"));
        if index_path.is_file() {
            return Ok(canonical_file(&index_path));
        }
    }

    Ok(None)
}

fn resolve_package_main(
    package_root: &Path,
    conditions: &[&str],
) -> Result<Option<PathBuf>, CommonJsResolveError> {
    let Some(package_json) = read_package_json(package_root)? else {
        return Ok(None);
    };

    if let Some(exports) = package_json.get("exports") {
        match resolve_package_root_exports(package_root, exports, conditions)? {
            Some(PackageExportResolution::Resolved(resolved)) => return Ok(Some(resolved)),
            Some(PackageExportResolution::InvalidTarget(target)) => {
                return Err(CommonJsResolveError::invalid_package_target(
                    package_root,
                    ".",
                    &target,
                ));
            }
            Some(PackageExportResolution::NotFound | PackageExportResolution::Blocked) => {
                return Err(CommonJsResolveError::package_path_not_exported(
                    package_root,
                    ".",
                ));
            }
            None => {}
        }
    }

    // package.json#module is a bundler/ESM convention, not a CommonJS require entry.
    // Keep require() aligned with Node-compatible exports/main/index semantics.
    let Some(main) = package_json.get("main").and_then(|value| value.as_str()) else {
        return Ok(None);
    };
    let main_candidate = package_root.join(main);
    resolve_path_candidate(&main_candidate, conditions)
}

fn resolve_package_subpath(
    package_root: &Path,
    subpath: &str,
    conditions: &[&str],
) -> Result<Option<PathBuf>, CommonJsResolveError> {
    let Some(package_json) = read_package_json(package_root)? else {
        return resolve_path_candidate(&package_root.join(subpath), conditions);
    };

    let Some(exports) = package_json.get("exports") else {
        return resolve_path_candidate(&package_root.join(subpath), conditions);
    };
    let export_key = format!("./{}", subpath.trim_start_matches('/'));
    let Some(exports_obj) = exports.as_object() else {
        return Err(CommonJsResolveError::package_path_not_exported(
            package_root,
            &export_key,
        ));
    };
    validate_package_exports_config(package_root, exports_obj)?;

    if let Some(target) = exports_obj.get(&export_key) {
        return match resolve_package_export_value(package_root, target, conditions)? {
            PackageExportResolution::Resolved(resolved) => Ok(Some(resolved)),
            PackageExportResolution::InvalidTarget(target) => Err(
                CommonJsResolveError::invalid_package_target(package_root, &export_key, &target),
            ),
            PackageExportResolution::NotFound | PackageExportResolution::Blocked => Err(
                CommonJsResolveError::package_path_not_exported(package_root, &export_key),
            ),
        };
    }

    match resolve_package_pattern_export(package_root, exports_obj, &export_key, conditions)? {
        PackageExportResolution::Resolved(resolved) => Ok(Some(resolved)),
        PackageExportResolution::InvalidTarget(target) => Err(
            CommonJsResolveError::invalid_package_target(package_root, &export_key, &target),
        ),
        PackageExportResolution::NotFound | PackageExportResolution::Blocked => Err(
            CommonJsResolveError::package_path_not_exported(package_root, &export_key),
        ),
    }
}

fn read_package_json(
    package_root: &Path,
) -> Result<Option<serde_json::Value>, CommonJsResolveError> {
    if let Ok(cache) = PACKAGE_JSON_CACHE.read() {
        if let Some(cached) = cache.get(package_root) {
            return Ok(cached.clone());
        }
    }

    let package_json_path = package_root.join("package.json");
    if !package_json_path.is_file() {
        if let Ok(mut cache) = PACKAGE_JSON_CACHE.write() {
            cache.insert(package_root.to_path_buf(), None);
        }
        return Ok(None);
    }

    // Sandbox may deny parent package.json (e.g. repo root while running a fixture).
    // Treat that as "no package.json here" so type/exports probing does not abort startup.
    if crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::FileSystem,
        crate::permissions::PermissionAction::Read,
        crate::permissions::ResourceId::Path(package_json_path.clone()),
    )
    .is_err()
    {
        if let Ok(mut cache) = PACKAGE_JSON_CACHE.write() {
            cache.insert(package_root.to_path_buf(), None);
        }
        return Ok(None);
    }

    let content = fs::read_to_string(&package_json_path).map_err(|error| {
        CommonJsResolveError::invalid_package_config(
            package_root,
            format!("failed to read package.json: {error}"),
        )
    })?;
    let package_json: serde_json::Value = serde_json::from_str(&content).map_err(|error| {
        CommonJsResolveError::invalid_package_config(package_root, error.to_string())
    })?;

    if let Ok(mut cache) = PACKAGE_JSON_CACHE.write() {
        cache.insert(package_root.to_path_buf(), Some(package_json.clone()));
    }

    Ok(Some(package_json))
}

fn resolve_package_root_exports(
    package_root: &Path,
    exports: &serde_json::Value,
    conditions: &[&str],
) -> Result<Option<PackageExportResolution>, CommonJsResolveError> {
    if exports.is_string() || exports.is_array() || exports.is_null() {
        return Ok(Some(resolve_package_export_value(
            package_root,
            exports,
            conditions,
        )?));
    }

    let Some(exports_obj) = exports.as_object() else {
        return Ok(Some(PackageExportResolution::InvalidTarget(
            exports.to_string(),
        )));
    };
    validate_package_exports_config(package_root, exports_obj)?;
    let Some(root_target) = exports_obj.get(".") else {
        if exports_obj.keys().any(|key| key.starts_with('.')) {
            return Ok(Some(PackageExportResolution::NotFound));
        }
        return Ok(Some(resolve_package_export_value(
            package_root,
            exports,
            conditions,
        )?));
    };

    Ok(Some(resolve_package_export_value(
        package_root,
        root_target,
        conditions,
    )?))
}

fn validate_package_exports_config(
    package_root: &Path,
    exports_obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), CommonJsResolveError> {
    let has_subpath_key = exports_obj.keys().any(|key| key.starts_with('.'));
    let has_condition_key = exports_obj.keys().any(|key| !key.starts_with('.'));

    if has_subpath_key && has_condition_key {
        return Err(CommonJsResolveError::invalid_package_config(
            package_root,
            "exports cannot mix subpath keys and condition keys",
        ));
    }

    Ok(())
}

fn resolve_package_export_value(
    package_root: &Path,
    target: &serde_json::Value,
    conditions: &[&str],
) -> Result<PackageExportResolution, CommonJsResolveError> {
    resolve_package_export_value_with_pattern(package_root, target, None, conditions)
}

fn resolve_package_export_value_with_pattern(
    package_root: &Path,
    target: &serde_json::Value,
    pattern_capture: Option<&str>,
    conditions: &[&str],
) -> Result<PackageExportResolution, CommonJsResolveError> {
    if target.is_null() {
        return Ok(PackageExportResolution::Blocked);
    }

    if let Some(target) = target.as_str() {
        let target = if let Some(pattern_capture) = pattern_capture {
            target.replace('*', pattern_capture)
        } else {
            target.to_string()
        };
        return resolve_package_string_target(package_root, &target, conditions);
    }

    if let Some(targets) = target.as_array() {
        if targets.is_empty() {
            return Ok(PackageExportResolution::Blocked);
        }

        let mut last_fallback = PackageExportResolution::NotFound;
        for target in targets {
            match resolve_package_export_value_with_pattern(
                package_root,
                target,
                pattern_capture,
                conditions,
            )? {
                PackageExportResolution::Resolved(resolved) => {
                    return Ok(PackageExportResolution::Resolved(resolved));
                }
                PackageExportResolution::InvalidTarget(target) => {
                    last_fallback = PackageExportResolution::InvalidTarget(target);
                }
                PackageExportResolution::Blocked => {
                    return Ok(PackageExportResolution::Blocked);
                }
                PackageExportResolution::NotFound => {
                    if !matches!(last_fallback, PackageExportResolution::InvalidTarget(_)) {
                        last_fallback = PackageExportResolution::NotFound;
                    }
                }
            }
        }
        return Ok(last_fallback);
    }

    if let Some(target_obj) = target.as_object() {
        for (condition, condition_target) in target_obj {
            if is_export_condition(condition, conditions) {
                match resolve_package_export_value_with_pattern(
                    package_root,
                    condition_target,
                    pattern_capture,
                    conditions,
                )? {
                    PackageExportResolution::Resolved(resolved) => {
                        return Ok(PackageExportResolution::Resolved(resolved));
                    }
                    PackageExportResolution::Blocked => {
                        return Ok(PackageExportResolution::Blocked)
                    }
                    PackageExportResolution::InvalidTarget(target) => {
                        return Ok(PackageExportResolution::InvalidTarget(target));
                    }
                    PackageExportResolution::NotFound => {}
                }
            }
        }
        return Ok(PackageExportResolution::NotFound);
    }

    Ok(PackageExportResolution::InvalidTarget(target.to_string()))
}

fn resolve_package_pattern_export(
    package_root: &Path,
    exports_obj: &serde_json::Map<String, serde_json::Value>,
    export_key: &str,
    conditions: &[&str],
) -> Result<PackageExportResolution, CommonJsResolveError> {
    let mut best_match: Option<(&serde_json::Value, String, (usize, usize))> = None;

    for (pattern_key, target) in exports_obj {
        if let Some((capture, score)) = match_package_pattern(pattern_key, export_key, "./") {
            if best_match
                .as_ref()
                .map(|(_, _, best_score)| score > *best_score)
                .unwrap_or(true)
            {
                best_match = Some((target, capture, score));
            }
        }
    }

    if let Some((target, capture, _score)) = best_match {
        if is_invalid_package_export_pattern_capture(&capture) {
            return Err(CommonJsResolveError::invalid_module_specifier(
                package_root,
                export_key,
            ));
        }
        return resolve_package_export_value_with_pattern(
            package_root,
            target,
            Some(&capture),
            conditions,
        );
    }

    Ok(PackageExportResolution::NotFound)
}

fn resolve_package_pattern_import(
    package_root: &Path,
    imports_obj: &serde_json::Map<String, serde_json::Value>,
    specifier: &str,
    conditions: &[&str],
) -> Result<PackageImportResolution, CommonJsResolveError> {
    let mut best_match: Option<(&serde_json::Value, String, (usize, usize))> = None;

    for (pattern_key, target) in imports_obj {
        if let Some((capture, score)) = match_package_pattern(pattern_key, specifier, "#") {
            if best_match
                .as_ref()
                .map(|(_, _, best_score)| score > *best_score)
                .unwrap_or(true)
            {
                best_match = Some((target, capture, score));
            }
        }
    }

    if let Some((target, capture, _score)) = best_match {
        if is_invalid_package_export_pattern_capture(&capture) {
            return Err(CommonJsResolveError::invalid_module_specifier(
                package_root,
                specifier,
            ));
        }
        return resolve_package_import_value_with_pattern(
            package_root,
            target,
            Some(&capture),
            conditions,
        );
    }

    Ok(PackageImportResolution::NotFound)
}

fn match_package_pattern(
    pattern_key: &str,
    match_key: &str,
    required_prefix: &str,
) -> Option<(String, (usize, usize))> {
    if pattern_key.matches('*').count() != 1 {
        return None;
    }

    let (prefix, suffix) = pattern_key.split_once('*')?;
    if !prefix.starts_with(required_prefix) || !match_key.starts_with(prefix) || match_key == prefix
    {
        return None;
    }
    if !suffix.is_empty() && (!match_key.ends_with(suffix) || match_key.len() < pattern_key.len()) {
        return None;
    }

    let capture_start = prefix.len();
    let capture_end = match_key.len().checked_sub(suffix.len())?;
    if capture_end <= capture_start {
        return None;
    }

    Some((
        match_key[capture_start..capture_end].to_string(),
        (prefix.len(), pattern_key.len()),
    ))
}

fn resolve_package_string_target(
    package_root: &Path,
    target: &str,
    conditions: &[&str],
) -> Result<PackageExportResolution, CommonJsResolveError> {
    if target.is_empty() {
        return Ok(PackageExportResolution::InvalidTarget(target.to_string()));
    }

    if !target.starts_with("./") {
        return Ok(PackageExportResolution::InvalidTarget(target.to_string()));
    }

    let Some(target_remainder) = target.strip_prefix("./") else {
        return Ok(PackageExportResolution::InvalidTarget(target.to_string()));
    };
    if target_remainder.is_empty() {
        return Ok(PackageExportResolution::InvalidTarget(target.to_string()));
    }

    let target_path = Path::new(target_remainder);
    if target_path.is_absolute()
        || target_path
            .components()
            .any(|component| is_invalid_package_export_target_segment(&component))
    {
        return Ok(PackageExportResolution::InvalidTarget(target.to_string()));
    }

    let Some(resolved) = resolve_path_candidate(&package_root.join(target_path), conditions)?
    else {
        return Ok(PackageExportResolution::NotFound);
    };
    let canonical_root =
        fs::canonicalize(package_root).unwrap_or_else(|_| package_root.to_path_buf());
    if resolved.starts_with(&canonical_root) {
        Ok(PackageExportResolution::Resolved(resolved))
    } else {
        Ok(PackageExportResolution::InvalidTarget(target.to_string()))
    }
}

fn is_invalid_package_export_target_segment(component: &Component<'_>) -> bool {
    match component {
        Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
            true
        }
        Component::Normal(segment) => *segment == OsStr::new("node_modules"),
    }
}

fn is_invalid_package_export_pattern_capture(capture: &str) -> bool {
    capture
        .split(['/', '\\'])
        .any(is_invalid_package_export_pattern_capture_segment)
}

fn is_invalid_package_export_pattern_capture_segment(segment: &str) -> bool {
    segment.is_empty()
        || segment == "."
        || segment == ".."
        || segment.eq_ignore_ascii_case("node_modules")
}

fn canonical_file(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path)
        .ok()
        .or_else(|| Some(path.to_path_buf()))
}
