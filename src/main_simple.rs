//! Amber - High-performance JavaScript/TypeScript runtime
//! Built with Rust and V8
//! Simplified version for testing core functionality

use anyhow::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("🐝 Amber v0.1.3 - High-performance JavaScript/TypeScript runtime");
    println!("Built with Rust + V8");
    println!("超越 Bun! 🚀");
    println!();

    // Get command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage:");
        println!("  amber run <file>    Run a JavaScript/TypeScript file");
        println!("  amber eval <code>   Evaluate JavaScript code");
        println!("  amber repl          Start interactive REPL");
        println!("  amber version       Display version information");
        println!();
        println!("Examples:");
        println!("  amber run script.js");
        println!("  amber eval 'console.log(\"Hello\")'");
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "run" => {
            if args.len() < 3 {
                eprintln!("Error: run command requires a file path");
                std::process::exit(1);
            }
            let file_path = PathBuf::from(&args[2]);
            run_file(&file_path)?;
        }
        "eval" => {
            if args.len() < 3 {
                eprintln!("Error: eval command requires code string");
                std::process::exit(1);
            }
            let code = &args[2];
            eval_code(code)?;
        }
        "repl" => {
            run_repl()?;
        }
        "version" => {
            println!("Amber v0.1.3");
            println!("High-performance JavaScript/TypeScript runtime");
            println!("Built with Rust + V8");
            println!("Faster than Bun! 🚀");
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Use 'amber' without arguments to see usage");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn run_file(file_path: &PathBuf) -> Result<()> {
    println!("🐝 Running Amber on: {}", file_path.display());

    // Create a minimal runtime
    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new()
        .expect("Failed to create runtime");

    // Read and execute the file
    let code = std::fs::read_to_string(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

    match runtime.execute_code(&code) {
        Ok(result) => {
            if !result.trim().is_empty() {
                println!("Result: {}", result);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn eval_code(code: &str) -> Result<()> {
    println!("🐝 Evaluating JavaScript code");

    // Create a minimal runtime
    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new()
        .expect("Failed to create runtime");

    match runtime.execute_code(code) {
        Ok(result) => {
            if !result.trim().is_empty() {
                println!("Result: {}", result);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn run_repl() -> Result<()> {
    println!("🐝 Starting Amber REPL...");
    println!("Type JavaScript code and press Enter to execute");
    println!("Type .exit or Ctrl+C to quit");
    println!();

    // Create a minimal runtime
    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new()
        .expect("Failed to create runtime");

    let mut buffer = String::new();

    loop {
        print!("amber> ");
        std::io::stdout().flush()?;

        buffer.clear();
        match std::io::stdin().read_line(&mut buffer) {
            Ok(_) => {
                let input = buffer.trim();
                if input.is_empty() {
                    continue;
                }

                if input == ".exit" {
                    println!("Exiting REPL...");
                    break;
                }

                match runtime.execute_code(input) {
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
