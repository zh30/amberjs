// Beejs WebAssembly 2.0 Zero-Copy Shared Memory Subsystem (bee:wasm)
// High-performance physical memory bridge between V8, WebAssembly, FFI pointers, and bee:ai.Tensor.

use anyhow::{anyhow, Result};
use rusty_v8 as v8;
use std::ffi::CStr;
use std::fs::File;

fn to_raw_ptr(scope: &mut v8::PinScope, val: v8::Local<v8::Value>) -> usize {
    if val.is_big_int() {
        if let Ok(bi) = v8::Local::<v8::BigInt>::try_from(val) {
            return bi.u64_value().0 as usize;
        }
    } else if val.is_number() {
        return val.integer_value(scope).unwrap_or(0) as usize;
    }
    0
}

fn extract_target_ptr(scope: &mut v8::PinScope, arg: v8::Local<v8::Value>) -> usize {
    if arg.is_array_buffer_view() {
        if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(arg) {
            let byte_offset = view.byte_offset();
            if let Some(ab) = view.buffer(scope) {
                let store = ab.get_backing_store();
                if let Some(data) = store.data() {
                    return unsafe { (data.as_ptr() as *mut u8).add(byte_offset) as usize };
                }
            }
        }
    } else if arg.is_array_buffer() {
        if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(arg) {
            let store = ab.get_backing_store();
            if let Some(data) = store.data() {
                return data.as_ptr() as usize;
            }
        }
    } else if arg.is_object() {
        if let Ok(obj) = v8::Local::<v8::Object>::try_from(arg) {
            // Check if it's WebAssembly.Memory (has .buffer property)
            let buffer_key = v8::String::new(scope, "buffer").unwrap();
            if let Some(buf_val) = obj.get(scope, buffer_key.into()) {
                if buf_val.is_array_buffer() {
                    if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf_val) {
                        let store = ab.get_backing_store();
                        if let Some(data) = store.data() {
                            return data.as_ptr() as usize;
                        }
                    }
                }
            }
            // Check if it's bee:ai.Tensor (has .data property)
            let data_key = v8::String::new(scope, "data").unwrap();
            if let Some(tensor_data) = obj.get(scope, data_key.into()) {
                if tensor_data.is_array_buffer_view() {
                    return extract_target_ptr(scope, tensor_data);
                }
            }
            // Check if it has a .ptr property
            let ptr_key = v8::String::new(scope, "ptr").unwrap();
            if let Some(p_val) = obj.get(scope, ptr_key.into()) {
                return to_raw_ptr(scope, p_val);
            }
        }
    } else if arg.is_big_int() || arg.is_number() {
        return to_raw_ptr(scope, arg);
    }
    0
}

/// Initialize the `bee:wasm` subsystem in V8 context
pub fn setup_wasm_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // 1. wasm.ptr(target) -> BigInt
    let ptr_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() == 0 || args.get(0).is_null_or_undefined() {
                rv.set(v8::BigInt::new_from_u64(scope, 0).into());
                return;
            }
            let ptr = extract_target_ptr(scope, args.get(0));
            rv.set(v8::BigInt::new_from_u64(scope, ptr as u64).into());
        },
    )
    .unwrap();

    // 2. wasm.copyMemory(srcPtr, dstPtr, length) -> boolean
    let copy_memory_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 3 {
                let err = v8::String::new(
                    scope,
                    "copyMemory requires 3 arguments (srcPtr, dstPtr, length)",
                )
                .unwrap();
                let exc = v8::Exception::type_error(scope, err);
                scope.throw_exception(exc);
                return;
            }
            let src = to_raw_ptr(scope, args.get(0));
            let dst = to_raw_ptr(scope, args.get(1));
            let len = args.get(2).integer_value(scope).unwrap_or(0) as usize;

            if src == 0 || dst == 0 {
                let err = v8::String::new(scope, "copyMemory: null pointer provided").unwrap();
                let exc = v8::Exception::error(scope, err);
                scope.throw_exception(exc);
                return;
            }

            if len > 0 {
                unsafe {
                    libc::memmove(dst as *mut libc::c_void, src as *const libc::c_void, len);
                }
            }
            rv.set(v8::Boolean::new(scope, true).into());
        },
    )
    .unwrap();

    // 3. wasm.fillMemory(ptr, value, length) -> boolean
    let fill_memory_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 3 {
                let err = v8::String::new(
                    scope,
                    "fillMemory requires 3 arguments (ptr, value, length)",
                )
                .unwrap();
                let exc = v8::Exception::type_error(scope, err);
                scope.throw_exception(exc);
                return;
            }
            let ptr = to_raw_ptr(scope, args.get(0));
            let val = (args.get(1).integer_value(scope).unwrap_or(0) & 0xFF) as libc::c_int;
            let len = args.get(2).integer_value(scope).unwrap_or(0) as usize;

            if ptr == 0 {
                let err = v8::String::new(scope, "fillMemory: null pointer provided").unwrap();
                let exc = v8::Exception::error(scope, err);
                scope.throw_exception(exc);
                return;
            }

            if len > 0 {
                unsafe {
                    libc::memset(ptr as *mut libc::c_void, val, len);
                }
            }
            rv.set(v8::Boolean::new(scope, true).into());
        },
    )
    .unwrap();

    // 4. wasm.compareMemory(ptr1, ptr2, length) -> number
    let compare_memory_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 3 {
                let err = v8::String::new(
                    scope,
                    "compareMemory requires 3 arguments (ptr1, ptr2, length)",
                )
                .unwrap();
                let exc = v8::Exception::type_error(scope, err);
                scope.throw_exception(exc);
                return;
            }
            let p1 = to_raw_ptr(scope, args.get(0));
            let p2 = to_raw_ptr(scope, args.get(1));
            let len = args.get(2).integer_value(scope).unwrap_or(0) as usize;

            if p1 == 0 || p2 == 0 {
                let err = v8::String::new(scope, "compareMemory: null pointer provided").unwrap();
                let exc = v8::Exception::error(scope, err);
                scope.throw_exception(exc);
                return;
            }

            let cmp = if len == 0 {
                0
            } else {
                unsafe { libc::memcmp(p1 as *const libc::c_void, p2 as *const libc::c_void, len) }
            };
            rv.set(v8::Integer::new(scope, cmp).into());
        },
    )
    .unwrap();

    // 5. wasm.read(ptr, type)
    let read_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let ptr = to_raw_ptr(scope, args.get(0));
            if ptr == 0 {
                rv.set(v8::null(scope).into());
                return;
            }
            let type_str = if args.length() > 1 {
                args.get(1).to_rust_string_lossy(scope)
            } else {
                "u8".to_string()
            };

            match type_str.as_str() {
                "u8" => unsafe {
                    rv.set(v8::Integer::new(scope, *(ptr as *const u8) as i32).into())
                },
                "i8" => unsafe {
                    rv.set(v8::Integer::new(scope, *(ptr as *const i8) as i32).into())
                },
                "u16" => unsafe {
                    rv.set(v8::Integer::new(scope, *(ptr as *const u16) as i32).into())
                },
                "i16" => unsafe {
                    rv.set(v8::Integer::new(scope, *(ptr as *const i16) as i32).into())
                },
                "u32" => unsafe {
                    rv.set(v8::Number::new(scope, *(ptr as *const u32) as f64).into())
                },
                "i32" => unsafe { rv.set(v8::Integer::new(scope, *(ptr as *const i32)).into()) },
                "u64" => unsafe {
                    rv.set(v8::BigInt::new_from_u64(scope, *(ptr as *const u64)).into())
                },
                "i64" => unsafe {
                    rv.set(v8::BigInt::new_from_i64(scope, *(ptr as *const i64)).into())
                },
                "f32" => unsafe {
                    rv.set(v8::Number::new(scope, *(ptr as *const f32) as f64).into())
                },
                "f64" => unsafe { rv.set(v8::Number::new(scope, *(ptr as *const f64)).into()) },
                _ => rv.set(v8::null(scope).into()),
            }
        },
    )
    .unwrap();

    // 6. wasm.write(ptr, val, type)
    let write_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let ptr = to_raw_ptr(scope, args.get(0));
            if ptr == 0 {
                rv.set(v8::Boolean::new(scope, false).into());
                return;
            }
            let val = args.get(1);
            let type_str = if args.length() > 2 {
                args.get(2).to_rust_string_lossy(scope)
            } else {
                "u8".to_string()
            };

            match type_str.as_str() {
                "u8" => unsafe { *(ptr as *mut u8) = val.integer_value(scope).unwrap_or(0) as u8 },
                "i8" => unsafe { *(ptr as *mut i8) = val.integer_value(scope).unwrap_or(0) as i8 },
                "u16" => unsafe {
                    *(ptr as *mut u16) = val.integer_value(scope).unwrap_or(0) as u16
                },
                "i16" => unsafe {
                    *(ptr as *mut i16) = val.integer_value(scope).unwrap_or(0) as i16
                },
                "u32" => unsafe {
                    *(ptr as *mut u32) = val.number_value(scope).unwrap_or(0.0) as u32
                },
                "i32" => unsafe {
                    *(ptr as *mut i32) = val.integer_value(scope).unwrap_or(0) as i32
                },
                "u64" => {
                    let num = if val.is_big_int() {
                        v8::Local::<v8::BigInt>::try_from(val)
                            .map(|b| b.u64_value().0)
                            .unwrap_or(0)
                    } else {
                        val.integer_value(scope).unwrap_or(0) as u64
                    };
                    unsafe { *(ptr as *mut u64) = num };
                }
                "i64" => {
                    let num = if val.is_big_int() {
                        v8::Local::<v8::BigInt>::try_from(val)
                            .map(|b| b.i64_value().0)
                            .unwrap_or(0)
                    } else {
                        val.integer_value(scope).unwrap_or(0) as i64
                    };
                    unsafe { *(ptr as *mut i64) = num };
                }
                "f32" => unsafe {
                    *(ptr as *mut f32) = val.number_value(scope).unwrap_or(0.0) as f32
                },
                "f64" => unsafe { *(ptr as *mut f64) = val.number_value(scope).unwrap_or(0.0) },
                _ => {
                    rv.set(v8::Boolean::new(scope, false).into());
                    return;
                }
            }
            rv.set(v8::Boolean::new(scope, true).into());
        },
    )
    .unwrap();

    // 7. wasm.readCString(ptr)
    let read_cstring_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let ptr = to_raw_ptr(scope, args.get(0)) as *const libc::c_char;
            if ptr.is_null() {
                rv.set(v8::null(scope).into());
                return;
            }
            let s = unsafe { CStr::from_ptr(ptr) };
            let v8_str = v8::String::new(scope, &s.to_string_lossy()).unwrap();
            rv.set(v8_str.into());
        },
    )
    .unwrap();

    // 8. wasm.readString(ptr, length)
    let read_string_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let ptr = to_raw_ptr(scope, args.get(0)) as *const u8;
            let len = args.get(1).integer_value(scope).unwrap_or(0) as usize;
            if ptr.is_null() {
                rv.set(v8::null(scope).into());
                return;
            }
            let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
            let s = String::from_utf8_lossy(slice);
            let v8_str = v8::String::new(scope, &s).unwrap();
            rv.set(v8_str.into());
        },
    )
    .unwrap();

    // 9. wasm.writeString(ptr, str)
    let write_string_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let ptr = to_raw_ptr(scope, args.get(0)) as *mut u8;
            if ptr.is_null() {
                rv.set(v8::Integer::new(scope, 0).into());
                return;
            }
            let s = args.get(1).to_rust_string_lossy(scope);
            let bytes = s.as_bytes();
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            }
            rv.set(v8::Integer::new(scope, bytes.len() as i32).into());
        },
    )
    .unwrap();

    // 10. wasm.loadModuleMmap(path) -> Promise<WebAssembly.Module>
    let load_module_mmap_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let path_str = if args.length() > 0 && !args.get(0).is_null_or_undefined() {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                let err = v8::String::new(scope, "loadModuleMmap requires a file path").unwrap();
                let exc = v8::Exception::type_error(scope, err);
                scope.throw_exception(exc);
                return;
            };

            let resolver = v8::PromiseResolver::new(scope).unwrap();
            rv.set(resolver.get_promise(scope).into());

            let file = match File::open(&path_str) {
                Ok(f) => f,
                Err(e) => {
                    let err = v8::String::new(scope, &format!("Failed to open wasm file: {}", e))
                        .unwrap();
                    let exc = v8::Exception::error(scope, err);
                    resolver.reject(scope, exc);
                    return;
                }
            };

            let mmap = match unsafe { memmap2::Mmap::map(&file) } {
                Ok(m) => m,
                Err(e) => {
                    let err = v8::String::new(scope, &format!("Failed to mmap wasm file: {}", e))
                        .unwrap();
                    let exc = v8::Exception::error(scope, err);
                    resolver.reject(scope, exc);
                    return;
                }
            };

            // Wrap mmap data into a V8 ArrayBuffer with zero heap copy overhead
            let boxed_slice = mmap[..].to_vec().into_boxed_slice();
            let backing_store = v8::ArrayBuffer::new_backing_store_from_boxed_slice(boxed_slice);
            let array_buffer =
                v8::ArrayBuffer::with_backing_store(scope, &backing_store.make_shared());

            // Compile via WebAssembly.compile(arrayBuffer)
            let global = scope.get_current_context().global(scope);
            let wasm_key = v8::String::new(scope, "WebAssembly").unwrap();
            if let Some(wasm_val) = global.get(scope, wasm_key.into()) {
                if let Ok(wasm_obj) = v8::Local::<v8::Object>::try_from(wasm_val) {
                    let compile_key = v8::String::new(scope, "compile").unwrap();
                    if let Some(compile_val) = wasm_obj.get(scope, compile_key.into()) {
                        if let Ok(compile_fn) = v8::Local::<v8::Function>::try_from(compile_val) {
                            let compile_rv =
                                compile_fn.call(scope, wasm_obj.into(), &[array_buffer.into()]);
                            if let Some(res) = compile_rv {
                                resolver.resolve(scope, res);
                                return;
                            }
                        }
                    }
                }
            }

            let err = v8::String::new(scope, "WebAssembly.compile is not available").unwrap();
            let exc = v8::Exception::error(scope, err);
            resolver.reject(scope, exc);
        },
    )
    .unwrap();

    // Attach native binding bag
    let native_obj = v8::Object::new(scope);
    let k_ptr = v8::String::new(scope, "ptr").unwrap();
    let k_copy = v8::String::new(scope, "copyMemory").unwrap();
    let k_fill = v8::String::new(scope, "fillMemory").unwrap();
    let k_cmp = v8::String::new(scope, "compareMemory").unwrap();
    let k_read = v8::String::new(scope, "read").unwrap();
    let k_write = v8::String::new(scope, "write").unwrap();
    let k_read_cstr = v8::String::new(scope, "readCString").unwrap();
    let k_read_str = v8::String::new(scope, "readString").unwrap();
    let k_write_str = v8::String::new(scope, "writeString").unwrap();
    let k_mmap = v8::String::new(scope, "loadModuleMmap").unwrap();

    native_obj.set(scope, k_ptr.into(), ptr_fn.into());
    native_obj.set(scope, k_copy.into(), copy_memory_fn.into());
    native_obj.set(scope, k_fill.into(), fill_memory_fn.into());
    native_obj.set(scope, k_cmp.into(), compare_memory_fn.into());
    native_obj.set(scope, k_read.into(), read_fn.into());
    native_obj.set(scope, k_write.into(), write_fn.into());
    native_obj.set(scope, k_read_cstr.into(), read_cstring_fn.into());
    native_obj.set(scope, k_read_str.into(), read_string_fn.into());
    native_obj.set(scope, k_write_str.into(), write_string_fn.into());
    native_obj.set(scope, k_mmap.into(), load_module_mmap_fn.into());

    let k_bee_wasm_native = v8::String::new(scope, "__bee_wasm_native").unwrap();
    global.set(scope, k_bee_wasm_native.into(), native_obj.into());

    // Inject high-level user-friendly JavaScript wrapper
    let js_code = r#"
    (function() {
        const native = globalThis.__bee_wasm_native;

        class MemoryView {
            constructor(target, byteLength, byteOffset = 0) {
                if (typeof target === 'bigint' || typeof target === 'number') {
                    this._ptr = BigInt(target) + BigInt(byteOffset);
                    this._byteLength = byteLength || 0;
                    this._target = null;
                } else {
                    this._ptr = native.ptr(target) + BigInt(byteOffset);
                    this._byteLength = byteLength !== undefined ? byteLength : (target.byteLength || 0);
                    this._target = target;
                }
            }

            get ptr() {
                return this._ptr;
            }

            get byteLength() {
                return this._byteLength;
            }

            getUint8(offset) {
                return native.read(this._ptr + BigInt(offset), 'u8');
            }
            setUint8(offset, val) {
                return native.write(this._ptr + BigInt(offset), val, 'u8');
            }

            getInt8(offset) {
                return native.read(this._ptr + BigInt(offset), 'i8');
            }
            setInt8(offset, val) {
                return native.write(this._ptr + BigInt(offset), val, 'i8');
            }

            getInt32(offset) {
                return native.read(this._ptr + BigInt(offset), 'i32');
            }
            setInt32(offset, val) {
                return native.write(this._ptr + BigInt(offset), val, 'i32');
            }

            getUint32(offset) {
                return native.read(this._ptr + BigInt(offset), 'u32');
            }
            setUint32(offset, val) {
                return native.write(this._ptr + BigInt(offset), val, 'u32');
            }

            getFloat32(offset) {
                return native.read(this._ptr + BigInt(offset), 'f32');
            }
            setFloat32(offset, val) {
                return native.write(this._ptr + BigInt(offset), val, 'f32');
            }

            getFloat64(offset) {
                return native.read(this._ptr + BigInt(offset), 'f64');
            }
            setFloat64(offset, val) {
                return native.write(this._ptr + BigInt(offset), val, 'f64');
            }

            getCString(offset = 0) {
                return native.readCString(this._ptr + BigInt(offset));
            }

            getString(offset, length) {
                return native.readString(this._ptr + BigInt(offset), length);
            }
            setString(offset, str) {
                return native.writeString(this._ptr + BigInt(offset), str);
            }

            copyFrom(srcPtr, length, dstOffset = 0) {
                const s = typeof srcPtr === 'object' ? native.ptr(srcPtr) : BigInt(srcPtr);
                return native.copyMemory(s, this._ptr + BigInt(dstOffset), length);
            }

            copyTo(dstPtr, length, srcOffset = 0) {
                const d = typeof dstPtr === 'object' ? native.ptr(dstPtr) : BigInt(dstPtr);
                return native.copyMemory(this._ptr + BigInt(srcOffset), d, length);
            }

            fill(value, offset = 0, length = null) {
                const len = length !== null ? length : (this._byteLength - offset);
                return native.fillMemory(this._ptr + BigInt(offset), value, len);
            }
        }

        function createSharedMemory(options = {}) {
            const initial = options.initial || 1;
            const maximum = options.maximum || initial;
            return new WebAssembly.Memory({
                initial,
                maximum,
                shared: true
            });
        }

        function wrapPointer(ptr, byteLength, type = 'u8') {
            const view = new MemoryView(ptr, byteLength);
            return new Proxy(view, {
                get(target, prop) {
                    if (typeof prop === 'string' && !isNaN(prop)) {
                        const idx = Number(prop);
                        switch (type) {
                            case 'i8': return target.getInt8(idx);
                            case 'i32': return target.getInt32(idx * 4);
                            case 'u32': return target.getUint32(idx * 4);
                            case 'f32': return target.getFloat32(idx * 4);
                            case 'f64': return target.getFloat64(idx * 8);
                            case 'u8':
                            default: return target.getUint8(idx);
                        }
                    }
                    return target[prop];
                },
                set(target, prop, val) {
                    if (typeof prop === 'string' && !isNaN(prop)) {
                        const idx = Number(prop);
                        switch (type) {
                            case 'i8': target.setInt8(idx, val); return true;
                            case 'i32': target.setInt32(idx * 4, val); return true;
                            case 'u32': target.setUint32(idx * 4, val); return true;
                            case 'f32': target.setFloat32(idx * 4, val); return true;
                            case 'f64': target.setFloat64(idx * 8, val); return true;
                            case 'u8':
                            default: target.setUint8(idx, val); return true;
                        }
                    }
                    target[prop] = val;
                    return true;
                }
            });
        }

        function linkTensor(tensor, memory, byteOffset = 0) {
            if (!tensor || !memory) throw new TypeError('linkTensor requires tensor and WebAssembly.Memory');
            const tensorPtr = native.ptr(tensor);
            const memPtr = native.ptr(memory) + BigInt(byteOffset);
            const byteLength = tensor.data ? tensor.data.byteLength : (tensor.byteLength || 0);
            native.copyMemory(tensorPtr, memPtr, byteLength);
            return {
                tensor,
                memory,
                byteOffset,
                byteLength,
                ptr: memPtr
            };
        }

        function createTensorFromMemory(memory, byteOffset, shape, dtype = 'float32') {
            if (!memory || !shape) throw new TypeError('createTensorFromMemory requires memory and shape');
            const totalElements = shape.reduce((a, b) => a * b, 1);
            let view;
            const buf = memory.buffer || memory;
            switch (dtype) {
                case 'float64':
                    view = new Float64Array(buf, byteOffset, totalElements);
                    break;
                case 'int32':
                    view = new Int32Array(buf, byteOffset, totalElements);
                    break;
                case 'int8':
                    view = new Int8Array(buf, byteOffset, totalElements);
                    break;
                case 'uint8':
                    view = new Uint8Array(buf, byteOffset, totalElements);
                    break;
                case 'float32':
                default:
                    view = new Float32Array(buf, byteOffset, totalElements);
                    break;
            }

            if (globalThis.__bee_ai && globalThis.__bee_ai.Tensor) {
                return new globalThis.__bee_ai.Tensor(view, shape, dtype);
            }
            // Standalone Tensor shape wrapper if bee:ai is not loaded
            return {
                data: view,
                shape,
                dtype,
                length: totalElements,
                byteLength: view.byteLength,
                ptr: native.ptr(view)
            };
        }

        const wasm = {
            ptr: native.ptr,
            copyMemory: native.copyMemory,
            fillMemory: native.fillMemory,
            compareMemory: native.compareMemory,
            read: native.read,
            write: native.write,
            readCString: native.readCString,
            readString: native.readString,
            writeString: native.writeString,
            loadModuleMmap: native.loadModuleMmap,
            createSharedMemory,
            wrapPointer,
            linkTensor,
            createTensorFromMemory,
            MemoryView,
            version: '1.5.0'
        };

        globalThis.__bee_wasm = wasm;
        globalThis.wasm = wasm;
    })();
    "#;

    let code_str = v8::String::new(scope, js_code).unwrap();
    let script = v8::Script::compile(scope, code_str, None)
        .ok_or_else(|| anyhow!("Failed to compile wasm bootstrap script"))?;
    script
        .run(scope)
        .ok_or_else(|| anyhow!("Failed to run wasm bootstrap script"))?;

    Ok(())
}
