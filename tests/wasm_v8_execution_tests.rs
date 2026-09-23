use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run_wasm_script(script: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("Failed to create minimal runtime");
    runtime
        .execute_code(script)
        .expect("WASM script should execute successfully")
        .trim()
        .to_string()
}

#[test]
#[serial]
fn test_v8_wasm_global_object_exists() {
    let output = run_wasm_script("typeof WebAssembly");
    assert_eq!(output, "object");
}

#[test]
#[serial]
fn test_v8_wasm_compile_and_instantiate_add_function() {
    // WASM module bytecode exporting add(i32, i32) -> i32
    let script = r#"
    const bytes = new Uint8Array([
        0, 97, 115, 109, 1, 0, 0, 0,
        1, 7, 1, 96, 2, 127, 127, 1, 127,
        3, 2, 1, 0,
        7, 7, 1, 3, 97, 100, 100, 0, 0,
        10, 9, 1, 7, 0, 32, 0, 32, 1, 106, 11
    ]);
    const mod = new WebAssembly.Module(bytes);
    const inst = new WebAssembly.Instance(mod);
    inst.exports.add(25, 17);
    "#;

    let output = run_wasm_script(script);
    assert_eq!(output, "42");
}

#[test]
#[serial]
fn test_v8_wasm_memory_allocation_and_sharing() {
    let script = r#"
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 2 });
    const buffer = new Uint8Array(memory.buffer);
    buffer[0] = 123;
    buffer[1] = 234;
    `${memory.buffer.byteLength}:${buffer[0]}:${buffer[1]}`;
    "#;

    let output = run_wasm_script(script);
    assert_eq!(output, "65536:123:234");
}

#[test]
#[serial]
fn test_v8_wasm_invalid_bytecode_throws_compile_error() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    let script = r#"
    const badBytes = new Uint8Array([0, 0, 0, 0]);
    new WebAssembly.Module(badBytes);
    "#;

    let result = runtime.execute_code(script);
    assert!(
        result.is_err(),
        "Invalid WASM bytecode should throw CompileError"
    );
}
