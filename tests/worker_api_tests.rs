// Worker API tests for Amber runtime
// v0.3.320: Tests for Web Worker support

use std::fs;
use std::process::{Command, Stdio};

/// Helper function to run a JavaScript script with amberjs
fn run_script(script: &str) -> std::process::Output {
    // Create a temporary file with the script
    let temp_dir = tempfile::Builder::new()
        .prefix("amberjs-worker-test-")
        .tempdir()
        .unwrap();
    let temp_file = temp_dir.path().join("test.js");
    fs::write(&temp_file, script).unwrap();

    // Run amberjs with the script
    let output = Command::new("./target/debug/amber")
        .arg("run")
        .arg(&temp_file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to run amber");

    // Clean up
    drop(temp_dir);

    output
}

#[cfg(test)]
mod worker_api_tests {
    use super::*;

    #[test]
    fn test_worker_constructor_exists() {
        // Test that Worker constructor is available
        let script = r#"
            if (typeof Worker !== 'undefined') {
                console.log('SUCCESS: Worker constructor exists');
            } else {
                throw new Error('Worker constructor not found');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "Worker constructor should exist: {}",
            stdout
        );
        assert!(
            stdout.contains("SUCCESS: Worker constructor exists"),
            "Output: {}",
            stdout
        );
    }

    #[test]
    fn test_worker_missing_script_url_is_a_type_error() {
        let script = r#"
            try {
                new Worker();
                console.log('ERROR: Worker created without script URL');
            } catch (e) {
                console.log((e instanceof TypeError ? 'SUCCESS: ' : 'ERROR: wrong type: ') + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS: Worker requires a script URL or source"),
            "Worker without a script URL should be a TypeError: {}",
            stdout
        );
    }

    #[test]
    fn test_worker_unreadable_script_path_reports_the_path() {
        let script = r#"
            try {
                new Worker('./definitely-missing-worker.js');
                console.log('ERROR: Worker accepted an unreadable path');
            } catch (e) {
                console.log('SUCCESS: ' + e.message);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(
                "SUCCESS: WorkerHost could not load script './definitely-missing-worker.js'"
            ),
            "an unreadable worker path should name the path it tried: {}",
            stdout
        );
    }

    /// The data URL body is the worker source; the `data:,` prefix must be
    /// stripped rather than handed to the compiler.
    #[test]
    fn test_worker_data_url_body_is_executed_as_source() {
        let script = r#"
            const worker = new Worker("data:text/javascript,console.log('SUCCESS: data URL body ran')");
            setTimeout(() => worker.terminate(), 300);
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stdout.contains("SUCCESS: data URL body ran"),
            "the data URL body should run in the worker: stdout={} stderr={}",
            stdout,
            stderr
        );
        assert!(
            !stderr.contains("SyntaxError"),
            "the data URL prefix must not reach the compiler: {}",
            stderr
        );
    }

    #[test]
    fn test_worker_percent_encoded_data_url_is_decoded() {
        let script = r#"
            const worker = new Worker("data:,console.log('SUCCESS%3A%20decoded')");
            setTimeout(() => worker.terminate(), 300);
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stdout.contains("SUCCESS: decoded"),
            "percent-encoded data URL payloads should be decoded: stdout={} stderr={}",
            stdout,
            stderr
        );
    }

    #[test]
    fn test_worker_base64_data_url_is_decoded() {
        // base64 of: console.log('SUCCESS: base64')
        let script = r#"
            const worker = new Worker("data:text/javascript;base64,Y29uc29sZS5sb2coJ1NVQ0NFU1M6IGJhc2U2NCcp");
            setTimeout(() => worker.terminate(), 300);
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stdout.contains("SUCCESS: base64"),
            "base64 data URL payloads should be decoded: stdout={} stderr={}",
            stdout,
            stderr
        );
    }

    #[test]
    fn test_worker_exposes_post_message_and_terminate() {
        let script = r#"
            const worker = new Worker("data:,self === undefined");
            console.log('postMessage=' + typeof worker.postMessage + ' terminate=' + typeof worker.terminate);
            worker.terminate();
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("postMessage=function terminate=function"),
            "a spawned worker should expose its lifecycle methods: {}",
            stdout
        );
    }
}
