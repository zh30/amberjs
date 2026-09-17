use amberjs::nodejs_core::commonjs_resolver::{resolve_esm_module, ResolvedModule};
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[test]
#[serial]
fn test_wintertc_dom_exception() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    if (typeof DOMException === 'undefined') throw new Error('DOMException is not defined');

    // 1. Instantiation and properties
    const err = new DOMException('Operation timed out', 'TimeoutError');
    if (err.name !== 'TimeoutError') throw new Error('Expected TimeoutError, got ' + err.name);
    if (err.message !== 'Operation timed out') throw new Error('Expected message mismatch');
    if (err.code !== 23) throw new Error('Expected code 23 (TIMEOUT_ERR), got ' + err.code);
    if (!(err instanceof Error)) throw new Error('DOMException must inherit from Error');
    if (!(err instanceof DOMException)) throw new Error('instanceof DOMException failed');

    // 2. Static constants
    if (DOMException.INDEX_SIZE_ERR !== 1) throw new Error('INDEX_SIZE_ERR mismatch');
    if (DOMException.NOT_FOUND_ERR !== 8) throw new Error('NOT_FOUND_ERR mismatch');
    if (DOMException.SYNTAX_ERR !== 12) throw new Error('SYNTAX_ERR mismatch');
    if (DOMException.ABORT_ERR !== 20) throw new Error('ABORT_ERR mismatch');
    if (DOMException.TIMEOUT_ERR !== 23) throw new Error('TIMEOUT_ERR mismatch');

    // 3. AbortError code
    const abortErr = new DOMException('Aborted', 'AbortError');
    if (abortErr.code !== 20) throw new Error('Expected code 20 for AbortError');

    JSON.stringify({ success: true, errCode: err.code, abortCode: abortErr.code });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_wintertc_navigator_and_globals() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    // 1. globalThis.self requirement (ECMA-429 Section 5.2)
    if (globalThis.self !== globalThis) throw new Error('globalThis.self must equal globalThis');

    // 2. reportError function (ECMA-429 Section 5.2)
    if (typeof globalThis.reportError !== 'function') throw new Error('reportError is not a function');

    // 3. PromiseRejectionEvent
    if (typeof globalThis.PromiseRejectionEvent !== 'function') throw new Error('PromiseRejectionEvent is not a constructor');
    const p = Promise.resolve(1);
    const rejEvent = new PromiseRejectionEvent('unhandledrejection', { promise: p, reason: 'error reason' });
    if (rejEvent.reason !== 'error reason') throw new Error('PromiseRejectionEvent reason mismatch');

    // 4. navigator conforming to ECMA-429 Section 7
    if (typeof navigator === 'undefined') throw new Error('navigator is undefined');
    if (typeof navigator.userAgent !== 'string' || !navigator.userAgent.startsWith('Beejs/')) {
        throw new Error('Invalid userAgent: ' + navigator.userAgent);
    }
    if (typeof navigator.hardwareConcurrency !== 'number' || navigator.hardwareConcurrency < 1) {
        throw new Error('Invalid hardwareConcurrency: ' + navigator.hardwareConcurrency);
    }
    if (navigator.language !== 'en-US') throw new Error('Invalid language: ' + navigator.language);
    if (!Array.isArray(navigator.languages) || navigator.languages.length === 0) {
        throw new Error('Invalid languages array');
    }
    if (typeof navigator.platform !== 'string') throw new Error('Invalid platform');

    JSON.stringify({
        success: true,
        userAgent: navigator.userAgent,
        hardwareConcurrency: navigator.hardwareConcurrency
    });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_wintertc_url_pattern() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    if (typeof URLPattern === 'undefined') throw new Error('URLPattern is not defined');

    // 1. Object constructor with pathname pattern
    const p1 = new URLPattern({ pathname: '/books/:id' });
    if (!p1.test('https://example.com/books/123')) throw new Error('p1 test should match');
    if (p1.test('https://example.com/authors/123')) throw new Error('p1 test should not match');

    const res1 = p1.exec('https://example.com/books/42');
    if (!res1 || !res1.pathname || res1.pathname.groups.id !== '42') {
        throw new Error('p1 exec failed to capture id group: ' + JSON.stringify(res1));
    }

    // 2. Relative pattern with baseURL
    const p2 = new URLPattern('/api/:version/users/:user', 'https://api.example.com');
    const res2 = p2.exec('https://api.example.com/api/v2/users/alice');
    if (!res2 || res2.pathname.groups.version !== 'v2' || res2.pathname.groups.user !== 'alice') {
        throw new Error('p2 exec group match failed: ' + JSON.stringify(res2));
    }

    // 3. Wildcard hostname pattern
    const p3 = new URLPattern('https://*.example.com/*');
    if (!p3.test('https://blog.example.com/post-1')) throw new Error('p3 wildcard test should match');

    JSON.stringify({ success: true, id: res1.pathname.groups.id, user: res2.pathname.groups.user });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_wintertc_queuing_strategies_and_stream_iter() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    // 1. ByteLengthQueuingStrategy
    if (typeof ByteLengthQueuingStrategy === 'undefined') throw new Error('ByteLengthQueuingStrategy missing');
    const byteStrategy = new ByteLengthQueuingStrategy({ highWaterMark: 1024 });
    if (byteStrategy.highWaterMark !== 1024) throw new Error('highWaterMark mismatch');
    if (byteStrategy.size({ byteLength: 512 }) !== 512) throw new Error('size mismatch');

    // 2. CountQueuingStrategy
    if (typeof CountQueuingStrategy === 'undefined') throw new Error('CountQueuingStrategy missing');
    const countStrategy = new CountQueuingStrategy({ highWaterMark: 16 });
    if (countStrategy.highWaterMark !== 16) throw new Error('highWaterMark mismatch');
    if (countStrategy.size({ item: 1 }) !== 1) throw new Error('size should be 1');

    // 3. ReadableStream.from and Symbol.asyncIterator
    if (typeof ReadableStream.from !== 'function') throw new Error('ReadableStream.from missing');
    const stream = ReadableStream.from(['hello', ' ', 'wintertc']);
    if (!stream || !(stream instanceof ReadableStream)) throw new Error('ReadableStream.from did not return ReadableStream');
    if (typeof stream[Symbol.asyncIterator] !== 'function') throw new Error('Symbol.asyncIterator missing on stream');

    JSON.stringify({ success: true });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_wintertc_sockets_api() {
    // Bind a real listener so `connect()` performs an actual TCP handshake
    // (the kernel completes SYN-ACK from the listen backlog; accept() is not required).
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let port = listener.local_addr().expect("local_addr").port();

    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = format!(
        r#"
    // 1. Require from bee:sockets or global
    const sockets = require('bee:sockets');
    if (!sockets || typeof sockets.connect !== 'function') throw new Error('bee:sockets.connect missing');
    if (typeof globalThis.connect !== 'function') throw new Error('globalThis.connect missing');
    if (typeof globalThis.Socket !== 'function') throw new Error('globalThis.Socket missing');

    // 2. Instantiate Socket object against a live local listener.
    const socket = sockets.connect({{ hostname: '127.0.0.1', port: {port} }});
    if (!socket || !(socket instanceof globalThis.Socket)) throw new Error('Invalid socket instance');

    // 3. Check Web Streams properties
    if (!socket.readable || !(socket.readable instanceof ReadableStream)) {{
        throw new Error('socket.readable must be a ReadableStream');
    }}
    if (!socket.writable || !(socket.writable instanceof WritableStream)) {{
        throw new Error('socket.writable must be a WritableStream');
    }}
    if (!(socket.opened instanceof Promise)) throw new Error('socket.opened must be a Promise');
    if (!(socket.closed instanceof Promise)) throw new Error('socket.closed must be a Promise');
    if (typeof socket.close !== 'function') throw new Error('socket.close must be a function');
    if (typeof socket.startTls !== 'function') throw new Error('socket.startTls must be a function');

    JSON.stringify({{ success: true, port: {port} }});
    "#
    );

    let res = runtime.execute_code(&code).expect("Execution failed");
    assert!(res.contains("\"success\":true"), "unexpected result: {res}");
    drop(listener);
}

#[test]
#[serial]
fn test_wintertc_sockets_tls_rejects_untrusted_self_signed() {
    amberjs::sockets::clear_test_root_cas();
    let listener = amberjs::sockets::start_self_signed_tls_listener().expect("tls listener");
    let port = listener.port;

    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = format!(
        r#"
    globalThis.__tlsErr = 'pending';
    const tlsSocket = require('bee:sockets').connect(
        {{ hostname: '127.0.0.1', port: {port} }},
        {{ secureTransport: 'on', sni: 'localhost' }}
    );
    tlsSocket.opened.then(
        () => {{ globalThis.__tlsErr = 'opened'; }},
        (e) => {{ globalThis.__tlsErr = String(e); }}
    );
    JSON.stringify({{ queued: true }});
    "#
    );
    runtime.execute_code(&code).expect("tls untrusted execute");
    let tls_res = runtime
        .execute_code("JSON.stringify({ err: globalThis.__tlsErr })")
        .expect("tls untrusted outcome");
    let lower = tls_res.to_lowercase();
    assert!(
        lower.contains("certificate")
            || lower.contains("unknownissuer")
            || lower.contains("unknown issuer")
            || lower.contains("invalid")
            || lower.contains("tls handshake failed"),
        "untrusted self-signed cert must fail the rustls handshake, got {tls_res}"
    );
}

#[test]
#[serial]
fn test_wintertc_sockets_tls_handshake_with_test_ca() {
    amberjs::sockets::clear_test_root_cas();
    let listener = amberjs::sockets::start_self_signed_tls_listener().expect("tls listener");
    amberjs::sockets::install_test_root_ca_der(listener.cert_der.clone());
    let port = listener.port;

    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = format!(
        r#"
    globalThis.__tlsOpened = 'pending';
    globalThis.__tlsUpgraded = false;
    const tlsSocket = require('bee:sockets').connect(
        {{ hostname: '127.0.0.1', port: {port} }},
        {{ secureTransport: 'on', sni: 'localhost' }}
    );
    tlsSocket.opened.then(
        () => {{
            globalThis.__tlsOpened = 'ok';
            globalThis.__tlsUpgraded = tlsSocket.upgraded === true;
        }},
        (e) => {{ globalThis.__tlsOpened = String(e); }}
    );
    JSON.stringify({{ queued: true }});
    "#
    );
    runtime.execute_code(&code).expect("tls trusted execute");
    let tls_res = runtime
        .execute_code("JSON.stringify({ opened: globalThis.__tlsOpened, upgraded: globalThis.__tlsUpgraded })")
        .expect("tls trusted outcome");
    assert!(
        tls_res.contains("\"opened\":\"ok\""),
        "trusted self-signed handshake must succeed, got {tls_res}"
    );
    assert!(
        tls_res.contains("\"upgraded\":true"),
        "secureTransport on must set upgraded after handshake, got {tls_res}"
    );
    amberjs::sockets::clear_test_root_cas();
}

#[test]
#[serial]
fn test_wintertc_sockets_start_tls_upgrades_opened_socket() {
    amberjs::sockets::clear_test_root_cas();
    let listener = amberjs::sockets::start_self_signed_tls_listener().expect("tls listener");
    amberjs::sockets::install_test_root_ca_der(listener.cert_der.clone());
    let port = listener.port;

    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = format!(
        r#"
    globalThis.__startTls = 'pending';
    (async () => {{
        const sockets = require('bee:sockets');
        const socket = sockets.connect({{ hostname: '127.0.0.1', port: {port} }}, {{ sni: 'localhost' }});
        await socket.opened;
        if (socket.upgraded) throw new Error('plain connect must not be upgraded');
        await socket.startTls();
        if (!socket.upgraded) throw new Error('startTls must set upgraded on success');
        globalThis.__startTls = 'upgraded';
    }})().catch((e) => {{ globalThis.__startTls = String(e); }});
    JSON.stringify({{ queued: true }});
    "#
    );
    runtime.execute_code(&code).expect("startTls execute");
    let res = runtime
        .execute_code("JSON.stringify({ startTls: globalThis.__startTls })")
        .expect("startTls outcome");
    assert!(
        res.contains("\"startTls\":\"upgraded\""),
        "startTls must complete rustls handshake, got {res}"
    );
    amberjs::sockets::clear_test_root_cas();
}

#[test]
#[serial]
fn test_wintertc_sockets_start_tls_rejects_untrusted_and_does_not_upgrade() {
    amberjs::sockets::clear_test_root_cas();
    let listener = amberjs::sockets::start_self_signed_tls_listener().expect("tls listener");
    let port = listener.port;

    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let code = format!(
        r#"
    globalThis.__startTlsFail = 'pending';
    globalThis.__startTlsUpgraded = null;
    (async () => {{
        const sockets = require('bee:sockets');
        const socket = sockets.connect({{ hostname: '127.0.0.1', port: {port} }}, {{ sni: 'localhost' }});
        await socket.opened;
        if (socket.upgraded) throw new Error('plain connect must not be upgraded');
        try {{
            await socket.startTls();
            globalThis.__startTlsFail = 'upgraded-without-trust';
            globalThis.__startTlsUpgraded = socket.upgraded === true;
        }} catch (e) {{
            globalThis.__startTlsFail = String(e);
            globalThis.__startTlsUpgraded = socket.upgraded === true;
        }}
    }})().catch((e) => {{ globalThis.__startTlsFail = String(e); }});
    JSON.stringify({{ queued: true }});
    "#
    );
    runtime
        .execute_code(&code)
        .expect("startTls untrusted execute");
    let res = runtime
        .execute_code(
            "JSON.stringify({ err: globalThis.__startTlsFail, upgraded: globalThis.__startTlsUpgraded })",
        )
        .expect("startTls untrusted outcome");
    let lower = res.to_lowercase();
    assert!(
        !res.contains("upgraded-without-trust"),
        "startTls must not succeed against an untrusted self-signed cert, got {res}"
    );
    assert!(
        lower.contains("certificate")
            || lower.contains("unknownissuer")
            || lower.contains("unknown issuer")
            || lower.contains("invalid")
            || lower.contains("tls handshake failed"),
        "startTls without a trusted CA must fail the rustls handshake, got {res}"
    );
    assert!(
        res.contains("\"upgraded\":false"),
        "failed startTls must leave upgraded false, got {res}"
    );
}

#[test]
#[serial]
fn test_wintertc_runtime_keys_and_import_meta() {
    // 1. Test package.json conditional exports with "wintercg"
    let dir = tempdir().expect("tempdir");
    let pkg_dir = dir.path().join("node_modules").join("winter-lib");
    fs::create_dir_all(&pkg_dir).expect("create dir");

    let pkg_json = r#"{
        "name": "winter-lib",
        "version": "1.0.0",
        "exports": {
            "wintercg": "./winter.js",
            "node": "./node.js",
            "default": "./index.js"
        }
    }"#;
    fs::write(pkg_dir.join("package.json"), pkg_json).expect("write pkg");
    fs::write(
        pkg_dir.join("winter.js"),
        "export const flavor = 'wintercg';",
    )
    .expect("write winter.js");
    fs::write(pkg_dir.join("node.js"), "export const flavor = 'node';").expect("write node.js");
    fs::write(pkg_dir.join("index.js"), "export const flavor = 'default';")
        .expect("write index.js");

    // Resolve ESM module from pkg_dir parent
    let resolved = resolve_esm_module("winter-lib", dir.path()).expect("resolution failed");
    match resolved {
        ResolvedModule::File(path) => {
            let filename = path.file_name().unwrap().to_str().unwrap();
            assert_eq!(
                filename, "winter.js",
                "Expected wintercg conditional export to be prioritized!"
            );
        }
        _ => panic!("Expected File resolution"),
    }

    // 2. Test import.meta.main / env / resolve (wintercg export must win)
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let entry = dir.path().join("app.js");
    fs::write(&entry, "export default 1;\n").expect("write entry");
    runtime.set_main_module_path(&entry);
    let code = r#"
    const meta = (typeof globalThis.import !== 'undefined' && globalThis.import.meta) ? globalThis.import.meta : null;
    if (!meta) throw new Error('import.meta is undefined');
    if (typeof meta.main !== 'boolean') throw new Error('import.meta.main must be a boolean');
    if (typeof meta.env !== 'object') throw new Error('import.meta.env must be an object');
    if (typeof meta.resolve !== 'function') throw new Error('import.meta.resolve must be a function');
    const href = meta.resolve('winter-lib');
    if (typeof href !== 'string' || href.indexOf('winter.js') < 0) {
        throw new Error('import.meta.resolve did not honor wintercg export: ' + href);
    }

    JSON.stringify({
        success: true,
        main: meta.main,
        hasEnv: typeof meta.env === 'object',
        href: href
    });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
    assert!(
        res.contains("winter.js"),
        "import.meta.resolve should return winter.js path, got {res}"
    );
}

#[test]
#[serial]
fn test_wintertc_unhandled_rejection_dispatches_event() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");
    let setup = r#"
    globalThis.__seenRejection = null;
    globalThis.onunhandledrejection = function(ev) {
        globalThis.__seenRejection = {
            name: ev && ev.constructor ? ev.constructor.name : '',
            reason: ev && ev.reason
        };
    };
    Promise.reject('winter-reject');
    JSON.stringify({ queued: true });
    "#;
    runtime.execute_code(setup).expect("reject setup");
    let res = runtime
        .execute_code("JSON.stringify(globalThis.__seenRejection)")
        .expect("read rejection");
    assert!(
        res.contains("PromiseRejectionEvent") || res.contains("winter-reject"),
        "handler should receive PromiseRejectionEvent, got {res}"
    );
    assert!(
        res.contains("winter-reject"),
        "handler should receive rejection reason, got {res}"
    );
}
