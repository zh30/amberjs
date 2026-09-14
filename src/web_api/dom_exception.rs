// WebIDL DOMException API implementation for Web Standard and WinterTC ECMA-429
// Provides standard DOMException interface with name, message, code and legacy error constants.

use anyhow::Result;
use rusty_v8 as v8;

const DOM_EXCEPTION_CONSTANTS: &[(&str, u16)] = &[
    ("INDEX_SIZE_ERR", 1),
    ("DOMSTRING_SIZE_ERR", 2),
    ("HIERARCHY_REQUEST_ERR", 3),
    ("WRONG_DOCUMENT_ERR", 4),
    ("INVALID_CHARACTER_ERR", 5),
    ("NO_DATA_ALLOWED_ERR", 6),
    ("NO_MODIFICATION_ALLOWED_ERR", 7),
    ("NOT_FOUND_ERR", 8),
    ("NOT_SUPPORTED_ERR", 9),
    ("INUSE_ATTRIBUTE_ERR", 10),
    ("INVALID_STATE_ERR", 11),
    ("SYNTAX_ERR", 12),
    ("INVALID_MODIFICATION_ERR", 13),
    ("NAMESPACE_ERR", 14),
    ("INVALID_ACCESS_ERR", 15),
    ("VALIDATION_ERR", 16),
    ("TYPE_MISMATCH_ERR", 17),
    ("SECURITY_ERR", 18),
    ("NETWORK_ERR", 19),
    ("ABORT_ERR", 20),
    ("URL_MISMATCH_ERR", 21),
    ("QUOTA_EXCEEDED_ERR", 22),
    ("TIMEOUT_ERR", 23),
    ("INVALID_NODE_TYPE_ERR", 24),
    ("DATA_CLONE_ERR", 25),
];

fn name_to_code(name: &str) -> u16 {
    match name {
        "IndexSizeError" => 1,
        "HierarchyRequestError" => 3,
        "WrongDocumentError" => 4,
        "InvalidCharacterError" => 5,
        "NoModificationAllowedError" => 7,
        "NotFoundError" => 8,
        "NotSupportedError" => 9,
        "InUseAttributeError" => 10,
        "InvalidStateError" => 11,
        "SyntaxError" => 12,
        "InvalidModificationError" => 13,
        "NamespaceError" => 14,
        "InvalidAccessError" => 15,
        "TypeMismatchError" => 17,
        "SecurityError" => 18,
        "NetworkError" => 19,
        "AbortError" => 20,
        "URLMismatchError" => 21,
        "QuotaExceededError" => 22,
        "TimeoutError" => 23,
        "InvalidNodeTypeError" => 24,
        "DataCloneError" => 25,
        _ => 0,
    }
}

/// Setup DOMException constructor in V8 context
pub fn setup_dom_exception_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Create DOMException constructor function
    let constructor_fn = v8::Function::new(scope, dom_exception_constructor)
        .ok_or_else(|| anyhow::anyhow!("Failed to create DOMException constructor"))?;

    // Create prototype object inheriting from Error.prototype
    let prototype = v8::Object::new(scope);
    let error_str = v8::String::new(scope, "Error").unwrap();
    if let Some(error_val) = global.get(scope, error_str.into()) {
        if error_val.is_function() {
            let error_fn: v8::Local<v8::Function> = v8::Local::cast(error_val);
            let proto_key = v8::String::new(scope, "prototype").unwrap();
            if let Some(error_proto) = error_fn.get(scope, proto_key.into()) {
                if error_proto.is_object() {
                    let error_proto_obj: v8::Local<v8::Object> = v8::Local::cast(error_proto);
                    let _ = prototype.set_prototype(scope, error_proto_obj.into());
                }
            }
        }
    }

    // Set constructor on prototype
    let constructor_key = v8::String::new(scope, "constructor").unwrap();
    prototype.set(scope, constructor_key.into(), constructor_fn.into());

    // Set name on prototype
    let name_key = v8::String::new(scope, "name").unwrap();
    let name_val = v8::String::new(scope, "DOMException").unwrap();
    prototype.set(scope, name_key.into(), name_val.into());

    // Attach static constants on both constructor and prototype
    for &(const_name, code) in DOM_EXCEPTION_CONSTANTS {
        let key = v8::String::new(scope, const_name).unwrap();
        let val = v8::Integer::new(scope, code as i32);
        constructor_fn.set(scope, key.into(), val.into());
        prototype.set(scope, key.into(), val.into());
    }

    // Set prototype on constructor
    let proto_key = v8::String::new(scope, "prototype").unwrap();
    constructor_fn.set(scope, proto_key.into(), prototype.into());

    // Expose DOMException on global
    let dom_exception_key = v8::String::new(scope, "DOMException").unwrap();
    global.set(scope, dom_exception_key.into(), constructor_fn.into());

    Ok(())
}

fn dom_exception_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();

    let message = if args.length() > 0 && !args.get(0).is_undefined() {
        args.get(0)
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let name = if args.length() > 1 && !args.get(1).is_undefined() {
        args.get(1)
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "Error".to_string())
    } else {
        "Error".to_string()
    };

    let code = name_to_code(&name);

    let msg_key = v8::String::new(scope, "message").unwrap();
    let msg_val = v8::String::new(scope, &message).unwrap();
    this.set(scope, msg_key.into(), msg_val.into());

    let name_key = v8::String::new(scope, "name").unwrap();
    let name_val = v8::String::new(scope, &name).unwrap();
    this.set(scope, name_key.into(), name_val.into());

    let code_key = v8::String::new(scope, "code").unwrap();
    let code_val = v8::Integer::new(scope, code as i32);
    this.set(scope, code_key.into(), code_val.into());

    // Capture stack trace if possible
    let stack_key = v8::String::new(scope, "stack").unwrap();
    let stack_val = v8::String::new(
        scope,
        &format!("{}: {}\n    at new DOMException", name, message),
    )
    .unwrap();
    this.set(scope, stack_key.into(), stack_val.into());

    retval.set(this.into());
}
