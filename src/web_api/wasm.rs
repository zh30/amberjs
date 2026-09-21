//! WebAssembly Web API Streaming extensions (compileStreaming, instantiateStreaming).

use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_wasm_streaming_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);
    let wasm_key = v8::String::new(scope, "WebAssembly").unwrap();
    if let Some(wasm_val) = global.get(scope, wasm_key.into()) {
        if wasm_val.is_object() {
            let js_script = r#"
            (function(WebAssembly) {
                if (!WebAssembly) return;
                async function bytesFromSource(source) {
                    const response = await Promise.resolve(source);
                    if (!response) {
                        throw new TypeError("Parameter 1 must be a Response object or a Promise for one");
                    }
                    if (typeof response.ok === 'boolean' && !response.ok) {
                        throw new TypeError("HTTP status code is not ok (status: " + response.status + ")");
                    }
                    if (response instanceof ArrayBuffer || ArrayBuffer.isView(response)) {
                        return response;
                    }
                    if (typeof response.arrayBuffer === 'function') {
                        let buffer = response.arrayBuffer();
                        if (buffer && typeof buffer.then === 'function') {
                            buffer = await buffer;
                        }
                        return buffer;
                    }
                    throw new TypeError("Response body is not a valid ArrayBuffer or Response");
                }
                if (typeof WebAssembly.compileStreaming !== 'function') {
                    WebAssembly.compileStreaming = async function compileStreaming(source) {
                        return new WebAssembly.Module(await bytesFromSource(source));
                    };
                }
                if (typeof WebAssembly.instantiateStreaming !== 'function') {
                    WebAssembly.instantiateStreaming = async function instantiateStreaming(source, importObject) {
                        const buffer = await bytesFromSource(source);
                        const module = new WebAssembly.Module(buffer);
                        const instance = new WebAssembly.Instance(module, importObject);
                        return { module, instance };
                    };
                }
            })(WebAssembly);
            "#;

            let script_src = v8::String::new(scope, js_script).unwrap();
            if let Some(script) = v8::Script::compile(scope, script_src, None) {
                let _ = script.run(scope);
            }
        }
    }
    Ok(())
}
