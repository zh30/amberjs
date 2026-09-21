// Script Executor Module
// Stage 56.2 - Script Execution Engine
//
// Provides enhanced script execution capabilities including:
// - File type detection (JS, TS, MJS, CJS, JSON)
// - Execution context (__dirname, __filename, process.argv)
// - Module resolution (ES Modules and CommonJS)
// - Environment variable handling
/// - Shebang detection
use std::path::PathBuf;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::Path;
/// File type enumeration for script detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Standard JavaScript (.js)
    JavaScript,
    /// ES Module JavaScript (.mjs)
    EsModule,
    /// CommonJS JavaScript (.cjs)
    CommonJs,
    /// TypeScript (.ts, .tsx, .mts, .cts)
    TypeScript,
    /// JSON (.json)
    Json,
    /// Unknown file type
    Unknown,
}
impl FileType {
    /// Check if this file type requires transpilation
    pub fn needs_transpilation(&self) -> bool {
        matches!(self, FileType::TypeScript)
    }
    /// Check if this file type is executable
    pub fn is_executable(&self) -> bool {
        matches!(
            self,
            FileType::JavaScript
                | FileType::EsModule
                | FileType::CommonJs
                | FileType::TypeScript
        )
    }
    /// Get the module system for this file type
    pub fn module_system(&self) -> ModuleSystem {
        match self {
            FileType::EsModule => ModuleSystem::ESModule,
            FileType::CommonJs => ModuleSystem::CommonJS,
            FileType::JavaScript => ModuleSystem::Auto, // Detect from package.json or content
            FileType::TypeScript => ModuleSystem::Auto,
            _ => ModuleSystem::None,
        }
    }
}
/// Module system type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleSystem {
    /// ES Modules (import/export)
    ESModule,
    /// CommonJS (require/module.exports)
    CommonJS,
    /// Auto-detect based on package.json or content
    Auto,
    /// Not a module (JSON, etc.)
    None,
}
/// Detect file type from path extension
pub fn detect_file_type(path: &PathBuf) -> FileType {
    let extension: _ = path.extension().and_then(|ext| ext.to_str());
    match extension {
        Some("js") => FileType::JavaScript,
        Some("mjs") => FileType::EsModule,
        Some("cjs") => FileType::CommonJs,
        Some("ts") | Some("tsx") | Some("mts") | Some("cts") => FileType::TypeScript,
        Some("json") => FileType::Json,
        _ => FileType::Unknown,
    }
}
/// Execution context for scripts
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Current working directory
    pub cwd: PathBuf,
    /// Absolute path to the script file
    pub script_path: PathBuf,
    /// __dirname equivalent - directory containing the script
    pub dirname: PathBuf,
    /// __filename equivalent - full path to the script
    pub filename: PathBuf,
    /// process.argv array
    pub argv: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// File type of the script
    pub file_type: FileType,
    /// Module system to use
    pub module_system: ModuleSystem,
}
impl ExecutionContext {
    /// Create a new execution context for a script
    pub fn new(script_path: PathBuf) -> Self {
        let cwd: _ = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Resolve to absolute path
        let absolute_path: _ = if script_path.is_absolute() {
            script_path.clone()
        } else {
            cwd.join(&script_path)
        };
        // Get directory containing the script
        let dirname: _ = absolute_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        // Detect file type
        let file_type: _ = detect_file_type(&absolute_path);
        let module_system: _ = file_type.module_system();
        // Build initial argv
        let argv: _ = vec![
            "amber".to_string(),
            script_path.to_string_lossy().to_string(),
        ];
        // Collect environment variables
        let env: HashMap<String, String> = std::env::vars().collect();
        Self {
            cwd,
            script_path: absolute_path.clone(),
            dirname,
            filename: absolute_path,
            argv,
            env,
            file_type,
            module_system,
        }
    }
    /// Add script arguments to process.argv
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.argv.extend(args);
        self
    }
    /// Add or override an environment variable
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }
    /// Set the module system explicitly
    pub fn with_module_system(mut self, system: ModuleSystem) -> Self {
        self.module_system = system;
        self
    }
    /// Generate JavaScript code to set up the execution context globals
    pub fn to_setup_code(&self) -> String {
        let dirname_escaped: _ = self.dirname.to_string_lossy().replace('\\', "\\\\");
        let filename_escaped: _ = self.filename.to_string_lossy().replace('\\', "\\\\");
        let cwd_escaped: _ = self.cwd.to_string_lossy().replace('\\', "\\\\");
        // Build argv JSON array
        let argv_json: Vec<String> = self
            .argv
            .iter()
            .map(|a| format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();
        format!(
            r#"
// Amber execution context setup
globalThis.__dirname = "{}";
globalThis.__filename = "{}";
// Ensure process object exists
if (typeof globalThis.process === 'undefined') {{
    globalThis.process = {{}};
}}
// Set process.cwd()
globalThis.process.cwd = function() {{ return "{}"; }};
// Set process.argv
globalThis.process.argv = [{}];
// Note: process.env is set up separately via native binding
"#,
            dirname_escaped,
            filename_escaped,
            cwd_escaped,
            argv_json.join(", ")
        )
    }
}
/// Script executor configuration
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Enable TypeScript transpilation
    pub transpile_ts: bool,
    /// Enable hot reload / watch mode
    pub hot_reload: bool,
    /// Enable source maps for debugging
    pub source_maps: bool,
    /// Verbose output
    pub verbose: bool,
}
impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            transpile_ts: true,
            hot_reload: false,
            source_maps: true,
            verbose: false,
        }
    }
}
/// Script executor for running JS/TS files
pub struct ScriptExecutor {
    /// Executor configuration
    config: ExecutorConfig,
}
impl ScriptExecutor {
    /// Create a new script executor with the given configuration
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }
    /// Validate that a script file can be executed
    pub fn validate_script(&self, path: &PathBuf) -> Result<FileType, String> {
        // Check file exists
        if !path.exists() {
            return Err(format!("Script not found: {}", path.display()));
        }
        // Detect file type
        let file_type: _ = detect_file_type(path);
        match file_type {
            FileType::JavaScript | FileType::EsModule | FileType::CommonJs => Ok(file_type),
            FileType::TypeScript => {
                if self.config.transpile_ts {
                    Ok(file_type)
                } else {
                    Err("TypeScript transpilation is disabled".to_string())
                }
            }
            FileType::Json => Ok(file_type), // JSON can be imported/required
            FileType::Unknown => Err(format!(
                "Unknown file type: {}. Supported: .js, .mjs, .cjs, .ts, .tsx, .json",
                path.display())),
        }
    }
    /// Check if a file needs transpilation before execution
    pub fn needs_transpilation(&self, path: &PathBuf) -> bool {
        let file_type: _ = detect_file_type(path);
        file_type.needs_transpilation()
    }
    /// Build the execution context for a script
    pub fn build_context(&self, path: PathBuf, args: Vec<String>) -> ExecutionContext {
        ExecutionContext::new(path).with_args(args)
    }
    /// Get the executor configuration
    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }
}
/// Shebang detection utilities
pub mod shebang {
    /// Detect shebang line from file content
    pub fn detect(content: &str) -> Option<String> {
        let first_line: _ = content.lines().next()?;
        if first_line.starts_with("#!") {
            Some(first_line[2..].trim().to_string())
        } else {
            None
        }
    }
    /// Check if a shebang indicates a Amber-compatible script
    pub fn is_compatible(shebang: &str) -> bool {
        shebang.contains("amber")
            || shebang.ends_with("/env amber")
            || shebang.ends_with("/env node") // Node.js compatibility
            || shebang.ends_with("/env bun")  // Bun compatibility
            || shebang.ends_with("/node")
            || shebang.ends_with("/bun")
    }
    /// Strip shebang from script content for execution
    pub fn strip(content: &str) -> &str {
        if content.starts_with("#!") {
            // Find the end of the first line
            content.find('\n').map(|i| &content[i + 1..]).unwrap_or("")
        } else {
            content
        }
    }
}
/// Argument parsing utilities for CLI
pub mod args {
    /// Options that take a value argument
    const OPTIONS_WITH_VALUES: &[&str] = &[
        "--config",
        "-c",
        "--env",
        "-e",
        "--loader",
        "--outdir",
        "--outfile",
        "--target",
        "--tsconfig",
        "--define",
        "--external",
    ];
    /// Parsed command line arguments
    #[derive(Debug, Clone)]
    pub struct ParsedArgs {
        /// Arguments for the runtime (before script path)
        pub runtime_args: Vec<String>,
        /// Arguments for the script (after script path or --)
        pub script_args: Vec<String>,
        /// Path to the script file
        pub script_path: Option<String>,
    }
    impl ParsedArgs {
        /// Parse command line arguments with -- separator support
        pub fn parse(args: Vec<String>) -> Self {
            let mut runtime_args = Vec::new();
            let mut script_args = Vec::new();
            let mut script_path = None;
            let mut found_separator = false;
            let mut found_script = false;
            let mut expect_value = false;
            for arg in args.into_iter().skip(1) {
                if arg == "--" {
                    found_separator = true;
                    continue;
                }
                if found_separator {
                    script_args.push(arg);
                } else if expect_value {
                    runtime_args.push(arg);
                    expect_value = false;
                } else if !found_script && !arg.starts_with('-') {
                    script_path = Some(arg);
                    found_script = true;
                } else if found_script {
                    script_args.push(arg);
                } else {
                    runtime_args.push(arg.clone());
                    if OPTIONS_WITH_VALUES.contains(&arg.as_str()) {
                        expect_value = true;
                    }
                }
            }
            Self {
                runtime_args,
                script_args,
                script_path,
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_file_type_detection() {
        assert_eq!(detect_file_type(&PathBuf::from("test.js")), FileType::JavaScript);
        assert_eq!(detect_file_type(&PathBuf::from("test.mjs")), FileType::EsModule);
        assert_eq!(detect_file_type(&PathBuf::from("test.cjs")), FileType::CommonJs);
        assert_eq!(detect_file_type(&PathBuf::from("test.ts")), FileType::TypeScript);
        assert_eq!(detect_file_type(&PathBuf::from("test.tsx")), FileType::TypeScript);
        assert_eq!(detect_file_type(&PathBuf::from("test.json")), FileType::Json);
        assert_eq!(detect_file_type(&PathBuf::from("test.txt")), FileType::Unknown);
    }
    #[test]
    fn test_file_type_needs_transpilation() {
        assert!(!FileType::JavaScript.needs_transpilation());
        assert!(!FileType::EsModule.needs_transpilation());
        assert!(FileType::TypeScript.needs_transpilation());
    }
    #[test]
    fn test_execution_context_creation() {
        let ctx: _ = ExecutionContext::new(PathBuf::from("test.js"));
        assert!(ctx.argv.len() >= 2);
        assert_eq!(ctx.argv[0], "amber");
    }
    #[test]
    fn test_execution_context_with_args() {
        let ctx: _ = ExecutionContext::new(PathBuf::from("test.js"))
            .with_args(vec!["--port".to_string(), "3000".to_string()]);
        assert_eq!(ctx.argv.len(), 4);
    }
    #[test]
    fn test_shebang_detection() {
        assert_eq!(
            shebang::detect("#!/usr/bin/env amber\nconsole.log('hi')"),
            Some("/usr/bin/env amber".to_string()));
        assert!(shebang::detect("console.log('hi')").is_none());
    }
    #[test]
    fn test_shebang_strip() {
        let content: _ = "#!/usr/bin/env amber\nconsole.log('hi')";
        assert_eq!(shebang::strip(content), "console.log('hi')");
    }
    #[test]
    fn test_shebang_compatibility() {
        assert!(shebang::is_compatible("/usr/bin/env amber"));
        assert!(shebang::is_compatible("/usr/bin/env node"));
        assert!(shebang::is_compatible("/usr/bin/env bun"));
        assert!(!shebang::is_compatible("/usr/bin/python"));
    }
}
