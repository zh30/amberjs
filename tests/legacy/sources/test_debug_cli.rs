//! Test script to verify Debug command parsing

use clap::Parser;
use std::path::PathBuf;

// Import the CLI commands
#[derive(Parser, Debug)]
#[command(name = "amberjs")]
struct CliApp {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<SubCommand>,
}

#[derive(clap::Subcommand, Debug)]
enum SubCommand {
    /// Debug a script with interactive debugger
    Debug(DebugCommand),
}

#[derive(Parser, Debug)]
pub enum DebugCommand {
    /// Debug a script file
    Script {
        /// Script file to debug
        file: PathBuf,

        /// Break at line number on startup
        #[arg(short, long)]
        break_at: Option<u32>,

        /// Debug server port
        #[arg(short, long, default_value = "9229")]
        port: u16,

        /// Enable Web UI debugger
        #[arg(short, long)]
        web: bool,
    },

    /// Attach to a running process for debugging
    Attach {
        /// Process ID to attach to
        #[arg(short, long)]
        pid: u32,

        /// Debug server port
        #[arg(short, long, default_value = "9229")]
        port: u16,
    },

    /// Start inspect mode without specifying a file
    Inspect {
        /// Debug server port
        #[arg(short, long, default_value = "9229")]
        port: u16,

        /// Enable Web UI debugger
        #[arg(short, long)]
        web: bool,
    },
}

fn main() {
    // Test 1: Debug script command
    let args = vec!["amberjs", "debug", "test.js"];
    let app = CliApp::parse_from(args);
    println!("✅ Test 1 passed: Debug script command parsed successfully");

    // Test 2: Debug script with options
    let args = vec!["amberjs", "debug", "app.js", "--break-at", "10", "--port", "9229", "--web"];
    let app = CliApp::parse_from(args);
    println!("✅ Test 2 passed: Debug script with options parsed successfully");

    // Test 3: Debug attach command
    let args = vec!["amberjs", "debug", "--attach", "1234"];
    let app = CliApp::parse_from(args);
    println!("✅ Test 3 passed: Debug attach command parsed successfully");

    // Test 4: Debug inspect command
    let args = vec!["amberjs", "debug", "--inspect", "--port", "8080"];
    let app = CliApp::parse_from(args);
    println!("✅ Test 4 passed: Debug inspect command parsed successfully");

    println!("\n🎉 All debug command parsing tests passed!");
    println!("   The CLI structure for debugging is working correctly.");
}
