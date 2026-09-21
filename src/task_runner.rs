//! Task Runner for executing package.json scripts
//!
//! Provides lightweight, npm-free task execution for projects with package.json scripts.
//! Automatically prioritizes local `node_modules/.bin` and the active `amber` executable in PATH.

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus};

/// Finds the nearest `package.json` by searching upward from `start_dir`.
pub fn find_package_json(start_dir: &Path) -> Option<PathBuf> {
    let mut current = if start_dir.is_file() {
        start_dir.parent()?.to_path_buf()
    } else {
        start_dir.to_path_buf()
    };

    loop {
        let candidate = current.join("package.json");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Reads the `scripts` object from a given `package.json` path.
pub fn load_scripts(package_json_path: &Path) -> Result<BTreeMap<String, String>> {
    let content = std::fs::read_to_string(package_json_path)
        .map_err(|e| anyhow!("Failed to read {}: {}", package_json_path.display(), e))?;

    let json: Value = serde_json::from_str(&content)
        .map_err(|e| anyhow!("Failed to parse {}: {}", package_json_path.display(), e))?;

    let mut scripts = BTreeMap::new();
    if let Some(scripts_obj) = json.get("scripts").and_then(|s| s.as_object()) {
        for (name, cmd_val) in scripts_obj {
            if let Some(cmd_str) = cmd_val.as_str() {
                scripts.insert(name.clone(), cmd_str.to_string());
            }
        }
    }

    Ok(scripts)
}

/// Builds an enriched `PATH` environment variable string including `node_modules/.bin`
/// and the directory of the current executable.
pub fn build_enriched_path(project_dir: &Path) -> String {
    let current_path = env::var("PATH").unwrap_or_default();
    let local_bin = project_dir.join("node_modules").join(".bin");

    let mut paths = Vec::new();

    // 1. Local project node_modules/.bin
    if local_bin.exists() {
        paths.push(local_bin.to_string_lossy().to_string());
    }

    // 2. Directory containing the current `amber` executable
    if let Ok(current_exe) = env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            paths.push(exe_dir.to_string_lossy().to_string());
        }
    }

    // 3. Existing system PATH
    if !current_path.is_empty() {
        paths.push(current_path);
    }

    #[cfg(windows)]
    let separator = ";";
    #[cfg(not(windows))]
    let separator = ":";

    paths.join(separator)
}

/// Executes a script by name defined in the nearest package.json.
pub fn run_script(
    start_dir: &Path,
    script_name: &str,
    extra_args: &[String],
) -> Result<ExitStatus> {
    let package_json = find_package_json(start_dir).ok_or_else(|| {
        anyhow!(
            "No package.json found in {} or any parent directory",
            start_dir.display()
        )
    })?;

    let project_dir = package_json.parent().unwrap_or_else(|| Path::new("."));

    let scripts = load_scripts(&package_json)?;
    let raw_cmd = scripts.get(script_name).ok_or_else(|| {
        let available = if scripts.is_empty() {
            "No scripts defined in package.json".to_string()
        } else {
            let names: Vec<&String> = scripts.keys().collect();
            format!(
                "Available scripts: {}",
                names
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        anyhow!(
            "Script '{}' not found in {}.\n{}",
            script_name,
            package_json.display(),
            available
        )
    })?;

    let mut full_cmd = raw_cmd.clone();
    if !extra_args.is_empty() {
        full_cmd.push(' ');
        full_cmd.push_str(&extra_args.join(" "));
    }

    let enriched_path = build_enriched_path(project_dir);

    #[cfg(windows)]
    let mut command = {
        let mut cmd = ProcessCommand::new("cmd");
        cmd.args(["/C", &full_cmd]);
        cmd
    };

    #[cfg(not(windows))]
    let mut command = {
        let mut cmd = ProcessCommand::new("sh");
        cmd.args(["-c", &full_cmd]);
        cmd
    };

    command.current_dir(project_dir);
    command.env("PATH", enriched_path);
    command.stdin(std::process::Stdio::inherit());
    command.stdout(std::process::Stdio::inherit());
    command.stderr(std::process::Stdio::inherit());

    let status = command
        .status()
        .map_err(|e| anyhow!("Failed to execute script '{}': {}", script_name, e))?;

    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_find_package_json_and_load_scripts() {
        let dir = tempdir().expect("tempdir");
        let pkg = dir.path().join("package.json");
        std::fs::write(
            &pkg,
            r#"{"name": "test-pkg", "scripts": {"hello": "echo hello", "build": "echo build"}}"#,
        )
        .expect("write");

        let found = find_package_json(dir.path()).expect("should find package.json");
        assert_eq!(found, pkg);

        let sub_dir = dir.path().join("sub").join("deep");
        std::fs::create_dir_all(&sub_dir).expect("create sub dirs");
        let found_nested = find_package_json(&sub_dir).expect("should find in parent");
        assert_eq!(found_nested, pkg);

        let scripts = load_scripts(&pkg).expect("load scripts");
        assert_eq!(scripts.get("hello"), Some(&"echo hello".to_string()));
        assert_eq!(scripts.get("build"), Some(&"echo build".to_string()));
    }
}
