use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn extension_toml_registers_js_ts_and_beejs() {
    let toml = fs::read_to_string(root().join("extension.toml")).unwrap();
    assert!(
        toml.contains("[language_servers.bee-lsp]"),
        "must declare [language_servers.bee-lsp]: {toml}"
    );
    assert!(
        toml.contains("name = \"Beejs LSP\""),
        "language server needs a name: {toml}"
    );
    for lang in ["JavaScript", "TypeScript", "Beejs"] {
        assert!(
            toml.contains(&format!("\"{lang}\"")),
            "languages must include {lang}: {toml}"
        );
    }
}

#[test]
fn language_server_command_calls_shipped_helper_via_worktree_which() {
    let lib = fs::read_to_string(root().join("src/lib.rs")).unwrap();
    let command = fs::read_to_string(root().join("src/command.rs")).unwrap();
    assert!(lib.contains("fn language_server_command"));
    assert!(
        lib.contains("build_language_server_command"),
        "Extension::language_server_command must call the shipped helper"
    );
    assert!(
        lib.contains("worktree.which") && lib.contains("current_platform"),
        "I/O edge must use Worktree::which and current_platform: {lib}"
    );
    let command_code: String = command
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect();
    assert!(
        !command_code.contains("std::env::var"),
        "command assembly must not read env vars"
    );
    assert!(
        !lib.contains("std::env::var(\"PATH\")") && !lib.contains("std::env::var(\"Path\")"),
        "language_server_command must not use std::env::var for PATH: {lib}"
    );
}
