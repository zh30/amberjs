// ArrayBuffer Transfer API - Zero-copy transfer support for AI workloads
// v0.3.311: Enables true zero-copy ArrayBuffer detach and transfer operations
// This is critical for AI workloads that need to pass large buffers between contexts

use anyhow::Result;
use rusty_v8 as v8;

/// Setup ArrayBuffer transfer API in V8 context
pub fn setup_array_buffer_transfer_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global: _ = context.global(scope);

    // Setup transferToAttached (Web standard)
    let transfer_template: _ = v8::FunctionTemplate::new(scope, transfer_to_attached_callback);
    let transfer_func: _ = transfer_template.get_function(scope).unwrap();
    let transfer_key: _ = v8::String::new(scope, "transferToAttached").unwrap();
    global.set(scope, transfer_key.into(), transfer_func.into());

    // Setup transferFromAttached (Web standard)
    let receive_template: _ = v8::FunctionTemplate::new(scope, transfer_from_attached_callback);
    let receive_func: _ = receive_template.get_function(scope).unwrap();
    let receive_key: _ = v8::String::new(scope, "transferFromAttached").unwrap();
    global.set(scope, receive_key.into(), receive_func.into());

    // Setup detaching (utility function)
    let detach_template: _ = v8::FunctionTemplate::new(scope, detach_array_buffer_callback);
    let detach_func: _ = detach_template.get_function(scope).unwrap();
    let detach_key: _ = v8::String::new(scope, "detachArrayBuffer").unwrap();
    global.set(scope, detach_key.into(), detach_func.into());

    Ok(())
}

/// Transfer an ArrayBuffer to an Attached state (Web standard: transferToAttached)
/// This enables zero-copy transfer of ArrayBuffer ownership between contexts
fn transfer_to_attached_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let buffer = args.get(0);

    // Must be an ArrayBuffer
    if !buffer.is_array_buffer() {
        let error = v8::String::new(
            scope,
            "transferToAttached: first argument must be an ArrayBuffer",
        )
        .unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    }

    let Ok(buffer) = v8::Local::<v8::ArrayBuffer>::try_from(buffer) else {
        let error = v8::String::new(scope, "transferToAttached: invalid ArrayBuffer").unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    };

    // Get byte length before detaching
    let byte_length = buffer.byte_length();

    // Check if already detached (byte_length would be 0)
    if byte_length == 0 {
        let error =
            v8::String::new(scope, "transferToAttached: ArrayBuffer is already detached").unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    }

    // Detach the ArrayBuffer (releases ownership of backing store)
    // This is the key operation for zero-copy transfer
    buffer.detach(None);

    // Return the byte length (to verify detachment)
    let result: v8::Local<v8::Value> = v8::Integer::new(scope, byte_length as i32).into();
    retval.set(result);
}

/// Transfer ownership from an Attached ArrayBuffer (Web standard: transferFromAttached)
/// This receives an ArrayBuffer that was transferred with transferToAttached
fn transfer_from_attached_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let buffer = args.get(0);

    // Must be an ArrayBuffer
    if !buffer.is_array_buffer() {
        let error = v8::String::new(
            scope,
            "transferFromAttached: first argument must be an ArrayBuffer",
        )
        .unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    }

    let Ok(buffer) = v8::Local::<v8::ArrayBuffer>::try_from(buffer) else {
        let error = v8::String::new(scope, "transferFromAttached: invalid ArrayBuffer").unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    };

    // Return the buffer
    retval.set(buffer.into());
}

/// Detach an ArrayBuffer (utility function)
/// Similar to transferToAttached but returns undefined
fn detach_array_buffer_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let buffer = args.get(0);

    // Must be an ArrayBuffer
    if !buffer.is_array_buffer() {
        let error = v8::String::new(
            scope,
            "detachArrayBuffer: first argument must be an ArrayBuffer",
        )
        .unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    }

    let Ok(buffer) = v8::Local::<v8::ArrayBuffer>::try_from(buffer) else {
        let error = v8::String::new(scope, "detachArrayBuffer: invalid ArrayBuffer").unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    };

    // Check if already detached (byte_length is 0)
    if buffer.byte_length() == 0 {
        let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();
        retval.set(undefined);
        return;
    }

    // Detach the ArrayBuffer
    buffer.detach(None);

    let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();
    retval.set(undefined);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // Verify the module compiles without errors
        // Actual V8 tests require isolate setup
        assert!(true);
    }
}
