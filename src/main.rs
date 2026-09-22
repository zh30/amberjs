//! Amber - High-performance JavaScript/TypeScript runtime
//! Built with Rust and V8

extern crate amberjs as amberjs;

use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};
use serde::Deserialize;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "amber")]
#[command(about = "Amber - JavaScript/TypeScript runtime built with Rust and V8")]
#[command(version)]
struct Cli {
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Args, Clone, Debug, Default)]
struct PermissionCliOptions {
    /// Load JavaScript permission policy from a JSON file
    #[arg(
        long = "permission-policy",
        visible_alias = "policy",
        value_name = "PATH"
    )]
    policy: Option<PathBuf>,
    /// Deny JavaScript file-system reads and writes unless explicitly allowed
    #[arg(long = "deny-fs")]
    deny_fs: bool,
    /// Deny JavaScript network connections unless explicitly allowed
    #[arg(long = "deny-net")]
    deny_net: bool,
    /// Deny JavaScript environment variable reads unless explicitly allowed
    #[arg(long = "deny-env")]
    deny_env: bool,
    /// Deny JavaScript child process execution unless explicitly allowed
    #[arg(long = "deny-run")]
    deny_run: bool,
    /// Deny all JavaScript I/O, then overlay --allow-* / --permission-policy
    #[arg(long = "sandbox")]
    sandbox: bool,
    /// Append ResourceBroker decisions as JSONL (kind/action/resource/decision only)
    #[arg(long = "audit-log", value_name = "PATH")]
    audit_log: Option<PathBuf>,
    /// Allow JavaScript file-system reads for an exact path (repeatable)
    #[arg(long = "allow-read", value_name = "PATH")]
    allow_read: Vec<PathBuf>,
    /// Allow JavaScript file-system writes for an exact path (repeatable)
    #[arg(long = "allow-write", value_name = "PATH")]
    allow_write: Vec<PathBuf>,
    /// Allow JavaScript network connections for a host or exact URL (repeatable)
    #[arg(long = "allow-net", value_name = "HOST_OR_URL")]
    allow_net: Vec<String>,
    /// Allow JavaScript network listeners for a host or exact URL (repeatable)
    #[arg(long = "allow-listen", value_name = "HOST_OR_URL")]
    allow_listen: Vec<String>,
    /// Allow JavaScript environment variable reads for an exact variable name (repeatable)
    #[arg(long = "allow-env", value_name = "NAME")]
    allow_env: Vec<String>,
    /// Allow JavaScript child process execution for an exact command name (repeatable)
    #[arg(long = "allow-run", value_name = "COMMAND")]
    allow_run: Vec<String>,
    /// Seed for deterministic PRNG (Math.random & crypto.getRandomValues)
    #[arg(long = "seed", value_name = "SEED")]
    seed: Option<u64>,
    /// Freeze virtual clock time to fixed timestamp or ISO8601 string (Date.now & performance.now)
    #[arg(long = "freeze-time", value_name = "TIMESTAMP_OR_ISO")]
    freeze_time: Option<String>,
    /// Maximum script execution time in milliseconds before watchdog termination
    #[arg(long = "timeout", value_name = "MILLISECONDS")]
    timeout: Option<u64>,
    /// Maximum isolate heap memory in megabytes
    #[arg(long = "max-memory", value_name = "MB")]
    max_memory: Option<usize>,
    /// Path to an import map JSON file for bare specifier remapping
    #[arg(long = "import-map", value_name = "PATH")]
    import_map: Option<PathBuf>,
    /// Enable in-memory virtual filesystem sandbox for safe agent execution
    #[arg(long = "virtual-fs", visible_alias = "vfs")]
    virtual_fs: bool,
    /// Disable host disk Copy-on-Write fallback for virtual filesystem (pure isolated RAM)
    #[arg(long = "virtual-fs-strict")]
    virtual_fs_strict: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct PermissionPolicyFile {
    permissions: PermissionPolicyRules,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct PermissionPolicyRules {
    deny_fs: bool,
    deny_net: bool,
    deny_env: bool,
    deny_run: bool,
    allow_read: Vec<PathBuf>,
    allow_write: Vec<PathBuf>,
    allow_net: Vec<String>,
    allow_listen: Vec<String>,
    allow_env: Vec<String>,
    allow_run: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run a script file
    Run {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Script file to execute
        file: PathBuf,
        /// Arguments to pass to the script
        args: Vec<String>,
        /// Enable watch mode (hot reload)
        #[arg(short, long)]
        watch: bool,
        /// Debounce time in milliseconds for watch mode
        #[arg(long, default_value = "100")]
        debounce: u64,
        /// WebSocket port for hot reload notifications
        #[arg(short = 'p', long, default_value = "9999")]
        websocket_port: u16,
        /// Import a module before other modules are loaded (can be used multiple times)
        #[arg(short = 'r', long = "preload", value_name = "MODULE")]
        preloads: Vec<String>,
        /// Alias of --preload for Node.js compatibility
        #[arg(long = "require", value_name = "MODULE")]
        require: Vec<String>,
        /// Print exported tool schemas as JSON and exit
        #[arg(long = "export-tools")]
        export_tools: bool,
        /// Number of parallel multi-isolate worker threads for parallel HTTP execution (default: 1, or via AMBER_WORKERS)
        #[arg(short = 'W', long = "workers", default_value = "1")]
        workers: usize,
        /// Enable V8 Inspector agent for Chrome DevTools / VS Code debugging
        #[arg(long)]
        inspect: bool,
        /// Enable V8 Inspector agent and break at beginning of user script
        #[arg(long = "inspect-brk")]
        inspect_brk: bool,
        /// Port for V8 Inspector agent (default: 9229)
        #[arg(long = "inspect-port", default_value = "9229")]
        inspect_port: u16,
        /// Ignored for one-shot CLI. Isolate pooling is used by `amber test`.
        #[arg(long = "warm", hide = true)]
        warm: bool,
    },
    /// JSON-RPC session over stdin/stdout for Agent hosts
    Session {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Tool module entry (JS/TS)
        file: PathBuf,
        /// New isolate + core APIs for every tools/call
        #[arg(long = "isolate-per-call")]
        isolate_per_call: bool,
    },
    /// Model Context Protocol (MCP) server execution and inspection
    Mcp {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Tool module entry (JS/TS, optional with --inspect)
        file: Option<PathBuf>,
        /// New isolate + core APIs for every tools/call
        #[arg(long = "isolate-per-call")]
        isolate_per_call: bool,
        /// Inspect exposed tools instead of starting stdio server
        #[arg(short, long)]
        inspect: bool,
    },
    /// Evaluate JavaScript code
    Eval {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// JavaScript code to execute
        code: String,
        /// Ignored for one-shot CLI. Isolate pooling is used by `amber test`.
        #[arg(long = "warm", hide = true)]
        warm: bool,
    },
    /// Run in REPL mode
    Repl,
    /// Run tests
    Test {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Test file to run (optional)
        file: Option<PathBuf>,
        /// Filter tests by name pattern (regex)
        #[arg(short = 't', long = "test-name-pattern")]
        test_name_pattern: Option<String>,
        /// Only run tests matching pattern (shorthand for --test-name-pattern)
        #[arg(short = 'n', long = "test-only", conflicts_with = "test_skip")]
        test_only: Option<String>,
        /// Skip tests matching pattern
        #[arg(long = "test-skip")]
        test_skip: Option<String>,
        /// Bail on first failure
        #[arg(short = 'b', long = "bail")]
        bail: bool,
        /// Run tests in parallel
        #[arg(long = "parallel")]
        parallel: bool,
        /// Update missing or mismatched file snapshots
        #[arg(long = "update-snapshots")]
        update_snapshots: bool,
        /// Verbose output
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
        /// Watch files for changes and re-run tests
        #[arg(short = 'w', long = "watch")]
        watch: bool,
        /// Collect code coverage and output lcov report
        #[arg(long)]
        coverage: bool,
    },
    /// Bundle a local JS/TS module graph into one JS file
    Bundle {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Entry file to bundle
        entry: PathBuf,
        /// Output file path
        #[arg(short = 'o', long = "outfile", alias = "output")]
        outfile: Option<PathBuf>,
        /// Minify output
        #[arg(short, long)]
        minify: bool,
        /// Generate source map
        #[arg(long)]
        sourcemap: bool,
        /// Target environment
        #[arg(short = 't', long, default_value = "browser")]
        target: String,
        /// Accepted and ignored (no tree-shaking is performed)
        #[arg(long = "tree-shake")]
        tree_shake: bool,
    },
    /// Debug a script
    Debug {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Script file to debug
        file: PathBuf,
    },
    /// Record deterministic execution trace for an AI Agent or script
    Record {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Script file to execute and record
        file: PathBuf,
        /// Trace output JSON path (defaults to <file>.amber-trace.json)
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
        /// Arguments to pass to the script
        args: Vec<String>,
    },
    /// Replay an AI Agent or script execution trace with offline determinism
    Replay {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Trace JSON file to replay
        trace: PathBuf,
        /// Verify step consistency and report divergence
        #[arg(long)]
        verify: bool,
        /// Verbose trace event logging
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
    /// Display version information
    Version,
    /// Manage V8 startup snapshots for sub-millisecond cold start
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },
    /// Start HTTP/HTTPS server for a web app or script (e.g. app.ts / server.js)
    Serve {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Optional script file to serve (exports fetch handler or default app)
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,
        /// Port number
        #[arg(short, long, default_value = "3000")]
        port: u16,
        /// Host address
        #[arg(long, default_value = "localhost")]
        host: String,
        /// Enable HTTPS with TLS certificate
        #[arg(long)]
        https: bool,
        /// TLS certificate file path
        #[arg(long, requires = "https")]
        cert: Option<String>,
        /// TLS private key file path
        #[arg(long, requires = "https")]
        key: Option<String>,
    },
    /// Initialize new project
    Init {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Project name
        name: Option<String>,
    },
    /// Add dependency package
    Add {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Package name (with optional version, e.g., "lodash@4.17.21")
        package: String,
        /// Install exact version (no caret/tilde prefix)
        #[arg(long)]
        save_exact: bool,
        /// Install as devDependency
        #[arg(long)]
        dev: bool,
    },
    /// Remove dependency package
    Remove {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Package name to remove
        package: String,
    },
    /// Install dependencies from package.json
    Install {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Fail if package.json and package-lock.json are out of sync
        #[arg(long = "frozen-lockfile")]
        frozen_lockfile: bool,
    },
    /// Remove unused dependencies from node_modules
    Prune {
        #[command(flatten)]
        permissions: PermissionCliOptions,
    },
    /// Create new project
    Create {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Project name
        name: String,
        /// Template type (js/ts)
        #[arg(default_value = "js")]
        template: String,
    },
    /// Execute a package binary directly without local installation (like npx/bunx)
    #[command(alias = "dlx", alias = "bunx")]
    X {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Package name (with optional version, e.g., "cowsay@1.5.0")
        package: String,
        /// Arguments to pass to the package
        args: Vec<String>,
    },
    /// Generate production container (Docker), standalone binary, or Kubernetes deployment scaffolding
    Deploy {
        /// Target deployment environment (docker, standalone, k8s)
        #[arg(short = 't', long = "target", default_value = "docker")]
        target: String,
        /// Output directory
        #[arg(short = 'o', long = "output", default_value = ".")]
        output: PathBuf,
        /// Port to expose
        #[arg(short = 'p', long = "port", default_value = "3000")]
        port: u16,
        /// Application name override
        #[arg(long = "name")]
        name: Option<String>,
        /// Entrypoint script override
        #[arg(long = "entry")]
        entry: Option<PathBuf>,
    },
    /// Upgrade dependencies to latest versions
    Upgrade {
        #[command(flatten)]
        permissions: PermissionCliOptions,
        /// Package to upgrade (all if not specified)
        package: Option<String>,
    },
    /// Format JavaScript and TypeScript source files
    Fmt {
        /// Files or directories to format
        #[arg(default_value = ".")]
        files: Vec<PathBuf>,
        /// Check if files are formatted without writing
        #[arg(long)]
        check: bool,
    },
    /// Lint JavaScript and TypeScript source files
    Lint {
        /// Files or directories to lint
        #[arg(default_value = ".")]
        files: Vec<PathBuf>,
    },
    /// Run microbenchmarks
    Bench {
        /// Files or directories containing benchmarks
        #[arg(default_value = ".")]
        files: Vec<PathBuf>,
    },
    /// Compile a script into a standalone self-executing binary
    Compile {
        /// Entry script (JS/TS) to compile
        entry: PathBuf,
        /// Output binary path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Export TypeScript type definitions for Amber built-in APIs
    Types {
        /// Output file path (defaults to stdout)
        #[arg(short, long)]
        outfile: Option<PathBuf>,
    },
    /// Run a script task defined in package.json
    Task {
        /// Task name to run (lists all tasks if omitted)
        name: Option<String>,
        /// Arguments to pass to the task
        args: Vec<String>,
    },
    /// Profile a script and export a Chrome DevTools compatible CPU profile
    Profile {
        /// Script file to profile
        file: PathBuf,
        /// Output .cpuprofile file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Start Language Server Protocol (LSP) server for editor integration
    Lsp,
}

#[derive(Subcommand, Debug)]
enum SnapshotAction {
    /// Build or rebuild the startup snapshot
    Build,
    /// Display snapshot status, path, and size
    Status,
    /// Clean and remove the cached snapshot
    Clean,
}

/// Read and compile source code (JavaScript or TypeScript)
fn read_and_compile_source(file: &Path) -> Result<String> {
    let extension = file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let source = {
        check_file_read_permission(file)?;
        std::fs::read_to_string(file).map_err(|e| anyhow!("Failed to read file: {}", e))?
    };

    // If it's a TypeScript file, compile it
    if matches!(extension.as_str(), "ts" | "tsx" | "mts" | "cts" | "jsx") {
        match amberjs::typescript::compile_typescript(&source, &file.to_string_lossy()) {
            Ok(output) => {
                // Show diagnostics (warnings/errors)
                if !output.diagnostics.is_empty() {
                    for diagnostic in &output.diagnostics {
                        match diagnostic.severity {
                            amberjs::typescript::ErrorSeverity::Warning => {
                                eprintln!("⚠️  Warning: {}", diagnostic.message);
                            }
                            amberjs::typescript::ErrorSeverity::Error => {
                                eprintln!("❌ Error: {}", diagnostic.message);
                            }
                            amberjs::typescript::ErrorSeverity::Info => {
                                eprintln!("ℹ️  Info: {}", diagnostic.message);
                            }
                        }
                    }
                }
                let error_messages: Vec<&str> = output
                    .diagnostics
                    .iter()
                    .filter_map(|diagnostic| match diagnostic.severity {
                        amberjs::typescript::ErrorSeverity::Error => {
                            Some(diagnostic.message.as_str())
                        }
                        _ => None,
                    })
                    .collect();
                if !error_messages.is_empty() {
                    return Err(anyhow!(
                        "TypeScript compilation failed with {} error(s): {}",
                        error_messages.len(),
                        error_messages.join("; ")
                    ));
                }
                let mut compiled = format!(
                    "{}\n//# sourceURL={}",
                    output.js_code,
                    file.to_string_lossy()
                );
                if let Some(ref map) = output.source_map {
                    compiled.push_str(&amberjs::typescript::source_mapping_url_comment(map));
                    amberjs::runtime_minimal::set_active_source_map(map.clone());
                }
                Ok(compiled)
            }
            Err(e) => Err(anyhow!("TypeScript compilation failed: {}", e)),
        }
    } else {
        // Return JavaScript as-is
        Ok(source)
    }
}

fn normalize_create_args(name: String, template: String) -> (String, String) {
    match (name.as_str(), template.as_str()) {
        ("js" | "ts", actual_name) if actual_name != "js" && actual_name != "ts" => {
            (actual_name.to_string(), name)
        }
        _ => (name, template),
    }
}

fn allow_sandbox_entry_file(sandbox: bool, file: &Path) -> Result<()> {
    if !sandbox {
        return Ok(());
    }
    let mut broker = amberjs::permissions::global_resource_broker()
        .write()
        .map_err(|_| anyhow!("resource broker lock poisoned"))?;
    broker.allow(
        amberjs::permissions::PermissionKind::FileSystem,
        amberjs::permissions::PermissionAction::Read,
        amberjs::permissions::ResourceId::Path(file.to_path_buf()),
    );
    Ok(())
}

fn print_exported_tools(file: &Path) -> Result<()> {
    let tools = amberjs::agent::export_tools_from_entry(file)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&amberjs::agent::tools_list_json(&tools))?
    );
    Ok(())
}

fn apply_permission_cli_options(options: &PermissionCliOptions) -> Result<()> {
    use amberjs::permissions::{
        global_resource_broker, PermissionAction, PermissionKind, ResourceBroker, ResourceId,
    };

    let mut broker = global_resource_broker()
        .write()
        .map_err(|_| anyhow!("resource broker lock poisoned"))?;
    *broker = ResourceBroker::default();
    amberjs::permissions::reset_runtime_permission_state();
    amberjs::permissions::set_sandbox_strict_env(options.sandbox);
    if let Some(audit_log) = &options.audit_log {
        amberjs::permissions::set_audit_log_path(Some(audit_log.clone()))
            .map_err(|e| anyhow!(e))?;
    }
    amberjs::permissions::set_deterministic_seed(options.seed);
    if let Some(freeze_time_str) = &options.freeze_time {
        let ts = amberjs::permissions::parse_time_spec(freeze_time_str).map_err(|e| anyhow!(e))?;
        amberjs::permissions::set_frozen_time_ms(Some(ts));
    }
    if let Some(map_path) = &options.import_map {
        let map = amberjs::tooling::import_map::ImportMap::load(map_path)?;
        amberjs::tooling::import_map::set_global_import_map(Some(map));
    } else {
        amberjs::tooling::import_map::set_global_import_map(None);
    }

    if options.virtual_fs {
        let cow = !options.virtual_fs_strict;
        amberjs::sandbox::virtual_fs::enable(cow);
    } else {
        amberjs::sandbox::virtual_fs::disable();
    }

    if options.sandbox {
        broker.deny_all();
    }

    if let Some(policy_path) = &options.policy {
        apply_permission_policy_file(&mut broker, policy_path)?;
    }

    if options.deny_fs {
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Any,
        );
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Any,
        );
    }

    if options.deny_net {
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Any,
        );
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Listen,
            ResourceId::Any,
        );
    }

    if options.deny_env {
        broker.deny(
            PermissionKind::Environment,
            PermissionAction::Read,
            ResourceId::Any,
        );
    }

    if options.deny_run {
        broker.deny(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Any,
        );
    }

    for path in &options.allow_read {
        broker.allow(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(path.clone()),
        );
    }

    for path in &options.allow_write {
        broker.allow(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(path.clone()),
        );
    }

    for target in &options.allow_net {
        broker.allow(
            PermissionKind::Network,
            PermissionAction::Connect,
            network_resource_from_cli_target(target),
        );
    }

    for target in &options.allow_listen {
        broker.allow(
            PermissionKind::Network,
            PermissionAction::Listen,
            network_resource_from_cli_target(target),
        );
    }

    for name in &options.allow_env {
        broker.allow(
            PermissionKind::Environment,
            PermissionAction::Read,
            ResourceId::Name(name.clone()),
        );
    }

    for command in &options.allow_run {
        broker.allow(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Name(command.clone()),
        );
    }

    Ok(())
}

fn apply_permission_policy_file(
    broker: &mut amberjs::permissions::ResourceBroker,
    policy_path: &Path,
) -> Result<()> {
    let contents = std::fs::read_to_string(policy_path).map_err(|e| {
        anyhow!(
            "Failed to read permission policy {}: {}",
            policy_path.display(),
            e
        )
    })?;
    let policy = parse_permission_policy(policy_path, &contents)?;
    let base_dir = policy_path.parent().unwrap_or_else(|| Path::new("."));
    apply_permission_policy_rules(broker, &policy.permissions, base_dir);
    Ok(())
}

fn parse_permission_policy(policy_path: &Path, contents: &str) -> Result<PermissionPolicyFile> {
    serde_json::from_str(contents).map_err(|e| {
        anyhow!(
            "Failed to parse permission policy {} as JSON: {}",
            policy_path.display(),
            e
        )
    })
}

fn apply_permission_policy_rules(
    broker: &mut amberjs::permissions::ResourceBroker,
    rules: &PermissionPolicyRules,
    base_dir: &Path,
) {
    use amberjs::permissions::{PermissionAction, PermissionKind, ResourceId};

    if rules.deny_fs {
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Any,
        );
        broker.deny(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Any,
        );
    }

    if rules.deny_net {
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Connect,
            ResourceId::Any,
        );
        broker.deny(
            PermissionKind::Network,
            PermissionAction::Listen,
            ResourceId::Any,
        );
    }

    if rules.deny_env {
        broker.deny(
            PermissionKind::Environment,
            PermissionAction::Read,
            ResourceId::Any,
        );
    }

    if rules.deny_run {
        broker.deny(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Any,
        );
    }

    for path in &rules.allow_read {
        broker.allow(
            PermissionKind::FileSystem,
            PermissionAction::Read,
            ResourceId::Path(resolve_policy_path(base_dir, path)),
        );
    }

    for path in &rules.allow_write {
        broker.allow(
            PermissionKind::FileSystem,
            PermissionAction::Write,
            ResourceId::Path(resolve_policy_path(base_dir, path)),
        );
    }

    for target in &rules.allow_net {
        broker.allow(
            PermissionKind::Network,
            PermissionAction::Connect,
            network_resource_from_cli_target(target),
        );
    }

    for target in &rules.allow_listen {
        broker.allow(
            PermissionKind::Network,
            PermissionAction::Listen,
            network_resource_from_cli_target(target),
        );
    }

    for name in &rules.allow_env {
        broker.allow(
            PermissionKind::Environment,
            PermissionAction::Read,
            ResourceId::Name(name.clone()),
        );
    }

    for command in &rules.allow_run {
        broker.allow(
            PermissionKind::Process,
            PermissionAction::Execute,
            ResourceId::Name(command.clone()),
        );
    }
}

fn resolve_policy_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn network_resource_from_cli_target(target: &str) -> amberjs::permissions::ResourceId {
    if target.contains("://") {
        amberjs::permissions::ResourceId::Url(target.to_string())
    } else {
        amberjs::permissions::ResourceId::Name(target.to_string())
    }
}

fn bundle_cli_fail(err: impl std::fmt::Display) -> ! {
    let msg = err.to_string();
    if msg.contains(amberjs::tooling::bundler::BUNDLE_ERROR_PREFIX) {
        eprintln!("{msg}");
    } else {
        eprintln!("{} {msg}", amberjs::tooling::bundler::BUNDLE_ERROR_PREFIX);
    }
    std::process::exit(1);
}

fn check_file_read_permission(path: &Path) -> Result<()> {
    amberjs::permissions::check_global_permission(
        amberjs::permissions::PermissionKind::FileSystem,
        amberjs::permissions::PermissionAction::Read,
        amberjs::permissions::ResourceId::Path(path.to_path_buf()),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

fn check_file_write_permission(path: &Path) -> Result<()> {
    amberjs::permissions::check_global_permission(
        amberjs::permissions::PermissionKind::FileSystem,
        amberjs::permissions::PermissionAction::Write,
        amberjs::permissions::ResourceId::Path(path.to_path_buf()),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

fn check_network_listen_permission(target: &str) -> Result<()> {
    amberjs::permissions::check_global_permission(
        amberjs::permissions::PermissionKind::Network,
        amberjs::permissions::PermissionAction::Listen,
        network_resource_from_cli_target(target),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

fn check_network_connect_permission(target: &str) -> Result<()> {
    amberjs::permissions::check_global_permission(
        amberjs::permissions::PermissionKind::Network,
        amberjs::permissions::PermissionAction::Connect,
        network_resource_from_cli_target(target),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

fn check_process_execute_permission(command: &str) -> Result<()> {
    amberjs::permissions::check_global_permission(
        amberjs::permissions::PermissionKind::Process,
        amberjs::permissions::PermissionAction::Execute,
        amberjs::permissions::ResourceId::Name(command.to_string()),
    )
    .map_err(|e| anyhow!(e.to_string()))
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SemverTriple {
    major: u64,
    minor: u64,
    patch: u64,
}

fn validate_frozen_lockfile(package_data: &serde_json::Value, lock_path: &Path) -> Result<()> {
    if !lock_path.exists() {
        return Err(anyhow!(
            "frozen lockfile requires package-lock.json to exist"
        ));
    }

    check_file_read_permission(lock_path)?;
    let lock_content = std::fs::read_to_string(lock_path)
        .map_err(|e| anyhow!("Failed to read package-lock.json: {}", e))?;
    let lock: amberjs::package_manager::PackageLock = serde_json::from_str(&lock_content)
        .map_err(|e| anyhow!("Failed to parse package-lock.json: {}", e))?;
    let locked_deps = lock.dependencies.unwrap_or_default();

    for section in ["dependencies", "devDependencies", "optionalDependencies"] {
        let Some(deps) = package_data
            .get(section)
            .and_then(|value| value.as_object())
        else {
            continue;
        };

        for (name, requested_value) in deps {
            let requested = requested_value.as_str().ok_or_else(|| {
                anyhow!(
                    "frozen lockfile cannot validate non-string dependency '{}' in {}",
                    name,
                    section
                )
            })?;
            let locked = locked_deps.get(name).ok_or_else(|| {
                anyhow!(
                    "frozen lockfile mismatch for package '{}': missing from package-lock.json",
                    name
                )
            })?;

            if !dependency_request_matches_locked_version(requested, &locked.version) {
                return Err(anyhow!(
                    "frozen lockfile mismatch for package '{}': package.json requests '{}' but package-lock.json locks '{}'",
                    name,
                    requested,
                    locked.version
                ));
            }
        }
    }

    Ok(())
}

fn dependency_request_matches_locked_version(requested: &str, locked: &str) -> bool {
    let requested = requested.trim();
    let locked = locked.trim();
    if requested == "*" || requested.eq_ignore_ascii_case("latest") {
        return true;
    }

    if requested == locked {
        return true;
    }

    if let Some(base) = requested.strip_prefix('^') {
        return semver_caret_matches(base, locked);
    }
    if let Some(base) = requested.strip_prefix('~') {
        return semver_tilde_matches(base, locked);
    }
    if let Some(base) = requested.strip_prefix('=') {
        return locked == base.trim();
    }

    false
}

fn semver_caret_matches(base: &str, locked: &str) -> bool {
    let Some(base) = parse_semver_triplet(base) else {
        return false;
    };
    let Some(locked) = parse_semver_triplet(locked) else {
        return false;
    };

    if locked < base {
        return false;
    }
    if base.major > 0 {
        return locked.major == base.major;
    }
    if base.minor > 0 {
        return locked.major == 0 && locked.minor == base.minor;
    }
    locked.major == 0 && locked.minor == 0 && locked.patch == base.patch
}

fn semver_tilde_matches(base: &str, locked: &str) -> bool {
    let Some(base) = parse_semver_triplet(base) else {
        return false;
    };
    let Some(locked) = parse_semver_triplet(locked) else {
        return false;
    };

    locked >= base && locked.major == base.major && locked.minor == base.minor
}

fn parse_semver_triplet(version: &str) -> Option<SemverTriple> {
    let core = version
        .trim()
        .trim_start_matches('v')
        .split(['-', '+'])
        .next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some(SemverTriple {
        major,
        minor,
        patch,
    })
}

fn startup_trace_enabled() -> bool {
    std::env::var_os("AMBER_TRACE_STARTUP").is_some()
}

fn startup_mark(start: Instant, phase: &str) {
    if startup_trace_enabled() {
        eprintln!(
            "amber_startup {:<18} {:>8.3} ms",
            phase,
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}

fn build_process_argv(file: &Path, args: &[String]) -> Vec<String> {
    let mut argv = vec!["amber".to_string(), file.to_string_lossy().into_owned()];
    argv.extend(args.iter().cloned());
    argv
}

fn preload_require_source(preload: &str) -> Result<String> {
    let preload_path = Path::new(preload);
    let specifier = if preload_path.exists() {
        preload_path
            .canonicalize()
            .map_err(|e| anyhow!("Failed to resolve preload file {}: {}", preload, e))?
            .to_string_lossy()
            .to_string()
    } else {
        preload.to_string()
    };
    let specifier = serde_json::to_string(&specifier)
        .map_err(|e| anyhow!("Failed to encode preload specifier: {}", e))?;
    Ok(format!("require({specifier});"))
}

fn snapshot_path_for_test_file(test_file: &Path) -> PathBuf {
    let file_name = test_file
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "test.js".to_string());
    let base_dir = test_file.parent().unwrap_or_else(|| Path::new("."));
    base_dir
        .join("__snapshots__")
        .join(format!("{}.snap", file_name))
}

fn read_snapshot_content(test_file: &Path) -> Result<(PathBuf, Option<String>)> {
    let snapshot_path = snapshot_path_for_test_file(test_file);
    if !snapshot_path.is_file() {
        return Ok((snapshot_path, None));
    }

    check_file_read_permission(&snapshot_path)?;
    let content = std::fs::read_to_string(&snapshot_path).map_err(|e| {
        anyhow!(
            "Failed to read snapshot file {}: {}",
            snapshot_path.display(),
            e
        )
    })?;
    Ok((snapshot_path, Some(content)))
}

#[derive(Debug, Deserialize)]
struct TestFileRunResult {
    summary: String,
    #[serde(default, rename = "snapshotUpdated")]
    snapshot_updated: bool,
    #[serde(default, rename = "snapshotContent")]
    snapshot_content: Option<String>,
    #[serde(default, rename = "inlineSnapshotUpdated")]
    inline_snapshot_updated: bool,
    #[serde(default, rename = "inlineSnapshotUpdates")]
    inline_snapshot_updates: Vec<InlineSnapshotUpdate>,
}

#[derive(Debug, Deserialize)]
struct InlineSnapshotUpdate {
    index: usize,
    content: String,
}

fn write_snapshot_content(snapshot_path: &Path, content: &str) -> Result<()> {
    let snapshot_dir = snapshot_path
        .parent()
        .ok_or_else(|| anyhow!("Snapshot path has no parent: {}", snapshot_path.display()))?;
    check_file_write_permission(snapshot_dir)?;
    check_file_write_permission(snapshot_path)?;
    std::fs::create_dir_all(snapshot_dir).map_err(|e| {
        anyhow!(
            "Failed to create snapshot directory {}: {}",
            snapshot_dir.display(),
            e
        )
    })?;
    std::fs::write(snapshot_path, content).map_err(|e| {
        anyhow!(
            "Failed to write snapshot file {}: {}",
            snapshot_path.display(),
            e
        )
    })
}

fn escape_inline_snapshot_content(content: &str) -> String {
    content
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('$', "\\$")
}

fn find_matching_paren(source: &str, open_paren_index: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for (offset, ch) in source[open_paren_index..].char_indices() {
        let index = open_paren_index + offset;

        if let Some(quote_char) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' | '`' => quote = Some(ch),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }

    None
}

fn apply_inline_snapshot_updates(source: &str, updates: &[InlineSnapshotUpdate]) -> Result<String> {
    if updates.is_empty() {
        return Ok(source.to_string());
    }

    let marker = ".toMatchInlineSnapshot(";
    let mut call_index = 0usize;
    let mut search_start = 0usize;
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();

    while let Some(relative_start) = source[search_start..].find(marker) {
        let marker_start = search_start + relative_start;
        let open_paren = marker_start + marker.len() - 1;
        let close_paren = find_matching_paren(source, open_paren).ok_or_else(|| {
            anyhow!(
                "Failed to find closing parenthesis for inline snapshot call {}",
                call_index + 1
            )
        })?;

        call_index += 1;
        if let Some(update) = updates.iter().find(|update| update.index == call_index) {
            let escaped = escape_inline_snapshot_content(&update.content);
            replacements.push((open_paren + 1, close_paren, format!("`\n{}\n`", escaped)));
        }

        search_start = close_paren + 1;
    }

    for update in updates {
        if update.index == 0 || update.index > call_index {
            return Err(anyhow!(
                "Inline snapshot update referenced call {}, but only found {} inline snapshot call(s)",
                update.index,
                call_index
            ));
        }
    }

    let mut updated_source = source.to_string();
    for (start, end, replacement) in replacements.into_iter().rev() {
        updated_source.replace_range(start..end, &replacement);
    }
    Ok(updated_source)
}

fn write_inline_snapshot_source(test_file: &Path, updates: &[InlineSnapshotUpdate]) -> Result<()> {
    check_file_write_permission(test_file)?;
    check_file_read_permission(test_file)?;
    let source = std::fs::read_to_string(test_file)
        .map_err(|e| anyhow!("Failed to read test file {}: {}", test_file.display(), e))?;
    let updated_source = apply_inline_snapshot_updates(&source, updates)?;
    std::fs::write(test_file, updated_source)
        .map_err(|e| anyhow!("Failed to write test file {}: {}", test_file.display(), e))
}

fn process_test_file_result(
    test_file: &Path,
    snapshot_path: &Path,
    options: &TestFileOptions,
    raw_result: String,
) -> Result<String> {
    let trimmed = raw_result.trim();
    let Ok(run_result) = serde_json::from_str::<TestFileRunResult>(trimmed) else {
        return Ok(raw_result);
    };

    if run_result.inline_snapshot_updated {
        if !options.update_snapshots {
            return Err(anyhow!(
                "inline snapshot update requested without --update-snapshots"
            ));
        }
        write_inline_snapshot_source(test_file, &run_result.inline_snapshot_updates)?;
    }

    if run_result.snapshot_updated {
        let content = run_result
            .snapshot_content
            .ok_or_else(|| anyhow!("snapshot update result omitted snapshot content"))?;
        if !options.update_snapshots {
            return Err(anyhow!(
                "snapshot update requested without --update-snapshots"
            ));
        }
        write_snapshot_content(snapshot_path, &content)?;
    }

    Ok(run_result.summary)
}

fn execute_test_file(test_file: &Path, options: &TestFileOptions) -> Result<String> {
    let code = read_and_compile_source(test_file)?;
    let (snapshot_path, snapshot_content) = read_snapshot_content(test_file)?;
    let code = wrap_test_source(&code, options, &snapshot_path, snapshot_content.as_deref());
    let mut runtime =
        amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_process_argv(build_process_argv(test_file, &[]));
    let runtime_test_path = test_file.with_extension("amber-test.cjs");
    runtime.set_main_module_path(&runtime_test_path);
    runtime.set_timer_drain_limit_ms(options.timeout_seconds.unwrap_or(30).saturating_mul(1000));

    let result = runtime.execute_code(&code)?;
    if result.trim() == "[object Promise]" {
        return Err(anyhow!(
            "test run did not settle before the configured timeout"
        ));
    }

    process_test_file_result(test_file, &snapshot_path, options, result)
}

#[derive(Clone, Debug)]
struct TestFileOptions {
    include_pattern: Option<String>,
    skip_pattern: Option<String>,
    bail: bool,
    timeout_seconds: Option<u64>,
    update_snapshots: bool,
}

fn wrap_test_source(
    source: &str,
    options: &TestFileOptions,
    snapshot_path: &Path,
    snapshot_content: Option<&str>,
) -> String {
    let config = serde_json::json!({
        "includePattern": options.include_pattern.as_deref().unwrap_or(""),
        "skipPattern": options.skip_pattern.as_deref().unwrap_or(""),
        "bail": options.bail,
        "timeoutSeconds": options.timeout_seconds.unwrap_or(0),
        "updateSnapshots": options.update_snapshots,
        "snapshotPath": snapshot_path.to_string_lossy().to_string(),
        "snapshotContent": snapshot_content,
    });

    let mut wrapped = format!(
        "// @amberjs-no-runtime-typescript-transpile\nlet __amberjsTestConfig = {};\n",
        config
    );
    wrapped.push_str(
        r#"
let __amberjsTestPassed = 0;
let __amberjsTestFailed = 0;
let __amberjsTestSkipped = 0;
let __amberjsTestErrors = [];
let __amberjsTestQueue = [];
let __amberjsDescribeStack = [];
let __amberjsSuiteCounter = 0;
let __amberjsSuiteRegistry = {};
let __amberjsSuiteOrder = [];
let __amberjsRemainingSuiteTests = {};
let __amberjsStartedSuites = {};
let __amberjsFinishedSuites = {};
let __amberjsFailedBeforeAllSuites = {};
let __amberjsMockFunctions = [];
let __amberjsSpyRestorers = [];
let __amberjsCustomMatchers = Object.create(null);
let __amberjsCurrentTestName = "";
let __amberjsAssertionCount = 0;
let __amberjsExpectedAssertionCount = undefined;
let __amberjsHasAssertionExpectation = false;
let __amberjsSnapshotCounters = {};
let __amberjsSnapshotUpdates = {};
let __amberjsInlineSnapshotCounter = 0;
let __amberjsInlineSnapshotUpdates = [];
let __amberjsRootHooks = {
  id: "root",
  name: "",
  beforeAll: [],
  beforeEach: [],
  afterEach: [],
  afterAll: []
};
__amberjsSuiteRegistry[__amberjsRootHooks.id] = __amberjsRootHooks;
__amberjsSuiteOrder.push(__amberjsRootHooks.id);

function __amberjsNormalizeSnapshotText(value) {
  let text = String(value);
  if (text.startsWith("\n")) {
    text = text.slice(1);
  }
  if (text.endsWith("\n")) {
    text = text.slice(0, -1);
  }
  return text;
}

function __amberjsUnescapeSnapshotLiteral(value) {
  return String(value)
    .replace(/\\`/g, "`")
    .replace(/\\\$/g, "$")
    .replace(/\\\\/g, "\\");
}

function __amberjsEscapeSnapshotLiteral(value) {
  return String(value)
    .replace(/\\/g, "\\\\")
    .replace(/`/g, "\\`")
    .replace(/\$/g, "\\$");
}

function __amberjsParseSnapshots(content) {
  const snapshots = {};
  if (typeof content !== "string" || content.length === 0) {
    return snapshots;
  }
  const pattern = /exports\[`((?:\\`|[^`])+)`\]\s*=\s*`([\s\S]*?)`;/g;
  let match;
  while ((match = pattern.exec(content)) !== null) {
    const key = __amberjsUnescapeSnapshotLiteral(match[1]);
    snapshots[key] = __amberjsNormalizeSnapshotText(__amberjsUnescapeSnapshotLiteral(match[2]));
  }
  return snapshots;
}

const __amberjsSnapshots = __amberjsParseSnapshots(__amberjsTestConfig.snapshotContent || "");

function __amberjsBuildSnapshotFileContent() {
  const merged = {};
  for (const key of Object.keys(__amberjsSnapshots)) {
    merged[key] = __amberjsSnapshots[key];
  }
  for (const key of Object.keys(__amberjsSnapshotUpdates)) {
    merged[key] = __amberjsSnapshotUpdates[key];
  }

  return Object.keys(merged).sort().map((key) => {
    const escapedKey = __amberjsEscapeSnapshotLiteral(key);
    const escapedValue = __amberjsEscapeSnapshotLiteral(merged[key]);
    return `exports[\`${escapedKey}\`] = \`\n${escapedValue}\n\`;\n`;
  }).join("\n");
}

function __amberjsFormatValue(value) {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (__amberjsIsMap(value)) {
    return `Map ${__amberjsFormatValue(Array.from(value.entries()))}`;
  }
  if (__amberjsIsSet(value)) {
    return `Set ${__amberjsFormatValue(Array.from(value.values()))}`;
  }
  try {
    const json = JSON.stringify(value);
    return json === undefined ? String(value) : json;
  } catch (_) {
    return String(value);
  }
}

function __amberjsRecordFailure(name, error) {
  __amberjsTestFailed++;
  const message = error && error.message ? error.message : String(error);
  const line = `${name}: ${message}`;
  __amberjsTestErrors.push(line);
  console.error(`FAIL ${line}`);
}

function __amberjsPatternMatches(pattern, name, suite) {
  if (pattern === "") {
    return true;
  }
  let regex;
  try {
    regex = new RegExp(String(pattern));
  } catch (error) {
    throw new Error(`Invalid test pattern ${JSON.stringify(pattern)}: ${error.message}`);
  }
  return regex.test(String(name)) || regex.test(String(suite || ""));
}

function __amberjsCurrentHookFrame() {
  if (__amberjsDescribeStack.length === 0) {
    return __amberjsRootHooks;
  }
  return __amberjsDescribeStack[__amberjsDescribeStack.length - 1];
}

function __amberjsCreateSuiteFrame(name) {
  return __amberjsCreateSuiteFrameWithOptions(name, {});
}

function __amberjsCreateSuiteFrameWithOptions(name, options) {
  const frame = {
    id: `suite:${++__amberjsSuiteCounter}`,
    name: String(name),
    skip: Boolean(options && options.skip),
    only: Boolean(options && options.only),
    beforeAll: [],
    beforeEach: [],
    afterEach: [],
    afterAll: []
  };
  __amberjsSuiteRegistry[frame.id] = frame;
  __amberjsSuiteOrder.push(frame.id);
  return frame;
}

function __amberjsCurrentSuiteHasFlag(flag) {
  return __amberjsDescribeStack.some((frame) => Boolean(frame[flag]));
}

function __amberjsCaptureSuiteIds() {
  return [__amberjsRootHooks].concat(__amberjsDescribeStack).map((frame) => frame.id);
}

function __amberjsCaptureBeforeEachHooks() {
  let hooks = __amberjsRootHooks.beforeEach.slice();
  for (const frame of __amberjsDescribeStack) {
    hooks = hooks.concat(frame.beforeEach);
  }
  return hooks;
}

function __amberjsCaptureAfterEachHooks() {
  let hooks = [];
  for (let i = __amberjsDescribeStack.length - 1; i >= 0; i--) {
    hooks = hooks.concat(__amberjsDescribeStack[i].afterEach);
  }
  return hooks.concat(__amberjsRootHooks.afterEach);
}

function __amberjsQueueTest(name, callback, options) {
  const suite = __amberjsDescribeStack.map((frame) => frame.name).join(" ");
  const suiteSkip = __amberjsCurrentSuiteHasFlag("skip");
  const skip = Boolean(options && options.skip) || suiteSkip;
  __amberjsTestQueue.push({
    name: String(name),
    suite,
    callback,
    skip,
    failing: Boolean(options && options.failing),
    only: !skip && (Boolean(options && options.only) || __amberjsCurrentSuiteHasFlag("only")),
    suiteIds: __amberjsCaptureSuiteIds(),
    beforeEachHooks: __amberjsCaptureBeforeEachHooks(),
    afterEachHooks: __amberjsCaptureAfterEachHooks()
  });
}

function test(name, callback) {
  if (typeof callback !== "function") {
    __amberjsRecordFailure(name, new Error("test callback must be a function"));
    return;
  }
  __amberjsQueueTest(name, callback, {});
}

test.skip = function testSkip(name, callback) {
  __amberjsQueueTest(name || "skipped test", callback, { skip: true });
};
test.only = function testOnly(name, callback) {
  if (typeof callback !== "function") {
    __amberjsRecordFailure(name, new Error("test callback must be a function"));
    return;
  }
  __amberjsQueueTest(name, callback, { only: true });
};
test.todo = function testTodo(name) {
  __amberjsQueueTest(name || "todo test", undefined, { skip: true });
};
test.failing = function testFailing(name, callback) {
  if (typeof callback !== "function") {
    __amberjsRecordFailure(name, new Error("test callback must be a function"));
    return;
  }
  __amberjsQueueTest(name, callback, { failing: true });
};

function __amberjsCreateConcurrentTest() {
  function concurrent(name, callback) {
    if (typeof callback !== "function") {
      __amberjsRecordFailure(name, new Error("test callback must be a function"));
      return;
    }
    __amberjsQueueTest(name, callback, {});
  }

  concurrent.skip = function concurrentSkip(name, callback) {
    __amberjsQueueTest(name || "skipped test", callback, { skip: true });
  };
  concurrent.only = function concurrentOnly(name, callback) {
    if (typeof callback !== "function") {
      __amberjsRecordFailure(name, new Error("test callback must be a function"));
      return;
    }
    __amberjsQueueTest(name, callback, { only: true });
  };
  concurrent.todo = function concurrentTodo(name) {
    __amberjsQueueTest(name || "todo test", undefined, { skip: true });
  };
  concurrent.failing = function concurrentFailing(name, callback) {
    if (typeof callback !== "function") {
      __amberjsRecordFailure(name, new Error("test callback must be a function"));
      return;
    }
    __amberjsQueueTest(name, callback, { failing: true });
  };
  return concurrent;
}

test.concurrent = __amberjsCreateConcurrentTest();

const it = test;
it.skip = test.skip;
it.only = test.only;
it.todo = test.todo;
it.failing = test.failing;
it.concurrent = test.concurrent;

function beforeEach(callback) {
  if (typeof callback !== "function") {
    __amberjsRecordFailure("beforeEach", new Error("beforeEach callback must be a function"));
    return;
  }
  __amberjsCurrentHookFrame().beforeEach.push(callback);
}

function afterEach(callback) {
  if (typeof callback !== "function") {
    __amberjsRecordFailure("afterEach", new Error("afterEach callback must be a function"));
    return;
  }
  __amberjsCurrentHookFrame().afterEach.push(callback);
}

function beforeAll(callback) {
  if (typeof callback !== "function") {
    __amberjsRecordFailure("beforeAll", new Error("beforeAll callback must be a function"));
    return;
  }
  __amberjsCurrentHookFrame().beforeAll.push(callback);
}

function afterAll(callback) {
  if (typeof callback !== "function") {
    __amberjsRecordFailure("afterAll", new Error("afterAll callback must be a function"));
    return;
  }
  __amberjsCurrentHookFrame().afterAll.push(callback);
}

function describe(name, callback) {
  return __amberjsDescribe(name, callback, {});
}

function __amberjsDescribe(name, callback, options) {
  if (typeof callback !== "function") {
    __amberjsRecordFailure(name, new Error("describe callback must be a function"));
    return;
  }

  try {
    __amberjsDescribeStack.push(__amberjsCreateSuiteFrameWithOptions(name, options));
    callback();
  } catch (error) {
    __amberjsRecordFailure(name, error);
  } finally {
    __amberjsDescribeStack.pop();
  }
}

describe.skip = function describeSkip(name, callback) {
  return __amberjsDescribe(name, callback, { skip: true });
};
describe.only = function describeOnly(name, callback) {
  return __amberjsDescribe(name, callback, { only: true });
};

function __amberjsEachArgs(row) {
  return Array.isArray(row) ? row : [row];
}

function __amberjsSplitEachLine(line) {
  const cells = String(line).split("|").map((cell) => cell.trim());
  if (cells.length > 0 && cells[0] === "") {
    cells.shift();
  }
  if (cells.length > 0 && cells[cells.length - 1] === "") {
    cells.pop();
  }
  return cells;
}

function __amberjsEachValueMarker(index) {
  return `__AMBER_EACH_VALUE_${index}__`;
}

function __amberjsParseEachTemplateCell(cell, values) {
  const exactMatch = String(cell).match(/^__AMBER_EACH_VALUE_(\d+)__$/);
  if (exactMatch) {
    return values[Number(exactMatch[1])];
  }
  return String(cell).replace(/__AMBER_EACH_VALUE_(\d+)__/g, (_, index) => {
    return String(values[Number(index)]);
  });
}

function __amberjsParseEachTemplateTable(strings, values) {
  let text = "";
  for (let index = 0; index < strings.length; index++) {
    text += strings[index];
    if (index < values.length) {
      text += __amberjsEachValueMarker(index);
    }
  }

  const lines = text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  if (lines.length < 2) {
    return [];
  }
  const headers = __amberjsSplitEachLine(lines[0]);
  return lines.slice(1).map((line) => {
    const cells = __amberjsSplitEachLine(line);
    const row = {};
    headers.forEach((header, index) => {
      row[header] = __amberjsParseEachTemplateCell(cells[index] || "", values);
    });
    return row;
  });
}

function __amberjsResolveEachRows(table, values) {
  if (Array.isArray(table) && Array.isArray(table.raw)) {
    return __amberjsParseEachTemplateTable(table, values);
  }
  if (Array.isArray(table)) {
    return table;
  }
  return null;
}

function __amberjsFormatEachTitle(name, args, rowIndex) {
  let argIndex = 0;
  let title = String(name);
  if (args.length === 1 && args[0] !== null && typeof args[0] === "object" && !Array.isArray(args[0])) {
    const row = args[0];
    title = title.replace(/\$#|\$[A-Za-z_$][A-Za-z0-9_$]*/g, (token) => {
      if (token === "$#") {
        return String(rowIndex);
      }
      const key = token.slice(1);
      return Object.prototype.hasOwnProperty.call(row, key) ? String(row[key]) : token;
    });
  }
  return title.replace(/%[#sdifjp]/g, (token) => {
    if (token === "%#") {
      return String(rowIndex);
    }
    const value = args[argIndex++];
    if (token === "%s") {
      return String(value);
    }
    if (token === "%i" || token === "%d") {
      return String(Number.parseInt(value, 10));
    }
    if (token === "%f") {
      return String(Number(value));
    }
    if (token === "%j") {
      try {
        return JSON.stringify(value);
      } catch (_) {
        return String(value);
      }
    }
    return __amberjsFormatValue(value);
  });
}

function __amberjsEachCallback(callback, args) {
  if (typeof callback !== "function") {
    return callback;
  }
  if (callback.length > args.length) {
    return function (done) {
      return callback.apply(undefined, args.concat(done));
    };
  }
  return function () {
    return callback.apply(undefined, args);
  };
}

function __amberjsCreateTestEach(registerTest) {
  return function each(table, ...values) {
    const rows = __amberjsResolveEachRows(table, values);
    return function eachTest(name, callback) {
      if (!rows) {
        __amberjsRecordFailure(name, new Error("each table must be an array"));
        return;
      }
      rows.forEach((row, rowIndex) => {
        const args = __amberjsEachArgs(row);
        registerTest(__amberjsFormatEachTitle(name, args, rowIndex), __amberjsEachCallback(callback, args));
      });
    };
  };
}

function __amberjsCreateDescribeEach(registerDescribe) {
  return function each(table, ...values) {
    const rows = __amberjsResolveEachRows(table, values);
    return function eachDescribe(name, callback) {
      if (!rows) {
        __amberjsRecordFailure(name, new Error("each table must be an array"));
        return;
      }
      rows.forEach((row, rowIndex) => {
        const args = __amberjsEachArgs(row);
        registerDescribe(__amberjsFormatEachTitle(name, args, rowIndex), function () {
          return callback.apply(undefined, args);
        });
      });
    };
  };
}

test.each = __amberjsCreateTestEach(test);
test.skip.each = __amberjsCreateTestEach(test.skip);
test.only.each = __amberjsCreateTestEach(test.only);
test.failing.each = __amberjsCreateTestEach(test.failing);
test.concurrent.each = __amberjsCreateTestEach(test.concurrent);
test.concurrent.skip.each = __amberjsCreateTestEach(test.concurrent.skip);
test.concurrent.only.each = __amberjsCreateTestEach(test.concurrent.only);
test.concurrent.failing.each = __amberjsCreateTestEach(test.concurrent.failing);
it.each = test.each;
it.skip.each = test.skip.each;
it.only.each = test.only.each;
it.failing.each = test.failing.each;
it.concurrent.each = test.concurrent.each;
it.concurrent.skip.each = test.concurrent.skip.each;
it.concurrent.only.each = test.concurrent.only.each;
it.concurrent.failing.each = test.concurrent.failing.each;
describe.each = __amberjsCreateDescribeEach(describe);
describe.skip.each = __amberjsCreateDescribeEach(describe.skip);
describe.only.each = __amberjsCreateDescribeEach(describe.only);

function __amberjsIsAsymmetricMatcher(value) {
  return Boolean(value && value.__amberjsAsymmetricMatcher === true && typeof value.asymmetricMatch === "function");
}

function __amberjsContainsAsymmetricMatcher(value) {
  if (__amberjsIsAsymmetricMatcher(value)) {
    return true;
  }
  if (value === null || typeof value !== "object") {
    return false;
  }
  const keys = Object.keys(value);
  for (const key of keys) {
    if (__amberjsContainsAsymmetricMatcher(value[key])) {
      return true;
    }
  }
  return false;
}

function __amberjsIsMap(value) {
  return Object.prototype.toString.call(value) === "[object Map]";
}

function __amberjsIsSet(value) {
  return Object.prototype.toString.call(value) === "[object Set]";
}

function __amberjsMapsEqual(actual, expected, valuesEqual) {
  if (!__amberjsIsMap(actual) || !__amberjsIsMap(expected) || actual.size !== expected.size) {
    return false;
  }

  const expectedEntries = Array.from(expected.entries());
  const matched = new Array(expectedEntries.length).fill(false);
  for (const [actualKey, actualValue] of actual.entries()) {
    let found = false;
    for (let index = 0; index < expectedEntries.length; index++) {
      if (matched[index]) {
        continue;
      }
      const [expectedKey, expectedValue] = expectedEntries[index];
      if (valuesEqual(actualKey, expectedKey) && valuesEqual(actualValue, expectedValue)) {
        matched[index] = true;
        found = true;
        break;
      }
    }
    if (!found) {
      return false;
    }
  }
  return true;
}

function __amberjsSetsEqual(actual, expected, valuesEqual) {
  if (!__amberjsIsSet(actual) || !__amberjsIsSet(expected) || actual.size !== expected.size) {
    return false;
  }

  const expectedValues = Array.from(expected.values());
  const matched = new Array(expectedValues.length).fill(false);
  for (const actualValue of actual.values()) {
    let found = false;
    for (let index = 0; index < expectedValues.length; index++) {
      if (matched[index]) {
        continue;
      }
      if (valuesEqual(actualValue, expectedValues[index])) {
        matched[index] = true;
        found = true;
        break;
      }
    }
    if (!found) {
      return false;
    }
  }
  return true;
}

function __amberjsValuesEqual(actual, expected) {
  if (__amberjsIsAsymmetricMatcher(expected)) {
    return expected.asymmetricMatch(actual);
  }
  if (Object.is(actual, expected)) {
    return true;
  }
  if (__amberjsIsMap(actual) || __amberjsIsMap(expected)) {
    return __amberjsMapsEqual(actual, expected, __amberjsValuesEqual);
  }
  if (__amberjsIsSet(actual) || __amberjsIsSet(expected)) {
    return __amberjsSetsEqual(actual, expected, __amberjsValuesEqual);
  }
  if (__amberjsContainsAsymmetricMatcher(expected)) {
    if (actual === null || expected === null || typeof actual !== "object" || typeof expected !== "object") {
      return false;
    }
    if (Array.isArray(actual) || Array.isArray(expected)) {
      if (!Array.isArray(actual) || !Array.isArray(expected) || actual.length !== expected.length) {
        return false;
      }
      for (let index = 0; index < expected.length; index++) {
        if (!__amberjsValuesEqual(actual[index], expected[index])) {
          return false;
        }
      }
      return true;
    }

    const actualKeys = Object.keys(actual).sort();
    const expectedKeys = Object.keys(expected).sort();
    if (actualKeys.length !== expectedKeys.length) {
      return false;
    }
    for (let index = 0; index < expectedKeys.length; index++) {
      const key = expectedKeys[index];
      if (actualKeys[index] !== key || !__amberjsValuesEqual(actual[key], expected[key])) {
        return false;
      }
    }
    return true;
  }
  return JSON.stringify(actual) === JSON.stringify(expected);
}

function __amberjsOwnKeys(value) {
  return Reflect.ownKeys(value).sort((left, right) => {
    const leftText = String(left);
    const rightText = String(right);
    if (leftText < rightText) {
      return -1;
    }
    if (leftText > rightText) {
      return 1;
    }
    return 0;
  });
}

function __amberjsStrictValuesEqual(actual, expected) {
  if (__amberjsIsAsymmetricMatcher(expected)) {
    return expected.asymmetricMatch(actual);
  }
  if (Object.is(actual, expected)) {
    return true;
  }
  if (actual === null || expected === null || typeof actual !== "object" || typeof expected !== "object") {
    return false;
  }
  if (Object.getPrototypeOf(actual) !== Object.getPrototypeOf(expected)) {
    return false;
  }
  if (__amberjsIsMap(actual) || __amberjsIsMap(expected)) {
    return __amberjsMapsEqual(actual, expected, __amberjsStrictValuesEqual);
  }
  if (__amberjsIsSet(actual) || __amberjsIsSet(expected)) {
    return __amberjsSetsEqual(actual, expected, __amberjsStrictValuesEqual);
  }
  if (Array.isArray(actual) || Array.isArray(expected)) {
    if (!Array.isArray(actual) || !Array.isArray(expected) || actual.length !== expected.length) {
      return false;
    }
    for (let index = 0; index < actual.length; index++) {
      const actualHasIndex = Object.prototype.hasOwnProperty.call(actual, index);
      const expectedHasIndex = Object.prototype.hasOwnProperty.call(expected, index);
      if (actualHasIndex !== expectedHasIndex) {
        return false;
      }
      if (actualHasIndex && !__amberjsStrictValuesEqual(actual[index], expected[index])) {
        return false;
      }
    }
  }

  const actualKeys = __amberjsOwnKeys(actual);
  const expectedKeys = __amberjsOwnKeys(expected);
  if (actualKeys.length !== expectedKeys.length) {
    return false;
  }
  for (let index = 0; index < actualKeys.length; index++) {
    if (actualKeys[index] !== expectedKeys[index]) {
      return false;
    }
    const key = actualKeys[index];
    if (!__amberjsStrictValuesEqual(actual[key], expected[key])) {
      return false;
    }
  }
  return true;
}

function __amberjsContains(actual, expected) {
  if (typeof actual === "string") {
    return actual.includes(String(expected));
  }
  if (Array.isArray(actual)) {
    return actual.some((item) => __amberjsValuesEqual(item, expected));
  }
  if (actual && typeof actual.includes === "function") {
    return actual.includes(expected);
  }
  return false;
}

function __amberjsContainsEqual(actual, expected) {
  if (!Array.isArray(actual)) {
    return false;
  }
  return actual.some((item) => __amberjsValuesEqual(item, expected));
}

function __amberjsLengthOf(actual) {
  if (actual == null || typeof actual.length !== "number") {
    throw new Error(`Expected ${__amberjsFormatValue(actual)} to have a length property`);
  }
  return actual.length;
}

function __amberjsEnsureFiniteNumber(value, label) {
  if (typeof value !== "number" || Number.isNaN(value)) {
    throw new Error(`Expected ${label} to be a number, got ${__amberjsFormatValue(value)}`);
  }
  return value;
}

function __amberjsCloseTo(actual, expected, precision) {
  const actualNumber = __amberjsEnsureFiniteNumber(actual, "actual value");
  const expectedNumber = __amberjsEnsureFiniteNumber(expected, "expected value");
  const digits = precision === undefined ? 2 : Number(precision);
  if (!Number.isInteger(digits) || digits < 0) {
    throw new Error(`Expected precision to be a non-negative integer, got ${__amberjsFormatValue(precision)}`);
  }
  return Math.abs(actualNumber - expectedNumber) < 10 ** -digits / 2;
}

function __amberjsMatches(actual, expected) {
  const text = String(actual);
  if (expected instanceof RegExp) {
    return expected.test(text);
  }
  return text.includes(String(expected));
}

function __amberjsPropertyPathSegments(path) {
  if (Array.isArray(path)) {
    return path.map((segment) => String(segment));
  }
  if (typeof path === "string") {
    const segments = [];
    let current = "";
    for (let index = 0; index < path.length; index++) {
      const character = path[index];
      if (character === ".") {
        segments.push(current);
        current = "";
        continue;
      }
      if (character === "[") {
        if (current !== "") {
          segments.push(current);
          current = "";
        }
        const closeIndex = path.indexOf("]", index + 1);
        if (closeIndex === -1) {
          throw new Error(`Invalid property path ${__amberjsFormatValue(path)}: missing closing ]`);
        }
        const bracketSegment = path.slice(index + 1, closeIndex).trim();
        if (bracketSegment === "") {
          throw new Error(`Invalid property path ${__amberjsFormatValue(path)}: empty bracket segment`);
        }
        let segment = bracketSegment;
        if ((segment[0] === '"' && segment[segment.length - 1] === '"') ||
            (segment[0] === "'" && segment[segment.length - 1] === "'")) {
          if (segment[0] === '"') {
            try {
              segment = JSON.parse(segment);
            } catch (_) {
              throw new Error(`Invalid property path ${__amberjsFormatValue(path)}: invalid quoted bracket segment`);
            }
          } else {
            segment = segment.slice(1, -1).replace(/\\'/g, "'").replace(/\\\\/g, "\\");
          }
        }
        segments.push(String(segment));
        index = path[closeIndex + 1] === "." ? closeIndex + 1 : closeIndex;
        continue;
      }
      current += character;
    }
    if (current !== "" || path.length === 0 || path[path.length - 1] === ".") {
      segments.push(current);
    }
    return segments;
  }
  throw new Error(`Expected property path to be a string or array, got ${__amberjsFormatValue(path)}`);
}

function __amberjsGetPropertyAtPath(actual, path) {
  let current = actual;
  for (const segment of __amberjsPropertyPathSegments(path)) {
    if (current === null || current === undefined) {
      return { exists: false, value: undefined };
    }
    const object = Object(current);
    if (!(segment in object)) {
      return { exists: false, value: undefined };
    }
    current = object[segment];
  }
  return { exists: true, value: current };
}

function __amberjsPartialObjectMatches(actual, expected) {
  if (__amberjsIsAsymmetricMatcher(expected)) {
    return expected.asymmetricMatch(actual);
  }
  if (expected === null || typeof expected !== "object") {
    return __amberjsValuesEqual(actual, expected);
  }
  if (Array.isArray(expected)) {
    if (!Array.isArray(actual) || actual.length < expected.length) {
      return false;
    }
    return expected.every((item, index) => __amberjsPartialObjectMatches(actual[index], item));
  }
  if (actual === null || typeof actual !== "object") {
    return false;
  }
  const object = Object(actual);
  for (const key of Object.keys(expected)) {
    if (!(key in object) || !__amberjsPartialObjectMatches(object[key], expected[key])) {
      return false;
    }
  }
  return true;
}

function __amberjsThrownMessage(error) {
  if (error && error.message !== undefined) {
    return String(error.message);
  }
  return String(error);
}

function __amberjsThrowMatches(error, expected) {
  if (expected === undefined) {
    return true;
  }
  if (typeof expected === "string") {
    return __amberjsThrownMessage(error).includes(expected);
  }
  if (expected instanceof RegExp) {
    return expected.test(__amberjsThrownMessage(error));
  }
  if (typeof expected === "function") {
    return error instanceof expected;
  }
  if (expected && expected.message !== undefined) {
    return __amberjsThrownMessage(error).includes(String(expected.message));
  }
  return false;
}

function __amberjsDescribeThrowExpected(expected) {
  if (expected === undefined) {
    return "";
  }
  if (typeof expected === "function" && expected.name) {
    return ` ${expected.name}`;
  }
  return ` matching ${__amberjsFormatValue(expected)}`;
}

function __amberjsFormatPromiseReason(reason) {
  if (reason && reason.name !== undefined && reason.message !== undefined) {
    return `${String(reason.name)}: ${String(reason.message)}`;
  }
  return __amberjsFormatValue(reason);
}

function __amberjsAssertRejectedToThrow(error, expected, negate) {
  const expectedLabel = __amberjsDescribeThrowExpected(expected);
  const pass = __amberjsThrowMatches(error, expected);
  if (negate ? pass : !pass) {
    throw new Error(
      negate
        ? `Expected promise rejection not to throw an error${expectedLabel}`
        : `Expected promise rejection to throw an error${expectedLabel}`
    );
  }
}

function __amberjsCreateMockFunction(implementation) {
  const onceImplementations = [];
  let defaultImplementation = typeof implementation === "function" ? implementation : undefined;
  let mockName = "jest.fn()";

  function mockFn(...args) {
    mockFn.mock.calls.push(args);
    mockFn.mock.contexts.push(this);
    if (this !== undefined && this !== null && this !== globalThis) {
      mockFn.mock.instances.push(this);
    }

    const nextImplementation = onceImplementations.length > 0
      ? onceImplementations.shift()
      : defaultImplementation;

    try {
      const value = typeof nextImplementation === "function"
        ? nextImplementation.apply(this, args)
        : undefined;
      const result = {};
      result["type"] = "return";
      result.value = value;
      mockFn.mock.results.push(result);
      return value;
    } catch (error) {
      const result = {};
      result["type"] = "throw";
      result.value = error;
      mockFn.mock.results.push(result);
      throw error;
    }
  }

  mockFn._isMockFunction = true;
  mockFn.mock = {
    calls: [],
    results: [],
    instances: [],
    contexts: []
  };
  Object.defineProperty(mockFn.mock, "lastCall", {
    configurable: true,
    enumerable: true,
    get() {
      return mockFn.mock.calls.length === 0
        ? undefined
        : mockFn.mock.calls[mockFn.mock.calls.length - 1];
    }
  });
  mockFn.getMockName = function () {
    return mockName;
  };
  mockFn.mockName = function (name) {
    mockName = String(name);
    return mockFn;
  };
  mockFn.mockClear = function () {
    mockFn.mock.calls.length = 0;
    mockFn.mock.results.length = 0;
    mockFn.mock.instances.length = 0;
    mockFn.mock.contexts.length = 0;
    return mockFn;
  };
  mockFn.mockReset = function () {
    mockFn.mockClear();
    onceImplementations.length = 0;
    defaultImplementation = undefined;
    return mockFn;
  };
  mockFn.mockImplementation = function (callback) {
    if (typeof callback !== "function") {
      throw new Error("mockImplementation callback must be a function");
    }
    defaultImplementation = callback;
    return mockFn;
  };
  mockFn.getMockImplementation = function () {
    return defaultImplementation;
  };
  mockFn.mockImplementationOnce = function (callback) {
    if (typeof callback !== "function") {
      throw new Error("mockImplementationOnce callback must be a function");
    }
    onceImplementations.push(callback);
    return mockFn;
  };
  mockFn.withImplementation = function (temporaryImplementation, callback) {
    if (typeof temporaryImplementation !== "function") {
      throw new Error("withImplementation temporary implementation must be a function");
    }
    if (typeof callback !== "function") {
      throw new Error("withImplementation callback must be a function");
    }
    const previousImplementation = defaultImplementation;
    defaultImplementation = temporaryImplementation;
    try {
      const result = callback();
      if (result && typeof result.then === "function") {
        return Promise.resolve(result).then(
          (value) => {
            defaultImplementation = previousImplementation;
            return value;
          },
          (error) => {
            defaultImplementation = previousImplementation;
            throw error;
          }
        );
      }
      defaultImplementation = previousImplementation;
      return result;
    } catch (error) {
      defaultImplementation = previousImplementation;
      throw error;
    }
  };
  mockFn.mockReturnValue = function (value) {
    defaultImplementation = function () {
      return value;
    };
    return mockFn;
  };
  mockFn.mockReturnThis = function () {
    defaultImplementation = function () {
      return this;
    };
    return mockFn;
  };
  mockFn.mockReturnValueOnce = function (value) {
    onceImplementations.push(function () {
      return value;
    });
    return mockFn;
  };
  mockFn.mockResolvedValue = function (value) {
    defaultImplementation = function () {
      return Promise.resolve(value);
    };
    return mockFn;
  };
  mockFn.mockResolvedValueOnce = function (value) {
    onceImplementations.push(function () {
      return Promise.resolve(value);
    });
    return mockFn;
  };
  mockFn.mockRejectedValue = function (value) {
    defaultImplementation = function () {
      return Promise.reject(value);
    };
    return mockFn;
  };
  mockFn.mockRejectedValueOnce = function (value) {
    onceImplementations.push(function () {
      return Promise.reject(value);
    });
    return mockFn;
  };

  __amberjsMockFunctions.push(mockFn);
  return mockFn;
}

function __amberjsClearAllMocks() {
  for (const mockFn of __amberjsMockFunctions) {
    mockFn.mockClear();
  }
}

function __amberjsResetAllMocks() {
  for (const mockFn of __amberjsMockFunctions) {
    mockFn.mockReset();
  }
}

function __amberjsSpyOn(target, propertyName, accessType) {
  if (target === null || (typeof target !== "object" && typeof target !== "function")) {
    throw new Error("jest.spyOn target must be an object");
  }
  if (accessType !== undefined && accessType !== "get" && accessType !== "set") {
    throw new Error(`jest.spyOn accessType must be "get" or "set", got ${__amberjsFormatValue(accessType)}`);
  }
  const propertyKey = String(propertyName);
  const hadOwnProperty = Object.prototype.hasOwnProperty.call(target, propertyKey);
  const originalOwnDescriptor = Object.getOwnPropertyDescriptor(target, propertyKey);
  let descriptorOwner = target;
  let descriptor = Object.getOwnPropertyDescriptor(descriptorOwner, propertyKey);
  while (!descriptor && descriptorOwner !== null) {
    descriptorOwner = Object.getPrototypeOf(descriptorOwner);
    descriptor = descriptorOwner
      ? Object.getOwnPropertyDescriptor(descriptorOwner, propertyKey)
      : undefined;
  }
  if (!descriptor) {
    throw new Error(`Property ${propertyKey} does not exist`);
  }

  if (accessType !== undefined) {
    const accessor = accessType === "get" ? descriptor.get : descriptor.set;
    if (typeof accessor !== "function") {
      throw new Error(`Property ${propertyKey} does not have a ${accessType === "get" ? "getter" : "setter"}`);
    }
    const targetDescriptor = Object.getOwnPropertyDescriptor(target, propertyKey);
    if (targetDescriptor && targetDescriptor.configurable === false) {
      throw new Error(`Property ${propertyKey} is not configurable`);
    }

    const spy = __amberjsCreateMockFunction(function (...args) {
      return accessor.apply(this, args);
    });
    let restored = false;
    function restore() {
      if (restored) {
        return;
      }
      if (hadOwnProperty) {
        Object.defineProperty(target, propertyKey, originalOwnDescriptor);
      } else {
        delete target[propertyKey];
      }
      restored = true;
    }
    spy.mockRestore = function () {
      restore();
      spy.mockReset();
      return spy;
    };
    Object.defineProperty(target, propertyKey, {
      configurable: true,
      enumerable: descriptor.enumerable,
      get: accessType === "get" ? spy : descriptor.get,
      set: accessType === "set" ? spy : descriptor.set
    });
    __amberjsSpyRestorers.push(restore);
    return spy;
  }

  if (descriptor.get || descriptor.set) {
    throw new Error("jest.spyOn accessors require an accessType of \"get\" or \"set\"");
  }
  const original = descriptor.value;
  if (typeof original !== "function") {
    throw new Error(`Property ${propertyKey} is not a function`);
  }
  const targetDescriptor = Object.getOwnPropertyDescriptor(target, propertyKey);
  if (targetDescriptor && targetDescriptor.configurable === false && targetDescriptor.writable === false) {
    throw new Error(`Property ${propertyKey} is not configurable`);
  }

  const spy = __amberjsCreateMockFunction(function (...args) {
    return original.apply(this, args);
  });
  let restored = false;
  function restore() {
    if (restored) {
      return;
    }
    if (hadOwnProperty) {
      Object.defineProperty(target, propertyKey, originalOwnDescriptor);
    } else {
      delete target[propertyKey];
    }
    restored = true;
  }
  spy.mockRestore = function () {
    restore();
    spy.mockReset();
    return spy;
  };
  Object.defineProperty(target, propertyKey, {
    configurable: true,
    enumerable: descriptor.enumerable,
    writable: true,
    value: spy
  });
  __amberjsSpyRestorers.push(restore);
  return spy;
}

function __amberjsReplaceProperty(target, propertyName, value) {
  if (target === null || (typeof target !== "object" && typeof target !== "function")) {
    throw new Error("jest.replaceProperty target must be an object");
  }
  const propertyKey = String(propertyName);
  if (!(propertyKey in Object(target))) {
    throw new Error(`Property ${propertyKey} does not exist`);
  }

  const hadOwnProperty = Object.prototype.hasOwnProperty.call(target, propertyKey);
  const originalOwnDescriptor = Object.getOwnPropertyDescriptor(target, propertyKey);
  let descriptorOwner = target;
  let descriptor = Object.getOwnPropertyDescriptor(descriptorOwner, propertyKey);
  while (!descriptor && descriptorOwner !== null) {
    descriptorOwner = Object.getPrototypeOf(descriptorOwner);
    descriptor = descriptorOwner
      ? Object.getOwnPropertyDescriptor(descriptorOwner, propertyKey)
      : undefined;
  }
  if (!descriptor) {
    throw new Error(`Property ${propertyKey} does not exist`);
  }
  if (descriptor.get || descriptor.set) {
    throw new Error(`Property ${propertyKey} has accessors; use jest.spyOn with accessType instead`);
  }
  if (descriptor.writable === false && descriptor.configurable === false) {
    throw new Error(`Property ${propertyKey} is not configurable`);
  }

  let restored = false;
  function restore() {
    if (restored) {
      return;
    }
    if (hadOwnProperty) {
      Object.defineProperty(target, propertyKey, originalOwnDescriptor);
    } else {
      delete target[propertyKey];
    }
    restored = true;
  }

  const replacedProperty = {
    replaceValue(nextValue) {
      Object.defineProperty(target, propertyKey, {
        configurable: true,
        enumerable: descriptor.enumerable,
        writable: true,
        value: nextValue
      });
      restored = false;
      return replacedProperty;
    },
    restore() {
      restore();
      return replacedProperty;
    }
  };

  replacedProperty.replaceValue(value);
  __amberjsSpyRestorers.push(restore);
  return replacedProperty;
}

function __amberjsRestoreAllMocks() {
  const restorers = __amberjsSpyRestorers.slice().reverse();
  __amberjsSpyRestorers.length = 0;
  for (const restore of restorers) {
    restore();
  }
}

const __amberjsMockModuleFactories = Object.create(null);
let __amberjsMockModuleExports = Object.create(null);
let __amberjsBypassModuleMocks = false;

function __amberjsResolveModuleSpecifier(specifier) {
  if (typeof globalThis.require !== "function" || typeof globalThis.require.resolve !== "function") {
    throw new Error("require.resolve is not available");
  }
  return String(globalThis.require.resolve(String(specifier)));
}

function __amberjsResetMockModuleInstances() {
  __amberjsMockModuleExports = Object.create(null);
}

function __amberjsMaterializeMockModule(resolvedSpecifier) {
  if (!Object.prototype.hasOwnProperty.call(__amberjsMockModuleFactories, resolvedSpecifier)) {
    throw new Error(`No mock factory registered for ${resolvedSpecifier}`);
  }
  if (!Object.prototype.hasOwnProperty.call(__amberjsMockModuleExports, resolvedSpecifier)) {
    __amberjsMockModuleExports[resolvedSpecifier] = __amberjsMockModuleFactories[resolvedSpecifier]();
  }
  return __amberjsMockModuleExports[resolvedSpecifier];
}

function __amberjsInstallMockAwareRequire() {
  const originalRequire = globalThis.require;
  if (typeof originalRequire !== "function" || originalRequire.__amberjsMockAware === true) {
    return;
  }

  function requireWithMocks(specifier) {
    const resolvedSpecifier = __amberjsResolveModuleSpecifier(specifier);
    if (!__amberjsBypassModuleMocks && Object.prototype.hasOwnProperty.call(__amberjsMockModuleFactories, resolvedSpecifier)) {
      return __amberjsMaterializeMockModule(resolvedSpecifier);
    }
    return originalRequire(specifier);
  }

  requireWithMocks.resolve = function (specifier) {
    if (typeof originalRequire.resolve !== "function") {
      throw new Error("require.resolve is not available");
    }
    return originalRequire.resolve(specifier);
  };
  requireWithMocks.main = originalRequire.main;
  Object.defineProperty(requireWithMocks, "__amberjsMockAware", {
    configurable: false,
    enumerable: false,
    value: true
  });
  globalThis.require = requireWithMocks;
}

function __amberjsRequireActual(specifier) {
  const previousBypass = __amberjsBypassModuleMocks;
  __amberjsBypassModuleMocks = true;
  try {
    return globalThis.require(specifier);
  } finally {
    __amberjsBypassModuleMocks = previousBypass;
  }
}

function __amberjsCaptureModuleCaches() {
  return {
    hasModuleCache: Object.prototype.hasOwnProperty.call(globalThis, "__amberjsModuleCache"),
    moduleCache: globalThis.__amberjsModuleCache,
    hasEsmNamespaceCache: Object.prototype.hasOwnProperty.call(globalThis, "__amberjsEsmNamespaceCache"),
    esmNamespaceCache: globalThis.__amberjsEsmNamespaceCache,
    hasEsmNamespaceFingerprintCache: Object.prototype.hasOwnProperty.call(globalThis, "__amberjsEsmNamespaceFingerprintCache"),
    esmNamespaceFingerprintCache: globalThis.__amberjsEsmNamespaceFingerprintCache,
    mockModuleExports: __amberjsMockModuleExports
  };
}

function __amberjsRestoreModuleCaches(caches) {
  if (caches.hasModuleCache) {
    globalThis.__amberjsModuleCache = caches.moduleCache;
  } else {
    delete globalThis.__amberjsModuleCache;
  }

  if (caches.hasEsmNamespaceCache) {
    globalThis.__amberjsEsmNamespaceCache = caches.esmNamespaceCache;
  } else {
    delete globalThis.__amberjsEsmNamespaceCache;
  }

  if (caches.hasEsmNamespaceFingerprintCache) {
    globalThis.__amberjsEsmNamespaceFingerprintCache = caches.esmNamespaceFingerprintCache;
  } else {
    delete globalThis.__amberjsEsmNamespaceFingerprintCache;
  }
  __amberjsMockModuleExports = caches.mockModuleExports;
}

function __amberjsUseFreshModuleCaches() {
  globalThis.__amberjsModuleCache = Object.create(null);
  globalThis.__amberjsEsmNamespaceCache = Object.create(null);
  globalThis.__amberjsEsmNamespaceFingerprintCache = Object.create(null);
  __amberjsResetMockModuleInstances();
}

function __amberjsResetModuleCaches() {
  __amberjsUseFreshModuleCaches();
}

const jest = {};
jest.fn = __amberjsCreateMockFunction;
jest.spyOn = __amberjsSpyOn;
jest.replaceProperty = __amberjsReplaceProperty;
jest.isMockFunction = function (value) {
  return typeof value === "function" && value._isMockFunction === true && !!value.mock;
};
jest.clearAllMocks = __amberjsClearAllMocks;
jest.resetAllMocks = __amberjsResetAllMocks;
jest.restoreAllMocks = __amberjsRestoreAllMocks;
jest.resetModules = function () {
  __amberjsResetModuleCaches();
  return jest;
};
jest.doMock = function (specifier, factory) {
  if (typeof factory !== "function") {
    throw new Error("jest.doMock() expects a module factory function");
  }
  const resolvedSpecifier = __amberjsResolveModuleSpecifier(specifier);
  __amberjsMockModuleFactories[resolvedSpecifier] = factory;
  delete __amberjsMockModuleExports[resolvedSpecifier];
  return jest;
};
jest.mock = jest.doMock;
jest.setMock = function (specifier, moduleExports) {
  const resolvedSpecifier = __amberjsResolveModuleSpecifier(specifier);
  __amberjsMockModuleFactories[resolvedSpecifier] = function () {
    return moduleExports;
  };
  __amberjsMockModuleExports[resolvedSpecifier] = moduleExports;
  return jest;
};
jest.requireMock = function (specifier) {
  const resolvedSpecifier = __amberjsResolveModuleSpecifier(specifier);
  return __amberjsMaterializeMockModule(resolvedSpecifier);
};
jest.unmock = function (specifier) {
  const resolvedSpecifier = __amberjsResolveModuleSpecifier(specifier);
  delete __amberjsMockModuleFactories[resolvedSpecifier];
  delete __amberjsMockModuleExports[resolvedSpecifier];
  return jest;
};
jest.dontMock = jest.unmock;
jest.requireActual = function (specifier) {
  return __amberjsRequireActual(specifier);
};
jest.isolateModules = function (callback) {
  if (typeof callback !== "function") {
    throw new Error(`jest.isolateModules() expects a callback function, got ${__amberjsFormatValue(callback)}`);
  }

  const previousCaches = __amberjsCaptureModuleCaches();
  __amberjsUseFreshModuleCaches();
  try {
    callback();
  } finally {
    __amberjsRestoreModuleCaches(previousCaches);
  }
  return jest;
};
jest.isolateModulesAsync = function (callback) {
  if (typeof callback !== "function") {
    throw new Error(`jest.isolateModulesAsync() expects a callback function, got ${__amberjsFormatValue(callback)}`);
  }

  const previousCaches = __amberjsCaptureModuleCaches();
  __amberjsUseFreshModuleCaches();
  let callbackResult;
  try {
    callbackResult = callback();
  } catch (error) {
    __amberjsRestoreModuleCaches(previousCaches);
    return Promise.reject(error);
  }

  return Promise.resolve(callbackResult).then(
    function () {
      __amberjsRestoreModuleCaches(previousCaches);
      return jest;
    },
    function (error) {
      __amberjsRestoreModuleCaches(previousCaches);
      throw error;
    }
  );
};
jest.setTimeout = function (timeoutMs) {
  const milliseconds = Number(timeoutMs);
  if (!Number.isFinite(milliseconds) || milliseconds < 0) {
    throw new Error(`jest.setTimeout() expects a non-negative timeout in milliseconds, got ${__amberjsFormatValue(timeoutMs)}`);
  }
  __amberjsTestConfig.timeoutSeconds = milliseconds / 1000;
};
__amberjsInstallMockAwareRequire();
globalThis.jest = jest;

function __amberjsEnsureMockFunction(value) {
  if (!value || value._isMockFunction !== true || !value.mock) {
    throw new Error("Expected value to be a mock function");
  }
  return value;
}

function __amberjsMockCalledWith(mockFn, expectedArgs) {
  return mockFn.mock.calls.some((call) => __amberjsValuesEqual(call, expectedArgs));
}

function __amberjsEnsurePositiveInteger(value, label) {
  const number = Number(value);
  if (!Number.isInteger(number) || number < 1) {
    throw new Error(`Expected ${label} to be a positive integer, got ${__amberjsFormatValue(value)}`);
  }
  return number;
}

function __amberjsMockNthCalledWith(mockFn, nthCall, expectedArgs) {
  const index = nthCall - 1;
  if (index < 0 || index >= mockFn.mock.calls.length) {
    return false;
  }
  return __amberjsValuesEqual(mockFn.mock.calls[index], expectedArgs);
}

function __amberjsMockLastCalledWith(mockFn, expectedArgs) {
  if (mockFn.mock.calls.length === 0) {
    return false;
  }
  return __amberjsValuesEqual(mockFn.mock.calls[mockFn.mock.calls.length - 1], expectedArgs);
}

function __amberjsMockReturnCount(mockFn) {
  return mockFn.mock.results.filter((result) => result && result["type"] === "return").length;
}

function __amberjsMockReturnedWith(mockFn, expectedValue) {
  return mockFn.mock.results.some((result) => {
    return result && result["type"] === "return" && __amberjsValuesEqual(result.value, expectedValue);
  });
}

function __amberjsMockNthReturnedWith(mockFn, nthCall, expectedValue) {
  const index = nthCall - 1;
  if (index < 0 || index >= mockFn.mock.results.length) {
    return false;
  }
  const result = mockFn.mock.results[index];
  return result && result["type"] === "return" && __amberjsValuesEqual(result.value, expectedValue);
}

function __amberjsMockLastReturnedWith(mockFn, expectedValue) {
  if (mockFn.mock.results.length === 0) {
    return false;
  }
  const result = mockFn.mock.results[mockFn.mock.results.length - 1];
  return result && result["type"] === "return" && __amberjsValuesEqual(result.value, expectedValue);
}

function __amberjsSnapshotTestName(testCase) {
  return testCase.suite ? `${testCase.suite} ${testCase.name}` : testCase.name;
}

function __amberjsSnapshotIndent(level) {
  let text = "";
  for (let i = 0; i < level; i++) {
    text += "  ";
  }
  return text;
}

function __amberjsSerializeSnapshotValue(value, level) {
  const depth = Number(level) || 0;
  if (value === null) {
    return "null";
  }
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) {
    if (value.length === 0) {
      return "[]";
    }
    const items = value.map((item) => {
      return `${__amberjsSnapshotIndent(depth + 1)}${__amberjsSerializeSnapshotValue(item, depth + 1)}`;
    });
    return `[\n${items.join(",\n")}\n${__amberjsSnapshotIndent(depth)}]`;
  }
  if (value && typeof value === "object") {
    const keys = Object.keys(value);
    if (keys.length === 0) {
      return "{}";
    }
    const entries = keys.map((key) => {
      return `${__amberjsSnapshotIndent(depth + 1)}${JSON.stringify(key)}: ${__amberjsSerializeSnapshotValue(value[key], depth + 1)}`;
    });
    return `{\n${entries.join(",\n")}\n${__amberjsSnapshotIndent(depth)}}`;
  }
  return String(value);
}

function __amberjsNextSnapshotKey(hint) {
  const baseName = hint === undefined
    ? __amberjsCurrentTestName
    : `${__amberjsCurrentTestName}: ${String(hint)}`;
  const nextIndex = (__amberjsSnapshotCounters[baseName] || 0) + 1;
  __amberjsSnapshotCounters[baseName] = nextIndex;
  return `${baseName} ${nextIndex}`;
}

function __amberjsResetAssertionState() {
  __amberjsAssertionCount = 0;
  __amberjsExpectedAssertionCount = undefined;
  __amberjsHasAssertionExpectation = false;
}

function __amberjsRecordAssertion() {
  __amberjsAssertionCount++;
}

function __amberjsSetExpectedAssertionCount(expectedCount) {
  const count = Number(expectedCount);
  if (!Number.isInteger(count) || count < 0) {
    throw new Error(`expect.assertions() expects a non-negative integer, got ${__amberjsFormatValue(expectedCount)}`);
  }
  __amberjsExpectedAssertionCount = count;
}

function __amberjsVerifyAssertionState() {
  if (__amberjsExpectedAssertionCount !== undefined && __amberjsAssertionCount !== __amberjsExpectedAssertionCount) {
    throw new Error(`Expected ${__amberjsExpectedAssertionCount} assertions, but ${__amberjsAssertionCount} were run`);
  }
  if (__amberjsHasAssertionExpectation && __amberjsAssertionCount === 0) {
    throw new Error("Expected at least one assertion to be called, but none were called");
  }
}

function __amberjsCountMatcherCalls(matchers) {
  for (const matcherName of Object.keys(matchers)) {
    const matcher = matchers[matcherName];
    if (typeof matcher !== "function") {
      continue;
    }
    matchers[matcherName] = function (...args) {
      __amberjsRecordAssertion();
      return matcher.apply(this, args);
    };
  }
  return matchers;
}

function __amberjsCustomMatcherMessage(result, matcherName, matcherContext) {
  if (result && typeof result.message === "function") {
    return String(result.message.call(matcherContext));
  }
  if (result && result.message !== undefined) {
    return String(result.message);
  }
  return `Custom matcher ${matcherName} failed`;
}

function __amberjsBuildCustomMatcher(actual, negate, matcherName, matcher) {
  return function (...expectedArgs) {
    const matcherContext = {
      isNot: negate,
      promise: "",
      equals: __amberjsValuesEqual
    };
    const result = matcher.call(matcherContext, actual, ...expectedArgs);
    if (!result || typeof result.pass !== "boolean") {
      throw new Error(`Custom matcher ${matcherName} must return an object with a boolean pass field`);
    }
    if (negate ? result.pass : !result.pass) {
      throw new Error(__amberjsCustomMatcherMessage(result, matcherName, matcherContext));
    }
  };
}

function __amberjsAddCustomMatchers(matchers, actual, negate) {
  for (const matcherName of Object.keys(__amberjsCustomMatchers)) {
    matchers[matcherName] = __amberjsBuildCustomMatcher(
      actual,
      negate,
      matcherName,
      __amberjsCustomMatchers[matcherName]
    );
  }
  return matchers;
}

function __amberjsBuildMatchers(actual, negate) {
  function assertMatcher(pass, positiveMessage, negativeMessage) {
    if (negate ? pass : !pass) {
      const message = negate ? negativeMessage : positiveMessage;
      throw new Error(typeof message === "function" ? message() : message);
    }
  }

  const matchers = {
    toBe(expected) {
      assertMatcher(
        Object.is(actual, expected),
        () => `Expected ${__amberjsFormatValue(actual)} to be ${__amberjsFormatValue(expected)}`,
        () => `Expected ${__amberjsFormatValue(actual)} not to be ${__amberjsFormatValue(expected)}`
      );
    },
    toEqual(expected) {
      assertMatcher(
        __amberjsValuesEqual(actual, expected),
        () => `Expected ${__amberjsFormatValue(actual)} to equal ${__amberjsFormatValue(expected)}`,
        () => `Expected ${__amberjsFormatValue(actual)} not to equal ${__amberjsFormatValue(expected)}`
      );
    },
    toStrictEqual(expected) {
      assertMatcher(
        __amberjsStrictValuesEqual(actual, expected),
        () => `Expected ${__amberjsFormatValue(actual)} to strictly equal ${__amberjsFormatValue(expected)}`,
        () => `Expected ${__amberjsFormatValue(actual)} not to strictly equal ${__amberjsFormatValue(expected)}`
      );
    },
    toBeTruthy() {
      assertMatcher(
        Boolean(actual),
        `Expected ${__amberjsFormatValue(actual)} to be truthy`,
        `Expected ${__amberjsFormatValue(actual)} not to be truthy`
      );
    },
    toBeFalsy() {
      assertMatcher(
        !actual,
        `Expected ${__amberjsFormatValue(actual)} to be falsy`,
        `Expected ${__amberjsFormatValue(actual)} not to be falsy`
      );
    },
    toBeDefined() {
      assertMatcher(
        actual !== undefined,
        "Expected value to be defined",
        `Expected ${__amberjsFormatValue(actual)} not to be defined`
      );
    },
    toBeUndefined() {
      assertMatcher(
        actual === undefined,
        `Expected ${__amberjsFormatValue(actual)} to be undefined`,
        "Expected value not to be undefined"
      );
    },
    toBeNull() {
      assertMatcher(
        actual === null,
        `Expected ${__amberjsFormatValue(actual)} to be null`,
        "Expected value not to be null"
      );
    },
    toBeNaN() {
      assertMatcher(
        Number.isNaN(actual),
        `Expected ${__amberjsFormatValue(actual)} to be NaN`,
        `Expected ${__amberjsFormatValue(actual)} not to be NaN`
      );
    },
    toContain(expected) {
      assertMatcher(
        __amberjsContains(actual, expected),
        `Expected ${__amberjsFormatValue(actual)} to contain ${__amberjsFormatValue(expected)}`,
        `Expected ${__amberjsFormatValue(actual)} not to contain ${__amberjsFormatValue(expected)}`
      );
    },
    toContainEqual(expected) {
      assertMatcher(
        __amberjsContainsEqual(actual, expected),
        `Expected ${__amberjsFormatValue(actual)} to contain equal ${__amberjsFormatValue(expected)}`,
        `Expected ${__amberjsFormatValue(actual)} not to contain equal ${__amberjsFormatValue(expected)}`
      );
    },
    toHaveLength(expected) {
      const actualLength = __amberjsLengthOf(actual);
      assertMatcher(
        Object.is(actualLength, expected),
        `Expected ${__amberjsFormatValue(actual)} to have length ${__amberjsFormatValue(expected)}, got ${actualLength}`,
        `Expected ${__amberjsFormatValue(actual)} not to have length ${__amberjsFormatValue(expected)}`
      );
    },
    toMatch(expected) {
      assertMatcher(
        __amberjsMatches(actual, expected),
        `Expected ${__amberjsFormatValue(actual)} to match ${__amberjsFormatValue(expected)}`,
        `Expected ${__amberjsFormatValue(actual)} not to match ${__amberjsFormatValue(expected)}`
      );
    },
    toHaveProperty(path, expected) {
      const hasExpectedValue = arguments.length > 1;
      const result = __amberjsGetPropertyAtPath(actual, path);
      const pass = result.exists && (!hasExpectedValue || __amberjsValuesEqual(result.value, expected));
      const expectedSuffix = hasExpectedValue
        ? ` with value ${__amberjsFormatValue(expected)}`
        : "";
      assertMatcher(
        pass,
        `Expected ${__amberjsFormatValue(actual)} to have property ${__amberjsFormatValue(path)}${expectedSuffix}`,
        `Expected ${__amberjsFormatValue(actual)} not to have property ${__amberjsFormatValue(path)}${expectedSuffix}`
      );
    },
    toBeInstanceOf(expectedConstructor) {
      if (typeof expectedConstructor !== "function") {
        throw new Error("Expected constructor to be a function");
      }
      const constructorName = expectedConstructor.name || "provided constructor";
      assertMatcher(
        actual instanceof expectedConstructor,
        `Expected ${__amberjsFormatValue(actual)} to be instance of ${constructorName}`,
        `Expected ${__amberjsFormatValue(actual)} not to be instance of ${constructorName}`
      );
    },
    toMatchObject(expected) {
      assertMatcher(
        __amberjsPartialObjectMatches(actual, expected),
        `Expected ${__amberjsFormatValue(actual)} to match object ${__amberjsFormatValue(expected)}`,
        `Expected ${__amberjsFormatValue(actual)} not to match object ${__amberjsFormatValue(expected)}`
      );
    },
    toBeGreaterThan(expected) {
      const actualNumber = __amberjsEnsureFiniteNumber(actual, "actual value");
      const expectedNumber = __amberjsEnsureFiniteNumber(expected, "expected value");
      assertMatcher(
        actualNumber > expectedNumber,
        `Expected ${__amberjsFormatValue(actual)} to be greater than ${__amberjsFormatValue(expected)}`,
        `Expected ${__amberjsFormatValue(actual)} not to be greater than ${__amberjsFormatValue(expected)}`
      );
    },
    toBeLessThan(expected) {
      const actualNumber = __amberjsEnsureFiniteNumber(actual, "actual value");
      const expectedNumber = __amberjsEnsureFiniteNumber(expected, "expected value");
      assertMatcher(
        actualNumber < expectedNumber,
        `Expected ${__amberjsFormatValue(actual)} to be less than ${__amberjsFormatValue(expected)}`,
        `Expected ${__amberjsFormatValue(actual)} not to be less than ${__amberjsFormatValue(expected)}`
      );
    },
    toBeGreaterThanOrEqual(expected) {
      const actualNumber = __amberjsEnsureFiniteNumber(actual, "actual value");
      const expectedNumber = __amberjsEnsureFiniteNumber(expected, "expected value");
      assertMatcher(
        actualNumber >= expectedNumber,
        `Expected ${__amberjsFormatValue(actual)} to be greater than or equal to ${__amberjsFormatValue(expected)}`,
        `Expected ${__amberjsFormatValue(actual)} not to be greater than or equal to ${__amberjsFormatValue(expected)}`
      );
    },
    toBeLessThanOrEqual(expected) {
      const actualNumber = __amberjsEnsureFiniteNumber(actual, "actual value");
      const expectedNumber = __amberjsEnsureFiniteNumber(expected, "expected value");
      assertMatcher(
        actualNumber <= expectedNumber,
        `Expected ${__amberjsFormatValue(actual)} to be less than or equal to ${__amberjsFormatValue(expected)}`,
        `Expected ${__amberjsFormatValue(actual)} not to be less than or equal to ${__amberjsFormatValue(expected)}`
      );
    },
    toBeCloseTo(expected, precision) {
      assertMatcher(
        __amberjsCloseTo(actual, expected, precision),
        `Expected ${__amberjsFormatValue(actual)} to be close to ${__amberjsFormatValue(expected)}`,
        `Expected ${__amberjsFormatValue(actual)} not to be close to ${__amberjsFormatValue(expected)}`
      );
    },
    toHaveBeenCalled() {
      const mockFn = __amberjsEnsureMockFunction(actual);
      assertMatcher(
        mockFn.mock.calls.length > 0,
        "Expected mock to have been called",
        "Expected mock not to have been called"
      );
    },
    toHaveBeenCalledTimes(expected) {
      const mockFn = __amberjsEnsureMockFunction(actual);
      assertMatcher(
        Object.is(mockFn.mock.calls.length, expected),
        `Expected mock to have been called ${__amberjsFormatValue(expected)} times, got ${mockFn.mock.calls.length}`,
        `Expected mock not to have been called ${__amberjsFormatValue(expected)} times`
      );
    },
    toHaveBeenCalledWith(...expectedArgs) {
      const mockFn = __amberjsEnsureMockFunction(actual);
      assertMatcher(
        __amberjsMockCalledWith(mockFn, expectedArgs),
        `Expected mock to have been called with ${__amberjsFormatValue(expectedArgs)}`,
        `Expected mock not to have been called with ${__amberjsFormatValue(expectedArgs)}`
      );
    },
    toHaveBeenNthCalledWith(nthCall, ...expectedArgs) {
      const mockFn = __amberjsEnsureMockFunction(actual);
      const callNumber = __amberjsEnsurePositiveInteger(nthCall, "nth call");
      assertMatcher(
        __amberjsMockNthCalledWith(mockFn, callNumber, expectedArgs),
        `Expected mock nth call ${callNumber} to have been called with ${__amberjsFormatValue(expectedArgs)}`,
        `Expected mock nth call ${callNumber} not to have been called with ${__amberjsFormatValue(expectedArgs)}`
      );
    },
    toHaveBeenLastCalledWith(...expectedArgs) {
      const mockFn = __amberjsEnsureMockFunction(actual);
      assertMatcher(
        __amberjsMockLastCalledWith(mockFn, expectedArgs),
        `Expected mock last call to have been called with ${__amberjsFormatValue(expectedArgs)}`,
        `Expected mock last call not to have been called with ${__amberjsFormatValue(expectedArgs)}`
      );
    },
    toHaveReturned() {
      const mockFn = __amberjsEnsureMockFunction(actual);
      assertMatcher(
        __amberjsMockReturnCount(mockFn) > 0,
        "Expected mock to have returned",
        "Expected mock not to have returned"
      );
    },
    toHaveReturnedTimes(expected) {
      const mockFn = __amberjsEnsureMockFunction(actual);
      const returnCount = __amberjsMockReturnCount(mockFn);
      assertMatcher(
        Object.is(returnCount, expected),
        `Expected mock to have returned ${__amberjsFormatValue(expected)} times, got ${returnCount}`,
        `Expected mock not to have returned ${__amberjsFormatValue(expected)} times`
      );
    },
    toHaveReturnedWith(expectedValue) {
      const mockFn = __amberjsEnsureMockFunction(actual);
      assertMatcher(
        __amberjsMockReturnedWith(mockFn, expectedValue),
        `Expected mock to have returned with ${__amberjsFormatValue(expectedValue)}`,
        `Expected mock not to have returned with ${__amberjsFormatValue(expectedValue)}`
      );
    },
    toHaveLastReturnedWith(expectedValue) {
      const mockFn = __amberjsEnsureMockFunction(actual);
      assertMatcher(
        __amberjsMockLastReturnedWith(mockFn, expectedValue),
        `Expected mock last return to have returned with ${__amberjsFormatValue(expectedValue)}`,
        `Expected mock last return not to have returned with ${__amberjsFormatValue(expectedValue)}`
      );
    },
    toHaveNthReturnedWith(nthCall, expectedValue) {
      const mockFn = __amberjsEnsureMockFunction(actual);
      const callNumber = __amberjsEnsurePositiveInteger(nthCall, "nth return");
      assertMatcher(
        __amberjsMockNthReturnedWith(mockFn, callNumber, expectedValue),
        `Expected mock nth return ${callNumber} to have returned with ${__amberjsFormatValue(expectedValue)}`,
        `Expected mock nth return ${callNumber} not to have returned with ${__amberjsFormatValue(expectedValue)}`
      );
    },
    toThrow(expected) {
      if (typeof actual !== "function") {
        throw new Error("Expected value to be a function");
      }
      let didThrow = false;
      let thrownError;
      try {
        actual();
      } catch (error) {
        didThrow = true;
        thrownError = error;
      }
      const expectedLabel = __amberjsDescribeThrowExpected(expected);
      const pass = didThrow && __amberjsThrowMatches(thrownError, expected);
      assertMatcher(
        pass,
        `Expected function to throw an error${expectedLabel}`,
        `Expected function not to throw an error${expectedLabel}`
      );
    },
    toMatchSnapshot(hint) {
      const key = __amberjsNextSnapshotKey(hint);
      const received = __amberjsSerializeSnapshotValue(actual);
      const expected = __amberjsSnapshots[key];
      if (expected === undefined) {
        if (__amberjsTestConfig.updateSnapshots) {
          __amberjsSnapshotUpdates[key] = received;
          return;
        }
        throw new Error(`Snapshot not found for ${key} in ${__amberjsTestConfig.snapshotPath}`);
      }
      if (expected !== received && __amberjsTestConfig.updateSnapshots) {
        __amberjsSnapshotUpdates[key] = received;
        return;
      }
      assertMatcher(
        expected === received,
        `Snapshot mismatch for ${key}\nExpected:\n${expected}\nReceived:\n${received}`,
        `Expected snapshot ${key} not to match`
      );
    },
    toMatchInlineSnapshot(expectedSnapshot) {
      const snapshotIndex = ++__amberjsInlineSnapshotCounter;
      const received = __amberjsSerializeSnapshotValue(actual);
      if (expectedSnapshot === undefined) {
        if (__amberjsTestConfig.updateSnapshots) {
          __amberjsInlineSnapshotUpdates.push({ index: snapshotIndex, content: received });
          return;
        }
        throw new Error("Inline snapshot value must be provided");
      }
      const expected = __amberjsNormalizeSnapshotText(String(expectedSnapshot));
      if (expected !== received && __amberjsTestConfig.updateSnapshots) {
        __amberjsInlineSnapshotUpdates.push({ index: snapshotIndex, content: received });
        return;
      }
      assertMatcher(
        expected === received,
        `Inline snapshot mismatch\nExpected:\n${expected}\nReceived:\n${received}`,
        "Expected inline snapshot not to match"
      );
    }
  };

  matchers.toBeCalled = matchers.toHaveBeenCalled;
  matchers.toBeCalledTimes = matchers.toHaveBeenCalledTimes;
  matchers.toBeCalledWith = matchers.toHaveBeenCalledWith;
  matchers.nthCalledWith = matchers.toHaveBeenNthCalledWith;
  matchers.lastCalledWith = matchers.toHaveBeenLastCalledWith;
  matchers.toReturn = matchers.toHaveReturned;
  matchers.toReturnTimes = matchers.toHaveReturnedTimes;
  matchers.toReturnWith = matchers.toHaveReturnedWith;
  matchers.nthReturnedWith = matchers.toHaveNthReturnedWith;
  matchers.lastReturnedWith = matchers.toHaveLastReturnedWith;

  __amberjsAddCustomMatchers(matchers, actual, negate);

  return __amberjsCountMatcherCalls(matchers);
}

function __amberjsAwaitExpectedPromiseState(actual, mode) {
  return Promise.resolve(actual).then(
    (value) => {
      if (mode === "rejects") {
        throw new Error(`Expected promise to reject, but it resolved with ${__amberjsFormatValue(value)}`);
      }
      return value;
    },
    (error) => {
      if (mode === "resolves") {
        throw new Error(`Expected promise to resolve, but it rejected with ${__amberjsFormatPromiseReason(error)}`);
      }
      return error;
    }
  );
}

function __amberjsCreateAsyncMatcherSet(actual, mode, negate) {
  const asyncMatchers = {};
  for (const matcherName of Object.keys(__amberjsBuildMatchers(undefined, false))) {
    asyncMatchers[matcherName] = async function (...args) {
      const settledValue = await __amberjsAwaitExpectedPromiseState(actual, mode);
      if (mode === "rejects" && matcherName === "toThrow") {
        __amberjsRecordAssertion();
        __amberjsAssertRejectedToThrow(settledValue, args[0], negate);
        return;
      }
      const matcher = __amberjsBuildMatchers(settledValue, negate)[matcherName];
      return matcher(...args);
    };
  }
  return asyncMatchers;
}

function __amberjsBuildAsyncMatchers(actual, mode) {
  const matchers = __amberjsCreateAsyncMatcherSet(actual, mode, false);
  matchers.not = __amberjsCreateAsyncMatcherSet(actual, mode, true);
  return matchers;
}

function expect(actual) {
  const matchers = __amberjsBuildMatchers(actual, false);
  matchers.not = __amberjsBuildMatchers(actual, true);
  matchers.resolves = __amberjsBuildAsyncMatchers(actual, "resolves");
  matchers.rejects = __amberjsBuildAsyncMatchers(actual, "rejects");
  return matchers;
}

expect.assertions = function expectAssertions(expectedCount) {
  __amberjsSetExpectedAssertionCount(expectedCount);
};

expect.hasAssertions = function expectHasAssertions() {
  __amberjsHasAssertionExpectation = true;
};

expect.extend = function expectExtend(matchers) {
  if (!matchers || typeof matchers !== "object") {
    throw new Error("expect.extend() expects an object of matcher functions");
  }
  for (const matcherName of Object.keys(matchers)) {
    if (typeof matchers[matcherName] !== "function") {
      throw new Error(`expect.extend matcher ${matcherName} must be a function`);
    }
    __amberjsCustomMatchers[matcherName] = matchers[matcherName];
  }
};

function __amberjsCreateAsymmetricMatcher(name, matcher) {
  return {
    __amberjsAsymmetricMatcher: true,
    asymmetricMatch: matcher,
    toString() {
      return name;
    }
  };
}

function __amberjsInvertAsymmetricMatcher(matcher) {
  return __amberjsCreateAsymmetricMatcher(`Not<${matcher.toString()}>`, function (actual) {
    return !matcher.asymmetricMatch(actual);
  });
}

expect.any = function expectAny(expectedConstructor) {
  if (typeof expectedConstructor !== "function") {
    throw new Error("expect.any() expects a constructor function");
  }
  return __amberjsCreateAsymmetricMatcher(`Any<${expectedConstructor.name || "anonymous"}>`, function (actual) {
    if (expectedConstructor === String) {
      return typeof actual === "string" || actual instanceof String;
    }
    if (expectedConstructor === Number) {
      return typeof actual === "number" || actual instanceof Number;
    }
    if (expectedConstructor === Boolean) {
      return typeof actual === "boolean" || actual instanceof Boolean;
    }
    if (expectedConstructor === Function) {
      return typeof actual === "function";
    }
    if (expectedConstructor === Object) {
      return actual !== null && typeof actual === "object";
    }
    if (expectedConstructor === Array) {
      return Array.isArray(actual);
    }
    return actual instanceof expectedConstructor;
  });
};

expect.anything = function expectAnything() {
  return __amberjsCreateAsymmetricMatcher("Anything", function (actual) {
    return actual !== null && actual !== undefined;
  });
};

expect.objectContaining = function expectObjectContaining(sample) {
  if (sample === null || typeof sample !== "object" || Array.isArray(sample)) {
    throw new Error("expect.objectContaining() expects an object");
  }
  return __amberjsCreateAsymmetricMatcher("ObjectContaining", function (actual) {
    return __amberjsPartialObjectMatches(actual, sample);
  });
};

expect.arrayContaining = function expectArrayContaining(sample) {
  if (!Array.isArray(sample)) {
    throw new Error("expect.arrayContaining() expects an array");
  }
  return __amberjsCreateAsymmetricMatcher("ArrayContaining", function (actual) {
    if (!Array.isArray(actual)) {
      return false;
    }
    return sample.every((expectedItem) => {
      return actual.some((actualItem) => __amberjsValuesEqual(actualItem, expectedItem));
    });
  });
};

expect.stringContaining = function expectStringContaining(sample) {
  return __amberjsCreateAsymmetricMatcher("StringContaining", function (actual) {
    if (typeof actual !== "string" && !(actual instanceof String)) {
      return false;
    }
    return String(actual).includes(String(sample));
  });
};

expect.stringMatching = function expectStringMatching(sample) {
  const matcher = sample instanceof RegExp ? sample : new RegExp(String(sample));
  return __amberjsCreateAsymmetricMatcher("StringMatching", function (actual) {
    if (typeof actual !== "string" && !(actual instanceof String)) {
      return false;
    }
    matcher.lastIndex = 0;
    return matcher.test(String(actual));
  });
};

expect.closeTo = function expectCloseTo(expected, precision) {
  return __amberjsCreateAsymmetricMatcher("CloseTo", function (actual) {
    return __amberjsCloseTo(actual, expected, precision);
  });
};

expect.not = {};
expect.not.objectContaining = function expectNotObjectContaining(sample) {
  return __amberjsInvertAsymmetricMatcher(expect.objectContaining(sample));
};
expect.not.arrayContaining = function expectNotArrayContaining(sample) {
  return __amberjsInvertAsymmetricMatcher(expect.arrayContaining(sample));
};
expect.not.stringContaining = function expectNotStringContaining(sample) {
  return __amberjsInvertAsymmetricMatcher(expect.stringContaining(sample));
};
expect.not.stringMatching = function expectNotStringMatching(sample) {
  return __amberjsInvertAsymmetricMatcher(expect.stringMatching(sample));
};

function __amberjsShouldSkip(testCase, hasOnlyTests) {
  if (testCase.skip) {
    return true;
  }
  if (hasOnlyTests && !testCase.only) {
    return true;
  }
  if (__amberjsTestConfig.includePattern &&
      !__amberjsPatternMatches(__amberjsTestConfig.includePattern, testCase.name, testCase.suite)) {
    return true;
  }
  if (__amberjsTestConfig.skipPattern &&
      __amberjsPatternMatches(__amberjsTestConfig.skipPattern, testCase.name, testCase.suite)) {
    return true;
  }
  return false;
}

function __amberjsTimeoutError() {
  return new Error(`timed out after ${__amberjsTestConfig.timeoutSeconds}s`);
}

function __amberjsAwaitTestResult(result) {
  if (!result || typeof result.then !== "function") {
    return Promise.resolve();
  }

  const timeoutMs = Number(__amberjsTestConfig.timeoutSeconds) <= 0
    ? 0
    : Number(__amberjsTestConfig.timeoutSeconds) * 1000;

  return new Promise((resolve, reject) => {
    let timeoutId = setTimeout(() => {
      reject(__amberjsTimeoutError());
    }, timeoutMs);

    Promise.resolve(result).then(
      () => {
        clearTimeout(timeoutId);
        resolve();
      },
      (error) => {
        clearTimeout(timeoutId);
        reject(error);
      }
    );
  });
}

function __amberjsNormalizeDoneError(error) {
  if (error === undefined || error === null) {
    return undefined;
  }
  if (error instanceof Error) {
    return error;
  }
  return new Error(String(error));
}

function __amberjsAwaitDoneCallback(callback, callbackKind) {
  const timeoutMs = Number(__amberjsTestConfig.timeoutSeconds) <= 0
    ? 0
    : Number(__amberjsTestConfig.timeoutSeconds) * 1000;

  return new Promise((resolve, reject) => {
    let settled = false;
    let timeoutId = setTimeout(() => {
      finish(__amberjsTimeoutError());
    }, timeoutMs);

    function finish(error) {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timeoutId);
      const normalizedError = __amberjsNormalizeDoneError(error);
      if (normalizedError) {
        reject(normalizedError);
      } else {
        resolve();
      }
    }

    try {
      const result = callback(finish);
      if (result && typeof result.then === "function") {
        finish(new Error(`${callbackKind} callback cannot both use done callback and return a Promise`));
      }
    } catch (error) {
      finish(error);
    }
  });
}

function __amberjsRunTestCallback(callback) {
  if (callback.length > 0) {
    return __amberjsAwaitDoneCallback(callback, "Test");
  }
  return __amberjsAwaitTestResult(callback());
}

function __amberjsRunHookCallback(callback) {
  if (callback.length > 0) {
    return __amberjsAwaitDoneCallback(callback, "Hook");
  }
  return __amberjsAwaitTestResult(callback());
}

function __amberjsRunHooks(hooks) {
  let chain = Promise.resolve();
  for (const hook of hooks) {
    chain = chain.then(() => {
      try {
        return __amberjsRunHookCallback(hook);
      } catch (error) {
        return Promise.reject(error);
      }
    });
  }
  return chain;
}

function __amberjsBuildRemainingSuiteTests(hasOnlyTests) {
  const remaining = {};
  for (const testCase of __amberjsTestQueue) {
    if (__amberjsShouldSkip(testCase, hasOnlyTests)) {
      continue;
    }
    for (const suiteId of testCase.suiteIds) {
      remaining[suiteId] = (remaining[suiteId] || 0) + 1;
    }
  }
  return remaining;
}

function __amberjsRunBeforeAllHooks(testCase) {
  let chain = Promise.resolve();
  for (const suiteId of testCase.suiteIds) {
    const suite = __amberjsSuiteRegistry[suiteId];
    if (!suite || (__amberjsRemainingSuiteTests[suiteId] || 0) === 0) {
      continue;
    }
    if (!__amberjsStartedSuites[suiteId]) {
      __amberjsStartedSuites[suiteId] = true;
      chain = chain.then(() => __amberjsRunHooks(suite.beforeAll)).catch((error) => {
        __amberjsFailedBeforeAllSuites[suiteId] = true;
        const suiteName = suite.name || testCase.suite || "file";
        __amberjsRecordFailure(`${suiteName} beforeAll`, error);
        return Promise.reject(error);
      });
    }
  }

  return chain;
}

function __amberjsHasFailedBeforeAllSuite(testCase) {
  return testCase.suiteIds.some((suiteId) => Boolean(__amberjsFailedBeforeAllSuites[suiteId]));
}

function __amberjsRunAfterAllHooksForTest(testCase) {
  for (const suiteId of testCase.suiteIds) {
    if ((__amberjsRemainingSuiteTests[suiteId] || 0) > 0) {
      __amberjsRemainingSuiteTests[suiteId]--;
    }
  }

  let hooks = [];
  for (let i = testCase.suiteIds.length - 1; i >= 0; i--) {
    const suiteId = testCase.suiteIds[i];
    const suite = __amberjsSuiteRegistry[suiteId];
    if (!suite || !__amberjsStartedSuites[suiteId] || __amberjsFinishedSuites[suiteId]) {
      continue;
    }
    if ((__amberjsRemainingSuiteTests[suiteId] || 0) === 0) {
      __amberjsFinishedSuites[suiteId] = true;
      hooks = hooks.concat(suite.afterAll);
    }
  }

  return __amberjsRunHooks(hooks).catch((error) => {
    __amberjsRecordFailure("afterAll", error);
  });
}

function __amberjsRunRemainingAfterAllHooks() {
  let hooks = [];
  for (let i = __amberjsSuiteOrder.length - 1; i >= 0; i--) {
    const suiteId = __amberjsSuiteOrder[i];
    const suite = __amberjsSuiteRegistry[suiteId];
    if (!suite || !__amberjsStartedSuites[suiteId] || __amberjsFinishedSuites[suiteId]) {
      continue;
    }
    __amberjsFinishedSuites[suiteId] = true;
    hooks = hooks.concat(suite.afterAll);
  }

  return __amberjsRunHooks(hooks).catch((error) => {
    __amberjsRecordFailure("afterAll", error);
  });
}

function __amberjsRunOneTest(testCase) {
  if (__amberjsTestConfig.bail && __amberjsTestFailed > 0) {
    __amberjsTestSkipped++;
    return Promise.resolve();
  }

  let beforeEachErrors = [];
  let testErrors = [];
  let afterEachErrors = [];
  let assertionErrors = [];

  __amberjsResetAssertionState();
  __amberjsCurrentTestName = __amberjsSnapshotTestName(testCase);

  return __amberjsRunHooks(testCase.beforeEachHooks)
    .catch((error) => {
      beforeEachErrors.push(error);
    })
    .then(() => {
      if (beforeEachErrors.length > 0) {
        return undefined;
      }
      try {
        return __amberjsRunTestCallback(testCase.callback).catch((error) => {
          testErrors.push(error);
        });
      } catch (error) {
        testErrors.push(error);
        return undefined;
      }
    })
    .then(() => {
      return __amberjsRunHooks(testCase.afterEachHooks);
    })
    .catch((error) => {
      afterEachErrors.push(error);
    })
    .then(() => {
      try {
        __amberjsVerifyAssertionState();
      } catch (error) {
        assertionErrors.push(error);
      }
      __amberjsCurrentTestName = "";

      const infrastructureErrors = beforeEachErrors.concat(afterEachErrors);
      const expectedFailureErrors = testErrors.concat(assertionErrors);
      if (testCase.failing) {
        if (infrastructureErrors.length > 0) {
          const message = infrastructureErrors
            .map((error) => error && error.message ? error.message : String(error))
            .join("; ");
          __amberjsRecordFailure(testCase.name, new Error(message));
        } else if (expectedFailureErrors.length > 0) {
          __amberjsTestPassed++;
        } else {
          __amberjsRecordFailure(testCase.name, new Error("Expected failing test to fail, but it passed"));
        }
        return;
      }

      const errors = infrastructureErrors.concat(expectedFailureErrors);
      if (errors.length === 0) {
        __amberjsTestPassed++;
      } else {
        const message = errors
          .map((error) => error && error.message ? error.message : String(error))
          .join("; ");
        __amberjsRecordFailure(testCase.name, new Error(message));
      }
    });
}

function __amberjsRunTests() {
  if (__amberjsTestQueue.length === 0 && __amberjsTestFailed === 0) {
    return Promise.reject(new Error("No tests found in test file"));
  }

  const hasOnlyTests = __amberjsTestQueue.some((testCase) => testCase.only);
  __amberjsRemainingSuiteTests = __amberjsBuildRemainingSuiteTests(hasOnlyTests);
  let chain = Promise.resolve();

  for (const testCase of __amberjsTestQueue) {
    chain = chain.then(() => {
      if (__amberjsShouldSkip(testCase, hasOnlyTests)) {
        __amberjsTestSkipped++;
        return undefined;
      }
      if (__amberjsTestConfig.bail && __amberjsTestFailed > 0) {
        __amberjsTestSkipped++;
        return undefined;
      }
      if (__amberjsHasFailedBeforeAllSuite(testCase)) {
        __amberjsTestSkipped++;
        return __amberjsRunAfterAllHooksForTest(testCase);
      }
      return __amberjsRunBeforeAllHooks(testCase)
        .then(() => __amberjsRunOneTest(testCase))
        .catch(() => {
          if (__amberjsHasFailedBeforeAllSuite(testCase)) {
            __amberjsTestSkipped++;
          }
          return undefined;
        })
        .then(() => __amberjsRunAfterAllHooksForTest(testCase));
    });
  }

  return chain.then(() => __amberjsRunRemainingAfterAllHooks()).then(() => {
    if (__amberjsTestFailed > 0) {
      throw new Error(__amberjsTestErrors.join("\n"));
    }

    const summary = `${__amberjsTestPassed} passed, ${__amberjsTestFailed} failed, ${__amberjsTestSkipped} skipped`;
    const snapshotUpdated = Object.keys(__amberjsSnapshotUpdates).length > 0;
    const inlineSnapshotUpdated = __amberjsInlineSnapshotUpdates.length > 0;
    return JSON.stringify({
      summary,
      snapshotUpdated,
      snapshotContent: snapshotUpdated ? __amberjsBuildSnapshotFileContent() : null,
      inlineSnapshotUpdated,
      inlineSnapshotUpdates: __amberjsInlineSnapshotUpdates
    });
  });
}

"#,
    );
    wrapped.push_str(source);
    wrapped.push_str(
        r#"

__amberjsRunTests();
"#,
    );
    wrapped
}

#[allow(clippy::needless_return)]
fn main() -> Result<()> {
    // 0. Standalone binary self-execution check (compiled via `amber compile`)
    if let Ok(Some(standalone_script)) = amberjs::tooling::compiler::detect_standalone_payload() {
        let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new()
            .map_err(|e| anyhow!("Failed to initialize standalone runtime: {}", e))?;
        let mut argv = Vec::new();
        let exe_str = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "app".to_string());
        argv.push(exe_str.clone());
        argv.push(exe_str);
        argv.extend(std::env::args().skip(1));
        runtime.set_process_argv(argv);
        if let Err(e) = runtime.execute_code(&standalone_script) {
            eprintln!("Error executing standalone binary: {}", e);
            std::process::exit(1);
        }
        return Ok(());
    }

    let cli = Cli::parse();
    let verbose = cli.verbose;

    // Handle subcommands
    match cli.command {
        Some(Command::Repl) => {
            amberjs::repl::run_interactive_repl(verbose)?;
            return Ok(());
        }
        Some(Command::Run {
            permissions,
            file,
            args,
            watch,
            debounce,
            websocket_port,
            preloads,
            require,
            export_tools,
            workers,
            inspect,
            inspect_brk,
            inspect_port,
            warm,
        }) => {
            let inspector = if inspect || inspect_brk {
                let inspector = amberjs::tooling::inspector::InspectorServer::new(
                    "127.0.0.1",
                    inspect_port,
                    &file.to_string_lossy(),
                );
                inspector.start()?;
                Some(inspector)
            } else {
                None
            };
            // Check if target is a package.json script name (e.g., `amber run build`)
            if !file.exists() {
                if let Some(script_name) = file.to_str() {
                    if let Some(pkg_path) = amberjs::task_runner::find_package_json(Path::new("."))
                    {
                        if let Ok(scripts) = amberjs::task_runner::load_scripts(&pkg_path) {
                            if scripts.contains_key(script_name) {
                                let status = amberjs::task_runner::run_script(
                                    Path::new("."),
                                    script_name,
                                    &args,
                                )?;
                                if !status.success() {
                                    std::process::exit(status.code().unwrap_or(1));
                                }
                                return Ok(());
                            }
                        }
                    }
                }
            }

            apply_permission_cli_options(&permissions)?;
            allow_sandbox_entry_file(permissions.sandbox, &file)?;
            if export_tools {
                print_exported_tools(&file)?;
                return Ok(());
            }

            // Combine preloads and require (they are equivalent)
            let all_preloads: Vec<String> =
                preloads.iter().chain(require.iter()).cloned().collect();

            if verbose {
                println!("Running Amber on: {}", file.display());
            }
            if verbose && !args.is_empty() {
                println!("Args: {:?}", args);
            }
            if verbose && !all_preloads.is_empty() {
                println!("Preloaded modules: {:?}", all_preloads);
            }

            if watch {
                check_file_read_permission(&file)?;

                // Watch mode: enable hot reload
                println!("🔥 Watch mode enabled (debounce: {}ms)", debounce);

                // Get the directory to watch
                let watch_path = if file.is_file() {
                    file.parent().unwrap_or(&file).to_path_buf()
                } else {
                    file.clone()
                };

                // Create WebSocket hot reloader
                let ws_config = amberjs::watcher_websocket::WebSocketConfig {
                    port: websocket_port,
                    host: "127.0.0.1".to_string(),
                    channel_capacity: 100,
                };
                let ws_reloader =
                    amberjs::watcher_websocket::WebSocketHotReloader::with_config(ws_config);

                // Create a hot reloader for file watching
                let watcher_config = amberjs::watcher::WatcherConfigBuilder::new()
                    .debounce_ms(debounce)
                    .build();
                let mut reloader = amberjs::watcher::HotReloader::with_config(watcher_config);

                let rx = reloader
                    .watch(&watch_path)
                    .map_err(|e| anyhow::anyhow!("Failed to start watcher: {}", e))?;

                println!("👀 Watching for changes in {:?}...", watch_path);
                println!(
                    "🔌 WebSocket server ready on ws://127.0.0.1:{}",
                    websocket_port
                );

                // Initial execution
                let execute_file = |file: &PathBuf| -> Result<()> {
                    let code = read_and_compile_source(file)?;

                    amberjs::v8_snapshot::enable_startup_snapshot_for_cli();
                    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new()
                        .expect("Failed to create runtime");
                    runtime.set_process_argv(build_process_argv(file, &args));
                    runtime.set_main_module_path(file);
                    runtime.set_http_server_keep_alive(true);

                    match runtime.execute_code(&code) {
                        Ok(result) => {
                            if !result.trim().is_empty() {
                                println!("\n📊 Result: {}", result);
                            }
                            println!("✅ Executed successfully");
                        }
                        Err(e) => {
                            eprintln!("❌ Error: {}", e);
                        }
                    }
                    Ok(())
                };

                // Initial run
                execute_file(&file)?;

                // Watch mode is the only CLI path that needs Tokio. Keeping the
                // runtime local avoids paying multi-thread scheduler startup for
                // short-lived commands such as `amber eval` and `amber run`.
                let watch_runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| anyhow!("Failed to create watch runtime: {}", error))?;

                // Start WebSocket server in background
                let ws_reloader_clone = ws_reloader.clone();
                let _ws_handle = watch_runtime.spawn(async move {
                    let _ = ws_reloader_clone.start().await;
                });

                // Give WebSocket server time to start
                std::thread::sleep(std::time::Duration::from_millis(100));

                // Watch for changes
                loop {
                    match rx.recv() {
                        Ok(change) => {
                            let file_name = change
                                .path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "unknown".to_string());

                            println!("\n🔄 Detected change: {}", file_name);

                            // Broadcast via WebSocket
                            ws_reloader.broadcast_reload(
                                change.path.to_string_lossy().to_string(),
                                "modified".to_string(),
                            );

                            // Clear console for better readability
                            print!("\x1B[2J\x1B[1;1H");

                            let start = std::time::Instant::now();
                            if let Err(e) = execute_file(&file) {
                                eprintln!("❌ Reload failed: {}", e);
                            }
                            let duration = start.elapsed().as_millis();
                            println!("🔄 Reloaded in {}ms", duration);
                        }
                        Err(e) => {
                            eprintln!("❌ Watch error: {}", e);
                            break;
                        }
                    }
                }

                // Stop WebSocket server
                ws_reloader.stop();
            } else {
                // Normal execution mode (Single or Multi-worker)
                let num_workers = if workers > 1 {
                    workers
                } else {
                    std::env::var("AMBER_WORKERS")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1)
                };

                let code = read_and_compile_source(&file)?;

                if num_workers > 1 {
                    if verbose {
                        println!(
                            "🚀 Starting {} parallel multi-isolate workers...",
                            num_workers
                        );
                    }
                    let mut worker_handles = Vec::with_capacity(num_workers - 1);

                    for worker_id in 1..num_workers {
                        let file_clone = file.clone();
                        let args_clone = args.clone();
                        let preloads_clone = all_preloads.clone();
                        let code_clone = code.clone();

                        let handle = std::thread::Builder::new()
                            .name(format!("amber-worker-{}", worker_id))
                            .spawn(move || {
                                amberjs::v8_snapshot::enable_startup_snapshot_for_cli();
                                let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new()
                                    .expect("Failed to create worker runtime");
                                runtime
                                    .set_process_argv(build_process_argv(&file_clone, &args_clone));
                                runtime.set_main_module_path(&file_clone);
                                runtime.set_http_server_keep_alive(true);

                                for preload in &preloads_clone {
                                    if let Ok(preload_code) = preload_require_source(preload) {
                                        let _ = runtime.execute_code(&preload_code);
                                    }
                                }

                                if let Err(e) = runtime.execute_code(&code_clone) {
                                    eprintln!("[Worker {} Error] {}", worker_id, e);
                                }
                            })
                            .expect("Failed to spawn worker thread");
                        worker_handles.push(handle);
                    }

                    // Run worker 0 on the main thread
                    amberjs::v8_snapshot::enable_startup_snapshot_for_cli();
                    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new()
                        .expect("Failed to create runtime");
                    runtime.set_process_argv(build_process_argv(&file, &args));
                    runtime.set_main_module_path(&file);
                    runtime.set_http_server_keep_alive(true);

                    for preload in &all_preloads {
                        if let Ok(preload_code) = preload_require_source(preload) {
                            let _ = runtime.execute_code(&preload_code);
                        }
                    }

                    match runtime.execute_code(&code) {
                        Ok(result) => {
                            let trimmed = result.trim();
                            if !trimmed.is_empty() && trimmed != "undefined" {
                                println!("{trimmed}");
                            }
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    }

                    for handle in worker_handles {
                        let _ = handle.join();
                    }
                    return Ok(());
                }

                // Default single-isolate execution.
                // `--warm` is a no-op here: a new process has no standby isolate to reuse.
                let _ = warm;
                let startup_t0 = Instant::now();
                startup_mark(startup_t0, "cli_ready");
                amberjs::v8_snapshot::enable_startup_snapshot_for_cli();
                let mut runtime = if let Some(mem_mb) = permissions.max_memory {
                    amberjs::runtime_minimal::MinimalRuntime::with_memory_limit(mem_mb)
                        .expect("Failed to create runtime with memory limit")
                } else {
                    amberjs::runtime_minimal::MinimalRuntime::new()
                        .expect("Failed to create runtime")
                };
                startup_mark(startup_t0, "runtime_created");
                runtime.set_process_argv(build_process_argv(&file, &args));
                runtime.set_main_module_path(&file);
                runtime.set_http_server_keep_alive(true);

                // Execute preload modules first
                for preload in &all_preloads {
                    if verbose {
                        println!("Loading preload: {}", preload);
                    }
                    let preload_code = preload_require_source(preload)?;

                    if let Err(e) = runtime.execute_code(&preload_code) {
                        return Err(anyhow!("Preload '{}' failed: {}", preload, e));
                    }
                }

                if inspect_brk {
                    if let Some(ref inspector) = inspector {
                        inspector.wait_while_evaluating(|expression| {
                            runtime.execute_code(expression).map_err(|e| e.to_string())
                        });
                    }
                }

                let timeout_ms = permissions.timeout;
                let watchdog = if let Some(ms) = timeout_ms {
                    let isolate_handle = runtime.isolate_handle();
                    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                    let cancel_clone = cancel.clone();
                    let thread = std::thread::spawn(move || {
                        let start = std::time::Instant::now();
                        while start.elapsed().as_millis() < ms as u128 {
                            if cancel_clone.load(std::sync::atomic::Ordering::Relaxed) {
                                return;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        if !cancel_clone.load(std::sync::atomic::Ordering::Relaxed) {
                            isolate_handle.terminate_execution();
                        }
                    });
                    Some((cancel, thread))
                } else {
                    None
                };

                let exec_result = runtime.execute_code(&code);
                startup_mark(startup_t0, "script_executed");

                if let Some((cancel, thread)) = watchdog {
                    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                    let _ = thread.join();
                }

                match exec_result {
                    Ok(result) => {
                        let trimmed = result.trim();
                        if !trimmed.is_empty() && trimmed != "undefined" {
                            println!("{trimmed}");
                        }
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        if let Some(ms) = timeout_ms {
                            if err_str.contains("execution terminated") || err_str.is_empty() {
                                eprintln!("Error: Execution timed out after {}ms", ms);
                                std::process::exit(1);
                            }
                        }
                        eprintln!("Error: {}", err_str);
                        std::process::exit(1);
                    }
                }
            }
            return Ok(());
        }
        Some(Command::Eval {
            permissions,
            code,
            warm,
        }) => {
            apply_permission_cli_options(&permissions)?;

            if verbose {
                println!("Evaluating JavaScript code");
            }

            let _ = warm;
            let startup_t0 = Instant::now();
            startup_mark(startup_t0, "cli_ready");
            amberjs::v8_snapshot::enable_startup_snapshot_for_cli();
            let mut runtime =
                amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
            startup_mark(startup_t0, "runtime_created");
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            if code.contains("import ") || code.contains("export ") || code.contains("import{") {
                runtime.set_main_module_path(cwd.join("eval.mjs"));
            } else {
                runtime.set_main_module_path(cwd.join("eval.js"));
            }

            match runtime.execute_code(&code) {
                Ok(result) => {
                    startup_mark(startup_t0, "script_executed");
                    let trimmed = result.trim();
                    if !trimmed.is_empty() && trimmed != "undefined" {
                        println!("{trimmed}");
                    }
                }
                Err(e) => {
                    startup_mark(startup_t0, "script_executed");
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            return Ok(());
        }
        Some(Command::Version) => {
            println!("Amber {}", env!("CARGO_PKG_VERSION"));
            println!("JavaScript/TypeScript runtime");
            println!("Built with Rust + V8");
            return Ok(());
        }
        Some(Command::Snapshot { action }) => {
            match action {
                SnapshotAction::Build => {
                    println!("🔨 Building V8 startup snapshot...");
                    let start = std::time::Instant::now();
                    match amberjs::v8_snapshot::rebuild_startup_blob() {
                        Ok(size) => {
                            let duration = start.elapsed().as_millis();
                            let path = amberjs::v8_snapshot::startup_blob_path();
                            println!(
                                "✅ Snapshot built successfully in {}ms ({} bytes)",
                                duration, size
                            );
                            println!("📁 Path: {}", path.display());
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to build snapshot: {e}");
                            std::process::exit(1);
                        }
                    }
                }
                SnapshotAction::Status => {
                    let status = amberjs::v8_snapshot::startup_blob_status();
                    println!("🐝 Amber Snapshot Status:");
                    println!("  Version:       {}", status.version);
                    println!(
                        "  Enabled:       {}",
                        if status.enabled { "yes" } else { "no" }
                    );
                    println!(
                        "  File Exists:   {}",
                        if status.exists { "yes" } else { "no" }
                    );
                    println!("  Size:          {} bytes", status.size_bytes);
                    println!("  Location:      {}", status.path.display());
                }
                SnapshotAction::Clean => match amberjs::v8_snapshot::clear_startup_blob_cache() {
                    Ok(true) => println!("✅ Snapshot cache removed successfully"),
                    Ok(false) => println!("ℹ️ Snapshot cache was not present"),
                    Err(e) => eprintln!("❌ Failed to clear snapshot cache: {e}"),
                },
            }
            return Ok(());
        }
        Some(Command::Test {
            permissions,
            file,
            test_name_pattern,
            test_only,
            test_skip,
            bail,
            parallel,
            update_snapshots,
            verbose,
            watch,
            coverage,
        }) => {
            apply_permission_cli_options(&permissions)?;
            let timeout = permissions.timeout;

            if parallel {
                eprintln!(
                    "amber test --parallel is not supported: V8 isolates cannot be shared across threads"
                );
                std::process::exit(2);
            }

            println!("🐝 Running tests...");

            // Build test filter from CLI options
            use amberjs::testing::enhanced_runner::TestFilter;
            let mut filter = TestFilter::new();

            // Handle test-only (shorthand for --test-name-pattern)
            if let Some(pattern) = &test_only {
                filter.only_tests = true;
                filter.include(pattern.clone());
                if verbose {
                    println!("  Filter: only tests matching '{}'", pattern);
                }
            }
            // Handle test-name-pattern
            if let Some(pattern) = &test_name_pattern {
                if filter.include_patterns.is_empty() {
                    filter.include(pattern.clone());
                }
                if verbose {
                    println!("  Filter: tests matching '{}'", pattern);
                }
            }
            // Handle test-skip
            if let Some(pattern) = &test_skip {
                filter.skip_tests = true;
                filter.exclude(pattern.clone());
                if verbose {
                    println!("  Filter: skip tests matching '{}'", pattern);
                }
            }

            let test_file_options = TestFileOptions {
                include_pattern: test_only.or(test_name_pattern),
                skip_pattern: test_skip,
                bail,
                timeout_seconds: timeout,
                update_snapshots,
            };

            if let Some(test_file) = file {
                if test_file.is_dir() {
                    use amberjs::testing::test_discoverer::{TestDiscoverer, TestDiscovererConfig};

                    let mut discoverer_config = TestDiscovererConfig {
                        root_path: test_file.clone(),
                        ..Default::default()
                    };
                    discoverer_config.exclude_patterns.extend([
                        ".git".to_string(),
                        "target".to_string(),
                        "dist".to_string(),
                        "manual".to_string(),
                        "__snapshots__".to_string(),
                    ]);
                    let discovery = TestDiscoverer::new(discoverer_config)
                        .discover_with_read_permission(|path| {
                            check_file_read_permission(path).map_err(|e| {
                                std::io::Error::new(
                                    std::io::ErrorKind::PermissionDenied,
                                    e.to_string(),
                                )
                            })
                        })?;

                    if discovery.test_files.is_empty() {
                        return Err(anyhow!("No test files found in {}", test_file.display()));
                    }

                    if parallel {
                        eprintln!(
                            "⚠️  --parallel is not supported for directory test mode; running serially"
                        );
                    }

                    let mut passed_files = 0;
                    let mut failed_files = 0;
                    for discovered in &discovery.test_files {
                        println!("Running test file: {}", discovered.display());
                        match execute_test_file(discovered, &test_file_options) {
                            Ok(result) => {
                                println!("Test result: {}", result);
                                passed_files += 1;
                            }
                            Err(e) => {
                                eprintln!("❌ Test failed in {}: {}", discovered.display(), e);
                                failed_files += 1;
                                if bail {
                                    eprintln!("🛑 Stopping on first failure");
                                    std::process::exit(1);
                                }
                            }
                        }
                    }
                    if failed_files > 0 {
                        eprintln!("❌ {failed_files} test file(s) failed, {passed_files} passed");
                        std::process::exit(1);
                    }
                    println!("✅ {passed_files} test file(s) passed");
                    if coverage {
                        let mut report = amberjs::tooling::coverage::CoverageReport::new();
                        for discovered in &discovery.test_files {
                            if let Ok(content) = std::fs::read_to_string(discovered) {
                                report.record_file(discovered, &content);
                            }
                        }
                        report.print_summary();
                        let _ = report.write_lcov(Path::new("coverage"));
                    }
                    return Ok(());
                }

                // Run specific test file
                println!("Running test file: {}", test_file.display());
                if parallel {
                    eprintln!("⚠️  --parallel is not supported for single-file test mode; running serially");
                }
                if verbose {
                    if bail {
                        println!("  Mode: bail on first failure");
                    }
                    if let Some(timeout) = timeout {
                        println!("  Timeout: {}s", timeout);
                    }
                }

                match execute_test_file(&test_file, &test_file_options) {
                    Ok(result) => {
                        println!("Test result: {}", result);
                        println!("✅ Tests passed!");
                        if coverage {
                            let mut report = amberjs::tooling::coverage::CoverageReport::new();
                            if let Ok(content) = std::fs::read_to_string(&test_file) {
                                report.record_file(&test_file, &content);
                            }
                            report.print_summary();
                            let _ = report.write_lcov(Path::new("coverage"));
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Test failed: {}", e);
                        if !watch {
                            std::process::exit(1);
                        }
                    }
                }

                if watch {
                    println!(
                        "\n👀 Watching for changes in {}... (Ctrl+C to quit)",
                        test_file.display()
                    );
                    let watch_dir = test_file
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .to_path_buf();
                    let watcher_config = amberjs::watcher::WatcherConfigBuilder::new()
                        .debounce_ms(200)
                        .build();
                    let mut reloader = amberjs::watcher::HotReloader::with_config(watcher_config);
                    let rx = reloader
                        .watch(&watch_dir)
                        .map_err(|e| anyhow::anyhow!("Failed to start watcher: {}", e))?;
                    loop {
                        if let Ok(change) = rx.recv() {
                            let ext = change
                                .path
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("");
                            if ext == "js" || ext == "ts" {
                                println!(
                                    "\n🔄 File changed: {}. Re-running test...",
                                    change.path.display()
                                );
                                let _ = execute_test_file(&test_file, &test_file_options);
                            }
                        }
                    }
                }
            } else {
                use amberjs::testing::test_discoverer::{TestDiscoverer, TestDiscovererConfig};

                let mut discoverer_config = TestDiscovererConfig {
                    root_path: std::env::current_dir()?,
                    ..Default::default()
                };
                discoverer_config.exclude_patterns.extend([
                    ".git".to_string(),
                    "target".to_string(),
                    "dist".to_string(),
                    "manual".to_string(),
                    "__snapshots__".to_string(),
                    "node_modules".to_string(),
                ]);
                let discovery = TestDiscoverer::new(discoverer_config)
                    .discover_with_read_permission(|path| {
                        check_file_read_permission(path).map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string())
                        })
                    })?;

                if !discovery.test_files.is_empty() {
                    if parallel {
                        eprintln!(
                            "⚠️  --parallel is not supported for discovered test mode; running serially"
                        );
                    }
                    if verbose {
                        println!("  Discovered {} test file(s)", discovery.test_files.len());
                        if bail {
                            println!("  Mode: bail on first failure");
                        }
                        if let Some(timeout) = timeout {
                            println!("  Timeout: {}s", timeout);
                        }
                    }

                    let mut passed_files = 0;
                    let mut failed_files = 0;
                    for test_file in &discovery.test_files {
                        println!("Running test file: {}", test_file.display());
                        match execute_test_file(test_file, &test_file_options) {
                            Ok(result) => {
                                println!("Test result: {}", result);
                                passed_files += 1;
                            }
                            Err(e) => {
                                eprintln!("❌ Test failed in {}: {}", test_file.display(), e);
                                failed_files += 1;
                                if bail {
                                    eprintln!("🛑 Stopping on first failure");
                                    std::process::exit(1);
                                }
                            }
                        }
                    }

                    println!(
                        "\n📊 Test File Summary: {} passed, {} failed",
                        passed_files, failed_files
                    );
                    if watch {
                        println!("\n👀 Watching for changes in workspace... (Ctrl+C to quit)");
                        let watcher_config = amberjs::watcher::WatcherConfigBuilder::new()
                            .debounce_ms(200)
                            .build();
                        let mut reloader =
                            amberjs::watcher::HotReloader::with_config(watcher_config);
                        let rx = reloader
                            .watch(Path::new("."))
                            .map_err(|e| anyhow::anyhow!("Failed to start watcher: {}", e))?;
                        loop {
                            if let Ok(change) = rx.recv() {
                                let ext = change
                                    .path
                                    .extension()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("");
                                if ext == "js" || ext == "ts" {
                                    println!(
                                        "\n🔄 File changed: {}. Re-running discovered tests...",
                                        change.path.display()
                                    );
                                    for f in &discovery.test_files {
                                        let _ = execute_test_file(f, &test_file_options);
                                    }
                                }
                            }
                        }
                    }
                    if coverage {
                        let mut report = amberjs::tooling::coverage::CoverageReport::new();
                        for test_file in &discovery.test_files {
                            if let Ok(content) = std::fs::read_to_string(test_file) {
                                report.record_file(test_file, &content);
                            }
                        }
                        report.print_summary();
                        let _ = report.write_lcov(Path::new("coverage"));
                    }
                    if failed_files > 0 {
                        std::process::exit(1);
                    }
                    return Ok(());
                }

                // Run built-in test suite with filtering
                let test_cases = [
                    ("1 + 1", "2"),
                    ("'Hello World'", "Hello World"),
                    ("[1, 2, 3].length", "3"),
                    ("console.log('test'); 42", "42"),
                    ("function add(a, b) { return a + b; } add(5, 3)", "8"),
                    ("[1, 2, 3, 4, 5].map(x => x * 2).join(',')", "2,4,6,8,10"),
                    ("JSON.parse('{\"name\": \"amberjs\"}').name", "amberjs"),
                    ("'hello'.toUpperCase()", "HELLO"),
                ];

                let mut passed = 0;
                let mut failed = 0;
                let mut skipped = 0;
                let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new()
                    .expect("Failed to create runtime");

                for (i, (input, expected)) in test_cases.iter().enumerate() {
                    let test_name = format!("test_{}", i);
                    let suite_name = "builtin_tests";

                    // Apply filter if set
                    if !filter.include_patterns.is_empty()
                        && !filter.matches(&test_name, suite_name)
                    {
                        if verbose {
                            println!("⏭️  Test {} skipped (filter mismatch)", i + 1);
                        }
                        skipped += 1;
                        continue;
                    }
                    if filter.skip_tests
                        && !filter.exclude_patterns.is_empty()
                        && !filter.matches(&test_name, suite_name)
                    {
                        if verbose {
                            println!("⏭️  Test {} skipped (excluded by filter)", i + 1);
                        }
                        skipped += 1;
                        continue;
                    }

                    match runtime.execute_code(input) {
                        Ok(result) => {
                            if result.trim() == *expected {
                                if verbose {
                                    println!(
                                        "✅ Test {} passed: {} = {}",
                                        i + 1,
                                        input,
                                        result.trim()
                                    );
                                }
                                passed += 1;
                            } else {
                                println!(
                                    "❌ Test {} failed: {} expected '{}' but got '{}'",
                                    i + 1,
                                    input,
                                    expected,
                                    result.trim()
                                );
                                failed += 1;
                                if bail {
                                    eprintln!("🛑 Stopping on first failure");
                                    std::process::exit(1);
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Test {} failed with error: {}", i + 1, e);
                            failed += 1;
                            if bail {
                                eprintln!("🛑 Stopping on first failure");
                                std::process::exit(1);
                            }
                        }
                    }
                }

                println!(
                    "\n📊 Test Summary: {} passed, {} failed, {} skipped",
                    passed, failed, skipped
                );
                if failed > 0 {
                    std::process::exit(1);
                }
            }
            return Ok(());
        }
        Some(Command::Bundle {
            permissions,
            entry,
            outfile,
            minify,
            sourcemap,
            target,
            tree_shake: _tree_shake,
        }) => {
            let import_map = permissions.import_map.clone();
            if let Err(e) = apply_permission_cli_options(&permissions) {
                bundle_cli_fail(e);
            }
            println!("📦 Bundling JavaScript/TypeScript with Amber Bundler 2.0 (oxc)...");

            if let Err(e) = check_file_read_permission(&entry) {
                bundle_cli_fail(e);
            }
            let output_path = outfile.unwrap_or_else(|| {
                let mut path = entry.clone();
                path.set_extension("bundle.js");
                path
            });
            if let Err(e) = check_file_write_permission(&output_path) {
                bundle_cli_fail(e);
            }

            let options = amberjs::tooling::bundler::BundleOptions {
                entry,
                outfile: Some(output_path.clone()),
                minify,
                sourcemap,
                target,
                import_map,
            };

            let bundle_out = match amberjs::tooling::bundler::bundle_project(&options) {
                Ok(out) => out,
                Err(e) => bundle_cli_fail(e),
            };
            println!(
                "✅ Bundle created: {} ({} modules, {} bytes)",
                output_path.display(),
                bundle_out.module_count,
                bundle_out.total_bytes
            );
            return Ok(());
        }
        Some(Command::Debug { permissions, file }) => {
            apply_permission_cli_options(&permissions)?;
            println!("🐝 Debugging script: {}", file.display());

            // Read and display the file content
            check_file_read_permission(&file)?;
            let code = std::fs::read_to_string(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

            let inspector = amberjs::tooling::inspector::InspectorServer::new(
                "127.0.0.1",
                9229,
                &file.to_string_lossy(),
            );
            let _ = inspector.start();

            // Create runtime with debug mode
            let mut runtime =
                amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");

            // Execute with detailed error reporting
            match runtime.execute_code(&code) {
                Ok(result) => {
                    println!("\n✅ Execution successful");
                    if !result.trim().is_empty() {
                        println!("Result: {}", result);
                    }
                }
                Err(e) => {
                    eprintln!("\n❌ Execution failed: {}", e);
                    std::process::exit(1);
                }
            }
            return Ok(());
        }
        Some(Command::Record {
            permissions,
            file,
            output,
            args,
        }) => {
            apply_permission_cli_options(&permissions)?;
            check_file_read_permission(&file)?;

            let output_path = output.unwrap_or_else(|| {
                let mut p = file.clone();
                let stem = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                p.set_file_name(format!("{}.amber-trace.json", stem));
                p
            });

            println!("📼 Recording execution to: {}", output_path.display());

            if let Ok(mut engine) = amberjs::replay::GLOBAL_REPLAY.write() {
                engine.start_recording(
                    Some(file.to_string_lossy().to_string()),
                    Some(output_path.to_string_lossy().to_string()),
                );
            }

            let code = read_and_compile_source(&file)?;
            let mut runtime =
                amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create runtime");
            runtime.set_process_argv(build_process_argv(&file, &args));
            runtime.set_main_module_path(&file);

            let run_res = runtime.execute_code(&code);

            if let Ok(mut engine) = amberjs::replay::GLOBAL_REPLAY.write() {
                if engine.mode == amberjs::replay::ReplayMode::Recording {
                    match engine.stop_recording(None) {
                        Ok(trace) => {
                            let stats = engine.get_stats();
                            println!(
                                "✅ Trace recorded: {} events ({} agent steps) -> {}",
                                trace.events.len(),
                                stats.step_count,
                                output_path.display()
                            );
                        }
                        Err(e) => {
                            eprintln!("⚠️ Failed to save trace: {}", e);
                        }
                    }
                }
            }

            if let Err(e) = run_res {
                eprintln!("\n❌ Execution error during recording: {}", e);
                std::process::exit(1);
            }

            return Ok(());
        }
        Some(Command::Replay {
            permissions,
            trace,
            verify: _,
            verbose,
        }) => {
            apply_permission_cli_options(&permissions)?;
            check_file_read_permission(&trace)?;

            println!("⏯️ Loading trace: {}", trace.display());

            let (script_path, total_events, step_count) = {
                let mut engine = amberjs::replay::GLOBAL_REPLAY
                    .write()
                    .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
                engine
                    .load_trace_from_file(&trace.to_string_lossy())
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                let stats = engine.get_stats();
                (stats.script.clone(), stats.total_events, stats.step_count)
            };

            if verbose {
                println!(
                    "ℹ️ Trace metadata: {} total events, {} agent steps",
                    total_events, step_count
                );
            }

            if let Some(target_script) = script_path {
                let script_buf = PathBuf::from(&target_script);
                if script_buf.exists() {
                    let code = read_and_compile_source(&script_buf)?;
                    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new()
                        .expect("Failed to create runtime");
                    runtime.set_process_argv(build_process_argv(&script_buf, &[]));
                    runtime.set_main_module_path(&script_buf);

                    if let Err(e) = runtime.execute_code(&code) {
                        eprintln!("\n❌ Replay Divergence Error: {}", e);
                        std::process::exit(1);
                    }
                    println!("✅ Replay finished successfully with zero divergence.");
                } else {
                    println!(
                        "⚠️ Original script '{}' not found, trace loaded into replay engine.",
                        target_script
                    );
                }
            } else {
                println!(
                    "✅ Trace loaded into replay engine ({} events).",
                    total_events
                );
            }

            return Ok(());
        }
        Some(Command::Serve {
            permissions,
            file,
            port,
            host,
            https,
            cert,
            key,
        }) => {
            apply_permission_cli_options(&permissions)?;
            let scheme = if https { "https" } else { "http" };
            let bind_target = format!("{scheme}://{host}:{port}");
            check_network_listen_permission(&bind_target)?;

            if https {
                let cert_path = match cert {
                    Some(path) => path,
                    None => {
                        eprintln!(
                            "error: amber serve --https requires --cert PATH (PEM certificate)"
                        );
                        std::process::exit(2);
                    }
                };
                let key_path = match key {
                    Some(path) => path,
                    None => {
                        eprintln!(
                            "error: amber serve --https requires --key PATH (PEM private key)"
                        );
                        std::process::exit(2);
                    }
                };
                if !Path::new(&cert_path).is_file() {
                    eprintln!("error: TLS certificate not found: {cert_path} (pass --cert PATH)");
                    std::process::exit(2);
                }
                if !Path::new(&key_path).is_file() {
                    eprintln!("error: TLS private key not found: {key_path} (pass --key PATH)");
                    std::process::exit(2);
                }
                let tls_cert =
                    amberjs::nodejs_core::http::load_tls_certificate(&cert_path, &key_path)
                        .map_err(|e| anyhow!("invalid TLS material: {e}"))?;
                let tls_config =
                    amberjs::nodejs_core::http::try_create_tls_server_config_http11(&tls_cert)
                        .map_err(|e| anyhow!(e))?;

                let effective_file = if let Some(f) = file {
                    Some(f)
                } else {
                    [
                        "app.ts",
                        "app.js",
                        "server.ts",
                        "server.js",
                        "index.ts",
                        "index.js",
                    ]
                    .iter()
                    .map(PathBuf::from)
                    .find(|p| p.exists() && p.is_file())
                };

                let addr = format!("{}:{}", host, port);
                let listener = std::net::TcpListener::bind(&addr)
                    .map_err(|e| anyhow!("failed to bind {addr}: {e}"))?;
                let bound = listener
                    .local_addr()
                    .unwrap_or_else(|_| addr.parse().unwrap());
                println!("🚀 Starting Amber Web Server on https://{}", bound);
                if let Some(ref file_path) = effective_file {
                    println!("📄 Serving application: {}", file_path.display());
                    check_file_read_permission(file_path)?;
                    let code = read_and_compile_source(file_path)?;
                    let mut runtime = if let Some(mem_mb) = permissions.max_memory {
                        amberjs::runtime_minimal::MinimalRuntime::with_memory_limit(mem_mb)?
                    } else {
                        amberjs::runtime_minimal::MinimalRuntime::new()?
                    };
                    runtime.set_main_module_path(file_path);
                    runtime.execute_code(HTTPS_FETCH_BRIDGE)?;
                    let wrapped_user_code = format!(
                        r#"
                    (function() {{
                        const module = {{ exports: {{}} }};
                        const exports = module.exports;
                        {}
                        globalThis.__amberjs_app__ = (module.exports && (module.exports.default || module.exports.fetch)) ? module.exports : (typeof fetch !== 'undefined' ? {{ fetch }} : module.exports);
                    }})();
                    "#,
                        code
                    );
                    let _ = runtime.execute_code(&wrapped_user_code);
                    println!("✅ Listening on https://{} (Ctrl+C to stop)", bound);
                    serve_https_fetch_loop(listener, tls_config, &mut runtime, &bound)?;
                } else {
                    println!("💡 No script specified, serving default health status");
                    println!("✅ Listening on https://{} (Ctrl+C to stop)", bound);
                    serve_https_health_loop(listener, tls_config)?;
                }
                return Ok(());
            }

            let effective_file = if let Some(f) = file {
                Some(f)
            } else {
                [
                    "app.ts",
                    "app.js",
                    "server.ts",
                    "server.js",
                    "index.ts",
                    "index.js",
                ]
                .iter()
                .map(PathBuf::from)
                .find(|p| p.exists() && p.is_file())
            };

            let addr = format!("{}:{}", host, port);
            println!("🚀 Starting Amber Web Server on http://{}", addr);

            if let Some(ref file_path) = effective_file {
                println!("📄 Serving application: {}", file_path.display());
                check_file_read_permission(file_path)?;

                let code = read_and_compile_source(file_path)?;
                let mut runtime = if let Some(mem_mb) = permissions.max_memory {
                    amberjs::runtime_minimal::MinimalRuntime::with_memory_limit(mem_mb)?
                } else {
                    amberjs::runtime_minimal::MinimalRuntime::new()?
                };
                runtime.set_main_module_path(file_path);

                let bridge_init = r#"
globalThis.__amberjs_app__ = undefined;
globalThis.__amberjs_handle_http__ = async function(method, url, headersJson, bodyStr) {
    try {
        const headers = JSON.parse(headersJson);
        const reqInit = { method, headers };
        if (method !== "GET" && method !== "HEAD" && bodyStr && bodyStr.length > 0) {
            reqInit.body = bodyStr;
        }
        const req = new Request(url, reqInit);
        let handler = globalThis.__amberjs_app__;
        if (handler && typeof handler.default === 'object' && typeof handler.default.fetch === 'function') {
            handler = handler.default.fetch.bind(handler.default);
        } else if (handler && typeof handler.default === 'function') {
            handler = handler.default;
        } else if (handler && typeof handler.fetch === 'function') {
            handler = handler.fetch;
        } else if (typeof globalThis.fetchHandler === 'function') {
            handler = globalThis.fetchHandler;
        }
        if (typeof handler !== 'function') {
            return JSON.stringify({ status: 404, headers: { "content-type": "text/plain" }, body: "Not Found: No fetch handler exported" });
        }
        const res = await handler(req);
        const status = (res && res.status) ? res.status : 200;
        const resHeaders = {};
        if (res && res.headers && typeof res.headers.forEach === 'function') {
            res.headers.forEach((v, k) => { resHeaders[k] = v; });
        }
        let bodyText = "";
        if (res) {
            if (typeof res._bodyText === 'string') {
                bodyText = res._bodyText;
            } else if (typeof res.text === 'function') {
                try {
                    bodyText = await res.text();
                } catch (_) {
                    bodyText = res.body ? String(res.body) : "";
                }
            } else {
                bodyText = res.body ? String(res.body) : "";
            }
        }
        return JSON.stringify({ status, headers: resHeaders, body: bodyText });
    } catch (e) {
        return JSON.stringify({ status: 500, headers: { "content-type": "text/plain" }, body: "Internal Server Error: " + (e ? e.message : e) });
    }
};
"#;
                runtime.execute_code(bridge_init)?;

                // Execute user code and capture export
                let wrapped_user_code = format!(
                    r#"
                    (function() {{
                        const module = {{ exports: {{}} }};
                        const exports = module.exports;
                        {}
                        globalThis.__amberjs_app__ = (module.exports && (module.exports.default || module.exports.fetch)) ? module.exports : (typeof fetch !== 'undefined' ? {{ fetch }} : module.exports);
                    }})();
                    "#,
                    code
                );
                let _ = runtime.execute_code(&wrapped_user_code);

                let server = tiny_http::Server::http(&addr)
                    .map_err(|e| anyhow::anyhow!("failed to bind {}: {}", addr, e))?;
                println!("✅ Listening on http://{} (Ctrl+C to stop)", addr);

                for mut request in server.incoming_requests() {
                    let method = request.method().as_str().to_string();
                    let url = format!("http://{}{}", addr, request.url());
                    let mut headers_map = std::collections::HashMap::new();
                    for h in request.headers() {
                        headers_map
                            .insert(h.field.as_str().to_string(), h.value.as_str().to_string());
                    }
                    let headers_json =
                        serde_json::to_string(&headers_map).unwrap_or_else(|_| "{}".to_string());
                    let mut body_str = String::new();
                    let _ = request.as_reader().read_to_string(&mut body_str);

                    let dispatch_script = format!(
                        r#"globalThis.__amberjs_handle_http__({}, {}, {}, {});"#,
                        serde_json::to_string(&method).unwrap(),
                        serde_json::to_string(&url).unwrap(),
                        serde_json::to_string(&headers_json).unwrap(),
                        serde_json::to_string(&body_str).unwrap(),
                    );

                    let resp_json = match runtime.execute_code(&dispatch_script) {
                        Ok(raw_json) => raw_json,
                        Err(e) => format!(
                            r#"{{"status":500,"headers":{{"content-type":"text/plain"}},"body":"Handler error: {}"}}"#,
                            e
                        ),
                    };

                    let val: serde_json::Value =
                        serde_json::from_str(resp_json.trim()).unwrap_or_default();
                    let status_code =
                        val.get("status").and_then(|s| s.as_u64()).unwrap_or(200) as u16;
                    let body_text = val.get("body").and_then(|b| b.as_str()).unwrap_or("");
                    let mut resp = tiny_http::Response::from_string(body_text.to_string())
                        .with_status_code(status_code);

                    if let Some(headers_obj) = val.get("headers").and_then(|h| h.as_object()) {
                        for (k, v) in headers_obj {
                            if let Some(v_str) = v.as_str() {
                                if let Ok(header) =
                                    tiny_http::Header::from_bytes(k.as_bytes(), v_str.as_bytes())
                                {
                                    resp = resp.with_header(header);
                                }
                            }
                        }
                    }
                    let _ = request.respond(resp);
                }
            } else {
                println!("💡 No script specified, serving default health status");
                println!(
                    "💡 Tip: Pass a script file `amber serve app.ts` to serve a custom web app"
                );
                let server = tiny_http::Server::http(&addr)
                    .map_err(|e| anyhow::anyhow!("failed to bind {}: {}", addr, e))?;
                println!("✅ Listening on http://{} (Ctrl+C to stop)", addr);
                for request in server.incoming_requests() {
                    let response = tiny_http::Response::from_string(concat!(
                        "{\"runtime\":\"amberjs\",\"ok\":true,\"version\":\"",
                        env!("CARGO_PKG_VERSION"),
                        "\"}\n"
                    ))
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .unwrap(),
                    );
                    let _ = request.respond(response);
                }
            }
            return Ok(());
        }
        Some(Command::Init { permissions, name }) => {
            apply_permission_cli_options(&permissions)?;
            let project_name = name.as_deref().unwrap_or("my-amberjs-project");
            println!("📦 Initializing new project: {}", project_name);
            let project_dir = std::path::Path::new(project_name);
            let package_json_path = project_dir.join("package.json");
            let index_path = project_dir.join("index.js");
            check_file_write_permission(project_dir)?;
            check_file_write_permission(&package_json_path)?;
            check_file_write_permission(&index_path)?;

            // Create project directory
            std::fs::create_dir_all(project_dir)?;

            // Create package.json
            let package_json = format!(
                "{{
  \"name\": \"{}\",
  \"version\": \"0.1.0\",
  \"description\": \"A Amber project\",
  \"main\": \"index.js\",
  \"scripts\": {{
    \"start\": \"amber run index.js\"
  }},
  \"dependencies\": {{}},
  \"devDependencies\": {{}}
}}",
                project_name
            );

            std::fs::write(&package_json_path, package_json)?;

            // Create example file
            let example_code = "console.log('Hello from Amber!');\n";
            std::fs::write(&index_path, example_code)?;

            println!("✅ Project initialized!");
            println!("  Project directory: {}", project_name);
            println!("  Entry file: {}/index.js", project_name);
            println!("\nRun 'cd {} && amber run index.js' to start", project_name);
            return Ok(());
        }
        Some(Command::Add {
            permissions,
            package,
            save_exact,
            dev,
        }) => {
            apply_permission_cli_options(&permissions)?;
            println!("📦 Adding dependency: {}", package);
            println!("  Save exact: {}", save_exact);
            println!("  As devDependency: {}", dev);

            // Parse package name and version (`@scope/name@version` included)
            let (name, version) = amberjs::package_manager::parse_npm_package_spec(&package);

            println!("  Package: {}", name);
            println!("  Version: {}", version);

            // Check if package.json exists
            let package_json_path = std::path::Path::new("package.json");
            if !package_json_path.exists() {
                return Err(anyhow!(
                    "package.json not found in current directory. Run 'amber init' first."
                ));
            }
            check_file_read_permission(package_json_path)?;
            check_file_write_permission(package_json_path)?;
            let lock_path = std::path::Path::new("package-lock.json");
            if lock_path.exists() {
                check_file_read_permission(lock_path)?;
            }
            check_file_write_permission(lock_path)?;

            // Create package manager
            let config = amberjs::package_manager::PackageManagerConfig::default();
            let pm = amberjs::package_manager::PackageManager::new(config)
                .map_err(|e| anyhow!("Failed to create package manager: {}", e))?;

            // Install the package
            match pm.install_package(&name, &version) {
                Ok(result) => {
                    println!("✅ Installed {}@{}", name, result.package.version);

                    // Read existing package.json
                    check_file_read_permission(package_json_path)?;
                    let content = std::fs::read_to_string(package_json_path)
                        .map_err(|e| anyhow!("Failed to read package.json: {}", e))?;

                    let mut package_data: serde_json::Value = serde_json::from_str(&content)
                        .map_err(|e| anyhow!("Failed to parse package.json: {}", e))?;

                    // Determine version string to save
                    let version_to_save = if save_exact {
                        result.package.version.clone()
                    } else {
                        format!("^{}", result.package.version)
                    };

                    // Add to appropriate dependencies section
                    let dep_key = if dev {
                        "devDependencies"
                    } else {
                        "dependencies"
                    };

                    if let Some(deps) = package_data.get_mut(dep_key) {
                        if deps.is_object() {
                            deps.as_object_mut()
                                .unwrap()
                                .insert(name.clone(), serde_json::Value::String(version_to_save));
                        }
                    } else {
                        // Create the dependencies section if it doesn't exist
                        package_data[dep_key] = serde_json::json!({ &name: version_to_save });
                    }

                    // Write updated package.json
                    let updated_content = serde_json::to_string_pretty(&package_data)
                        .map_err(|e| anyhow!("Failed to serialize package.json: {}", e))?;
                    check_file_write_permission(package_json_path)?;
                    std::fs::write(package_json_path, updated_content)
                        .map_err(|e| anyhow!("Failed to write package.json: {}", e))?;

                    println!("✅ Added '{}' to {}", name, dep_key);

                    // Generate/update package-lock.json
                    if let Some(project_name) = package_data.get("name").and_then(|n| n.as_str()) {
                        let project_version = package_data
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("1.0.0");

                        if lock_path.exists() {
                            // Update existing lock file with new dependency
                            let locked_dep = amberjs::package_manager::LockedDependency {
                                version: result.package.version.clone(),
                                resolved: result.tarball_url.clone().or_else(|| {
                                    Some(format!(
                                        "https://registry.npmjs.org/{}/-/{}-{}.tgz",
                                        name,
                                        name.split('/').next_back().unwrap_or(&name),
                                        result.package.version
                                    ))
                                }),
                                integrity: result.integrity.clone(),
                                dev: Some(dev),
                                dependencies: None,
                            };
                            pm.update_package_lock(
                                lock_path,
                                project_name,
                                project_version,
                                vec![(name, locked_dep)],
                            )?;
                        } else {
                            // Generate new lock file
                            pm.generate_package_lock(lock_path, project_name, project_version)?;
                        }
                        println!("✅ Updated package-lock.json");
                    }

                    return Ok(());
                }
                Err(e) => {
                    return Err(anyhow!("Failed to install package: {}", e));
                }
            }
        }
        Some(Command::Remove {
            permissions,
            package,
        }) => {
            apply_permission_cli_options(&permissions)?;
            println!("🗑️  Removing dependency: {}", package);

            // Check if package.json exists
            let package_json_path = std::path::Path::new("package.json");
            check_file_read_permission(package_json_path)?;
            if !package_json_path.exists() {
                return Err(anyhow!("package.json not found in current directory"));
            }

            // Read package.json
            let content = std::fs::read_to_string(package_json_path)
                .map_err(|e| anyhow!("Failed to read package.json: {}", e))?;

            // Parse JSON
            let mut package_data: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse package.json: {}", e))?;

            // Track what was removed
            let mut removed_from = Vec::new();

            // Remove from dependencies
            if let Some(deps) = package_data.get_mut("dependencies") {
                if deps.is_object() && deps.get(&package).is_some() {
                    deps.as_object_mut().unwrap().remove(&package);
                    removed_from.push("dependencies");
                }
            }

            // Remove from devDependencies
            if let Some(dev_deps) = package_data.get_mut("devDependencies") {
                if dev_deps.is_object() && dev_deps.get(&package).is_some() {
                    dev_deps.as_object_mut().unwrap().remove(&package);
                    removed_from.push("devDependencies");
                }
            }

            // Remove from optionalDependencies
            if let Some(optional_deps) = package_data.get_mut("optionalDependencies") {
                if optional_deps.is_object() && optional_deps.get(&package).is_some() {
                    optional_deps.as_object_mut().unwrap().remove(&package);
                    removed_from.push("optionalDependencies");
                }
            }

            if removed_from.is_empty() {
                println!("⚠️  Package '{}' not found in package.json", package);
                println!("💡 Tip: Check if the package is listed in dependencies");
                return Ok(());
            }

            // Write updated package.json
            let updated_content = serde_json::to_string_pretty(&package_data)
                .map_err(|e| anyhow!("Failed to serialize package.json: {}", e))?;
            check_file_write_permission(package_json_path)?;
            std::fs::write(package_json_path, updated_content)
                .map_err(|e| anyhow!("Failed to write package.json: {}", e))?;

            println!("✅ Removed '{}' from {}", package, removed_from.join(", "));
            println!("💡 Run 'amber install' to update node_modules");

            return Ok(());
        }
        Some(Command::Install {
            permissions,
            frozen_lockfile,
        }) => {
            apply_permission_cli_options(&permissions)?;
            println!("📦 Installing dependencies from package.json...");

            // Check if package.json exists
            let package_json_path = std::path::Path::new("package.json");
            if !package_json_path.exists() {
                return Err(anyhow!(
                    "package.json not found in current directory. Run 'amber init' first."
                ));
            }

            // Read package.json
            check_file_read_permission(package_json_path)?;
            let content = std::fs::read_to_string(package_json_path)
                .map_err(|e| anyhow!("Failed to read package.json: {}", e))?;

            // Parse package.json
            let package_data: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse package.json: {}", e))?;
            let lock_path = std::path::Path::new("package-lock.json");
            if frozen_lockfile {
                validate_frozen_lockfile(&package_data, lock_path)?;
            } else if lock_path.exists() {
                check_file_read_permission(lock_path)?;
                check_file_write_permission(lock_path)?;
            } else {
                check_file_write_permission(lock_path)?;
            }

            // Create package manager
            let config = amberjs::package_manager::PackageManagerConfig::default();
            let pm = amberjs::package_manager::PackageManager::new(config)
                .map_err(|e| anyhow!("Failed to create package manager: {}", e))?;

            // Parse package.json using PackageManager's method
            let package_json = pm
                .parse_package_json(package_json_path)
                .map_err(|e| anyhow!("Failed to parse package.json: {}", e))?;

            println!("  Project: {}@{}", package_json.name, package_json.version);

            // Install all dependencies
            match pm.install_dependencies(&package_json) {
                Ok(results) => {
                    println!("✅ Installed {} dependencies", results.len());

                    // Show installed packages
                    for result in &results {
                        println!("  - {}@{}", result.package.name, result.package.version);
                    }

                    // Generate/update package-lock.json unless frozen mode made it read-only.
                    if frozen_lockfile {
                        println!("✅ Verified frozen package-lock.json");
                    } else if let Some(project_name) =
                        package_data.get("name").and_then(|n| n.as_str())
                    {
                        let project_version = package_data
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("1.0.0");

                        if lock_path.exists() {
                            // Update existing lock file
                            pm.generate_package_lock(lock_path, project_name, project_version)?;
                        } else {
                            // Generate new lock file
                            pm.generate_package_lock(lock_path, project_name, project_version)?;
                        }
                        println!("✅ Generated package-lock.json");
                    }

                    println!("\n📦 node_modules directory ready!");
                    println!("💡 Run 'amber run <script>' to execute scripts");
                }
                Err(e) => {
                    return Err(anyhow!("Failed to install dependencies: {}", e));
                }
            }

            return Ok(());
        }
        Some(Command::Prune { permissions }) => {
            apply_permission_cli_options(&permissions)?;
            println!("✂️ Pruning unused dependencies from node_modules...");

            // Check if package.json exists
            let package_json_path = std::path::Path::new("package.json");
            if !package_json_path.exists() {
                return Err(anyhow!(
                    "package.json not found in current directory. Run 'amber init' first."
                ));
            }

            // Check if node_modules exists
            let node_modules_path = std::path::Path::new("node_modules");
            check_file_read_permission(node_modules_path)?;
            if !node_modules_path.exists() {
                println!("✅ No node_modules directory found - nothing to prune");
                return Ok(());
            }

            // Create package manager
            let config = amberjs::package_manager::PackageManagerConfig::default();
            let pm = amberjs::package_manager::PackageManager::new(config)
                .map_err(|e| anyhow!("Failed to create package manager: {}", e))?;

            // Parse package.json using PackageManager's method
            let package_json = pm
                .parse_package_json(package_json_path)
                .map_err(|e| anyhow!("Failed to parse package.json: {}", e))?;

            // Prune unused dependencies
            match pm.prune(&package_json) {
                Ok(removed) => {
                    if removed.is_empty() {
                        println!("✅ No unused dependencies found - node_modules is clean");
                    } else {
                        println!("✅ Removed {} unused package(s):", removed.len());
                        for pkg in &removed {
                            println!("  - {}", pkg);
                        }
                    }
                    println!("\n💡 Run 'amber install' to restore dependencies if needed");
                }
                Err(e) => {
                    return Err(anyhow!("Failed to prune dependencies: {}", e));
                }
            }

            return Ok(());
        }
        Some(Command::Create {
            permissions,
            template,
            name,
        }) => {
            apply_permission_cli_options(&permissions)?;
            let (name, template) = normalize_create_args(name, template);
            println!("🎨 Creating new project: {}", name);
            println!("  Template: {}", template);
            let project_dir = std::path::Path::new(&name);
            let index_path = if template == "ts" {
                project_dir.join("index.ts")
            } else {
                project_dir.join("index.js")
            };
            check_file_write_permission(project_dir)?;
            check_file_write_permission(&index_path)?;

            // Create project directory
            std::fs::create_dir_all(project_dir)?;

            match template.as_str() {
                "ts" => {
                    let ts_code = "function greet(name: string): string {\n    return `Hello, ${name}!`;\n}\n\nconsole.log(greet('Amber'));\n";
                    std::fs::write(index_path, ts_code)?;
                    println!("✅ TypeScript project created");
                }
                _ => {
                    let js_code = "console.log('Hello from Amber!');\n";
                    std::fs::write(index_path, js_code)?;
                    println!("✅ JavaScript project created");
                }
            }

            println!(
                "\nRun 'cd {} && amber run index.{}' to start",
                name, template
            );
            return Ok(());
        }
        Some(Command::X {
            permissions,
            package,
            args,
        }) => {
            apply_permission_cli_options(&permissions)?;
            check_process_execute_permission(&package)?;
            check_network_connect_permission("https://registry.npmjs.org")?;
            let exit_code = amberjs::tooling::dlx::run_dlx(&package, &args)?;
            std::process::exit(exit_code);
        }
        Some(Command::Deploy {
            target,
            output,
            port,
            name,
            entry,
        }) => {
            let deploy_target = amberjs::tooling::deploy::DeployTarget::from_str(&target)?;
            amberjs::tooling::deploy::run_deploy(amberjs::tooling::deploy::DeployOptions {
                target: deploy_target,
                output_dir: output,
                port,
                name,
                entry,
            })?;
            return Ok(());
        }
        Some(Command::Upgrade {
            permissions,
            package,
        }) => {
            apply_permission_cli_options(&permissions)?;
            println!("⬆️  Upgrading dependencies...");

            // Check if package.json exists
            let package_json_path = std::path::Path::new("package.json");
            if !package_json_path.exists() {
                return Err(anyhow!("package.json not found in current directory"));
            }

            // Read package.json
            check_file_read_permission(package_json_path)?;
            let content = std::fs::read_to_string(package_json_path)
                .map_err(|e| anyhow!("Failed to read package.json: {}", e))?;

            let mut package_data: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse package.json: {}", e))?;
            check_file_write_permission(package_json_path)?;
            let lock_path = std::path::Path::new("package-lock.json");
            if lock_path.exists() {
                check_file_read_permission(lock_path)?;
            }
            check_file_write_permission(lock_path)?;

            // Create package manager
            let config = amberjs::package_manager::PackageManagerConfig::default();
            let pm = amberjs::package_manager::PackageManager::new(config)
                .map_err(|e| anyhow!("Failed to create package manager: {}", e))?;

            // Determine which dependencies to upgrade
            let dep_types = vec!["dependencies", "devDependencies"];
            let mut upgraded = Vec::new();
            let mut errors = Vec::new();

            for dep_type in dep_types {
                if let Some(deps) = package_data.get_mut(dep_type) {
                    if let Some(deps_obj) = deps.as_object_mut() {
                        let packages: Vec<(String, String)> = deps_obj
                            .iter()
                            .filter(|(name, _)| {
                                package.as_ref().map(|p| p == *name).unwrap_or(true)
                            })
                            .map(|(name, v)| {
                                (name.clone(), v.as_str().unwrap_or("latest").to_string())
                            })
                            .collect();

                        for (pkg_name, _current_version) in packages {
                            print!("  Checking {}...", pkg_name);
                            std::io::stdout().flush()?;

                            // Fetch latest version from registry
                            match pm.fetch_package_info(&pkg_name) {
                                Ok(info) => {
                                    // Get latest version from dist-tags
                                    let latest_version = info
                                        .get("dist-tags")
                                        .and_then(|tags| tags.get("latest"))
                                        .and_then(|v| v.as_str())
                                        .ok_or(anyhow!("No latest version found"))?
                                        .to_string();
                                    let current_version = deps_obj
                                        .get(&pkg_name)
                                        .and_then(|v| v.as_str())
                                        .map(|v| {
                                            v.trim_start_matches('^')
                                                .trim_start_matches('~')
                                                .to_string()
                                        })
                                        .unwrap_or_else(|| "unknown".to_string());

                                    if current_version != latest_version {
                                        // Reinstall with latest version
                                        match pm.install_package(&pkg_name, &latest_version) {
                                            Ok(result) => {
                                                // Update package.json
                                                let new_version_str =
                                                    format!("^{}", result.package.version);
                                                deps_obj.insert(
                                                    pkg_name.clone(),
                                                    serde_json::Value::String(new_version_str),
                                                );
                                                println!(
                                                    " {} → {}",
                                                    current_version, result.package.version
                                                );
                                                upgraded.push((
                                                    pkg_name,
                                                    current_version,
                                                    result.package.version,
                                                ));
                                            }
                                            Err(e) => {
                                                println!(" failed");
                                                errors.push(format!("{}: {}", pkg_name, e));
                                            }
                                        }
                                    } else {
                                        println!(" up to date ({})", current_version);
                                    }
                                }
                                Err(e) => {
                                    if e.to_string().contains("permission denied") {
                                        return Err(anyhow!(
                                            "Failed to fetch package info for {}: {}",
                                            pkg_name,
                                            e
                                        ));
                                    }
                                    println!(" failed to fetch info");
                                    errors.push(format!("{}: {}", pkg_name, e));
                                }
                            }
                        }
                    }
                }
            }

            // Write updated package.json
            let updated_content = serde_json::to_string_pretty(&package_data)
                .map_err(|e| anyhow!("Failed to serialize package.json: {}", e))?;
            check_file_write_permission(package_json_path)?;
            std::fs::write(package_json_path, updated_content)
                .map_err(|e| anyhow!("Failed to write package.json: {}", e))?;

            // Generate new package-lock.json
            if let Some(project_name) = package_data.get("name").and_then(|n| n.as_str()) {
                let project_version = package_data
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("1.0.0");
                pm.generate_package_lock(lock_path, project_name, project_version)?;
            }

            println!("\n✅ Upgrade complete!");
            if !upgraded.is_empty() {
                println!("  Upgraded packages:");
                for (name, old_ver, new_ver) in &upgraded {
                    println!("    {}: {} → {}", name, old_ver, new_ver);
                }
            }
            if !errors.is_empty() {
                println!("  Errors:");
                for error in &errors {
                    println!("    - {}", error);
                }
            }

            return Ok(());
        }
        Some(Command::Session {
            permissions,
            file,
            isolate_per_call,
        }) => {
            apply_permission_cli_options(&permissions)?;
            allow_sandbox_entry_file(permissions.sandbox, &file)?;
            check_file_read_permission(&file)?;
            amberjs::agent::run_jsonrpc_session(
                file,
                isolate_per_call,
                io::BufReader::new(io::stdin()),
                io::stdout(),
            )?;
            return Ok(());
        }
        Some(Command::Mcp {
            permissions,
            file,
            isolate_per_call,
            inspect,
        }) => {
            apply_permission_cli_options(&permissions)?;

            if let Some(target_file) = file {
                allow_sandbox_entry_file(permissions.sandbox, &target_file)?;
                check_file_read_permission(&target_file)?;

                if inspect {
                    println!("🔍 Inspecting MCP Tool Module: {}\n", target_file.display());
                    let tools = amberjs::agent::export_tools_from_entry(&target_file)?;
                    println!("📡 Protocol Version: 2024-11-05");
                    println!("🛠  Discovered {} tool(s):\n", tools.len());
                    let headers = vec![
                        "Tool Name".to_string(),
                        "Description".to_string(),
                        "Input Schema".to_string(),
                    ];
                    let mut rows = Vec::new();
                    for t in tools {
                        let schema_str = serde_json::to_string(&t.input_schema).unwrap_or_default();
                        rows.push(vec![t.name, t.description, schema_str]);
                    }
                    println!("{}", amberjs::std_lib::cli::format_table(&headers, &rows));
                    return Ok(());
                }

                amberjs::agent::run_mcp_server(
                    target_file,
                    isolate_per_call,
                    io::stdin(),
                    io::stdout(),
                )?;
                return Ok(());
            } else if inspect {
                println!("🔍 Inspecting MCP Built-in Tools\n");
                let tools = vec![
                    amberjs::agent::ToolSchema {
                        name: "execute_command".to_string(),
                        description: "Execute a sandboxed shell command".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": { "cmd": { "type": "string" }, "args": { "type": "array" } },
                            "required": ["cmd"]
                        }),
                    },
                    amberjs::agent::ToolSchema {
                        name: "read_file".to_string(),
                        description: "Read file contents from filesystem".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": { "path": { "type": "string" } },
                            "required": ["path"]
                        }),
                    },
                    amberjs::agent::ToolSchema {
                        name: "write_file".to_string(),
                        description: "Write file contents to virtual or host filesystem"
                            .to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": { "path": { "type": "string" }, "data": { "type": "string" } },
                            "required": ["path", "data"]
                        }),
                    },
                ];
                println!("📡 Protocol Version: 2024-11-05");
                println!("🛠  Discovered {} tool(s):\n", tools.len());
                let headers = vec![
                    "Tool Name".to_string(),
                    "Description".to_string(),
                    "Input Schema".to_string(),
                ];
                let mut rows = Vec::new();
                for t in tools {
                    let schema_str = serde_json::to_string(&t.input_schema).unwrap_or_default();
                    rows.push(vec![t.name, t.description, schema_str]);
                }
                println!("{}", amberjs::std_lib::cli::format_table(&headers, &rows));
                return Ok(());
            } else {
                return Err(anyhow!(
                    "Please specify a tool module file or pass --inspect"
                ));
            }
        }
        Some(Command::Fmt { files, check }) => {
            let summary = amberjs::tooling::formatter::format_paths(&files, check)?;
            if check {
                if !summary.unformatted_files.is_empty() {
                    eprintln!(
                        "❌ {} file(s) would be formatted",
                        summary.unformatted_files.len()
                    );
                    std::process::exit(1);
                } else {
                    println!(
                        "✅ All {} scanned file(s) are properly formatted",
                        summary.total_scanned
                    );
                }
            } else {
                println!(
                    "✅ Formatted {} of {} scanned file(s)",
                    summary.formatted, summary.total_scanned
                );
            }
            return Ok(());
        }
        Some(Command::Lint { files }) => {
            let summary = amberjs::tooling::linter::lint_paths(&files)?;
            if summary.total_problems > 0 {
                eprintln!(
                    "\n❌ Found {} problem(s) ({} error(s), {} warning(s)) across {} file(s)",
                    summary.total_problems, summary.errors, summary.warnings, summary.total_scanned
                );
                if summary.errors > 0 {
                    std::process::exit(1);
                }
            } else {
                println!(
                    "✅ No lint problems found in {} file(s)",
                    summary.total_scanned
                );
            }
            return Ok(());
        }
        Some(Command::Bench { files }) => {
            let bench_files = amberjs::tooling::benchmark::discover_benchmark_files(&files);
            if bench_files.is_empty() {
                println!("ℹ️  No benchmark files (*.bench.js, *.bench.ts) found");
                return Ok(());
            }
            for bf in bench_files {
                let results = amberjs::tooling::benchmark::run_benchmark_file(&bf)?;
                amberjs::tooling::benchmark::print_benchmark_table(&bf.to_string_lossy(), &results);
            }
            return Ok(());
        }
        Some(Command::Compile { entry, output }) => {
            let out = output.unwrap_or_else(|| {
                let stem = entry
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                #[cfg(windows)]
                let name = format!("{}.exe", stem);
                #[cfg(not(windows))]
                let name = stem.to_string();
                PathBuf::from(name)
            });
            amberjs::tooling::compiler::compile_binary(&entry, &out)?;
            return Ok(());
        }
        Some(Command::Types { outfile }) => {
            amberjs::types_export::export_types(outfile.as_deref())?;
            return Ok(());
        }
        Some(Command::Task { name, args }) => {
            match name {
                Some(task_name) => {
                    let status =
                        amberjs::task_runner::run_script(Path::new("."), &task_name, &args)?;
                    if !status.success() {
                        std::process::exit(status.code().unwrap_or(1));
                    }
                }
                None => {
                    let pkg_path = amberjs::task_runner::find_package_json(Path::new("."))
                        .ok_or_else(|| {
                            anyhow!("No package.json found in current or parent directories")
                        })?;
                    let scripts = amberjs::task_runner::load_scripts(&pkg_path)?;
                    println!("📋 Available scripts in {}:", pkg_path.display());
                    println!("{:-<60}", "");
                    for (k, v) in scripts {
                        println!("  {:<20} {}", k, v);
                    }
                    println!("{:-<60}", "");
                }
            }
            return Ok(());
        }
        Some(Command::Profile { file, output }) => {
            let _ = amberjs::tooling::profiler::profile_script(&file, output.as_deref())?;
            return Ok(());
        }
        Some(Command::Lsp) => {
            amberjs::tooling::lsp::run_lsp_server(std::io::stdin(), std::io::stdout())?;
            return Ok(());
        }
        None => {
            // No command provided, show help
            println!("🐝 Amber - High-performance JavaScript/TypeScript runtime");
            println!();
            println!("Usage: amber [COMMAND]");
            println!();
            println!("Commands:");
            println!("  run <file>       Run a JavaScript/TypeScript file");
            println!("  task [name]      Run a script task defined in package.json");
            println!("  fmt [files]      Format JavaScript/TypeScript files");
            println!("  lint [files]     Lint JavaScript/TypeScript files");
            println!("  lsp              Start Language Server Protocol (LSP) server");
            println!("  test [file]      Run tests (with --coverage support)");
            println!("  bench [files]    Run microbenchmarks");
            println!("  compile <entry>  Compile into standalone single binary");
            println!("  types [-o file]  Export TypeScript type definitions");
            println!("  profile <file>   Profile execution and export .cpuprofile");
            println!("  session <file>   JSON-RPC tool session over stdin");
            println!("  mcp <file>       MCP stdio server for the tool file");
            println!("  snapshot <act>   Manage V8 startup snapshot (build, status, clean)");
            println!("  eval <code>      Evaluate JavaScript code");
            println!("  repl             Start interactive REPL");
            println!("  test [file]      Run tests (built-in or from file)");
            println!("  bundle <file>    Bundle a local JS/TS module graph into one JS file");
            println!("  debug <file>     Debug a script with detailed output");
            println!("  serve [options]  HTTP/HTTPS fetch-handler server");
            println!("  init [name]      Initialize new project");
            println!("  add <package>    Add dependency package");
            println!("  remove <package> Remove dependency package");
            println!("  create <name> [template] Create new project");
            println!("  bunx <package>   Run a package without installing");
            println!("  upgrade [pkg]    Upgrade dependencies to latest");
            println!("  version          Display version information");
            println!();
            println!("Examples:");
            println!("  amber run script.js");
            println!("  amber run --sandbox --allow-read ./workspace tool.ts");
            println!("  amber eval 'console.log(\"Hello\")'");
            println!("  amber repl");
            println!("  amber test");
            println!("  amber bundle entry.ts --output bundle.js");
            println!("  amber debug script.ts");
            println!("  amber serve --port 8080");
            println!("  amber init my-project");
            println!("  amber add react --save-exact");
            println!("  amber add typescript --dev");
            println!("  amber upgrade");
            println!("  amber upgrade lodash");
            return Ok(());
        }
    }
}

const HTTPS_FETCH_BRIDGE: &str = r#"
globalThis.__amberjs_app__ = undefined;
globalThis.__amberjs_handle_http__ = async function(method, url, headersJson, bodyStr) {
    try {
        const headers = JSON.parse(headersJson);
        const reqInit = { method, headers };
        if (method !== "GET" && method !== "HEAD" && bodyStr && bodyStr.length > 0) {
            reqInit.body = bodyStr;
        }
        const req = new Request(url, reqInit);
        let handler = globalThis.__amberjs_app__;
        if (handler && typeof handler.default === 'object' && typeof handler.default.fetch === 'function') {
            handler = handler.default.fetch.bind(handler.default);
        } else if (handler && typeof handler.default === 'function') {
            handler = handler.default;
        } else if (handler && typeof handler.fetch === 'function') {
            handler = handler.fetch;
        } else if (typeof globalThis.fetchHandler === 'function') {
            handler = globalThis.fetchHandler;
        }
        if (typeof handler !== 'function') {
            return JSON.stringify({ status: 404, headers: { "content-type": "text/plain" }, body: "Not Found: No fetch handler exported" });
        }
        const res = await handler(req);
        const status = (res && res.status) ? res.status : 200;
        const resHeaders = {};
        if (res && res.headers && typeof res.headers.forEach === 'function') {
            res.headers.forEach((v, k) => { resHeaders[k] = v; });
        }
        let bodyText = "";
        if (res) {
            if (typeof res._bodyText === 'string') {
                bodyText = res._bodyText;
            } else if (typeof res.text === 'function') {
                try { bodyText = await res.text(); } catch (_) { bodyText = res.body ? String(res.body) : ""; }
            } else {
                bodyText = res.body ? String(res.body) : "";
            }
        }
        return JSON.stringify({ status, headers: resHeaders, body: bodyText });
    } catch (e) {
        return JSON.stringify({ status: 500, headers: { "content-type": "text/plain" }, body: "Internal Server Error: " + (e ? e.message : e) });
    }
};
"#;

fn serve_https_health_loop(
    listener: std::net::TcpListener,
    tls_config: std::sync::Arc<rustls::ServerConfig>,
) -> Result<()> {
    let body = format!(
        "{{\"runtime\":\"amberjs\",\"ok\":true,\"version\":\"{}\"}}\n",
        env!("CARGO_PKG_VERSION")
    );
    for tcp in listener.incoming() {
        let Ok(tcp) = tcp else { continue };
        let Ok(conn) = rustls::ServerConnection::new(tls_config.clone()) else {
            continue;
        };
        let mut stream = rustls::StreamOwned::new(conn, tcp);
        let _ = read_http11_request(&mut stream);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
    }
    Ok(())
}

fn serve_https_fetch_loop(
    listener: std::net::TcpListener,
    tls_config: std::sync::Arc<rustls::ServerConfig>,
    runtime: &mut amberjs::runtime_minimal::MinimalRuntime,
    bound: &std::net::SocketAddr,
) -> Result<()> {
    for tcp in listener.incoming() {
        let Ok(tcp) = tcp else { continue };
        let Ok(conn) = rustls::ServerConnection::new(tls_config.clone()) else {
            continue;
        };
        let mut stream = rustls::StreamOwned::new(conn, tcp);
        let Some((method, path, headers, body_str)) = read_http11_request(&mut stream) else {
            continue;
        };
        let url = format!("https://{}{}", bound, path);
        let headers_json = serde_json::to_string(&headers).unwrap_or_else(|_| "{}".to_string());
        let dispatch_script = format!(
            r#"globalThis.__amberjs_handle_http__({}, {}, {}, {});"#,
            serde_json::to_string(&method).unwrap(),
            serde_json::to_string(&url).unwrap(),
            serde_json::to_string(&headers_json).unwrap(),
            serde_json::to_string(&body_str).unwrap(),
        );
        let resp_json = match runtime.execute_code(&dispatch_script) {
            Ok(raw_json) => raw_json,
            Err(e) => format!(
                r#"{{"status":500,"headers":{{"content-type":"text/plain"}},"body":"Handler error: {}"}}"#,
                e
            ),
        };
        let val: serde_json::Value = serde_json::from_str(resp_json.trim()).unwrap_or_default();
        let status_code = val.get("status").and_then(|s| s.as_u64()).unwrap_or(200);
        let body_text = val.get("body").and_then(|b| b.as_str()).unwrap_or("");
        let mut header_lines = String::from("Content-Type: text/plain\r\n");
        if let Some(headers_obj) = val.get("headers").and_then(|h| h.as_object()) {
            header_lines.clear();
            for (k, v) in headers_obj {
                if let Some(v_str) = v.as_str() {
                    header_lines.push_str(&format!("{k}: {v_str}\r\n"));
                }
            }
        }
        let resp = format!(
            "HTTP/1.1 {status_code} OK\r\n{header_lines}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            body_text.len(),
            body_text
        );
        let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
    }
    Ok(())
}

fn read_http11_request<S: std::io::Read>(
    stream: &mut S,
) -> Option<(
    String,
    String,
    std::collections::HashMap<String, String>,
    String,
)> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 1024 * 1024 {
            break;
        }
    }
    let req = amberjs::nodejs_core::http::parse_http_request(&buf)?;
    Some((
        req.method,
        req.url,
        req.headers,
        String::from_utf8_lossy(&req.body).into_owned(),
    ))
}
