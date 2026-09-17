// Web API modules - Stage 43.0 / Stage 74.0
pub mod abort;
pub mod array_buffer_transfer; // v0.3.311: ArrayBuffer transfer (zero-copy detach)
pub mod background_sync; // v0.3.327: Background Sync API (SyncManager, SyncEvent)
pub mod blob; // Stage 74: Blob/File API
pub mod broadcast_channel; // v0.3.312: BroadcastChannel API (cross-tab communication)
pub mod clipboard; // v0.3.342: Clipboard API (copy/paste for AI workloads)
pub mod compression; // v0.3.295: CompressionStream API (gzip/deflate)
pub mod crypto;
pub mod custom_event; // v0.3.337: CustomEvent API (custom event handling)
pub mod dom_exception; // WinterTC ECMA-429: DOMException
pub mod dom_parser; // v0.3.341: DOMParser API (HTML/XML document parsing for AI workloads)
pub mod encoding; // Stage 74: TextEncoder/TextDecoder
pub mod error_event; // v0.3.333: ErrorEvent API (script error handling)
pub mod events;
/// Web标准API实现
pub mod fetch;
pub mod form_data;
pub mod message_channel; // v0.3.315: MessageChannel API (port-based communication)
pub mod navigator; // WinterTC ECMA-429: navigator, userAgent
pub mod notification; // v0.3.328: Notification API (system notifications)
pub mod payment_request; // v0.3.328: Payment Request API (payment processing)
pub mod performance; // Stage 74: Performance API
pub mod service_worker; // v0.3.324: ServiceWorker API (background tasks, push, offline)
pub mod shared_array_buffer; // v0.3.322: SharedArrayBuffer API (cross-Worker shared memory)
pub mod sockets; // WinterTC Sockets API (connect, Socket)
pub mod streams; // Stage 75: Web Streams API for AI workloads
pub mod structured_clone; // v0.3.299: structuredClone global function
pub mod timers; // Stage 74: Timer APIs
pub mod url;
pub mod url_pattern; // WinterTC ECMA-429: URLPattern
pub mod url_search_params;
pub mod wasm;
pub mod websocket;
pub mod worker; // v0.3.320: Worker API (Web Worker support for parallel execution) // v0.3.353: URLSearchParams API (query string manipulation)
pub mod worker_host; // Multi-isolate WorkerHost // WebAssembly streaming APIs (compileStreaming, instantiateStreaming)
use anyhow::Result;
use rusty_v8 as v8;
// 从各模块导入设置函数
use abort::setup_abort_api;
use array_buffer_transfer::setup_array_buffer_transfer_api;
use background_sync::setup_background_sync_api;
use blob::setup_blob_api;
use broadcast_channel::setup_broadcast_channel_api;
use clipboard::setup_clipboard_api;
use compression::setup_compression_api;
use crypto::setup_crypto_api;
use custom_event::setup_custom_event_api;
use dom_exception::setup_dom_exception_api;
use dom_parser::setup_dom_parser_api;
use encoding::setup_encoding_api;
use error_event::setup_error_event_api;
use events::setup_events_api;
use fetch::setup_fetch_api;
use form_data::setup_form_data_api;
use message_channel::setup_message_channel_api;
use navigator::setup_navigator_api;
use notification::setup_notification_api;
use payment_request::setup_payment_request_api;
use performance::setup_performance_api;
use service_worker::setup_service_worker_api;
use shared_array_buffer::setup_shared_array_buffer_api;
use sockets::setup_sockets_api;
use structured_clone::setup_structured_clone_api;
use timers::setup_timer_api;
use url::setup_url_api;
use url_pattern::setup_url_pattern_api;
use url_search_params::setup_url_search_params_api;
use websocket::setup_websocket_api;
use worker::setup_worker_api;
#[allow(unused_imports)]
use worker_host::setup_worker_host_api;
/// 初始化所有 Web API 到 V8 上下文
pub fn init_web_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);
    // 按照依赖顺序初始化各个 API
    // 1. 基础 API（无依赖）
    setup_crypto_api(scope, context)?;
    setup_events_api(scope, context)?;
    setup_abort_api(scope, context)?;
    setup_blob_api(scope, context)?; // Stage 74: Blob/File API
    setup_timer_api(scope, context)?;
    setup_encoding_api(scope, context)?; // Stage 74: Encoding APIs
    setup_performance_api(scope, context)?; // Stage 74: Performance API
                                            // 2. URL API（依赖 events）
    setup_url_api(scope, context)?;
    // v0.3.353: URLSearchParams API (query string manipulation)
    setup_url_search_params_api(scope, context);
    // 3. FormData API（依赖 URL）
    setup_form_data_api(scope, context)?;
    // 4. Fetch API（依赖 URL, Headers, Request, Response, Abort）
    setup_fetch_api(scope, context)?;
    // 5. WebSocket API（依赖 Events）
    setup_websocket_api(scope, context)?;
    // v0.3.295: CompressionStream API（依赖 Web Streams）
    setup_compression_api(scope, context)?;
    // v0.3.299: structuredClone global function
    setup_structured_clone_api(scope, context)?;
    // v0.3.311: ArrayBuffer transfer API (zero-copy detach for AI workloads)
    setup_array_buffer_transfer_api(scope, context)?;
    // v0.3.312: BroadcastChannel API (cross-tab communication)
    setup_broadcast_channel_api(scope, context)?;
    // v0.3.315: MessageChannel API (port-based communication)
    setup_message_channel_api(scope, context)?;
    // v0.3.320: Worker API (Web Worker support for parallel execution)
    setup_worker_api(scope, context)?;
    // v0.3.322: SharedArrayBuffer API (cross-Worker shared memory)
    setup_shared_array_buffer_api(scope, context)?;
    // v0.3.324: ServiceWorker API (background tasks, push notifications, offline caching)
    setup_service_worker_api(scope, context)?;
    // v0.3.327: Background Sync API (SyncManager, SyncEvent)
    setup_background_sync_api(scope, context)?;
    // v0.3.328: Notification API (system notifications)
    setup_notification_api(scope, context, global)?;
    // v0.3.328: Payment Request API (payment processing)
    setup_payment_request_api(scope, context, global)?;
    // v0.3.333: ErrorEvent API (script error handling for WebSocket, Worker, etc.)
    setup_error_event_api(scope, context);
    // v0.3.337: CustomEvent API (custom event handling for AI agents and UI frameworks)
    setup_custom_event_api(scope, context);
    // v0.3.341: DOMParser API (HTML/XML document parsing for AI workloads)
    setup_dom_parser_api(scope, context)?;
    // v0.3.342: Clipboard API (copy/paste for AI workloads)
    setup_clipboard_api(scope, context)?;
    // WebAssembly streaming APIs (compileStreaming, instantiateStreaming)
    wasm::setup_wasm_streaming_api(scope, context)?;

    // WinterTC ECMA-429 & Sockets API initialization
    setup_dom_exception_api(scope, context)?;
    setup_navigator_api(scope, context)?;
    setup_url_pattern_api(scope, context)?;
    setup_sockets_api(scope, context)?;

    // WinterTC ECMA-429 Section 5.2 / 6 global aliases and error reporting
    let wintertc_helpers_js = r#"
    (function() {
        // globalThis.self requirement
        if (typeof globalThis.self === 'undefined') {
            globalThis.self = globalThis;
        }

        // reportError requirement
        if (typeof globalThis.reportError !== 'function') {
            globalThis.reportError = function(error) {
                if (typeof globalThis.onerror === 'function') {
                    try {
                        const msg = (error && error.message) ? error.message : String(error);
                        globalThis.onerror(msg, '', 0, 0, error);
                        return;
                    } catch (_) {}
                }
                console.error('Unhandled error:', error);
            };
        }

        // PromiseRejectionEvent
        if (typeof globalThis.PromiseRejectionEvent === 'undefined') {
            const Base = (typeof Event === 'function') ? Event : Object;
            globalThis.PromiseRejectionEvent = class PromiseRejectionEvent extends Base {
                constructor(type, init = {}) {
                    try { super(type, init); } catch (_) { super(); }
                    this.type = type;
                    this.promise = init.promise;
                    this.reason = init.reason;
                }
            };
        }

        globalThis.__amber_dispatch_unhandled_rejection = function(promise, reason) {
            let event;
            try {
                event = new PromiseRejectionEvent('unhandledrejection', {
                    promise: promise,
                    reason: reason,
                    cancelable: true
                });
            } catch (_) {
                event = { type: 'unhandledrejection', promise: promise, reason: reason };
            }
            try {
                if (typeof globalThis.onunhandledrejection === 'function') {
                    globalThis.onunhandledrejection(event);
                }
            } catch (_) {}
            try {
                if (typeof globalThis.dispatchEvent === 'function') {
                    globalThis.dispatchEvent(event);
                }
            } catch (_) {}
        };
    })();
    "#;
    if let Some(code) = v8::String::new(scope, wintertc_helpers_js) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    // Note: Streams API is initialized separately in runtime_minimal.rs
    // to avoid duplicate initialization
    Ok(())
}
