// Multi-Isolate Worker Threads & Web Workers 2.0 Integration Tests
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[test]
#[serial]
fn test_worker_threads_basic_execution() {
    let dir = tempdir().expect("Failed to create tempdir");
    let worker_file = dir.path().join("worker.js");
    fs::write(
        &worker_file,
        r#"
const { parentPort, workerData, isMainThread } = require('worker_threads');
if (isMainThread) {
    throw new Error('Worker should not have isMainThread === true');
}
const n = workerData.n;
function fib(x) {
    return x <= 1 ? x : fib(x - 1) + fib(x - 2);
}
parentPort.postMessage({ result: fib(n) });
"#,
    )
    .expect("Failed to write worker.js");

    let main_file = dir.path().join("main.js");
    let worker_path_str = worker_file.to_str().unwrap().replace('\\', "/");
    fs::write(
        &main_file,
        format!(
            r#"
const {{ Worker, isMainThread }} = require('worker_threads');
if (!isMainThread) {{
    throw new Error('Main should have isMainThread === true');
}}

let received = null;
const worker = new Worker('{worker_path}', {{ workerData: {{ n: 10 }} }});
worker.on('message', (msg) => {{
    received = msg.result;
}});
"#,
            worker_path = worker_path_str
        ),
    )
    .expect("Failed to write main.js");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_file);
    let code = fs::read_to_string(&main_file).unwrap();
    let result = runtime.execute_code(&code);
    assert!(
        result.is_ok(),
        "Worker execution failed: {:?}",
        result.err()
    );

    // Verify received variable
    let check = runtime.execute_code("received");
    assert_eq!(check.unwrap(), "55");
}

#[test]
#[serial]
fn test_worker_threads_bidirectional_ping_pong() {
    let dir = tempdir().expect("Failed to create tempdir");
    let worker_file = dir.path().join("echo.js");
    fs::write(
        &worker_file,
        r#"
const { parentPort } = require('worker_threads');
parentPort.on('message', (msg) => {
    parentPort.postMessage({ reply: msg.text.toUpperCase() });
});
"#,
    )
    .expect("Failed to write echo.js");

    let main_file = dir.path().join("main.js");
    let worker_path_str = worker_file.to_str().unwrap().replace('\\', "/");
    fs::write(
        &main_file,
        format!(
            r#"
const {{ Worker }} = require('worker_threads');
let replyReceived = null;
const worker = new Worker('{worker_path}');

worker.on('message', (msg) => {{
    replyReceived = msg.reply;
    worker.terminate();
}});

worker.postMessage({{ text: 'hello beejs' }});
"#,
            worker_path = worker_path_str
        ),
    )
    .expect("Failed to write main.js");

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime.set_main_module_path(&main_file);
    let code = fs::read_to_string(&main_file).unwrap();
    let result = runtime.execute_code(&code);
    assert!(result.is_ok(), "Ping-pong failed: {:?}", result.err());

    let check = runtime.execute_code("replyReceived");
    assert_eq!(check.unwrap(), "HELLO BEEJS");
}

#[test]
#[serial]
fn test_web_worker_standards_compatibility() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
let webWorkerResult = null;
const inlineWorkerCode = "self.onmessage = (e) => { postMessage({ doubled: e.data.val * 2 }); };";
const worker = new Worker(inlineWorkerCode, { eval: true });
worker.onmessage = (e) => {
    webWorkerResult = e.data.doubled;
    worker.terminate();
};
worker.postMessage({ val: 21 });
"#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok(), "Web Worker test failed: {:?}", result.err());

    let check = runtime.execute_code("webWorkerResult");
    assert_eq!(check.unwrap(), "42");
}

#[test]
#[serial]
fn test_worker_threads_parallel_concurrency() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
const { Worker } = require('worker_threads');
const workerCode = `
const { parentPort, workerData } = require('worker_threads');
let sum = 0;
for (let i = 0; i <= workerData.count; i++) {
    sum += i;
}
parentPort.postMessage({ id: workerData.id, sum });
`;

let finished = 0;
const results = {};

for (let i = 1; i <= 3; i++) {
    const w = new Worker(workerCode, { eval: true, workerData: { id: i, count: i * 100 } });
    w.on('message', (msg) => {
        results[msg.id] = msg.sum;
        finished++;
    });
}
"#;

    let result = runtime.execute_code(code);
    assert!(
        result.is_ok(),
        "Parallel workers failed: {:?}",
        result.err()
    );

    let check_finished = runtime.execute_code("finished").unwrap();
    assert_eq!(check_finished, "3");

    let r1 = runtime.execute_code("results[1]").unwrap();
    assert_eq!(r1, "5050"); // sum 1..100 = 5050
    let r2 = runtime.execute_code("results[2]").unwrap();
    assert_eq!(r2, "20100"); // sum 1..200 = 20100
    let r3 = runtime.execute_code("results[3]").unwrap();
    assert_eq!(r3, "45150"); // sum 1..300 = 45150
}
