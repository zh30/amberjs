//! Integration tests for Beejs Peripheral Tooling Suite
//!
//! Tests fmt, lint, bench, compile, types, task, and coverage.

use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_types_export_cli_and_lib() {
    let types = amberjs::types_export::get_type_definitions();
    assert!(types.contains("declare module \"bee:ai\""));
    assert!(types.contains("export class Tensor"));
    assert!(types.contains("export class LLM"));
    assert!(types.contains("export class AgentPipeline"));
    assert!(types.contains("declare namespace bee"));

    let dir = tempdir().expect("tempdir");
    let out_file = dir.path().join("beejs.d.ts");
    amberjs::types_export::export_types(Some(&out_file)).expect("export should succeed");
    assert!(out_file.exists());
    let written = fs::read_to_string(&out_file).expect("read");
    assert_eq!(written, types);
}

#[test]
fn test_task_runner_executes_scripts() {
    let dir = tempdir().expect("tempdir");
    let pkg = dir.path().join("package.json");
    fs::write(
        &pkg,
        r#"{
            "name": "task-test",
            "scripts": {
                "greet": "echo task_runner_success",
                "custom-arg": "echo"
            }
        }"#,
    )
    .expect("write package.json");

    let scripts = amberjs::task_runner::load_scripts(&pkg).expect("load scripts");
    assert!(scripts.contains_key("greet"));
    assert_eq!(scripts.get("greet").unwrap(), "echo task_runner_success");

    let status = amberjs::task_runner::run_script(dir.path(), "greet", &[]).expect("run_script");
    assert!(status.success());
}

#[test]
fn test_oxc_formatter_in_memory_and_disk() {
    let unformatted = "function   calc( a,b ){ return  a*b; }";
    let formatted =
        amberjs::tooling::formatter::format_source(unformatted, "calc.js").expect("format_source");
    assert!(formatted.contains("function calc(a, b)"));

    let dir = tempdir().expect("tempdir");
    let file_path = dir.path().join("script.js");
    fs::write(&file_path, unformatted).expect("write");

    // Check mode first
    let summary = amberjs::tooling::formatter::format_paths(std::slice::from_ref(&file_path), true)
        .expect("format_paths check");
    assert_eq!(summary.formatted, 1);
    assert_eq!(fs::read_to_string(&file_path).unwrap(), unformatted);

    // Write mode
    let summary_write =
        amberjs::tooling::formatter::format_paths(std::slice::from_ref(&file_path), false)
            .expect("format_paths write");
    assert_eq!(summary_write.formatted, 1);
    let after_write = fs::read_to_string(&file_path).unwrap();
    assert_eq!(after_write, formatted);
}

#[test]
fn test_oxc_linter_detects_violations() {
    let bad_code = r#"
        function risky() {
            debugger;
            eval("console.log(1)");
            if (true) {}
            const map = { key: 1, key: 2 };
        }
    "#;

    let diags = amberjs::tooling::linter::lint_source(bad_code, "risky.js");
    let rules: Vec<&str> = diags.iter().map(|d| d.rule_name).collect();

    assert!(
        rules.contains(&"no-debugger"),
        "Expected no-debugger diagnostic"
    );
    assert!(rules.contains(&"no-eval"), "Expected no-eval diagnostic");
    assert!(rules.contains(&"no-empty"), "Expected no-empty diagnostic");
    assert!(
        rules.contains(&"no-dupe-keys"),
        "Expected no-dupe-keys diagnostic"
    );
}

#[test]
fn test_single_binary_compiler_and_sea_execution() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("entry.js");
    fs::write(&entry, "console.log('SEA_HELLO_WORLD');").expect("write entry");

    let out_bin = dir.path().join("my_standalone_tool");
    amberjs::tooling::compiler::compile_binary(&entry, &out_bin).expect("compile_binary");
    assert!(out_bin.exists());

    // Execute the standalone binary directly
    let output = Command::new(&out_bin)
        .output()
        .expect("execute standalone binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SEA_HELLO_WORLD"));
}

#[test]
fn test_coverage_collector_generates_lcov() {
    let dir = tempdir().expect("tempdir");
    let mut report = amberjs::tooling::coverage::CoverageReport::new();

    let sample_file = dir.path().join("sample.js");
    let code = "const a = 1;\nconst b = 2;\n// comment\nconst c = a + b;";
    fs::write(&sample_file, code).expect("write");

    report.record_file(&sample_file, code);

    let cov_dir = dir.path().join("cov_out");
    let lcov_path = report.write_lcov(&cov_dir).expect("write lcov");
    assert!(lcov_path.exists());

    let lcov_content = fs::read_to_string(&lcov_path).expect("read lcov");
    assert!(lcov_content.contains(&format!("SF:{}", sample_file.display())));
    assert!(lcov_content.contains("DA:1,1"));
    assert!(lcov_content.contains("DA:2,1"));
    assert!(lcov_content.contains("DA:4,1"));
    assert!(lcov_content.contains("end_of_record"));
}

#[test]
fn test_benchmark_runner_executes_sample() {
    let dir = tempdir().expect("tempdir");
    let bench_file = dir.path().join("test.bench.js");
    fs::write(
        &bench_file,
        r#"
        bench("fast addition", () => {
            let x = 1 + 2;
        });
        "#,
    )
    .expect("write bench");

    let results =
        amberjs::tooling::benchmark::run_benchmark_file(&bench_file).expect("run_benchmark_file");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "fast addition");
    assert!(results[0].ops_per_sec > 0.0);
    assert_eq!(results[0].iterations, 1000);
}

#[test]
fn test_lsp_server_full_session() {
    use serde_json::json;

    // Construct request sequence: initialize -> didOpen (with error) -> formatting -> hover -> shutdown -> exit
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    let did_open = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.js",
                "languageId": "javascript",
                "version": 1,
                "text": "function   unformatted( a,b ){ debugger; return a+b; }"
            }
        }
    });

    let format_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/formatting",
        "params": {
            "textDocument": { "uri": "file:///test.js" }
        }
    });

    let hover_req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": "file:///test.js" },
            "position": { "line": 0, "character": 11 } // hover on unformatted
        }
    });

    let shutdown_req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "shutdown"
    });

    let exit_notif = json!({
        "jsonrpc": "2.0",
        "method": "exit"
    });

    let mut input_buf = Vec::new();
    amberjs::tooling::lsp::write_lsp_message(&mut input_buf, &init_req).unwrap();
    amberjs::tooling::lsp::write_lsp_message(&mut input_buf, &did_open).unwrap();
    amberjs::tooling::lsp::write_lsp_message(&mut input_buf, &format_req).unwrap();
    amberjs::tooling::lsp::write_lsp_message(&mut input_buf, &hover_req).unwrap();
    amberjs::tooling::lsp::write_lsp_message(&mut input_buf, &shutdown_req).unwrap();
    amberjs::tooling::lsp::write_lsp_message(&mut input_buf, &exit_notif).unwrap();

    let mut output_buf = Vec::new();
    amberjs::tooling::lsp::run_lsp_server(&input_buf[..], &mut output_buf).expect("run_lsp_server");

    // Read responses
    let mut reader = std::io::BufReader::new(&output_buf[..]);

    // 1. initialize response
    let init_resp = amberjs::tooling::lsp::read_lsp_message(&mut reader)
        .unwrap()
        .unwrap();
    assert_eq!(init_resp.get("id").and_then(|id| id.as_u64()), Some(1));
    assert!(
        init_resp["result"]["capabilities"]["documentFormattingProvider"]
            .as_bool()
            .unwrap()
    );

    // 2. publishDiagnostics notification
    let diag_notif = amberjs::tooling::lsp::read_lsp_message(&mut reader)
        .unwrap()
        .unwrap();
    assert_eq!(
        diag_notif.get("method").and_then(|m| m.as_str()),
        Some("textDocument/publishDiagnostics")
    );
    let diags = diag_notif["params"]["diagnostics"].as_array().unwrap();
    assert!(diags.iter().any(|d| d["code"] == "no-debugger"));

    // 3. formatting response
    let fmt_resp = amberjs::tooling::lsp::read_lsp_message(&mut reader)
        .unwrap()
        .unwrap();
    assert_eq!(fmt_resp.get("id").and_then(|id| id.as_u64()), Some(2));
    let edits = fmt_resp["result"].as_array().unwrap();
    assert_eq!(edits.len(), 1);
    assert!(edits[0]["newText"]
        .as_str()
        .unwrap()
        .contains("function unformatted(a, b)"));

    // 4. hover response
    let hover_resp = amberjs::tooling::lsp::read_lsp_message(&mut reader)
        .unwrap()
        .unwrap();
    assert_eq!(hover_resp.get("id").and_then(|id| id.as_u64()), Some(3));

    // 5. shutdown response
    let shutdown_resp = amberjs::tooling::lsp::read_lsp_message(&mut reader)
        .unwrap()
        .unwrap();
    assert_eq!(shutdown_resp.get("id").and_then(|id| id.as_u64()), Some(4));
}

#[test]
fn test_inspector_http_and_websocket() {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use tungstenite::connect;

    let port = 19345;
    let inspector = amberjs::tooling::inspector::InspectorServer::new("127.0.0.1", port, "test.js");
    inspector.start().expect("start inspector");

    // Wait a brief moment for bind
    std::thread::sleep(std::time::Duration::from_millis(50));

    // 1. Test HTTP GET /json/version
    let mut stream = match TcpStream::connect(format!("127.0.0.1:{}", port)) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!(
                "Skipping network connect test due to sandbox restriction: {}",
                e
            );
            return;
        }
        Err(e) => panic!("connect HTTP failed: {}", e),
    };
    stream
        .write_all(b"GET /json/version HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("write GET");

    let mut resp = Vec::new();
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).expect("read");
    resp.extend_from_slice(&buf[..n]);
    let resp_str = String::from_utf8_lossy(&resp);
    assert!(resp_str.contains("200 OK"));
    assert!(resp_str.contains("Beejs/"));

    // 2. Test WebSocket connection
    let (mut socket, _) = connect(format!("ws://127.0.0.1:{}/ws", port)).expect("connect ws");

    // Should receive Debugger.paused
    let msg = socket.read().expect("read ws");
    assert!(msg.to_string().contains("Debugger.paused"));

    // Send resume
    let resume_req = r#"{"id": 1, "method": "Debugger.resume"}"#;
    socket
        .send(tungstenite::Message::Text(resume_req.to_string()))
        .expect("send resume");

    // Should receive Debugger.resumed
    let mut received_resumed = false;
    for _ in 0..3 {
        if let Ok(m) = socket.read() {
            if m.to_string().contains("Debugger.resumed") {
                received_resumed = true;
                break;
            }
        }
    }
    assert!(received_resumed);
}

#[test]
fn test_bundler_multi_module_and_minify() {
    let dir = tempdir().expect("tempdir");
    let math_file = dir.path().join("math.ts");
    fs::write(
        &math_file,
        r#"
        export function add(a: number, b: number): number {
            return a + b;
        }
        export const PI: number = 3.14159;
        "#,
    )
    .expect("write math.ts");

    let entry_file = dir.path().join("main.js");
    fs::write(
        &entry_file,
        r#"
        import { add, PI } from "./math.ts";
        const result = add(10, 20);
        globalThis.__TEST_RESULT__ = result;
        "#,
    )
    .expect("write main.js");

    let options = amberjs::tooling::bundler::BundleOptions {
        entry: entry_file.clone(),
        outfile: None,
        minify: false,
        sourcemap: true,
        target: "esnext".to_string(),
        import_map: None,
    };

    let result =
        amberjs::tooling::bundler::bundle_project(&options).expect("bundle should succeed");

    assert_eq!(result.module_count, 2);
    assert!(result.code.contains("function(modules)"));
    assert!(result.code.contains("__beejs_require__"));
    assert!(result.map.is_some());

    // Test with minification enabled
    let min_options = amberjs::tooling::bundler::BundleOptions {
        entry: entry_file,
        outfile: None,
        minify: true,
        sourcemap: false,
        target: "esnext".to_string(),
        import_map: None,
    };
    let min_result = amberjs::tooling::bundler::bundle_project(&min_options)
        .expect("minified bundle should succeed");

    assert!(min_result.code.len() <= result.code.len());
    assert!(min_result.map.is_none());
}

#[test]
fn test_import_map_parser_and_bundler() {
    let dir = tempdir().expect("tempdir");
    let map_file = dir.path().join("import_map.json");
    fs::write(
        &map_file,
        r#"{
            "imports": {
                "math": "./src/math.js",
                "helpers/": "./src/helpers/"
            }
        }"#,
    )
    .expect("write import map");

    let map = amberjs::tooling::import_map::ImportMap::load(&map_file).expect("load import map");
    let resolved = map.resolve("math", None).expect("resolve math");
    assert!(resolved.ends_with("src/math.js"));

    let resolved_helper = map
        .resolve("helpers/string.js", None)
        .expect("resolve helper");
    assert!(resolved_helper.ends_with("src/helpers/string.js"));
}

#[test]
fn test_agent_timeout_watchdog() {
    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new().expect("runtime");
    let handle = runtime.isolate_handle();

    // Watchdog thread: pulse terminate_execution every 20ms to handle any scheduling race
    let watchdog = std::thread::spawn(move || {
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            handle.terminate_execution();
        }
    });

    let start = std::time::Instant::now();
    // Run an infinite loop
    let res = runtime.execute_code("while (true) {}");
    let elapsed = start.elapsed().as_millis();
    let _ = watchdog.join();

    assert!(res.is_err(), "Infinite loop must be terminated by watchdog");
    assert!(elapsed < 1000, "Should terminate promptly around 50ms");
}

#[test]
fn test_deterministic_seed_prng() {
    amberjs::permissions::set_deterministic_seed(Some(123456789));

    let mut runtime1 = amberjs::runtime_minimal::MinimalRuntime::new().expect("runtime1");
    let val1 = runtime1.execute_code("Math.random()").expect("rand1");

    let mut runtime2 = amberjs::runtime_minimal::MinimalRuntime::new().expect("runtime2");
    let val2 = runtime2.execute_code("Math.random()").expect("rand2");

    assert_eq!(
        val1, val2,
        "Identical seed must produce identical Math.random output"
    );

    // Clean up
    amberjs::permissions::set_deterministic_seed(None);
}

#[test]
fn test_deterministic_frozen_time() {
    let fixed_ts: i64 = 1700000000000;
    amberjs::permissions::set_frozen_time_ms(Some(fixed_ts));

    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new().expect("runtime");
    let date_now = runtime.execute_code("Date.now()").expect("date now");

    assert_eq!(date_now.trim(), "1700000000000");

    // Clean up
    amberjs::permissions::set_frozen_time_ms(None);
}

#[test]
fn test_process_dlopen_interface() {
    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new().expect("runtime");
    let res = runtime.execute_code("typeof process.dlopen");
    assert_eq!(res.unwrap().trim(), "function");

    // Trying to load non-existent addon throws proper exception
    let err = runtime.execute_code("process.dlopen({}, '/non/existent/path/addon.node')");
    assert!(err.is_err());
    assert!(err
        .unwrap_err()
        .to_string()
        .contains("Cannot find native addon"));
}

#[test]
fn test_repl_multiline_block_detection() {
    assert!(amberjs::repl::is_unclosed_block("function calc() {"));
    assert!(amberjs::repl::is_unclosed_block("const list = [1, 2,"));
    assert!(amberjs::repl::is_unclosed_block("let str = `multi\nline"));
    assert!(!amberjs::repl::is_unclosed_block(
        "function calc() { return 42; }"
    ));
    assert!(!amberjs::repl::is_unclosed_block("const list = [1, 2, 3];"));
}

#[test]
fn test_web_server_fetch_handler_dispatch() {
    let mut runtime = amberjs::runtime_minimal::MinimalRuntime::new().expect("runtime");

    let bridge_init = r#"
globalThis.__beejs_app__ = undefined;
globalThis.__beejs_handle_http__ = async function(method, url, headersJson, bodyStr) {
    try {
        const headers = JSON.parse(headersJson);
        const reqInit = { method, headers };
        if (method !== "GET" && method !== "HEAD" && bodyStr && bodyStr.length > 0) {
            reqInit.body = bodyStr;
        }
        const req = new Request(url, reqInit);
        let handler = globalThis.__beejs_app__;
        if (handler && typeof handler.default === 'object' && typeof handler.default.fetch === 'function') {
            handler = handler.default.fetch.bind(handler.default);
        } else if (handler && typeof handler.default === 'function') {
            handler = handler.default;
        } else if (handler && typeof handler.fetch === 'function') {
            handler = handler.fetch;
        } else if (typeof globalThis.fetchHandler === 'function') {
            handler = globalThis.fetchHandler;
        }
        if (typeof handler !== 'function') {
            return JSON.stringify({ status: 404, headers: { "content-type": "text/plain" }, body: "Not Found: No fetch handler exported" });
        }
        const res = await handler(req);
        const status = (res && res.status) ? res.status : 200;
        const resHeaders = {};
        if (res && res.headers && typeof res.headers.forEach === 'function') {
            res.headers.forEach((v, k) => { resHeaders[k] = v; });
        }
        let bodyText = "";
        if (res) {
            if (typeof res._bodyText === 'string') {
                bodyText = res._bodyText;
            } else if (typeof res.text === 'function') {
                try {
                    bodyText = await res.text();
                } catch (_) {
                    bodyText = res.body ? String(res.body) : "";
                }
            } else {
                bodyText = res.body ? String(res.body) : "";
            }
        }
        return JSON.stringify({ status, headers: resHeaders, body: bodyText });
    } catch (e) {
        return JSON.stringify({ status: 500, headers: { "content-type": "text/plain" }, body: "Internal Server Error: " + (e ? e.message : e) });
    }
};
"#;
    runtime.execute_code(bridge_init).expect("bridge init");

    // 1. Test synchronous export default { fetch(req) }
    let app_code = r#"
globalThis.__beejs_app__ = {
    default: {
        fetch(req) {
            return new Response("Hello Beejs Server!", {
                status: 201,
                headers: { "x-custom-powered": "beejs-runtime" }
            });
        }
    }
};
"#;
    runtime.execute_code(app_code).expect("app code");

    let dispatch_script =
        r#"globalThis.__beejs_handle_http__("GET", "http://localhost:3000/hello", "{}", "");"#;
    let resp_str = runtime.execute_code(dispatch_script).expect("dispatch");
    let resp_val: serde_json::Value = serde_json::from_str(resp_str.trim()).expect("parse json");

    assert_eq!(resp_val["status"].as_u64().unwrap(), 201);
    assert_eq!(resp_val["body"].as_str().unwrap(), "Hello Beejs Server!");
    assert_eq!(
        resp_val["headers"]["x-custom-powered"].as_str().unwrap(),
        "beejs-runtime"
    );

    // 2. Test async handler with JSON request body
    let async_app_code = r#"
globalThis.__beejs_app__ = {
    default: {
        async fetch(req) {
            const text = await req.text();
            return new Response("Echo: " + text, { status: 200 });
        }
    }
};
"#;
    runtime.execute_code(async_app_code).expect("async app");
    let dispatch_async = r#"globalThis.__beejs_handle_http__("POST", "http://localhost:3000/echo", "{}", "ping-data");"#;
    let async_resp_str = runtime
        .execute_code(dispatch_async)
        .expect("dispatch async");
    let async_resp_val: serde_json::Value =
        serde_json::from_str(async_resp_str.trim()).expect("parse async json");

    assert_eq!(async_resp_val["status"].as_u64().unwrap(), 200);
    assert_eq!(async_resp_val["body"].as_str().unwrap(), "Echo: ping-data");
}
