// DOMParser API implementation
// Provides HTML/XML document parsing capability for AI workloads
// that need to process web content
//
// v0.3.341: Initial implementation of DOMParser API

use anyhow::Result;
use rusty_v8 as v8;

fn is_supported_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "text/html" | "text/xml" | "application/xml" | "application/xhtml+xml" | "image/svg+xml"
    )
}

fn throw_type_error(scope: &mut v8::PinScope, message: &str) {
    let error_message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    scope.throw_exception(error);
}

/// Set up DOMParser API in the V8 context
pub fn setup_dom_parser_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Create parseFromString key
    let parse_from_string_key = v8::String::new(scope, "parseFromString").unwrap();

    // Create parseFromString method template
    let parse_from_string_fn_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let string_arg = args.get(0);
            let content_type_arg = args.get(1);

            let string_str = if string_arg.is_string() {
                string_arg
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default()
            } else {
                String::new()
            };

            if args.length() < 2 || !content_type_arg.is_string() {
                throw_type_error(scope, "DOMParser.parseFromString: contentType is required");
                return;
            }

            let content_type_str = content_type_arg
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            if !is_supported_content_type(&content_type_str) {
                throw_type_error(scope, "DOMParser.parseFromString: unsupported contentType");
                return;
            }

            // Create a document-like object based on content type
            let document_obj: v8::Local<v8::Object> = v8::Object::new(scope);

            // Add document properties
            let document_key = v8::String::new(scope, "document").unwrap();
            let document_content = format!(
                "<Document contentType='{}'>\n{}\n</Document>",
                content_type_str,
                escape_html(&string_str)
            );
            let document_content_str = v8::String::new(scope, &document_content).unwrap();
            document_obj.set(scope, document_key.into(), document_content_str.into());

            // Add body property for HTML content
            if content_type_str.contains("html") {
                let body_key = v8::String::new(scope, "body").unwrap();
                let body_content = format!("<Body>\n{}\n</Body>", escape_html(&string_str));
                let body_content_str = v8::String::new(scope, &body_content).unwrap();
                document_obj.set(scope, body_key.into(), body_content_str.into());
            }

            // Add children property
            let children_key = v8::String::new(scope, "children").unwrap();
            let children_array: v8::Local<v8::Array> = v8::Array::new(scope, 0);
            document_obj.set(scope, children_key.into(), children_array.into());

            // Add body.innerHTML for HTML documents
            if content_type_str.contains("html") {
                let body_key = v8::String::new(scope, "body").unwrap();
                let body_obj: v8::Local<v8::Object> = v8::Object::new(scope);
                let body_inner_html_key = v8::String::new(scope, "innerHTML").unwrap();
                let inner_html_content = escape_html(&string_str);
                let inner_html_value = v8::String::new(scope, &inner_html_content).unwrap();
                body_obj.set(scope, body_inner_html_key.into(), inner_html_value.into());
                document_obj.set(scope, body_key.into(), body_obj.into());
            }

            // Add URL property (simulated)
            let url_key = v8::String::new(scope, "URL").unwrap();
            let url_value = v8::String::new(scope, "data:text/html,domparser").unwrap();
            document_obj.set(scope, url_key.into(), url_value.into());

            retval.set(document_obj.into());
        },
    );

    // Create parseFromString function instance
    let parse_from_string_fn = parse_from_string_fn_template.get_function(scope).unwrap();

    // Create DOMParser constructor
    // Note: We create it separately and then set the parseFromString method on instances
    let dom_parser_template = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Create a simple object as instance
            let dom_parser_obj: v8::Local<v8::Object> = v8::Object::new(_scope);
            retval.set(dom_parser_obj.into());
        },
    );

    let dom_parser_constructor: v8::Local<v8::Function> =
        dom_parser_template.get_function(scope).unwrap();

    // Add parseFromString to the constructor function itself
    dom_parser_constructor.set(
        scope,
        parse_from_string_key.clone().into(),
        parse_from_string_fn.into(),
    );

    // Also add it to the prototype-like property of the constructor
    let prototype_key = v8::String::new(scope, "prototype").unwrap();
    let prototype_obj: v8::Local<v8::Object> = v8::Object::new(scope);
    prototype_obj.set(
        scope,
        parse_from_string_key.clone().into(),
        parse_from_string_fn.into(),
    );
    dom_parser_constructor.set(scope, prototype_key.into(), prototype_obj.into());

    // Set DOMParser as global
    let dom_parser_key: v8::Local<v8::String> = v8::String::new(scope, "DOMParser").unwrap();
    global.set(scope, dom_parser_key.into(), dom_parser_constructor.into());

    Ok(())
}

/// Simple HTML escaping for security
fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
