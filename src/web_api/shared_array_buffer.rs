// SharedArrayBuffer API for cross-Worker shared memory
// v0.3.322: Shared memory support for AI workloads requiring multi-threaded data access
// Enables true shared memory between Workers without serialization overhead

use anyhow::Result;
use rusty_v8 as v8;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Track SharedArrayBuffer allocations for memory management
static SHARED_BUFFER_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Get the number of active SharedArrayBuffers
pub fn get_shared_buffer_count() -> usize {
    SHARED_BUFFER_COUNT.load(Ordering::SeqCst)
}

/// Setup SharedArrayBuffer API in V8 context
/// Note: SharedArrayBuffer is typically built-in to V8, this function ensures it's exposed
pub fn setup_shared_array_buffer_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Get the built-in SharedArrayBuffer constructor from V8
    let shared_buffer_key = v8::String::new(scope, "SharedArrayBuffer").unwrap();

    // Check if SharedArrayBuffer is already available as a built-in
    if let Some(shared_buffer_val) = global.get(scope, shared_buffer_key.into()) {
        if shared_buffer_val.is_function() {
            return Ok(());
        }
    }

    // If not available as built-in, create a wrapper using V8's SharedArrayBuffer
    // V8 exposes SharedArrayBuffer through the Isolate when enabled
    let shared_buffer_constructor = v8::FunctionTemplate::new(scope, shared_array_buffer_callback);

    let shared_buffer_func = shared_buffer_constructor.get_function(scope).unwrap();
    global.set(scope, shared_buffer_key.into(), shared_buffer_func.into());

    Ok(())
}

/// SharedArrayBuffer constructor callback
/// Creates a new SharedArrayBuffer with the specified byte length
fn shared_array_buffer_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let first = args.get(0);

    // SharedArrayBuffer requires a byte length
    if !first.is_number() {
        let error = v8::String::new(scope, "SharedArrayBuffer: byteLength is required").unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    }

    let size = first.to_integer(scope).unwrap().value() as usize;

    // Validate size
    if size == 0 {
        // Create empty SharedArrayBuffer (allowed per spec)
        if let Some(buffer) = v8::SharedArrayBuffer::new(scope, 0) {
            SHARED_BUFFER_COUNT.fetch_add(1, Ordering::SeqCst);
            retval.set(buffer.into());
        } else {
            let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();
            retval.set(undefined);
        }
        return;
    }

    // Check for reasonable size (prevent excessive allocation)
    const MAX_SIZE: usize = 1024 * 1024 * 1024; // 1GB limit
    if size > MAX_SIZE {
        let error = v8::String::new(
            scope,
            &format!(
                "SharedArrayBuffer: size {} exceeds maximum allowed ({})",
                size, MAX_SIZE
            ),
        )
        .unwrap();
        let exception = v8::Exception::range_error(scope, error);
        scope.throw_exception(exception);
        return;
    }

    // Create the SharedArrayBuffer
    if let Some(buffer) = v8::SharedArrayBuffer::new(scope, size) {
        // Track allocation
        SHARED_BUFFER_COUNT.fetch_add(1, Ordering::SeqCst);
        retval.set(buffer.into());
    } else {
        // Allocation failed
        let error = v8::String::new(scope, "SharedArrayBuffer: allocation failed").unwrap();
        let exception = v8::Exception::range_error(scope, error);
        scope.throw_exception(exception);
    }
}

/// Get SharedArrayBuffer byte length
pub fn get_shared_buffer_byte_length(buffer: v8::Local<v8::SharedArrayBuffer>) -> usize {
    buffer.byte_length()
}

/// Check if a value is a SharedArrayBuffer
pub fn is_shared_array_buffer(value: v8::Local<v8::Value>) -> bool {
    value.is_shared_array_buffer()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_buffer_count_initial() {
        // Verify the counter starts at 0
        assert_eq!(get_shared_buffer_count(), 0);
    }

    // Note: is_shared_array_buffer tests require V8 isolate setup
    // See shared_array_buffer_tests.rs for full integration tests
}
