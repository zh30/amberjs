//! Amber Peripheral Tooling Module
//!
//! Exposes built-in developer tools:
//! - `formatter`: code formatting based on OXC AST
//! - `linter`: static code analysis based on OXC semantic model
//! - `benchmark`: native microbenchmark suite
//! - `compiler`: single binary application compiler
//! - `coverage`: code coverage collector and reporter
//! - `profiler`: CPU profiling and flamegraph export

pub mod benchmark;
pub mod bundler;
pub mod compiler;
pub mod coverage;
pub mod deploy;
pub mod dlx;
pub mod formatter;
pub mod import_map;
pub mod inspector;
pub mod linter;
pub mod lsp;
pub mod profiler;
