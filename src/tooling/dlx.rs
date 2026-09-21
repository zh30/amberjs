//! Dynamic on-demand package runner for Amber (`amber x` / `amber dlx`).
//!
//! Fetches and executes packages from npm registry on the fly without local installation,
//! caching binaries globally in `~/.amberjs/x_cache/` for instant subsequent execution.

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tar::Archive;

/// Get the global cache directory for `amber x`
pub fn get_x_cache_dir() -> PathBuf {
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join(".amberjs").join("x_cache")
}

/// Parse a package specifier like "cowsay", "cowsay@1.5.0", "@biomejs/biome@1.8.0"
pub fn parse_pkg_spec(spec: &str) -> (String, Option<String>) {
    if spec.starts_with('@') {
        // Scoped package
        if let Some(idx) = spec[1..].find('@') {
            let name = &spec[..idx + 1];
            let version = &spec[idx + 2..];
            (name.to_string(), Some(version.to_string()))
        } else {
            (spec.to_string(), None)
        }
    } else if let Some((name, ver)) = spec.split_once('@') {
        (name.to_string(), Some(ver.to_string()))
    } else {
        (spec.to_string(), None)
    }
}

/// Sanitize package name for filesystem path
fn sanitize_cache_name(name: &str, version: &str) -> String {
    let clean_name = name.replace('/', "__").replace('@', "");
    format!("{}@{}", clean_name, version)
}

/// Fetch and extract package to cache directory, returning the package root directory
pub fn ensure_package_cached(pkg_spec: &str) -> Result<PathBuf> {
    let (name, ver_opt) = parse_pkg_spec(pkg_spec);
    let cache_base = get_x_cache_dir();
    fs::create_dir_all(&cache_base)?;

    let target_version = ver_opt.unwrap_or_else(|| "latest".to_string());
    let folder_name = sanitize_cache_name(&name, &target_version);
    let target_dir = cache_base.join(&folder_name);

    if target_dir.join("package.json").exists() {
        return Ok(target_dir);
    }

    // Download from npm registry
    let client = reqwest::blocking::Client::builder()
        .user_agent("amberjs-x/1.4.0")
        .build()?;

    // 1. Resolve package metadata
    let metadata_url = format!("https://registry.npmjs.org/{}", name);
    let resp = client.get(&metadata_url).send()?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch package '{}' from registry (HTTP {})",
            name,
            resp.status()
        ));
    }

    let meta: Value = resp.json()?;
    let resolved_version = if target_version == "latest" {
        meta.get("dist-tags")
            .and_then(|t| t.get("latest"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No latest version found for package '{}'", name))?
    } else {
        &target_version
    };

    let tarball_url = meta
        .get("versions")
        .and_then(|v| v.get(resolved_version))
        .and_then(|v| v.get("dist"))
        .and_then(|d| d.get("tarball"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| anyhow!("Tarball URL not found for {}@{}", name, resolved_version))?;

    // 2. Download tarball
    let tarball_resp = client.get(tarball_url).send()?;
    let tarball_bytes = tarball_resp.bytes()?;

    // 3. Extract to target directory
    let decoder = GzDecoder::new(&tarball_bytes[..]);
    let mut archive = Archive::new(decoder);

    fs::create_dir_all(&target_dir)?;

    for entry_res in archive.entries()? {
        let mut entry = entry_res?;
        let path = entry.path()?.into_owned();

        // npm tarballs package under "package/..."
        let relative_path = if let Ok(stripped) = path.strip_prefix("package") {
            stripped.to_path_buf()
        } else {
            path
        };

        let dest_file = target_dir.join(relative_path);
        if let Some(parent) = dest_file.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(dest_file)?;
    }

    Ok(target_dir)
}

/// Find binary executable from package.json
pub fn resolve_package_bin(package_dir: &Path, pkg_name: &str) -> Result<PathBuf> {
    let pkg_json_path = package_dir.join("package.json");
    if !pkg_json_path.exists() {
        return Err(anyhow!(
            "package.json not found in {}",
            package_dir.display()
        ));
    }

    let content = fs::read_to_string(&pkg_json_path)?;
    let pkg_info: Value = serde_json::from_str(&content)?;

    let bin_val = pkg_info
        .get("bin")
        .ok_or_else(|| anyhow!("Package '{}' has no 'bin' field", pkg_name))?;

    let relative_bin = if let Some(bin_str) = bin_val.as_str() {
        bin_str.to_string()
    } else if let Some(bin_obj) = bin_val.as_object() {
        // Find matching package name or first entry
        let clean_pkg_name = pkg_name.split('/').last().unwrap_or(pkg_name);
        if let Some(found) = bin_obj.get(clean_pkg_name).and_then(|v| v.as_str()) {
            found.to_string()
        } else if let Some((_, first_val)) = bin_obj.iter().next() {
            first_val
                .as_str()
                .ok_or_else(|| anyhow!("Invalid bin definition"))?
                .to_string()
        } else {
            return Err(anyhow!("Empty bin map in package.json"));
        }
    } else {
        return Err(anyhow!("Unsupported bin field in package.json"));
    };

    let bin_path = package_dir.join(relative_bin);
    if !bin_path.exists() {
        return Err(anyhow!("Binary not found at {}", bin_path.display()));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&bin_path) {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o755);
            let _ = fs::set_permissions(&bin_path, perms);
        }
    }

    Ok(bin_path)
}

/// Run a package directly with `amber x`
pub fn run_dlx(package_spec: &str, args: &[String]) -> Result<i32> {
    println!("⚡ amber x: Resolving package '{}'...", package_spec);
    let (name, _) = parse_pkg_spec(package_spec);

    let pkg_dir = ensure_package_cached(package_spec)?;
    let bin_path = resolve_package_bin(&pkg_dir, &name)?;

    // Check if the file is a JavaScript or TypeScript script
    let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("amber"));

    let is_js = bin_path.extension().map_or(false, |ext| {
        ext == "js" || ext == "mjs" || ext == "cjs" || ext == "ts"
    });

    let has_node_shebang = fs::read_to_string(&bin_path)
        .map(|s| s.starts_with("#!") && (s.contains("node") || s.contains("amber")))
        .unwrap_or(false);

    let mut cmd = if is_js || has_node_shebang {
        let mut c = Command::new(&current_exe);
        c.arg("run");
        c.arg(&bin_path);
        c
    } else {
        Command::new(&bin_path)
    };

    cmd.args(args);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    // Inject package_dir node_modules into NODE_PATH
    let mut node_path = std::env::var("NODE_PATH").unwrap_or_default();
    let local_nm = pkg_dir.join("node_modules").to_string_lossy().to_string();
    if !node_path.is_empty() {
        node_path.push(':');
    }
    node_path.push_str(&local_nm);
    cmd.env("NODE_PATH", node_path);

    let status = cmd.status()?;
    Ok(status.code().unwrap_or(0))
}
