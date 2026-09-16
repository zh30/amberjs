// WebAssembly 2.0 Zero-Copy Shared Memory Subsystem Integration Tests (bee:wasm)
// Tests zero-copy virtual address sharing between WebAssembly.Memory, V8 ArrayBuffer, TypedArrays, and Tensors.

use beejs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction, MemorySection,
    MemoryType, Module, TypeSection, ValType,
};

fn run_wasm_script(script: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("Failed to create minimal runtime");
    runtime
        .execute_code(script)
        .expect("WASM script should execute successfully")
        .trim()
        .to_string()
}

/// Helper to generate a WASM module that imports or exports memory and processes an array in-place.
/// Exports:
/// - "process_vector": (ptr: i32, len: i32, addend: i32) -> i32 (increments each byte by addend)
/// - "dot_product_f32": (ptrA: i32, ptrB: i32, count: i32) -> f32 (computes float32 dot product)
fn build_test_wasm_module() -> Vec<u8> {
    let mut module = Module::new();

    // 1. Type section
    let mut types = TypeSection::new();
    // type 0: (i32, i32, i32) -> i32
    types.ty().function(
        vec![ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );
    // type 1: (i32, i32, i32) -> f32
    types.ty().function(
        vec![ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::F32],
    );
    module.section(&types);

    // 2. Function section
    let mut funcs = FunctionSection::new();
    funcs.function(0); // process_vector
    funcs.function(1); // dot_product_f32
    module.section(&funcs);

    // 3. Memory section: export memory (1 page = 64KB, max 16 pages = 1MB)
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: Some(16),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    module.section(&memories);

    // 4. Export section
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("process_vector", ExportKind::Func, 0);
    exports.export("dot_product_f32", ExportKind::Func, 1);
    module.section(&exports);

    // 5. Code section
    let mut code = CodeSection::new();

    // Function 0: process_vector(ptr, len, addend) -> returns sum of processed bytes
    // locals: 0:ptr, 1:len, 2:addend, 3:i (loop index), 4:total (accumulator)
    let mut func0 = Function::new(vec![(2, ValType::I32)]);
    // i = 0
    func0.instruction(&Instruction::I32Const(0));
    func0.instruction(&Instruction::LocalSet(3));
    // total = 0
    func0.instruction(&Instruction::I32Const(0));
    func0.instruction(&Instruction::LocalSet(4));

    // Loop
    func0.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    func0.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

    // if i >= len break
    func0.instruction(&Instruction::LocalGet(3));
    func0.instruction(&Instruction::LocalGet(1));
    func0.instruction(&Instruction::I32GeU);
    func0.instruction(&Instruction::BrIf(1));

    // load byte at (ptr + i)
    func0.instruction(&Instruction::LocalGet(0));
    func0.instruction(&Instruction::LocalGet(3));
    func0.instruction(&Instruction::I32Add);
    func0.instruction(&Instruction::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));

    // val = byte + addend
    func0.instruction(&Instruction::LocalGet(2));
    func0.instruction(&Instruction::I32Add);

    // store byte at (ptr + i)
    // Note: store requires [addr, value]
    // compute addr again
    func0.instruction(&Instruction::LocalGet(0));
    func0.instruction(&Instruction::LocalGet(3));
    func0.instruction(&Instruction::I32Add);
    // swap stack to have [addr, val]
    // To be clean: local.tee val into total accumulator
    func0.instruction(&Instruction::LocalGet(0));
    func0.instruction(&Instruction::LocalGet(3));
    func0.instruction(&Instruction::I32Add);

    // re-read and add addend
    func0.instruction(&Instruction::LocalGet(0));
    func0.instruction(&Instruction::LocalGet(3));
    func0.instruction(&Instruction::I32Add);
    func0.instruction(&Instruction::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    func0.instruction(&Instruction::LocalGet(2));
    func0.instruction(&Instruction::I32Add);

    // store 8 bits
    func0.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));

    // i = i + 1
    func0.instruction(&Instruction::LocalGet(3));
    func0.instruction(&Instruction::I32Const(1));
    func0.instruction(&Instruction::I32Add);
    func0.instruction(&Instruction::LocalSet(3));

    func0.instruction(&Instruction::Br(0));
    func0.instruction(&Instruction::End); // loop
    func0.instruction(&Instruction::End); // block

    // return len
    func0.instruction(&Instruction::LocalGet(1));
    func0.instruction(&Instruction::End);
    code.function(&func0);

    // Function 1: dot_product_f32(ptrA, ptrB, count) -> f32
    // locals: 3:i (loop index), 4:sum (f32 accumulator)
    let mut func1 = Function::new(vec![(1, ValType::I32), (1, ValType::F32)]);
    // i = 0
    func1.instruction(&Instruction::I32Const(0));
    func1.instruction(&Instruction::LocalSet(3));
    // sum = 0.0f
    func1.instruction(&Instruction::F32Const(0.0.into()));
    func1.instruction(&Instruction::LocalSet(4));

    func1.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    func1.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

    // if i >= count break
    func1.instruction(&Instruction::LocalGet(3));
    func1.instruction(&Instruction::LocalGet(2));
    func1.instruction(&Instruction::I32GeU);
    func1.instruction(&Instruction::BrIf(1));

    // sum = sum + (load f32 from ptrA + i * 4) * (load f32 from ptrB + i * 4)
    func1.instruction(&Instruction::LocalGet(4));

    // load f32 from ptrA + i * 4
    func1.instruction(&Instruction::LocalGet(0));
    func1.instruction(&Instruction::LocalGet(3));
    func1.instruction(&Instruction::I32Const(4));
    func1.instruction(&Instruction::I32Mul);
    func1.instruction(&Instruction::I32Add);
    func1.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));

    // load f32 from ptrB + i * 4
    func1.instruction(&Instruction::LocalGet(1));
    func1.instruction(&Instruction::LocalGet(3));
    func1.instruction(&Instruction::I32Const(4));
    func1.instruction(&Instruction::I32Mul);
    func1.instruction(&Instruction::I32Add);
    func1.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));

    // mul
    func1.instruction(&Instruction::F32Mul);
    // add to sum
    func1.instruction(&Instruction::F32Add);
    func1.instruction(&Instruction::LocalSet(4));

    // i = i + 1
    func1.instruction(&Instruction::LocalGet(3));
    func1.instruction(&Instruction::I32Const(1));
    func1.instruction(&Instruction::I32Add);
    func1.instruction(&Instruction::LocalSet(3));

    func1.instruction(&Instruction::Br(0));
    func1.instruction(&Instruction::End); // loop
    func1.instruction(&Instruction::End); // block

    // return sum
    func1.instruction(&Instruction::LocalGet(4));
    func1.instruction(&Instruction::End);
    code.function(&func1);

    module.section(&code);
    module.finish()
}

#[test]
#[serial]
fn test_wasm_memory_pointer_and_prototype_extensions() {
    let script = r#"
    const mem = new WebAssembly.Memory({ initial: 1 });
    const p1 = mem.ptr;
    const p2 = mem.getPointer();
    const ab = mem.buffer;
    const ab_p = ab.ptr;
    const u8 = new Uint8Array(ab);
    const u8_p = u8.ptr;

    [
        typeof p1 === 'bigint',
        p1 > 0n,
        p1 === p2,
        p1 === ab_p,
        p1 === u8_p
    ].every(Boolean);
    "#;

    let output = run_wasm_script(script);
    assert_eq!(output, "true");
}

#[test]
#[serial]
fn test_wasm_zero_copy_array_buffer_aliasing() {
    let script = r#"
    const mem = new WebAssembly.Memory({ initial: 2 }); // 128KB
    const zeroCopyBuf = mem.createZeroCopyBuffer(1024, 64);
    
    // Both views point to the exact same physical memory offset 1024
    const memView = new Uint8Array(mem.buffer, 1024, 64);
    const sliceView = new Uint8Array(zeroCopyBuf);

    memView[0] = 42;
    memView[1] = 99;
    
    const readFromSlice = sliceView[0] === 42 && sliceView[1] === 99;

    // Mutate via sliceView, should reflect in memView instantly (zero-copy)
    sliceView[0] = 123;
    sliceView[1] = 234;

    const readFromMem = memView[0] === 123 && memView[1] === 234;

    `${readFromSlice}:${readFromMem}:${zeroCopyBuf.byteLength}`;
    "#;

    let output = run_wasm_script(script);
    assert_eq!(output, "true:true:64");
}

#[test]
#[serial]
fn test_wasm_in_place_vector_execution_zero_copy() {
    let wasm_bytes = build_test_wasm_module();
    let hex_bytes = wasm_bytes
        .iter()
        .map(|b| format!("0x{:02x}", b))
        .collect::<Vec<_>>()
        .join(", ");

    let script = format!(
        r#"
    const bytes = new Uint8Array([{hex_bytes}]);
    const module = new WebAssembly.Module(bytes);
    const instance = new WebAssembly.Instance(module);

    const mem = instance.exports.memory;
    // Create zero-copy views directly from memory
    const zeroCopyView = mem.asUint8Array(100, 10);

    // Initialize 10 bytes: [10, 11, 12, 13, 14, 15, 16, 17, 18, 19]
    for (let i = 0; i < 10; i++) {{
        zeroCopyView[i] = 10 + i;
    }}

    // Call WASM function to increment all 10 bytes by +5 in-place
    const processed = instance.exports.process_vector(100, 10, 5);

    // JS directly reads the result with zero copies
    const results = [];
    for (let i = 0; i < 10; i++) {{
        results.push(zeroCopyView[i]);
    }}

    `${{processed}}:${{results.join(',')}}`;
    "#
    );

    let output = run_wasm_script(&script);
    assert_eq!(output, "10:15,16,17,18,19,20,21,22,23,24");
}

#[test]
#[serial]
fn test_wasm_float32_simd_vector_dot_product() {
    let wasm_bytes = build_test_wasm_module();
    let hex_bytes = wasm_bytes
        .iter()
        .map(|b| format!("0x{:02x}", b))
        .collect::<Vec<_>>()
        .join(", ");

    let script = format!(
        r#"
    const bytes = new Uint8Array([{hex_bytes}]);
    const module = new WebAssembly.Module(bytes);
    const instance = new WebAssembly.Instance(module);

    const mem = instance.exports.memory;

    // Allocate two float32 vectors in WASM memory:
    // vecA at offset 1024, vecB at offset 2048, length 4 elements
    const vecA = mem.asFloat32Array(1024, 4);
    const vecB = mem.asFloat32Array(2048, 4);

    vecA[0] = 1.0; vecA[1] = 2.0; vecA[2] = 3.0; vecA[3] = 4.0;
    vecB[0] = 2.0; vecB[1] = 3.0; vecB[2] = 4.0; vecB[3] = 5.0;

    // Dot product: 1*2 + 2*3 + 3*4 + 4*5 = 2 + 6 + 12 + 20 = 40.0
    const dot = instance.exports.dot_product_f32(1024, 2048, 4);

    // Modify vecA[0] in-place via zero-copy alias
    vecA[0] = 10.0;
    // New Dot product: 10*2 + 2*3 + 3*4 + 4*5 = 20 + 6 + 12 + 20 = 58.0
    const dot2 = instance.exports.dot_product_f32(1024, 2048, 4);

    `${{dot}}:${{dot2}}`;
    "#
    );

    let output = run_wasm_script(&script);
    assert_eq!(output, "40:58");
}

#[test]
#[serial]
fn test_wasm_shared_memory_and_atomics() {
    let script = r#"
    const mem = wasm.createSharedMemory({ initial: 1, maximum: 2 });
    const zeroCopyBuf = mem.createZeroCopyBuffer(0, 64);
    
    const sharedU32 = new Int32Array(zeroCopyBuf);
    Atomics.store(sharedU32, 0, 4242);
    
    const loaded = Atomics.load(sharedU32, 0);
    const directU32 = new Int32Array(mem.buffer);
    const directLoaded = Atomics.load(directU32, 0);

    `${zeroCopyBuf instanceof SharedArrayBuffer}:${loaded}:${directLoaded}`;
    "#;

    let output = run_wasm_script(script);
    assert_eq!(output, "true:4242:4242");
}

#[test]
#[serial]
fn test_wasm_module_mmap_zero_copy_execution() {
    let wasm_bytes = build_test_wasm_module();
    let dir = tempdir().expect("Failed to create temp dir");
    let file_path = dir.path().join("vector.wasm");

    let mut file = File::create(&file_path).expect("Failed to create file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write wasm bytes");
    file.sync_all().expect("Failed to sync file");

    let path_str = file_path.to_str().unwrap().replace('\\', "/");
    let script = format!(
        r#"
    let result = null;
    wasm.loadModuleMmap('{path_str}').then(module => {{
        const instance = new WebAssembly.Instance(module);
        const mem = instance.exports.memory;
        const view = mem.asUint8Array(0, 5);
        view[0] = 1; view[1] = 2; view[2] = 3; view[3] = 4; view[4] = 5;
        instance.exports.process_vector(0, 5, 10);
        result = `${{view[0]}},${{view[1]}},${{view[2]}},${{view[3]}},${{view[4]}}`;
    }});
    "#
    );

    let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
    runtime
        .execute_code(&script)
        .expect("mmap script execution failed");

    let check = runtime.execute_code("result").unwrap();
    assert_eq!(check, "11,12,13,14,15");
}

#[test]
#[serial]
fn test_wasm_share_memory_facade_and_builtin_require() {
    let script = r#"
    const wasmModule = require('bee:wasm');
    const wasmNative = require('wasm');

    const mem = new WebAssembly.Memory({ initial: 1 });
    const shared = wasmModule.shareMemory(mem);

    shared.uint8[0] = 77;
    shared.float32[1] = 3.14; // offset 4..8
    shared.int32[2] = 999;    // offset 8..12

    const u8_check = mem.asUint8Array(0, 1)[0] === 77;
    const f32_check = Math.abs(mem.asFloat32Array(4, 1)[0] - 3.14) < 0.001;
    const i32_check = mem.asInt32Array(8, 1)[0] === 999;

    [
        wasmModule === wasmNative,
        wasmModule.version === '2.0.0',
        shared.ptr > 0n,
        u8_check,
        f32_check,
        i32_check
    ].every(Boolean);
    "#;

    let output = run_wasm_script(script);
    assert_eq!(output, "true");
}
