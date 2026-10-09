//! Preview Cache / CacheStorage (`caches`) on the CLI / runtime_minimal path.
//! Not a Stable contract. G9–G16 and G29–G30 stay intact.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn amber_path() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_amber").unwrap_or_else(|_| "./target/debug/amber".to_string()),
    )
}

fn run_script(script: &str) -> std::process::Output {
    let temp_dir = tempfile::Builder::new()
        .prefix("amberjs-cache-storage-test-")
        .tempdir()
        .unwrap();
    let temp_file = temp_dir.path().join("test.js");
    fs::write(&temp_file, script).unwrap();
    let output = Command::new(amber_path())
        .arg("run")
        .arg(&temp_file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to run amber");
    drop(temp_dir);
    output
}

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("runtime")
}

const ROUND_TRIP: &str = r#"
globalThis.__amberCacheResult = 'pending';
caches.open('preview-cache-roundtrip').then((cache) => {
  return cache.put('https://example.test/item', new Response('hello-cache', {
    status: 201,
    statusText: 'Created',
    headers: { 'x-amber': '1' }
  })).then(() => Promise.all([
    cache.match('https://example.test/item'),
    caches.has('preview-cache-roundtrip'),
    cache.keys()
  ])).then((values) => {
    const hit = values[0];
    const has = values[1];
    const keys = values[2];
    if (!hit) {
      globalThis.__amberCacheResult = 'miss';
      return;
    }
    const body = hit.text();
    const text = typeof body === 'string' ? body : String(body);
    const header = hit.headers && typeof hit.headers.get === 'function'
      ? hit.headers.get('x-amber')
      : '';
    globalThis.__amberCacheResult = [
      text,
      String(hit.status),
      String(has),
      String(keys.length),
      String(header)
    ].join('|');
    console.log(globalThis.__amberCacheResult);
  });
}).catch((error) => {
  globalThis.__amberCacheResult = 'error:' + String(error && error.message || error);
  console.log(globalThis.__amberCacheResult);
});
"#;

#[test]
fn amber_run_cache_open_put_match_round_trip() {
    let output = run_script(ROUND_TRIP);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "amber run should succeed: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("hello-cache|201|true|1|1"),
        "open/put/match round-trip: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn amber_run_cache_delete_and_storage_keys() {
    let script = r#"
        caches.open('preview-cache-delete').then((cache) => {
          return cache.put('/a', new Response('a')).then(() => cache.delete('/a')).then((deleted) => {
            return Promise.all([cache.match('/a'), caches.keys(), caches.delete('preview-cache-delete')]).then((values) => {
              const miss = values[0];
              const names = values[1];
              const storageDeleted = values[2];
              console.log([
                String(deleted),
                String(miss === undefined),
                String(names.indexOf('preview-cache-delete') >= 0),
                String(storageDeleted)
              ].join('|'));
            });
          });
        }).catch((error) => {
          console.log('error:' + String(error && error.message || error));
        });
    "#;
    let output = run_script(script);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("true|true|true|true"),
        "delete/keys: {}",
        stdout
    );
}

#[test]
#[serial]
fn runtime_minimal_cache_open_put_match_round_trip() {
    let mut runtime = runtime();
    runtime
        .execute_code(ROUND_TRIP)
        .unwrap_or_else(|err| panic!("execution failed: {err}"));
    let result = runtime
        .execute_code("globalThis.__amberCacheResult")
        .unwrap_or_else(|err| panic!("read failed: {err}"))
        .trim()
        .to_string();
    assert_eq!(result, "hello-cache|201|true|1|1");
}
