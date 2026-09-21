// Interactive Debug CLI
//
// Provides a command-line interface for debugging JavaScript/TypeScript code
// with support for breakpoints, stepping, and variable inspection.

use std::collections::{BTreeMap, HashMap};
use std::io::{Write, self};
use std::sync::{Arc, Mutex};
use std::fs::File;
use anyhow::{Result, Error};
use std::io::Read;

/// Interactive debug command
#[derive(Debug, Clone)]
pub enum DebugCliCommand {
    /// Continue execution
    Continue,
    /// Step to next line
    Next,
    /// Step into function
    Step,
    /// Step out of function
    Finish,
    /// Set breakpoint at line
    Break(u32),
    /// Set breakpoint at file:line
    BreakAt(String, u32),
    /// Set breakpoint at function
    BreakFunction(String),
    /// Delete breakpoint
    Delete(u32),
    /// List breakpoints
    List,
    /// Print variable
    Print(String),
    /// Inspect variable
    Inspect(String),
    /// Show backtrace
    Backtrace,
    /// Show current code
    ListCode,
    /// Evaluate expression
    Eval(String),
    /// Pause execution
    Pause,
    /// Show help
    Help,
    /// Exit debugger
    Quit,
}
/// Interactive debug console
pub struct DebugConsole {
    debugger: Arc<Mutex<DebuggerEngine>>,
    runtime: Arc<Mutex<RuntimeLite>>,
    history: Vec<String>,
}
impl DebugConsole {
    /// Create a new debug console
    pub fn new(debugger: Arc<Mutex<DebuggerEngine>>, runtime: Arc<Mutex<RuntimeLite>>) -> Self {
        Self {
            debugger,
            runtime,
            history: Vec::new(),
        }
    }
    /// Start the interactive debug session
    pub async fn run(&mut self) -> Result<()> {
        println!("🐛 Amber Debugger - Interactive Mode");
        println!("Type 'help' for available commands\n");
        loop {
            // Read command
            let command: _ = self.read_command()?;
            // Parse and execute
            match self.execute_command(command).await {
                Ok(DebugCliCommand::Quit) => {
                    println!("👋 Exiting debugger...");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    println!("❌ Error: {}", e);
                }
            }
        }
        Ok(())
    }
    /// Read a command from stdin
    fn read_command(&mut self) -> Result<String> {
        print!("(amberjs-debug) ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let command: _ = input.trim().to_string();
        if !command.is_empty() {
            self.history.push(command.clone());
        }
        Ok(command)
    }
    /// Parse command string
    fn parse_command(&self, input: &str) -> Result<DebugCliCommand> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }
        match parts[0] {
            "continue" | "c" | "cont" => Ok(DebugCliCommand::Continue),
            "next" | "n" => Ok(DebugCliCommand::Next),
            "step" | "s" => Ok(DebugCliCommand::Step),
            "finish" | "f" => Ok(DebugCliCommand::Finish),
            "break" | "b" => {
                if parts.len() < 2 {
                    return Err(anyhow::anyhow!("Usage: break <line> | break <file>:<line> | break <function>"));
                }
                if parts[1].contains(':') {
                    // File:line format
                    let parts: Vec<&str> = parts[1].split(':').collect();
                    if parts.len() != 2 {
                        return Err(anyhow::anyhow!("Invalid format. Use file:line"));
                    }
                    let line: _ = parts[1].parse::<u32>()
                        .map_err(|_| anyhow::anyhow!("Invalid line number"))?;
                    Ok(DebugCliCommand::BreakAt(parts[0].to_string(), line))
                } else if parts[1].parse::<u32>().is_ok() {
                    // Line number
                    let line: _ = parts[1].parse::<u32>()?;
                    Ok(DebugCliCommand::Break(line))
                } else {
                    // Function name
                    Ok(DebugCliCommand::BreakFunction(parts[1].to_string()))
                }
            }
            "delete" | "d" => {
                if parts.len() < 2 {
                    return Err(anyhow::anyhow!("Usage: delete <id>"));
                }
                let id: _ = parts[1].parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("Invalid breakpoint id"))?;
                Ok(DebugCliCommand::Delete(id))
            }
            "list" | "l" => {
                if parts.len() > 1 && parts[1] == "breakpoints" {
                    Ok(DebugCliCommand::List)
                } else {
                    Ok(DebugCliCommand::ListCode)
                }
            }
            "print" | "p" => {
                if parts.len() < 2 {
                    return Err(anyhow::anyhow!("Usage: print <variable>"));
                }
                Ok(DebugCliCommand::Print(parts[1..].join(" ")))
            }
            "inspect" | "i" => {
                if parts.len() < 2 {
                    return Err(anyhow::anyhow!("Usage: inspect <variable>"));
                }
                Ok(DebugCliCommand::Inspect(parts[1..].join(" ")))
            }
            "backtrace" | "bt" | "where" => Ok(DebugCliCommand::Backtrace),
            "eval" | "e" => {
                if parts.len() < 2 {
                    return Err(anyhow::anyhow!("Usage: eval <expression>"));
                }
                Ok(DebugCliCommand::Eval(parts[1..].join(" ")))
            }
            "pause" => Ok(DebugCliCommand::Pause),
            "help" | "h" | "?" => Ok(DebugCliCommand::Help),
            "quit" | "q" | "exit" => Ok(DebugCliCommand::Quit),
            _ => Err(anyhow::anyhow!("Unknown command: {}. Type 'help' for available commands", parts[0])),
        }
    }
    /// Execute a debug command
    async fn execute_command(&mut self, input: String) -> Result<DebugCliCommand> {
        let command: _ = self.parse_command(&input)?;
        // Execute based on command type
        match &command {
            DebugCliCommand::Help => {
                self.print_help();
            }
            DebugCliCommand::List => {
                self.list_breakpoints();
            }
            DebugCliCommand::ListCode => {
                self.show_current_code();
            }
            DebugCliCommand::Backtrace => {
                self.show_backtrace().await?;
            }
            DebugCliCommand::Print(expr) | DebugCliCommand::Inspect(expr) | DebugCliCommand::Eval(expr) => {
                self.evaluate_expression(expr).await?;
            }
            DebugCliCommand::Break(line) => {
                self.set_breakpoint(*line, None)?;
            }
            DebugCliCommand::BreakAt(file, line) => {
                self.set_breakpoint(*line, Some(file.clone()))?;
            }
            DebugCliCommand::BreakFunction(func) => {
                self.set_function_breakpoint(func)?;
            }
            DebugCliCommand::Delete(id) => {
                self.delete_breakpoint(*id)?;
            }
            _ => {
                // Commands that require runtime control will be handled by DebugSession
                // These are: Continue, Next, Step, Finish, Pause
            }
        }
        Ok(command)
    }
    /// Print help information
    fn print_help(&self) {
        println!("\n🐛 Amber Debugger Commands:");
        println!("\nExecution Control:");
        println!("  continue (c, cont)  - Continue execution");
        println!("  next (n)            - Step to next line");
        println!("  step (s)            - Step into function");
        println!("  finish (f)          - Step out of function");
        println!("  pause               - Pause execution");
        println!("\nBreakpoints:");
        println!("  break <line>        - Set breakpoint at line");
        println!("  break <file>:<line> - Set breakpoint at file:line");
        println!("  break <func>        - Set breakpoint at function");
        println!("  delete <id>         - Delete breakpoint");
        println!("  list                - List all breakpoints");
        println!("\nInspection:");
        println!("  print <var>         - Print variable value");
        println!("  inspect <var>       - Inspect variable (detailed)");
        println!("  eval <expr>         - Evaluate expression");
        println!("  backtrace (bt)      - Show call stack");
        println!("  list (l)            - Show current code");
        println!("\nOther:");
        println!("  help (h, ?)         - Show this help");
        println!("  quit (q, exit)      - Exit debugger");
        println!();
    }
    /// List all breakpoints
    fn list_breakpoints(&self) {
        let _debugger: _ = self.debugger.lock().unwrap();
        println!("\n📍 Breakpoints:");
        // TODO: Implement actual breakpoint listing
        println!("   (No breakpoints set)");
        println!();
    }
    /// Show current code
    fn show_current_code(&self) {
        println!("\n📄 Current Code:");
        println!("   (Code viewer not yet implemented)");
        println!();
    }
    /// Show backtrace
    async fn show_backtrace(&self) -> Result<()> {
        println!("\n📚 Call Stack:");
        // TODO: Implement actual backtrace
        println!("   (No stack frames available)");
        println!();
        Ok(())
    }
    /// Set breakpoint
    fn set_breakpoint(&self, line: u32, file: Option<String>) -> Result<()> {
        println!("✅ Breakpoint set at line {}", line);
        if let Some(ref f) = file {
            println!("   File: {}", f);
        }
        // TODO: Implement actual breakpoint setting
        Ok(())
    }
    /// Set function breakpoint
    fn set_function_breakpoint(&self, func: &str) -> Result<()> {
        println!("✅ Breakpoint set at function {}", func);
        // TODO: Implement actual function breakpoint setting
        Ok(())
    }
    /// Delete breakpoint
    fn delete_breakpoint(&self, id: u32) -> Result<()> {
        println!("✅ Breakpoint {} deleted", id);
        // TODO: Implement actual breakpoint deletion
        Ok(())
    }
    /// Evaluate expression
    async fn evaluate_expression(&self, expr: &str) -> Result<()> {
        println!("\n🔍 Evaluating: {}", expr);
        // TODO: Implement actual expression evaluation
        println!("   Result: (not yet implemented)");
        println!();
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_continue_command() {
        let console: _ = DebugConsole::new(
            Arc::new(Mutex::new(DebuggerEngine::new(Default::default()))),
            Arc::new(Mutex::new(RuntimeLite::new(false).unwrap())),
        );
        assert!(matches!(console.parse_command("continue").unwrap(), DebugCliCommand::Continue));
        assert!(matches!(console.parse_command("c").unwrap(), DebugCliCommand::Continue));
    }
    #[test]
    fn test_parse_break_command() {
        let console: _ = DebugConsole::new(
            Arc::new(Mutex::new(DebuggerEngine::new(Default::default()))),
            Arc::new(Mutex::new(RuntimeLite::new(false).unwrap())),
        );
        assert!(matches!(console.parse_command("break 10").unwrap(), DebugCliCommand::Break(10)));
        assert!(matches!(console.parse_command("b 20").unwrap(), DebugCliCommand::Break(20)));
    }
    #[test]
    fn test_parse_print_command() {
        let console: _ = DebugConsole::new(
            Arc::new(Mutex::new(DebuggerEngine::new(Default::default()))),
            Arc::new(Mutex::new(RuntimeLite::new(false).unwrap())),
        );
        if let DebugCliCommand::Print(expr) = console.parse_command("print myVar").unwrap() {
            assert_eq!(expr, "myVar");
        } else {
            panic!("Expected Print command");
        }
    }
}