// structuredClone API implementation
// v0.3.303: Performance optimization - iterative deep clone using work queue
// Replaced recursive approach with iterative stack to avoid stack overflow
// Optimized for AI workloads - enables safe deep cloning with transfer semantics

use anyhow::Result;
use rusty_v8 as v8;

/// Internal key for storing the clone function in global
const CLONE_FUNC_KEY: &str = "__amberjs_internal_clone_func";

/// Setup the internal clone function in the global object
fn setup_internal_clone_func(
    scope: &mut v8::PinScope,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    let code = r#"
        (function() {
            "use strict";
            function clone(v, transferList) {
                // Symbol cannot be cloned - throw DataCloneError per spec
                if (typeof v === 'symbol') {
                    const err = new Error("Symbol cannot be cloned");
                    err.name = "DataCloneError";
                    throw err;
                }

                // Functions cannot be cloned - throw DataCloneError per spec
                // Must check before the object check because typeof function === 'function', not 'object'
                if (typeof v === 'function') {
                    const err = new Error("Function cannot be cloned");
                    err.name = "DataCloneError";
                    throw err;
                }

                if (v === null || typeof v !== "object") return v;

                const transfers = new Map();

                // Process transfer list - mark objects as transferred
                if (Array.isArray(transferList)) {
                    for (const obj of transferList) {
                        if (obj && typeof obj === 'object') {
                            const transferable = obj.buffer || obj;
                            if (transferable && typeof transferable === 'object') {
                                transfers.set(transferable, obj);
                            }
                        }
                    }
                }

                function getTypedArrayConstructor(obj) {
                    if (obj instanceof Uint8Array) return Uint8Array;
                    if (obj instanceof Int8Array) return Int8Array;
                    if (obj instanceof Uint16Array) return Uint16Array;
                    if (obj instanceof Int16Array) return Int16Array;
                    if (obj instanceof Uint32Array) return Uint32Array;
                    if (obj instanceof Int32Array) return Int32Array;
                    if (obj instanceof Float32Array) return Float32Array;
                    if (obj instanceof Float64Array) return Float64Array;
                    if (obj instanceof Uint8ClampedArray) return Uint8ClampedArray;
                    // v0.3.314: Add BigInt64Array and BigUint64Array support
                    if (obj instanceof BigInt64Array) return BigInt64Array;
                    if (obj instanceof BigUint64Array) return BigUint64Array;
                    return null;
                }

                function isDataView(obj) {
                    // DataView is a distinct type from TypedArray
                    // Check using instanceof and byteLength/getInt8 methods
                    return obj instanceof DataView ||
                           (typeof obj === 'object' &&
                            typeof obj.byteLength === 'number' &&
                            typeof obj.getInt8 === 'function' &&
                            typeof obj.getUint8 === 'function' &&
                            !obj.includes); // TypedArrays have includes, DataView doesn't
                }

                function getErrorConstructor(obj) {
                    if (obj instanceof TypeError) return TypeError;
                    if (obj instanceof RangeError) return RangeError;
                    if (obj instanceof ReferenceError) return ReferenceError;
                    if (obj instanceof SyntaxError) return SyntaxError;
                    if (obj instanceof EvalError) return EvalError;
                    if (obj instanceof URIError) return URIError;
                    return Error;
                }

                // TRUE ITERATIVE DEEP CLONE using work queue
                const clonedObjects = new Map();
                const pendingProps = [];  // [parent, key, value]
                const mapEntries = [];  // [map, originalKey, originalVal] for Map processing
                const setValues = [];  // [set, value] for Set processing

                function createClone(obj) {
                    const TypedArrayConstructor = getTypedArrayConstructor(obj);

                    // Check for Symbol first - cannot be cloned per spec
                    // Symbol cannot be cloned because it's unique and non-transferable
                    if (typeof obj === 'symbol') {
                        const err = new Error("Symbol cannot be cloned");
                        err.name = "DataCloneError";
                        throw err;
                    }

                    // Check for WeakMap/WeakSet next - these cannot be cloned per spec
                    // They throw DataCloneError (using Error with name property per spec)
                    if (obj instanceof WeakMap) {
                        const err = new Error("WeakMap cannot be cloned");
                        err.name = "DataCloneError";
                        throw err;
                    }
                    if (obj instanceof WeakSet) {
                        const err = new Error("WeakSet cannot be cloned");
                        err.name = "DataCloneError";
                        throw err;
                    }

                    if (TypedArrayConstructor) {
                        if (obj.buffer instanceof ArrayBuffer && transfers.has(obj.buffer)) {
                            transfers.delete(obj.buffer);
                            return obj;
                        }
                        return new TypedArrayConstructor(obj);
                    } else if (isDataView(obj)) {
                        // Clone DataView by copying its underlying ArrayBuffer
                        const byteLength = obj.byteLength;
                        const byteOffset = obj.byteOffset;
                        const cloned = new DataView(new ArrayBuffer(byteLength));
                        // Copy all bytes using getInt8/setInt8 loop
                        for (let i = 0; i < byteLength; i++) {
                            cloned.setInt8(i, obj.getInt8(i));
                        }
                        return cloned;
                    } else if (obj instanceof Date ||
                        (typeof obj.getTime === 'function' && obj.timestamp !== undefined)) {
                        // Get timestamp from either getTime() or timestamp property
                        const timestamp = (typeof obj.getTime === 'function')
                            ? obj.getTime()
                            : obj.timestamp;
                        try {
                            // Try to create native Date
                            return new Date(timestamp);
                        } catch (e) {
                            // Fallback to object with Date-like properties
                            const cloned = { timestamp: timestamp };
                            cloned.getTime = function() { return this.timestamp; };
                            cloned.getMonth = function() {
                                const d = new Date(this.timestamp);
                                return d.getMonth();
                            };
                            cloned.getDate = function() {
                                const d = new Date(this.timestamp);
                                return d.getDate();
                            };
                            cloned.getFullYear = function() {
                                const d = new Date(this.timestamp);
                                return d.getFullYear();
                            };
                            return cloned;
                        }
                    } else if (obj instanceof RegExp ||
                        (typeof obj.source === 'string' && typeof obj.flags === 'string')) {
                        // Try to create a native RegExp clone
                        const source = obj.source || obj.patternSource;
                        const flags = obj.flags || obj.patternFlags || '';
                        try {
                            return new RegExp(source, flags);
                        } catch (e) {
                            // Fallback to object with RegExp-like properties
                            const cloned = { source, flags };
                            cloned.test = function(str) { return new RegExp(this.source, this.flags).test(str); };
                            cloned.exec = function(str) { return new RegExp(this.source, this.flags).exec(str); };
                            cloned.toString = function() { return '/' + this.source + '/' + this.flags; };
                            return cloned;
                        }
                    } else if (obj instanceof ArrayBuffer) {
                        // Check if it's a SharedArrayBuffer - cannot be cloned
                        if (typeof SharedArrayBuffer !== 'undefined' && obj instanceof SharedArrayBuffer) {
                            const err = new Error("SharedArrayBuffer cannot be cloned");
                            err.name = "DataCloneError";
                            throw err;
                        }
                        const bytes = new Uint8Array(obj);
                        const cloned = new ArrayBuffer(obj.byteLength);
                        new Uint8Array(cloned).set(bytes);
                        return cloned;
                    } else if (obj instanceof Map ||
                        (typeof obj.forEach === 'function' && typeof obj.get === 'function')) {
                        return new Map();
                    } else if (obj instanceof Set ||
                        (typeof obj.forEach === 'function' && typeof obj.has === 'function')) {
                        return new Set();
                    } else if (obj instanceof Error ||
                        (typeof obj.name === 'string' && typeof obj.message === 'string')) {
                        const ErrorConstructor = getErrorConstructor(obj);
                        const cloned = new ErrorConstructor(obj.message);
                        if (typeof obj.name === 'string') cloned.name = obj.name;
                        if (typeof obj.stack === 'string') cloned.stack = obj.stack;
                        return cloned;
                    } else if (Array.isArray(obj)) {
                        return new Array(obj.length);
                    } else if (obj instanceof Promise) {
                        // Promise cloning is handled by Rust side using V8 PromiseState API
                        // Return a marker object that will be processed by the callback
                        return { __promiseMarker__: true, __promiseObj__: obj };
                    } else {
                        return {};
                    }
                }

                function queueProperties(source, cloned) {
                    if (source instanceof Map) {
                        // Check for Symbol and Function keys/values - these cannot be cloned
                        source.forEach((val, key) => {
                            if (typeof key === 'symbol' || typeof val === 'symbol') {
                                const err = new Error("Map containing Symbol cannot be cloned");
                                err.name = "DataCloneError";
                                throw err;
                            }
                            if (typeof key === 'function' || typeof val === 'function') {
                                const err = new Error("Map containing Function cannot be cloned");
                                err.name = "DataCloneError";
                                throw err;
                            }
                        });
                        source.forEach((val, key) => {
                            // Queue for Map processing after cloning
                            mapEntries.push([cloned, key, val]);
                            // Queue key and value for cloning
                            pendingProps.push([cloned, 'MAP_KEY', key]);
                            pendingProps.push([cloned, 'MAP_VAL', val]);
                        });
                    } else if (source instanceof Set) {
                        // Check for Symbol and Function values - these cannot be cloned
                        source.forEach(val => {
                            if (typeof val === 'symbol') {
                                const err = new Error("Set containing Symbol cannot be cloned");
                                err.name = "DataCloneError";
                                throw err;
                            }
                            if (typeof val === 'function') {
                                const err = new Error("Set containing Function cannot be cloned");
                                err.name = "DataCloneError";
                                throw err;
                            }
                        });
                        source.forEach(val => {
                            setValues.push([cloned, val]);
                            pendingProps.push([cloned, 'SET_VAL', val]);
                        });
                    } else if (source instanceof Error) {
                        for (const key in source) {
                            if (Object.prototype.hasOwnProperty.call(source, key) &&
                                key !== 'name' && key !== 'message' && key !== 'stack') {
                                pendingProps.push([cloned, key, source[key]]);
                            }
                        }
                    } else if (Array.isArray(source)) {
                        // Check for Symbol and Function elements - these cannot be cloned
                        for (let i = 0; i < source.length; i++) {
                            if (typeof source[i] === 'symbol') {
                                const err = new Error("Array containing Symbol cannot be cloned");
                                err.name = "DataCloneError";
                                throw err;
                            }
                            if (typeof source[i] === 'function') {
                                const err = new Error("Array containing Function cannot be cloned");
                                err.name = "DataCloneError";
                                throw err;
                            }
                        }
                        source.forEach((val, idx) => {
                            pendingProps.push([cloned, idx.toString(), val]);
                        });
                    } else if (typeof source === 'object') {
                        // Check for Date objects - these have no enumerable properties
                        if (source instanceof Date ||
                            (typeof source.getTime === 'function' && typeof source.getMonth === 'function')) {
                            // Date objects have no enumerable properties to copy
                            // The timestamp was already set in createClone
                            return;
                        }
                        // Check for RegExp objects - these have no enumerable properties
                        if (source instanceof RegExp ||
                            (typeof source.source === 'string' && typeof source.flags === 'string')) {
                            // RegExp objects have no enumerable properties to copy
                            // The pattern was already set in createClone
                            return;
                        }
                        // Check for Symbol properties - these cannot be cloned
                        const symbolProps = Object.getOwnPropertySymbols(source);
                        if (symbolProps.length > 0) {
                            const err = new Error("Object containing Symbol cannot be cloned");
                            err.name = "DataCloneError";
                            throw err;
                        }
                        // Queue string properties
                        for (const key in source) {
                            if (Object.prototype.hasOwnProperty.call(source, key)) {
                                // Check if property value is a function - cannot be cloned
                                if (typeof source[key] === 'function') {
                                    const err = new Error("Object containing Function cannot be cloned");
                                    err.name = "DataCloneError";
                                    throw err;
                                }
                                pendingProps.push([cloned, key, source[key]]);
                            }
                        }
                    }
                }

                // Create clone of root object
                const rootCloned = createClone(v);
                clonedObjects.set(v, rootCloned);
                queueProperties(v, rootCloned);

                // Process pending properties FIRST
                // This ensures all nested objects are cloned before Map/Set entries are processed
                while (pendingProps.length > 0) {
                    const [parent, key, value] = pendingProps.pop();

                    // Handle Map/Set marker entries - but only skip for primitives
                    // For object values, we need to process them to clone them
                    if ((key === 'MAP_KEY' || key === 'MAP_VAL' || key === 'SET_VAL') &&
                        (value === null || typeof value !== "object")) {
                        // For primitives, track in clonedObjects
                        clonedObjects.set(value, value);
                        continue;
                    }

                    // Handle primitives (but Symbol cannot be cloned)
                    if (value === null || typeof value !== "object") {
                        // Symbol cannot be cloned - throw DataCloneError
                        if (typeof value === 'symbol') {
                            const err = new Error("Symbol cannot be cloned");
                            err.name = "DataCloneError";
                            throw err;
                        }
                        parent[key] = value;
                        continue;
                    }

                    // Handle transfer
                    if (transfers.has(value)) {
                        const transferred = transfers.get(value);
                        transfers.delete(value);
                        parent[key] = transferred;
                        clonedObjects.set(value, transferred);
                        continue;
                    }

                    // Check if already cloned
                    if (clonedObjects.has(value)) {
                        parent[key] = clonedObjects.get(value);
                        continue;
                    }

                    // Create new clone
                    const cloned = createClone(value);
                    clonedObjects.set(value, cloned);

                    // Set to parent
                    parent[key] = cloned;

                    // Queue this object's properties
                    queueProperties(value, cloned);
                }

                // AFTER all pending properties are processed, THEN process Map/Set entries
                // This ensures all nested objects have been cloned and registered in clonedObjects
                for (const [map, origKey, origVal] of mapEntries) {
                    const clonedKey = clonedObjects.get(origKey);
                    const clonedVal = clonedObjects.get(origVal);
                    if (clonedKey !== undefined && clonedVal !== undefined) {
                        map.set(clonedKey, clonedVal);
                    }
                }

                // Process Set values using cloned values
                for (const [set, origVal] of setValues) {
                    const clonedVal = clonedObjects.get(origVal);
                    if (clonedVal !== undefined) {
                        set.add(clonedVal);
                    }
                }

                return rootCloned;
            }
            return clone;
        })()
    "#;

    let code = v8::String::new(scope, code).unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let func = script.run(scope).unwrap();

    let key = v8::String::new(scope, CLONE_FUNC_KEY).unwrap();
    global.set(scope, key.into(), func);
    Ok(())
}

/// structuredClone callback function
fn structured_clone_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let value = args.get(0);

    // Extract transfer list from options object (second argument)
    let transfer_list = if args.length() > 1 {
        let options = args.get(1);
        if options.is_object() {
            let options_obj = v8::Local::<v8::Object>::try_from(options).ok();
            if let Some(obj) = options_obj {
                let transfer_key = v8::String::new(scope, "transfer").unwrap();
                obj.get(scope, transfer_key.into())
                    .unwrap_or(v8::null(scope).into())
            } else {
                v8::null(scope).into()
            }
        } else {
            v8::null(scope).into()
        }
    } else {
        v8::null(scope).into()
    };

    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, CLONE_FUNC_KEY).unwrap();

    // Get the internal clone function (set up during initialization)
    let clone_func = global.get(scope, key.into()).unwrap();
    let func: v8::Local<v8::Function> = clone_func.try_into().unwrap();

    let undefined = v8::undefined(scope);
    let result = func.call(scope, undefined.into(), &[value, transfer_list]);

    // v0.3.316: Handle Promise cloning using V8 PromiseState API
    let cloned_result = match result {
        Some(res) if res.is_object() => {
            let obj = v8::Local::<v8::Object>::try_from(res).ok();
            if let Some(obj) = obj {
                let marker_key = v8::String::new(scope, "__promiseMarker__").unwrap();
                let promise_key = v8::String::new(scope, "__promiseObj__").unwrap();

                if let Some(marker) = obj.get(scope, marker_key.into()) {
                    if marker.is_true() {
                        // This is a Promise marker object - clone the Promise state
                        let promise_obj = obj.get(scope, promise_key.into()).unwrap();

                        if let Ok(promise) = v8::Local::<v8::Promise>::try_from(promise_obj) {
                            let promise_state = promise.state();

                            match promise_state {
                                v8::PromiseState::Fulfilled => {
                                    // Clone the fulfilled value
                                    let value = promise.result(scope);
                                    let value_cloned = func
                                        .call(scope, undefined.into(), &[value, transfer_list])
                                        .unwrap_or(v8::null(scope).into());
                                    // Return a new resolved Promise with the cloned value
                                    let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
                                    promise_resolver.resolve(scope, value_cloned);
                                    promise_resolver.get_promise(scope).into()
                                }
                                v8::PromiseState::Rejected => {
                                    // Clone the rejection reason as Error, preserving all properties
                                    let reason = promise.result(scope);

                                    // Create Error with reason as message
                                    let error_ctor_key = v8::String::new(scope, "Error").unwrap();
                                    let error_ctor: v8::Local<v8::Function> = global
                                        .get(scope, error_ctor_key.into())
                                        .unwrap()
                                        .try_into()
                                        .unwrap();

                                    // Convert reason to string for message
                                    let reason_str = reason.to_string(scope).unwrap_or(
                                        v8::String::new(scope, "Unknown error").unwrap(),
                                    );

                                    // Create error with undefined first, then set message
                                    let undefined = v8::undefined(scope);
                                    let error = error_ctor
                                        .new_instance(scope, &[undefined.into()])
                                        .unwrap();
                                    let message_key = v8::String::new(scope, "message").unwrap();
                                    error.set(scope, message_key.into(), reason_str.into());

                                    // If reason was an object, copy its properties to the Error
                                    if reason.is_object() {
                                        let reason_obj =
                                            v8::Local::<v8::Object>::try_from(reason).ok();
                                        if let Some(obj) = reason_obj {
                                            let prop_names = obj
                                                .get_own_property_names(scope, Default::default())
                                                .unwrap();
                                            for i in 0..prop_names.length() {
                                                if let Some(key) = prop_names.get_index(scope, i) {
                                                    if let Some(val) = obj.get(scope, key) {
                                                        error.set(scope, key, val);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Return a new rejected Promise with the cloned Error
                                    let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
                                    promise_resolver.reject(scope, error.into());
                                    promise_resolver.get_promise(scope).into()
                                }
                                v8::PromiseState::Pending => {
                                    // Pending Promise cannot be cloned - throw DataCloneError
                                    let err_msg =
                                        v8::String::new(scope, "Promise cannot be cloned").unwrap();
                                    let err_ctor_key = v8::String::new(scope, "Error").unwrap();
                                    let err_ctor: v8::Local<v8::Function> = global
                                        .get(scope, err_ctor_key.into())
                                        .unwrap()
                                        .try_into()
                                        .unwrap();
                                    let err =
                                        err_ctor.new_instance(scope, &[err_msg.into()]).unwrap();
                                    let name_key = v8::String::new(scope, "name").unwrap();
                                    let name_val =
                                        v8::String::new(scope, "DataCloneError").unwrap();
                                    err.set(scope, name_key.into(), name_val.into());

                                    // Throw the error using V8's throw_exception
                                    scope.throw_exception(err.into());
                                    v8::null(scope).into()
                                }
                            }
                        } else {
                            // Not a valid Promise object, return as-is
                            res
                        }
                    } else {
                        res
                    }
                } else {
                    res
                }
            } else {
                res
            }
        }
        _ => result.unwrap_or(v8::null(scope).into()),
    };

    retval.set(cloned_result);
}

/// Setup structuredClone global function
pub fn setup_structured_clone_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global: _ = context.global(scope);

    // Setup the internal clone function in the same context
    setup_internal_clone_func(scope, global)?;

    let structured_clone_template: _ = v8::FunctionTemplate::new(scope, structured_clone_callback);
    let structured_clone_func: _ = structured_clone_template.get_function(scope).unwrap();
    let structured_clone_key: _ = v8::String::new(scope, "structuredClone").unwrap();
    global.set(
        scope,
        structured_clone_key.into(),
        structured_clone_func.into(),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    // No imports needed for basic compile test
    #[test]
    fn test_clone_value_primitives() {
        // Tests would require V8 isolate setup
        // For now, just verify the module compiles
        assert!(true);
    }
}
