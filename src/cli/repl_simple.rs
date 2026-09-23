//! Simple REPL implementation using MinimalRuntime
//! This is a basic REPL for quick testing and interaction

use anyhow::Result;
use std::io::{self, Write};
use std::sync::Arc;
use crate::runtime_minimal::MinimalRuntime;

/// Simple REPL configuration
#[derive(Debug, Clone)]
pub struct SimpleReplConfig {
    /// Prompt to display
    pub prompt: String,
    /// Maximum history size
    pub history_size: usize,
}

impl Default for SimpleReplConfig {
    fn default() -> Self {
        Self {
            prompt: "amber> ".to_string(),
            history_size: 100,
        }
    }
}

/// Simple REPL implementation
pub struct SimpleRepl {
    /// Runtime to execute code
    runtime: Arc<Mutex<MinimalRuntime>>,
    /// Configuration
    config: SimpleReplConfig,
}

impl SimpleRepl {
    /// Create a new simple REPL
    pub fn new() -> Result<Self> {
        let runtime = MinimalRuntime::new()?;
        Ok(Self {
            runtime: Arc::new(Mutex::new(runtime)),
            config: SimpleReplConfig::default(),
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: SimpleReplConfig) -> Result<Self> {
        let runtime = MinimalRuntime::new()?;
        Ok(Self {
            runtime: Arc::new(Mutex::new(runtime)),
            config,
        })
    }

    /// Run the REPL
    pub fn run(&mut self) -> Result<()> {
        println!("🐝 Amber REPL - High-performance JavaScript/TypeScript runtime");
        println!("Built with Rust + V8 (minimal edition)");
        println!("Type JavaScript code and press Enter to execute");
        println!("Type .exit or Ctrl+C to quit");
        println!("Type .help for more information");
        println!();

        let mut buffer = String::new();

        loop {
            // Display prompt
            print!("{}", self.config.prompt);
            io::stdout().flush()?;

            // Read input
            buffer.clear();
            match io::stdin().read_line(&mut buffer) {
                Ok(_) => {
                    let input = buffer.trim();
                    if input.is_empty() {
                        continue;
                    }

                    // Handle special commands
                    if input.starts_with('.') {
                        self.handle_command(input)?;
                        continue;
                    }

                    // Execute JavaScript code
                    let result = self.execute_code(input);
                    match result {
                        Ok(output) => {
                            if !output.trim().is_empty() {
                                println!("{}", output);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
                Err(_) => {
                    println!("\nExiting REPL...");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle special REPL commands
    fn handle_command(&mut self, command: &str) -> Result<()> {
        match command {
            ".help" => {
                println!("Available commands:");
                println!("  .help     - Show this help message");
                println!("  .exit     - Exit the REPL");
                println!("  .clear    - Clear the screen");
                println!("  .info     - Show runtime information");
            }
            ".exit" => {
                println!("Exiting REPL...");
                std::process::exit(0);
            }
            ".clear" => {
                // Clear screen (works on most terminals)
                print!("\x1B[2J\x1B[H");
                io::stdout().flush()?;
            }
            ".info" => {
                println!("Amber Runtime Information:");
                println!("  Version: 0.1.3");
                println!("  Engine: V8 (rusty_v8)");
                println!("  Runtime: MinimalRuntime");
                println!("  Features: Basic JavaScript execution");
            }
            _ => {
                println!("Unknown command: {}", command);
                println!("Type .help for available commands");
            }
        }
        Ok(())
    }

    /// Execute JavaScript code
    fn execute_code(&self, code: &str) -> Result<String> {
        let runtime = self.runtime.lock().map_err(|e| anyhow::anyhow!("Failed to lock runtime: {}", e))?;
        runtime.execute_code(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_repl_creation() {
        let repl = SimpleRepl::new();
        assert!(repl.is_ok());
    }

    #[test]
    fn test_simple_repl_with_config() {
        let config = SimpleReplConfig {
            prompt: "test> ".to_string(),
            history_size: 50,
        };
        let repl = SimpleRepl::with_config(config);
        assert!(repl.is_ok());
    }
}
