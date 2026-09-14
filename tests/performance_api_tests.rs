// v0.3.275: Performance API integration tests
// Tests for performance.now(), performance.mark(), performance.measure(), etc.

#[cfg(test)]
mod performance_api_tests {
    use serial_test::serial;
    use std::path::PathBuf;
    use std::process::Command;

    fn beejs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_bee").unwrap_or_else(|_| "./target/debug/bee".to_string()),
        )
    }

    #[test]
    fn test_performance_now_exists() {
        let output = Command::new(beejs_path())
            .args(["eval", "console.log(typeof performance.now)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "performance.now should exist");
    }

    #[test]
    fn test_performance_now_returns_number() {
        let output = Command::new(beejs_path())
            .args(["eval", "const n = performance.now(); console.log(typeof n)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("number"),
            "performance.now should return a number"
        );
    }

    #[test]
    fn test_performance_now_monotonic() {
        let output = Command::new(beejs_path())
            .args(["eval", "const n1 = performance.now(); for(let i=0; i<1000; i++){} const n2 = performance.now(); console.log(n2 >= n1)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "performance.now should be monotonic"
        );
    }

    #[test]
    fn test_performance_now_is_relative_to_time_origin() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const origin = performance.timeOrigin;
                const first = performance.now();
                const second = performance.now();
                const wallClockDelta = Math.abs(Date.now() - (origin + second));
                console.log(
                    origin > 1700000000000,
                    first >= 0,
                    first < 10000,
                    second >= first,
                    wallClockDelta < 10000
                );
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stdout.contains("true true true true true"),
            "performance.now should be timeOrigin-relative; stdout: {stdout}; stderr: {stderr}"
        );
    }

    #[test]
    fn test_performance_time_origin_exists() {
        let output = Command::new(beejs_path())
            .args(["eval", "console.log(typeof performance.timeOrigin)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("number"),
            "performance.timeOrigin should exist"
        );
    }

    #[test]
    fn test_performance_time_origin_reasonable() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                "console.log(performance.timeOrigin > 1700000000000)",
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "timeOrigin should be a reasonable Unix timestamp"
        );
    }

    #[test]
    fn test_performance_mark() {
        let output = Command::new(beejs_path())
            .args(["eval", "performance.mark('test'); const entries = performance.getEntriesByName('test'); console.log(entries.length)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("1"),
            "performance.mark should create an entry"
        );
    }

    #[test]
    fn test_performance_measure() {
        let output = Command::new(beejs_path())
            .args(["eval", "performance.mark('start'); performance.mark('end'); performance.measure('test', 'start', 'end'); const entries = performance.getEntriesByName('test'); console.log(entries.length, entries[0]?.entryType)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("1"),
            "performance.measure should create an entry"
        );
        assert!(
            stdout.contains("measure"),
            "measure entry should have entryType 'measure'"
        );
    }

    #[test]
    fn test_performance_measure_duration() {
        let output = Command::new(beejs_path())
            .args(["eval", "performance.mark('start'); for(let i=0; i<100000; i++){} performance.mark('end'); performance.measure('test', 'start', 'end'); const entries = performance.getEntriesByName('test'); console.log(entries[0]?.duration > 0)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "measure should have positive duration"
        );
    }

    #[test]
    fn test_performance_get_entries() {
        let output = Command::new(beejs_path())
            .args(["eval", "performance.mark('a'); performance.mark('b'); const entries = performance.getEntries(); console.log(entries.length)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("2"), "getEntries should return all entries");
    }

    #[test]
    fn test_performance_get_entries_by_type() {
        let output = Command::new(beejs_path())
            .args(["eval", "performance.mark('a'); performance.measure('test', 'a', 'a'); const marks = performance.getEntriesByType('mark'); const measures = performance.getEntriesByType('measure'); console.log(marks.length, measures.length)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("1 1"),
            "getEntriesByType should filter by type"
        );
    }

    #[test]
    fn test_performance_clear_marks() {
        let output = Command::new(beejs_path())
            .args(["eval", "performance.mark('test'); performance.clearMarks(); const entries = performance.getEntriesByName('test'); console.log(entries.length)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("0"),
            "clearMarks should remove mark entries"
        );
    }

    #[test]
    fn test_performance_clear_measures() {
        let output = Command::new(beejs_path())
            .args(["eval", "performance.mark('start'); performance.measure('test', 'start', 'start'); performance.clearMeasures(); const entries = performance.getEntriesByType('measure'); console.log(entries.length)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("0"),
            "clearMeasures should remove measure entries"
        );
    }

    #[test]
    fn test_performance_to_json() {
        let output = Command::new(beejs_path())
            .args(["eval", "const json = performance.toJSON(); console.log(typeof json, typeof json.now, typeof json.timeOrigin)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("object number number"),
            "toJSON should return an object with now and timeOrigin"
        );
    }

    #[test]
    fn test_performance_measure_without_marks() {
        let output = Command::new(beejs_path())
            .args(["eval", "performance.measure('test'); const entries = performance.getEntriesByName('test'); console.log(entries.length, entries[0]?.startTime === 0, entries[0]?.duration >= 0)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("1 true true"),
            "measure should work without explicit marks from the performance timeline origin"
        );
    }

    #[test]
    fn test_performance_precision() {
        let output = Command::new(beejs_path())
            .args(["eval", "const n1 = performance.now(); const n2 = performance.now(); const diff = n2 - n1; console.log(typeof diff, diff >= 0)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should return a number >= 0
        assert!(
            stdout.contains("number true"),
            "performance.now should return a number >= 0"
        );
    }

    #[test]
    fn test_performance_timing_ai_workload() {
        // Test typical AI workload timing scenario
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                performance.mark('model_load_start');
                // Simulate model inference with simple computation
                let result = 0;
                for(let i=0; i<500000; i++) { result += i * i; }
                performance.mark('model_load_end');
                performance.measure('model_load', 'model_load_start', 'model_load_end');
                const entries = performance.getEntriesByName('model_load');
                console.log('AI inference time:', entries[0]?.duration.toFixed(2), 'ms');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("ms"), "Should measure AI workload timing");
    }

    #[test]
    #[serial]
    fn test_web_api_init_uses_real_performance_timeline() {
        beejs::initialize_v8().expect("V8 should initialize");

        let mut isolate = rusty_v8::Isolate::new(Default::default());
        rusty_v8::scope!(let scope, &mut isolate);
        let context = rusty_v8::Context::new(scope, Default::default());
        let scope = &mut rusty_v8::ContextScope::new(scope, context);

        beejs::web_api::init_web_api(scope, &context).expect("web APIs should initialize");

        let source = rusty_v8::String::new(
            scope,
            r#"
            performance.mark('web-init-mark');
            const entries = performance.getEntriesByName('web-init-mark');
            `${entries.length}:${entries[0]?.entryType}:${entries[0]?.duration}`;
        "#,
        )
        .unwrap();
        let script = rusty_v8::Script::compile(scope, source, None).unwrap();
        let result = script.run(scope).unwrap().to_rust_string_lossy(scope);

        assert_eq!(
            result, "1:mark:0",
            "web_api::init_web_api should not install the old empty performance timeline"
        );
    }
}
