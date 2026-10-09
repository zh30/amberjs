//! Stable G35 Service Worker fetch intercept (`FetchEvent.respondWith`) contract pins.
//! See `docs/SW_FETCH_CONTRACT.md`. G16 stays registration-only; G9 sync Response when
//! there is no fetch listener.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("runtime")
}

fn exec(runtime: &mut MinimalRuntime, code: &str) {
    runtime
        .execute_code(code)
        .unwrap_or_else(|err| panic!("execution failed: {err}"));
}

fn read_result(runtime: &mut MinimalRuntime) -> String {
    runtime
        .execute_code("globalThis.__amberSwFetchResult")
        .unwrap_or_else(|err| panic!("read failed: {err}"))
        .trim()
        .to_string()
}

fn amber_path() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_amber").unwrap_or_else(|_| "./target/debug/amber".to_string()),
    )
}

fn read_until_request_headers(stream: &mut std::net::TcpStream) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(500) {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

fn spawn_http(body: &str, hits: Arc<AtomicUsize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    let raw = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes();
    thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking");
        let started = Instant::now();
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    hits.fetch_add(1, Ordering::SeqCst);
                    let _ = stream.set_nodelay(true);
                    read_until_request_headers(&mut stream);
                    let _ = stream.write_all(&raw);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if started.elapsed() > Duration::from_secs(3) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(15));
                }
                Err(_) => break,
            }
        }
    });
    format!("http://{address}")
}

fn run_script(script: &str) -> std::process::Output {
    let temp_dir = tempfile::Builder::new()
        .prefix("amberjs-sw-fetch-intercept-")
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

#[test]
#[serial]
fn fetch_event_constructor_exposes_respond_with() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        const e = new FetchEvent('fetch', { requestUrl: '/x' });
        globalThis.__amberSwFetchResult = [
          typeof FetchEvent,
          typeof e.respondWith,
          e.type,
          e.requestUrl
        ].join('|');
        "#,
    );
    assert_eq!(read_result(&mut runtime), "function|function|fetch|/x");
}

#[test]
#[serial]
fn respond_with_supplies_response_and_skips_network() {
    let hits = Arc::new(AtomicUsize::new(0));
    let url = spawn_http("from-network", Arc::clone(&hits));
    let mut runtime = runtime();
    exec(
        &mut runtime,
        &r#"
        globalThis.__amberSwFetchResult = 'pending';
        const fetchBefore = fetch;
        const source = [
          "self.addEventListener('install', () => { self.skipWaiting(); });",
          "self.addEventListener('activate', () => { self.clients.claim(); });",
          "self.addEventListener('fetch', (event) => {",
          "  event.respondWith(new Response('from-sw', { status: 201, headers: { 'x-sw': 'yes' } }));",
          "});"
        ].join('\n');
        const swUrl = 'data:text/javascript,' + encodeURIComponent(source);
        let registration = null;
        const timer = setTimeout(() => {
          globalThis.__amberSwFetchResult = 'timeout';
          if (registration) registration.unregister();
        }, 2000);
        navigator.serviceWorker.register(swUrl).then((reg) => {
          registration = reg;
          const wrapped = fetch !== fetchBefore;
          return Promise.resolve(fetch('__URL__')).then((response) => {
            const body = typeof response.text === 'function' ? response.text() : String(response);
            const text = typeof body === 'string' ? body : String(body);
            const header = response.headers && typeof response.headers.get === 'function'
              ? response.headers.get('x-sw')
              : '';
            clearTimeout(timer);
            globalThis.__amberSwFetchResult = [
              text,
              String(response.status),
              String(header),
              String(wrapped)
            ].join('|');
            return reg.unregister();
          });
        }).catch((err) => {
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = 'error:' + String(err && err.message || err);
          if (registration) registration.unregister();
        });
        "#
        .replace("__URL__", &url),
    );
    assert_eq!(read_result(&mut runtime), "from-sw|201|yes|true");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "respondWith should skip the G9 network hop"
    );
}

#[test]
#[serial]
fn respond_with_caches_match() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        globalThis.__amberSwFetchResult = 'pending';
        const source = [
          "self.addEventListener('install', () => { self.skipWaiting(); });",
          "self.addEventListener('activate', () => { self.clients.claim(); });",
          "self.addEventListener('fetch', (event) => {",
          "  event.respondWith(",
          "    caches.open('sw-fetch-intercept').then((cache) => cache.match(event.request))",
          "      .then((hit) => hit || new Response('miss'))",
          "  );",
          "});"
        ].join('\n');
        const swUrl = 'data:text/javascript,' + encodeURIComponent(source);
        let registration = null;
        const timer = setTimeout(() => {
          globalThis.__amberSwFetchResult = 'timeout';
          if (registration) registration.unregister();
        }, 2000);
        caches.open('sw-fetch-intercept').then((cache) => {
          return cache.put('https://example.test/sw-cached', new Response('cached-by-sw', {
            status: 200,
            headers: { 'content-type': 'text/plain' }
          }));
        }).then(() => navigator.serviceWorker.register(swUrl)).then((reg) => {
          registration = reg;
          return Promise.resolve(fetch('https://example.test/sw-cached')).then((response) => {
            const body = typeof response.text === 'function' ? response.text() : String(response);
            const text = typeof body === 'string' ? body : String(body);
            clearTimeout(timer);
            globalThis.__amberSwFetchResult = text;
            return reg.unregister();
          });
        }).catch((err) => {
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = 'error:' + String(err && err.message || err);
          if (registration) registration.unregister();
        });
        "#,
    );
    assert_eq!(read_result(&mut runtime), "cached-by-sw");
}

#[test]
#[serial]
fn no_fetch_listener_keeps_g9_fetch_and_hits_network() {
    let hits = Arc::new(AtomicUsize::new(0));
    let url = spawn_http("from-network", Arc::clone(&hits));
    let mut runtime = runtime();
    exec(
        &mut runtime,
        &r#"
        globalThis.__amberSwFetchResult = 'pending';
        const fetchBefore = fetch;
        const source = [
          "self.addEventListener('install', () => { self.skipWaiting(); });",
          "self.addEventListener('activate', () => { self.clients.claim(); });"
        ].join('\n');
        const swUrl = 'data:text/javascript,' + encodeURIComponent(source);
        let registration = null;
        const timer = setTimeout(() => {
          globalThis.__amberSwFetchResult = 'timeout';
          if (registration) registration.unregister();
        }, 2000);
        navigator.serviceWorker.register(swUrl).then((reg) => {
          registration = reg;
          const same = fetch === fetchBefore;
          const response = fetch('__URL__');
          const body = typeof response.text === 'function' ? response.text() : String(response);
          const text = typeof body === 'string' ? body : String(body);
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = [text, String(same)].join('|');
          return reg.unregister();
        }).catch((err) => {
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = 'error:' + String(err && err.message || err);
          if (registration) registration.unregister();
        });
        "#
        .replace("__URL__", &url),
    );
    assert_eq!(read_result(&mut runtime), "from-network|true");
    assert!(
        hits.load(Ordering::SeqCst) >= 1,
        "G9 fetch without a fetch listener must hit the network"
    );
}

#[test]
#[serial]
fn fetch_listener_without_respond_with_falls_through_to_network() {
    let hits = Arc::new(AtomicUsize::new(0));
    let url = spawn_http("passthrough", Arc::clone(&hits));
    let mut runtime = runtime();
    exec(
        &mut runtime,
        &r#"
        globalThis.__amberSwFetchResult = 'pending';
        const source = [
          "self.addEventListener('install', () => { self.skipWaiting(); });",
          "self.addEventListener('activate', () => { self.clients.claim(); });",
          "self.addEventListener('fetch', () => { /* observe only */ });"
        ].join('\n');
        const swUrl = 'data:text/javascript,' + encodeURIComponent(source);
        let registration = null;
        const timer = setTimeout(() => {
          globalThis.__amberSwFetchResult = 'timeout';
          if (registration) registration.unregister();
        }, 2000);
        navigator.serviceWorker.register(swUrl).then((reg) => {
          registration = reg;
          return Promise.resolve(fetch('__URL__')).then((response) => {
            const body = typeof response.text === 'function' ? response.text() : String(response);
            const text = typeof body === 'string' ? body : String(body);
            clearTimeout(timer);
            globalThis.__amberSwFetchResult = text;
            return reg.unregister();
          });
        }).catch((err) => {
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = 'error:' + String(err && err.message || err);
          if (registration) registration.unregister();
        });
        "#
        .replace("__URL__", &url),
    );
    assert_eq!(read_result(&mut runtime), "passthrough");
    assert!(
        hits.load(Ordering::SeqCst) >= 1,
        "a fetch listener that does not call respondWith should hit G9 fetch"
    );
}

#[test]
fn amber_run_respond_with_intercepts_fetch() {
    let script = r#"
        const source = [
          "self.addEventListener('install', () => { self.skipWaiting(); });",
          "self.addEventListener('activate', () => { self.clients.claim(); });",
          "self.addEventListener('fetch', (event) => {",
          "  event.respondWith(new Response('cli-sw'));",
          "});"
        ].join('\n');
        const swUrl = 'data:text/javascript,' + encodeURIComponent(source);
        navigator.serviceWorker.register(swUrl).then((reg) => {
          return Promise.resolve(fetch('https://example.test/cli-sw')).then((response) => {
            const body = typeof response.text === 'function' ? response.text() : String(response);
            const text = typeof body === 'string' ? body : String(body);
            console.log(text === 'cli-sw' ? 'SUCCESS' : 'ERROR:' + text);
            return reg.unregister();
          });
        }).catch((err) => {
          console.log('ERROR:' + String(err && err.message || err));
        });
    "#;
    let output = run_script(script);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "amber run intercept failed: stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("SUCCESS"),
        "amber run should intercept fetch: {stdout}"
    );
}

#[test]
#[serial]
fn waiting_worker_does_not_intercept_fetch() {
    let hits = Arc::new(AtomicUsize::new(0));
    let url = spawn_http("waiting-network", Arc::clone(&hits));
    let mut runtime = runtime();
    exec(
        &mut runtime,
        &r#"
        globalThis.__amberSwFetchResult = 'pending';
        const fetchBefore = fetch;
        // No skipWaiting → stays installed/waiting; must not arm intercept.
        const source = [
          "self.addEventListener('fetch', (event) => {",
          "  event.respondWith(new Response('from-waiting'));",
          "});"
        ].join('\n');
        const swUrl = 'data:text/javascript,' + encodeURIComponent(source);
        let registration = null;
        const timer = setTimeout(() => {
          globalThis.__amberSwFetchResult = 'timeout';
          if (registration) registration.unregister();
        }, 2000);
        navigator.serviceWorker.register(swUrl).then((reg) => {
          registration = reg;
          const state = reg.waiting && reg.waiting.state
            ? reg.waiting.state
            : (reg.active && reg.active.state) || 'none';
          const same = fetch === fetchBefore;
          const response = fetch('__URL__');
          const body = typeof response.text === 'function' ? response.text() : String(response);
          const text = typeof body === 'string' ? body : String(body);
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = [text, String(same), state].join('|');
          return reg.unregister();
        }).catch((err) => {
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = 'error:' + String(err && err.message || err);
          if (registration) registration.unregister();
        });
        "#
        .replace("__URL__", &url),
    );
    let result = read_result(&mut runtime);
    assert!(
        result.starts_with("waiting-network|true|"),
        "waiting worker must keep G9 fetch identity and hit network: {result}"
    );
    assert!(
        result.contains("|installed") || result.contains("|waiting"),
        "expected installed/waiting state: {result}"
    );
    assert!(
        hits.load(Ordering::SeqCst) >= 1,
        "waiting worker must not intercept; network should be hit"
    );
}

#[test]
#[serial]
fn scope_is_not_matched_for_intercept() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        globalThis.__amberSwFetchResult = 'pending';
        const source = [
          "self.addEventListener('install', () => { self.skipWaiting(); });",
          "self.addEventListener('activate', () => { self.clients.claim(); });",
          "self.addEventListener('fetch', (event) => {",
          "  event.respondWith(new Response('scoped-sw'));",
          "});"
        ].join('\n');
        const swUrl = 'data:text/javascript,' + encodeURIComponent(source);
        let registration = null;
        const timer = setTimeout(() => {
          globalThis.__amberSwFetchResult = 'timeout';
          if (registration) registration.unregister();
        }, 2000);
        // Scope is recorded (G16) but must not gate intercept against the request URL.
        navigator.serviceWorker.register(swUrl, { scope: '/only-this-scope/' }).then((reg) => {
          registration = reg;
          return Promise.resolve(fetch('https://example.test/outside-scope')).then((response) => {
            const body = typeof response.text === 'function' ? response.text() : String(response);
            const text = typeof body === 'string' ? body : String(body);
            clearTimeout(timer);
            globalThis.__amberSwFetchResult = [text, reg.scope].join('|');
            return reg.unregister();
          });
        }).catch((err) => {
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = 'error:' + String(err && err.message || err);
          if (registration) registration.unregister();
        });
        "#,
    );
    assert_eq!(read_result(&mut runtime), "scoped-sw|/only-this-scope/");
}

#[test]
#[serial]
fn intercepted_fetch_returns_a_promise() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        globalThis.__amberSwFetchResult = 'pending';
        const source = [
          "self.addEventListener('install', () => { self.skipWaiting(); });",
          "self.addEventListener('activate', () => { self.clients.claim(); });",
          "self.addEventListener('fetch', (event) => {",
          "  event.respondWith(new Response('promise-body'));",
          "});"
        ].join('\n');
        const swUrl = 'data:text/javascript,' + encodeURIComponent(source);
        let registration = null;
        const timer = setTimeout(() => {
          globalThis.__amberSwFetchResult = 'timeout';
          if (registration) registration.unregister();
        }, 2000);
        navigator.serviceWorker.register(swUrl).then((reg) => {
          registration = reg;
          const value = fetch('https://example.test/promise-pin');
          const isPromise = !!value && typeof value.then === 'function';
          return Promise.resolve(value).then((response) => {
            const body = typeof response.text === 'function' ? response.text() : String(response);
            const text = typeof body === 'string' ? body : String(body);
            clearTimeout(timer);
            globalThis.__amberSwFetchResult = [String(isPromise), text].join('|');
            return reg.unregister();
          });
        }).catch((err) => {
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = 'error:' + String(err && err.message || err);
          if (registration) registration.unregister();
        });
        "#,
    );
    assert_eq!(read_result(&mut runtime), "true|promise-body");
}

#[test]
#[serial]
fn respond_with_transfers_json_text_body_across_isolate() {
    let mut runtime = runtime();
    exec(
        &mut runtime,
        r#"
        globalThis.__amberSwFetchResult = 'pending';
        const source = [
          "self.addEventListener('install', () => { self.skipWaiting(); });",
          "self.addEventListener('activate', () => { self.clients.claim(); });",
          "self.addEventListener('fetch', (event) => {",
          "  const url = event.requestUrl || (event.request && event.request.url) || '';",
          "  event.respondWith(new Response(JSON.stringify({ echo: url, note: 'text-cross' }), {",
          "    status: 200,",
          "    headers: { 'content-type': 'application/json', 'x-cross': '1' }",
          "  }));",
          "});"
        ].join('\n');
        const swUrl = 'data:text/javascript,' + encodeURIComponent(source);
        let registration = null;
        const timer = setTimeout(() => {
          globalThis.__amberSwFetchResult = 'timeout';
          if (registration) registration.unregister();
        }, 2000);
        navigator.serviceWorker.register(swUrl).then((reg) => {
          registration = reg;
          return Promise.resolve(fetch('https://example.test/json-cross')).then((response) => {
            const body = typeof response.text === 'function' ? response.text() : String(response);
            const text = typeof body === 'string' ? body : String(body);
            const header = response.headers && typeof response.headers.get === 'function'
              ? response.headers.get('x-cross')
              : '';
            let parsed = null;
            try { parsed = JSON.parse(text); } catch (_) {}
            clearTimeout(timer);
            globalThis.__amberSwFetchResult = [
              parsed && parsed.note ? parsed.note : 'bad',
              parsed && parsed.echo ? String(parsed.echo) : '',
              String(header),
              typeof text
            ].join('|');
            return reg.unregister();
          });
        }).catch((err) => {
          clearTimeout(timer);
          globalThis.__amberSwFetchResult = 'error:' + String(err && err.message || err);
          if (registration) registration.unregister();
        });
        "#,
    );
    assert_eq!(
        read_result(&mut runtime),
        "text-cross|https://example.test/json-cross|1|string"
    );
}
