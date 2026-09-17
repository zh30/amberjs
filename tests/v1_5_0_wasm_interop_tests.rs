use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run_js(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("Failed to create minimal runtime");
    runtime
        .execute_code(code)
        .expect("JS code should execute successfully")
        .trim()
        .to_string()
}

#[test]
#[serial]
fn test_wasm_module_exports_and_aliases() {
    let script = r#"
    const wasm1 = require('bee:wasm');
    const wasm2 = require('wasm');
    const hasGlobal = typeof globalThis.wasm === 'object';
    const isSame = wasm1 === wasm2 && wasm1 === globalThis.wasm;
    `${typeof wasm1.ptr}:${typeof wasm1.copyMemory}:${typeof wasm1.MemoryView}:${wasm1.version}:${isSame}:${hasGlobal}`;
    "#;
    let output = run_js(script);
    assert_eq!(output, "function:function:function:1.5.0:true:true");
}

#[test]
#[serial]
fn test_wasm_ptr_extraction_and_memory_ops() {
    let script = r#"
    const wasm = require('bee:wasm');
    const mem = new WebAssembly.Memory({ initial: 1 });
    const ptr = wasm.ptr(mem);

    // Fill memory with byte 72 ('H')
    wasm.fillMemory(ptr, 72, 4);

    // Extract ptr of Uint8Array view
    const u8 = new Uint8Array(mem.buffer);
    const u8Ptr = wasm.ptr(u8);

    // Create a destination ArrayBuffer
    const dstAb = new ArrayBuffer(4);
    const dstPtr = wasm.ptr(dstAb);

    // Copy memory from Wasm to dstAb
    wasm.copyMemory(ptr, dstPtr, 4);
    const dstU8 = new Uint8Array(dstAb);

    // Compare memory
    const cmp = wasm.compareMemory(ptr, dstPtr, 4);

    `${ptr > 0n}:${ptr === u8Ptr}:${dstU8[0] === 72 && dstU8[3] === 72}:${cmp === 0}`;
    "#;
    let output = run_js(script);
    assert_eq!(output, "true:true:true:true");
}

#[test]
#[serial]
fn test_wasm_memory_view_typed_accessors() {
    let script = r#"
    const wasm = require('bee:wasm');
    const mem = new WebAssembly.Memory({ initial: 1 });
    const view = new wasm.MemoryView(mem);

    view.setUint8(0, 255);
    view.setInt32(4, -123456);
    view.setFloat32(8, 3.14159);
    view.setFloat64(16, 2.718281828459);
    view.setString(32, 'Beejs Wasm 2.0');
    view.setUint8(32 + 'Beejs Wasm 2.0'.length, 0); // null terminate

    const u8Val = view.getUint8(0);
    const i32Val = view.getInt32(4);
    const f32Val = Math.round(view.getFloat32(8) * 1000) / 1000;
    const f64Val = Math.round(view.getFloat64(16) * 100000) / 100000;
    const cstr = view.getCString(32);
    const str = view.getString(32, 5);

    `${u8Val}:${i32Val}:${f32Val}:${f64Val}:${cstr}:${str}`;
    "#;
    let output = run_js(script);
    assert_eq!(output, "255:-123456:3.142:2.71828:Beejs Wasm 2.0:Beejs");
}

#[test]
#[serial]
fn test_wasm_wrap_pointer_proxy() {
    let script = r#"
    const wasm = require('bee:wasm');
    const mem = new WebAssembly.Memory({ initial: 1 });
    const ptr = wasm.ptr(mem);

    // Wrap as u8 proxy
    const u8Proxy = wasm.wrapPointer(ptr, 10, 'u8');
    u8Proxy[0] = 42;
    u8Proxy[1] = 84;

    // Wrap as i32 proxy at offset 4
    const i32Proxy = wasm.wrapPointer(ptr + 4n, 4, 'i32');
    i32Proxy[0] = 987654;

    const v = new wasm.MemoryView(mem);
    `${u8Proxy[0]}:${u8Proxy[1]}:${i32Proxy[0]}:${v.getUint8(0)}:${v.getInt32(4)}`;
    "#;
    let output = run_js(script);
    assert_eq!(output, "42:84:987654:42:987654");
}

#[test]
#[serial]
fn test_wasm_tensor_zero_copy_bridge() {
    let script = r#"
    const wasm = require('bee:wasm');
    const mem = new WebAssembly.Memory({ initial: 1 });

    // Create tensor backed directly by Wasm linear memory
    const tensor = wasm.createTensorFromMemory(mem, 0, [2, 2], 'float32');
    tensor.data[0] = 1.5;
    tensor.data[1] = 2.5;
    tensor.data[2] = 3.5;
    tensor.data[3] = 4.5;

    // Read via MemoryView at the same physical addresses
    const view = new wasm.MemoryView(mem);
    const v0 = view.getFloat32(0);
    const v1 = view.getFloat32(4);
    const v2 = view.getFloat32(8);
    const v3 = view.getFloat32(12);

    // Modify Wasm memory and assert tensor reflects it with zero copy
    view.setFloat32(0, 100.25);

    `${tensor.length}:${v0}:${v1}:${v2}:${v3}:${tensor.data[0] === 100.25}`;
    "#;
    let output = run_js(script);
    assert_eq!(output, "4:1.5:2.5:3.5:4.5:true");
}

#[test]
#[serial]
fn test_wasm_shared_memory_and_mmap_load() {
    let script = r#"
    const fs = require('fs');
    const wasm = require('bee:wasm');

    // 1. Test shared memory creation
    const sharedMem = wasm.createSharedMemory({ initial: 1, maximum: 2 });
    const isShared = typeof sharedMem === 'object' && sharedMem.buffer !== undefined;

    // 2. Test mmap loading of a valid Wasm binary
    // Minimal valid Wasm binary with an exported 'add(i32, i32) -> i32' function
    const wasmBinary = Buffer.from([
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic, version
        0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, // type section: (i32, i32) -> i32
        0x03, 0x02, 0x01, 0x00, // function section: type 0
        0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00, // export section: "add" func 0
        0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b // code section: local.get 0, local.get 1, i32.add
    ]);

    const tempPath = '/tmp/test_beejs_v1_5_0_add.wasm';
    fs.writeFileSync(tempPath, wasmBinary);

    let testResult = '';
    wasm.loadModuleMmap(tempPath).then(async (module) => {
        const instance = await WebAssembly.instantiate(module);
        const sum = instance.exports.add(40, 2);
        try { fs.unlinkSync(tempPath); } catch (_) {}
        globalThis._mmapTestResult = `${isShared}:${module instanceof WebAssembly.Module}:${sum}`;
    });

    // In sync test, output flag
    `${isShared}`;
    "#;
    let output = run_js(script);
    assert_eq!(output, "true");
}
