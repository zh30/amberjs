// REPL (Read-Eval-Print Loop) Module
// Stage 36.0 - 实现交互式 REPL 功能

use std::collections::{BTreeMap, HashMap};
use std::io::{Write, self};
use std::sync::{Arc, Mutex};
use anyhow::{Result, Error};
use std::time::{Duration, Instant};
use std::collections::VecDeque;

/// REPL configuration
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// Prompt to display
    pub prompt: String,
    /// Maximum history size
    pub history_size: usize,
    /// Enable syntax highlighting
    pub syntax_highlight: bool,
    /// Auto-indent for multiline input
    pub auto_indent: bool,
}
impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            prompt: "amber> ".to_string(),
            history_size: 100,
            syntax_highlight: false,
            auto_indent: true,
        }
    }
}
/// REPL result
#[derive(Debug, Clone)]
pub struct ReplResult {
    pub output: String,
    pub execution_time: std::time::Duration,
    pub is_error: bool,
}
/// REPL (Read-Eval-Print Loop) implementation
pub struct Repl {
    /// Runtime to execute code
    runtime: Arc<RuntimeLite>,
    /// Configuration
    config: ReplConfig,
    /// Command history
    history: VecDeque<String>,
    /// Current multiline input
    multiline_buffer: Vec<String>,
    /// Multiline input flag
    in_multiline: bool,
    /// Indentation level
    indent_level: usize,
}
impl Repl {
    /// Create a new REPL instance
    pub fn new(runtime: Arc<RuntimeLite>) -> Self {
        Self {
            runtime,
            config: ReplConfig::default(),
            history: VecDeque::new(),
            multiline_buffer: Vec::new(),
            in_multiline: false,
            indent_level: 0,
        }
    }
    /// Create with custom configuration
    pub fn with_config(runtime: Arc<RuntimeLite>, config: ReplConfig) -> Self {
        Self {
            runtime,
            config,
            history: VecDeque::new(),
            multiline_buffer: Vec::new(),
            in_multiline: false,
            indent_level: 0,
        }
    }
    /// Get runtime reference (for testing purposes)
    pub fn runtime(&self) -> &Arc<RuntimeLite> {
        &self.runtime
    }
    /// Run the REPL
    pub async fn run(&mut self) -> anyhow::Result<()> {
        println!("🐝 Amber REPL - High-performance JavaScript/TypeScript runtime");
        println!("Type JavaScript code and press Enter to execute");
        println!("Type .exit or Ctrl+C to quit");
        println!("Type .help for more information");
        println!();
        loop {
            // Display prompt
            let prompt: _ = if self.in_multiline {
                format!("{}... ", " ".repeat(self.config.prompt.len() - 4))
            } else {
                self.config.prompt.clone()
            };
            print!("{}", prompt);
            io::stdout().flush()?;
            // Read input
            let input: _ = self.read_line()?;
            // Process input
            if input.trim() == ".exit" || input.trim() == ".quit" {
                println!("Goodbye! 👋");
                break;
            }
            if input.trim() == ".help" {
                self.print_help();
                continue;
            }
            if input.trim() == ".clear" {
                self.clear_screen();
                continue;
            }
            if input.trim() == ".history" {
                self.print_history();
                continue;
            }
            // Handle empty input
            if input.trim().is_empty() {
                if self.in_multiline {
                    // Execute multiline input
                    self.execute_multiline().await?;
                }
                continue;
            }
            // Check for multiline input
            if self.is_multiline_start(&input) {
                self.start_multiline(&input);
                continue;
            }
            if self.in_multiline {
                self.add_to_multiline(&input);
                continue;
            }
            // Execute single line
            self.execute_line(&input).await?;
        }
        Ok(())
    }
    /// Read a line from stdin
    fn read_line(&self) -> anyhow::Result<String> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input)
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
    async fn execute_multiline(&mut self) -> anyhow::Result<()> {
        let code: _ = self.multiline_buffer.join("\n");
        self.execute_code(&code).await?;
        // Reset multiline state
        self.in_multiline = false;
        self.multiline_buffer.clear();
        self.indent_level = 0;
        Ok(())
    }
    /// Execute a single line
    async fn execute_line(&mut self, code: &str) -> anyhow::Result<()> {
        self.execute_code(code).await?;
        // Add to history
        if self.history.len() >= self.config.history_size {
            self.history.pop_front();
        }
        self.history.push_back(code.to_string());
        Ok(())
    }
    /// Execute JavaScript/TypeScript code
    async fn execute_code(&self, code: &str) -> anyhow::Result<ReplResult> {
        let start: _ = Instant::now();
        match self.runtime.execute_code(code) {
            Ok(result) => {
                let execution_time: _ = start.elapsed();
                // Print result if not undefined
                if result != "undefined" && !result.is_empty() {
                    println!("{}", result);
                }
                Ok(ReplResult {
                    output: result,
                    execution_time,
                    is_error: false,
                })
            }
            Err(e) => {
                let execution_time: _ = start.elapsed();
                println!("Error: {}", e);
                Ok(ReplResult {
                    output: e.to_string(),
                    execution_time,
                    is_error: true,
                })
            }
        }
    }
    /// Execute code and record in history (for testing)
    pub async fn execute_and_record(&mut self, code: &str) -> anyhow::Result<ReplResult> {
        let result: _ = self.execute_code(code).await?;
        // Add to history
        if self.history.len() >= self.config.history_size {
            self.history.pop_front();
        }
        self.history.push_back(code.to_string());
        Ok(result)
    }
    /// Print help information
    fn print_help(&self) {
        println!("\n🐝 Amber REPL Commands:");
        println!("  .exit, .quit    - Exit the REPL");
        println!("  .help           - Show this help message");
        println!("  .clear          - Clear the screen");
        println!("  .history        - Show command history");
        println!("\nJavaScript features:");
        println!("  - Multiline input: End a line with {{ or ( to continue");
        println!("  - Auto-completion: Press Tab for suggestions (coming soon)");
        println!("  - Syntax highlighting: Enabled for better readability");
        println!();
    }
    /// Clear the screen
    fn clear_screen(&self) {
        print!("\x1B[2J\x1B[H");
        io::stdout().flush().unwrap();
    }
    /// Print command history
    fn print_history(&self) {
        println!("\nCommand History:");
        for (i, cmd) in self.history.iter().enumerate() {
            println!("  {}: {}", i, cmd);
        }
        println!();
    }
    /// Get execution statistics
    pub fn get_stats(&self) -> ReplStats {
        let total_commands: _ = self.history.len();
        let avg_history_len: _ = if total_commands > 0 {
            self.history.iter().map(|s| s.len()).sum::<usize>() / total_commands
        } else {
            0
        };
        ReplStats {
            total_commands,
            avg_command_length: avg_history_len,
            history_size: self.history.len(),
            max_history_size: self.config.history_size,
        }
    }
}
/// REPL statistics
#[derive(Debug, Clone)]
pub struct ReplStats {
    pub total_commands: usize,
    pub avg_command_length: usize,
    pub history_size: usize,
    pub max_history_size: usize,
}
impl ReplStats {
    pub fn print(&self) {
        println!("\n📊 REPL Statistics:");
        println!("  Total commands executed: {}", self.total_commands);
        println!("  Average command length: {} characters", self.avg_command_length);
        println!("  History size: {}/{}", self.history_size, self.max_history_size);
        println!();
    }
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_repl_basic_execution() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).expect("Failed to create runtime")));
        let mut repl = Repl::new(runtime);
        let result: _ = repl.execute_code("1 + 1").await.expect("Failed to execute");
        assert_eq!(result.output, "2");
        assert!(!result.is_error);
    }
    #[tokio::test]
    async fn test_repl_error_handling() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).expect("Failed to create runtime")));
        let mut repl = Repl::new(runtime);
        let result: _ = repl.execute_code("invalid syntax {{").await.expect("Failed to execute");
        assert!(result.is_error);
    }
    #[tokio::test]
    async fn test_repl_multiline_detection() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).expect("Failed to create runtime")));
        let mut repl = Repl::new(runtime);
        assert!(repl.is_multiline_start("function foo() {"));
        assert!(repl.is_multiline_start("if (true) {"));
        assert!(!repl.is_multiline_start("console.log('hello')"));
    }
    #[tokio::test]
    async fn test_repl_history() {
        let runtime: _ = Arc::new(Mutex::new(RuntimeLite::new(false).expect("Failed to create runtime")));
        let mut repl = Repl::new(runtime);
        repl.execute_and_record("1 + 1").await.expect("Failed to execute");
        repl.execute_and_record("2 + 2").await.expect("Failed to execute");
        let stats: _ = repl.get_stats();
        assert_eq!(stats.total_commands, 2);
    }
}