//! Node.js `zlib` builtin (gzip/deflate via flate2).
//!
//! G21: six sync methods. G48: async `gzip` / `gunzip` callbacks (nextTick delivery).

use anyhow::Result;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
use flate2::Compression;
use rusty_v8 as v8;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{Read, Write};

thread_local! {
    static ZLIB_JOBS: RefCell<HashMap<u32, ZlibAsyncJob>> = RefCell::new(HashMap::new());
    static ZLIB_JOB_SEQ: Cell<u32> = const { Cell::new(1) };
}

enum ZlibAsyncOp {
    Gzip,
    Gunzip,
}

struct ZlibAsyncJob {
    op: ZlibAsyncOp,
    input: Vec<u8>,
    callback: v8::Global<v8::Function>,
}

pub fn setup_zlib_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);
    let zlib = v8::Object::new(scope);

    let gzip_sync = v8::Function::new(scope, gzip_sync_cb).unwrap();
    let gunzip_sync = v8::Function::new(scope, gunzip_sync_cb).unwrap();
    let deflate_sync = v8::Function::new(scope, deflate_sync_cb).unwrap();
    let inflate_sync = v8::Function::new(scope, inflate_sync_cb).unwrap();
    let deflate_raw_sync = v8::Function::new(scope, deflate_raw_sync_cb).unwrap();
    let inflate_raw_sync = v8::Function::new(scope, inflate_raw_sync_cb).unwrap();
    let gzip = v8::Function::new(scope, gzip_async_cb).unwrap();
    let gunzip = v8::Function::new(scope, gunzip_async_cb).unwrap();

    for (name, func) in [
        ("gzipSync", gzip_sync),
        ("gunzipSync", gunzip_sync),
        ("deflateSync", deflate_sync),
        ("inflateSync", inflate_sync),
        ("deflateRawSync", deflate_raw_sync),
        ("inflateRawSync", inflate_raw_sync),
        ("gzip", gzip),
        ("gunzip", gunzip),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        zlib.set(scope, key.into(), func.into());
    }

    let key = v8::String::new(scope, "zlib").unwrap();
    global.set(scope, key.into(), zlib.into());
    Ok(())
}

fn throw_type(scope: &mut v8::PinScope, message: &str) {
    let msg = v8::String::new(scope, message).unwrap();
    let exc = v8::Exception::type_error(scope, msg);
    scope.throw_exception(exc);
}

fn throw_err(scope: &mut v8::PinScope, message: &str) {
    let msg = v8::String::new(scope, message).unwrap();
    let exc = v8::Exception::error(scope, msg);
    scope.throw_exception(exc);
}

fn bytes_from_arg(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<Vec<u8>> {
    if value.is_array_buffer() {
        let buf = v8::Local::<v8::ArrayBuffer>::try_from(value).ok()?;
        let len = buf.byte_length();
        if len == 0 {
            return Some(Vec::new());
        }
        let store = buf.get_backing_store();
        let ptr = store.as_ref().as_ptr();
        if ptr.is_null() {
            return Some(Vec::new());
        }
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        return Some(slice.to_vec());
    }
    if value.is_typed_array() {
        let ta = v8::Local::<v8::TypedArray>::try_from(value).ok()?;
        let len = ta.byte_length() as usize;
        if len == 0 {
            return Some(Vec::new());
        }
        let buf = ta.buffer(scope)?;
        let store = buf.get_backing_store();
        let ptr = store.as_ref().as_ptr();
        if ptr.is_null() {
            return Some(Vec::new());
        }
        let offset = ta.byte_offset();
        let slice = unsafe { std::slice::from_raw_parts((ptr as *const u8).add(offset), len) };
        return Some(slice.to_vec());
    }
    if value.is_object() {
        if let Ok(obj) = v8::Local::<v8::Object>::try_from(value) {
            let buffer_key = v8::String::new(scope, "buffer").unwrap();
            if let Some(inner) = obj.get(scope, buffer_key.into()) {
                if inner.is_array_buffer() {
                    return bytes_from_arg(scope, inner);
                }
            }
        }
    }
    if value.is_string() {
        let s = value.to_string(scope)?;
        return Some(s.to_rust_string_lossy(scope).into_bytes());
    }
    None
}

fn make_buffer_value<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    bytes: &[u8],
) -> v8::Local<'a, v8::Value> {
    let buffer = v8::ArrayBuffer::new(scope, bytes.len());
    if !bytes.is_empty() {
        let store = buffer.get_backing_store();
        let slice = unsafe {
            std::slice::from_raw_parts_mut(store.as_ref().as_ptr() as *mut u8, bytes.len())
        };
        slice.copy_from_slice(bytes);
    }
    let global = scope.get_current_context().global(scope);
    let buffer_key = v8::String::new(scope, "Buffer").unwrap();
    if let Some(buf_ctor_val) = global.get(scope, buffer_key.into()) {
        if let Ok(buf_ctor) = v8::Local::<v8::Object>::try_from(buf_ctor_val) {
            let from_key = v8::String::new(scope, "from").unwrap();
            if let Some(from_val) = buf_ctor.get(scope, from_key.into()) {
                if let Ok(from_fn) = v8::Local::<v8::Function>::try_from(from_val) {
                    if let Some(buf_obj) = from_fn.call(scope, buf_ctor_val, &[buffer.into()]) {
                        return buf_obj;
                    }
                }
            }
        }
    }
    buffer.into()
}

fn return_buffer(scope: &mut v8::PinScope, bytes: &[u8], rv: &mut v8::ReturnValue) {
    rv.set(make_buffer_value(scope, bytes));
}

fn gzip_sync_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(input) = bytes_from_arg(scope, args.get(0)) else {
        throw_type(scope, "zlib.gzipSync: invalid input");
        return;
    };
    match gzip_bytes(&input) {
        Ok(out) => return_buffer(scope, &out, &mut rv),
        Err(_) => throw_err(scope, "zlib.gzipSync failed"),
    }
}

fn gunzip_sync_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(input) = bytes_from_arg(scope, args.get(0)) else {
        throw_type(scope, "zlib.gunzipSync: invalid input");
        return;
    };
    match gunzip_bytes(&input) {
        Ok(out) => return_buffer(scope, &out, &mut rv),
        Err(_) => throw_err(scope, "zlib.gunzipSync failed"),
    }
}

fn deflate_sync_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(input) = bytes_from_arg(scope, args.get(0)) else {
        throw_type(scope, "zlib.deflateSync: invalid input");
        return;
    };
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    if encoder.write_all(&input).is_err() {
        throw_err(scope, "zlib.deflateSync failed");
        return;
    }
    match encoder.finish() {
        Ok(out) => return_buffer(scope, &out, &mut rv),
        Err(_) => throw_err(scope, "zlib.deflateSync failed"),
    }
}

fn inflate_sync_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(input) = bytes_from_arg(scope, args.get(0)) else {
        throw_type(scope, "zlib.inflateSync: invalid input");
        return;
    };
    let mut decoder = ZlibDecoder::new(&input[..]);
    let mut out = Vec::new();
    if decoder.read_to_end(&mut out).is_err() {
        throw_err(scope, "zlib.inflateSync failed");
        return;
    }
    return_buffer(scope, &out, &mut rv);
}

fn deflate_raw_sync_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(input) = bytes_from_arg(scope, args.get(0)) else {
        throw_type(scope, "zlib.deflateRawSync: invalid input");
        return;
    };
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    if encoder.write_all(&input).is_err() {
        throw_err(scope, "zlib.deflateRawSync failed");
        return;
    }
    match encoder.finish() {
        Ok(out) => return_buffer(scope, &out, &mut rv),
        Err(_) => throw_err(scope, "zlib.deflateRawSync failed"),
    }
}

fn inflate_raw_sync_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(input) = bytes_from_arg(scope, args.get(0)) else {
        throw_type(scope, "zlib.inflateRawSync: invalid input");
        return;
    };
    let mut decoder = DeflateDecoder::new(&input[..]);
    let mut out = Vec::new();
    if decoder.read_to_end(&mut out).is_err() {
        throw_err(scope, "zlib.inflateRawSync failed");
        return;
    }
    return_buffer(scope, &out, &mut rv);
}

fn gzip_bytes(input: &[u8]) -> Result<Vec<u8>, ()> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(input).map_err(|_| ())?;
    encoder.finish().map_err(|_| ())
}

fn gunzip_bytes(input: &[u8]) -> Result<Vec<u8>, ()> {
    let mut decoder = GzDecoder::new(input);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(|_| ())?;
    Ok(out)
}

fn gzip_async_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    rv: v8::ReturnValue,
) {
    enqueue_async_codec(scope, args, rv, ZlibAsyncOp::Gzip, "zlib.gzip");
}

fn gunzip_async_cb(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    rv: v8::ReturnValue,
) {
    enqueue_async_codec(scope, args, rv, ZlibAsyncOp::Gunzip, "zlib.gunzip");
}

fn enqueue_async_codec(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
    op: ZlibAsyncOp,
    name: &str,
) {
    // Contract shape: method(input, callback). No options bag.
    if args.length() != 2 {
        throw_type(
            scope,
            &format!("{name}: expected (input, callback); options are not supported"),
        );
        return;
    }
    let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(1)) else {
        throw_type(scope, &format!("{name}: callback must be a function"));
        return;
    };
    let Some(input) = bytes_from_arg(scope, args.get(0)) else {
        throw_type(scope, &format!("{name}: invalid input"));
        return;
    };

    let id = ZLIB_JOB_SEQ.with(|seq| {
        let id = seq.get();
        seq.set(id.wrapping_add(1));
        id
    });
    ZLIB_JOBS.with(|jobs| {
        jobs.borrow_mut().insert(
            id,
            ZlibAsyncJob {
                op,
                input,
                callback: v8::Global::new(scope, callback),
            },
        );
    });

    let trampoline = v8::Function::new(scope, zlib_job_trampoline).unwrap();
    let id_val = v8::Number::new(scope, f64::from(id));
    let trampoline_val: v8::Local<v8::Value> = trampoline.into();
    let id_value: v8::Local<v8::Value> = id_val.into();
    crate::nodejs_core::process::push_next_tick_callback(
        v8::Global::new(scope, trampoline_val),
        vec![v8::Global::new(scope, id_value)],
    );
    rv.set(v8::undefined(scope).into());
}

fn zlib_job_trampoline(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let id = args.get(0).number_value(scope).unwrap_or(0.0) as u32;
    let job = ZLIB_JOBS.with(|jobs| jobs.borrow_mut().remove(&id));
    let Some(job) = job else {
        return;
    };

    let result = match job.op {
        ZlibAsyncOp::Gzip => gzip_bytes(&job.input).map_err(|_| "zlib.gzip failed"),
        ZlibAsyncOp::Gunzip => gunzip_bytes(&job.input).map_err(|_| "zlib.gunzip failed"),
    };

    let callback = v8::Local::new(scope, job.callback);
    let undefined = v8::undefined(scope);
    match result {
        Ok(bytes) => {
            let null: v8::Local<v8::Value> = v8::null(scope).into();
            let buf = make_buffer_value(scope, &bytes);
            let _ = callback.call(scope, undefined.into(), &[null, buf]);
        }
        Err(message) => {
            let msg = v8::String::new(scope, message).unwrap();
            let err = v8::Exception::error(scope, msg);
            let _ = callback.call(scope, undefined.into(), &[err]);
        }
    }
}
