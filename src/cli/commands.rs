// CLI Commands Module
// Stage 56.1 - CLI Core Architecture
// Stage 91 Phase 4.1 - 开发者体验提升
//
// Implements a proper subcommand-based CLI structure similar to Bun

use clap::{Parser, Subcommand};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// Main CLI application
#[derive(Parser, Debug)]
#[command(
    name = "amber",
    about = "High-performance JavaScript/TypeScript runtime (faster than Bun)",
    version = "0.1.1",
    author = "Henry Zhang"
)]
pub struct CliApp {
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// Environment variables file (.env)
    #[arg(short = 'e', long)]
    pub env: Option<PathBuf>,
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<SubCommand>,
}
/// Subcommand enumeration
#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Run a script file
    Run(RunCommand),
    /// Run tests
    Test(TestCommand),
    /// Start REPL (interactive shell)
    Repl(ReplCommand),
    /// Bundle code for production
    Bundle(BundleCommand),
    /// Profile script performance
    Profile(ProfileCommand),
    /// Debug a script with interactive debugger
    Debug {
        /// Script file to debug
        file: Option<PathBuf>,
        /// Break at line number on startup
        #[arg(short, long)]
        break_at: Option<u32>,
        /// Debug server port
        #[arg(short, long, default_value = "9229")]
        port: u16,
        /// Enable Web UI debugger
        #[arg(short, long)]
        web: bool,
        /// Attach to a running process
        #[arg(short, long)]
        pid: Option<u32>,
    },
    /// WebAssembly module operations
    #[command(alias = "wasm")]
    Wasm {
        #[command(subcommand)]
        command: super::wasm_commands::WasmSubCommand,
    },
    /// Initialize a new project
    Init(InitCommand),
    /// Display system and runtime information
    Info(InfoCommandArgs),
    /// Diagnose environment problems
    Doctor(DoctorCommandArgs),
    /// Upgrade Amber to latest version
    Upgrade(UpgradeCommand),
    /// Version information
    Version,
}
/// Run command - execute scripts
#[derive(Parser, Debug)]
pub struct RunCommand {
    /// Script file to execute
    pub script: PathBuf,
    /// Arguments to pass to the script
    pub args: Vec<String>,
    /// Watch mode - reload on file changes
    #[arg(short, long)]
    pub watch: bool,
    /// Transpile TypeScript (default: auto-detect)
    #[arg(short, long)]
    pub transpile: bool,
    /// Hot reload (alias for --watch)
    #[arg(short = 'H', long)]
    pub hot: bool,
}
/// Test command - run test suite
#[derive(Parser, Debug)]
pub struct TestCommand {
    /// Test file or directory (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Test pattern (glob pattern)
    #[arg(short, long, default_value = "**/*.{test,spec}.{js,ts}")]
    pub pattern: String,
    /// Reporter type (default: spec)
    #[arg(short = 'r', long, value_enum, default_value = "spec")]
    pub reporter: TestReporter,
    /// Run tests in watch mode
    #[arg(short, long)]
    pub watch: bool,
    /// Run tests once and exit
    #[arg(short = 'o', long)]
    pub run_once: bool,
    /// Enable coverage reporting
    #[arg(short, long)]
    pub coverage: bool,
    /// Maximum test timeout in seconds
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,
}
/// REPL command - interactive shell
#[derive(Parser, Debug)]
pub struct ReplCommand {
    /// Load a file into the REPL session
    #[arg(short, long)]
    pub load: Option<PathBuf>,
    /// Evaluate expression on startup
    #[arg(short, long)]
    pub eval: Option<String>,
    /// Save REPL session to file on exit
    #[arg(short, long)]
    pub save: Option<PathBuf>,
    /// Enable TypeScript mode
    #[arg(short, long)]
    pub typescript: bool,
}
/// Bundle command - create production bundles
#[derive(Parser, Debug)]
pub struct BundleCommand {
    /// Entry file
    pub entry: PathBuf,
    /// Output file
    #[arg(short, long)]
    pub outfile: Option<PathBuf>,
    /// Minify output
    #[arg(short, long)]
    pub minify: bool,
    /// Enable source maps
    #[arg(short, long)]
    pub sourcemap: bool,
    /// Target environment (default: browser)
    #[arg(short = 't', long, value_enum, default_value = "browser")]
    pub target: BundleTarget,
    /// External dependencies (comma-separated)
    #[arg(short, long)]
    pub external: Option<Vec<String>>,
    /// Enable tree shaking
    #[arg(short = 'T', long)]
    pub tree_shake: bool,
}
/// Test reporter type
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestReporter {
    /// Spec reporter (default)
    Spec,
    /// JSON reporter
    Json,
    /// Dot reporter
    Dot,
    /// Tap reporter
    Tap,
}
/// Bundle target environment
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleTarget {
    /// Browser environment
    Browser,
    /// Node.js environment
    Node,
    /// Bun environment
    Bun,
    /// Neutral (no runtime-specific code)
    Neutral,
}
impl Default for TestReporter {
    fn default() -> Self {
        TestReporter::Spec
    }
}
impl Default for BundleTarget {
    fn default() -> Self {
        BundleTarget::Browser
    }
}
/// Profile command - performance profiling
#[derive(Parser, Debug)]
pub struct ProfileCommand {
    /// Script file to profile
    pub script: PathBuf,
    /// Arguments to pass to the script
    pub args: Vec<String>,
    /// Enable detailed profiling
    #[arg(short = 'v', long)]
    pub detailed: bool,
    /// Run in interactive mode
    #[arg(short, long)]
    pub interactive: bool,
    /// Output format (text, json, html)
    #[arg(long = "format", default_value = "text")]
    pub output_format: String,
    /// Output directory for reports
    #[arg(short = 'd', long = "dir")]
    pub output_dir: Option<PathBuf>,
    /// Profiling duration in seconds
    #[arg(short = 't', long, default_value = "10")]
    pub duration: u64,
    /// Sampling rate (events per second)
    #[arg(short = 'r', long, default_value = "100")]
    pub sampling_rate: u32,
}
// ========== Stage 91 Phase 4.1: 新增命令 ==========
/// Init command - initialize new project
#[derive(Parser, Debug)]
pub struct InitCommand {
    /// Project directory (default: current directory)
    #[arg(default_value = ".")]
    pub dir: PathBuf,
    /// Project template to use
    #[arg(short, long, value_enum, default_value = "basic")]
    pub template: ProjectTemplateArg,
    /// Skip git initialization
    #[arg(long)]
    pub no_git: bool,
    /// Run in interactive mode
    #[arg(short, long)]
    pub interactive: bool,
    /// Project name (default: directory name)
    #[arg(short, long)]
    pub name: Option<String>,
}
/// Project template types
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplateArg {
    /// Basic JavaScript project
    Basic,
    /// TypeScript project
    Typescript,
    /// Web API server
    WebApi,
    /// CLI tool
    CliTool,
}
impl Default for ProjectTemplateArg {
    fn default() -> Self {
        ProjectTemplateArg::Basic
    }
}
/// Info command arguments
#[derive(Parser, Debug)]
pub struct InfoCommandArgs {
    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: String,
}
/// Doctor command arguments
#[derive(Parser, Debug)]
pub struct DoctorCommandArgs {
    /// Run all checks including optional ones
    #[arg(short, long)]
    pub full: bool,
    /// Fix issues automatically when possible
    #[arg(long)]
    pub fix: bool,
}
/// Upgrade command - upgrade Amber
#[derive(Parser, Debug)]
pub struct UpgradeCommand {
    /// Target version (default: latest)
    #[arg(short, long)]
    pub version: Option<String>,
    /// Force upgrade even if already up to date
    #[arg(short, long)]
    pub force: bool,
    /// Check for updates without installing
    #[arg(long)]
    pub check: bool,
}
