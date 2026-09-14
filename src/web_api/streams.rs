// Web Streams API implementation for Web standard
// Stage 75: Web Streams API for AI workloads
// Provides ReadableStream, WritableStream, TransformStream, TextDecoderStream
//
// Optimized for streaming LLM responses and AI data processing pipelines
//
// v0.3.286: Enhanced TextDecoderStream with actual UTF-8 decoding

use anyhow::Result;
use rusty_v8 as v8;

fn get_bool_property(scope: &mut v8::PinScope, object: v8::Local<v8::Object>, name: &str) -> bool {
    let key = v8::String::new(scope, name).unwrap();
    object
        .get(scope, key.into())
        .is_some_and(|value| value.is_true())
}

fn set_bool_property(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    name: &str,
    value: bool,
) {
    let key = v8::String::new(scope, name).unwrap();
    let bool_value: v8::Local<v8::Value> = v8::Boolean::new(scope, value).into();
    object.set(scope, key.into(), bool_value);
}

fn throw_type_error(scope: &mut v8::PinScope, message: &str) {
    let message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
}

// ============================================================
// ReadableStream Implementation
// ============================================================

/// ReadableStream constructor - enhanced with start() and enqueue support
/// Uses JavaScript arrays on the stream object for queue storage
fn readable_stream_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: v8::Local<v8::Object> = args.this();

    // Create JavaScript array to store enqueued chunks
    let queue_array = v8::Array::new(scope, 0);
    let queue_key = v8::String::new(scope, "_queue").unwrap();
    this.set(scope, queue_key.into(), queue_array.into());

    // Create state variable (0=open, 1=closed, 2=errored)
    let state_key = v8::String::new(scope, "_state").unwrap();
    let state_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into(); // 0 = Open
    this.set(scope, state_key.into(), state_val);

    // Create index for reading from queue
    let read_index_key = v8::String::new(scope, "_readIndex").unwrap();
    let read_index_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
    this.set(scope, read_index_key.into(), read_index_val);

    // Check if first argument is an object with start method
    if args.length() > 0 {
        let underlying_source = args.get(0);
        if underlying_source.is_object() {
            let source_obj = v8::Local::<v8::Object>::try_from(underlying_source).unwrap();
            let start_key = v8::String::new(scope, "start").unwrap();
            if let Some(start_fn_val) = source_obj.get(scope, start_key.into()) {
                if start_fn_val.is_function() {
                    let start_fn = v8::Local::<v8::Function>::try_from(start_fn_val).unwrap();

                    // Create controller object with enqueue, close, error methods
                    let controller = v8::Object::new(scope);

                    // Store reference to stream on controller so methods can access it
                    let controller_stream_key = v8::String::new(scope, "_stream").unwrap();
                    controller.set(scope, controller_stream_key.into(), this.into());

                    // enqueue(chunk) - adds a chunk to the queue array on the stream
                    let enqueue_fn = v8::Function::new(
                        scope,
                        |scope: &mut v8::PinScope,
                         args: v8::FunctionCallbackArguments,
                         _rv: v8::ReturnValue| {
                            if args.length() > 0 {
                                let chunk = args.get(0);
                                // Get the stream from controller (stored on controller)
                                let controller_obj = args.this();
                                let stream_key = v8::String::new(scope, "_stream").unwrap();
                                if let Some(stream_val) =
                                    controller_obj.get(scope, stream_key.into())
                                {
                                    if let Ok(stream) =
                                        v8::Local::<v8::Object>::try_from(stream_val)
                                    {
                                        let queue_key = v8::String::new(scope, "_queue").unwrap();
                                        if let Some(queue_val) = stream.get(scope, queue_key.into())
                                        {
                                            if let Ok(queue) =
                                                v8::Local::<v8::Array>::try_from(queue_val)
                                            {
                                                // Push to the JavaScript array
                                                let length = queue.length();
                                                queue.set_index(scope, length, chunk);
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    )
                    .unwrap();
                    let enqueue_key = v8::String::new(scope, "enqueue").unwrap();
                    controller.set(scope, enqueue_key.into(), enqueue_fn.into());

                    // close() - sets state to closed
                    let close_fn = v8::Function::new(
                        scope,
                        |scope: &mut v8::PinScope,
                         args: v8::FunctionCallbackArguments,
                         _rv: v8::ReturnValue| {
                            let controller_obj = args.this();
                            let stream_key = v8::String::new(scope, "_stream").unwrap();
                            let state_key = v8::String::new(scope, "_state").unwrap();
                            let state_val: v8::Local<v8::Value> = v8::Integer::new(scope, 1).into(); // 1 = Closed
                            if let Some(stream_val) = controller_obj.get(scope, stream_key.into()) {
                                if let Ok(stream) = v8::Local::<v8::Object>::try_from(stream_val) {
                                    stream.set(scope, state_key.into(), state_val);
                                }
                            }
                        },
                    )
                    .unwrap();
                    let close_key = v8::String::new(scope, "close").unwrap();
                    controller.set(scope, close_key.into(), close_fn.into());

                    // error(e) - sets state to errored
                    let error_fn = v8::Function::new(
                        scope,
                        |scope: &mut v8::PinScope,
                         args: v8::FunctionCallbackArguments,
                         _rv: v8::ReturnValue| {
                            let controller_obj = args.this();
                            let stream_key = v8::String::new(scope, "_stream").unwrap();
                            let state_key = v8::String::new(scope, "_state").unwrap();
                            let state_val: v8::Local<v8::Value> = v8::Integer::new(scope, 2).into(); // 2 = Errored
                            if let Some(stream_val) = controller_obj.get(scope, stream_key.into()) {
                                if let Ok(stream) = v8::Local::<v8::Object>::try_from(stream_val) {
                                    stream.set(scope, state_key.into(), state_val);
                                }
                            }
                        },
                    )
                    .unwrap();
                    let error_key = v8::String::new(scope, "error").unwrap();
                    controller.set(scope, error_key.into(), error_fn.into());

                    // Call start(controller)
                    let undefined = v8::undefined(scope).into();
                    let _ = start_fn.call(scope, undefined, &[controller.into()]);
                }
            }
        }
    }

    // Setup getReader method
    let get_reader_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let stream_this = args.this();
            if get_bool_property(_scope, stream_this, "locked") {
                throw_type_error(_scope, "ReadableStream is locked");
                return;
            }
            set_bool_property(_scope, stream_this, "locked", true);

            let reader: v8::Local<v8::Object> = v8::Object::new(_scope);

            // Store stream reference on reader object
            let stream_key = v8::String::new(_scope, "_stream").unwrap();
            reader.set(_scope, stream_key.into(), stream_this.into());
            set_bool_property(_scope, reader, "_released", false);

            // v0.3.294: Setup read() method - returns Promise<{done, value}>
            // Supports BYOB (Bring Your Own Buffer) when a TypedArray view is passed
            let read_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();
                    let result: v8::Local<v8::Object> = v8::Object::new(__scope);
                    let done_key: v8::Local<v8::String> = v8::String::new(__scope, "done").unwrap();
                    let value_key: v8::Local<v8::String> =
                        v8::String::new(__scope, "value").unwrap();

                    // v0.3.294: Check for BYOB view parameter (TypedArray/ArrayBufferView)
                    let mut byob_view: Option<v8::Local<v8::Object>> = None;
                    let mut byob_buffer: Option<v8::Local<v8::ArrayBuffer>> = None;
                    let mut byob_byte_offset = 0u64;
                    let mut byob_byte_length = 0u64;

                    if _a.length() > 0 {
                        let view_arg = _a.get(0);
                        if view_arg.is_object() {
                            if let Ok(view_obj) = v8::Local::<v8::Object>::try_from(view_arg) {
                                // Check if it's a TypedArray or ArrayBufferView
                                let buffer_key = v8::String::new(__scope, "buffer").unwrap();
                                if let Some(buffer_val) = view_obj.get(__scope, buffer_key.into()) {
                                    if buffer_val.is_array_buffer() {
                                        if let Ok(buffer) =
                                            v8::Local::<v8::ArrayBuffer>::try_from(buffer_val)
                                        {
                                            byob_view = Some(view_obj);
                                            byob_buffer = Some(buffer);

                                            // Get byteOffset and byteLength
                                            let byte_offset_key =
                                                v8::String::new(__scope, "byteOffset").unwrap();
                                            let byte_length_key =
                                                v8::String::new(__scope, "byteLength").unwrap();

                                            if let Some(offset_val) =
                                                view_obj.get(__scope, byte_offset_key.into())
                                            {
                                                byob_byte_offset =
                                                    offset_val.integer_value(__scope).unwrap_or(0)
                                                        as u64;
                                            }
                                            if let Some(length_val) =
                                                view_obj.get(__scope, byte_length_key.into())
                                            {
                                                byob_byte_length =
                                                    length_val.integer_value(__scope).unwrap_or(0)
                                                        as u64;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Get stream from reader object
                    let reader_this = _a.this();
                    let stream_key = v8::String::new(__scope, "_stream").unwrap();
                    if let Some(stream_val) = reader_this
                        .get(__scope, stream_key.into())
                        .filter(|s| s.is_object())
                    {
                        if let Ok(stream) = v8::Local::<v8::Object>::try_from(stream_val) {
                            // Get queue from stream object
                            let queue_key = v8::String::new(__scope, "_queue").unwrap();
                            let state_key = v8::String::new(__scope, "_state").unwrap();
                            let idx_key = v8::String::new(__scope, "_readIndex").unwrap();

                            if let Some(queue_val) = stream.get(__scope, queue_key.into()) {
                                if let Ok(queue) = v8::Local::<v8::Array>::try_from(queue_val) {
                                    // Get state
                                    let state = stream
                                        .get(__scope, state_key.into())
                                        .and_then(|s| s.to_integer(__scope))
                                        .map(|i| i.value() as i32)
                                        .unwrap_or(0);

                                    // Get read index
                                    let read_index = stream
                                        .get(__scope, idx_key.into())
                                        .and_then(|i| i.to_integer(__scope))
                                        .map(|i| i.value() as i32)
                                        .unwrap_or(0);

                                    let queue_length = queue.length();

                                    // Priority 1: Has chunks in queue - return next one
                                    if (read_index as u32) < queue_length {
                                        let chunk = queue
                                            .get_index(__scope, read_index as u32)
                                            .unwrap_or_else(|| v8::undefined(__scope).into());
                                        // Increment read index
                                        let new_idx: v8::Local<v8::Value> =
                                            v8::Integer::new(__scope, read_index + 1).into();
                                        stream.set(__scope, idx_key.into(), new_idx);
                                        let done_val: v8::Local<v8::Value> =
                                            v8::Boolean::new(__scope, false).into();
                                        result.set(__scope, done_key.into(), done_val);

                                        // v0.3.294: Handle BYOB (Bring Your Own Buffer)
                                        if let (Some(view_obj), Some(buffer)) =
                                            (byob_view, byob_buffer)
                                        {
                                            // BYOB mode: copy chunk data to the provided buffer
                                            let chunk_is_uint8array = chunk.is_uint8_array();

                                            if chunk_is_uint8array {
                                                // Get the chunk's buffer and copy data
                                                if let Ok(chunk_uint8) =
                                                    v8::Local::<v8::Uint8Array>::try_from(chunk)
                                                {
                                                    let chunk_buffer =
                                                        chunk_uint8.buffer(__scope).unwrap();
                                                    let chunk_store =
                                                        chunk_buffer.get_backing_store();
                                                    let chunk_data_ptr = chunk_store
                                                        .data()
                                                        .map(|p| p.as_ptr() as *const u8)
                                                        .unwrap_or(std::ptr::null());
                                                    let chunk_len = chunk_uint8.byte_length();

                                                    // Get target buffer info
                                                    let buffer_store = buffer.get_backing_store();
                                                    let target_ptr = buffer_store
                                                        .data()
                                                        .map(|p| p.as_ptr() as *mut u8)
                                                        .unwrap_or(std::ptr::null_mut());

                                                    // Calculate how many bytes we can copy
                                                    let bytes_to_copy = std::cmp::min(
                                                        chunk_len as u64,
                                                        byob_byte_length,
                                                    )
                                                        as usize;

                                                    // Copy data to BYOB buffer
                                                    unsafe {
                                                        std::ptr::copy_nonoverlapping(
                                                            chunk_data_ptr,
                                                            target_ptr
                                                                .add(byob_byte_offset as usize),
                                                            bytes_to_copy,
                                                        );
                                                    }

                                                    // Return the BYOB view (same view that was passed in)
                                                    result.set(
                                                        __scope,
                                                        value_key.into(),
                                                        view_obj.into(),
                                                    );

                                                    // Add bytesRead to result (Web Streams API BYOB specification)
                                                    let bytes_read_key =
                                                        v8::String::new(__scope, "bytesRead")
                                                            .unwrap();
                                                    let bytes_read_val: v8::Local<v8::Value> =
                                                        v8::Integer::new(
                                                            __scope,
                                                            bytes_to_copy as i32,
                                                        )
                                                        .into();
                                                    result.set(
                                                        __scope,
                                                        bytes_read_key.into(),
                                                        bytes_read_val,
                                                    );
                                                }
                                            } else {
                                                // Non-Uint8Array chunk, return as-is but still in BYOB mode
                                                result.set(__scope, value_key.into(), chunk);
                                            }
                                        } else {
                                            // Normal mode: return the chunk directly
                                            result.set(__scope, value_key.into(), chunk);
                                        }

                                        let _ = promise.resolve(__scope, result.into());
                                        _r.set(promise.into());
                                        return;
                                    }

                                    // Priority 2: Stream closed or errored, no more chunks
                                    if state == 1 || state == 2 {
                                        let done_val: v8::Local<v8::Value> =
                                            v8::Boolean::new(__scope, true).into();
                                        let undefined_val: v8::Local<v8::Value> =
                                            v8::undefined(__scope).into();
                                        result.set(__scope, done_key.into(), done_val);
                                        result.set(__scope, value_key.into(), undefined_val);
                                        let _ = promise.resolve(__scope, result.into());
                                        _r.set(promise.into());
                                        return;
                                    }

                                    // Priority 3: No chunks yet, stream still open - wait
                                    let done_val: v8::Local<v8::Value> =
                                        v8::Boolean::new(__scope, true).into();
                                    let undefined_val: v8::Local<v8::Value> =
                                        v8::undefined(__scope).into();
                                    result.set(__scope, done_key.into(), done_val);
                                    result.set(__scope, value_key.into(), undefined_val);
                                    let _ = promise.resolve(__scope, result.into());
                                    _r.set(promise.into());
                                    return;
                                }
                            }
                        }
                    }

                    // Default: stream done
                    let done_val: v8::Local<v8::Value> = v8::Boolean::new(__scope, true).into();
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    result.set(__scope, done_key.into(), done_val);
                    result.set(__scope, value_key.into(), undefined_val);
                    let _ = promise.resolve(__scope, result.into());
                    _r.set(promise.into());
                },
            );

            // Setup releaseLock() method
            let _release_fn = v8::FunctionTemplate::new(
                _scope,
                |_scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 _r: v8::ReturnValue| {
                    // releaseLock logic - for basic implementation, does nothing
                },
            );

            // Create closed Promise
            let closed_promise: v8::Local<v8::PromiseResolver> =
                v8::PromiseResolver::new(_scope).unwrap();
            let closed_promise_val: v8::Local<v8::Value> = closed_promise.into();
            let undefined_val: v8::Local<v8::Value> = v8::undefined(_scope).into();
            let _ = closed_promise.resolve(_scope, undefined_val);

            // Add methods and properties to reader
            let read_key: v8::Local<v8::String> = v8::String::new(_scope, "read").unwrap();
            let release_key: v8::Local<v8::String> =
                v8::String::new(_scope, "releaseLock").unwrap();
            let closed_key: v8::Local<v8::String> = v8::String::new(_scope, "closed").unwrap();

            let read_func: v8::Local<v8::Function> = read_fn.get_function(_scope).unwrap();
            let release_func = v8::Function::new(
                _scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _r: v8::ReturnValue| {
                    let reader_this = args.this();
                    if get_bool_property(_scope, reader_this, "_released") {
                        return;
                    }

                    let stream_key = v8::String::new(_scope, "_stream").unwrap();
                    if let Some(stream_val) = reader_this
                        .get(_scope, stream_key.into())
                        .filter(|s| s.is_object())
                    {
                        if let Ok(stream) = v8::Local::<v8::Object>::try_from(stream_val) {
                            set_bool_property(_scope, stream, "locked", false);
                        }
                    }
                    set_bool_property(_scope, reader_this, "_released", true);
                },
            )
            .unwrap();

            reader.set(_scope, read_key.into(), read_func.into());
            reader.set(_scope, release_key.into(), release_func.into());
            reader.set(_scope, closed_key.into(), closed_promise_val);

            retval.set(reader.into());
        },
    );

    let get_reader_key: v8::Local<v8::String> = v8::String::new(scope, "getReader").unwrap();
    let get_reader_func: v8::Local<v8::Function> = get_reader_fn.get_function(scope).unwrap();
    this.set(scope, get_reader_key.into(), get_reader_func.into());

    // v0.3.291: Setup pipeTo() method - pipes this readable to a writable stream
    // Supports options: { preventClose: boolean, preventAbort: boolean, signal: AbortSignal }
    // Returns a Promise that resolves when piping completes or rejects on error
    let pipe_to_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Get the destination writable stream
            let dest_writable = args.get(0);
            if !dest_writable.is_object() {
                let error =
                    v8::String::new(_scope, "Destination must be a WritableStream").unwrap();
                retval.set(error.into());
                return;
            }

            // Parse options (second argument) for preventClose, preventAbort, and signal
            let mut prevent_close = false;
            let mut prevent_abort = false;
            let mut signal_obj: Option<v8::Local<v8::Object>> = None;
            if args.length() > 1 {
                let options = args.get(1);
                if options.is_object() {
                    let options_obj = v8::Local::<v8::Object>::try_from(options).unwrap();

                    let prevent_close_key = v8::String::new(_scope, "preventClose").unwrap();
                    if let Some(prevent_val) = options_obj.get(_scope, prevent_close_key.into()) {
                        prevent_close = prevent_val.is_true();
                    }

                    let prevent_abort_key = v8::String::new(_scope, "preventAbort").unwrap();
                    if let Some(abort_val) = options_obj.get(_scope, prevent_abort_key.into()) {
                        prevent_abort = abort_val.is_true();
                    }

                    let signal_key = v8::String::new(_scope, "signal").unwrap();
                    if let Some(sig_val) = options_obj.get(_scope, signal_key.into()) {
                        if sig_val.is_object() {
                            signal_obj = Some(v8::Local::<v8::Object>::try_from(sig_val).unwrap());
                        }
                    }
                }
            }

            // Store reference to this stream
            let this_stream = args.this();

            // v0.3.291: Create pump code with signal handling
            // The pump function handles errors, state transitions, and promise resolution/rejection
            // It also monitors the abort signal and rejects appropriately
            let pump_code = if prevent_close {
                r#"
                (function(readable, writable, preventAbort, signal) {
                    var reader = readable.getReader();
                    var writer = writable.getWriter();
                    var rejected = false;
                    var done = false;

                    // v0.3.291: Check if already aborted
                    if (signal && signal.aborted) {
                        var err = new Error('Aborted');
                        err.name = 'AbortError';
                        return Promise.reject(err);
                    }

                    function pump() {
                        return reader.read().then(function(result) {
                            if (rejected || done) return;
                            if (result.done) {
                                done = true;
                                return;
                            }
                            return writer.write(result.value).then(function() {
                                return pump();
                            });
                        }, function(err) {
                            if (rejected) return;
                            rejected = true;
                            if (preventAbort) {
                                return Promise.reject(err);
                            }
                            return writer.abort(err).then(function() {
                                return Promise.reject(err);
                            });
                        });
                    }

                    var pumpPromise = pump();

                    // v0.3.291: Set up abort listener if signal provided
                    if (signal && signal.addEventListener) {
                        signal.addEventListener('abort', function() {
                            if (rejected) return;
                            rejected = true;
                            var err = new Error('Aborted');
                            err.name = 'AbortError';
                            if (preventAbort) {
                                pumpPromise = Promise.reject(err);
                            } else {
                                pumpPromise = writer.abort(err).then(function() {
                                    return Promise.reject(err);
                                });
                            }
                        });
                    }

                    return pumpPromise;
                })
            "#
            } else {
                r#"
                (function(readable, writable, preventAbort, signal) {
                    var reader = readable.getReader();
                    var writer = writable.getWriter();
                    var rejected = false;
                    var done = false;

                    // v0.3.291: Check if already aborted
                    if (signal && signal.aborted) {
                        var err = new Error('Aborted');
                        err.name = 'AbortError';
                        return Promise.reject(err);
                    }

                    function pump() {
                        return reader.read().then(function(result) {
                            if (rejected || done) return;
                            if (result.done) {
                                done = true;
                                return writer.close().catch(function(err) {
                                    if (rejected) return;
                                    rejected = true;
                                    if (preventAbort) {
                                        return Promise.reject(err);
                                    }
                                    return writer.abort(err).then(function() {
                                        return Promise.reject(err);
                                    });
                                });
                            }
                            return writer.write(result.value).then(function() {
                                return pump();
                            }, function(err) {
                                if (rejected) return;
                                rejected = true;
                                if (preventAbort) {
                                    return Promise.reject(err);
                                }
                                return writer.abort(err).then(function() {
                                    return Promise.reject(err);
                                });
                            });
                        }, function(err) {
                            if (rejected) return;
                            rejected = true;
                            if (preventAbort) {
                                return Promise.reject(err);
                            }
                            return writer.abort(err).then(function() {
                                return Promise.reject(err);
                            });
                        });
                    }

                    var pumpPromise = pump();

                    // v0.3.291: Set up abort listener if signal provided
                    if (signal && signal.addEventListener) {
                        signal.addEventListener('abort', function() {
                            if (rejected) return;
                            rejected = true;
                            var err = new Error('Aborted');
                            err.name = 'AbortError';
                            if (preventAbort) {
                                pumpPromise = Promise.reject(err);
                            } else {
                                pumpPromise = writer.abort(err).then(function() {
                                    return Promise.reject(err);
                                });
                            }
                        });
                    }

                    return pumpPromise;
                })
            "#
            };

            let pump_fn_str = v8::String::new(_scope, pump_code).unwrap();
            let pump_fn_val = v8::Script::compile(_scope, pump_fn_str, None)
                .unwrap()
                .run(_scope)
                .unwrap();

            if let Ok(pump_fn) = v8::Local::<v8::Function>::try_from(pump_fn_val) {
                let undefined = v8::undefined(_scope).into();
                let this_stream_val: v8::Local<v8::Value> = this_stream.into();
                let dest_writable_val: v8::Local<v8::Value> = dest_writable.into();
                let prevent_abort_val: v8::Local<v8::Value> =
                    v8::Boolean::new(_scope, prevent_abort).into();

                // v0.3.291: Pass signal to pump function (as undefined if not provided)
                let signal_val: v8::Local<v8::Value> = if let Some(ref sig) = signal_obj {
                    sig.clone().into()
                } else {
                    v8::undefined(_scope).into()
                };

                // Call pump to get the pump promise
                let pump_promise_val = pump_fn.call(
                    _scope,
                    undefined,
                    &[
                        this_stream_val,
                        dest_writable_val,
                        prevent_abort_val,
                        signal_val,
                    ],
                );

                // Return the pump promise directly - this is the promise that will resolve/reject
                // when piping is complete or encounters an error
                retval.set(pump_promise_val.unwrap_or(undefined));
                return;
            }

            retval.set(v8::undefined(_scope).into());
        },
    );
    let pipe_to_key: v8::Local<v8::String> = v8::String::new(scope, "pipeTo").unwrap();
    let pipe_to_func: v8::Local<v8::Function> = pipe_to_fn.get_function(scope).unwrap();
    this.set(scope, pipe_to_key.into(), pipe_to_func.into());

    // v0.3.288: Setup pipeThrough() method - pipes this readable through a transform
    let pipe_through_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Get the transform stream
            let transform = args.get(0);
            if !transform.is_object() {
                let undefined = v8::undefined(_scope).into();
                retval.set(undefined);
                return;
            }

            let transform_obj = v8::Local::<v8::Object>::try_from(transform).unwrap();

            // Get the readable and writable properties from transform
            let readable_key = v8::String::new(_scope, "readable").unwrap();
            let writable_key = v8::String::new(_scope, "writable").unwrap();

            let transform_readable = transform_obj.get(_scope, readable_key.into());
            let transform_writable = transform_obj.get(_scope, writable_key.into());

            let Some(transform_writable) = transform_writable else {
                let undefined = v8::undefined(_scope).into();
                retval.set(undefined);
                return;
            };

            // Pipe this readable to transform's writable side using pipeTo
            let this_stream = args.this();
            let writable_val: v8::Local<v8::Value> = transform_writable.into();

            // Use JavaScript to perform the pipeTo operation
            let pipe_code = r#"
            (function(readable, writable) {
                return readable.pipeTo(writable);
            })
        "#;

            let pipe_fn_str = v8::String::new(_scope, pipe_code).unwrap();
            if let Some(pipe_fn_val) = v8::Script::compile(_scope, pipe_fn_str, None)
                .unwrap()
                .run(_scope)
            {
                if let Ok(pipe_fn) = v8::Local::<v8::Function>::try_from(pipe_fn_val) {
                    let this_val: v8::Local<v8::Value> = this_stream.into();
                    let _ = pipe_fn.call(_scope, this_val, &[this_val, writable_val]);
                }
            }

            if let Some(readable) = transform_readable {
                if let Ok(readable_obj) = v8::Local::<v8::Object>::try_from(readable) {
                    let readable_key = v8::String::new(_scope, "readable").unwrap();
                    readable_obj.set(_scope, readable_key.into(), readable);
                    let writable_key = v8::String::new(_scope, "writable").unwrap();
                    readable_obj.set(_scope, writable_key.into(), transform_writable);
                }
                retval.set(readable);
                return;
            }

            retval.set(v8::undefined(_scope).into());
        },
    );
    let pipe_through_key: v8::Local<v8::String> = v8::String::new(scope, "pipeThrough").unwrap();
    let pipe_through_func: v8::Local<v8::Function> = pipe_through_fn.get_function(scope).unwrap();
    this.set(scope, pipe_through_key.into(), pipe_through_func.into());

    // Setup locked property
    let locked_value: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
    let locked_key: v8::Local<v8::String> = v8::String::new(scope, "locked").unwrap();
    this.set(scope, locked_key.into(), locked_value);

    retval.set(this.into());
}

// ============================================================
// WritableStream Implementation
// ============================================================

/// WritableStream constructor - enhanced with start() callback and write queue
/// Uses JavaScript arrays on the stream object for write queue storage
fn writable_stream_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: v8::Local<v8::Object> = args.this();

    // Create JavaScript array to store chunks pending write
    let write_queue_array = v8::Array::new(scope, 0);
    let write_queue_key = v8::String::new(scope, "_writeQueue").unwrap();
    this.set(scope, write_queue_key.into(), write_queue_array.into());

    // Create state variable (0=open, 1=closed, 2=errored)
    let state_key = v8::String::new(scope, "_state").unwrap();
    let state_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into(); // 0 = Open
    this.set(scope, state_key.into(), state_val);

    // Create index for tracking write position
    let write_index_key = v8::String::new(scope, "_writeIndex").unwrap();
    let write_index_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
    this.set(scope, write_index_key.into(), write_index_val);

    // Store reference to the writable stream on itself for controller access
    let self_key = v8::String::new(scope, "_writable").unwrap();
    this.set(scope, self_key.into(), this.into());

    // Check if first argument is an object with start/write methods
    if args.length() > 0 {
        let underlying_sink = args.get(0);
        if underlying_sink.is_object() {
            let sink_obj = v8::Local::<v8::Object>::try_from(underlying_sink).unwrap();

            // Store write callback if provided
            let write_key = v8::String::new(scope, "write").unwrap();
            if let Some(write_fn_val) = sink_obj.get(scope, write_key.into()) {
                if write_fn_val.is_function() {
                    let write_cb_key = v8::String::new(scope, "_writeCallback").unwrap();
                    this.set(scope, write_cb_key.into(), write_fn_val);
                }
            }

            let start_key = v8::String::new(scope, "start").unwrap();
            if let Some(start_fn_val) = sink_obj.get(scope, start_key.into()) {
                if start_fn_val.is_function() {
                    let start_fn = v8::Local::<v8::Function>::try_from(start_fn_val).unwrap();

                    // Create controller object with writeable reference
                    let controller = v8::Object::new(scope);

                    // Store reference to writable stream on controller
                    let controller_writable_key = v8::String::new(scope, "_writable").unwrap();
                    controller.set(scope, controller_writable_key.into(), this.into());

                    // Call start(controller)
                    let undefined = v8::undefined(scope).into();
                    let _ = start_fn.call(scope, undefined, &[controller.into()]);
                }
            }
        }
    }

    // Setup getWriter method
    let get_writer_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let writable_this = _args.this();
            if get_bool_property(_scope, writable_this, "locked") {
                throw_type_error(_scope, "WritableStream is locked");
                return;
            }
            set_bool_property(_scope, writable_this, "locked", true);

            let writer: v8::Local<v8::Object> = v8::Object::new(_scope);

            // Store reference to writable stream on writer
            let writable_key = v8::String::new(_scope, "_writable").unwrap();
            writer.set(_scope, writable_key.into(), writable_this.into());
            set_bool_property(_scope, writer, "_released", false);

            // v0.3.292: Setup write() method - adds chunk to queue and calls write callback
            let write_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();

                    // Get writable from writer object
                    let writer_this = args.this();
                    let writable_key = v8::String::new(__scope, "_writable").unwrap();

                    if let Some(writable_val) = writer_this.get(__scope, writable_key.into()) {
                        if writable_val.is_object() {
                            if let Ok(writable) = v8::Local::<v8::Object>::try_from(writable_val) {
                                // Get write queue and state
                                let queue_key = v8::String::new(__scope, "_writeQueue").unwrap();
                                let state_key = v8::String::new(__scope, "_state").unwrap();

                                // Get the stored write callback
                                let write_cb_key =
                                    v8::String::new(__scope, "_writeCallback").unwrap();
                                let write_callback = writable.get(__scope, write_cb_key.into());

                                if let Some(queue_val) = writable.get(__scope, queue_key.into()) {
                                    if let Ok(queue) = v8::Local::<v8::Array>::try_from(queue_val) {
                                        // Get state
                                        let state = writable
                                            .get(__scope, state_key.into())
                                            .and_then(|s| s.to_integer(__scope))
                                            .map(|i| i.value() as i32)
                                            .unwrap_or(0);

                                        // Only add to queue if stream is open
                                        if state == 0 && args.length() > 0 {
                                            let chunk = args.get(0);
                                            let length = queue.length();
                                            queue.set_index(__scope, length, chunk);

                                            // v0.3.292: Call the user's write callback if provided
                                            if let Some(cb) = write_callback {
                                                if cb.is_function() {
                                                    if let Ok(cb_fn) =
                                                        v8::Local::<v8::Function>::try_from(cb)
                                                    {
                                                        let undefined =
                                                            v8::undefined(__scope).into();
                                                        // Call the write callback and get the return value
                                                        let cb_result = cb_fn.call(
                                                            __scope,
                                                            undefined,
                                                            &[chunk],
                                                        );

                                                        // v0.3.292: Check if callback returned a Promise and handle async
                                                        if let Some(result_val) = cb_result {
                                                            if result_val.is_object() {
                                                                // Check if callback result is thenable
                                                                if let Ok(result_obj) = v8::Local::<
                                                                    v8::Object,
                                                                >::try_from(
                                                                    result_val
                                                                ) {
                                                                    let then_key = v8::String::new(
                                                                        __scope, "then",
                                                                    )
                                                                    .unwrap();
                                                                    if let Some(then_fn_val) =
                                                                        result_obj.get(
                                                                            __scope,
                                                                            then_key.into(),
                                                                        )
                                                                    {
                                                                        if then_fn_val.is_function()
                                                                        {
                                                                            if let Ok(then_fn) =
                                                                                v8::Local::<
                                                                                    v8::Function,
                                                                                >::try_from(
                                                                                    then_fn_val,
                                                                                )
                                                                            {
                                                                                // Create a Promise in JavaScript that we'll chain to the callback's Promise
                                                                                // This is more reliable than trying to access V8's PromiseResolver from JS
                                                                                let promise_code = r#"
                                                                            (function() {
                                                                                var resolver = null;
                                                                                var promise = new Promise(function(resolve, reject) {
                                                                                    resolver = { resolve: resolve, reject: reject };
                                                                                });
                                                                                return { promise: promise, resolver: resolver };
                                                                            })
                                                                        "#;
                                                                                let promise_str = v8::String::new(__scope, promise_code).unwrap();
                                                                                if let Some(promise_compiled) = v8::Script::compile(__scope, promise_str, None) {
                                                                            if let Some(promise_run) = promise_compiled.run(__scope) {
                                                                                if let Ok(promise_fn) = v8::Local::<v8::Function>::try_from(promise_run) {
                                                                                    if let Some(result) = promise_fn.call(__scope, undefined.into(), &[]) {
                                                                                        // Get the promise and its resolver
                                                                                        if let Ok(result_obj) = v8::Local::<v8::Object>::try_from(result) {
                                                                                            let promise_key = v8::String::new(__scope, "promise").unwrap();
                                                                                            let resolver_key = v8::String::new(__scope, "resolver").unwrap();

                                                                                            let inner_promise = result_obj.get(__scope, promise_key.into()).unwrap();
                                                                                            let inner_resolver = result_obj.get(__scope, resolver_key.into()).unwrap();

                                                                                            // Chain the callback's Promise to our resolver
                                                                                            let chain_code = r#"
                                                                                                (function(thenFn, callbackResult, resolver) {
                                                                                                    if (resolver && resolver.resolve) {
                                                                                                        thenFn.call(callbackResult,
                                                                                                            function(v) { resolver.resolve(v); },
                                                                                                            function(e) { resolver.reject(e); }
                                                                                                        );
                                                                                                    }
                                                                                                })
                                                                                            "#;
                                                                                            let chain_str = v8::String::new(__scope, chain_code).unwrap();
                                                                                            if let Some(chain_compiled) = v8::Script::compile(__scope, chain_str, None) {
                                                                                                if let Some(chain_run) = chain_compiled.run(__scope) {
                                                                                                    if let Ok(chain_fn) = v8::Local::<v8::Function>::try_from(chain_run) {
                                                                                                        let _ = chain_fn.call(__scope, undefined.into(), &[then_fn.into(), result_val, inner_resolver]);

                                                                                                        // Return the inner promise
                                                                                                        _r.set(inner_promise);
                                                                                                        return;
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Resolve promise immediately for sync case
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            // Setup close() method
            let close_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();

                    // Get writable from writer object
                    let writer_this = args.this();
                    let writable_key = v8::String::new(__scope, "_writable").unwrap();

                    if let Some(writable_val) = writer_this
                        .get(__scope, writable_key.into())
                        .filter(|s| s.is_object())
                    {
                        if let Ok(writable) = v8::Local::<v8::Object>::try_from(writable_val) {
                            let state_key = v8::String::new(__scope, "_state").unwrap();
                            let state_val: v8::Local<v8::Value> =
                                v8::Integer::new(__scope, 1).into(); // 1 = Closed
                            writable.set(__scope, state_key.into(), state_val);
                        }
                    }

                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            // Setup abort() method
            let abort_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();

                    // Get writable from writer object
                    let writer_this = args.this();
                    let writable_key = v8::String::new(__scope, "_writable").unwrap();

                    if let Some(writable_val) = writer_this
                        .get(__scope, writable_key.into())
                        .filter(|s| s.is_object())
                    {
                        if let Ok(writable) = v8::Local::<v8::Object>::try_from(writable_val) {
                            let state_key = v8::String::new(__scope, "_state").unwrap();
                            let state_val: v8::Local<v8::Value> =
                                v8::Integer::new(__scope, 2).into(); // 2 = Errored
                            writable.set(__scope, state_key.into(), state_val);
                        }
                    }

                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            // Create ready Promise
            let ready_promise: v8::Local<v8::PromiseResolver> =
                v8::PromiseResolver::new(_scope).unwrap();
            let ready_promise_val: v8::Local<v8::Value> = ready_promise.into();
            let undefined_val: v8::Local<v8::Value> = v8::undefined(_scope).into();
            let _ = ready_promise.resolve(_scope, undefined_val);

            // Create closed Promise
            let closed_promise: v8::Local<v8::PromiseResolver> =
                v8::PromiseResolver::new(_scope).unwrap();
            let closed_promise_val: v8::Local<v8::Value> = closed_promise.into();
            let _ = closed_promise.resolve(_scope, undefined_val);

            // Add methods and properties
            let write_key: v8::Local<v8::String> = v8::String::new(_scope, "write").unwrap();
            let close_key: v8::Local<v8::String> = v8::String::new(_scope, "close").unwrap();
            let abort_key: v8::Local<v8::String> = v8::String::new(_scope, "abort").unwrap();
            let ready_key: v8::Local<v8::String> = v8::String::new(_scope, "ready").unwrap();
            let closed_key: v8::Local<v8::String> = v8::String::new(_scope, "closed").unwrap();
            let release_key: v8::Local<v8::String> =
                v8::String::new(_scope, "releaseLock").unwrap();

            let write_func: v8::Local<v8::Function> = write_fn.get_function(_scope).unwrap();
            let close_func: v8::Local<v8::Function> = close_fn.get_function(_scope).unwrap();
            let abort_func: v8::Local<v8::Function> = abort_fn.get_function(_scope).unwrap();
            let release_func = v8::Function::new(
                _scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    let writer_this = args.this();
                    if get_bool_property(_scope, writer_this, "_released") {
                        return;
                    }

                    let writable_key = v8::String::new(_scope, "_writable").unwrap();
                    if let Some(writable_val) = writer_this
                        .get(_scope, writable_key.into())
                        .filter(|s| s.is_object())
                    {
                        if let Ok(writable) = v8::Local::<v8::Object>::try_from(writable_val) {
                            set_bool_property(_scope, writable, "locked", false);
                        }
                    }
                    set_bool_property(_scope, writer_this, "_released", true);
                },
            )
            .unwrap();

            writer.set(_scope, write_key.into(), write_func.into());
            writer.set(_scope, close_key.into(), close_func.into());
            writer.set(_scope, abort_key.into(), abort_func.into());
            writer.set(_scope, ready_key.into(), ready_promise_val);
            writer.set(_scope, closed_key.into(), closed_promise_val);
            writer.set(_scope, release_key.into(), release_func.into());

            // desiredSize ≈ highWaterMark - queue size (WHATWG-ish backpressure signal)
            let hwm_key = v8::String::new(_scope, "_highWaterMark").unwrap();
            let queue_key = v8::String::new(_scope, "_queue").unwrap();
            let hwm = writable_this
                .get(_scope, hwm_key.into())
                .and_then(|v| v.number_value(_scope))
                .unwrap_or(1.0);
            let queued = writable_this
                .get(_scope, queue_key.into())
                .and_then(|v| v8::Local::<v8::Array>::try_from(v).ok())
                .map(|a| a.length() as f64)
                .unwrap_or(0.0);
            let desired_size_key: v8::Local<v8::String> =
                v8::String::new(_scope, "desiredSize").unwrap();
            let desired_size_val: v8::Local<v8::Value> =
                v8::Number::new(_scope, hwm - queued).into();
            writer.set(_scope, desired_size_key.into(), desired_size_val);

            retval.set(writer.into());
        },
    );

    let get_writer_key: v8::Local<v8::String> = v8::String::new(scope, "getWriter").unwrap();
    let get_writer_func: v8::Local<v8::Function> = get_writer_fn.get_function(scope).unwrap();
    this.set(scope, get_writer_key.into(), get_writer_func.into());

    // Setup locked property (tracks if writer is attached)
    let locked_value: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
    let locked_key: v8::Local<v8::String> = v8::String::new(scope, "locked").unwrap();
    this.set(scope, locked_key.into(), locked_value);

    retval.set(this.into());
}

// ============================================================
// TransformStream Implementation
// ============================================================

/// TransformStream constructor - enhanced with transform() support
/// Connects writable stream writes to readable stream output via transformer
fn transform_stream_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let transform_obj: v8::Local<v8::Object> = v8::Object::new(scope);

    // Create queue for transformed chunks (shared between writable and readable)
    let transform_queue = v8::Array::new(scope, 0);
    let queue_key = v8::String::new(scope, "_transformQueue").unwrap();
    transform_obj.set(scope, queue_key.into(), transform_queue.into());

    // Create read index for readable side
    let read_index_key = v8::String::new(scope, "_readIndex").unwrap();
    let read_index_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
    transform_obj.set(scope, read_index_key.into(), read_index_val);

    // Create state: 0=readable, 1=error, 2=closed
    let ts_state_key = v8::String::new(scope, "_transformState").unwrap();
    let ts_state_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
    transform_obj.set(scope, ts_state_key.into(), ts_state_val);

    // Store transformer reference if provided
    if args.length() > 0 {
        let transformer = args.get(0);
        if transformer.is_object() {
            let transformer_obj = v8::Local::<v8::Object>::try_from(transformer).unwrap();

            // Check for transform() method
            let transform_key = v8::String::new(scope, "transform").unwrap();
            if let Some(transform_fn_val) = transformer_obj.get(scope, transform_key.into()) {
                if transform_fn_val.is_function() {
                    // Store transform function on transform object for closure access
                    let tf_key = v8::String::new(scope, "_transformFn").unwrap();
                    transform_obj.set(scope, tf_key.into(), transform_fn_val);
                }
            }

            // v0.3.287: Check for flush() method
            let flush_key = v8::String::new(scope, "flush").unwrap();
            if let Some(flush_fn_val) = transformer_obj.get(scope, flush_key.into()) {
                if flush_fn_val.is_function() {
                    // Store flush function for calling during close
                    let flush_key_name = v8::String::new(scope, "_flushFn").unwrap();
                    transform_obj.set(scope, flush_key_name.into(), flush_fn_val);
                }
            }

            // Create transform controller with enqueue method
            let controller = v8::Object::new(scope);
            let controller_queue_key = v8::String::new(scope, "_queue").unwrap();
            controller.set(scope, controller_queue_key.into(), transform_queue.into());
            let controller_state_key = v8::String::new(scope, "_state").unwrap();
            controller.set(scope, controller_state_key.into(), ts_state_val);
            let controller_readable_key = v8::String::new(scope, "_readable").unwrap();
            controller.set(scope, controller_readable_key.into(), transform_obj.into());

            // Store controller on transform object for closure access
            let ctrl_key = v8::String::new(scope, "_controller").unwrap();
            transform_obj.set(scope, ctrl_key.into(), controller.into());

            // enqueue(chunk) - adds to the transform queue
            let enqueue_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    if args.length() > 0 {
                        let chunk = args.get(0);
                        let ctrl = args.this();
                        let queue_key = v8::String::new(scope, "_queue").unwrap();
                        if let Some(queue_val) = ctrl.get(scope, queue_key.into()) {
                            if let Ok(queue) = v8::Local::<v8::Array>::try_from(queue_val) {
                                let length = queue.length();
                                queue.set_index(scope, length, chunk);
                            }
                        }
                    }
                },
            )
            .unwrap();
            let enqueue_key = v8::String::new(scope, "enqueue").unwrap();
            controller.set(scope, enqueue_key.into(), enqueue_fn.into());

            // close() - marks transform as done
            let close_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    let ctrl = args.this();
                    let state_key = v8::String::new(scope, "_state").unwrap();
                    let state_val: v8::Local<v8::Value> = v8::Integer::new(scope, 2).into(); // 2 = Closed
                    ctrl.set(scope, state_key.into(), state_val);
                },
            )
            .unwrap();
            let close_key = v8::String::new(scope, "close").unwrap();
            controller.set(scope, close_key.into(), close_fn.into());

            // error(e) - marks transform as errored
            let error_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    let ctrl = args.this();
                    let state_key = v8::String::new(scope, "_state").unwrap();
                    let state_val: v8::Local<v8::Value> = v8::Integer::new(scope, 1).into(); // 1 = Errored
                    ctrl.set(scope, state_key.into(), state_val);
                },
            )
            .unwrap();
            let error_key = v8::String::new(scope, "error").unwrap();
            controller.set(scope, error_key.into(), error_fn.into());

            // Call start(controller) if provided
            let start_key = v8::String::new(scope, "start").unwrap();
            if let Some(start_fn_val) = transformer_obj.get(scope, start_key.into()) {
                if start_fn_val.is_function() {
                    let start_fn = v8::Local::<v8::Function>::try_from(start_fn_val).unwrap();
                    let undefined = v8::undefined(scope).into();
                    let _ = start_fn.call(scope, undefined, &[controller.into()]);
                }
            }
        }
    }

    // Create readable stream with getReader method
    let readable_stream: v8::Local<v8::Object> = v8::Object::new(scope);

    // Store transform reference on readable
    let readable_transform_key = v8::String::new(scope, "_transform").unwrap();
    readable_stream.set(scope, readable_transform_key.into(), transform_obj.into());

    // Create getReader template for readable stream
    let readable_get_reader_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let reader: v8::Local<v8::Object> = v8::Object::new(_scope);

            // Store transform reference on reader
            let reader_transform_key = v8::String::new(_scope, "_transform").unwrap();
            let this_stream = args.this();
            if let Some(transform_val) = this_stream.get(_scope, reader_transform_key.into()) {
                reader.set(_scope, reader_transform_key.into(), transform_val);
            }

            // Setup read() method - reads from transform queue
            let read_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();
                    let result: v8::Local<v8::Object> = v8::Object::new(__scope);
                    let done_key: v8::Local<v8::String> = v8::String::new(__scope, "done").unwrap();
                    let value_key: v8::Local<v8::String> =
                        v8::String::new(__scope, "value").unwrap();

                    // Get transform from reader
                    let reader_this = _a.this();
                    let transform_key = v8::String::new(__scope, "_transform").unwrap();

                    if let Some(transform_val) = reader_this
                        .get(__scope, transform_key.into())
                        .filter(|s| s.is_object())
                    {
                        if let Ok(transform) = v8::Local::<v8::Object>::try_from(transform_val) {
                            let queue_key = v8::String::new(__scope, "_transformQueue").unwrap();
                            let idx_key = v8::String::new(__scope, "_readIndex").unwrap();
                            let state_key = v8::String::new(__scope, "_transformState").unwrap();

                            if let Some(queue_val) = transform.get(__scope, queue_key.into()) {
                                if let Ok(queue) = v8::Local::<v8::Array>::try_from(queue_val) {
                                    let read_index = transform
                                        .get(__scope, idx_key.into())
                                        .and_then(|i| i.to_integer(__scope))
                                        .map(|i| i.value() as u32)
                                        .unwrap_or(0);
                                    let state = transform
                                        .get(__scope, state_key.into())
                                        .and_then(|s| s.to_integer(__scope))
                                        .map(|i| i.value() as i32)
                                        .unwrap_or(0);
                                    let queue_len = queue.length();

                                    if read_index < queue_len {
                                        let chunk = queue
                                            .get_index(__scope, read_index)
                                            .unwrap_or_else(|| v8::undefined(__scope).into());
                                        let new_idx: v8::Local<v8::Value> =
                                            v8::Integer::new(__scope, read_index as i32 + 1).into();
                                        transform.set(__scope, idx_key.into(), new_idx);

                                        let done_val: v8::Local<v8::Value> =
                                            v8::Boolean::new(__scope, false).into();
                                        result.set(__scope, done_key.into(), done_val);
                                        result.set(__scope, value_key.into(), chunk);
                                        let _ = promise.resolve(__scope, result.into());
                                        _r.set(promise.into());
                                        return;
                                    }

                                    // Check if transform is closed
                                    if state == 2 {
                                        let done_val: v8::Local<v8::Value> =
                                            v8::Boolean::new(__scope, true).into();
                                        let undefined_val: v8::Local<v8::Value> =
                                            v8::undefined(__scope).into();
                                        result.set(__scope, done_key.into(), done_val);
                                        result.set(__scope, value_key.into(), undefined_val);
                                        let _ = promise.resolve(__scope, result.into());
                                        _r.set(promise.into());
                                        return;
                                    }
                                }
                            }
                        }
                    }

                    // Default: not done, no value
                    let done_val: v8::Local<v8::Value> = v8::Boolean::new(__scope, true).into();
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    result.set(__scope, done_key.into(), done_val);
                    result.set(__scope, value_key.into(), undefined_val);
                    let _ = promise.resolve(__scope, result.into());
                    _r.set(promise.into());
                },
            );

            // Setup releaseLock() method
            let release_fn = v8::FunctionTemplate::new(
                _scope,
                |_scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 _r: v8::ReturnValue| {},
            );

            // Create closed Promise
            let closed_promise: v8::Local<v8::PromiseResolver> =
                v8::PromiseResolver::new(_scope).unwrap();
            let closed_promise_val: v8::Local<v8::Value> = closed_promise.into();
            let undefined_val: v8::Local<v8::Value> = v8::undefined(_scope).into();
            let _ = closed_promise.resolve(_scope, undefined_val);

            // Add methods and properties to reader
            let read_key: v8::Local<v8::String> = v8::String::new(_scope, "read").unwrap();
            let release_key: v8::Local<v8::String> =
                v8::String::new(_scope, "releaseLock").unwrap();
            let closed_key: v8::Local<v8::String> = v8::String::new(_scope, "closed").unwrap();

            let read_func: v8::Local<v8::Function> = read_fn.get_function(_scope).unwrap();
            let release_func: v8::Local<v8::Function> = release_fn.get_function(_scope).unwrap();

            reader.set(_scope, read_key.into(), read_func.into());
            reader.set(_scope, release_key.into(), release_func.into());
            reader.set(_scope, closed_key.into(), closed_promise_val);

            retval.set(reader.into());
        },
    );
    readable_get_reader_fn
        .set_class_name(v8::String::new(scope, "TransformReadableStreamReader").unwrap());

    // Setup getReader on readable stream
    let readable_get_reader_key: v8::Local<v8::String> =
        v8::String::new(scope, "getReader").unwrap();
    if let Some(func) = readable_get_reader_fn.get_function(scope) {
        readable_stream.set(scope, readable_get_reader_key.into(), func.into());
    }

    let readable_locked_key: v8::Local<v8::String> = v8::String::new(scope, "locked").unwrap();
    let readable_locked_val: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
    readable_stream.set(scope, readable_locked_key.into(), readable_locked_val);

    // Create writable stream with getWriter method
    let writable_stream: v8::Local<v8::Object> = v8::Object::new(scope);

    // Store transform reference on writable
    let writable_transform_key = v8::String::new(scope, "_transform").unwrap();
    writable_stream.set(scope, writable_transform_key.into(), transform_obj.into());

    // Create getWriter template for writable stream
    let writable_get_writer_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let writer: v8::Local<v8::Object> = v8::Object::new(_scope);

            // Store transform reference on writer
            let writer_transform_key = v8::String::new(_scope, "_transform").unwrap();
            let this_writable = args.this();
            if let Some(transform_val) = this_writable.get(_scope, writer_transform_key.into()) {
                writer.set(_scope, writer_transform_key.into(), transform_val);
            }

            // Setup write() method - calls transform() and enqueues result
            // Transform function and controller are stored on transform object for closure access
            let write_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();

                    // Get transform from writer
                    let writer_this = a.this();
                    let transform_key = v8::String::new(__scope, "_transform").unwrap();

                    if let Some(transform_val) = writer_this
                        .get(__scope, transform_key.into())
                        .filter(|s| s.is_object())
                    {
                        if let Ok(transform) = v8::Local::<v8::Object>::try_from(transform_val) {
                            // Get transform function from transform object
                            let tf_key = v8::String::new(__scope, "_transformFn").unwrap();
                            if let Some(tf_val) = transform.get(__scope, tf_key.into()) {
                                if let Ok(tf) = v8::Local::<v8::Function>::try_from(tf_val) {
                                    // Get controller from transform
                                    let ctrl_key = v8::String::new(__scope, "_controller").unwrap();
                                    if let Some(ctrl_val) = transform.get(__scope, ctrl_key.into())
                                    {
                                        if let Ok(ctrl) =
                                            v8::Local::<v8::Object>::try_from(ctrl_val)
                                        {
                                            if a.length() > 0 {
                                                let chunk = a.get(0);
                                                let undefined = v8::undefined(__scope).into();
                                                let _ = tf.call(
                                                    __scope,
                                                    undefined,
                                                    &[chunk, ctrl.into()],
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            // v0.3.338: Enhanced close() method - properly handles async flush with Promise support
            // If flush returns a Promise, we chain it and return the chained Promise to JavaScript
            let close_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    // Get transform from writer
                    let writer_this = a.this();
                    let transform_key = v8::String::new(__scope, "_transform").unwrap();

                    if let Some(transform_val) = writer_this
                        .get(__scope, transform_key.into())
                        .filter(|s| s.is_object())
                    {
                        if let Ok(transform) = v8::Local::<v8::Object>::try_from(transform_val) {
                            // v0.3.338: Call flush callback if provided
                            let flush_key = v8::String::new(__scope, "_flushFn").unwrap();
                            if let Some(flush_val) = transform.get(__scope, flush_key.into()) {
                                if let Ok(flush) = v8::Local::<v8::Function>::try_from(flush_val) {
                                    // Get controller
                                    let ctrl_key = v8::String::new(__scope, "_controller").unwrap();
                                    if let Some(ctrl_val) = transform.get(__scope, ctrl_key.into())
                                    {
                                        if let Ok(ctrl) =
                                            v8::Local::<v8::Object>::try_from(ctrl_val)
                                        {
                                            let undefined = v8::undefined(__scope).into();
                                            let flush_result =
                                                flush.call(__scope, undefined, &[ctrl.into()]);

                                            // v0.3.338: Check if flush returned a Promise and handle async properly
                                            if let Some(result_val) = flush_result {
                                                if result_val.is_promise() {
                                                    // Flush returned a Promise - we need to wait for it
                                                    // Create a Promise that chains the flush Promise and updates state when done
                                                    let then_code = r#"
                                                (flushPromise, transformObj) => {
                                                    transformObj._transformState = 1; // Mark as closing
                                                    return new Promise((resolve, reject) => {
                                                        Promise.resolve(flushPromise).then(() => {
                                                            transformObj._transformState = 2; // Closed
                                                            resolve(undefined);
                                                        }).catch(reject);
                                                    });
                                                }
                                            "#;

                                                    let then_source =
                                                        v8::String::new(__scope, then_code)
                                                            .unwrap();

                                                    // Create a function from the code (ScriptOrigin is None)
                                                    let origin: Option<&v8::ScriptOrigin<'_>> =
                                                        None;
                                                    let script = v8::Script::compile(
                                                        __scope,
                                                        then_source,
                                                        origin,
                                                    )
                                                    .unwrap();
                                                    let script_fn = script.run(__scope).unwrap();
                                                    let then_func =
                                                        v8::Local::<v8::Function>::try_from(
                                                            script_fn,
                                                        )
                                                        .unwrap();

                                                    // Call the then function with the flush Promise and transform
                                                    let args = [result_val, transform.into()];
                                                    let chained_promise_val =
                                                        then_func.call(__scope, undefined, &args);

                                                    // Return the chained Promise - JavaScript will await this
                                                    // The event loop will process timers and microtasks correctly
                                                    if let Some(p) = chained_promise_val {
                                                        _r.set(p);
                                                        return;
                                                    }
                                                }
                                            }

                                            // Non-Promise result - just update state and create resolved Promise
                                            let state_key =
                                                v8::String::new(__scope, "_transformState")
                                                    .unwrap();
                                            let state_val: v8::Local<v8::Value> =
                                                v8::Integer::new(__scope, 2).into(); // 2 = Closed
                                            transform.set(__scope, state_key.into(), state_val);
                                        }
                                    }
                                }
                            } else {
                                // No flush callback, just mark as closed
                                let state_key =
                                    v8::String::new(__scope, "_transformState").unwrap();
                                let state_val: v8::Local<v8::Value> =
                                    v8::Integer::new(__scope, 2).into(); // 2 = Closed
                                transform.set(__scope, state_key.into(), state_val);
                            }
                        }
                    }

                    // Return a resolved Promise
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            // Setup abort() method
            let abort_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            // Create ready Promise
            let ready_promise: v8::Local<v8::PromiseResolver> =
                v8::PromiseResolver::new(_scope).unwrap();
            let ready_promise_val: v8::Local<v8::Value> = ready_promise.into();
            let undefined_val: v8::Local<v8::Value> = v8::undefined(_scope).into();
            let _ = ready_promise.resolve(_scope, undefined_val);

            // Create closed Promise
            let closed_promise: v8::Local<v8::PromiseResolver> =
                v8::PromiseResolver::new(_scope).unwrap();
            let closed_promise_val: v8::Local<v8::Value> = closed_promise.into();
            let _ = closed_promise.resolve(_scope, undefined_val);

            // Add methods and properties
            let write_key: v8::Local<v8::String> = v8::String::new(_scope, "write").unwrap();
            let close_key: v8::Local<v8::String> = v8::String::new(_scope, "close").unwrap();
            let abort_key: v8::Local<v8::String> = v8::String::new(_scope, "abort").unwrap();
            let ready_key: v8::Local<v8::String> = v8::String::new(_scope, "ready").unwrap();
            let closed_key: v8::Local<v8::String> = v8::String::new(_scope, "closed").unwrap();

            let write_func: v8::Local<v8::Function> = write_fn.get_function(_scope).unwrap();
            let close_func: v8::Local<v8::Function> = close_fn.get_function(_scope).unwrap();
            let abort_func: v8::Local<v8::Function> = abort_fn.get_function(_scope).unwrap();

            writer.set(_scope, write_key.into(), write_func.into());
            writer.set(_scope, close_key.into(), close_func.into());
            writer.set(_scope, abort_key.into(), abort_func.into());
            writer.set(_scope, ready_key.into(), ready_promise_val);
            writer.set(_scope, closed_key.into(), closed_promise_val);

            let desired_size_key: v8::Local<v8::String> =
                v8::String::new(_scope, "desiredSize").unwrap();
            let desired_size_val: v8::Local<v8::Value> = v8::Number::new(_scope, 0.0).into();
            writer.set(_scope, desired_size_key.into(), desired_size_val);

            retval.set(writer.into());
        },
    );

    // Setup getWriter on writable stream
    let writable_get_writer_key: v8::Local<v8::String> =
        v8::String::new(scope, "getWriter").unwrap();
    let writable_get_writer_func: v8::Local<v8::Function> =
        writable_get_writer_fn.get_function(scope).unwrap();
    writable_stream.set(
        scope,
        writable_get_writer_key.into(),
        writable_get_writer_func.into(),
    );

    let writable_locked_key: v8::Local<v8::String> = v8::String::new(scope, "locked").unwrap();
    let writable_locked_val: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
    writable_stream.set(scope, writable_locked_key.into(), writable_locked_val);

    // Add readable and writable properties
    let readable_key: v8::Local<v8::String> = v8::String::new(scope, "readable").unwrap();
    let writable_key: v8::Local<v8::String> = v8::String::new(scope, "writable").unwrap();

    transform_obj.set(scope, readable_key.into(), readable_stream.into());
    transform_obj.set(scope, writable_key.into(), writable_stream.into());

    retval.set(transform_obj.into());
}

// ============================================================
// TextDecoderStream Implementation
// ============================================================

/// TextDecoderStream constructor for streaming UTF-8 decoding
/// Uses TransformStream internally for AI workloads
fn text_decoder_stream_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Parse options
    let mut encoding = "utf-8".to_string();
    let mut fatal = false;
    let mut ignore_bom = false;

    if args.length() > 0 {
        let options = args.get(0);
        if !options.is_undefined() && options.is_object() {
            let options_obj = v8::Local::<v8::Object>::try_from(options).unwrap();
            let enc_key = v8::String::new(scope, "encoding").unwrap();
            if let Some(enc_val) = options_obj.get(scope, enc_key.into()) {
                if enc_val.is_string() {
                    encoding = enc_val
                        .to_string(scope)
                        .unwrap()
                        .to_rust_string_lossy(scope)
                        .to_lowercase();
                }
            }
            let fatal_key = v8::String::new(scope, "fatal").unwrap();
            if let Some(fatal_val) = options_obj.get(scope, fatal_key.into()) {
                fatal = fatal_val.is_true();
            }
            let ignore_bom_key = v8::String::new(scope, "ignoreBOM").unwrap();
            if let Some(ignore_val) = options_obj.get(scope, ignore_bom_key.into()) {
                ignore_bom = ignore_val.is_true();
            }
        }
    }

    // Only support UTF-8 for now
    if encoding != "utf-8" && encoding != "utf8" {
        let error = v8::String::new(
            scope,
            &format!("TextDecoderStream: encoding '{}' not supported", encoding),
        )
        .unwrap();
        let error_obj = v8::Exception::range_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let transform_obj: v8::Local<v8::Object> = v8::Object::new(scope);

    // Create byte buffer for decoding
    let byte_buffer = v8::Array::new(scope, 0);
    let buffer_key = v8::String::new(scope, "_byteBuffer").unwrap();
    transform_obj.set(scope, buffer_key.into(), byte_buffer.into());

    // Create read index for output
    let read_index_key = v8::String::new(scope, "_readIndex").unwrap();
    let read_index_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
    transform_obj.set(scope, read_index_key.into(), read_index_val);

    // Create output chunks queue for decoded strings
    let output_queue = v8::Array::new(scope, 0);
    let output_key = v8::String::new(scope, "_outputQueue").unwrap();
    transform_obj.set(scope, output_key.into(), output_queue.into());

    // Store ignoreBOM setting
    let ignore_bom_val: v8::Local<v8::Value> = v8::Boolean::new(scope, ignore_bom).into();
    let ignore_bom_key = v8::String::new(scope, "_ignoreBOM").unwrap();
    transform_obj.set(scope, ignore_bom_key.into(), ignore_bom_val);

    // Create readable stream with getReader
    let readable_stream: v8::Local<v8::Object> = v8::Object::new(scope);

    // Store decoder reference on readable (use non-hidden property name)
    let readable_decoder_key = v8::String::new(scope, "decoderRef").unwrap();
    readable_stream.set(scope, readable_decoder_key.into(), transform_obj.into());

    let readable_get_reader_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let reader: v8::Local<v8::Object> = v8::Object::new(_scope);

            // Store decoder on reader
            let this_obj = args.this();
            let decoder_key = v8::String::new(_scope, "decoderRef").unwrap();
            if let Some(decoder_val) = this_obj.get(_scope, decoder_key.into()) {
                let reader_decoder_key = v8::String::new(_scope, "decoderRef").unwrap();
                reader.set(_scope, reader_decoder_key.into(), decoder_val);
            }

            let read_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();
                    let result: v8::Local<v8::Object> = v8::Object::new(__scope);
                    let done_key: v8::Local<v8::String> = v8::String::new(__scope, "done").unwrap();
                    let value_key: v8::Local<v8::String> =
                        v8::String::new(__scope, "value").unwrap();

                    // Get decoder from reader
                    let reader_decoder_key = v8::String::new(__scope, "decoderRef").unwrap();
                    if let Some(decoder_val) = _a.this().get(__scope, reader_decoder_key.into()) {
                        if let Ok(decoder) = v8::Local::<v8::Object>::try_from(decoder_val) {
                            let output_queue_key =
                                v8::String::new(__scope, "_outputQueue").unwrap();
                            let read_index_key = v8::String::new(__scope, "_readIndex").unwrap();

                            if let Some(output_val) = decoder.get(__scope, output_queue_key.into())
                            {
                                if let Ok(queue) = v8::Local::<v8::Array>::try_from(output_val) {
                                    if let Some(index_val) =
                                        decoder.get(__scope, read_index_key.into())
                                    {
                                        let index =
                                            index_val.integer_value(__scope).unwrap_or(0) as u32;
                                        let length = queue.length();

                                        if index < length {
                                            let chunk_val =
                                                queue.get_index(__scope, index).unwrap();
                                            let done_val: v8::Local<v8::Value> =
                                                v8::Boolean::new(__scope, false).into();
                                            result.set(__scope, done_key.into(), done_val);
                                            result.set(__scope, value_key.into(), chunk_val);

                                            let next_index: v8::Local<v8::Value> =
                                                v8::Integer::new(__scope, (index as i32) + 1)
                                                    .into();
                                            decoder.set(__scope, read_index_key.into(), next_index);
                                            let _ = promise.resolve(__scope, result.into());
                                            _r.set(promise.into());
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let done_val: v8::Local<v8::Value> = v8::Boolean::new(__scope, true).into();
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    result.set(__scope, done_key.into(), done_val);
                    result.set(__scope, value_key.into(), undefined_val);
                    let _ = promise.resolve(__scope, result.into());
                    _r.set(promise.into());
                },
            );

            let release_fn = v8::FunctionTemplate::new(
                _scope,
                |_scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 _r: v8::ReturnValue| {},
            );

            let closed_promise: v8::Local<v8::PromiseResolver> =
                v8::PromiseResolver::new(_scope).unwrap();
            let closed_promise_val: v8::Local<v8::Value> = closed_promise.into();
            let undefined_val: v8::Local<v8::Value> = v8::undefined(_scope).into();
            let _ = closed_promise.resolve(_scope, undefined_val);

            let read_key: v8::Local<v8::String> = v8::String::new(_scope, "read").unwrap();
            let release_key: v8::Local<v8::String> =
                v8::String::new(_scope, "releaseLock").unwrap();
            let closed_key: v8::Local<v8::String> = v8::String::new(_scope, "closed").unwrap();

            let read_func: v8::Local<v8::Function> = read_fn.get_function(_scope).unwrap();
            let release_func: v8::Local<v8::Function> = release_fn.get_function(_scope).unwrap();

            reader.set(_scope, read_key.into(), read_func.into());
            reader.set(_scope, release_key.into(), release_func.into());
            reader.set(_scope, closed_key.into(), closed_promise_val);

            retval.set(reader.into());
        },
    );

    let readable_get_reader_key: v8::Local<v8::String> =
        v8::String::new(scope, "getReader").unwrap();
    let readable_get_reader_func: v8::Local<v8::Function> =
        readable_get_reader_fn.get_function(scope).unwrap();
    readable_stream.set(
        scope,
        readable_get_reader_key.into(),
        readable_get_reader_func.into(),
    );

    let readable_locked_key: v8::Local<v8::String> = v8::String::new(scope, "locked").unwrap();
    let readable_locked_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
    readable_stream.set(scope, readable_locked_key.into(), readable_locked_val);

    // Create writable stream
    let writable_stream: v8::Local<v8::Object> = v8::Object::new(scope);

    // Store decoder reference on writable (use non-hidden property name)
    let writable_decoder_key = v8::String::new(scope, "decoderRef").unwrap();
    writable_stream.set(scope, writable_decoder_key.into(), transform_obj.into());

    let writable_get_writer_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let writer: v8::Local<v8::Object> = v8::Object::new(_scope);

            // Store decoder on writer
            let this_obj = args.this();
            let decoder_key = v8::String::new(_scope, "decoderRef").unwrap();
            if let Some(decoder_val) = this_obj.get(_scope, decoder_key.into()) {
                let writer_decoder_key = v8::String::new(_scope, "decoderRef").unwrap();
                writer.set(_scope, writer_decoder_key.into(), decoder_val);
            }

            let write_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();

                    // Get chunk to write
                    let chunk = a.get(0);
                    let mut bytes: Vec<u8> = Vec::new();

                    if chunk.is_uint8_array() {
                        let array = v8::Local::<v8::Uint8Array>::try_from(chunk).unwrap();
                        let len = array.byte_length();
                        bytes.resize(len, 0);
                        array.copy_contents(&mut bytes);
                    } else if chunk.is_string() {
                        let chunk_str = chunk
                            .to_string(__scope)
                            .unwrap()
                            .to_rust_string_lossy(__scope);
                        bytes.extend_from_slice(chunk_str.as_bytes());
                    }

                    // Get decoder from writer
                    let writer_decoder_key = v8::String::new(__scope, "decoderRef").unwrap();
                    if let Some(decoder_val) = a.this().get(__scope, writer_decoder_key.into()) {
                        if let Ok(decoder) = v8::Local::<v8::Object>::try_from(decoder_val) {
                            let buffer_key = v8::String::new(__scope, "_byteBuffer").unwrap();
                            let output_key = v8::String::new(__scope, "_outputQueue").unwrap();
                            let ignore_bom_key = v8::String::new(__scope, "_ignoreBOM").unwrap();

                            if let Some(buffer_val) = decoder.get(__scope, buffer_key.into()) {
                                if let Ok(buffer) = v8::Local::<v8::Array>::try_from(buffer_val) {
                                    let buffer_len = buffer.length();
                                    for (i, byte) in bytes.iter().enumerate() {
                                        let byte_val: v8::Local<v8::Value> =
                                            v8::Integer::new(__scope, *byte as i32).into();
                                        buffer.set_index(
                                            __scope,
                                            buffer_len + (i as u32),
                                            byte_val,
                                        );
                                    }

                                    if let Some(output_val) =
                                        decoder.get(__scope, output_key.into())
                                    {
                                        if let Ok(output) =
                                            v8::Local::<v8::Array>::try_from(output_val)
                                        {
                                            let ignore_bom = if let Some(ib_val) =
                                                decoder.get(__scope, ignore_bom_key.into())
                                            {
                                                ib_val.is_true()
                                            } else {
                                                false
                                            };

                                            let buffer_len = buffer.length();

                                            // Collect all bytes from buffer - using explicit loop to avoid E0524
                                            let mut all_bytes: Vec<u8> = Vec::new();
                                            for i in 0..buffer_len {
                                                if let Some(byte_val) = buffer.get_index(__scope, i)
                                                {
                                                    if let Some(byte) =
                                                        byte_val.integer_value(__scope)
                                                    {
                                                        all_bytes.push(byte as u8);
                                                    }
                                                }
                                            }

                                            let decoded = if ignore_bom {
                                                let start =
                                                    if all_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                                                        3
                                                    } else {
                                                        0
                                                    };
                                                String::from_utf8_lossy(&all_bytes[start..])
                                                    .into_owned()
                                            } else {
                                                String::from_utf8_lossy(&all_bytes).into_owned()
                                            };

                                            let output_len = output.length();
                                            let decoded_val =
                                                v8::String::new(__scope, &decoded).unwrap();
                                            output.set_index(
                                                __scope,
                                                output_len,
                                                decoded_val.into(),
                                            );

                                            let empty_buffer = v8::Array::new(__scope, 0);
                                            decoder.set(
                                                __scope,
                                                buffer_key.into(),
                                                empty_buffer.into(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            let close_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            let abort_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            let ready_promise: v8::Local<v8::PromiseResolver> =
                v8::PromiseResolver::new(_scope).unwrap();
            let ready_promise_val: v8::Local<v8::Value> = ready_promise.into();
            let undefined_val: v8::Local<v8::Value> = v8::undefined(_scope).into();
            let _ = ready_promise.resolve(_scope, undefined_val);

            let closed_promise: v8::Local<v8::PromiseResolver> =
                v8::PromiseResolver::new(_scope).unwrap();
            let closed_promise_val: v8::Local<v8::Value> = closed_promise.into();
            let _ = closed_promise.resolve(_scope, undefined_val);

            let write_key: v8::Local<v8::String> = v8::String::new(_scope, "write").unwrap();
            let close_key: v8::Local<v8::String> = v8::String::new(_scope, "close").unwrap();
            let abort_key: v8::Local<v8::String> = v8::String::new(_scope, "abort").unwrap();
            let ready_key: v8::Local<v8::String> = v8::String::new(_scope, "ready").unwrap();
            let closed_key: v8::Local<v8::String> = v8::String::new(_scope, "closed").unwrap();

            let write_func: v8::Local<v8::Function> = write_fn.get_function(_scope).unwrap();
            let close_func: v8::Local<v8::Function> = close_fn.get_function(_scope).unwrap();
            let abort_func: v8::Local<v8::Function> = abort_fn.get_function(_scope).unwrap();

            writer.set(_scope, write_key.into(), write_func.into());
            writer.set(_scope, close_key.into(), close_func.into());
            writer.set(_scope, abort_key.into(), abort_func.into());
            writer.set(_scope, ready_key.into(), ready_promise_val);
            writer.set(_scope, closed_key.into(), closed_promise_val);

            let desired_size_key: v8::Local<v8::String> =
                v8::String::new(_scope, "desiredSize").unwrap();
            let desired_size_val: v8::Local<v8::Value> = v8::Number::new(_scope, 0.0).into();
            writer.set(_scope, desired_size_key.into(), desired_size_val);

            retval.set(writer.into());
        },
    );

    let writable_get_writer_key: v8::Local<v8::String> =
        v8::String::new(scope, "getWriter").unwrap();
    let writable_get_writer_func: v8::Local<v8::Function> =
        writable_get_writer_fn.get_function(scope).unwrap();
    writable_stream.set(
        scope,
        writable_get_writer_key.into(),
        writable_get_writer_func.into(),
    );

    let writable_locked_key: v8::Local<v8::String> = v8::String::new(scope, "locked").unwrap();
    let writable_locked_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
    writable_stream.set(scope, writable_locked_key.into(), writable_locked_val);

    // Add properties
    let encoding_value: v8::Local<v8::Value> = v8::String::new(scope, "utf-8").unwrap().into();
    let fatal_value: v8::Local<v8::Value> = v8::Boolean::new(scope, fatal).into();
    let ignore_bom_value: v8::Local<v8::Value> = v8::Boolean::new(scope, ignore_bom).into();

    let readable_key: v8::Local<v8::String> = v8::String::new(scope, "readable").unwrap();
    let writable_key: v8::Local<v8::String> = v8::String::new(scope, "writable").unwrap();
    let encoding_key: v8::Local<v8::String> = v8::String::new(scope, "encoding").unwrap();
    let fatal_key: v8::Local<v8::String> = v8::String::new(scope, "fatal").unwrap();
    let ignore_bom_key: v8::Local<v8::String> = v8::String::new(scope, "ignoreBOM").unwrap();

    transform_obj.set(scope, readable_key.into(), readable_stream.into());
    transform_obj.set(scope, writable_key.into(), writable_stream.into());
    transform_obj.set(scope, encoding_key.into(), encoding_value);
    transform_obj.set(scope, fatal_key.into(), fatal_value);
    transform_obj.set(scope, ignore_bom_key.into(), ignore_bom_value);

    retval.set(transform_obj.into());
}

// ============================================================
// TextEncoderStream Implementation
// ============================================================

/// TextEncoderStream constructor for streaming UTF-8 encoding
/// Uses TransformStream internally for AI workloads
fn text_encoder_stream_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Parse options
    let mut encoding = "utf-8".to_string();

    if args.length() > 0 {
        let options = args.get(0);
        if !options.is_undefined() && options.is_object() {
            let options_obj = v8::Local::<v8::Object>::try_from(options).unwrap();
            let enc_key = v8::String::new(scope, "encoding").unwrap();
            if let Some(enc_val) = options_obj.get(scope, enc_key.into()) {
                if enc_val.is_string() {
                    encoding = enc_val
                        .to_string(scope)
                        .unwrap()
                        .to_rust_string_lossy(scope)
                        .to_lowercase();
                }
            }
        }
    }

    // Only support UTF-8 for now
    if encoding != "utf-8" && encoding != "utf8" {
        let error = v8::String::new(
            scope,
            &format!("TextEncoderStream: encoding '{}' not supported", encoding),
        )
        .unwrap();
        let error_obj = v8::Exception::range_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    let encoder_obj: v8::Local<v8::Object> = v8::Object::new(scope);

    // Create output chunks queue for encoded bytes
    let output_queue = v8::Array::new(scope, 0);
    let output_key = v8::String::new(scope, "_outputQueue").unwrap();
    encoder_obj.set(scope, output_key.into(), output_queue.into());

    // Create read index for output
    let read_index_key = v8::String::new(scope, "_readIndex").unwrap();
    let read_index_val: v8::Local<v8::Value> = v8::Integer::new(scope, 0).into();
    encoder_obj.set(scope, read_index_key.into(), read_index_val);

    // Create closed flag to track when writer is closed
    let closed_key = v8::String::new(scope, "_closed").unwrap();
    let closed_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
    encoder_obj.set(scope, closed_key.into(), closed_val);

    // Create readable stream with getReader
    let readable_stream: v8::Local<v8::Object> = v8::Object::new(scope);

    // Store encoder reference on readable
    let readable_encoder_key = v8::String::new(scope, "encoderRef").unwrap();
    readable_stream.set(scope, readable_encoder_key.into(), encoder_obj.into());

    let readable_get_reader_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let reader: v8::Local<v8::Object> = v8::Object::new(_scope);

            // Store encoder on reader
            let this_obj = args.this();
            let encoder_key = v8::String::new(_scope, "encoderRef").unwrap();
            if let Some(encoder_val) = this_obj.get(_scope, encoder_key.into()) {
                let reader_encoder_key = v8::String::new(_scope, "encoderRef").unwrap();
                reader.set(_scope, reader_encoder_key.into(), encoder_val);
            }

            let read_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();
                    let result: v8::Local<v8::Object> = v8::Object::new(__scope);
                    let done_key: v8::Local<v8::String> = v8::String::new(__scope, "done").unwrap();
                    let value_key: v8::Local<v8::String> =
                        v8::String::new(__scope, "value").unwrap();

                    // Get encoder from reader
                    let reader_encoder_key = v8::String::new(__scope, "encoderRef").unwrap();
                    if let Some(encoder_val) = _a.this().get(__scope, reader_encoder_key.into()) {
                        if let Ok(encoder) = v8::Local::<v8::Object>::try_from(encoder_val) {
                            let output_queue_key =
                                v8::String::new(__scope, "_outputQueue").unwrap();
                            let read_index_key = v8::String::new(__scope, "_readIndex").unwrap();
                            let closed_key = v8::String::new(__scope, "_closed").unwrap();

                            // Check if stream is closed and has no data
                            let is_closed =
                                if let Some(closed_val) = encoder.get(__scope, closed_key.into()) {
                                    closed_val.is_true()
                                } else {
                                    false
                                };

                            if let Some(output_val) = encoder.get(__scope, output_queue_key.into())
                            {
                                if let Ok(queue) = v8::Local::<v8::Array>::try_from(output_val) {
                                    if let Some(index_val) =
                                        encoder.get(__scope, read_index_key.into())
                                    {
                                        let index =
                                            index_val.integer_value(__scope).unwrap_or(0) as u32;
                                        let length = queue.length();

                                        if index < length {
                                            // Collect bytes from queue
                                            let byte_count = length - index;
                                            let mut bytes: Vec<u8> =
                                                Vec::with_capacity(byte_count as usize);
                                            for i in index..length {
                                                if let Some(byte_val) = queue.get_index(__scope, i)
                                                {
                                                    if let Some(byte) =
                                                        byte_val.integer_value(__scope)
                                                    {
                                                        bytes.push(byte as u8);
                                                    }
                                                }
                                            }

                                            // Create ArrayBuffer and Uint8Array view
                                            let buffer = v8::ArrayBuffer::new(__scope, bytes.len());
                                            let buffer_store = buffer.get_backing_store();
                                            let target_ptr = buffer_store
                                                .data()
                                                .map(|p| p.as_ptr() as *mut u8)
                                                .unwrap_or(std::ptr::null_mut());
                                            unsafe {
                                                std::ptr::copy_nonoverlapping(
                                                    bytes.as_ptr(),
                                                    target_ptr,
                                                    bytes.len(),
                                                );
                                            }
                                            let uint8_array = v8::Uint8Array::new(
                                                __scope,
                                                buffer,
                                                0,
                                                bytes.len(),
                                            );
                                            if let Some(uint8) = uint8_array {
                                                let done_val: v8::Local<v8::Value> =
                                                    v8::Boolean::new(__scope, false).into();
                                                result.set(__scope, done_key.into(), done_val);
                                                result.set(__scope, value_key.into(), uint8.into());
                                                let next_index: v8::Local<v8::Value> =
                                                    v8::Integer::new(__scope, length as i32).into();
                                                encoder.set(
                                                    __scope,
                                                    read_index_key.into(),
                                                    next_index,
                                                );
                                                let _ = promise.resolve(__scope, result.into());
                                                _r.set(promise.into());
                                                return;
                                            }
                                        } else if is_closed {
                                            // Stream is closed and queue is empty - return empty Uint8Array
                                            let empty_buffer = v8::ArrayBuffer::new(__scope, 0);
                                            let empty_uint8 =
                                                v8::Uint8Array::new(__scope, empty_buffer, 0, 0)
                                                    .unwrap();
                                            let done_val: v8::Local<v8::Value> =
                                                v8::Boolean::new(__scope, false).into();
                                            result.set(__scope, done_key.into(), done_val);
                                            result.set(
                                                __scope,
                                                value_key.into(),
                                                empty_uint8.into(),
                                            );
                                            let _ = promise.resolve(__scope, result.into());
                                            _r.set(promise.into());
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // No data available yet, resolve with done=true
                    let done_val: v8::Local<v8::Value> = v8::Boolean::new(__scope, true).into();
                    result.set(__scope, done_key.into(), done_val);
                    let _ = promise.resolve(__scope, result.into());
                    _r.set(promise.into());
                },
            );

            // releaseLock function - no-op for TextEncoderStream reader
            let release_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    // releaseLock is a no-op for TextEncoderStream
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    _r.set(undefined_val);
                },
            );

            let read_key: v8::Local<v8::String> = v8::String::new(_scope, "read").unwrap();
            let read_fn_func = read_fn.get_function(_scope).unwrap();
            reader.set(_scope, read_key.into(), read_fn_func.into());

            let release_fn_func = release_fn.get_function(_scope).unwrap();
            let release_key: v8::Local<v8::String> =
                v8::String::new(_scope, "releaseLock").unwrap();
            reader.set(_scope, release_key.into(), release_fn_func.into());

            retval.set(reader.into());
        },
    );

    let readable_get_reader_key: v8::Local<v8::String> =
        v8::String::new(scope, "getReader").unwrap();
    let readable_get_reader_func: v8::Local<v8::Function> =
        readable_get_reader_fn.get_function(scope).unwrap();
    readable_stream.set(
        scope,
        readable_get_reader_key.into(),
        readable_get_reader_func.into(),
    );

    // Create writable stream
    let writable_stream: v8::Local<v8::Object> = v8::Object::new(scope);

    // Store encoder reference on writable
    let writable_encoder_key = v8::String::new(scope, "encoderRef").unwrap();
    writable_stream.set(scope, writable_encoder_key.into(), encoder_obj.into());

    let writable_get_writer_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let writer: v8::Local<v8::Object> = v8::Object::new(_scope);

            // Store encoder on writer
            let this_obj = args.this();
            let encoder_key = v8::String::new(_scope, "encoderRef").unwrap();
            if let Some(encoder_val) = this_obj.get(_scope, encoder_key.into()) {
                let writer_encoder_key = v8::String::new(_scope, "encoderRef").unwrap();
                writer.set(_scope, writer_encoder_key.into(), encoder_val);
            }

            let write_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();

                    // Get encoder from writer
                    let writer_encoder_key = v8::String::new(__scope, "encoderRef").unwrap();
                    if let Some(encoder_val) = a.this().get(__scope, writer_encoder_key.into()) {
                        if let Ok(encoder) = v8::Local::<v8::Object>::try_from(encoder_val) {
                            if a.length() > 0 {
                                let chunk = a.get(0);

                                // Convert string to UTF-8 bytes
                                if chunk.is_string() {
                                    let str_val = chunk.to_string(__scope).unwrap();
                                    let rust_str = str_val.to_rust_string_lossy(__scope);

                                    // Encode to UTF-8 bytes
                                    let utf8_bytes: Vec<u8> = rust_str.into_bytes();

                                    // Add bytes to output queue as individual integers (following TextDecoderStream pattern)
                                    let output_queue_key =
                                        v8::String::new(__scope, "_outputQueue").unwrap();
                                    if let Some(queue_val) =
                                        encoder.get(__scope, output_queue_key.into())
                                    {
                                        if let Ok(queue) =
                                            v8::Local::<v8::Array>::try_from(queue_val)
                                        {
                                            let queue_len = queue.length();
                                            for (i, &byte) in utf8_bytes.iter().enumerate() {
                                                let byte_val: v8::Local<v8::Value> =
                                                    v8::Integer::new(__scope, byte as i32).into();
                                                queue.set_index(
                                                    __scope,
                                                    queue_len + (i as u32),
                                                    byte_val,
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                            let _ = promise.resolve(__scope, undefined_val);
                            _r.set(promise.into());
                            return;
                        }
                    }

                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            // TextEncoderStream close function - sets closed flag
            let close_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();

                    // Set closed flag on encoder
                    let writer_encoder_key = v8::String::new(__scope, "encoderRef").unwrap();
                    if let Some(encoder_val) = a.this().get(__scope, writer_encoder_key.into()) {
                        if let Ok(encoder) = v8::Local::<v8::Object>::try_from(encoder_val) {
                            let closed_key = v8::String::new(__scope, "_closed").unwrap();
                            let true_val: v8::Local<v8::Value> =
                                v8::Boolean::new(__scope, true).into();
                            encoder.set(__scope, closed_key.into(), true_val);
                        }
                    }

                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            let abort_fn = v8::FunctionTemplate::new(
                _scope,
                |__scope: &mut v8::PinScope,
                 _a: v8::FunctionCallbackArguments,
                 mut _r: v8::ReturnValue| {
                    let promise: v8::Local<v8::PromiseResolver> =
                        v8::PromiseResolver::new(__scope).unwrap();
                    let undefined_val: v8::Local<v8::Value> = v8::undefined(__scope).into();
                    let _ = promise.resolve(__scope, undefined_val);
                    _r.set(promise.into());
                },
            );

            let write_fn_func = write_fn.get_function(_scope).unwrap();
            let write_key: v8::Local<v8::String> = v8::String::new(_scope, "write").unwrap();
            writer.set(_scope, write_key.into(), write_fn_func.into());

            let close_fn_func = close_fn.get_function(_scope).unwrap();
            let close_key: v8::Local<v8::String> = v8::String::new(_scope, "close").unwrap();
            writer.set(_scope, close_key.into(), close_fn_func.into());

            let abort_fn_func = abort_fn.get_function(_scope).unwrap();
            let abort_key: v8::Local<v8::String> = v8::String::new(_scope, "abort").unwrap();
            writer.set(_scope, abort_key.into(), abort_fn_func.into());

            retval.set(writer.into());
        },
    );

    let writable_get_writer_key: v8::Local<v8::String> =
        v8::String::new(scope, "getWriter").unwrap();
    let writable_get_writer_func: v8::Local<v8::Function> =
        writable_get_writer_fn.get_function(scope).unwrap();
    writable_stream.set(
        scope,
        writable_get_writer_key.into(),
        writable_get_writer_func.into(),
    );

    let writable_locked_key: v8::Local<v8::String> = v8::String::new(scope, "locked").unwrap();
    let writable_locked_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
    writable_stream.set(scope, writable_locked_key.into(), writable_locked_val);

    // Add properties
    let encoding_value: v8::Local<v8::Value> = v8::String::new(scope, "utf-8").unwrap().into();

    let readable_key: v8::Local<v8::String> = v8::String::new(scope, "readable").unwrap();
    let writable_key: v8::Local<v8::String> = v8::String::new(scope, "writable").unwrap();
    let encoding_key: v8::Local<v8::String> = v8::String::new(scope, "encoding").unwrap();

    encoder_obj.set(scope, readable_key.into(), readable_stream.into());
    encoder_obj.set(scope, writable_key.into(), writable_stream.into());
    encoder_obj.set(scope, encoding_key.into(), encoding_value);

    retval.set(encoder_obj.into());
}

// ============================================================
// Setup Functions
// ============================================================

/// Setup ReadableStream constructor in V8 context
fn setup_readable_stream(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: v8::Local<v8::Context>,
) {
    let global: v8::Local<v8::Object> = context.global(scope);
    let readable_stream_template: v8::Local<v8::FunctionTemplate> =
        v8::FunctionTemplate::new(scope, readable_stream_constructor);
    readable_stream_template.set_class_name(v8::String::new(scope, "ReadableStream").unwrap());
    let readable_stream_constructor: v8::Local<v8::Function> =
        readable_stream_template.get_function(scope).unwrap();
    let readable_stream_key: v8::Local<v8::String> =
        v8::String::new(scope, "ReadableStream").unwrap();
    global.set(
        scope,
        readable_stream_key.into(),
        readable_stream_constructor.into(),
    );
}

/// Setup WritableStream constructor in V8 context
fn setup_writable_stream(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: v8::Local<v8::Context>,
) {
    let global: v8::Local<v8::Object> = context.global(scope);
    let writable_stream_template: v8::Local<v8::FunctionTemplate> =
        v8::FunctionTemplate::new(scope, writable_stream_constructor);
    let writable_stream_constructor: v8::Local<v8::Function> =
        writable_stream_template.get_function(scope).unwrap();
    let writable_stream_key: v8::Local<v8::String> =
        v8::String::new(scope, "WritableStream").unwrap();
    global.set(
        scope,
        writable_stream_key.into(),
        writable_stream_constructor.into(),
    );
}

/// Setup TransformStream constructor in V8 context
fn setup_transform_stream(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: v8::Local<v8::Context>,
) {
    let global: v8::Local<v8::Object> = context.global(scope);
    let transform_stream_template: v8::Local<v8::FunctionTemplate> =
        v8::FunctionTemplate::new(scope, transform_stream_constructor);
    let transform_stream_constructor: v8::Local<v8::Function> =
        transform_stream_template.get_function(scope).unwrap();
    let transform_stream_key: v8::Local<v8::String> =
        v8::String::new(scope, "TransformStream").unwrap();
    global.set(
        scope,
        transform_stream_key.into(),
        transform_stream_constructor.into(),
    );
}

/// Setup TextDecoderStream constructor in V8 context
fn setup_text_decoder_stream(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: v8::Local<v8::Context>,
) {
    let global: v8::Local<v8::Object> = context.global(scope);
    let text_decoder_stream_template: v8::Local<v8::FunctionTemplate> =
        v8::FunctionTemplate::new(scope, text_decoder_stream_constructor);
    let text_decoder_stream_constructor: v8::Local<v8::Function> =
        text_decoder_stream_template.get_function(scope).unwrap();
    let text_decoder_stream_key: v8::Local<v8::String> =
        v8::String::new(scope, "TextDecoderStream").unwrap();
    global.set(
        scope,
        text_decoder_stream_key.into(),
        text_decoder_stream_constructor.into(),
    );
}

/// Setup TextEncoderStream constructor in V8 context
fn setup_text_encoder_stream(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: v8::Local<v8::Context>,
) {
    let global: v8::Local<v8::Object> = context.global(scope);
    let text_encoder_stream_template: v8::Local<v8::FunctionTemplate> =
        v8::FunctionTemplate::new(scope, text_encoder_stream_constructor);
    let text_encoder_stream_constructor: v8::Local<v8::Function> =
        text_encoder_stream_template.get_function(scope).unwrap();
    let text_encoder_stream_key: v8::Local<v8::String> =
        v8::String::new(scope, "TextEncoderStream").unwrap();
    global.set(
        scope,
        text_encoder_stream_key.into(),
        text_encoder_stream_constructor.into(),
    );
}

/// Setup all Streams APIs in V8 context
pub fn setup_streams_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    setup_readable_stream(scope, *context);
    setup_writable_stream(scope, *context);
    setup_transform_stream(scope, *context);
    setup_text_decoder_stream(scope, *context);
    setup_text_encoder_stream(scope, *context);

    // Modern Web Streams enhancements (ReadableStream.from, Symbol.asyncIterator) (v1.5.0)
    let streams_helper_js = r#"
    (function() {
        if (typeof ReadableStream === 'undefined') return;

        ReadableStream.from = function(iterable) {
            if (!iterable) throw new TypeError('ReadableStream.from requires an iterable');
            if (typeof iterable[Symbol.asyncIterator] === 'function') {
                return new ReadableStream({
                    async start(controller) {
                        try {
                            for await (const chunk of iterable) {
                                controller.enqueue(chunk);
                            }
                            controller.close();
                        } catch (err) {
                            controller.error(err);
                        }
                    }
                });
            }
            return new ReadableStream({
                start(controller) {
                    try {
                        for (const chunk of iterable) {
                            controller.enqueue(chunk);
                        }
                        controller.close();
                    } catch (err) {
                        controller.error(err);
                    }
                }
            });
        };

        if (ReadableStream.prototype) {
            ReadableStream.prototype[Symbol.asyncIterator] = async function*() {
                const reader = this.getReader();
                try {
                    while (true) {
                        const { done, value } = await reader.read();
                        if (done) break;
                        yield value;
                    }
                } finally {
                    if (reader && typeof reader.releaseLock === 'function') {
                        reader.releaseLock();
                    }
                }
            };
        }

        if (typeof TransformStream !== 'undefined') {
            const OrigTS = TransformStream;
            globalThis.TransformStream = function(transformer) {
                const ts = new OrigTS(transformer);
                if (ts.readable && typeof ReadableStream !== 'undefined' && ReadableStream.prototype) {
                    Object.setPrototypeOf(ts.readable, ReadableStream.prototype);
                }
                if (ts.writable && typeof WritableStream !== 'undefined' && WritableStream.prototype) {
                    Object.setPrototypeOf(ts.writable, WritableStream.prototype);
                }
                return ts;
            };
            globalThis.TransformStream.prototype = OrigTS.prototype;
        }

        // WinterTC ECMA-429 Queuing Strategies
        if (typeof globalThis.ByteLengthQueuingStrategy === 'undefined') {
            globalThis.ByteLengthQueuingStrategy = class ByteLengthQueuingStrategy {
                constructor(options = {}) {
                    if (typeof options.highWaterMark === 'undefined') {
                        throw new TypeError("ByteLengthQueuingStrategy: highWaterMark is required");
                    }
                    this.highWaterMark = options.highWaterMark;
                }
                size(chunk) {
                    return (chunk && chunk.byteLength !== undefined) ? chunk.byteLength : 0;
                }
            };
        }

        if (typeof globalThis.CountQueuingStrategy === 'undefined') {
            globalThis.CountQueuingStrategy = class CountQueuingStrategy {
                constructor(options = {}) {
                    if (typeof options.highWaterMark === 'undefined') {
                        throw new TypeError("CountQueuingStrategy: highWaterMark is required");
                    }
                    this.highWaterMark = options.highWaterMark;
                }
                size() {
                    return 1;
                }
            };
        }
    })();
    "#;
    if let Some(code) = v8::String::new(scope, streams_helper_js) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
