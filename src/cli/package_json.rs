// Package.json Integration Module
/// Stage 36.0 - 实现 package.json 解析和脚本执行
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};
/// Package.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageJson {
    /// Package name
    pub name: Option<String>,
    /// Package version
    pub version: Option<String>,
    /// Package description
    pub description: Option<String>,
    /// Scripts to run
    pub scripts: Option<HashMap<String, String>>,
    /// Dependencies
    pub dependencies: Option<HashMap<String, String>>,
    /// Dev dependencies
    pub dev_dependencies: Option<HashMap<String, String>>,
    /// Amber specific configuration
    pub amberjs: Option<AmberConfig>,
}
/// Amber-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmberConfig {
    /// Entry point for the application
    pub entry: Option<String>,
    /// Optimization mode
    pub optimize: Option<String>,
    /// Target ECMAScript version
    pub target: Option<String>,
    /// Watch mode settings
    pub watch: Option<WatchConfig>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
}
/// Watch mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Paths to watch
    pub paths: Option<Vec<String>>,
    /// Extensions to watch
    pub extensions: Option<Vec<String>>,
    /// Directories to ignore
    pub ignore: Option<Vec<String>>,
    /// Polling interval in milliseconds
    pub interval: Option<u64>,
}
impl PackageJson {
    /// Load package.json from a directory
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let package_path: _ = dir.join("package.json");
        if !package_path.exists() {
            return Err(anyhow::anyhow!("package.json not found in {:?}", dir).into());
        }
        let content: _ = fs::read_to_string(&package_path)?;
        let package: PackageJson = serde_json::from_str(&content)?;
        Ok(package)
    }
    /// Load package.json from a specific file path
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Err(anyhow::anyhow!("package.json not found: {:?}", path).into());
        }
        let content: _ = fs::read_to_string(path)?;
        let package: PackageJson = serde_json::from_str(&content)?;
        Ok(package)
    }
    /// Get a script command
    pub fn get_script(&self, script_name: &str) -> Option<&String> {
        self.scripts.as_ref()?.get(script_name)
    }
    /// Get all scripts
    pub fn get_scripts(&self) -> HashMap<String, String> {
        self.scripts.as_ref().cloned().unwrap_or_default()
    }
    /// Get entry point
    pub fn get_entry(&self) -> Option<PathBuf> {
        if let Some(amberjs_config) = &self.amberjs {
            if let Some(entry) = &amberjs_config.entry {
                return Some(PathBuf::from(entry));
            }
        }
        // Default entry points
        let default_entries: _ = ["src/index.js", "src/index.ts", "index.js", "main.js"];
        for entry in &default_entries {
            let path: _ = PathBuf::from(entry);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }
    /// Get amberjs configuration
    pub fn get_amberjs_config(&self) -> Option<&AmberConfig> {
        self.amberjs.as_ref()
    }
    /// Get optimization mode
    pub fn get_optimize_mode(&self) -> Option<&str> {
        if let Some(amberjs_config) = &self.amberjs {
            if let Some(optimize) = &amberjs_config.optimize {
                return Some(optimize.as_str());
            }
        }
        None
    }
    /// Get target ECMAScript version
    pub fn get_target(&self) -> Option<&str> {
        if let Some(amberjs_config) = &self.amberjs {
            if let Some(target) = &amberjs_config.target {
                return Some(target.as_str());
            }
        }
        None
    }
    /// Get watch configuration
    pub fn get_watch_config(&self) -> Option<&WatchConfig> {
        if let Some(amberjs_config) = &self.amberjs {
            if let Some(watch) = &amberjs_config.watch {
                return Some(watch);
            }
        }
        None
    }
    /// Get environment variables
    pub fn get_env_vars(&self) -> HashMap<String, String> {
        if let Some(amberjs_config) = &self.amberjs {
            if let Some(env) = &amberjs_config.env {
                return env.clone();
            }
        }
        HashMap::new()
    }
    /// Parse a script command into arguments
    pub fn parse_script_command(&self, script_name: &str) -> anyhow::Result<Vec<String>> {
        let script: _ = self.get_script(script_name)
            .ok_or_else(|| anyhow::anyhow!("Script '{}' not found", script_name))?;
        // Simple script parsing - split by spaces but respect quotes
        let mut args = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = '\0';
        for ch in script.chars() {
            if ch == '"' || ch == '\'' {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quotes = false;
                    quote_char = '\0';
                } else {
                    current.push(ch);
                }
            } else if ch == ' ' && !in_quotes {
                if !current.is_empty() {
                    args.push(current);
                    current = String::new();
                }
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            args.push(current);
        }
        Ok(args)
    }
    /// Validate package.json structure
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.is_none() {
            return Err(anyhow::anyhow!("package.json missing 'name' field").into());
        }
        if self.version.is_none() {
            return Err(anyhow::anyhow!("package.json missing 'version' field").into());
        }
        // Validate amberjs configuration
        if let Some(amberjs_config) = &self.amberjs {
            if let Some(entry) = &amberjs_config.entry {
                let entry_path: _ = Path::new(entry);
                if !entry_path.exists() && !entry_path.is_relative() {
                    return Err(anyhow::anyhow!("amberjs.entry file does not exist: {}", entry).into());
                }
            }
            if let Some(optimize) = &amberjs_config.optimize {
                let valid_optimize: _ = ["speed", "size", "auto"].contains(&optimize.as_str());
                if !valid_optimize {
                    return Err(anyhow::anyhow!(
                        "amberjs.optimize must be one of: speed, size, auto, got: {}",
                        optimize
                    ).into());
                }
            }
            if let Some(target) = &amberjs_config.target {
                let valid_targets: _ = ["es2015", "es2016", "es2017", "es2018", "es2019", "es2020", "es2021", "es2022"];
                if !valid_targets.contains(&target.as_str()) {
                    return Err(anyhow::anyhow!(
                        "amberjs.target must be a valid ES version, got: {}",
                        target
                    ).into());
                }
            }
        }
        Ok(())
    }
    /// Convert to JSON string
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    /// Save to file
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let json: _ = self.to_json()?;
        fs::write(path, json)?;
        Ok(())
    }
}
/// Script executor
pub struct ScriptExecutor {
    package: PackageJson,
    base_dir: PathBuf,
}
impl ScriptExecutor {
    /// Create a new script executor
    pub fn new(package: PackageJson, base_dir: PathBuf) -> Self {
        Self {
            package,
            base_dir,
        }
    }
    /// Execute a script
    pub async fn run_script(&self, script_name: &str) -> anyhow::Result<std::process::ExitStatus> {
        let args: _ = self.package.parse_script_command(script_name)?;
        if args.is_empty() {
            return Err(anyhow::anyhow!("Script '{}' has no command", script_name).into());
        }
        // Resolve the command
        let cmd: _ = &args[0];
        let cmd_args: _ = &args[1..];
        // If it's a amber command, use current executable
        let exec_path: _ = if cmd == "amber" {
            std::env::current_exe()?
        } else {
            // For now, only support amber commands
            return Err(anyhow::anyhow!("Only 'amber' commands are supported in scripts").into());
        };
        // Spawn the process
        let mut child = std::process::Command::new(&exec_path)
            .args(cmd_args)
            .current_dir(&self.base_dir)
            .spawn()?;
        Ok(child.wait()?)
    }
    /// List all available scripts
    pub fn list_scripts(&self) -> Vec<(String, String)> {
        self.package
            .scripts
            .as_ref()
            .map(|scripts| {
                scripts.iter()
                    .map(|(name, cmd)| (name.clone(), cmd.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
use anyhow::{Result, Error};
    #[test]
    fn test_package_json_load() {
        let temp_dir: _ = tempdir().expect("Failed to create temp dir");
        let package_json: _ = temp_dir.path().join("package.json");
        let content: _ = r#"{
            "name": "test-app",
            "version": "1.0.0",
            "scripts": {
                "start": "amber src/index.js",
                "dev": "amber watch src/index.js"
            }
        }"#;
        std::fs::write(&package_json, content).expect("Failed to write package.json");
        let pkg: _ = PackageJson::load_from_path(&package_json).expect("Failed to load package.json");
        assert_eq!(pkg.name, Some("test-app".to_string()));
        assert_eq!(pkg.version, Some("1.0.0".to_string()));
        let scripts: _ = pkg.get_scripts();
        assert_eq!(scripts.get("start"), Some(&"amber src/index.js".to_string()));
        assert_eq!(scripts.get("dev"), Some(&"amber watch src/index.js".to_string()));
        temp_dir.close().expect("Failed to close temp dir");
    }
    #[test]
    fn test_amberjs_config() {
        let temp_dir: _ = tempdir().expect("Failed to create temp dir");
        let package_json: _ = temp_dir.path().join("package.json");
        let content: _ = r#"{
            "name": "test-app",
            "version": "1.0.0",
            "amberjs": {
                "entry": "src/index.ts",
                "optimize": "aggressive",
                "target": "es2020"
            }
        }"#;
        std::fs::write(&package_json, content).expect("Failed to write package.json");
        let pkg: _ = PackageJson::load_from_path(&package_json).expect("Failed to load package.json");
        let config: _ = pkg.get_amberjs_config().expect("No amberjs config");
        assert_eq!(config.entry, Some("src/index.ts".to_string()));
        assert_eq!(config.optimize, Some("aggressive".to_string()));
        assert_eq!(config.target, Some("es2020".to_string()));
        temp_dir.close().expect("Failed to close temp dir");
    }
    #[test]
    fn test_script_parsing() {
        let pkg: _ = PackageJson {
            name: Some("test".to_string()),
            version: Some("1.0.0".to_string()),
            description: None,
            scripts: Some([("start".to_string(), "amber src/index.js --watch".to_string())].into()),
            dependencies: None,
            dev_dependencies: None,
            amberjs: None,
        };
        let args: _ = pkg.parse_script_command("start").expect("Failed to parse script");
        assert_eq!(args, vec!["amber", "src/index.js", "--watch"]);
    }
    #[test]
    fn test_package_json_validation() {
        // Valid package.json
        let valid_pkg: _ = PackageJson {
            name: Some("test-app".to_string()),
            version: Some("1.0.0".to_string()),
            description: None,
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            amberjs: None,
        };
        assert!(valid_pkg.validate().is_ok());
        // Invalid package.json - missing name
        let invalid_pkg: _ = PackageJson {
            name: None,
            version: Some("1.0.0".to_string()),
            description: None,
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            amberjs: None,
        };
        assert!(invalid_pkg.validate().is_err());
    }
}
