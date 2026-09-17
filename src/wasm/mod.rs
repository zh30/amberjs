// Amber WebAssembly 2.0 Zero-Copy Shared Memory Subsystem (amber:wasm)
// High-performance physical memory bridge between V8, WebAssembly, FFI pointers, and amber:ai.Tensor.

use anyhow::{anyhow, Result};
use rusty_v8 as v8;
use std::ffi::CStr;
use std::fs::File;

#[allow(unused_unsafe)]
unsafe extern "C" fn noop_backing_store_deleter(
    _data: *mut std::ffi::c_void,
    _byte_length: usize,
    _deleter_data: *mut std::ffi::c_void,
) {
    unsafe {
        let _ = _data;
    }
}

unsafe extern "C" fn mmap_backing_store_deleter(
    _data: *mut std::ffi::c_void,
    _byte_length: usize,
    deleter_data: *mut std::ffi::c_void,
) {
    if !deleter_data.is_null() {
        unsafe {
            drop(Box::from_raw(deleter_data as *mut memmap2::Mmap));
        }
    }
}

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
    } else if arg.is_shared_array_buffer() {
        if let Ok(sab) = v8::Local::<v8::SharedArrayBuffer>::try_from(arg) {
            let store = sab.get_backing_store();
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
                } else if buf_val.is_shared_array_buffer() {
                    if let Ok(sab) = v8::Local::<v8::SharedArrayBuffer>::try_from(buf_val) {
                        let store = sab.get_backing_store();
                        if let Some(data) = store.data() {
                            return data.as_ptr() as usize;
                        }
                    }
                }
            }
            // Check if it's amber:ai.Tensor (has .data property)
            let data_key = v8::String::new(scope, "data").unwrap();
            if let Some(tensor_data) = obj.get(scope, data_key.into()) {
                if tensor_data.is_array_buffer_view() || tensor_data.is_object() {
                    let ptr = extract_target_ptr(scope, tensor_data);
                    if ptr != 0 {
                        return ptr;
                    }
                }
            }
            // Check if it has a .ptr property
            let ptr_key = v8::String::new(scope, "ptr").unwrap();
            if let Some(p_val) = obj.get(scope, ptr_key.into()) {
                let ptr = to_raw_ptr(scope, p_val);
                if ptr != 0 {
                    return ptr;
                }
            }
        }
    } else if arg.is_big_int() || arg.is_number() {
        return to_raw_ptr(scope, arg);
    }
    0
}

/// Initialize the `amber:wasm` subsystem in V8 context
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

    // 10. wasm.loadModuleMmap(path) -> Promise<WebAssembly.Module> (Truly Zero-Copy)
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

            // Truly zero-copy ArrayBuffer backed directly by OS mmap page
            let mmap_box = Box::new(mmap);
            let ptr = mmap_box.as_ptr() as *mut std::ffi::c_void;
            let len = mmap_box.len();
            let deleter_data = Box::into_raw(mmap_box) as *mut std::ffi::c_void;

            let backing_store = unsafe {
                v8::ArrayBuffer::new_backing_store_from_ptr(
                    ptr,
                    len,
                    mmap_backing_store_deleter,
                    deleter_data,
                )
            };
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

    // 11. wasm.createBufferFromPointer(ptr, byteLength, shared) -> ArrayBuffer | SharedArrayBuffer
    let create_buffer_from_pointer_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 2 {
                let err = v8::String::new(
                    scope,
                    "createBufferFromPointer requires 2 arguments (ptr, byteLength)",
                )
                .unwrap();
                let exc = v8::Exception::type_error(scope, err);
                scope.throw_exception(exc);
                return;
            }
            let ptr = to_raw_ptr(scope, args.get(0));
            let len = args.get(1).integer_value(scope).unwrap_or(0) as usize;
            if ptr == 0 {
                let err = v8::String::new(scope, "createBufferFromPointer: null pointer provided")
                    .unwrap();
                let exc = v8::Exception::error(scope, err);
                scope.throw_exception(exc);
                return;
            }
            let shared = if args.length() > 2 {
                args.get(2).boolean_value(scope)
            } else {
                false
            };

            if shared {
                let backing_store = unsafe {
                    v8::SharedArrayBuffer::new_backing_store_from_ptr(
                        ptr as *mut std::ffi::c_void,
                        len,
                        noop_backing_store_deleter,
                        std::ptr::null_mut(),
                    )
                };
                let sab =
                    v8::SharedArrayBuffer::with_backing_store(scope, &backing_store.make_shared());
                rv.set(sab.into());
            } else {
                let backing_store = unsafe {
                    v8::ArrayBuffer::new_backing_store_from_ptr(
                        ptr as *mut std::ffi::c_void,
                        len,
                        noop_backing_store_deleter,
                        std::ptr::null_mut(),
                    )
                };
                let ab = v8::ArrayBuffer::with_backing_store(scope, &backing_store.make_shared());
                rv.set(ab.into());
            }
        },
    )
    .unwrap();

    // 12. wasm.sliceZeroCopy(target, byteOffset, byteLength) -> ArrayBuffer | SharedArrayBuffer
    let slice_zero_copy_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 1 {
                let err =
                    v8::String::new(scope, "sliceZeroCopy requires at least 1 argument (target)")
                        .unwrap();
                let exc = v8::Exception::type_error(scope, err);
                scope.throw_exception(exc);
                return;
            }
            let base_ptr = extract_target_ptr(scope, args.get(0));
            if base_ptr == 0 {
                let err = v8::String::new(scope, "sliceZeroCopy: target has invalid null pointer")
                    .unwrap();
                let exc = v8::Exception::error(scope, err);
                scope.throw_exception(exc);
                return;
            }
            let offset = if args.length() > 1 {
                args.get(1).integer_value(scope).unwrap_or(0) as usize
            } else {
                0
            };
            let len = if args.length() > 2 && !args.get(2).is_null_or_undefined() {
                args.get(2).integer_value(scope).unwrap_or(0) as usize
            } else {
                let arg0 = args.get(0);
                if arg0.is_array_buffer_view() {
                    if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(arg0) {
                        view.byte_length().saturating_sub(offset)
                    } else {
                        0
                    }
                } else if arg0.is_array_buffer() {
                    if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(arg0) {
                        ab.byte_length().saturating_sub(offset)
                    } else {
                        0
                    }
                } else if arg0.is_shared_array_buffer() {
                    if let Ok(sab) = v8::Local::<v8::SharedArrayBuffer>::try_from(arg0) {
                        sab.byte_length().saturating_sub(offset)
                    } else {
                        0
                    }
                } else if arg0.is_object() {
                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(arg0) {
                        let buffer_key = v8::String::new(scope, "buffer").unwrap();
                        if let Some(buf_val) = obj.get(scope, buffer_key.into()) {
                            if buf_val.is_array_buffer() {
                                if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf_val) {
                                    ab.byte_length().saturating_sub(offset)
                                } else {
                                    0
                                }
                            } else if buf_val.is_shared_array_buffer() {
                                if let Ok(sab) =
                                    v8::Local::<v8::SharedArrayBuffer>::try_from(buf_val)
                                {
                                    sab.byte_length().saturating_sub(offset)
                                } else {
                                    0
                                }
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else {
                    0
                }
            };

            let is_shared = args.get(0).is_shared_array_buffer()
                || (args.get(0).is_object() && {
                    if let Ok(obj) = v8::Local::<v8::Object>::try_from(args.get(0)) {
                        let buffer_key = v8::String::new(scope, "buffer").unwrap();
                        obj.get(scope, buffer_key.into())
                            .map(|b| b.is_shared_array_buffer())
                            .unwrap_or(false)
                    } else {
                        false
                    }
                });

            let final_ptr = base_ptr + offset;
            if is_shared {
                let backing_store = unsafe {
                    v8::SharedArrayBuffer::new_backing_store_from_ptr(
                        final_ptr as *mut std::ffi::c_void,
                        len,
                        noop_backing_store_deleter,
                        std::ptr::null_mut(),
                    )
                };
                let sab =
                    v8::SharedArrayBuffer::with_backing_store(scope, &backing_store.make_shared());
                rv.set(sab.into());
            } else {
                let backing_store = unsafe {
                    v8::ArrayBuffer::new_backing_store_from_ptr(
                        final_ptr as *mut std::ffi::c_void,
                        len,
                        noop_backing_store_deleter,
                        std::ptr::null_mut(),
                    )
                };
                let ab = v8::ArrayBuffer::with_backing_store(scope, &backing_store.make_shared());
                rv.set(ab.into());
            }
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
    let k_create_buf = v8::String::new(scope, "createBufferFromPointer").unwrap();
    let k_slice = v8::String::new(scope, "sliceZeroCopy").unwrap();

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
    native_obj.set(
        scope,
        k_create_buf.into(),
        create_buffer_from_pointer_fn.into(),
    );
    native_obj.set(scope, k_slice.into(), slice_zero_copy_fn.into());

    let k_amber_wasm_native = v8::String::new(scope, "__amber_wasm_native").unwrap();
    global.set(scope, k_amber_wasm_native.into(), native_obj.into());

    // Inject high-level user-friendly JavaScript wrapper
    let js_code = r#"
    (function() {
        const native = globalThis.__amber_wasm_native;

        // 1. Prototype extensions for WebAssembly.Memory, ArrayBuffer, TypedArrays
        if (typeof WebAssembly !== 'undefined' && WebAssembly.Memory) {
            Object.defineProperty(WebAssembly.Memory.prototype, 'ptr', {
                get() {
                    return native.ptr(this);
                },
                configurable: true,
                enumerable: false
            });

            WebAssembly.Memory.prototype.getPointer = function() {
                return native.ptr(this);
            };

            WebAssembly.Memory.prototype.createZeroCopyBuffer = function(byteOffset = 0, byteLength) {
                return native.sliceZeroCopy(this, byteOffset, byteLength);
            };

            WebAssembly.Memory.prototype.asUint8Array = function(byteOffset = 0, length) {
                const buf = this.createZeroCopyBuffer(byteOffset, length);
                return new Uint8Array(buf);
            };

            WebAssembly.Memory.prototype.asFloat32Array = function(byteOffset = 0, length) {
                const len = length !== undefined ? length * 4 : undefined;
                const buf = this.createZeroCopyBuffer(byteOffset, len);
                return new Float32Array(buf);
            };

            WebAssembly.Memory.prototype.asFloat64Array = function(byteOffset = 0, length) {
                const len = length !== undefined ? length * 8 : undefined;
                const buf = this.createZeroCopyBuffer(byteOffset, len);
                return new Float64Array(buf);
            };

            WebAssembly.Memory.prototype.asInt32Array = function(byteOffset = 0, length) {
                const len = length !== undefined ? length * 4 : undefined;
                const buf = this.createZeroCopyBuffer(byteOffset, len);
                return new Int32Array(buf);
            };

            WebAssembly.Memory.prototype.asInt8Array = function(byteOffset = 0, length) {
                const buf = this.createZeroCopyBuffer(byteOffset, length);
                return new Int8Array(buf);
            };

            WebAssembly.Memory.prototype.asUint32Array = function(byteOffset = 0, length) {
                const len = length !== undefined ? length * 4 : undefined;
                const buf = this.createZeroCopyBuffer(byteOffset, len);
                return new Uint32Array(buf);
            };
        }

        if (typeof ArrayBuffer !== 'undefined') {
            Object.defineProperty(ArrayBuffer.prototype, 'ptr', {
                get() {
                    return native.ptr(this);
                },
                configurable: true,
                enumerable: false
            });

            ArrayBuffer.prototype.getPointer = function() {
                return native.ptr(this);
            };

            ArrayBuffer.prototype.createZeroCopyView = function(byteOffset = 0, byteLength) {
                return native.sliceZeroCopy(this, byteOffset, byteLength);
            };
        }

        if (typeof SharedArrayBuffer !== 'undefined') {
            Object.defineProperty(SharedArrayBuffer.prototype, 'ptr', {
                get() {
                    return native.ptr(this);
                },
                configurable: true,
                enumerable: false
            });

            SharedArrayBuffer.prototype.getPointer = function() {
                return native.ptr(this);
            };

            SharedArrayBuffer.prototype.createZeroCopyView = function(byteOffset = 0, byteLength) {
                return native.sliceZeroCopy(this, byteOffset, byteLength);
            };
        }

        const TypedArrayProto = Object.getPrototypeOf(Uint8Array.prototype);
        if (TypedArrayProto) {
            Object.defineProperty(TypedArrayProto, 'ptr', {
                get() {
                    return native.ptr(this);
                },
                configurable: true,
                enumerable: false
            });

            TypedArrayProto.getPointer = function() {
                return native.ptr(this);
            };
        }

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

            asArrayBuffer(shared = false) {
                return native.createBufferFromPointer(this._ptr, this._byteLength, shared);
            }

            asUint8Array() {
                return new Uint8Array(this.asArrayBuffer());
            }

            asFloat32Array() {
                return new Float32Array(this.asArrayBuffer());
            }

            asFloat64Array() {
                return new Float64Array(this.asArrayBuffer());
            }

            asInt32Array() {
                return new Int32Array(this.asArrayBuffer());
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

        function shareMemory(memory) {
            if (!memory || !(memory instanceof WebAssembly.Memory)) {
                throw new TypeError('shareMemory requires a WebAssembly.Memory instance');
            }
            const ptr = native.ptr(memory);
            return {
                ptr,
                get byteLength() { return memory.buffer.byteLength; },
                get buffer() { return memory.buffer; },
                get uint8() { return new Uint8Array(memory.buffer); },
                get int8() { return new Int8Array(memory.buffer); },
                get uint32() { return new Uint32Array(memory.buffer); },
                get int32() { return new Int32Array(memory.buffer); },
                get float32() { return new Float32Array(memory.buffer); },
                get float64() { return new Float64Array(memory.buffer); },
                createBuffer(offset = 0, length) {
                    return native.sliceZeroCopy(memory, offset, length);
                },
                view(type = 'u8', offset = 0, length) {
                    return createZeroCopyTypedArray(memory, type, offset, length);
                }
            };
        }

        function createZeroCopyBuffer(target, byteOffset = 0, byteLength) {
            return native.sliceZeroCopy(target, byteOffset, byteLength);
        }

        function createZeroCopyTypedArray(target, type = 'u8', byteOffset = 0, length) {
            let byteLen = undefined;
            if (length !== undefined && length !== null) {
                switch (type) {
                    case 'u16':
                    case 'i16': byteLen = length * 2; break;
                    case 'u32':
                    case 'i32':
                    case 'f32': byteLen = length * 4; break;
                    case 'f64':
                    case 'u64':
                    case 'i64': byteLen = length * 8; break;
                    case 'u8':
                    case 'i8':
                    default: byteLen = length; break;
                }
            }
            const buf = native.sliceZeroCopy(target, byteOffset, byteLen);
            switch (type) {
                case 'i8': return new Int8Array(buf);
                case 'u16': return new Uint16Array(buf);
                case 'i16': return new Int16Array(buf);
                case 'u32': return new Uint32Array(buf);
                case 'i32': return new Int32Array(buf);
                case 'f32': return new Float32Array(buf);
                case 'f64': return new Float64Array(buf);
                case 'u64': return new BigUint64Array(buf);
                case 'i64': return new BigInt64Array(buf);
                case 'u8':
                default: return new Uint8Array(buf);
            }
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

            if (globalThis.__amber_ai && globalThis.__amber_ai.Tensor) {
                return new globalThis.__amber_ai.Tensor(view, shape, dtype);
            }
            // Standalone Tensor shape wrapper if amber:ai is not loaded
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
            createBufferFromPointer: native.createBufferFromPointer,
            sliceZeroCopy: native.sliceZeroCopy,
            createZeroCopyBuffer,
            createZeroCopyTypedArray,
            createSharedMemory,
            shareMemory,
            wrapPointer,
            linkTensor,
            createTensorFromMemory,
            MemoryView,
            version: '2.0.0'
        };

        globalThis.__amber_wasm = wasm;
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
