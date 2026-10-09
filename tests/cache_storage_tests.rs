//! Pins docs/CACHE_CONTRACT.md (G34).
//!
//! Stable surface: in-process `caches` / `Cache` put/match round-trip on the CLI
//! / runtime_minimal path. Not HTTP cache, not persistent, no CacheStorage.match
//! / ignore* options. match miss → undefined.

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
caches.open('stable-cache-roundtrip').then((cache) => {
  return cache.put('https://example.test/item', new Response('hello-cache', {
    status: 201,
    statusText: 'Created',
    headers: { 'x-amber': '1' }
  })).then(() => Promise.all([
    cache.match('https://example.test/item'),
    caches.has('stable-cache-roundtrip'),
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
        caches.open('stable-cache-delete').then((cache) => {
          return cache.put('/a', new Response('a')).then(() => cache.delete('/a')).then((deleted) => {
            return Promise.all([cache.match('/a'), caches.keys(), caches.delete('stable-cache-delete')]).then((values) => {
              const miss = values[0];
              const names = values[1];
              const storageDeleted = values[2];
              console.log([
                String(deleted),
                String(miss === undefined),
                String(names.indexOf('stable-cache-delete') >= 0),
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
fn amber_run_cache_match_miss_is_undefined() {
    let script = r#"
        caches.open('stable-cache-miss').then((cache) => {
          return cache.match('https://example.test/no-such-entry').then((miss) => {
            console.log([
              String(miss === undefined),
              String(miss === null)
            ].join('|'));
          });
        }).catch((error) => {
          console.log('error:' + String(error && error.message || error));
        });
    "#;
    let output = run_script(script);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("true|false"),
        "match miss must be undefined (not null): {}",
        stdout
    );
}

#[test]
fn amber_run_cache_storage_match_and_ignore_options_absent() {
    // Limits: no CacheStorage.match; no ignore* option APIs on the Stable surface.
    let script = r#"
        caches.open('stable-cache-limits').then((cache) => {
          const noStorageMatch = typeof caches.match === 'undefined';
          const noMatchAll = typeof cache.matchAll === 'undefined';
          // Calling match with an options bag must not unlock ignoreSearch (query is part of key).
          return cache.put('https://example.test/q?x=1', new Response('one')).then(() => {
            return Promise.all([
              cache.match('https://example.test/q?x=2'),
              cache.match('https://example.test/q?x=1')
            ]).then((values) => {
              const wrongQuery = values[0];
              const rightQuery = values[1];
              console.log([
                String(noStorageMatch),
                String(noMatchAll),
                String(wrongQuery === undefined),
                String(!!rightQuery)
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
        "no CacheStorage.match / matchAll; query string is part of key: {}",
        stdout
    );
}

#[test]
fn amber_run_cache_not_persistent_across_process() {
    // Limits: in-process Mutex only — a fresh amber process does not see prior puts.
    let put = r#"
        caches.open('stable-cache-persist').then((cache) => {
          return cache.put('https://example.test/persist', new Response('stale')).then(() => {
            console.log('put-ok');
          });
        }).catch((error) => {
          console.log('error:' + String(error && error.message || error));
        });
    "#;
    let put_out = run_script(put);
    let put_stdout = String::from_utf8_lossy(&put_out.stdout);
    assert!(
        put_out.status.success() && put_stdout.contains("put-ok"),
        "put process: {}",
        put_stdout
    );

    let match_script = r#"
        caches.open('stable-cache-persist').then((cache) => {
          return Promise.all([cache.match('https://example.test/persist'), caches.has('stable-cache-persist')]).then((values) => {
            const miss = values[0];
            const has = values[1];
            // New process: open creates the name, but prior Response body is gone.
            console.log([
              String(miss === undefined),
              String(has === true)
            ].join('|'));
          });
        }).catch((error) => {
          console.log('error:' + String(error && error.message || error));
        });
    "#;
    let match_out = run_script(match_script);
    let match_stdout = String::from_utf8_lossy(&match_out.stdout);
    assert!(
        match_stdout.contains("true|true"),
        "fresh process must miss prior put: {}",
        match_stdout
    );
}

#[test]
fn amber_run_cache_add_addall_surface() {
    let script = r#"
        caches.open('stable-cache-add-surface').then((cache) => {
          console.log([
            String(typeof cache.add === 'function'),
            String(typeof cache.addAll === 'function')
          ].join('|'));
        }).catch((error) => {
          console.log('error:' + String(error && error.message || error));
        });
    "#;
    let output = run_script(script);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("true|true"),
        "add/addAll surface: {}",
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
