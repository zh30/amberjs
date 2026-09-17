use anyhow::{Context, Result};
use amberjs::runtime_minimal::MinimalRuntime;
use serde_json::json;
use std::hint::black_box;
use std::time::{Duration, Instant};

const DEFAULT_ITERATIONS: usize = 100;
const DEFAULT_WARMUP_ITERATIONS: usize = 10;

#[derive(Debug)]
struct Summary {
    mean_ns: u128,
    p50_ns: u128,
    p95_ns: u128,
    min_ns: u128,
    max_ns: u128,
}

fn iteration_count(variable: &str, default: usize) -> Result<usize> {
    let Some(value) = std::env::var_os(variable) else {
        return Ok(default);
    };

    let value = value
        .to_str()
        .with_context(|| format!("{variable} must be valid UTF-8"))?;
    let parsed = value
        .parse::<usize>()
        .with_context(|| format!("{variable} must be a positive integer"))?;

    if parsed == 0 {
        anyhow::bail!("{variable} must be greater than zero");
    }

    Ok(parsed)
}

fn measure<F>(iterations: usize, mut operation: F) -> Result<Summary>
where
    F: FnMut() -> Result<()>,
{
    let mut samples = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let started = Instant::now();
        operation()?;
        samples.push(started.elapsed());
    }

    Ok(summarize(&mut samples))
}

fn summarize(samples: &mut [Duration]) -> Summary {
    samples.sort_unstable();

    let len = samples.len();
    let p50_index = (len - 1) / 2;
    let p95_index = ((len * 95).div_ceil(100)).saturating_sub(1);
    let total_ns = samples.iter().map(Duration::as_nanos).sum::<u128>();

    Summary {
        mean_ns: total_ns / len as u128,
        p50_ns: samples[p50_index].as_nanos(),
        p95_ns: samples[p95_index].as_nanos(),
        min_ns: samples[0].as_nanos(),
        max_ns: samples[len - 1].as_nanos(),
    }
}

fn summary_json(summary: &Summary) -> serde_json::Value {
    json!({
        "mean_ns": summary.mean_ns,
        "p50_ns": summary.p50_ns,
        "p95_ns": summary.p95_ns,
        "min_ns": summary.min_ns,
        "max_ns": summary.max_ns,
    })
}

fn main() -> Result<()> {
    let iterations = iteration_count("AMBER_BENCH_ITERATIONS", DEFAULT_ITERATIONS)?;
    let warmup_iterations = iteration_count("AMBER_BENCH_WARMUP", DEFAULT_WARMUP_ITERATIONS)?;

    let initialize_started = Instant::now();
    amberjs::initialize_v8()?;
    let initialize_v8_ns = initialize_started.elapsed().as_nanos();

    for _ in 0..warmup_iterations {
        let mut runtime = MinimalRuntime::new()?;
        black_box(runtime.execute_code(black_box("1"))?);
    }

    let runtime_create = measure(iterations, || {
        black_box(MinimalRuntime::new()?);
        Ok(())
    })?;

    let runtime_create_and_first_execute = measure(iterations, || {
        let mut runtime = MinimalRuntime::new()?;
        black_box(runtime.execute_code(black_box("1"))?);
        Ok(())
    })?;

    let mut reused_runtime = MinimalRuntime::new()?;
    black_box(reused_runtime.execute_code(black_box("1"))?);
    let runtime_reused_execute = measure(iterations, || {
        black_box(reused_runtime.execute_code(black_box("1"))?);
        Ok(())
    })?;

    let estimated_first_context_setup_p50_ns = runtime_create_and_first_execute
        .p50_ns
        .saturating_sub(runtime_create.p50_ns);

    let report = json!({
        "iterations": iterations,
        "warmup_iterations": warmup_iterations,
        "initialize_v8_ns": initialize_v8_ns,
        "runtime_create": summary_json(&runtime_create),
        "runtime_create_and_first_execute": summary_json(&runtime_create_and_first_execute),
        "estimated_first_context_setup_p50_ns": estimated_first_context_setup_p50_ns,
        "runtime_reused_execute": summary_json(&runtime_reused_execute),
    });

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
