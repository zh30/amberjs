//! WICG Import Maps Implementation for Amber
//!
//! Provides bare module specifier remapping and scoped resolution.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportMap {
    #[serde(default)]
    pub imports: HashMap<String, String>,
    #[serde(default)]
    pub scopes: HashMap<String, HashMap<String, String>>,
    #[serde(skip)]
    pub base_dir: PathBuf,
}

impl ImportMap {
    /// Load and parse an import map file from the given path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read import map at '{}': {}", path.display(), e))?;
        let mut map: ImportMap = serde_json::from_str(&content).map_err(|e| {
            anyhow!(
                "Failed to parse import map JSON at '{}': {}",
                path.display(),
                e
            )
        })?;

        map.base_dir = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Ok(map)
    }

    /// Resolves a specifier to a file path or URL string.
    pub fn resolve(&self, specifier: &str, referrer: Option<&Path>) -> Option<String> {
        // 1. Check scoped imports first if referrer is provided
        if let Some(ref_path) = referrer {
            let ref_str = ref_path.to_string_lossy();
            for (scope_prefix, scope_map) in &self.scopes {
                if ref_str.contains(scope_prefix) {
                    if let Some(target) = Self::match_entry(scope_map, specifier, &self.base_dir) {
                        return Some(target);
                    }
                }
            }
        }

        // 2. Check top-level imports
        Self::match_entry(&self.imports, specifier, &self.base_dir)
    }

    fn match_entry(
        entries: &HashMap<String, String>,
        specifier: &str,
        base_dir: &Path,
    ) -> Option<String> {
        // Exact match
        if let Some(target) = entries.get(specifier) {
            return Some(Self::normalize_target(target, base_dir));
        }

        // Prefix match (e.g., "utils/" -> "./src/utils/")
        let mut best_match: Option<(&String, &String)> = None;
        for (key, target) in entries {
            if key.ends_with('/') && specifier.starts_with(key) {
                match best_match {
                    None => best_match = Some((key, target)),
                    Some((prev_key, _)) if key.len() > prev_key.len() => {
                        best_match = Some((key, target));
                    }
                    _ => {}
                }
            }
        }

        if let Some((prefix, target)) = best_match {
            let suffix = &specifier[prefix.len()..];
            let combined = format!("{}{}", target, suffix);
            return Some(Self::normalize_target(&combined, base_dir));
        }

        None
    }

    fn normalize_target(target: &str, base_dir: &Path) -> String {
        if target.starts_with("./") || target.starts_with("../") {
            let resolved = base_dir.join(target);
            resolved.to_string_lossy().to_string()
        } else {
            target.to_string()
        }
    }
}

static GLOBAL_IMPORT_MAP: once_cell::sync::Lazy<std::sync::RwLock<Option<ImportMap>>> =
    once_cell::sync::Lazy::new(|| std::sync::RwLock::new(None));

/// Set the process-global import map
pub fn set_global_import_map(map: Option<ImportMap>) {
    if let Ok(mut lock) = GLOBAL_IMPORT_MAP.write() {
        *lock = map;
    }
}

/// Get a clone of the process-global import map if set
pub fn get_global_import_map() -> Option<ImportMap> {
    GLOBAL_IMPORT_MAP.read().ok().and_then(|lock| lock.clone())
}

/// Resolve a specifier using the process-global import map
pub fn resolve_from_global_import_map(specifier: &str, referrer: Option<&Path>) -> Option<String> {
    GLOBAL_IMPORT_MAP.read().ok().and_then(|lock| {
        lock.as_ref()
            .and_then(|map| map.resolve(specifier, referrer))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_map_exact_and_prefix() {
        let mut imports = HashMap::new();
        imports.insert("lodash".to_string(), "./deps/lodash.js".to_string());
        imports.insert("pkg/".to_string(), "./src/pkg/".to_string());

        let map = ImportMap {
            imports,
            scopes: HashMap::new(),
            base_dir: PathBuf::from("/project"),
        };

        assert_eq!(
            map.resolve("lodash", None),
            Some("/project/./deps/lodash.js".to_string())
        );
        assert_eq!(
            map.resolve("pkg/math.js", None),
            Some("/project/./src/pkg/math.js".to_string())
        );
        assert_eq!(map.resolve("unknown", None), None);
    }
}
