//! Built-in Microbenchmark Runner for Amber (`amber bench`).
//!
//! Provides high-resolution execution timing, iteration warmups, and ops/sec statistics.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub total_millis: f64,
    pub avg_micros: f64,
    pub ops_per_sec: f64,
}

/// Helper to check if a file is a benchmark script.
pub fn is_benchmark_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    (name.contains(".bench.")
        || name.contains("_bench.")
        || name.ends_with("bench.js")
        || name.ends_with("bench.ts"))
        && !name.starts_with('.')
}

/// Finds benchmark files in given paths.
pub fn discover_benchmark_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let roots = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.to_vec()
    };

    for root in roots {
        if root.is_file() {
            if is_benchmark_file(&root) {
                files.push(root);
            }
        } else if root.is_dir() {
            for entry in WalkDir::new(root)
                .into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.')
                        && name != "node_modules"
                        && name != "target"
                        && name != "dist"
                })
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if is_benchmark_file(path) {
                    files.push(path.to_path_buf());
                }
            }
        }
    }
    files
}

/// JS bootstrap harness to inject `amber.bench` and `bench` globals into the runtime.
pub const BENCHMARK_HARNESS_JS: &str = r#"
globalThis.__benchmarks = [];
globalThis.bench = function(name, fn) {
    globalThis.__benchmarks.push({ name, fn });
};
if (typeof globalThis.amber === 'undefined') {
    globalThis.amber = {};
}
globalThis.amber.bench = globalThis.bench;
"#;

/// Runs a single benchmark file and collects results.
pub fn run_benchmark_file(file_path: &Path) -> Result<Vec<BenchmarkResult>> {
    let source = fs::read_to_string(file_path).map_err(|e| {
        anyhow!(
            "Failed to read benchmark file '{}': {}",
            file_path.display(),
            e
        )
    })?;

    let file_str = file_path.to_string_lossy();
    let user_code = if file_path
        .extension()
        .map_or(false, |ext| ext == "ts" || ext == "tsx")
    {
        crate::typescript::compile_typescript(&source, &file_str)
            .map_err(|e| anyhow!("TS transpile failed: {}", e))?
            .js_code
    } else {
        source
    };

    let mut runtime = crate::runtime_minimal::MinimalRuntime::new()
        .map_err(|e| anyhow!("Failed to initialize runtime: {}", e))?;

    // Inject harness
    runtime
        .execute_code(BENCHMARK_HARNESS_JS)
        .map_err(|e| anyhow!("Failed to initialize benchmark harness: {}", e))?;

    // Execute user benchmark file (which calls bench(name, fn))
    runtime.execute_code(&user_code).map_err(|e| {
        anyhow!(
            "Failed to execute benchmark script '{}': {}",
            file_path.display(),
            e
        )
    })?;

    // Query benchmark count
    let count_code = "globalThis.__benchmarks.length;";
    let count_str = runtime
        .execute_code(count_code)
        .map_err(|e| anyhow!("Failed to query benchmarks: {}", e))?;
    let count: usize = count_str.trim().parse().unwrap_or(0);

    if count == 0 {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    for i in 0..count {
        // Extract benchmark name
        let name_code = format!("globalThis.__benchmarks[{}].name;", i);
        let name = runtime
            .execute_code(&name_code)
            .unwrap_or_else(|_| format!("benchmark_{}", i));
        let name = name.trim().trim_matches('"').trim_matches('\'').to_string();

        // Warmup (10 runs)
        let warmup_code = format!(
            r#"
            for (let _w = 0; _w < 10; _w++) {{
                globalThis.__benchmarks[{}].fn();
            }}
            "#,
            i
        );
        let _ = runtime.execute_code(&warmup_code);

        // Run measurement loop (aim for ~1000 iterations or 100ms)
        let iterations = 1000;
        let start = Instant::now();
        let loop_code = format!(
            r#"
            for (let _i = 0; _i < {}; _i++) {{
                globalThis.__benchmarks[{}].fn();
            }}
            "#,
            iterations, i
        );

        runtime
            .execute_code(&loop_code)
            .map_err(|e| anyhow!("Benchmark '{}' failed during execution: {}", name, e))?;

        let duration = start.elapsed();
        let total_millis = duration.as_secs_f64() * 1000.0;
        let avg_micros = (duration.as_secs_f64() * 1_000_000.0) / (iterations as f64);
        let ops_per_sec = (iterations as f64) / duration.as_secs_f64();

        results.push(BenchmarkResult {
            name,
            iterations,
            total_millis,
            avg_micros,
            ops_per_sec,
        });
    }

    Ok(results)
}

/// Prints formatted benchmark table.
pub fn print_benchmark_table(file_name: &str, results: &[BenchmarkResult]) {
    println!("\n📊 Benchmarks in {}:", file_name);
    println!("{:-<75}", "");
    println!(
        "{:<32} {:>12} {:>14} {:>14}",
        "Benchmark", "Iterations", "Avg Time", "Throughput"
    );
    println!("{:-<75}", "");

    for r in results {
        let avg_fmt = if r.avg_micros < 1.0 {
            format!("{:.2} ns", r.avg_micros * 1000.0)
        } else if r.avg_micros < 1000.0 {
            format!("{:.2} µs", r.avg_micros)
        } else {
            format!("{:.2} ms", r.avg_micros / 1000.0)
        };

        let ops_fmt = if r.ops_per_sec >= 1_000_000.0 {
            format!("{:.2} M ops/s", r.ops_per_sec / 1_000_000.0)
        } else if r.ops_per_sec >= 1_000.0 {
            format!("{:.2} K ops/s", r.ops_per_sec / 1_000.0)
        } else {
            format!("{:.0} ops/s", r.ops_per_sec)
        };

        println!(
            "{:<32} {:>12} {:>14} {:>14}",
            r.name, r.iterations, avg_fmt, ops_fmt
        );
    }
    println!("{:-<75}\n", "");
}
