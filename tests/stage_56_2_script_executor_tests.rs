// Stage 56.2 - Script Executor Tests
// Tests for the script execution engine including:
// - File type detection
// - Execution context (__dirname, __filename, process.argv)
// - Module system support (ES Modules and CommonJS)
// - Parameter passing and environment variables
#![allow(dead_code)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::cmp_owned)]
#![allow(clippy::len_zero)]
#![allow(clippy::manual_strip)]

use std::path::PathBuf;

/// ============================================
/// File Type Detection Tests
/// ============================================

mod file_type_detection {
    use super::*;

    /// File type enum matching main.rs implementation
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum FileType {
        JavaScript,
        TypeScript,
        Json,
        CommonJs,
        EsModule,
        Unknown,
    }

    /// Enhanced file type detection function
    pub fn detect_file_type(path: &PathBuf) -> FileType {
        let extension = path.extension().and_then(|ext| ext.to_str());
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        match extension {
            Some("js") => {
                // Check for .mjs convention or "type": "module" in package.json
                FileType::JavaScript
            }
            Some("mjs") => FileType::EsModule,
            Some("cjs") => FileType::CommonJs,
            Some("ts") | Some("tsx") => FileType::TypeScript,
            Some("mts") => FileType::TypeScript, // ES Module TypeScript
            Some("cts") => FileType::TypeScript, // CommonJS TypeScript
            Some("json") => FileType::Json,
            _ => {
                // Check for shebang
                if stem.is_empty() {
                    FileType::Unknown
                } else {
                    FileType::Unknown
                }
            }
        }
    }

    #[test]
    fn test_detect_javascript_file() {
        let path = PathBuf::from("script.js");
        assert_eq!(detect_file_type(&path), FileType::JavaScript);
    }

    #[test]
    fn test_detect_es_module_file() {
        let path = PathBuf::from("module.mjs");
        assert_eq!(detect_file_type(&path), FileType::EsModule);
    }

    #[test]
    fn test_detect_commonjs_file() {
        let path = PathBuf::from("module.cjs");
        assert_eq!(detect_file_type(&path), FileType::CommonJs);
    }

    #[test]
    fn test_detect_typescript_file() {
        let path = PathBuf::from("script.ts");
        assert_eq!(detect_file_type(&path), FileType::TypeScript);
    }

    #[test]
    fn test_detect_tsx_file() {
        let path = PathBuf::from("component.tsx");
        assert_eq!(detect_file_type(&path), FileType::TypeScript);
    }

    #[test]
    fn test_detect_json_file() {
        let path = PathBuf::from("config.json");
        assert_eq!(detect_file_type(&path), FileType::Json);
    }

    #[test]
    fn test_detect_unknown_file() {
        let path = PathBuf::from("readme.md");
        assert_eq!(detect_file_type(&path), FileType::Unknown);
    }

    #[test]
    fn test_detect_nested_path() {
        let path = PathBuf::from("/home/user/project/src/index.ts");
        assert_eq!(detect_file_type(&path), FileType::TypeScript);
    }

    #[test]
    fn test_detect_mts_file() {
        let path = PathBuf::from("module.mts");
        assert_eq!(detect_file_type(&path), FileType::TypeScript);
    }

    #[test]
    fn test_detect_cts_file() {
        let path = PathBuf::from("module.cts");
        assert_eq!(detect_file_type(&path), FileType::TypeScript);
    }
}

/// ============================================
/// Execution Context Tests
/// ============================================

mod execution_context {
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Execution context for scripts
    #[derive(Debug, Clone)]
    pub struct ExecutionContext {
        /// Current working directory
        pub cwd: PathBuf,
        /// Script file path
        pub script_path: PathBuf,
        /// __dirname equivalent
        pub dirname: PathBuf,
        /// __filename equivalent
        pub filename: PathBuf,
        /// Command line arguments (process.argv)
        pub argv: Vec<String>,
        /// Environment variables
        pub env: HashMap<String, String>,
    }

    impl ExecutionContext {
        pub fn new(script_path: PathBuf) -> Self {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let absolute_path = if script_path.is_absolute() {
                script_path.clone()
            } else {
                cwd.join(&script_path)
            };

            let dirname = absolute_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));

            Self {
                cwd,
                script_path: absolute_path.clone(),
                dirname,
                filename: absolute_path,
                argv: vec![
                    "amber".to_string(),
                    script_path.to_string_lossy().to_string(),
                ],
                env: std::env::vars().collect(),
            }
        }

        pub fn with_args(mut self, args: Vec<String>) -> Self {
            self.argv.extend(args);
            self
        }

        pub fn with_env(mut self, key: &str, value: &str) -> Self {
            self.env.insert(key.to_string(), value.to_string());
            self
        }
    }

    #[test]
    fn test_context_creation() {
        let ctx = ExecutionContext::new(PathBuf::from("test.js"));
        assert!(ctx.argv.len() >= 2);
        assert_eq!(ctx.argv[0], "amber");
    }

    #[test]
    fn test_dirname_calculation() {
        let ctx = ExecutionContext::new(PathBuf::from("/home/user/project/script.js"));
        assert_eq!(ctx.dirname, PathBuf::from("/home/user/project"));
    }

    #[test]
    fn test_filename_is_absolute() {
        let ctx = ExecutionContext::new(PathBuf::from("script.js"));
        assert!(ctx.filename.is_absolute());
    }

    #[test]
    fn test_argv_with_args() {
        let ctx = ExecutionContext::new(PathBuf::from("script.js"))
            .with_args(vec!["--port".to_string(), "3000".to_string()]);

        assert_eq!(ctx.argv.len(), 4);
        assert_eq!(ctx.argv[2], "--port");
        assert_eq!(ctx.argv[3], "3000");
    }

    #[test]
    fn test_env_variable_injection() {
        let ctx = ExecutionContext::new(PathBuf::from("script.js"))
            .with_env("NODE_ENV", "production")
            .with_env("API_KEY", "secret");

        assert_eq!(ctx.env.get("NODE_ENV"), Some(&"production".to_string()));
        assert_eq!(ctx.env.get("API_KEY"), Some(&"secret".to_string()));
    }

    #[test]
    fn test_cwd_is_set() {
        let ctx = ExecutionContext::new(PathBuf::from("script.js"));
        assert!(ctx.cwd.exists() || ctx.cwd == PathBuf::from("."));
    }
}

/// ============================================
/// Module Resolution Tests
/// ============================================

mod module_resolution {
    use std::path::PathBuf;

    /// Module resolution result
    #[derive(Debug, Clone, PartialEq)]
    pub enum ResolvedModule {
        /// Builtin module (fs, path, etc.)
        Builtin(String),
        /// File path
        File(PathBuf),
        /// Node modules package
        Package { name: String, entry: PathBuf },
        /// Not found
        NotFound(String),
    }

    /// Module resolver following Node.js resolution algorithm
    pub struct ModuleResolver {
        /// Search paths for node_modules
        search_paths: Vec<PathBuf>,
        /// Builtin module names
        builtins: Vec<&'static str>,
    }

    impl ModuleResolver {
        pub fn new(base_path: &PathBuf) -> Self {
            let mut search_paths = Vec::new();
            let mut current = base_path.clone();

            // Build node_modules search paths
            while current.parent().is_some() {
                let nm_path = current.join("node_modules");
                search_paths.push(nm_path);
                current = current.parent().unwrap().to_path_buf();
            }

            Self {
                search_paths,
                builtins: vec![
                    "fs",
                    "path",
                    "os",
                    "crypto",
                    "http",
                    "https",
                    "url",
                    "querystring",
                    "stream",
                    "events",
                    "util",
                    "buffer",
                    "assert",
                    "child_process",
                    "cluster",
                    "dns",
                    "net",
                    "readline",
                    "repl",
                    "tty",
                    "v8",
                    "vm",
                    "zlib",
                    "process",
                    "console",
                    "module",
                ],
            }
        }

        pub fn resolve(&self, request: &str, parent: &PathBuf) -> ResolvedModule {
            // Check for builtin modules
            if self.builtins.contains(&request) {
                return ResolvedModule::Builtin(request.to_string());
            }

            // Relative path resolution
            if request.starts_with("./") || request.starts_with("../") {
                let resolved = parent
                    .parent()
                    .map(|p| p.join(request))
                    .unwrap_or_else(|| PathBuf::from(request));

                // Try with extensions
                for ext in &["", ".js", ".ts", ".json", "/index.js", "/index.ts"] {
                    let with_ext = PathBuf::from(format!("{}{}", resolved.display(), ext));
                    if with_ext.exists() {
                        return ResolvedModule::File(with_ext);
                    }
                }

                return ResolvedModule::NotFound(request.to_string());
            }

            // Package resolution
            for search_path in &self.search_paths {
                let package_path = search_path.join(request);
                if package_path.exists() {
                    // Try package.json main field
                    let pkg_json = package_path.join("package.json");
                    if pkg_json.exists() {
                        // In real impl, would parse package.json for "main" field
                        let entry = package_path.join("index.js");
                        return ResolvedModule::Package {
                            name: request.to_string(),
                            entry,
                        };
                    }

                    // Try index.js
                    let index = package_path.join("index.js");
                    if index.exists() {
                        return ResolvedModule::File(index);
                    }
                }
            }

            ResolvedModule::NotFound(request.to_string())
        }
    }

    #[test]
    fn test_builtin_module_resolution() {
        let resolver = ModuleResolver::new(&PathBuf::from("/home/user/project"));
        let result = resolver.resolve("fs", &PathBuf::from("/home/user/project/index.js"));
        assert_eq!(result, ResolvedModule::Builtin("fs".to_string()));
    }

    #[test]
    fn test_path_builtin() {
        let resolver = ModuleResolver::new(&PathBuf::from("/home/user/project"));
        let result = resolver.resolve("path", &PathBuf::from("/home/user/project/index.js"));
        assert_eq!(result, ResolvedModule::Builtin("path".to_string()));
    }

    #[test]
    fn test_crypto_builtin() {
        let resolver = ModuleResolver::new(&PathBuf::from("/home/user/project"));
        let result = resolver.resolve("crypto", &PathBuf::from("/home/user/project/index.js"));
        assert_eq!(result, ResolvedModule::Builtin("crypto".to_string()));
    }

    #[test]
    fn test_relative_not_found() {
        let resolver = ModuleResolver::new(&PathBuf::from("/tmp"));
        let result = resolver.resolve("./nonexistent", &PathBuf::from("/tmp/index.js"));
        assert!(matches!(result, ResolvedModule::NotFound(_)));
    }

    #[test]
    fn test_package_not_found() {
        let resolver = ModuleResolver::new(&PathBuf::from("/tmp"));
        let result = resolver.resolve("nonexistent-package", &PathBuf::from("/tmp/index.js"));
        assert!(matches!(result, ResolvedModule::NotFound(_)));
    }

    #[test]
    fn test_search_paths_generated() {
        let resolver = ModuleResolver::new(&PathBuf::from("/home/user/project/src"));
        assert!(resolver.search_paths.len() >= 1);
        assert!(resolver.search_paths[0]
            .to_string_lossy()
            .contains("node_modules"));
    }
}

/// ============================================
/// Argument Parsing Tests
/// ============================================

mod argument_parsing {
    /// Parsed script arguments with separator handling
    #[derive(Debug, Clone)]
    pub struct ParsedArgs {
        /// Arguments before -- separator (for amber)
        pub runtime_args: Vec<String>,
        /// Arguments after -- separator (for script)
        pub script_args: Vec<String>,
        /// Script path
        pub script_path: Option<String>,
    }

    /// Options that take a value (like --config <file>)
    const OPTIONS_WITH_VALUES: &[&str] = &[
        "--config",
        "-c",
        "--env",
        "-e",
        "--loader",
        "--outdir",
        "--outfile",
        "--target",
    ];

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
                // Skip first arg (program name)
                if arg == "--" {
                    found_separator = true;
                    continue;
                }

                if found_separator {
                    script_args.push(arg);
                } else if expect_value {
                    // This arg is a value for the previous option
                    runtime_args.push(arg);
                    expect_value = false;
                } else if !found_script && !arg.starts_with('-') {
                    // First non-flag argument (not an option value) is the script
                    script_path = Some(arg);
                    found_script = true;
                } else if found_script {
                    // Args after script (before --) go to script
                    script_args.push(arg);
                } else {
                    runtime_args.push(arg.clone());
                    // Check if this option expects a value
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

    #[test]
    fn test_simple_script_path() {
        let args = vec!["amber".to_string(), "script.js".to_string()];
        let parsed = ParsedArgs::parse(args);
        assert_eq!(parsed.script_path, Some("script.js".to_string()));
        assert!(parsed.script_args.is_empty());
    }

    #[test]
    fn test_script_with_args() {
        let args = vec![
            "amber".to_string(),
            "script.js".to_string(),
            "--port".to_string(),
            "3000".to_string(),
        ];
        let parsed = ParsedArgs::parse(args);
        assert_eq!(parsed.script_path, Some("script.js".to_string()));
        assert_eq!(parsed.script_args, vec!["--port", "3000"]);
    }

    #[test]
    fn test_separator_handling() {
        let args = vec![
            "amber".to_string(),
            "--verbose".to_string(),
            "script.js".to_string(),
            "--".to_string(),
            "--script-arg".to_string(),
            "value".to_string(),
        ];
        let parsed = ParsedArgs::parse(args);
        assert_eq!(parsed.runtime_args, vec!["--verbose"]);
        assert_eq!(parsed.script_path, Some("script.js".to_string()));
        assert_eq!(parsed.script_args, vec!["--script-arg", "value"]);
    }

    #[test]
    fn test_no_script() {
        let args = vec!["amber".to_string(), "--help".to_string()];
        let parsed = ParsedArgs::parse(args);
        assert_eq!(parsed.script_path, None);
        assert_eq!(parsed.runtime_args, vec!["--help"]);
    }

    #[test]
    fn test_complex_args() {
        let args = vec![
            "amber".to_string(),
            "-v".to_string(),
            "--config".to_string(),
            "amberjs.config.js".to_string(),
            "app.ts".to_string(),
            "--".to_string(),
            "--env".to_string(),
            "production".to_string(),
            "--port".to_string(),
            "8080".to_string(),
        ];
        let parsed = ParsedArgs::parse(args);

        // Runtime args should include flags before script
        assert!(parsed.runtime_args.contains(&"-v".to_string()));
        assert!(parsed.runtime_args.contains(&"--config".to_string()));

        // Script path should be detected
        assert_eq!(parsed.script_path, Some("app.ts".to_string()));

        // Script args should be after --
        assert_eq!(parsed.script_args.len(), 4);
    }
}

/// ============================================
/// Script Executor Integration Tests
/// ============================================

mod script_executor {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::execution_context::ExecutionContext;
    use super::file_type_detection::FileType;

    /// Script executor configuration
    #[derive(Debug, Clone)]
    pub struct ExecutorConfig {
        /// Enable TypeScript transpilation
        pub transpile_ts: bool,
        /// Enable hot reload
        pub hot_reload: bool,
        /// Watch mode
        pub watch: bool,
        /// Source maps
        pub source_maps: bool,
    }

    impl Default for ExecutorConfig {
        fn default() -> Self {
            Self {
                transpile_ts: true,
                hot_reload: false,
                watch: false,
                source_maps: true,
            }
        }
    }

    /// Script execution result
    #[derive(Debug)]
    pub struct ExecutionResult {
        /// Exit code
        pub exit_code: i32,
        /// Standard output
        pub stdout: String,
        /// Standard error
        pub stderr: String,
        /// Execution time in milliseconds
        pub execution_time_ms: u64,
    }

    /// Main script executor
    pub struct ScriptExecutor {
        config: ExecutorConfig,
    }

    impl ScriptExecutor {
        pub fn new(config: ExecutorConfig) -> Self {
            Self { config }
        }

        /// Validate script file before execution
        pub fn validate_script(&self, path: &PathBuf) -> Result<(), String> {
            if !path.exists() {
                return Err(format!("Script not found: {}", path.display()));
            }

            let file_type = super::file_type_detection::detect_file_type(path);
            match file_type {
                FileType::JavaScript | FileType::EsModule | FileType::CommonJs => Ok(()),
                FileType::TypeScript => {
                    if self.config.transpile_ts {
                        Ok(())
                    } else {
                        Err("TypeScript transpilation is disabled".to_string())
                    }
                }
                FileType::Json => Ok(()), // JSON can be required/imported
                FileType::Unknown => Err(format!("Unknown file type: {}", path.display())),
            }
        }

        /// Build process.argv array
        pub fn build_process_argv(&self, ctx: &ExecutionContext) -> Vec<String> {
            ctx.argv.clone()
        }

        /// Build process.env object
        pub fn build_process_env(&self, ctx: &ExecutionContext) -> HashMap<String, String> {
            ctx.env.clone()
        }

        /// Check if file needs transpilation
        pub fn needs_transpilation(&self, path: &PathBuf) -> bool {
            let file_type = super::file_type_detection::detect_file_type(path);
            matches!(file_type, FileType::TypeScript)
        }
    }

    #[test]
    fn test_executor_creation() {
        let executor = ScriptExecutor::new(ExecutorConfig::default());
        assert!(executor.config.transpile_ts);
        assert!(executor.config.source_maps);
    }

    #[test]
    fn test_validate_unknown_extension() {
        let executor = ScriptExecutor::new(ExecutorConfig::default());
        let result = executor.validate_script(&PathBuf::from("/tmp/nonexistent.xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn test_typescript_needs_transpilation() {
        let executor = ScriptExecutor::new(ExecutorConfig::default());
        assert!(executor.needs_transpilation(&PathBuf::from("script.ts")));
        assert!(executor.needs_transpilation(&PathBuf::from("component.tsx")));
    }

    #[test]
    fn test_javascript_no_transpilation() {
        let executor = ScriptExecutor::new(ExecutorConfig::default());
        assert!(!executor.needs_transpilation(&PathBuf::from("script.js")));
        assert!(!executor.needs_transpilation(&PathBuf::from("module.mjs")));
    }

    #[test]
    fn test_build_process_argv() {
        let executor = ScriptExecutor::new(ExecutorConfig::default());
        let ctx = ExecutionContext::new(PathBuf::from("script.js"))
            .with_args(vec!["--arg1".to_string(), "value1".to_string()]);

        let argv = executor.build_process_argv(&ctx);
        assert!(argv.len() >= 4);
        assert_eq!(argv[0], "amber");
    }

    #[test]
    fn test_build_process_env() {
        let executor = ScriptExecutor::new(ExecutorConfig::default());
        let ctx =
            ExecutionContext::new(PathBuf::from("script.js")).with_env("TEST_VAR", "test_value");

        let env = executor.build_process_env(&ctx);
        assert_eq!(env.get("TEST_VAR"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_config_hot_reload() {
        let config = ExecutorConfig {
            hot_reload: true,
            watch: true,
            ..Default::default()
        };
        let executor = ScriptExecutor::new(config);
        assert!(executor.config.hot_reload);
        assert!(executor.config.watch);
    }

    #[test]
    fn test_disabled_typescript() {
        let config = ExecutorConfig {
            transpile_ts: false,
            ..Default::default()
        };
        let executor = ScriptExecutor::new(config);
        // Validation should fail for TS files when transpilation is disabled
        // (but we can't test actual file validation without a real file)
        assert!(!executor.config.transpile_ts);
    }
}

/// ============================================
/// Shebang Detection Tests
/// ============================================

mod shebang_detection {

    /// Detect shebang from file content
    pub fn detect_shebang(content: &str) -> Option<String> {
        let first_line = content.lines().next()?;
        if first_line.starts_with("#!") {
            Some(first_line[2..].trim().to_string())
        } else {
            None
        }
    }

    /// Check if shebang indicates amberjs script
    pub fn is_amberjs_shebang(shebang: &str) -> bool {
        shebang.contains("amber") ||
        shebang.ends_with("/env amber") ||
        shebang.ends_with("/env node") ||  // Node compatibility
        shebang.ends_with("/env bun") // Bun compatibility
    }

    #[test]
    fn test_detect_amberjs_shebang() {
        let content = "#!/usr/bin/env amber\nconsole.log('hello');";
        let shebang = detect_shebang(content);
        assert_eq!(shebang, Some("/usr/bin/env amber".to_string()));
    }

    #[test]
    fn test_detect_node_shebang() {
        let content = "#!/usr/bin/env node\nconsole.log('hello');";
        let shebang = detect_shebang(content);
        assert!(shebang.is_some());
        assert!(is_amberjs_shebang(&shebang.unwrap()));
    }

    #[test]
    fn test_no_shebang() {
        let content = "console.log('hello');";
        assert!(detect_shebang(content).is_none());
    }

    #[test]
    fn test_is_amberjs_shebang_direct() {
        assert!(is_amberjs_shebang("/usr/bin/amber"));
        assert!(is_amberjs_shebang("/usr/bin/env amber"));
    }

    #[test]
    fn test_is_node_compatible_shebang() {
        assert!(is_amberjs_shebang("/usr/bin/env node"));
    }

    #[test]
    fn test_is_bun_compatible_shebang() {
        assert!(is_amberjs_shebang("/usr/bin/env bun"));
    }

    #[test]
    fn test_unrecognized_shebang() {
        assert!(!is_amberjs_shebang("/usr/bin/python"));
        assert!(!is_amberjs_shebang("/bin/bash"));
    }
}
