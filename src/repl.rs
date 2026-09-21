//! Enhanced Interactive REPL for Amber (`amber repl`).
//!
//! Powered by `rustyline` for rich line editing, history persistence (~/.amberjs_history),
//! multi-line input detection, and full V8 context support via `MinimalRuntime`.

use anyhow::{anyhow, Result};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::Write;

/// REPL configuration
#[derive(Clone, Debug)]
pub struct ReplConfig {
    pub show_result: bool,
    pub show_time: bool,
    pub prompt: String,
    pub continuation_prompt: String,
    pub enable_history: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            show_result: true,
            show_time: false,
            prompt: "amber> ".to_string(),
            continuation_prompt: "...   ".to_string(),
            enable_history: true,
        }
    }
}

pub struct Repl {
    config: ReplConfig,
}

impl Repl {
    pub fn new(config: ReplConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(ReplConfig::default())
    }

    pub fn run(&mut self, verbose: bool) -> Result<()> {
        run_interactive_repl_with_config(&self.config, verbose)
    }
}

/// Run interactive REPL with default configuration
pub fn run_interactive_repl(verbose: bool) -> Result<()> {
    Repl::with_defaults().run(verbose)
}

pub fn run_interactive_repl_with_config(config: &ReplConfig, verbose: bool) -> Result<()> {
    println!("🐝 Amber REPL - High-performance JavaScript/TypeScript shell");
    println!("Type JavaScript code, '.help' for commands, '.exit' or Ctrl+D to quit.");
    println!();

    let mut runtime = crate::runtime_minimal::MinimalRuntime::new()
        .map_err(|e| anyhow!("Failed to initialize runtime: {}", e))?;

    let mut rl = DefaultEditor::new()
        .map_err(|e| anyhow!("Failed to initialize terminal readline editor: {}", e))?;

    let history_path = dirs::home_dir().map(|h| h.join(".amberjs_history"));
    if config.enable_history {
        if let Some(ref path) = history_path {
            let _ = rl.load_history(path);
        }
    }

    let mut multiline_buf = String::new();

    loop {
        let prompt = if multiline_buf.is_empty() {
            &config.prompt
        } else {
            &config.continuation_prompt
        };

        match rl.readline(prompt) {
            Ok(line) => {
                let trimmed = line.trim();

                // Check special commands when not inside a multiline block
                if multiline_buf.is_empty() {
                    if trimmed == ".exit" || trimmed == ".quit" || trimmed == ".q" {
                        println!("Goodbye! 👋");
                        break;
                    } else if trimmed == ".clear" || trimmed == ".cls" {
                        print!("\x1B[2J\x1B[1;1H");
                        let _ = std::io::stdout().flush();
                        continue;
                    } else if trimmed == ".help" || trimmed == ".h" {
                        println!("Amber REPL Commands:");
                        println!("  .exit, .quit, .q   Exit the REPL session");
                        println!("  .clear, .cls       Clear the console screen");
                        println!("  .help, .h          Show this help message");
                        println!();
                        println!("Keyboard Shortcuts:");
                        println!("  Up/Down            Recall command history");
                        println!("  Ctrl+C             Cancel current input line");
                        println!("  Ctrl+D             Exit the REPL");
                        continue;
                    }
                }

                if trimmed.is_empty() && multiline_buf.is_empty() {
                    continue;
                }

                if !multiline_buf.is_empty() {
                    multiline_buf.push('\n');
                }
                multiline_buf.push_str(&line);

                if is_unclosed_block(&multiline_buf) {
                    continue;
                }

                let full_code = std::mem::take(&mut multiline_buf);
                if config.enable_history {
                    let _ = rl.add_history_entry(&full_code);
                }

                let start = std::time::Instant::now();
                match runtime.execute_code(&full_code) {
                    Ok(result) => {
                        let trimmed_res = result.trim();
                        if config.show_result
                            && !trimmed_res.is_empty()
                            && trimmed_res != "undefined"
                        {
                            println!("{}", trimmed_res);
                        }
                        if config.show_time || verbose {
                            println!("⏱  {:.2}ms", start.elapsed().as_secs_f64() * 1000.0);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                if !multiline_buf.is_empty() {
                    multiline_buf.clear();
                    println!("(cancelled)");
                } else {
                    println!("^C (Type .exit or Ctrl+D to quit)");
                }
            }
            Err(ReadlineError::Eof) => {
                println!("\nGoodbye! 👋");
                break;
            }
            Err(err) => {
                eprintln!("Readline error: {:?}", err);
                break;
            }
        }
    }

    if config.enable_history {
        if let Some(ref path) = history_path {
            let _ = rl.save_history(path);
        }
    }

    Ok(())
}

/// Detects if the current buffer has open brackets, braces, parentheses, or string literals.
pub fn is_unclosed_block(code: &str) -> bool {
    let mut parens = 0isize;
    let mut braces = 0isize;
    let mut brackets = 0isize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_backtick = false;
    let mut prev = '\0';

    for ch in code.chars() {
        if prev == '\\' {
            prev = ch;
            continue;
        }
        match ch {
            '\x27' if !in_double_quote && !in_backtick => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && !in_backtick => in_double_quote = !in_double_quote,
            '`' if !in_single_quote && !in_double_quote => in_backtick = !in_backtick,
            '(' if !in_single_quote && !in_double_quote && !in_backtick => parens += 1,
            ')' if !in_single_quote && !in_double_quote && !in_backtick => parens -= 1,
            '{' if !in_single_quote && !in_double_quote && !in_backtick => braces += 1,
            '}' if !in_single_quote && !in_double_quote && !in_backtick => braces -= 1,
            '[' if !in_single_quote && !in_double_quote && !in_backtick => brackets += 1,
            ']' if !in_single_quote && !in_double_quote && !in_backtick => brackets -= 1,
            _ => {}
        }
        prev = ch;
    }

    parens > 0 || braces > 0 || brackets > 0 || in_backtick || code.trim_end().ends_with('\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiline_block_detection() {
        assert!(is_unclosed_block("function foo() {"));
        assert!(is_unclosed_block("const a = [1, 2,"));
        assert!(is_unclosed_block("const msg = `hello"));
        assert!(!is_unclosed_block("function foo() { return 1; }"));
        assert!(!is_unclosed_block("const a = 1 + 2;"));
    }
}
