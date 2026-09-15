// V8 Snapshot CoW & Isolate Prewarmer Integration Tests (Task 1.2 / v1.13.0)
use beejs::isolate_prewarmer::{global_prewarmer, IsolatePrewarmer};
use beejs::runtime_minimal::MinimalRuntime;
use beejs::v8_snapshot::{
    cached_startup_blob, enable_startup_snapshot_for_cli, is_startup_blob_cow_active,
};
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

#[test]
#[serial]
fn test_snapshot_cow_mmap_zero_copy() {
    enable_startup_snapshot_for_cli();
    let blob = cached_startup_blob();
    assert!(
        blob.is_some(),
        "cached_startup_blob should return snapshot slice"
    );
    let slice = blob.unwrap();
    assert!(
        slice.len() > 1024,
        "Startup snapshot blob should be substantial (found {} bytes)",
        slice.len()
    );
    assert!(
        is_startup_blob_cow_active(),
        "Startup blob should be actively loaded via CoW"
    );
}

#[test]
#[serial]
fn test_minimal_runtime_prewarm() {
    let mut runtime = MinimalRuntime::new_fast().expect("Failed to create fast runtime");
    assert!(
        runtime.prewarm().is_ok(),
        "MinimalRuntime prewarm should succeed"
    );

    // Code execution should be instant on pre-warmed runtime
    let start = Instant::now();
    let res = runtime.execute_code("1 + 2 + 3 + 4 + 5");
    let elapsed = start.elapsed();
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "15");
    println!("Prewarmed execution latency: {:.2}µs", elapsed.as_micros());

    // Extended APIs like Blob, TextEncoder, crypto should be ready
    let web_res = runtime
        .execute_code("typeof crypto.getRandomValues === 'function' && typeof Blob === 'function'");
    assert!(web_res.is_ok());
    assert_eq!(web_res.unwrap(), "true");
}

#[test]
#[serial]
fn test_isolate_prewarmer_pool_acquire() {
    let prewarmer = IsolatePrewarmer::new();
    let count = prewarmer.prewarm().expect("Prewarmer should prewarm");
    assert_eq!(count, 1);
    assert_eq!(prewarmer.available_count(), 1);
    assert!(prewarmer.is_ready());

    // Acquire first isolate from pool (Hit)
    {
        let mut rt1 = prewarmer.acquire().expect("Should acquire from pool");
        let res1 = rt1.execute_code("const x = 10; x * 2;");
        assert!(res1.is_ok());
        assert_eq!(res1.unwrap(), "20");
        assert_eq!(prewarmer.stats().cache_hits.load(Ordering::Relaxed), 1);
    }

    // Acquire second isolate (Miss, on demand)
    {
        let mut rt2 = prewarmer.acquire().expect("Should acquire from pool");
        let res2 = rt2.execute_code("Math.pow(2, 8)");
        assert!(res2.is_ok());
        assert_eq!(res2.unwrap(), "256");
        assert_eq!(prewarmer.stats().cache_misses.load(Ordering::Relaxed), 1);
    }
}

#[test]
#[serial]
fn test_prewarmed_isolation_integrity() {
    let prewarmer = IsolatePrewarmer::new();
    prewarmer.prewarm().expect("Failed to prewarm");

    // Acquire rt1, execute code with global modification, and drop rt1
    {
        let mut rt1 = prewarmer.acquire().expect("Failed to acquire rt1");
        let set_res = rt1.execute_code(
            "globalThis.__secret_flag_123__ = 'rt1_infected'; globalThis.__secret_flag_123__",
        );
        assert!(set_res.is_ok(), "rt1 execution failed: {:?}", set_res.err());
        assert_eq!(set_res.unwrap(), "rt1_infected");
    }

    // Acquire rt2, verify pristine environment
    {
        let mut rt2 = prewarmer.acquire().expect("Failed to acquire rt2");
        let check_res = rt2.execute_code("typeof globalThis.__secret_flag_123__");
        assert!(
            check_res.is_ok(),
            "rt2 execution failed: {:?}",
            check_res.err()
        );
        assert_eq!(
            check_res.unwrap(),
            "undefined",
            "Prewarmed isolates must have completely isolated global state"
        );
    }
}

#[test]
#[serial]
fn test_concurrent_multi_thread_checkout() {
    let num_threads = 4;
    let completed = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    for thread_idx in 0..num_threads {
        let completed_clone = Arc::clone(&completed);
        let handle = thread::Builder::new()
            .stack_size(4 * 1024 * 1024)
            .spawn(move || {
                let prewarmer = global_prewarmer();
                let mut runtime = prewarmer.acquire().expect("Thread acquire should succeed");
                let script = format!(
                    "function work(n) {{ return n * {}; }} work(7);",
                    thread_idx + 1
                );
                let res = runtime
                    .execute_code(&script)
                    .expect("Execution should succeed");
                let expected = (7 * (thread_idx + 1)).to_string();
                assert_eq!(res, expected);
                completed_clone.fetch_add(1, Ordering::SeqCst);
            })
            .expect("Failed to spawn thread");
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("Worker thread panicked");
    }

    assert_eq!(completed.load(Ordering::SeqCst), num_threads);
}

#[test]
#[serial]
fn test_prewarmed_execution_latency_benchmark() {
    let handle = thread::Builder::new()
        .stack_size(4 * 1024 * 1024)
        .spawn(|| {
            let prewarmer = global_prewarmer();
            let mut rt = prewarmer
                .acquire()
                .expect("Failed to acquire prewarmed isolate");

            let start = Instant::now();
            let res = rt.execute_code("const a = 12345; const b = 67890; a + b");
            let elapsed = start.elapsed();

            assert!(res.is_ok(), "Execution failed: {:?}", res.err());
            assert_eq!(res.unwrap(), "80235");

            let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
            println!(
                "⚡ [BENCHMARK] Pre-warmed Isolate execution latency: {:.2}µs ({:.4}ms)",
                elapsed.as_micros(),
                elapsed_ms
            );

            let max_allowed_ms = if cfg!(debug_assertions) { 25.0 } else { 1.5 };
            assert!(
                elapsed_ms < max_allowed_ms,
                "Prewarmed execution latency should be under {}ms (actual: {:.2}ms)",
                max_allowed_ms,
                elapsed_ms
            );
        })
        .expect("Failed to spawn thread");
    handle.join().expect("Thread panicked");
}
