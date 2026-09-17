// WinterTC Sockets API V8 bindings (TC55 proposal-sockets-api)
// Provides connect() function returning a Socket with ReadableStream and WritableStream.

use anyhow::Result;
use rusty_v8 as v8;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static SOCKETS_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_sockets_runtime() -> &'static Runtime {
    SOCKETS_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("amberjs-sockets-worker")
            .build()
            .expect("Failed to initialize sockets runtime")
    })
}

fn run_async<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    // Never nested-`block_on` or `block_in_place` on the isolate thread: those
    // panic across the V8 FFI (`failed to initiate panic, error 5`).
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    get_sockets_runtime().spawn(async move {
        let output = future.await;
        let _ = tx.send(output);
    });
    rx.recv_timeout(std::time::Duration::from_secs(30))
        .expect("amberjs sockets worker timed out or dropped")
}

pub fn setup_sockets_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Register low-level bridge functions
    let connect_raw_fn = v8::Function::new(scope, socket_connect_raw).unwrap();
    let k_conn = v8::String::new(scope, "__amber_socket_connect").unwrap();
    global.set(scope, k_conn.into(), connect_raw_fn.into());

    let read_raw_fn = v8::Function::new(scope, socket_read_raw).unwrap();
    let k_read = v8::String::new(scope, "__amber_socket_read").unwrap();
    global.set(scope, k_read.into(), read_raw_fn.into());

    let write_raw_fn = v8::Function::new(scope, socket_write_raw).unwrap();
    let k_write = v8::String::new(scope, "__amber_socket_write").unwrap();
    global.set(scope, k_write.into(), write_raw_fn.into());

    let close_raw_fn = v8::Function::new(scope, socket_close_raw).unwrap();
    let k_close = v8::String::new(scope, "__amber_socket_close").unwrap();
    global.set(scope, k_close.into(), close_raw_fn.into());

    let start_tls_raw_fn = v8::Function::new(scope, socket_start_tls_raw).unwrap();
    let k_tls = v8::String::new(scope, "__amber_socket_start_tls").unwrap();
    global.set(scope, k_tls.into(), start_tls_raw_fn.into());

    // Inject high-level Socket and connect() implementation into global
    let sockets_js = r#"
    (function() {
        if (typeof globalThis.connect !== 'undefined') return;

        class Socket {
            constructor(address, options = {}) {
                let hostname, port;
                if (typeof address === 'string') {
                    const parts = address.split(':');
                    hostname = parts[0] || '127.0.0.1';
                    port = parts[1] ? parseInt(parts[1], 10) : (options.secureTransport === 'on' ? 443 : 80);
                } else if (address && typeof address === 'object') {
                    hostname = address.hostname || '127.0.0.1';
                    port = address.port || (options.secureTransport === 'on' ? 443 : 80);
                } else {
                    throw new TypeError('Invalid address provided to connect()');
                }

                this._hostname = hostname;
                this._port = port;
                this._options = options;
                this._socketId = null;
                this._upgraded = false;
                this._isClosed = false;

                let resolveOpened, rejectOpened;
                this.opened = new Promise((resolve, reject) => {
                    resolveOpened = resolve;
                    rejectOpened = reject;
                });

                let resolveClosed, rejectClosed;
                this.closed = new Promise((resolve, reject) => {
                    resolveClosed = resolve;
                    rejectClosed = reject;
                });
                this._resolveClosed = resolveClosed;

                // Establish connection
                __amber_socket_connect(hostname, port, options.secureTransport || 'off', options.sni || null, options.alpn || [])
                    .then(info => {
                        this._socketId = info.id;
                        this._upgraded = (options.secureTransport === 'on');
                        resolveOpened({
                            remoteAddress: info.remoteAddress,
                            localAddress: info.localAddress,
                            alpn: info.alpn
                        });
                    })
                    .catch(err => {
                        this._isClosed = true;
                        rejectOpened(err);
                        rejectClosed(err);
                    });

                // ReadableStream backed by socket read
                this.readable = new ReadableStream({
                    pull: async (controller) => {
                        await this.opened;
                        if (this._isClosed || !this._socketId) {
                            controller.close();
                            return;
                        }
                        try {
                            const chunk = await __amber_socket_read(this._socketId, 65536);
                            if (chunk === null || (chunk && chunk.length === 0)) {
                                controller.close();
                                this._closeInternal();
                            } else {
                                controller.enqueue(chunk);
                            }
                        } catch (err) {
                            controller.error(err);
                            this._closeInternal();
                        }
                    },
                    cancel: async (reason) => {
                        await this.close(reason);
                    }
                });

                // WritableStream backed by socket write
                this.writable = new WritableStream({
                    write: async (chunk) => {
                        await this.opened;
                        if (this._isClosed || !this._socketId) {
                            throw new Error('Socket is closed');
                        }
                        const u8 = chunk instanceof Uint8Array ? chunk : new TextEncoder().encode(String(chunk));
                        await __amber_socket_write(this._socketId, u8);
                    },
                    close: async () => {
                        await this.close();
                    },
                    abort: async (reason) => {
                        await this.close(reason);
                    }
                });
            }

            get upgraded() {
                return this._upgraded;
            }

            async close(reason) {
                if (this._isClosed) return;
                this._isClosed = true;
                if (this._socketId) {
                    try {
                        await __amber_socket_close(this._socketId);
                    } catch (_) {}
                }
                this._resolveClosed();
            }

            _closeInternal() {
                if (this._isClosed) return;
                this._isClosed = true;
                this._resolveClosed();
            }

            async startTls() {
                await this.opened;
                if (this._isClosed || !this._socketId) {
                    throw new Error('startTls() requires an opened socket');
                }
                if (this._upgraded) {
                    return this;
                }
                await __amber_socket_start_tls(this._socketId, this._options.sni || this._hostname);
                this._upgraded = true;
                return this;
            }
        }

        function connect(address, options) {
            return new Socket(address, options);
        }

        globalThis.Socket = Socket;
        globalThis.connect = connect;

        const socketsModule = { connect, Socket, default: { connect, Socket } };
        globalThis.__amber_sockets = socketsModule;
        globalThis.__sockets = socketsModule;
        globalThis.sockets = socketsModule;
    })();
    "#;

    if let Some(code) = v8::String::new(scope, sockets_js) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}

fn socket_connect_raw(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let host = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let port = args.get(1).int32_value(scope).unwrap_or(80) as u16;
    let secure_transport = args
        .get(2)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_else(|| "off".to_string());
    let sni = if args.get(3).is_string() {
        Some(
            args.get(3)
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope),
        )
    } else {
        None
    };

    let host_clone = host.clone();
    let sec_clone = secure_transport.clone();
    let sni_clone = sni.clone();
    let local_res = run_async(async move {
        crate::sockets::connect_socket(
            &host_clone,
            port,
            &sec_clone,
            sni_clone.as_deref(),
            Vec::new(),
        )
        .await
    });

    match local_res {
        Ok((id, info)) => {
            let info_obj = v8::Object::new(scope);
            let k_id = v8::String::new(scope, "id").unwrap();
            let v_id = v8::Number::new(scope, id as f64);
            info_obj.set(scope, k_id.into(), v_id.into());

            let k_rem = v8::String::new(scope, "remoteAddress").unwrap();
            let v_rem = v8::String::new(scope, &info.remote_address).unwrap();
            info_obj.set(scope, k_rem.into(), v_rem.into());

            let k_loc = v8::String::new(scope, "localAddress").unwrap();
            let v_loc = v8::String::new(scope, &info.local_address).unwrap();
            info_obj.set(scope, k_loc.into(), v_loc.into());

            let k_alpn = v8::String::new(scope, "alpn").unwrap();
            if let Some(alpn) = info.alpn {
                let v_alpn = v8::String::new(scope, &alpn).unwrap();
                info_obj.set(scope, k_alpn.into(), v_alpn.into());
            } else {
                let null_val = v8::null(scope);
                info_obj.set(scope, k_alpn.into(), null_val.into());
            }

            resolver.resolve(scope, info_obj.into());
        }
        Err(e) => {
            let err_str = v8::String::new(scope, &e.to_string()).unwrap();
            let err = v8::Exception::error(scope, err_str);
            resolver.reject(scope, err);
        }
    }
}

fn socket_read_raw(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let id = args.get(0).number_value(scope).unwrap_or(0.0) as u64;
    let max_bytes = args.get(1).int32_value(scope).unwrap_or(65536) as usize;

    let res = run_async(async move { crate::sockets::read_socket(id, max_bytes).await });

    match res {
        Ok(Some(bytes)) => {
            let buffer = v8::ArrayBuffer::new(scope, bytes.len());
            if !bytes.is_empty() {
                let store = buffer.get_backing_store();
                let ptr = store.as_ref().as_ptr() as *mut u8;
                if !ptr.is_null() {
                    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, bytes.len()) };
                    slice.copy_from_slice(&bytes);
                }
            }
            let u8_arr = v8::Uint8Array::new(scope, buffer, 0, bytes.len()).unwrap();
            resolver.resolve(scope, u8_arr.into());
        }
        Ok(None) => {
            let null_val = v8::null(scope);
            resolver.resolve(scope, null_val.into());
        }
        Err(e) => {
            let err_str = v8::String::new(scope, &e.to_string()).unwrap();
            let err = v8::Exception::error(scope, err_str);
            resolver.reject(scope, err);
        }
    }
}

fn socket_write_raw(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let id = args.get(0).number_value(scope).unwrap_or(0.0) as u64;
    let chunk_val = args.get(1);

    let bytes = if chunk_val.is_uint8_array() {
        let u8_arr: v8::Local<v8::Uint8Array> = v8::Local::cast(chunk_val);
        let mut buffer = vec![0u8; u8_arr.byte_length()];
        u8_arr.copy_contents(&mut buffer);
        buffer
    } else if chunk_val.is_string() {
        chunk_val
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
            .into_bytes()
    } else {
        Vec::new()
    };

    let res = run_async(async move { crate::sockets::write_socket(id, &bytes).await });

    match res {
        Ok(n) => {
            let v_n = v8::Integer::new(scope, n as i32);
            resolver.resolve(scope, v_n.into());
        }
        Err(e) => {
            let err_str = v8::String::new(scope, &e.to_string()).unwrap();
            let err = v8::Exception::error(scope, err_str);
            resolver.reject(scope, err);
        }
    }
}

fn socket_close_raw(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let id = args.get(0).number_value(scope).unwrap_or(0.0) as u64;

    let res = run_async(async move { crate::sockets::close_socket(id).await });

    match res {
        Ok(_) => {
            let undef_val = v8::undefined(scope);
            resolver.resolve(scope, undef_val.into());
        }
        Err(e) => {
            let err_str = v8::String::new(scope, &e.to_string()).unwrap();
            let err = v8::Exception::error(scope, err_str);
            resolver.reject(scope, err);
        }
    }
}

fn socket_start_tls_raw(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    retval.set(promise.into());

    let id = args.get(0).number_value(scope).unwrap_or(0.0) as u64;
    let sni = if args.length() > 1 && args.get(1).is_string() {
        Some(
            args.get(1)
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope),
        )
    } else {
        None
    };

    let res = run_async(async move { crate::sockets::start_tls_socket(id, sni.as_deref()).await });

    match res {
        Ok(_) => {
            let undef_val = v8::undefined(scope);
            resolver.resolve(scope, undef_val.into());
        }
        Err(e) => {
            let err_str = v8::String::new(scope, &e.to_string()).unwrap();
            let err = v8::Exception::error(scope, err_str);
            resolver.reject(scope, err);
        }
    }
}
