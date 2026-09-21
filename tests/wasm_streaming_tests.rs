// WebAssembly 2.0 Web Standards & Streaming Integration Tests
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_wasm_instantiate_streaming() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
const bytes = new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f,
    0x03, 0x02, 0x01, 0x00,
    0x07, 0x08, 0x01, 0x04, 0x6d, 0x61, 0x69, 0x6e, 0x00, 0x00,
    0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b
]);

let wasmResult = null;
const response = new Response(bytes);
WebAssembly.instantiateStreaming(response).then(({ module, instance }) => {
    if (module instanceof WebAssembly.Module && instance instanceof WebAssembly.Instance) {
        wasmResult = instance.exports.main();
    }
});
"#;

    let result = runtime.execute_code(code);
    assert!(
        result.is_ok(),
        "instantiateStreaming failed: {:?}",
        result.err()
    );

    let check = runtime.execute_code("wasmResult");
    assert_eq!(check.unwrap(), "42");
}

#[test]
#[serial]
fn test_wasm_compile_streaming() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
const bytes = new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f,
    0x03, 0x02, 0x01, 0x00,
    0x07, 0x08, 0x01, 0x04, 0x6d, 0x61, 0x69, 0x6e, 0x00, 0x00,
    0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b // 42
]);

let compiledResult = null;
const responsePromise = Promise.resolve(new Response(bytes));
WebAssembly.compileStreaming(responsePromise).then(module => {
    const instance = new WebAssembly.Instance(module);
    compiledResult = instance.exports.main();
});
"#;

    let result = runtime.execute_code(code);
    assert!(
        result.is_ok(),
        "compileStreaming failed: {:?}",
        result.err()
    );

    let check = runtime.execute_code("compiledResult");
    assert_eq!(check.unwrap(), "42");
}

#[test]
#[serial]
fn test_wasm_streaming_bad_status_throws_type_error() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let code = r#"
let errorName = null;
const badResponse = new Response(new Uint8Array([]), { status: 404, statusText: "Not Found" });
WebAssembly.instantiateStreaming(badResponse).catch(err => {
    errorName = err.name;
});
"#;

    let result = runtime.execute_code(code);
    assert!(
        result.is_ok(),
        "bad response test failed: {:?}",
        result.err()
    );

    let check = runtime.execute_code("errorName");
    assert_eq!(check.unwrap(), "TypeError");
}
