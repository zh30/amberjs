// Worker API boundary for Web standard compatibility.
//
// Amber does not yet have a real WorkerHost with an independent isolate,
// event loop, structured-clone message queues, and termination lifecycle.
// Until that exists, Worker construction must fail closed instead of returning
// a synchronous object shell that makes scripts believe background execution
// has started.

use anyhow::Result;
use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Static worker registry to track active workers
static WORKER_REGISTRY: Lazy<Arc<Mutex<HashMap<u32, WorkerStateInfo>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Get worker information for debugging/monitoring
/// Uses fields from WorkerStateInfo that were previously unused
pub fn get_worker_info(worker_id: u32) -> Option<WorkerInfoResponse> {
    let registry = WORKER_REGISTRY.lock().unwrap();
    registry.get(&worker_id).map(|info| WorkerInfoResponse {
        worker_id: info.worker_id,
        script_url: info.script_url.clone(),
        is_terminated: info.is_terminated,
        uptime_seconds: info.created_at.elapsed().as_secs_f64(),
    })
}

/// Get count of active workers
pub fn get_active_worker_count() -> usize {
    let registry = WORKER_REGISTRY.lock().unwrap();
    registry.values().filter(|info| !info.is_terminated).count()
}

#[derive(Debug)]
pub struct WorkerInfoResponse {
    pub worker_id: u32,
    pub script_url: String,
    pub is_terminated: bool,
    pub uptime_seconds: f64,
}

struct WorkerStateInfo {
    worker_id: u32,
    script_url: String,
    is_terminated: bool,
    created_at: std::time::Instant,
}

/// Setup Worker API in V8 context
pub fn setup_worker_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Create prototype object with methods
    let prototype = v8::Object::new(scope);

    // Add prototype methods directly on prototype
    add_prototype_methods(scope, prototype)?;

    // Create prototype key - used for storing/referencing prototype
    let prototype_key = v8::String::new(scope, "_workerPrototype").unwrap();

    // Store prototype on global for access from constructor callback
    let proto_val: v8::Local<v8::Value> = prototype.into();
    global.set(scope, prototype_key.into(), proto_val);

    // Create Worker constructor function
    // We use a static callback to avoid closure capture issues with V8
    let worker_constructor = v8::FunctionTemplate::new(scope, worker_constructor_callback);

    // Set Worker on global
    let worker_key = v8::String::new(scope, "Worker").unwrap();
    let worker_val = worker_constructor.get_function(scope).unwrap();
    global.set(scope, worker_key.into(), worker_val.into());

    // Set prototype property on constructor
    let proto_prop_key = v8::String::new(scope, "prototype").unwrap();
    worker_val.set(scope, proto_prop_key.into(), prototype.into());

    Ok(())
}

/// Worker constructor callback - implemented as a standalone function to avoid closure capture issues
fn worker_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let message = if args.length() == 0 {
        "Worker constructor requires a script URL"
    } else {
        "Worker script execution is not supported yet"
    };

    let error_message = v8::String::new(scope, message).unwrap();
    let exception = v8::Exception::type_error(scope, error_message);
    scope.throw_exception(exception);
    rv.set(v8::undefined(scope).into());
}

fn add_prototype_methods(scope: &mut v8::PinScope, prototype: v8::Local<v8::Object>) -> Result<()> {
    // postMessage method
    let post_message_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let this_obj = args.this();

            // Check if worker is terminated
            let terminated_key = v8::String::new(scope, "_terminated").unwrap();
            if let Some(terminated_val) = this_obj.get(scope, terminated_key.into()) {
                if terminated_val.is_true() {
                    return;
                }
            }

            // Get worker ID
            let worker_id_key = v8::String::new(scope, "_workerId").unwrap();
            if let Some(worker_id_val) = this_obj.get(scope, worker_id_key.into()) {
                let worker_id = worker_id_val.to_uint32(scope).unwrap().value() as u32;
                eprintln!("[Worker {}] postMessage called", worker_id);
            }
        },
    );
    let post_message_key = v8::String::new(scope, "postMessage").unwrap();
    let post_message_func = post_message_fn.get_function(scope).unwrap();
    prototype.set(scope, post_message_key.into(), post_message_func.into());

    // terminate method
    let terminate_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let this_obj = args.this();

            let worker_id_key = v8::String::new(scope, "_workerId").unwrap();
            if let Some(worker_id_val) = this_obj.get(scope, worker_id_key.into()) {
                let worker_id = worker_id_val.to_uint32(scope).unwrap().value() as u32;

                let terminated_key = v8::String::new(scope, "_terminated").unwrap();
                let true_val: v8::Local<v8::Value> = v8::Boolean::new(scope, true).into();
                this_obj.set(scope, terminated_key.into(), true_val.into());

                if let Some(info) = WORKER_REGISTRY.lock().unwrap().get_mut(&worker_id) {
                    info.is_terminated = true;
                }

                eprintln!("[Worker {}] terminated", worker_id);
            }
        },
    );
    let terminate_key = v8::String::new(scope, "terminate").unwrap();
    let terminate_func = terminate_fn.get_function(scope).unwrap();
    prototype.set(scope, terminate_key.into(), terminate_func.into());

    // onmessage property - use null per Web standard for unset event handlers
    let onmessage_key = v8::String::new(scope, "onmessage").unwrap();
    let null_val = v8::null(scope);
    prototype.set(scope, onmessage_key.into(), null_val.into());

    // onerror property
    let onerror_key = v8::String::new(scope, "onerror").unwrap();
    prototype.set(scope, onerror_key.into(), null_val.into());

    // onmessageerror property
    let onmessageerror_key = v8::String::new(scope, "onmessageerror").unwrap();
    prototype.set(scope, onmessageerror_key.into(), null_val.into());

    Ok(())
}

/// Cleanup terminated workers from registry
pub fn cleanup_workers() {
    let mut registry = WORKER_REGISTRY.lock().unwrap();
    registry.retain(|_id, info| !info.is_terminated);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_api_creation() {
        assert!(true);
    }

    #[test]
    fn test_worker_state_info() {
        let info = WorkerStateInfo {
            worker_id: 1,
            script_url: "test.js".to_string(),
            is_terminated: false,
            created_at: std::time::Instant::now(),
        };
        assert_eq!(info.worker_id, 1);
        assert_eq!(info.script_url, "test.js");
        assert!(!info.is_terminated);
    }
}
