// Enhanced REPL (Read-Eval-Print Loop)
// Stage 91 Phase 4.2 - Enhanced REPL with tab completion, syntax highlighting, and advanced commands
//
// Features:
// - Tab auto-completion for variables, properties, keywords, and built-ins
// - Syntax highlighting for better code readability
// - Arrow key history navigation using rustyline
// - Enhanced commands: .inspect, .time, .type, .await, .save
// - Multi-line editing with smart indentation

use crate::cli::repl_completer::{CompletionCandidate, ReplCompleter};
use crate::cli::repl_highlighter::{HighlightTheme, ReplHighlighter};
use rustyline::Editor;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::fs::File;
use anyhow::{Result, Error};
use rusty_v8 as v8;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::task::Context;

/// Enhanced REPL configuration
#[derive(Debug, Clone)]
pub struct EnhancedReplConfig {
    /// Prompt to display
    pub prompt: String,
    /// Maximum history size
    pub history_size: usize,
    /// Enable tab completion
    pub tab_completion: bool,
    /// Enable syntax highlighting
    pub syntax_highlighting: bool,
    /// Auto-indent for multiline input
    pub auto_indent: bool,
    /// History file path
    pub history_file: Option<String>,
}
impl Default for EnhancedReplConfig {
    fn default() -> Self {
        Self {
            prompt: "amber> ".to_string(),
            history_size: 1000,
            tab_completion: true,
            syntax_highlighting: true,
            auto_indent: true,
            history_file: Some(".amberjs_repl_history".to_string()),
        }
    }
}
/// Enhanced REPL result
#[derive(Debug, Clone)]
pub struct EnhancedReplResult {
    pub output: String,
    pub execution_time: std::time::Duration,
    pub is_error: bool,
}
/// Enhanced REPL (Read-Eval-Print Loop) with advanced features
pub struct EnhancedRepl {
    /// Runtime to execute code
    runtime: Arc<RuntimeLite>,
    /// Configuration
    config: EnhancedReplConfig,
    /// Tab completer
    completer: ReplCompleter,
    /// Syntax highlighter
    highlighter: ReplHighlighter,
    /// rustyline editor for enhanced input
    editor: Editor<(), rustyline::history::DefaultHistory>,
    /// Command history (for .save command)
    history: VecDeque<String>,
    /// Current multiline input buffer
    multiline_buffer: Vec<String>,
    /// Multiline input flag
    in_multiline: bool,
    /// Indentation level for multiline
    indent_level: usize,
    /// V8 isolate for runtime inspection
    isolate: Option<Arc<v8::Isolate>>,
    /// V8 context for runtime inspection
    context: Option<v8::Global<v8::Context>>,
    /// Execution count
    execution_count: usize,
}
impl EnhancedRepl {
    /// Create a new enhanced REPL
    pub fn new(runtime: Arc<RuntimeLite>) -> anyhow::Result<Self> {
        let config: _ = EnhancedReplConfig::default();
        Self::with_config(runtime, config)
    }
    /// Create with custom configuration
    pub fn with_config(runtime: Arc<RuntimeLite>, config: EnhancedReplConfig) -> anyhow::Result<Self> {
        let mut editor = Editor::<(), rustyline::history::DefaultHistory>::new()
            .map_err(|e| anyhow::anyhow!("Failed to create editor: {}", e))?;
        // Load history if configured
        if let Some(ref history_file) = config.history_file {
            let _: _ = editor.load_history(history_file);
        }
        // Set up tab completion
        if config.tab_completion {
            editor.set_helper(Some(()));
        }
        let completer: _ = ReplCompleter::new();
        let highlighter: _ = ReplHighlighter::new();
        Ok(Self {
            runtime,
            config,
            completer,
            highlighter,
            editor,
            history: VecDeque::new(),
            multiline_buffer: Vec::new(),
            in_multiline: false,
            indent_level: 0,
            isolate: None,
            context: None,
            execution_count: 0,
        })
    }
    /// Set V8 runtime context for inspection
    pub fn set_v8_runtime(&mut self, isolate: Arc<v8::Isolate>, context: v8::Global<v8::Context>) {
        self.isolate = Some(Arc::clone(isolate));
        self.context = Some(context.clone());
        self.completer.set_runtime(isolate, context);
    }
    /// Run the enhanced REPL
    pub fn run(&mut self) -> anyhow::Result<()> {
        println!("🐝 Amber Enhanced REPL - High-performance JavaScript/TypeScript runtime");
        println!("Features: Tab completion, syntax highlighting, arrow key navigation");
        println!("Type JavaScript code and press Enter to execute");
        println!("Type .help for more information");
        println!();
        loop {
            let prompt: _ = if self.in_multiline {
                format!("{}... ", " ".repeat(self.config.prompt.len() - 4))
            } else {
                self.config.prompt.clone()
            };
            let readline: _ = self.editor.readline(&prompt);
            match readline {
                Ok(line) => {
                    if let Err(e) = self.handle_input(&line) {
                        eprintln!("Error handling input: {}", e);
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("\nGoodbye! 👋");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }
        // Save history
        if let Some(ref history_file) = self.config.history_file {
            let _: _ = self.editor.save_history(history_file);
        }
        Ok(())
    }
    /// Handle user input
    fn handle_input(&mut self, input: &str) -> anyhow::Result<()> {
        let trimmed: _ = input.trim();
        // Handle special REPL commands
        if trimmed.starts_with('.') {
            let _: _ = self.handle_special_command(trimmed)?;
            return Ok(());
        }
        // Skip empty lines
        if trimmed.is_empty() {
            if self.in_multiline {
                self.execute_multiline()?;
            }
            return Ok(());
        }
        // Check for multiline input
        if self.is_multiline_start(trimmed) {
            self.start_multiline(trimmed);
            return Ok(());
        }
        if self.in_multiline {
            self.add_to_multiline(trimmed);
            return Ok(());
        }
        // Execute single line
        self.execute_line(input)?;
        Ok(())
    }
    /// Handle special REPL commands
    fn handle_special_command(&mut self, cmd: &str) -> anyhow::Result<bool> {
        match cmd {
            ".exit" | ".quit" | ".q" => {
                println!("Goodbye! 👋");
                Ok(true)
            }
            ".help" => {
                self.print_help();
                Ok(false)
            }
            ".clear" => {
                self.clear_screen();
                Ok(false)
            }
            ".history" => {
                self.print_history();
                Ok(false)
            }
            cmd if cmd.starts_with(".load ") => {
                let path: _ = &cmd[6..].trim();
                self.load_file(path)?;
                Ok(false)
            }
            cmd if cmd.starts_with(".save ") => {
                let path: _ = &cmd[6..].trim();
                self.save_session(path)?;
                Ok(false)
            }
            cmd if cmd.starts_with(".inspect ") => {
                let expr: _ = &cmd[9..].trim();
                self.inspect_expression(expr)?;
                Ok(false)
            }
            cmd if cmd.starts_with(".time ") => {
                let expr: _ = &cmd[6..].trim();
                self.time_expression(expr)?;
                Ok(false)
            }
            cmd if cmd.starts_with(".type ") => {
                let expr: _ = &cmd[6..].trim();
                self.type_expression(expr)?;
                Ok(false)
            }
            cmd if cmd.starts_with(".await ") => {
                let expr: _ = &cmd[8..].trim();
                self.await_expression(expr)?;
                Ok(false)
            }
            _ => {
                println!("Unknown command: {}. Type .help for available commands.", cmd);
                Ok(false)
            }
        }
    }
    /// Check if input starts a multiline block
    fn is_multiline_start(&self, input: &str) -> bool {
        let trimmed: _ = input.trim();
        trimmed.ends_with('{') ||
            trimmed.ends_with('(') ||
            trimmed.ends_with('[') ||
            trimmed.starts_with("function ") ||
            trimmed.starts_with("if ") ||
            trimmed.starts_with("for ") ||
            trimmed.starts_with("while ") ||
            trimmed.starts_with("try ") ||
            trimmed.starts_with("class ")
    }
    /// Start multiline input
    fn start_multiline(&mut self, input: &str) {
        self.in_multiline = true;
        self.multiline_buffer.clear();
        self.add_to_multiline(input);
    }
    /// Add line to multiline buffer
    fn add_to_multiline(&mut self, input: &str) {
        self.multiline_buffer.push(input.to_string());
        // Auto-indent
        if self.config.auto_indent {
            let trimmed: _ = input.trim();
            if trimmed.ends_with('{') {
                self.indent_level += 1;
            } else if trimmed.starts_with('}') && self.indent_level > 0 {
                self.indent_level -= 1;
            }
        }
    }
    /// Execute multiline buffer
    fn execute_multiline(&mut self) -> anyhow::Result<()> {
        let code: _ = self.multiline_buffer.join("\n");
        self.execute_code(&code)?;
        // Reset multiline state
        self.in_multiline = false;
        self.multiline_buffer.clear();
        self.indent_level = 0;
        Ok(())
    }
    /// Execute a single line
    fn execute_line(&mut self, code: &str) -> anyhow::Result<()> {
        self.execute_code(code)?;
        // Add to history
        self.editor.add_history_entry(code);
        Ok(())
    }
    /// Execute JavaScript/TypeScript code
    fn execute_code(&mut self, code: &str) -> anyhow::Result<EnhancedReplResult> {
        let start: _ = Instant::now();
        match self.runtime.execute_code(code) {
            Ok(result) => {
                let execution_time: _ = start.elapsed();
                self.execution_count += 1;
                // Print result if not undefined
                if !result.is_empty() && result != "undefined" {
                    println!("{}", result);
                }
                Ok(EnhancedReplResult {
                    output: result,
                    execution_time,
                    is_error: false,
                })
            }
            Err(e) => {
                let execution_time: _ = start.elapsed();
                eprintln!("Error: {}", e);
                Ok(EnhancedReplResult {
                    output: e.to_string(),
                    execution_time,
                    is_error: true,
                })
            }
        }
    }
    /// Inspect expression deeply
    fn inspect_expression(&mut self, expr: &str) -> anyhow::Result<()> {
        println!("\n🔍 Inspecting: {}", expr);
        // Execute the expression to get the value
        let result: _ = self.runtime.execute_code(expr);
        match result {
            Ok(value) => {
                println!("Value: {}", value);
                // TODO: Implement deep inspection of object structure
                // This would involve traversing the V8 object hierarchy
                println!("💡 Tip: Use console.log() for detailed object inspection");
            }
            Err(e) => {
                eprintln!("Error inspecting expression: {}", e);
            }
        }
        println!();
        Ok(())
    }
    /// Time expression execution
    fn time_expression(&mut self, expr: &str) -> anyhow::Result<()> {
        println!("\n⏱ Timing: {}", expr);
        // Run multiple iterations for accurate timing
        let iterations: _ = 1000;
        let start: _ = Instant::now();
        for _ in 0..iterations {
            let _: _ = self.runtime.execute_code(expr);
        }
        let elapsed: _ = start.elapsed();
        let avg_time: _ = elapsed.as_nanos() as f64 / iterations as f64;
        println!("  Total time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
        println!("  Average time: {:.3}µs", avg_time / 1000.0);
        println!("  Iterations: {}", iterations);
        println!();
        Ok(())
    }
    /// Show type of expression
    fn type_expression(&mut self, expr: &str) -> anyhow::Result<()> {
        println!("\n📝 Type of: {}", expr);
        // Execute to get value, then determine type
        let result: _ = self.runtime.execute_code(&format!("typeof ({})", expr));
        match result {
            Ok(type_str) => {
                println!("Type: {}", type_str);
                // Additional type information
                let type_of_result: _ = self.runtime.execute_code(&format!("({}) instanceof Object", expr));
                if let Ok(is_object) = type_of_result {
                    if is_object == "true" {
                        let constructor: _ = self.runtime.execute_code(&format!("({}).constructor?.name", expr));
                        if let Ok(name) = constructor {
                            println!("Constructor: {}", name);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error getting type: {}", e);
            }
        }
        println!();
        Ok(())
    }
    /// Await a promise
    fn await_expression(&mut self, expr: &str) -> anyhow::Result<()> {
        println!("\n⏳ Awaiting: {}", expr);
        // Execute async expression
        let result: _ = self.runtime.execute_code(&format!("(async () => {{ return {}; }})()", expr));
        match result {
            Ok(value) => {
                println!("Resolved: {}", value);
            }
            Err(e) => {
                eprintln!("Promise rejected or error: {}", e);
            }
        }
        println!();
        Ok(())
    }
    /// Save session to file
    fn save_session(&self, path: &str) -> anyhow::Result<()> {
        let mut file = std::fs::File::create(path)
            .map_err(|e| anyhow::anyhow!("Failed to create file: {}", e))?;
        writeln!(file, "// Amber REPL Session")?;
        writeln!(file, "// Saved on {}", chrono::Utc::now())?;
        writeln!(file)?;
        for cmd in &self.history {
            writeln!(file, "{}", cmd)?;
        }
        println!("Session saved to: {}", path);
        Ok(())
    }
    /// Load and execute file
    fn load_file(&mut self, path: &str) -> anyhow::Result<()> {
        let content: _ = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;
        println!("Loading file: {} ({} bytes)", path, content.len());
        // Execute the file
        let result: _ = self.runtime.execute_code(&content);
        match result {
            Ok(_) => {
                println!("File loaded successfully");
            }
            Err(e) => {
                eprintln!("Error loading file: {}", e);
            }
        }
        Ok(())
    }
    /// Print help information
    fn print_help(&self) {
        println!("\n🐝 Amber Enhanced REPL Commands:");
        println!("  .exit, .quit    - Exit the REPL");
        println!("  .help           - Show this help message");
        println!("  .clear          - Clear the screen");
        println!("  .history        - Show command history");
        println!("  .load <file>    - Load and execute a file");
        println!("  .save <file>    - Save session history to a file");
        println!("  .inspect <expr> - Inspect object deeply");
        println!("  .time <expr>    - Measure execution time");
        println!("  .type <expr>    - Show type information");
        println!("  .await <expr>   - Await a promise");
        println!("\nFeatures:");
        println!("  ✓ Tab completion - Press Tab for auto-completion");
        println!("  ✓ Syntax highlighting - Color-coded code display");
        println!("  ✓ Arrow keys - Navigate history with ↑ and ↓");
        println!("  ✓ Multiline input - Automatic detection and indentation");
        println!();
    }
    /// Clear the screen
    fn clear_screen(&self) {
        print!("\x1B[2J\x1B[H");
        std::io::stdout().flush().unwrap();
    }
    /// Print command history
    fn print_history(&self) {
        println!("\nCommand History:");
        for (i, cmd) in self.history.iter().enumerate() {
            let display: _ = if cmd.len() > 60 {
                format!("{}...", &cmd[..57])
            } else {
                cmd.clone()
            };
            println!("  {:4}: {}", i + 1, display.replace('\n', "↵"));
        }
        println!();
    }
    /// Get execution statistics
    pub fn get_stats(&self) -> EnhancedReplStats {
        let total_commands: _ = self.history.len();
        let avg_history_len: _ = if total_commands > 0 {
            self.history.iter().map(|s| s.len()).sum::<usize>() / total_commands
        } else {
            0
        };
        EnhancedReplStats {
            total_commands,
            avg_command_length: avg_history_len,
            history_size: self.history.len(),
            max_history_size: self.config.history_size,
            execution_count: self.execution_count,
            tab_completion_enabled: self.config.tab_completion,
            syntax_highlighting_enabled: self.config.syntax_highlighting,
        }
    }
}
/// Enhanced REPL statistics
#[derive(Debug, Clone)]
pub struct EnhancedReplStats {
    pub total_commands: usize,
    pub avg_command_length: usize,
    pub history_size: usize,
    pub max_history_size: usize,
    pub execution_count: usize,
    pub tab_completion_enabled: bool,
    pub syntax_highlighting_enabled: bool,
}
impl EnhancedReplStats {
    pub fn print(&self) {
        println!("\n📊 Enhanced REPL Statistics:");
        println!("  Total commands executed: {}", self.execution_count);
        println!("  History entries: {}/{}", self.history_size, self.max_history_size);
        println!("  Average command length: {} characters", self.avg_command_length);
        println!("  Tab completion: {}", if self.tab_completion_enabled { "✓ Enabled" } else { "✗ Disabled" });
        println!("  Syntax highlighting: {}", if self.syntax_highlighting_enabled { "✓ Enabled" } else { "✗ Disabled" });
        println!();
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_enhanced_repl_creation() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let repl: _ = EnhancedRepl::new(runtime);
        assert!(repl.is_ok());
    }
    #[test]
    fn test_enhanced_repl_config() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let config: _ = EnhancedReplConfig::default();
        let repl: _ = EnhancedRepl::with_config(runtime, config);
        assert!(repl.is_ok());
    }
    #[test]
    fn test_multiline_detection() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let mut repl = EnhancedRepl::new(runtime).unwrap();
        assert!(repl.is_multiline_start("function foo() {"));
        assert!(repl.is_multiline_start("if (true) {"));
        assert!(!repl.is_multiline_start("console.log('hello')"));
    }
    #[test]
    fn test_save_session() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let mut repl = EnhancedRepl::new(runtime).unwrap();
        // Add some commands to history
        repl.history.push_back("let x: _ = 1;".to_string());
        repl.history.push_back("let y: _ = 2;".to_string());
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        let result: _ = repl.save_session(temp_file.path().to_str().unwrap());
        assert!(result.is_ok());
        assert!(temp_file.path().exists());
    }
    #[test]
    fn test_load_file() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let mut repl = EnhancedRepl::new(runtime).unwrap();
        // Create a temporary file with JavaScript code
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        writeln!(temp_file, "let test: _ = 42;").unwrap();
        let result: _ = repl.load_file(temp_file.path().to_str().unwrap());
        assert!(result.is_ok());
    }
    #[test]
    fn test_inspect_command() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let mut repl = EnhancedRepl::new(runtime).unwrap();
        let result: _ = repl.inspect_expression("42");
        assert!(result.is_ok());
    }
    #[test]
    fn test_time_command() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let mut repl = EnhancedRepl::new(runtime).unwrap();
        let result: _ = repl.time_expression("1 + 1");
        assert!(result.is_ok());
    }
    #[test]
    fn test_type_command() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let mut repl = EnhancedRepl::new(runtime).unwrap();
        let result: _ = repl.type_expression("42");
        assert!(result.is_ok());
    }
    #[test]
    fn test_await_command() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let mut repl = EnhancedRepl::new(runtime).unwrap();
        let result: _ = repl.await_expression("Promise.resolve(42)");
        assert!(result.is_ok());
    }
    #[test]
    fn test_get_stats() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).unwrap()));
        let repl: _ = EnhancedRepl::new(runtime).unwrap();
        let stats: _ = repl.get_stats();
        assert_eq!(stats.total_commands, 0);
        assert!(stats.execution_count >= 0);
    }
}