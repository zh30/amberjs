//! Host-agnostic `amber` discovery and `amber lsp` argv assembly.
//!
//! The WASM `Extension` impl supplies I/O (`Worktree::which`, settings,
//! `current_platform`). Tests inject fakes so they run without Zed.

/// Launch plan for `amber lsp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspLaunch {
    pub command: String,
    pub args: Vec<String>,
}

/// Inputs the WASM `language_server_command` method feeds into discovery.
pub struct WhichLookup<'a> {
    pub configured_path: Option<&'a str>,
    pub windows: bool,
    pub which: &'a dyn Fn(&str) -> Option<String>,
    pub extra_args: &'a [String],
}

/// Binary names `Worktree::which` should try, in order.
pub fn candidate_binary_names(windows: bool) -> &'static [&'static str] {
    if windows {
        &["amber.exe", "amber"]
    } else {
        &["amber"]
    }
}

/// Resolve a `amber` binary from an explicit runtime path or a which-lookup.
///
/// Does not read `std::env::var("PATH")`; callers pass `Worktree::which`.
pub fn resolve_amber_binary(
    configured_path: Option<&str>,
    windows: bool,
    which: impl Fn(&str) -> Option<String>,
) -> Result<String, String> {
    if let Some(path) = configured_path.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(path.to_string());
    }
    for name in candidate_binary_names(windows) {
        if let Some(found) = which(name) {
            return Ok(found);
        }
    }
    Err(
        "Amber `amber` binary not found. Install Amber so `amber` is on PATH, or set `lsp.amber-lsp.binary.path` in Zed settings."
            .to_string(),
    )
}

/// Build argv so the first argument is always `lsp`.
pub fn assemble_lsp_command(binary: String, extra_args: &[String]) -> LspLaunch {
    let mut args = Vec::new();
    if extra_args.first().map(String::as_str) != Some("lsp") {
        args.push("lsp".to_string());
    }
    args.extend(extra_args.iter().cloned());
    LspLaunch {
        command: binary,
        args,
    }
}

/// Shipped assembly used by `Extension::language_server_command`.
pub fn build_language_server_command(lookup: WhichLookup<'_>) -> Result<LspLaunch, String> {
    let binary = resolve_amber_binary(lookup.configured_path, lookup.windows, lookup.which)?;
    Ok(assemble_lsp_command(binary, lookup.extra_args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn fake_bee(dir: &Path) -> String {
        let path = dir.join("amber");
        fs::write(&path, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).unwrap();
        }
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn found_on_path_argv_starts_with_binary_then_lsp() {
        let dir = tempfile::tempdir().unwrap();
        let amber = fake_bee(dir.path());
        let amber_for_which = amber.clone();
        let which = move |name: &str| {
            if name == "amber" {
                Some(amber_for_which.clone())
            } else {
                None
            }
        };
        let launch = build_language_server_command(WhichLookup {
            configured_path: None,
            windows: false,
            which: &which,
            extra_args: &[],
        })
        .expect("amber on PATH");
        assert_eq!(launch.command, amber);
        assert_eq!(launch.args.first().map(String::as_str), Some("lsp"));
        assert_eq!(launch.args, vec!["lsp".to_string()]);
    }

    #[test]
    fn missing_binary_returns_actionable_error() {
        let which = |_name: &str| None;
        let err = build_language_server_command(WhichLookup {
            configured_path: None,
            windows: false,
            which: &which,
            extra_args: &[],
        })
        .expect_err("missing amber");
        assert!(
            err.contains("not found"),
            "error should say the binary is missing: {err}"
        );
        assert!(
            err.contains("PATH") && err.contains("lsp.amber-lsp.binary.path"),
            "error should tell the user how to install or configure: {err}"
        );
    }

    #[test]
    fn configured_runtime_path_wins_over_which() {
        let which = |_name: &str| Some("/from/which/amber".to_string());
        let launch = build_language_server_command(WhichLookup {
            configured_path: Some("/explicit/amber"),
            windows: false,
            which: &which,
            extra_args: &[],
        })
        .unwrap();
        assert_eq!(launch.command, "/explicit/amber");
        assert_eq!(launch.args, vec!["lsp".to_string()]);
    }

    #[test]
    fn extra_args_append_after_lsp() {
        let which = |_name: &str| Some("/opt/amber".to_string());
        let extra = vec!["--verbose".to_string()];
        let launch = build_language_server_command(WhichLookup {
            configured_path: None,
            windows: false,
            which: &which,
            extra_args: &extra,
        })
        .unwrap();
        assert_eq!(
            launch.args,
            vec!["lsp".to_string(), "--verbose".to_string()]
        );
    }

    #[test]
    fn windows_tries_amber_exe_first() {
        let which = |name: &str| {
            if name == "amber.exe" {
                Some(r"C:\Tools\amber.exe".to_string())
            } else {
                None
            }
        };
        let launch = build_language_server_command(WhichLookup {
            configured_path: None,
            windows: true,
            which: &which,
            extra_args: &[],
        })
        .unwrap();
        assert_eq!(launch.command, r"C:\Tools\amber.exe");
        assert_eq!(candidate_binary_names(true), &["amber.exe", "amber"]);
    }
}
