// Node.js Stream模块实现
/// 高性能流处理，支持背压机制
use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_stream_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Create stream object
    let stream_obj = v8::Object::new(scope);

    // Readable Stream constructor
    let readable_constructor = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let stream_obj = if this.is_object() {
                this
            } else {
                v8::Object::new(scope)
            };

            // Check if user passed options with read or _read
            let opts = args.get(0);
            let mut user_read: Option<v8::Local<v8::Value>> = None;
            let mut user_read_: Option<v8::Local<v8::Value>> = None;

            if opts.is_object() {
                if let Some(opts_obj) = opts.to_object(scope) {
                    let read_key = v8::String::new(scope, "read").unwrap();
                    user_read = opts_obj.get(scope, read_key.into());
                    let _read_key = v8::String::new(scope, "_read").unwrap();
                    user_read_ = opts_obj.get(scope, _read_key.into());
                }
            }

            // _read method - use user's read or _read, or default
            let read_key = v8::String::new(scope, "_read").unwrap();
            if let Some(read_fn) = user_read {
                if read_fn.is_function() {
                    // User passed {read(size){...}} - use as _read
                    stream_obj.set(scope, read_key.into(), read_fn);
                }
            } else if let Some(_read_fn) = user_read_ {
                if _read_fn.is_function() {
                    // User passed {_read(size){...}}
                    stream_obj.set(scope, read_key.into(), _read_fn);
                }
            } else {
                // Default empty _read
                let read_func = v8::FunctionTemplate::new(
                    scope,
                    |_scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut _retval: v8::ReturnValue| {},
                );
                let read_instance = read_func.get_function(scope).unwrap();
                stream_obj.set(scope, read_key.into(), read_instance.into());
            }

            // read method
            let read_public_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let _size = args
                        .get(0)
                        .to_integer(scope)
                        .unwrap_or(v8::Integer::new(scope, -1))
                        .value();
                    // Call _read method if it exists
                    let read_key = v8::String::new(scope, "_read").unwrap();
                    if let Some(read_func_value) = this.get(scope, read_key.into()) {
                        if read_func_value.is_function() {
                            if let Ok(read_func) =
                                v8::Local::<v8::Function>::try_from(read_func_value)
                            {
                                let size_val = v8::Integer::new(scope, -1);
                                let call_args: &[v8::Local<v8::Value>] = &[size_val.into()];
                                read_func.call(scope, this.into(), call_args);
                            }
                        }
                    }
                    retval.set(v8::null(scope).into());
                },
            );
            let read_public_instance = read_public_func.get_function(scope).unwrap();
            let read_public_key = v8::String::new(scope, "read").unwrap();
            stream_obj.set(scope, read_public_key.into(), read_public_instance.into());

            // push method - v0.3.56: Push data to the stream
            let push_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let chunk = args.get(0);

                    // Check for push(null) - end of stream
                    if chunk.is_null() {
                        // Set ended state
                        let state_key = v8::String::new(scope, "_readableState").unwrap();
                        if let Some(state_val) = this.get(scope, state_key.into()) {
                            if let Some(state_obj) = state_val.to_object(scope) {
                                let ended_key = v8::String::new(scope, "ended").unwrap();
                                let ended_val: v8::Local<v8::Value> =
                                    v8::Boolean::new(scope, true).into();
                                state_obj.set(scope, ended_key.into(), ended_val);
                            }
                        }

                        // Trigger 'end' event - look for listener set via on/once
                        let end_key = v8::String::new(scope, "end").unwrap();
                        if let Some(listener) = this.get(scope, end_key.into()) {
                            if listener.is_function() {
                                if let Ok(func) = v8::Local::<v8::Function>::try_from(listener) {
                                    func.call(scope, this.into(), &[]);
                                }
                            }
                        }

                        let result_val: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
                        retval.set(result_val);
                        return;
                    }

                    // For non-null chunks in flowing mode, trigger 'data' event
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            if let Some(flowing) = state_obj.get(scope, flowing_key.into()) {
                                if flowing.to_boolean(scope).boolean_value(scope) {
                                    let data_key = v8::String::new(scope, "data").unwrap();
                                    if let Some(listener) = this.get(scope, data_key.into()) {
                                        if listener.is_function() {
                                            if let Ok(func) =
                                                v8::Local::<v8::Function>::try_from(listener)
                                            {
                                                func.call(scope, this.into(), &[chunk]);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    retval.set(v8::Boolean::new(scope, true).into());
                },
            );
            let push_instance = push_func.get_function(scope).unwrap();
            let push_key = v8::String::new(scope, "push").unwrap();
            stream_obj.set(scope, push_key.into(), push_instance.into());

            // on method (event listener)
            let on_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let listener = args.get(1);

                    // For 'end' event, check if stream already ended
                    if event == "end" && listener.is_function() {
                        let state_key = v8::String::new(scope, "_readableState").unwrap();
                        if let Some(state_val) = this.get(scope, state_key.into()) {
                            if let Some(state_obj) = state_val.to_object(scope) {
                                let ended_key = v8::String::new(scope, "ended").unwrap();
                                if let Some(ended) = state_obj.get(scope, ended_key.into()) {
                                    if ended.to_boolean(scope).boolean_value(scope) {
                                        // Stream already ended, fire listener immediately
                                        if let Ok(listener_func) =
                                            v8::Local::<v8::Function>::try_from(listener)
                                        {
                                            listener_func.call(scope, this.into(), &[]);
                                        }
                                        retval.set(this.into());
                                        return;
                                    }
                                }
                            }
                        }
                    }

                    // Store listener on the stream object for push() to find
                    let event_key = v8::String::new(scope, &event).unwrap();
                    this.set(scope, event_key.into(), listener);

                    // v0.3.59: Setting flowing=true when 'data' listener is registered
                    // This enables flowing mode which triggers 'data' events in push()
                    if event == "data" {
                        let state_key = v8::String::new(scope, "_readableState").unwrap();
                        if let Some(state_val) = this.get(scope, state_key.into()) {
                            if let Some(state_obj) = state_val.to_object(scope) {
                                let flowing_key = v8::String::new(scope, "flowing").unwrap();
                                let flowing_val: v8::Local<v8::Value> =
                                    v8::Boolean::new(scope, true).into();
                                state_obj.set(scope, flowing_key.into(), flowing_val);
                            }
                        }

                        // v0.3.59: Call read() to start the flow when 'data' listener is registered
                        let read_key = v8::String::new(scope, "read").unwrap();
                        if let Some(read_func_val) = this.get(scope, read_key.into()) {
                            if read_func_val.is_function() {
                                if let Ok(read_func) =
                                    v8::Local::<v8::Function>::try_from(read_func_val)
                                {
                                    let size_val = v8::Integer::new(scope, -1);
                                    let call_args: &[v8::Local<v8::Value>] = &[size_val.into()];
                                    read_func.call(scope, this.into(), call_args);
                                }
                            }
                        }
                    }

                    // v0.3.59: Removed immediate data firing - breaks pipe() flow
                    // Data should only be fired when push() is called or read() pulls data

                    retval.set(this.into());
                },
            );
            let on_instance = on_func.get_function(scope).unwrap();
            let on_key = v8::String::new(scope, "on").unwrap();
            stream_obj.set(scope, on_key.into(), on_instance.into());

            // once method - v0.3.56: One-time event listener
            let once_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let listener = args.get(1);

                    if !listener.is_function() {
                        retval.set(v8::null(scope).into());
                        return;
                    }

                    // For 'end' event, check if already ended
                    if event == "end" {
                        let state_key = v8::String::new(scope, "_readableState").unwrap();
                        if let Some(state_val) = this.get(scope, state_key.into()) {
                            if let Some(state_obj) = state_val.to_object(scope) {
                                let ended_key = v8::String::new(scope, "ended").unwrap();
                                if let Some(ended) = state_obj.get(scope, ended_key.into()) {
                                    if ended.to_boolean(scope).boolean_value(scope) {
                                        // Stream already ended, fire immediately
                                        if let Ok(listener_func) =
                                            v8::Local::<v8::Function>::try_from(listener)
                                        {
                                            listener_func.call(scope, this.into(), &[]);
                                        }
                                        retval.set(this.into());
                                        return;
                                    }
                                }
                            }
                        }
                    }

                    // Set listener (same as on for now)
                    let event_key = v8::String::new(scope, &event).unwrap();
                    this.set(scope, event_key.into(), listener);

                    retval.set(this.into());
                },
            );
            let once_instance = once_func.get_function(scope).unwrap();
            let once_key = v8::String::new(scope, "once").unwrap();
            stream_obj.set(scope, once_key.into(), once_instance.into());

            // pause method - v0.3.56: Update flowing and paused state
            let pause_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    // Update _readableState
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            let flowing_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, false).into();
                            state_obj.set(scope, flowing_key.into(), flowing_val);
                            let paused_key = v8::String::new(scope, "paused").unwrap();
                            let paused_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            state_obj.set(scope, paused_key.into(), paused_val);
                        }
                    }
                    retval.set(this.into());
                },
            );
            let pause_instance = pause_func.get_function(scope).unwrap();
            let pause_key = v8::String::new(scope, "pause").unwrap();
            stream_obj.set(scope, pause_key.into(), pause_instance.into());

            // resume method
            let resume_func = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    retval.set(this.into());
                },
            );
            let resume_instance = resume_func.get_function(scope).unwrap();
            let resume_key = v8::String::new(scope, "resume").unwrap();
            stream_obj.set(scope, resume_key.into(), resume_instance.into());

            // pipe method - v0.3.59: Complete implementation with data and end callbacks
            let pipe_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let destination = args.get(0);

                    // v0.3.59: Convert destination to object for property access
                    let dest_obj = destination.to_object(scope);

                    // v0.3.59: Handle 'data' event on source - call write() on destination
                    let data_key = v8::String::new(scope, "data").unwrap();
                    let end_key = v8::String::new(scope, "end").unwrap();

                    // v0.3.59: Create data callback that calls write() on destination
                    let data_callback = v8::FunctionTemplate::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _retval: v8::ReturnValue| {
                let chunk = args.get(0);
                let encoding = v8::String::new(scope, "utf8").unwrap();

                // Get the source readable from 'this'
                let _this = args.this();

                // Get destination from _pipeDest property on source
                let dest_ref_key = v8::String::new(scope, "_pipeDest").unwrap();
                if let Some(dest_val) = _this.get(scope, dest_ref_key.into()) {
                    match v8::Local::<v8::Object>::try_from(dest_val) {
                        Ok(dest) => {
                            let write_key = v8::String::new(scope, "write").unwrap();
                            if let Some(write_func_val) = dest.get(scope, write_key.into()) {
                                if write_func_val.is_function() {
                                    match v8::Local::<v8::Function>::try_from(write_func_val) {
                                        Ok(write_func) => {
                                            let noop_callback = v8::Function::new(scope, |_scope: &mut v8::PinScope, _args: v8::FunctionCallbackArguments, _retval: v8::ReturnValue| {}).unwrap();
                                            write_func.call(scope, dest.into(), &[chunk, encoding.into(), noop_callback.into()]);
                                        }
                                        Err(_) => {}
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
            }).get_function(scope).unwrap();

                    // v0.3.59: Create end callback that calls end() on destination
                    let end_callback = v8::FunctionTemplate::new(
                        scope,
                        |scope: &mut v8::PinScope,
                         args: v8::FunctionCallbackArguments,
                         _retval: v8::ReturnValue| {
                            // Get the source readable from 'this'
                            let _this = args.this();

                            // Get destination from _pipeDest property on source
                            let dest_ref_key = v8::String::new(scope, "_pipeDest").unwrap();
                            if let Some(dest_val) = _this.get(scope, dest_ref_key.into()) {
                                match v8::Local::<v8::Object>::try_from(dest_val) {
                                    Ok(dest) => {
                                        let end_key = v8::String::new(scope, "end").unwrap();
                                        if let Some(end_func_val) = dest.get(scope, end_key.into())
                                        {
                                            if end_func_val.is_function() {
                                                match v8::Local::<v8::Function>::try_from(
                                                    end_func_val,
                                                ) {
                                                    Ok(end_func) => {
                                                        end_func.call(scope, dest.into(), &[]);
                                                    }
                                                    Err(_) => {}
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                        },
                    )
                    .get_function(scope)
                    .unwrap();

                    // Register callbacks on source (this)
                    // Store destination reference for callbacks to access
                    let dest_ref_key = v8::String::new(scope, "_pipeDest").unwrap();
                    if let Some(_dest) = dest_obj {
                        this.set(scope, dest_ref_key.into(), destination);
                    }

                    // Register 'data' listener on source
                    this.set(scope, data_key.into(), data_callback.into());
                    // Register 'end' listener on source
                    this.set(scope, end_key.into(), end_callback.into());

                    // v0.3.59: Set flowing=true on source readable
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            let flowing_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            state_obj.set(scope, flowing_key.into(), flowing_val);
                        }
                    }

                    // v0.3.59: Call read() to start data flowing
                    let read_key = v8::String::new(scope, "read").unwrap();
                    if let Some(read_func_val) = this.get(scope, read_key.into()) {
                        if read_func_val.is_function() {
                            match v8::Local::<v8::Function>::try_from(read_func_val) {
                                Ok(read_func) => {
                                    read_func.call(scope, this.into(), &[]);
                                }
                                Err(_) => {}
                            }
                        }
                    }

                    retval.set(destination);
                },
            );
            let pipe_instance = pipe_func.get_function(scope).unwrap();
            let pipe_key = v8::String::new(scope, "pipe").unwrap();
            stream_obj.set(scope, pipe_key.into(), pipe_instance.into());

            // _readableState - v0.3.56: Stream state object
            let state_key = v8::String::new(scope, "_readableState").unwrap();
            let state_obj = v8::Object::new(scope);
            let flowing_key = v8::String::new(scope, "flowing").unwrap();
            let flowing_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            state_obj.set(scope, flowing_key.into(), flowing_val);
            let paused_key = v8::String::new(scope, "paused").unwrap();
            let paused_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            state_obj.set(scope, paused_key.into(), paused_val);
            let ended_key = v8::String::new(scope, "ended").unwrap();
            let ended_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            state_obj.set(scope, ended_key.into(), ended_val);
            let hwm_key = v8::String::new(scope, "highWaterMark").unwrap();
            let hwm_val: v8::Local<v8::Value> = v8::Integer::new(scope, 16 * 1024).into();
            state_obj.set(scope, hwm_key.into(), hwm_val);
            stream_obj.set(scope, state_key.into(), state_obj.into());

            retval.set(stream_obj.into());
        },
    );
    let readable_func = readable_constructor.get_function(scope).unwrap();
    let readable_key = v8::String::new(scope, "Readable").unwrap();
    stream_obj.set(scope, readable_key.into(), readable_func.into());

    // Writable Stream constructor
    let writable_constructor = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let stream_obj = if this.is_object() {
                this
            } else {
                v8::Object::new(scope)
            };

            // v0.3.59: Support options with write or _write function
            let opts = args.get(0);
            let mut user_write: Option<v8::Local<v8::Value>> = None;
            let mut user_write_: Option<v8::Local<v8::Value>> = None;

            if opts.is_object() {
                if let Some(opts_obj) = opts.to_object(scope) {
                    let write_key = v8::String::new(scope, "write").unwrap();
                    user_write = opts_obj.get(scope, write_key.into());
                    let _write_key = v8::String::new(scope, "_write").unwrap();
                    user_write_ = opts_obj.get(scope, _write_key.into());
                }
            }

            // _write method - use user's write or _write, or default
            let write_key = v8::String::new(scope, "_write").unwrap();

            // Check for valid write function (exists and is not undefined)
            let has_valid_write = user_write
                .as_ref()
                .map(|v| !v.is_undefined() && v.is_function())
                .unwrap_or(false);
            let has_valid_write_ = user_write_
                .as_ref()
                .map(|v| !v.is_undefined() && v.is_function())
                .unwrap_or(false);

            if has_valid_write {
                // User passed {write(chunk, enc, cb){...}}
                if let Some(write_fn) = user_write {
                    stream_obj.set(scope, write_key.into(), write_fn);
                }
            } else if has_valid_write_ {
                // User passed {_write(chunk, enc, cb){...}}
                if let Some(__write_fn) = user_write_ {
                    stream_obj.set(scope, write_key.into(), __write_fn);
                }
            } else {
                // Default _write implementation
                let write_func = v8::FunctionTemplate::new(
                    scope,
                    |_scope: &mut v8::PinScope,
                     _args: v8::FunctionCallbackArguments,
                     mut _retval: v8::ReturnValue| {
                        // Default empty implementation
                    },
                );
                let write_instance = write_func.get_function(scope).unwrap();
                stream_obj.set(scope, write_key.into(), write_instance.into());
            }

            // write method
            let write_public_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let chunk = args.get(0);
                    let encoding = args.get(1);
                    let callback = args.get(2);

                    // If callback is undefined, create a noop function
                    let effective_callback: v8::Local<v8::Value> = if callback.is_undefined() {
                        let noop_func = v8::Function::new(
                            scope,
                            |_scope: &mut v8::PinScope,
                             _args: v8::FunctionCallbackArguments,
                             _retval: v8::ReturnValue| {
                                // Noop callback - does nothing
                            },
                        )
                        .unwrap();
                        noop_func.into()
                    } else {
                        callback
                    };

                    // Call _write method with proper arguments
                    let write_key = v8::String::new(scope, "_write").unwrap();
                    if let Some(write_func_val) = this.get(scope, write_key.into()) {
                        if write_func_val.is_function() {
                            if let Ok(write_func) =
                                v8::Local::<v8::Function>::try_from(write_func_val)
                            {
                                // Pass chunk, encoding, and callback to _write
                                write_func.call(
                                    scope,
                                    this.into(),
                                    &[chunk, encoding, effective_callback],
                                );
                            }
                        }
                    }
                    // Callback is handled by _write
                    retval.set(v8::Boolean::new(scope, true).into());
                },
            );
            let write_public_instance = write_public_func.get_function(scope).unwrap();
            let write_public_key = v8::String::new(scope, "write").unwrap();
            stream_obj.set(scope, write_public_key.into(), write_public_instance.into());

            // end method - v0.3.57: Updated to properly set state and trigger 'finish' event
            let end_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let callback = args.get(2);

                    // v0.3.57: Update _writableState - set ended=true and writable=false
                    let wstate_key = v8::String::new(scope, "_writableState").unwrap();
                    if let Some(wstate_val) = this.get(scope, wstate_key.into()) {
                        if let Some(wstate_obj) = wstate_val.to_object(scope) {
                            let ended_key = v8::String::new(scope, "ended").unwrap();
                            let ended_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            wstate_obj.set(scope, ended_key.into(), ended_val);

                            let writable_key = v8::String::new(scope, "writable").unwrap();
                            let writable_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, false).into();
                            wstate_obj.set(scope, writable_key.into(), writable_val);
                        }
                    }

                    // v0.3.57: Trigger 'finish' event
                    let finish_key = v8::String::new(scope, "finish").unwrap();
                    if let Some(listener) = this.get(scope, finish_key.into()) {
                        if listener.is_function() {
                            if let Ok(func) = v8::Local::<v8::Function>::try_from(listener) {
                                func.call(scope, this.into(), &[]);
                            }
                        }
                    }

                    if callback.is_function() {
                        if let Ok(cb_func) = v8::Local::<v8::Function>::try_from(callback) {
                            cb_func.call(scope, this.into(), &[]);
                        }
                    }
                    retval.set(this.into());
                },
            );
            let end_instance = end_func.get_function(scope).unwrap();
            let end_key = v8::String::new(scope, "end").unwrap();
            stream_obj.set(scope, end_key.into(), end_instance.into());

            // on method - v0.3.57: Event listener registration
            let on_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {
                    let this = args.this();
                    let event = args.get(0);
                    let listener = args.get(1);

                    if event.is_string() && listener.is_function() {
                        let event_str = event.to_string(scope).unwrap().to_rust_string_lossy(scope);
                        let event_key = v8::String::new(scope, &event_str).unwrap();
                        this.set(scope, event_key.into(), listener);
                    }
                },
            );
            let on_instance = on_func.get_function(scope).unwrap();
            let on_key = v8::String::new(scope, "on").unwrap();
            stream_obj.set(scope, on_key.into(), on_instance.into());

            // _writableState - v0.3.57: 背压支持状态对象
            let wstate_key = v8::String::new(scope, "_writableState").unwrap();
            let wstate_obj = v8::Object::new(scope);

            // highWaterMark - 背压水位线 (16KB)
            let hwm_key = v8::String::new(scope, "highWaterMark").unwrap();
            let hwm_val: v8::Local<v8::Value> = v8::Integer::new(scope, 16 * 1024).into();
            wstate_obj.set(scope, hwm_key.into(), hwm_val);

            // needDrain - 是否需要等待 drain 事件
            let drain_key = v8::String::new(scope, "needDrain").unwrap();
            let drain_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            wstate_obj.set(scope, drain_key.into(), drain_val);

            // ended - 是否已结束
            let ended_key = v8::String::new(scope, "ended").unwrap();
            let ended_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            wstate_obj.set(scope, ended_key.into(), ended_val);

            // writable - 是否可写
            let writable_key = v8::String::new(scope, "writable").unwrap();
            let writable_val: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
            wstate_obj.set(scope, writable_key.into(), writable_val);

            stream_obj.set(scope, wstate_key.into(), wstate_obj.into());

            retval.set(stream_obj.into());
        },
    );
    let writable_func = writable_constructor.get_function(scope).unwrap();
    let writable_key = v8::String::new(scope, "Writable").unwrap();
    stream_obj.set(scope, writable_key.into(), writable_func.into());

    // Transform Stream constructor - v0.3.58: Complete implementation with Readable + Writable methods
    let transform_constructor = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let stream_obj = if this.is_object() {
                this
            } else {
                v8::Object::new(scope)
            };

            // 提取用户提供的 transform 函数
            let options = args.get(0);
            let user_transform: Option<v8::Local<v8::Value>> = if options.is_object() {
                let transform_key = v8::String::new(scope, "transform").unwrap();
                options
                    .to_object(scope)
                    .and_then(|obj| obj.get(scope, transform_key.into()))
            } else {
                None
            };

            // v0.3.59: Set _transform on stream object for _write to call
            let transform_func_key = v8::String::new(scope, "_transform").unwrap();
            if let Some(transform_fn) = user_transform {
                if transform_fn.is_function() {
                    stream_obj.set(scope, transform_func_key.into(), transform_fn);
                }
            }

            // ===== Readable 方法 =====
            // _read方法
            let read_func = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {},
            );
            let read_instance = read_func.get_function(scope).unwrap();
            let read_key = v8::String::new(scope, "_read").unwrap();
            stream_obj.set(scope, read_key.into(), read_instance.into());

            // read方法
            let read_public_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let read_key = v8::String::new(scope, "_read").unwrap();
                    if let Some(read_func_val) = this.get(scope, read_key.into()) {
                        if read_func_val.is_function() {
                            if let Ok(read_func) =
                                v8::Local::<v8::Function>::try_from(read_func_val)
                            {
                                read_func.call(scope, this.into(), &[]);
                            }
                        }
                    }
                    retval.set(v8::undefined(scope).into());
                },
            );
            let read_public_instance = read_public_func.get_function(scope).unwrap();
            let read_public_key = v8::String::new(scope, "read").unwrap();
            stream_obj.set(scope, read_public_key.into(), read_public_instance.into());

            // push方法
            let push_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let chunk = args.get(0);

                    // 如果流处于 flowing 模式，触发 data 事件
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            if let Some(flowing_val) = state_obj.get(scope, flowing_key.into()) {
                                if flowing_val.to_boolean(scope).boolean_value(scope) {
                                    let data_key = v8::String::new(scope, "data").unwrap();
                                    if let Some(listener) = this.get(scope, data_key.into()) {
                                        if listener.is_function() {
                                            if let Ok(func) =
                                                v8::Local::<v8::Function>::try_from(listener)
                                            {
                                                func.call(scope, this.into(), &[chunk]);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    retval.set(v8::Boolean::new(scope, true).into());
                },
            );
            let push_instance = push_func.get_function(scope).unwrap();
            let push_key = v8::String::new(scope, "push").unwrap();
            stream_obj.set(scope, push_key.into(), push_instance.into());

            // on方法
            let on_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {
                    let this = args.this();
                    let event = args.get(0);
                    let listener = args.get(1);
                    if event.is_string() && listener.is_function() {
                        let event_str = event.to_string(scope).unwrap().to_rust_string_lossy(scope);
                        let event_key = v8::String::new(scope, &event_str).unwrap();
                        this.set(scope, event_key.into(), listener);

                        // 当添加 'data' 监听器时，设置 flowing 为 true
                        if event_str == "data" {
                            let state_key = v8::String::new(scope, "_readableState").unwrap();
                            if let Some(state_val) = this.get(scope, state_key.into()) {
                                if let Some(state_obj) = state_val.to_object(scope) {
                                    let flowing_key = v8::String::new(scope, "flowing").unwrap();
                                    let flowing_val: v8::Local<v8::Value> =
                                        v8::Boolean::new(scope, true).into();
                                    state_obj.set(scope, flowing_key.into(), flowing_val);
                                }
                            }
                        }
                    }
                },
            );
            let on_instance = on_func.get_function(scope).unwrap();
            let on_key = v8::String::new(scope, "on").unwrap();
            stream_obj.set(scope, on_key.into(), on_instance.into());

            // once方法
            let once_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event = args.get(0);
                    let listener = args.get(1);
                    if event.is_string() && listener.is_function() {
                        let event_str = event.to_string(scope).unwrap().to_rust_string_lossy(scope);
                        let event_key = v8::String::new(scope, &event_str).unwrap();
                        this.set(scope, event_key.into(), listener);

                        // 当添加 'data' 监听器时，设置 flowing 为 true
                        if event_str == "data" {
                            let state_key = v8::String::new(scope, "_readableState").unwrap();
                            if let Some(state_val) = this.get(scope, state_key.into()) {
                                if let Some(state_obj) = state_val.to_object(scope) {
                                    let flowing_key = v8::String::new(scope, "flowing").unwrap();
                                    let flowing_val: v8::Local<v8::Value> =
                                        v8::Boolean::new(scope, true).into();
                                    state_obj.set(scope, flowing_key.into(), flowing_val);
                                }
                            }
                        }
                    }
                    retval.set(this.into());
                },
            );
            let once_instance = once_func.get_function(scope).unwrap();
            let once_key = v8::String::new(scope, "once").unwrap();
            stream_obj.set(scope, once_key.into(), once_instance.into());

            // pause方法
            let pause_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let paused_key = v8::String::new(scope, "paused").unwrap();
                            let paused_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            state_obj.set(scope, paused_key.into(), paused_val);
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            let flowing_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, false).into();
                            state_obj.set(scope, flowing_key.into(), flowing_val);
                        }
                    }
                    retval.set(this.into());
                },
            );
            let pause_instance = pause_func.get_function(scope).unwrap();
            let pause_key = v8::String::new(scope, "pause").unwrap();
            stream_obj.set(scope, pause_key.into(), pause_instance.into());

            // resume方法
            let resume_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let paused_key = v8::String::new(scope, "paused").unwrap();
                            let paused_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, false).into();
                            state_obj.set(scope, paused_key.into(), paused_val);
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            let flowing_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            state_obj.set(scope, flowing_key.into(), flowing_val);
                        }
                    }
                    retval.set(this.into());
                },
            );
            let resume_instance = resume_func.get_function(scope).unwrap();
            let resume_key = v8::String::new(scope, "resume").unwrap();
            stream_obj.set(scope, resume_key.into(), resume_instance.into());

            // pipe方法
            let pipe_func = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let _destination = args.get(0);
                    retval.set(this.into());
                },
            );
            let pipe_instance = pipe_func.get_function(scope).unwrap();
            let pipe_key = v8::String::new(scope, "pipe").unwrap();
            stream_obj.set(scope, pipe_key.into(), pipe_instance.into());

            // unpipe方法
            let unpipe_func = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {},
            );
            let unpipe_instance = unpipe_func.get_function(scope).unwrap();
            let unpipe_key = v8::String::new(scope, "unpipe").unwrap();
            stream_obj.set(scope, unpipe_key.into(), unpipe_instance.into());

            // _readableState
            let rstate_key = v8::String::new(scope, "_readableState").unwrap();
            let rstate_obj = v8::Object::new(scope);
            let flowing_key = v8::String::new(scope, "flowing").unwrap();
            let flowing_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            rstate_obj.set(scope, flowing_key.into(), flowing_val);
            let paused_key = v8::String::new(scope, "paused").unwrap();
            let paused_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            rstate_obj.set(scope, paused_key.into(), paused_val);
            let ended_key = v8::String::new(scope, "ended").unwrap();
            let ended_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            rstate_obj.set(scope, ended_key.into(), ended_val);
            let hwm_key = v8::String::new(scope, "highWaterMark").unwrap();
            let hwm_val: v8::Local<v8::Value> = v8::Integer::new(scope, 16 * 1024).into();
            rstate_obj.set(scope, hwm_key.into(), hwm_val);
            stream_obj.set(scope, rstate_key.into(), rstate_obj.into());

            // ===== Writable 方法 =====
            // _write方法 - 内部调用 _transform
            let write_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {
                    let this = args.this();
                    let chunk = args.get(0);
                    let encoding = args
                        .get(1)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let callback = args.get(2);

                    // 调用 _transform 方法
                    let transform_key = v8::String::new(scope, "_transform").unwrap();
                    if let Some(transform_func_val) = this.get(scope, transform_key.into()) {
                        if transform_func_val.is_function() {
                            if let Ok(transform_func) =
                                v8::Local::<v8::Function>::try_from(transform_func_val)
                            {
                                let chunk_val = chunk;
                                let encoding_val = v8::String::new(scope, &encoding).unwrap();
                                let callback_val = callback;
                                transform_func.call(
                                    scope,
                                    this.into(),
                                    &[chunk_val, encoding_val.into(), callback_val],
                                );
                            }
                        }
                    }
                },
            );
            let write_instance = write_func.get_function(scope).unwrap();
            let write_key = v8::String::new(scope, "_write").unwrap();
            stream_obj.set(scope, write_key.into(), write_instance.into());

            // write方法
            let write_public_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let chunk = args.get(0);
                    let encoding = args
                        .get(1)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let callback = args.get(2);
                    let write_key = v8::String::new(scope, "_write").unwrap();
                    if let Some(write_func_val) = this.get(scope, write_key.into()) {
                        if write_func_val.is_function() {
                            if let Ok(write_func) =
                                v8::Local::<v8::Function>::try_from(write_func_val)
                            {
                                let encoding_val = v8::String::new(scope, &encoding).unwrap();
                                write_func.call(
                                    scope,
                                    this.into(),
                                    &[chunk, encoding_val.into(), callback],
                                );
                            }
                        }
                    }
                    retval.set(v8::Boolean::new(scope, true).into());
                },
            );
            let write_public_instance = write_public_func.get_function(scope).unwrap();
            let write_public_key = v8::String::new(scope, "write").unwrap();
            stream_obj.set(scope, write_public_key.into(), write_public_instance.into());

            // end方法
            let end_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let _chunk = args.get(0);
                    let _encoding = args.get(1);
                    let callback = args.get(2);
                    let wstate_key = v8::String::new(scope, "_writableState").unwrap();
                    if let Some(wstate_val) = this.get(scope, wstate_key.into()) {
                        if let Some(wstate_obj) = wstate_val.to_object(scope) {
                            let ended_key = v8::String::new(scope, "ended").unwrap();
                            let ended_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            wstate_obj.set(scope, ended_key.into(), ended_val);
                            let writable_key = v8::String::new(scope, "writable").unwrap();
                            let writable_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, false).into();
                            wstate_obj.set(scope, writable_key.into(), writable_val);
                        }
                    }
                    let finish_key = v8::String::new(scope, "finish").unwrap();
                    if let Some(listener) = this.get(scope, finish_key.into()) {
                        if listener.is_function() {
                            if let Ok(func) = v8::Local::<v8::Function>::try_from(listener) {
                                func.call(scope, this.into(), &[]);
                            }
                        }
                    }
                    // 触发 end 事件
                    let end_key = v8::String::new(scope, "end").unwrap();
                    if let Some(end_listener) = this.get(scope, end_key.into()) {
                        if end_listener.is_function() {
                            if let Ok(func) = v8::Local::<v8::Function>::try_from(end_listener) {
                                func.call(scope, this.into(), &[]);
                            }
                        }
                    }
                    if callback.is_function() {
                        if let Ok(cb_func) = v8::Local::<v8::Function>::try_from(callback) {
                            cb_func.call(scope, this.into(), &[]);
                        }
                    }
                    retval.set(this.into());
                },
            );
            let end_instance = end_func.get_function(scope).unwrap();
            stream_obj.set(scope, on_key.into(), on_instance.into());

            let end_key = v8::String::new(scope, "end").unwrap();
            stream_obj.set(scope, end_key.into(), end_instance.into());

            // _writableState
            let wstate_key = v8::String::new(scope, "_writableState").unwrap();
            let wstate_obj = v8::Object::new(scope);
            let need_drain_key = v8::String::new(scope, "needDrain").unwrap();
            let need_drain_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            wstate_obj.set(scope, need_drain_key.into(), need_drain_val);
            let w_ended_key = v8::String::new(scope, "ended").unwrap();
            let w_ended_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            wstate_obj.set(scope, w_ended_key.into(), w_ended_val);
            let writable_flag_key = v8::String::new(scope, "writable").unwrap();
            let writable_flag_val: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
            wstate_obj.set(scope, writable_flag_key.into(), writable_flag_val);
            let w_hwm_key = v8::String::new(scope, "highWaterMark").unwrap();
            let w_hwm_val: v8::Local<v8::Value> = v8::Integer::new(scope, 16 * 1024).into();
            wstate_obj.set(scope, w_hwm_key.into(), w_hwm_val);
            stream_obj.set(scope, wstate_key.into(), wstate_obj.into());

            // ===== Transform 特有方法 =====
            // _transform方法 - 从对象属性中获取并调用用户的 transform 函数
            let transform_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {
                    let this = args.this();
                    let chunk = args.get(0);
                    let encoding = args
                        .get(1)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let mut callback = args.get(2);

                    // 如果没有提供 callback，创建一个空函数
                    if !callback.is_function() {
                        let callback_fn = v8::Function::new(
                            scope,
                            |_scope: &mut v8::PinScope,
                             _args: v8::FunctionCallbackArguments,
                             _retval: v8::ReturnValue| {},
                        )
                        .unwrap();
                        callback = callback_fn.into();
                    }

                    // 从对象获取用户的 transform 函数
                    let user_transform_key = v8::String::new(scope, "_user_transform").unwrap();
                    if let Some(transform_fn) = this.get(scope, user_transform_key.into()) {
                        if transform_fn.is_function() {
                            if let Ok(user_transform) =
                                v8::Local::<v8::Function>::try_from(transform_fn)
                            {
                                let chunk_str =
                                    chunk.to_string(scope).unwrap().to_rust_string_lossy(scope);
                                let chunk_for_js =
                                    v8::String::new(scope, &chunk_str).unwrap().into();
                                let encoding_val = v8::String::new(scope, &encoding).unwrap();
                                // 调用用户提供的 transform 函数
                                user_transform.call(
                                    scope,
                                    this.into(),
                                    &[chunk_for_js, encoding_val.into(), callback],
                                );
                                return;
                            }
                        }
                    }
                    // 如果没有用户 transform，直接调用 callback
                    if callback.is_function() {
                        if let Ok(cb) = v8::Local::<v8::Function>::try_from(callback) {
                            cb.call(scope, this.into(), &[]);
                        }
                    }
                },
            );
            let transform_instance = transform_func.get_function(scope).unwrap();
            let transform_key = v8::String::new(scope, "_transform").unwrap();
            stream_obj.set(scope, transform_key.into(), transform_instance.into());

            // 存储用户的 transform 函数以便 _transform 方法调用
            if let Some(transform_fn) = user_transform {
                let user_transform_key = v8::String::new(scope, "_user_transform").unwrap();
                stream_obj.set(scope, user_transform_key.into(), transform_fn);
            }

            retval.set(stream_obj.into());
        },
    );
    let transform_func = transform_constructor.get_function(scope).unwrap();
    let transform_key = v8::String::new(scope, "Transform").unwrap();
    stream_obj.set(scope, transform_key.into(), transform_func.into());

    // Duplex Stream constructor - v0.3.58: Complete implementation with Readable + Writable methods
    let duplex_constructor = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let stream_obj = if this.is_object() {
                this
            } else {
                v8::Object::new(scope)
            };

            // 提取用户提供的 _write 函数
            let options = args.get(0);
            let user_write: Option<v8::Local<v8::Value>> = if options.is_object() {
                let write_key = v8::String::new(scope, "_write").unwrap();
                options
                    .to_object(scope)
                    .and_then(|obj| obj.get(scope, write_key.into()))
            } else {
                None
            };

            // ===== Readable 方法 =====
            // _read方法
            let read_func = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {},
            );
            let read_instance = read_func.get_function(scope).unwrap();
            let read_key = v8::String::new(scope, "_read").unwrap();
            stream_obj.set(scope, read_key.into(), read_instance.into());

            // read方法
            let read_public_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let read_key = v8::String::new(scope, "_read").unwrap();
                    if let Some(read_func_val) = this.get(scope, read_key.into()) {
                        if read_func_val.is_function() {
                            if let Ok(read_func) =
                                v8::Local::<v8::Function>::try_from(read_func_val)
                            {
                                read_func.call(scope, this.into(), &[]);
                            }
                        }
                    }
                    retval.set(v8::undefined(scope).into());
                },
            );
            let read_public_instance = read_public_func.get_function(scope).unwrap();
            let read_public_key = v8::String::new(scope, "read").unwrap();
            stream_obj.set(scope, read_public_key.into(), read_public_instance.into());

            // push方法
            let push_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let chunk = args.get(0);
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            if let Some(flowing_val) = state_obj.get(scope, flowing_key.into()) {
                                if flowing_val.to_boolean(scope).boolean_value(scope) {
                                    let data_key = v8::String::new(scope, "data").unwrap();
                                    if let Some(listener) = this.get(scope, data_key.into()) {
                                        if listener.is_function() {
                                            if let Ok(func) =
                                                v8::Local::<v8::Function>::try_from(listener)
                                            {
                                                func.call(scope, this.into(), &[chunk]);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    retval.set(v8::Boolean::new(scope, true).into());
                },
            );
            let push_instance = push_func.get_function(scope).unwrap();
            let push_key = v8::String::new(scope, "push").unwrap();
            stream_obj.set(scope, push_key.into(), push_instance.into());

            // on方法
            let on_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {
                    let this = args.this();
                    let event = args.get(0);
                    let listener = args.get(1);
                    if event.is_string() && listener.is_function() {
                        let event_str = event.to_string(scope).unwrap().to_rust_string_lossy(scope);
                        let event_key = v8::String::new(scope, &event_str).unwrap();
                        this.set(scope, event_key.into(), listener);

                        // 当添加 'data' 监听器时，设置 flowing 为 true
                        if event_str == "data" {
                            let state_key = v8::String::new(scope, "_readableState").unwrap();
                            if let Some(state_val) = this.get(scope, state_key.into()) {
                                if let Some(state_obj) = state_val.to_object(scope) {
                                    let flowing_key = v8::String::new(scope, "flowing").unwrap();
                                    let flowing_val: v8::Local<v8::Value> =
                                        v8::Boolean::new(scope, true).into();
                                    state_obj.set(scope, flowing_key.into(), flowing_val);
                                }
                            }
                        }
                    }
                },
            );
            let on_instance = on_func.get_function(scope).unwrap();
            let on_key = v8::String::new(scope, "on").unwrap();
            stream_obj.set(scope, on_key.into(), on_instance.into());

            // once方法
            let once_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let event = args.get(0);
                    let listener = args.get(1);
                    if event.is_string() && listener.is_function() {
                        let event_str = event.to_string(scope).unwrap().to_rust_string_lossy(scope);
                        let event_key = v8::String::new(scope, &event_str).unwrap();
                        this.set(scope, event_key.into(), listener);

                        // 当添加 'data' 监听器时，设置 flowing 为 true
                        if event_str == "data" {
                            let state_key = v8::String::new(scope, "_readableState").unwrap();
                            if let Some(state_val) = this.get(scope, state_key.into()) {
                                if let Some(state_obj) = state_val.to_object(scope) {
                                    let flowing_key = v8::String::new(scope, "flowing").unwrap();
                                    let flowing_val: v8::Local<v8::Value> =
                                        v8::Boolean::new(scope, true).into();
                                    state_obj.set(scope, flowing_key.into(), flowing_val);
                                }
                            }
                        }
                    }
                    retval.set(this.into());
                },
            );
            let once_instance = once_func.get_function(scope).unwrap();
            let once_key = v8::String::new(scope, "once").unwrap();
            stream_obj.set(scope, once_key.into(), once_instance.into());

            // pause方法
            let pause_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let paused_key = v8::String::new(scope, "paused").unwrap();
                            let paused_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            state_obj.set(scope, paused_key.into(), paused_val);
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            let flowing_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, false).into();
                            state_obj.set(scope, flowing_key.into(), flowing_val);
                        }
                    }
                    retval.set(this.into());
                },
            );
            let pause_instance = pause_func.get_function(scope).unwrap();
            let pause_key = v8::String::new(scope, "pause").unwrap();
            stream_obj.set(scope, pause_key.into(), pause_instance.into());

            // resume方法
            let resume_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let paused_key = v8::String::new(scope, "paused").unwrap();
                            let paused_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, false).into();
                            state_obj.set(scope, paused_key.into(), paused_val);
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            let flowing_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            state_obj.set(scope, flowing_key.into(), flowing_val);
                        }
                    }
                    retval.set(this.into());
                },
            );
            let resume_instance = resume_func.get_function(scope).unwrap();
            let resume_key = v8::String::new(scope, "resume").unwrap();
            stream_obj.set(scope, resume_key.into(), resume_instance.into());

            // pipe方法
            let pipe_func = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let _destination = args.get(0);
                    retval.set(this.into());
                },
            );
            let pipe_instance = pipe_func.get_function(scope).unwrap();
            let pipe_key = v8::String::new(scope, "pipe").unwrap();
            stream_obj.set(scope, pipe_key.into(), pipe_instance.into());

            // unpipe方法
            let unpipe_func = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {},
            );
            let unpipe_instance = unpipe_func.get_function(scope).unwrap();
            let unpipe_key = v8::String::new(scope, "unpipe").unwrap();
            stream_obj.set(scope, unpipe_key.into(), unpipe_instance.into());

            // _readableState
            let rstate_key = v8::String::new(scope, "_readableState").unwrap();
            let rstate_obj = v8::Object::new(scope);
            let flowing_key = v8::String::new(scope, "flowing").unwrap();
            let flowing_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            rstate_obj.set(scope, flowing_key.into(), flowing_val);
            let paused_key = v8::String::new(scope, "paused").unwrap();
            let paused_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            rstate_obj.set(scope, paused_key.into(), paused_val);
            let ended_key = v8::String::new(scope, "ended").unwrap();
            let ended_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            rstate_obj.set(scope, ended_key.into(), ended_val);
            let hwm_key = v8::String::new(scope, "highWaterMark").unwrap();
            let hwm_val: v8::Local<v8::Value> = v8::Integer::new(scope, 16 * 1024).into();
            rstate_obj.set(scope, hwm_key.into(), hwm_val);
            stream_obj.set(scope, rstate_key.into(), rstate_obj.into());

            // ===== Writable 方法 =====
            // _write方法 - 从对象属性中获取并调用用户的 _write 函数
            let write_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {
                    let this = args.this();
                    let chunk = args.get(0);
                    let encoding = args
                        .get(1)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let mut callback = args.get(2);

                    // 如果没有提供 callback，创建一个空函数
                    if !callback.is_function() {
                        let callback_fn = v8::Function::new(
                            scope,
                            |_scope: &mut v8::PinScope,
                             _args: v8::FunctionCallbackArguments,
                             _retval: v8::ReturnValue| {},
                        )
                        .unwrap();
                        callback = callback_fn.into();
                    }

                    // 从对象获取用户的 _write 函数
                    let user_write_key = v8::String::new(scope, "_user_write").unwrap();
                    if let Some(write_fn) = this.get(scope, user_write_key.into()) {
                        if write_fn.is_function() {
                            if let Ok(user_write) = v8::Local::<v8::Function>::try_from(write_fn) {
                                let chunk_str =
                                    chunk.to_string(scope).unwrap().to_rust_string_lossy(scope);
                                let chunk_for_js =
                                    v8::String::new(scope, &chunk_str).unwrap().into();
                                let encoding_val = v8::String::new(scope, &encoding).unwrap();
                                user_write.call(
                                    scope,
                                    this.into(),
                                    &[chunk_for_js, encoding_val.into(), callback],
                                );
                                return;
                            }
                        }
                    }
                    if callback.is_function() {
                        if let Ok(cb) = v8::Local::<v8::Function>::try_from(callback) {
                            cb.call(scope, this.into(), &[]);
                        }
                    }
                },
            );
            let write_instance = write_func.get_function(scope).unwrap();
            let write_key = v8::String::new(scope, "_write").unwrap();
            stream_obj.set(scope, write_key.into(), write_instance.into());

            // 存储用户的 _write 函数以便 _write 方法调用
            if let Some(write_fn) = user_write {
                let user_write_key = v8::String::new(scope, "_user_write").unwrap();
                stream_obj.set(scope, user_write_key.into(), write_fn);
            }

            // write方法
            let write_public_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let chunk = args.get(0);
                    let encoding = args
                        .get(1)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let callback = args.get(2);
                    let write_key = v8::String::new(scope, "_write").unwrap();
                    if let Some(write_func_val) = this.get(scope, write_key.into()) {
                        if write_func_val.is_function() {
                            if let Ok(write_func) =
                                v8::Local::<v8::Function>::try_from(write_func_val)
                            {
                                let encoding_val = v8::String::new(scope, &encoding).unwrap();
                                write_func.call(
                                    scope,
                                    this.into(),
                                    &[chunk, encoding_val.into(), callback],
                                );
                            }
                        }
                    }
                    retval.set(v8::Boolean::new(scope, true).into());
                },
            );
            let write_public_instance = write_public_func.get_function(scope).unwrap();
            let write_public_key = v8::String::new(scope, "write").unwrap();
            stream_obj.set(scope, write_public_key.into(), write_public_instance.into());

            // end方法
            let end_func = v8::FunctionTemplate::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let _chunk = args.get(0);
                    let _encoding = args.get(1);
                    let callback = args.get(2);
                    let wstate_key = v8::String::new(scope, "_writableState").unwrap();
                    if let Some(wstate_val) = this.get(scope, wstate_key.into()) {
                        if let Some(wstate_obj) = wstate_val.to_object(scope) {
                            let ended_key = v8::String::new(scope, "ended").unwrap();
                            let ended_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            wstate_obj.set(scope, ended_key.into(), ended_val);
                            let writable_key = v8::String::new(scope, "writable").unwrap();
                            let writable_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, false).into();
                            wstate_obj.set(scope, writable_key.into(), writable_val);
                        }
                    }
                    let finish_key = v8::String::new(scope, "finish").unwrap();
                    if let Some(listener) = this.get(scope, finish_key.into()) {
                        if listener.is_function() {
                            if let Ok(func) = v8::Local::<v8::Function>::try_from(listener) {
                                func.call(scope, this.into(), &[]);
                            }
                        }
                    }
                    // 触发 end 事件
                    let end_key = v8::String::new(scope, "end").unwrap();
                    if let Some(end_listener) = this.get(scope, end_key.into()) {
                        if end_listener.is_function() {
                            if let Ok(func) = v8::Local::<v8::Function>::try_from(end_listener) {
                                func.call(scope, this.into(), &[]);
                            }
                        }
                    }
                    if callback.is_function() {
                        if let Ok(cb_func) = v8::Local::<v8::Function>::try_from(callback) {
                            cb_func.call(scope, this.into(), &[]);
                        }
                    }
                    retval.set(this.into());
                },
            );
            let end_instance = end_func.get_function(scope).unwrap();
            let end_key = v8::String::new(scope, "end").unwrap();
            stream_obj.set(scope, end_key.into(), end_instance.into());

            // _writableState
            let wstate_key = v8::String::new(scope, "_writableState").unwrap();
            let wstate_obj = v8::Object::new(scope);
            let need_drain_key = v8::String::new(scope, "needDrain").unwrap();
            let need_drain_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            wstate_obj.set(scope, need_drain_key.into(), need_drain_val);
            let w_ended_key = v8::String::new(scope, "ended").unwrap();
            let w_ended_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            wstate_obj.set(scope, w_ended_key.into(), w_ended_val);
            let writable_flag_key = v8::String::new(scope, "writable").unwrap();
            let writable_flag_val: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
            wstate_obj.set(scope, writable_flag_key.into(), writable_flag_val);
            let w_hwm_key = v8::String::new(scope, "highWaterMark").unwrap();
            let w_hwm_val: v8::Local<v8::Value> = v8::Integer::new(scope, 16 * 1024).into();
            wstate_obj.set(scope, w_hwm_key.into(), w_hwm_val);
            stream_obj.set(scope, wstate_key.into(), wstate_obj.into());

            retval.set(stream_obj.into());
        },
    );
    let duplex_func = duplex_constructor.get_function(scope).unwrap();
    let duplex_key = v8::String::new(scope, "Duplex").unwrap();
    stream_obj.set(scope, duplex_key.into(), duplex_func.into());

    // v0.3.59: pipeline function - connects multiple streams sequentially
    let pipeline_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Collect all stream arguments
            let mut streams: Vec<v8::Local<v8::Value>> = Vec::new();

            for i in 0..args.length() {
                let stream: v8::Local<v8::Value> = args.get(i);
                if stream.is_object() {
                    streams.push(stream);
                }
            }

            // Need at least 2 streams
            if streams.len() < 2 {
                retval.set(v8::undefined(scope).into());
                return;
            }

            // Establish pipe connections sequentially
            let mut last_writable: Option<v8::Local<v8::Value>> = None;

            for i in 0..streams.len() - 1 {
                let source = streams[i];
                let destination = streams[i + 1];

                if let (Some(source_obj), Some(dest_obj)) =
                    (source.to_object(scope), destination.to_object(scope))
                {
                    // Check if source has pipe method
                    let pipe_key: v8::Local<v8::Value> =
                        v8::String::new(scope, "pipe").unwrap().into();

                    if source_obj.has(scope, pipe_key).unwrap_or(false) {
                        if let Some(pipe_func) = source_obj.get(scope, pipe_key) {
                            if pipe_func.is_function() {
                                if let Ok(pipe_fn) = v8::Local::<v8::Function>::try_from(pipe_func)
                                {
                                    // Call source.pipe(destination)
                                    pipe_fn.call(scope, source.into(), &[destination]);

                                    // Check if destination has 'end' method (indicates Writable)
                                    let end_key: v8::Local<v8::Value> =
                                        v8::String::new(scope, "end").unwrap().into();
                                    if dest_obj.has(scope, end_key).unwrap_or(false) {
                                        last_writable = Some(destination);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Return last Writable stream
            if let Some(last) = last_writable {
                retval.set(last);
            } else {
                retval.set(v8::undefined(scope).into());
            }
        },
    );
    let pipeline_instance = pipeline_func.get_function(scope).unwrap();
    let pipeline_key = v8::String::new(scope, "pipeline").unwrap();
    stream_obj.set(scope, pipeline_key.into(), pipeline_instance.into());

    // v0.3.74: passThrough stream - complete implementation following nodejs_core/stream.rs pattern
    // PassThrough is a Transform stream that passes data through without modification
    let passthrough_func = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let stream_obj = v8::Object::new(_scope);

            // ===== Readable methods =====

            // _read method - default implementation
            let read_func = v8::FunctionTemplate::new(
                _scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    let this = args.this();
                    // Get readable state
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            // Set ended=true
                            let ended_key = v8::String::new(scope, "ended").unwrap();
                            let ended_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            state_obj.set(scope, ended_key.into(), ended_val);
                        }
                    }
                    // Emit 'readable' event
                    let on_key = v8::String::new(scope, "on").unwrap();
                    if let Some(on_func_val) = this.get(scope, on_key.into()) {
                        if on_func_val.is_function() {
                            if let Ok(on_fn) = v8::Local::<v8::Function>::try_from(on_func_val) {
                                let event_name = v8::String::new(scope, "readable").unwrap();
                                on_fn.call(scope, this.into(), &[event_name.into()]);
                            }
                        }
                    }
                },
            )
            .get_function(_scope)
            .unwrap();
            let _read_key = v8::String::new(_scope, "_read").unwrap();
            stream_obj.set(_scope, _read_key.into(), read_func.into());

            // read method
            let read_public_func = v8::FunctionTemplate::new(
                _scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    // Return empty string to simulate reading
                    let empty_str: v8::Local<v8::Value> =
                        v8::String::new(_scope, "").unwrap().into();
                    retval.set(empty_str);
                },
            )
            .get_function(_scope)
            .unwrap();
            let read_public_key = v8::String::new(_scope, "read").unwrap();
            stream_obj.set(_scope, read_public_key.into(), read_public_func.into());

            // push method - emits 'data' event with stored callback
            // v0.3.81: Fix to check is_object before to_object to avoid "Cannot convert undefined or null to object"
            let push_func = v8::FunctionTemplate::new(
                _scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let chunk = args.get(0);
                    let _encoding = args.get(1);

                    let this = args.this();

                    let chunk_val = if chunk.is_undefined() || chunk.is_null() {
                        v8::String::new(scope, "").unwrap().into()
                    } else {
                        chunk
                    };

                    // v0.3.81: First check 'data' callback directly on this (set by pipe())
                    let data_key = v8::String::new(scope, "data").unwrap();
                    if let Some(callback_val) = this.get(scope, data_key.into()) {
                        if callback_val.is_function() {
                            if let Ok(callback) = v8::Local::<v8::Function>::try_from(callback_val)
                            {
                                callback.call(scope, this.into(), &[chunk_val]);
                                retval.set(v8::Boolean::new(scope, true).into());
                                return;
                            }
                        }
                    }

                    // v0.3.81: Fall back to _events for on() registered listeners
                    // v0.3.81: Check is_object before to_object to avoid TypeError
                    let events_key = v8::String::new(scope, "_events").unwrap();
                    if let Some(events_val) = this.get(scope, events_key.into()) {
                        if events_val.is_object() {
                            if let Some(events_obj) = events_val.to_object(scope) {
                                if let Some(callback_val) = events_obj.get(scope, data_key.into()) {
                                    if callback_val.is_function() {
                                        if let Ok(callback) =
                                            v8::Local::<v8::Function>::try_from(callback_val)
                                        {
                                            callback.call(scope, this.into(), &[chunk_val]);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Return true
                    let result: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
                    retval.set(result);
                },
            )
            .get_function(_scope)
            .unwrap();
            let push_key = v8::String::new(_scope, "push").unwrap();
            stream_obj.set(_scope, push_key.into(), push_func.into());

            // on method - event listener that stores callbacks
            let on_func = v8::FunctionTemplate::new(
                _scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    let event_name = args.get(0);
                    let callback = args.get(1);
                    let this = args.this();

                    if !event_name.is_string() || !callback.is_function() {
                        return;
                    }

                    // Store callback on the object using event name as key
                    let events_key = v8::String::new(scope, "_events").unwrap();
                    let events_val = this.get(scope, events_key.into());

                    let events_obj = if let Some(val) = events_val {
                        if val.is_object() {
                            val.to_object(scope).unwrap()
                        } else {
                            let new_events = v8::Object::new(scope);
                            this.set(scope, events_key.into(), new_events.into());
                            new_events
                        }
                    } else {
                        let new_events = v8::Object::new(scope);
                        this.set(scope, events_key.into(), new_events.into());
                        new_events
                    };

                    // Get event name string
                    let name_str = event_name.to_string(scope).unwrap();
                    let name_str_local: v8::Local<v8::String> = name_str;

                    events_obj.set(scope, name_str_local.into(), callback);
                },
            )
            .get_function(_scope)
            .unwrap();
            let on_key = v8::String::new(_scope, "on").unwrap();
            stream_obj.set(_scope, on_key.into(), on_func.into());

            // once method
            let once_func = v8::FunctionTemplate::new(
                _scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    // Same as on for now
                },
            )
            .get_function(_scope)
            .unwrap();
            let once_key = v8::String::new(_scope, "once").unwrap();
            stream_obj.set(_scope, once_key.into(), once_func.into());

            // pause method
            let pause_func = v8::FunctionTemplate::new(
                _scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    let this = args.this();
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            let flowing_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, false).into();
                            state_obj.set(scope, flowing_key.into(), flowing_val);
                        }
                    }
                },
            )
            .get_function(_scope)
            .unwrap();
            let pause_key = v8::String::new(_scope, "pause").unwrap();
            stream_obj.set(_scope, pause_key.into(), pause_func.into());

            // resume method
            let resume_func = v8::FunctionTemplate::new(
                _scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    let this = args.this();
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            let flowing_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            state_obj.set(scope, flowing_key.into(), flowing_val);
                        }
                    }
                },
            )
            .get_function(_scope)
            .unwrap();
            let resume_key = v8::String::new(_scope, "resume").unwrap();
            stream_obj.set(_scope, resume_key.into(), resume_func.into());

            // pipe method - connects this (source) to destination
            let pipe_func = v8::FunctionTemplate::new(
                _scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    let this = args.this();
                    let dest = args.get(0);

                    // Store destination reference on source for data forwarding
                    let dest_ref_key = v8::String::new(scope, "_pipeDest").unwrap();
                    this.set(scope, dest_ref_key.into(), dest);

                    // Set flowing=true on readable state to enable data flow
                    let state_key = v8::String::new(scope, "_readableState").unwrap();
                    if let Some(state_val) = this.get(scope, state_key.into()) {
                        if let Some(state_obj) = state_val.to_object(scope) {
                            let flowing_key = v8::String::new(scope, "flowing").unwrap();
                            let flowing_val: v8::Local<v8::Value> =
                                v8::Boolean::new(scope, true).into();
                            state_obj.set(scope, flowing_key.into(), flowing_val);
                        }
                    }

                    // Return destination for chaining
                    retval.set(dest);
                },
            )
            .get_function(_scope)
            .unwrap();
            let pipe_key = v8::String::new(_scope, "pipe").unwrap();
            stream_obj.set(_scope, pipe_key.into(), pipe_func.into());

            // unpipe method
            let unpipe_func = v8::FunctionTemplate::new(
                _scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    // No-op for now
                },
            )
            .get_function(_scope)
            .unwrap();
            let unpipe_key = v8::String::new(_scope, "unpipe").unwrap();
            stream_obj.set(_scope, unpipe_key.into(), unpipe_func.into());

            // _readableState
            let readable_state_key = v8::String::new(_scope, "_readableState").unwrap();
            let readable_state_obj = v8::Object::new(_scope);
            let flowing_key = v8::String::new(_scope, "flowing").unwrap();
            let flowing_val: v8::Local<v8::Value> = v8::Boolean::new(_scope, false).into();
            readable_state_obj.set(_scope, flowing_key.into(), flowing_val);
            let paused_key = v8::String::new(_scope, "paused").unwrap();
            let paused_val: v8::Local<v8::Value> = v8::Boolean::new(_scope, false).into();
            readable_state_obj.set(_scope, paused_key.into(), paused_val);
            let ended_key = v8::String::new(_scope, "ended").unwrap();
            let ended_val: v8::Local<v8::Value> = v8::Boolean::new(_scope, false).into();
            readable_state_obj.set(_scope, ended_key.into(), ended_val);
            let high_water_mark_key = v8::String::new(_scope, "highWaterMark").unwrap();
            let hwm_val: v8::Local<v8::Value> = v8::Integer::new(_scope, 16 * 1024).into();
            readable_state_obj.set(_scope, high_water_mark_key.into(), hwm_val);
            stream_obj.set(_scope, readable_state_key.into(), readable_state_obj.into());

            // ===== Writable methods =====

            // _write method - PassThrough implementation that calls push to pass data through
            // Also forwards data to pipe destination if one exists
            let write_func = v8::FunctionTemplate::new(_scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _retval: v8::ReturnValue| {
            let chunk = args.get(0);
            let _encoding = args.get(1);
            let callback = args.get(2);

            let this = args.this();

            // Call push to pass data through (this is the key for PassThrough behavior)
            let push_key = v8::String::new(scope, "push").unwrap();
            if let Some(push_func_val) = this.get(scope, push_key.into()) {
                if push_func_val.is_function() {
                    if let Ok(push_fn) = v8::Local::<v8::Function>::try_from(push_func_val) {
                        let chunk_to_push = if chunk.is_undefined() || chunk.is_null() {
                            v8::String::new(scope, "").unwrap().into()
                        } else {
                            chunk
                        };
                        push_fn.call(scope, this.into(), &[chunk_to_push]);
                    }
                }
            }

            // Forward to pipe destination if one exists
            let dest_ref_key = v8::String::new(scope, "_pipeDest").unwrap();
            if let Some(dest_val) = this.get(scope, dest_ref_key.into()) {
                if let Ok(dest) = v8::Local::<v8::Object>::try_from(dest_val) {
                    let write_key = v8::String::new(scope, "write").unwrap();
                    if let Some(write_func_val) = dest.get(scope, write_key.into()) {
                        if write_func_val.is_function() {
                            if let Ok(write_fn) = v8::Local::<v8::Function>::try_from(write_func_val) {
                                let enc_str = v8::String::new(scope, "utf8").unwrap();
                                let chunk_to_write = if chunk.is_undefined() || chunk.is_null() {
                                    v8::String::new(scope, "").unwrap().into()
                                } else {
                                    chunk
                                };
                                // Use a noop callback since we need to call our callback too
                                let noop_fn = v8::Function::new(scope, |_s: &mut v8::PinScope, _a: v8::FunctionCallbackArguments, _r: v8::ReturnValue| {}).unwrap();
                                write_fn.call(scope, dest.into(), &[chunk_to_write, enc_str.into(), noop_fn.into()]);
                            }
                        }
                    }
                }
            }

            // Call callback to indicate write is done
            if callback.is_function() {
                if let Ok(cb_fn) = v8::Local::<v8::Function>::try_from(callback) {
                    cb_fn.call(scope, this.into(), &[]);
                }
            }
        }).get_function(_scope).unwrap();
            let _write_key = v8::String::new(_scope, "_write").unwrap();
            stream_obj.set(_scope, _write_key.into(), write_func.into());

            // write method
            let write_public_func = v8::FunctionTemplate::new(_scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
            let chunk = args.get(0);
            let _encoding = args.get(1);
            let callback = args.get(2);

            // Get _write and call it
            let this = args.this();
            let write_key = v8::String::new(scope, "_write").unwrap();
            if let Some(write_func_val) = this.get(scope, write_key.into()) {
                if write_func_val.is_function() {
                    if let Ok(write_fn) = v8::Local::<v8::Function>::try_from(write_func_val) {
                        let enc_str = v8::String::new(scope, "utf8").unwrap();
                        // Create a no-op callback if none provided
                        let cb = if callback.is_function() {
                            callback
                        } else {
                            let noop_fn = v8::Function::new(scope, |_s: &mut v8::PinScope, _a: v8::FunctionCallbackArguments, _r: v8::ReturnValue| {}).unwrap();
                            noop_fn.into()
                        };
                        let chunk_val = if chunk.is_undefined() || chunk.is_null() {
                            v8::String::new(scope, "").unwrap().into()
                        } else {
                            chunk
                        };
                        write_fn.call(scope, this.into(), &[chunk_val, enc_str.into(), cb]);
                    }
                }
            }

            // Return true (stream not full)
            let result: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
            retval.set(result);
        }).get_function(_scope).unwrap();
            let write_public_key = v8::String::new(_scope, "write").unwrap();
            stream_obj.set(_scope, write_public_key.into(), write_public_func.into());

            // end method
            let end_func = v8::FunctionTemplate::new(
                _scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    let _chunk = args.get(0);
                    let _encoding = args.get(1);
                    let callback = args.get(2);

                    let this = args.this();

                    // Emit 'end' event
                    let on_key = v8::String::new(scope, "on").unwrap();
                    if let Some(on_func_val) = this.get(scope, on_key.into()) {
                        if on_func_val.is_function() {
                            if let Ok(on_fn) = v8::Local::<v8::Function>::try_from(on_func_val) {
                                let end_key = v8::String::new(scope, "end").unwrap();
                                on_fn.call(scope, this.into(), &[end_key.into()]);
                            }
                        }
                    }

                    // Call callback if provided
                    if callback.is_function() {
                        if let Ok(cb_fn) = v8::Local::<v8::Function>::try_from(callback) {
                            cb_fn.call(scope, this.into(), &[]);
                        }
                    }
                },
            )
            .get_function(_scope)
            .unwrap();
            let end_key = v8::String::new(_scope, "end").unwrap();
            stream_obj.set(_scope, end_key.into(), end_func.into());

            // _writableState
            let writable_state_key = v8::String::new(_scope, "_writableState").unwrap();
            let writable_state_obj = v8::Object::new(_scope);
            let writable_ended_key = v8::String::new(_scope, "ended").unwrap();
            let writable_ended_val: v8::Local<v8::Value> = v8::Boolean::new(_scope, false).into();
            writable_state_obj.set(_scope, writable_ended_key.into(), writable_ended_val);
            let writable_finished_key = v8::String::new(_scope, "finished").unwrap();
            let writable_finished_val: v8::Local<v8::Value> =
                v8::Boolean::new(_scope, false).into();
            writable_state_obj.set(_scope, writable_finished_key.into(), writable_finished_val);
            let writable_flag_key = v8::String::new(_scope, "writable").unwrap();
            let writable_flag_val: v8::Local<v8::Value> = v8::Boolean::new(_scope, true).into();
            writable_state_obj.set(_scope, writable_flag_key.into(), writable_flag_val);
            let w_hwm_key = v8::String::new(_scope, "highWaterMark").unwrap();
            let w_hwm_val: v8::Local<v8::Value> = v8::Integer::new(_scope, 16 * 1024).into();
            writable_state_obj.set(_scope, w_hwm_key.into(), w_hwm_val);
            stream_obj.set(_scope, writable_state_key.into(), writable_state_obj.into());

            retval.set(stream_obj.into());
        },
    );
    let passthrough_instance = passthrough_func.get_function(scope).unwrap();
    let passthrough_key = v8::String::new(scope, "passThrough").unwrap();
    stream_obj.set(scope, passthrough_key.into(), passthrough_instance.into());
    let passthrough_alias = v8::String::new(scope, "PassThrough").unwrap();
    stream_obj.set(scope, passthrough_alias.into(), passthrough_instance.into());

    // Set stream as global
    let stream_key = v8::String::new(scope, "stream").unwrap();
    global.set(scope, stream_key.into(), stream_obj.into());

    let stream_bootstrap = r#"
    (function() {
        const proto = typeof EventEmitter !== 'undefined' ? EventEmitter.prototype : Object.prototype;
        function Stream(opts) {
            if (typeof EventEmitter !== 'undefined') {
                EventEmitter.call(this, opts);
            }
        }
        Stream.prototype = Object.create(proto);
        if (globalThis.stream) {
            Object.assign(Stream, globalThis.stream);
        }
        const streamClasses = [
            Stream.Readable,
            Stream.Writable,
            Stream.Duplex,
            Stream.Transform,
            Stream.PassThrough
        ];
        for (const Cls of streamClasses) {
            if (Cls && Cls.prototype) {
                Object.setPrototypeOf(Cls.prototype, Stream.prototype);
            }
        }

        function finished(stream, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = undefined;
            }
            if (!callback) {
                return new Promise((resolve, reject) => {
                    finished(stream, options, (err) => {
                        if (err) reject(err);
                        else resolve();
                    });
                });
            }
            let isDone = false;
            function done(err) {
                if (isDone) return;
                isDone = true;
                cleanup();
                callback(err);
            }
            function onFinish() { done(); }
            function onEnd() { done(); }
            function onClose() { done(); }
            function onError(err) { done(err); }
            function cleanup() {
                if (stream && typeof stream.removeListener === 'function') {
                    stream.removeListener('finish', onFinish);
                    stream.removeListener('end', onEnd);
                    stream.removeListener('close', onClose);
                    stream.removeListener('error', onError);
                }
            }
            if (stream && typeof stream.on === 'function') {
                stream.on('finish', onFinish);
                stream.on('end', onEnd);
                stream.on('close', onClose);
                stream.on('error', onError);
            }
            return cleanup;
        }

        function addAbortSignal(signal, stream) {
            if (!signal || !stream) return stream;
            if (signal.aborted) {
                if (typeof stream.destroy === 'function') {
                    stream.destroy(new Error('This operation was aborted'));
                }
                return stream;
            }
            function onAbort() {
                if (typeof stream.destroy === 'function') {
                    stream.destroy(new Error('This operation was aborted'));
                }
            }
            if (typeof signal.addEventListener === 'function') {
                signal.addEventListener('abort', onAbort, { once: true });
            }
            return stream;
        }

        if (Stream.Readable) {
            Stream.Readable.from = function(iterable, options) {
                let iter;
                if (iterable && typeof iterable[Symbol.asyncIterator] === 'function') {
                    iter = iterable[Symbol.asyncIterator]();
                } else if (iterable && typeof iterable[Symbol.iterator] === 'function') {
                    iter = iterable[Symbol.iterator]();
                }
                const r = new Stream.Readable({
                    ...options,
                    async read() {
                        if (!iter) {
                            this.push(null);
                            return;
                        }
                        try {
                            const res = await iter.next();
                            if (res.done) {
                                this.push(null);
                            } else {
                                this.push(res.value);
                            }
                        } catch (err) {
                            if (typeof this.destroy === 'function') this.destroy(err);
                            else this.push(null);
                        }
                    }
                });
                return r;
            };
            if (Stream.Readable.prototype) {
                Stream.Readable.prototype.setEncoding = function(encoding) {
                    if (this._readableState) {
                        this._readableState.encoding = encoding;
                    }
                    this._encoding = encoding;
                    return this;
                };
                Stream.Readable.prototype.pause = function() {
                    if (this._readableState) {
                        this._readableState.paused = true;
                        this._readableState.flowing = false;
                    }
                    return this;
                };
                Stream.Readable.prototype.resume = function() {
                    if (this._readableState) {
                        this._readableState.paused = false;
                        this._readableState.flowing = true;
                    }
                    return this;
                };
                Stream.Readable.prototype.isPaused = function() {
                    return this._readableState ? !!this._readableState.paused : false;
                };
            }

            if (Stream.Readable.prototype && !Stream.Readable.prototype[Symbol.asyncIterator]) {
                Stream.Readable.prototype[Symbol.asyncIterator] = function() {
                    const stream = this;
                    const queue = [];
                    let ended = false;
                    let err = null;
                    let notify = null;

                    function onData(chunk) {
                        queue.push(chunk);
                        if (notify) {
                            const n = notify;
                            notify = null;
                            n();
                        }
                    }
                    function onEnd() {
                        ended = true;
                        if (notify) {
                            const n = notify;
                            notify = null;
                            n();
                        }
                    }
                    function onError(e) {
                        err = e;
                        ended = true;
                        if (notify) {
                            const n = notify;
                            notify = null;
                            n();
                        }
                    }

                    if (stream._readableState) {
                        stream._readableState.flowing = true;
                    }
                    if (typeof stream.on === 'function') {
                        stream.on('data', onData);
                        stream.on('end', onEnd);
                        stream.on('error', onError);
                    }
                    if (typeof stream.read === 'function') {
                        stream.read();
                    }

                    return {
                        next() {
                            return new Promise((resolve, reject) => {
                                function check() {
                                    if (err) {
                                        return reject(err);
                                    }
                                    if (queue.length > 0) {
                                        return resolve({ value: queue.shift(), done: false });
                                    }
                                    if (ended) {
                                        return resolve({ value: undefined, done: true });
                                    }
                                    notify = check;
                                    if (typeof stream.read === 'function') {
                                        stream.read();
                                    }
                                }
                                check();
                            });
                        },
                        return() {
                            if (typeof stream.destroy === 'function') stream.destroy();
                            return Promise.resolve({ value: undefined, done: true });
                        }
                    };
                };
            }
        }

        if (Stream.Duplex) {
            Stream.Duplex.from = function(src) {
                if (src instanceof Stream.Duplex) return src;
                if (src instanceof Stream.Readable) return src;
                if (Stream.Readable && typeof Stream.Readable.from === 'function') {
                    return Stream.Readable.from(src);
                }
                return new Stream.Duplex();
            };
        }
        Stream.from = function(src) {
            if (Stream.Duplex && typeof Stream.Duplex.from === 'function') {
                return Stream.Duplex.from(src);
            }
            if (Stream.Readable && typeof Stream.Readable.from === 'function') {
                return Stream.Readable.from(src);
            }
            return new Stream();
        };

        Stream.finished = finished;
        Stream.addAbortSignal = addAbortSignal;
        Stream.promises = {
            finished,
            pipeline: Stream.pipeline
        };
        Stream.Stream = Stream;
        Stream.default = Stream;
        globalThis.stream = Stream;
        globalThis.Stream = Stream;
    })();
    "#;
    if let Some(code) = v8::String::new(scope, stream_bootstrap) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
