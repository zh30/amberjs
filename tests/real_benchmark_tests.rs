// v0.3.363: Real Performance Benchmark Tests
// Actual benchmarks measuring Beejs runtime performance
// For AI-era high-performance JavaScript/TypeScript execution

#[cfg(test)]
mod real_benchmark_tests {
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{Duration, Instant};

    fn beejs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber")
                .unwrap_or_else(|_| "./target/debug/bee".to_string()),
        )
    }

    /// Run `bee eval`. Wall-clock throughput belongs in `benchmarks/`;
    /// `cargo test` on shared CI runners is not a bench machine (PR #30 flake).
    fn eval_script(script: &str) -> (String, Duration) {
        let start = Instant::now();
        let output = Command::new(beejs_path())
            .args(["eval", script])
            .output()
            .expect("Failed to run bee");
        let elapsed = start.elapsed();
        assert!(
            output.status.success(),
            "bee eval failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            elapsed < Duration::from_secs(30),
            "bee eval hung ({elapsed:?})"
        );
        (
            String::from_utf8_lossy(&output.stdout).into_owned(),
            elapsed,
        )
    }

    /// Benchmark: Simple arithmetic operations throughput
    #[test]
    fn benchmark_simple_arithmetic() {
        let script = r#"
            let result = 0;
            const start = performance.now();
            for (let i = 0; i < 1000000; i++) {
                result += i * i + i - i / 2;
            }
            const end = performance.now();
            console.log((end - start).toFixed(2));
        "#;

        let (stdout, elapsed) = eval_script(script);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("Simple arithmetic benchmark:");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time (incl. startup): {:.2?}", elapsed);
        println!(
            "  Throughput: {:.0} ops/sec",
            1_000_000.0 / (exec_time_ms / 1000.0)
        );
    }

    /// Benchmark: String manipulation throughput
    #[test]
    fn benchmark_string_operations() {
        let script = r#"
            let s = "hello";
            const start = performance.now();
            for (let i = 0; i < 100000; i++) {
                s = s + " world " + i + "!";
            }
            const end = performance.now();
            console.log((end - start).toFixed(2));
        "#;

        let (stdout, elapsed) = eval_script(script);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("String operations benchmark:");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time (incl. startup): {:.2?}", elapsed);
    }

    /// Benchmark: Array operations throughput
    #[test]
    fn benchmark_array_operations() {
        let script = r#"
            const arr = [];
            const start = performance.now();
            for (let i = 0; i < 100000; i++) {
                arr.push(i * 2);
            }
            // Filter and map
            const result = arr.filter(x => x % 4 === 0).map(x => x * 2);
            const end = performance.now();
            console.log((end - start).toFixed(2));
        "#;

        let (stdout, elapsed) = eval_script(script);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("Array operations benchmark:");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time (incl. startup): {:.2?}", elapsed);
    }

    /// Benchmark: Object creation throughput
    #[test]
    fn benchmark_object_creation() {
        let script = r#"
            const start = performance.now();
            for (let i = 0; i < 50000; i++) {
                const obj = {
                    id: i,
                    name: "item" + i,
                    value: i * 1.5,
                    nested: { a: 1, b: 2 }
                };
            }
            const end = performance.now();
            console.log((end - start).toFixed(2));
        "#;

        let (stdout, elapsed) = eval_script(script);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("Object creation benchmark:");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time (incl. startup): {:.2?}", elapsed);
    }

    /// Benchmark: Function call overhead
    #[test]
    fn benchmark_function_calls() {
        let script = r#"
            function add(a, b) { return a + b; }
            function multiply(a, b) { return a * b; }
            let result = 0;
            const start = performance.now();
            for (let i = 0; i < 500000; i++) {
                result += add(i, i) + multiply(i, 2);
            }
            const end = performance.now();
            console.log((end - start).toFixed(2));
        "#;

        let (stdout, elapsed) = eval_script(script);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("Function call benchmark:");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time (incl. startup): {:.2?}", elapsed);
        println!(
            "  Throughput: {:.0} calls/sec",
            500_000.0 / (exec_time_ms / 1000.0)
        );
    }

    /// Benchmark: Fibonacci calculation (CPU-intensive)
    #[test]
    fn benchmark_fibonacci() {
        let script = r#"
            function fib(n) {
                if (n <= 1) return n;
                return fib(n - 1) + fib(n - 2);
            }
            const start = performance.now();
            const result = fib(25);
            const end = performance.now();
            console.log((end - start).toFixed(2));
        "#;

        let (stdout, elapsed) = eval_script(script);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("Fibonacci(25) benchmark:");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time (incl. startup): {:.2?}", elapsed);
    }

    /// Benchmark: JSON parsing and serialization
    #[test]
    fn benchmark_json_operations() {
        let script = r#"
            const data = {
                name: "test",
                items: Array.from({length: 1000}, (_, i) => ({id: i, value: "item" + i})),
                nested: { a: 1, b: 2, c: [1, 2, 3] }
            };
            const start = performance.now();
            for (let i = 0; i < 1000; i++) {
                const str = JSON.stringify(data);
                const parsed = JSON.parse(str);
            }
            const end = performance.now();
            console.log((end - start).toFixed(2));
        "#;

        let (stdout, elapsed) = eval_script(script);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("JSON operations benchmark:");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time (incl. startup): {:.2?}", elapsed);
    }

    /// Benchmark: Startup time measurement
    /// Note: In debug mode (cargo test), startup is slower due to debug symbols.
    /// Release mode: < 50ms, Debug mode: < 3s
    #[test]
    fn benchmark_startup_time() {
        let start = Instant::now();
        let output = Command::new(beejs_path())
            .args(["eval", "1 + 1"])
            .output()
            .expect("Failed to run bee");
        let elapsed = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("2"), "Should output '2'");

        println!("Startup time benchmark:");
        println!("  Total time: {:?}", elapsed);

        // Debug mode (cargo test) is slow due to debug symbols; release binary is fast
        // We accept slower times in debug mode since this is just for testing
        assert!(
            elapsed < Duration::from_secs(3),
            "Startup took {:?}, expected < 3s (debug mode is slower)",
            elapsed
        );
    }

    /// Benchmark: setTimeout/setImmediate timing
    #[test]
    fn benchmark_async_timers() {
        let script = r#"
            const start = performance.now();
            let completed = 0;
            const total = 5;
            for (let i = 0; i < total; i++) {
                setImmediate(() => {
                    completed++;
                    if (completed === total) {
                        const end = performance.now();
                        console.log((end - start).toFixed(2));
                    }
                });
            }
        "#;

        let (stdout, elapsed) = eval_script(script);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("Async timers benchmark:");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time (incl. startup): {:.2?}", elapsed);
    }

    /// Benchmark: AI workload simulation (matrix operations)
    #[test]
    fn benchmark_ai_workload_simulation() {
        let script = r#"
            // Simulate AI inference workload: matrix multiply
            const size = 100;
            const a = Array.from({length: size}, () => Array.from({length: size}, () => Math.random()));
            const b = Array.from({length: size}, () => Array.from({length: size}, () => Math.random()));

            const start = performance.now();
            const result = [];
            for (let i = 0; i < size; i++) {
                const row = [];
                for (let j = 0; j < size; j++) {
                    let sum = 0;
                    for (let k = 0; k < size; k++) {
                        sum += a[i][k] * b[k][j];
                    }
                    row.push(sum);
                }
                result.push(row);
            }
            const end = performance.now();
            console.log((end - start).toFixed(2));
        "#;

        let (stdout, elapsed) = eval_script(script);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("AI workload simulation (100x100 matrix multiply):");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time (incl. startup): {:.2?}", elapsed);
    }
}

/// Performance regression detection tests
#[cfg(test)]
mod performance_regression_tests {
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::Duration;

    fn beejs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber")
                .unwrap_or_else(|_| "./target/debug/bee".to_string()),
        )
    }

    /// Baseline performance test - run multiple times to establish baseline
    #[test]
    fn baseline_performance_measurement() {
        let script = r#"for(let i=0; i<100000; i++){}"#;

        let mut times = Vec::new();
        for _ in 0..5 {
            let start = std::time::Instant::now();
            let _output = Command::new(beejs_path())
                .args(["eval", script])
                .output()
                .expect("Failed to run bee");
            let elapsed = start.elapsed();
            times.push(elapsed);
        }

        let avg_time = times.iter().sum::<Duration>() / times.len() as u32;
        let max_time = times.iter().max().unwrap();
        let min_time = times.iter().min().unwrap();

        println!("Baseline performance (5 runs):");
        println!("  Average: {:?}", avg_time);
        println!("  Min: {:?}", min_time);
        println!("  Max: {:?}", max_time);

        let _ratio = max_time.as_secs_f64() / min_time.as_secs_f64();
        assert!(
            *max_time < Duration::from_secs(30),
            "bee eval hung: max = {max_time:?}"
        );
    }

    /// Memory-intensive operation test
    #[test]
    fn memory_intensive_operations() {
        let script = r#"
            const start = performance.now();
            // Create and manipulate large arrays
            let data = [];
            for (let i = 0; i < 10000; i++) {
                data.push({
                    id: i,
                    data: new Float64Array(100).map((_, j) => j * 0.1)
                });
            }
            // Process data
            const sum = data.reduce((acc, item) => acc + item.data.reduce((a, b) => a + b, 0), 0);
            const end = performance.now();
            console.log((end - start).toFixed(2));
        "#;

        let start = std::time::Instant::now();
        let output = Command::new(beejs_path())
            .args(["eval", script])
            .output()
            .expect("Failed to run bee");
        let elapsed = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let exec_time_ms: f64 = stdout.trim().parse().unwrap_or(0.0);

        println!("Memory-intensive operations:");
        println!("  Execution time: {:.2} ms", exec_time_ms);
        println!("  Total time: {:?}", elapsed);

        assert!(
            output.status.success(),
            "bee eval failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            elapsed < std::time::Duration::from_secs(30),
            "bee eval hung ({elapsed:?})"
        );
    }
}

/// Comparative benchmarks (can be extended to compare with Bun/Node.js)
#[cfg(test)]
mod comparative_benchmarks {
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::Instant;

    fn beejs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber")
                .unwrap_or_else(|_| "./target/debug/bee".to_string()),
        )
    }

    /// Compare Beejs startup time with other runtimes
    #[test]
    fn compare_startup_time() {
        let script = "1+1";

        // Measure Beejs startup
        let beejs_start = Instant::now();
        let _ = Command::new(beejs_path())
            .args(["eval", script])
            .output()
            .expect("Failed to run bee");
        let beejs_elapsed = beejs_start.elapsed();

        println!("Startup comparison:");
        println!("  Beejs: {:?}", beejs_elapsed);

        // Debug mode is slower; just verify it completes
        assert!(
            beejs_elapsed < std::time::Duration::from_secs(3),
            "Beejs startup took {:?}, expected < 3s (debug mode is slower)",
            beejs_elapsed
        );
    }

    /// Compare execution throughput
    #[test]
    fn compare_execution_throughput() {
        let script = r#"for(let i=0; i<1000000; i++){1+1}"#;

        let start = Instant::now();
        let _output = Command::new(beejs_path())
            .args(["eval", script])
            .output()
            .expect("Failed to run bee");
        let elapsed = start.elapsed();

        let ops_per_sec = 1_000_000.0 / elapsed.as_secs_f64();

        println!("Execution throughput:");
        println!("  Operations/sec: {:.0}", ops_per_sec);
        println!("  Time: {:?}", elapsed);

        assert!(
            elapsed < std::time::Duration::from_secs(30),
            "bee eval hung ({elapsed:?})"
        );
        let _ = ops_per_sec;
    }
}
