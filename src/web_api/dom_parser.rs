// DOMParser API — real HTML/XML parse for the `amber run` / runtime_minimal path.
//
// HTML (`text/html`) uses scraper (html5ever) and exposes document/query helpers.
// XML MIME types use roxmltree with tag/#id query helpers.
// This is a read-only parse tree for CLI workloads, not a live browser DOM.
// Stable contract: docs/DOMPARSER_CONTRACT.md (G32), pinned by tests/dom_parser_tests.rs.

use anyhow::Result;
use rusty_v8 as v8;
use scraper::{Html, Selector};
use std::collections::VecDeque;

const SOURCE_KEY: &str = "__amberDomSource";
const KIND_KEY: &str = "__amberDomKind";
const SCOPE_KEY: &str = "__amberDomScope"; // "document" | "element"

fn is_supported_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "text/html" | "text/xml" | "application/xml" | "application/xhtml+xml" | "image/svg+xml"
    )
}

fn is_html_content_type(content_type: &str) -> bool {
    content_type == "text/html"
}

fn throw_type_error(scope: &mut v8::PinScope, message: &str) {
    let error_message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    scope.throw_exception(error);
}

fn throw_syntax_error(scope: &mut v8::PinScope, message: &str) {
    let error_message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::syntax_error(scope, error_message);
    scope.throw_exception(error);
}

fn set_string_prop(scope: &mut v8::PinScope, obj: v8::Local<v8::Object>, key: &str, value: &str) {
    let k = v8::String::new(scope, key).unwrap();
    let v = v8::String::new(scope, value).unwrap();
    obj.set(scope, k.into(), v.into());
}

fn get_string_prop(scope: &mut v8::PinScope, obj: v8::Local<v8::Object>, key: &str) -> String {
    let k = v8::String::new(scope, key).unwrap();
    match obj.get(scope, k.into()) {
        Some(val) if val.is_string() => val
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn arg_string(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    index: i32,
) -> String {
    if args.length() <= index {
        return String::new();
    }
    let arg = args.get(index);
    arg.to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

fn css_escape_ident(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for (i, ch) in value.chars().enumerate() {
        let ok = matches!(ch, 'A'..='Z' | 'a'..='z' | '_' | '-' | '\u{00A0}'..=char::MAX)
            || (i > 0 && ch.is_ascii_digit());
        if ok && ch != '\\' {
            out.push(ch);
        } else {
            out.push('\\');
            out.push(ch);
        }
    }
    out
}

fn id_selector(id: &str) -> String {
    if id.is_empty() {
        return String::new();
    }
    if id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
        && id
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '-')
    {
        format!("#{}", id)
    } else {
        let escaped = id.replace('\\', "\\\\").replace('"', "\\\"");
        format!(r#"[id="{}"]"#, escaped)
    }
}

/// Set up DOMParser API in the V8 context
pub fn setup_dom_parser_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    let template = v8::FunctionTemplate::new(scope, dom_parser_constructor);
    template.set_class_name(v8::String::new(scope, "DOMParser").unwrap());

    let proto = template.prototype_template(scope);
    proto.set(
        v8::String::new(scope, "parseFromString").unwrap().into(),
        v8::FunctionTemplate::new(scope, parse_from_string_callback).into(),
    );

    let constructor = template.get_function(scope).unwrap();
    global.set(
        scope,
        v8::String::new(scope, "DOMParser").unwrap().into(),
        constructor.into(),
    );

    Ok(())
}

fn dom_parser_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if !args.is_construct_call() {
        throw_type_error(scope, "DOMParser constructor must be called with new");
        return;
    }
    retval.set(args.this().into());
}

fn parse_from_string_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let string_str = arg_string(scope, &args, 0);
    let content_type_arg = args.get(1);

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

    let document = if is_html_content_type(&content_type_str) {
        build_html_document(scope, &string_str, &content_type_str)
    } else {
        build_xml_document(scope, &string_str, &content_type_str)
    };
    retval.set(document.into());
}

fn attach_method(
    scope: &mut v8::PinScope,
    obj: v8::Local<v8::Object>,
    name: &str,
    callback: impl v8::MapFnTo<v8::FunctionCallback>,
) {
    let key = v8::String::new(scope, name).unwrap();
    let func = v8::Function::new(scope, callback).unwrap();
    obj.set(scope, key.into(), func.into());
}

fn attach_document_methods(scope: &mut v8::PinScope, document: v8::Local<v8::Object>) {
    attach_method(
        scope,
        document,
        "getElementById",
        document_get_element_by_id,
    );
    attach_method(scope, document, "querySelector", document_query_selector);
    attach_method(
        scope,
        document,
        "querySelectorAll",
        document_query_selector_all,
    );
    attach_method(
        scope,
        document,
        "getElementsByTagName",
        document_get_elements_by_tag_name,
    );
}

fn attach_element_methods(scope: &mut v8::PinScope, element: v8::Local<v8::Object>) {
    attach_method(scope, element, "getAttribute", element_get_attribute);
    attach_method(scope, element, "querySelector", element_query_selector);
    attach_method(
        scope,
        element,
        "querySelectorAll",
        element_query_selector_all,
    );
    attach_method(
        scope,
        element,
        "getElementsByTagName",
        element_get_elements_by_tag_name,
    );
}

fn build_html_document<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    source: &str,
    content_type: &str,
) -> v8::Local<'a, v8::Object> {
    let document = v8::Object::new(scope);
    set_string_prop(scope, document, SOURCE_KEY, source);
    set_string_prop(scope, document, KIND_KEY, "html");
    set_string_prop(scope, document, SCOPE_KEY, "document");
    set_string_prop(scope, document, "contentType", content_type);
    set_string_prop(scope, document, "URL", "about:blank");

    let html = Html::parse_document(source);

    if let Ok(sel) = Selector::parse("html") {
        if let Some(root) = html.select(&sel).next() {
            let document_element = html_element_from_ref(scope, &root);
            let key = v8::String::new(scope, "documentElement").unwrap();
            document.set(scope, key.into(), document_element.into());
        }
    }

    if let Ok(sel) = Selector::parse("head") {
        if let Some(head) = html.select(&sel).next() {
            let head_obj = html_element_from_ref(scope, &head);
            let key = v8::String::new(scope, "head").unwrap();
            document.set(scope, key.into(), head_obj.into());
        }
    }

    if let Ok(sel) = Selector::parse("body") {
        if let Some(body) = html.select(&sel).next() {
            let body_obj = html_element_from_ref(scope, &body);
            let key = v8::String::new(scope, "body").unwrap();
            document.set(scope, key.into(), body_obj.into());
        } else {
            // Empty / fragment input still gets a body object with empty innerHTML.
            let body_obj = v8::Object::new(scope);
            set_string_prop(scope, body_obj, SOURCE_KEY, "<body></body>");
            set_string_prop(scope, body_obj, KIND_KEY, "html");
            set_string_prop(scope, body_obj, SCOPE_KEY, "element");
            set_string_prop(scope, body_obj, "tagName", "BODY");
            set_string_prop(scope, body_obj, "id", "");
            set_string_prop(scope, body_obj, "className", "");
            set_string_prop(scope, body_obj, "textContent", "");
            set_string_prop(scope, body_obj, "innerHTML", "");
            set_string_prop(scope, body_obj, "outerHTML", "<body></body>");
            let children = v8::Array::new(scope, 0);
            let children_key = v8::String::new(scope, "children").unwrap();
            body_obj.set(scope, children_key.into(), children.into());
            attach_element_methods(scope, body_obj);
            let key = v8::String::new(scope, "body").unwrap();
            document.set(scope, key.into(), body_obj.into());
        }
    }

    let title = if let Ok(sel) = Selector::parse("title") {
        html.select(&sel)
            .next()
            .map(|t| t.text().collect::<String>())
            .unwrap_or_default()
    } else {
        String::new()
    };
    set_string_prop(scope, document, "title", title.trim());

    let children = element_children_array(scope, &html, "html");
    let children_key = v8::String::new(scope, "children").unwrap();
    document.set(scope, children_key.into(), children.into());

    attach_document_methods(scope, document);
    document
}

fn html_element_from_ref<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    element: &scraper::ElementRef,
) -> v8::Local<'a, v8::Object> {
    let obj = v8::Object::new(scope);
    let outer = element.html();
    set_string_prop(scope, obj, SOURCE_KEY, &outer);
    set_string_prop(scope, obj, KIND_KEY, "html");
    set_string_prop(scope, obj, SCOPE_KEY, "element");

    let tag = element.value().name().to_ascii_uppercase();
    set_string_prop(scope, obj, "tagName", &tag);
    set_string_prop(scope, obj, "id", element.value().id().unwrap_or(""));
    set_string_prop(
        scope,
        obj,
        "className",
        element.value().attr("class").unwrap_or(""),
    );

    let text: String = element.text().collect();
    set_string_prop(scope, obj, "textContent", &text);
    set_string_prop(scope, obj, "innerHTML", &element.inner_html());
    set_string_prop(scope, obj, "outerHTML", &outer);

    let children = {
        let mut kids = Vec::new();
        for child in element.children() {
            if let Some(child_el) = scraper::ElementRef::wrap(child) {
                kids.push(html_element_from_ref(scope, &child_el));
            }
        }
        let arr = v8::Array::new(scope, kids.len() as i32);
        for (i, kid) in kids.into_iter().enumerate() {
            arr.set_index(scope, i as u32, kid.into());
        }
        arr
    };
    let children_key = v8::String::new(scope, "children").unwrap();
    obj.set(scope, children_key.into(), children.into());

    attach_element_methods(scope, obj);
    obj
}

fn element_children_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    html: &Html,
    parent_selector: &str,
) -> v8::Local<'a, v8::Array> {
    let Ok(sel) = Selector::parse(parent_selector) else {
        return v8::Array::new(scope, 0);
    };
    let Some(parent) = html.select(&sel).next() else {
        return v8::Array::new(scope, 0);
    };
    let mut kids = Vec::new();
    for child in parent.children() {
        if let Some(child_el) = scraper::ElementRef::wrap(child) {
            kids.push(html_element_from_ref(scope, &child_el));
        }
    }
    let arr = v8::Array::new(scope, kids.len() as i32);
    for (i, kid) in kids.into_iter().enumerate() {
        arr.set_index(scope, i as u32, kid.into());
    }
    arr
}

fn build_xml_document<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    source: &str,
    content_type: &str,
) -> v8::Local<'a, v8::Object> {
    let document = v8::Object::new(scope);
    set_string_prop(scope, document, SOURCE_KEY, source);
    set_string_prop(scope, document, KIND_KEY, "xml");
    set_string_prop(scope, document, SCOPE_KEY, "document");
    set_string_prop(scope, document, "contentType", content_type);
    set_string_prop(scope, document, "URL", "about:blank");

    match roxmltree::Document::parse(source) {
        Ok(doc) => {
            let root = doc.root_element();
            let document_element = xml_element_from_node(scope, &root);
            let key = v8::String::new(scope, "documentElement").unwrap();
            document.set(scope, key.into(), document_element.into());

            let children = v8::Array::new(scope, 1);
            children.set_index(scope, 0, document_element.into());
            let children_key = v8::String::new(scope, "children").unwrap();
            document.set(scope, children_key.into(), children.into());
        }
        Err(err) => {
            // DOMParser XML failure: return a document containing <parsererror>.
            let msg = format!("XML parse error: {err}");
            let error_html = format!(
                r#"<parsererror xmlns="http://www.mozilla.org/newlayout/xml/parsererror.xml">{}</parsererror>"#,
                xml_escape(&msg)
            );
            set_string_prop(scope, document, SOURCE_KEY, &error_html);
            let error_el = v8::Object::new(scope);
            set_string_prop(scope, error_el, SOURCE_KEY, &error_html);
            set_string_prop(scope, error_el, KIND_KEY, "xml");
            set_string_prop(scope, error_el, SCOPE_KEY, "element");
            set_string_prop(scope, error_el, "tagName", "parsererror");
            set_string_prop(scope, error_el, "id", "");
            set_string_prop(scope, error_el, "className", "");
            set_string_prop(scope, error_el, "textContent", &msg);
            set_string_prop(scope, error_el, "innerHTML", &xml_escape(&msg));
            set_string_prop(scope, error_el, "outerHTML", &error_html);
            let empty = v8::Array::new(scope, 0);
            let children_key = v8::String::new(scope, "children").unwrap();
            error_el.set(scope, children_key.into(), empty.into());
            attach_element_methods(scope, error_el);

            let key = v8::String::new(scope, "documentElement").unwrap();
            document.set(scope, key.into(), error_el.into());
            let children = v8::Array::new(scope, 1);
            children.set_index(scope, 0, error_el.into());
            document.set(scope, children_key.into(), children.into());
        }
    }

    // XML documents do not expose HTML body/head.
    attach_document_methods(scope, document);
    document
}

fn xml_element_from_node<'a, 'input>(
    scope: &mut v8::PinScope<'a, '_>,
    node: &roxmltree::Node<'input, 'input>,
) -> v8::Local<'a, v8::Object> {
    let obj = v8::Object::new(scope);
    let tag = node.tag_name().name().to_string();
    let outer = serialize_xml_element(node);
    set_string_prop(scope, obj, SOURCE_KEY, &outer);
    set_string_prop(scope, obj, KIND_KEY, "xml");
    set_string_prop(scope, obj, SCOPE_KEY, "element");
    set_string_prop(scope, obj, "tagName", &tag);
    set_string_prop(scope, obj, "id", node.attribute("id").unwrap_or(""));
    set_string_prop(
        scope,
        obj,
        "className",
        node.attribute("class").unwrap_or(""),
    );

    let text_content: String = node
        .descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect();
    set_string_prop(scope, obj, "textContent", &text_content);

    let inner: String = node
        .children()
        .map(|child| {
            if child.is_text() {
                child.text().unwrap_or("").to_string()
            } else if child.is_element() {
                serialize_xml_element(&child)
            } else {
                String::new()
            }
        })
        .collect();
    set_string_prop(scope, obj, "innerHTML", &inner);
    set_string_prop(scope, obj, "outerHTML", &outer);

    let mut kids = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        kids.push(xml_element_from_node(scope, &child));
    }
    let arr = v8::Array::new(scope, kids.len() as i32);
    for (i, kid) in kids.into_iter().enumerate() {
        arr.set_index(scope, i as u32, kid.into());
    }
    let children_key = v8::String::new(scope, "children").unwrap();
    obj.set(scope, children_key.into(), arr.into());

    attach_element_methods(scope, obj);
    obj
}

fn serialize_xml_element(node: &roxmltree::Node<'_, '_>) -> String {
    let mut out = String::new();
    out.push('<');
    out.push_str(node.tag_name().name());
    for attr in node.attributes() {
        out.push(' ');
        out.push_str(attr.name());
        out.push_str("=\"");
        out.push_str(&xml_escape(attr.value()));
        out.push('"');
    }
    let has_children = node.children().any(|c| c.is_element() || c.is_text());
    if !has_children {
        out.push_str("/>");
        return out;
    }
    out.push('>');
    for child in node.children() {
        if child.is_text() {
            out.push_str(&xml_escape(child.text().unwrap_or("")));
        } else if child.is_element() {
            out.push_str(&serialize_xml_element(&child));
        }
    }
    out.push_str("</");
    out.push_str(node.tag_name().name());
    out.push('>');
    out
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn document_kind(scope: &mut v8::PinScope, this: v8::Local<v8::Object>) -> String {
    get_string_prop(scope, this, KIND_KEY)
}

fn document_source(scope: &mut v8::PinScope, this: v8::Local<v8::Object>) -> String {
    get_string_prop(scope, this, SOURCE_KEY)
}

fn document_get_element_by_id(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let id = arg_string(scope, &args, 0);
    if id.is_empty() {
        retval.set(v8::null(scope).into());
        return;
    }
    let source = document_source(scope, this);
    let kind = document_kind(scope, this);
    if kind == "html" {
        let html = Html::parse_document(&source);
        let sel_str = id_selector(&id);
        if let Ok(sel) = Selector::parse(&sel_str) {
            if let Some(el) = html.select(&sel).next() {
                retval.set(html_element_from_ref(scope, &el).into());
                return;
            }
        }
        retval.set(v8::null(scope).into());
    } else {
        match roxmltree::Document::parse(&source) {
            Ok(doc) => {
                if let Some(node) = doc
                    .descendants()
                    .find(|n| n.is_element() && n.attribute("id") == Some(id.as_str()))
                {
                    retval.set(xml_element_from_node(scope, &node).into());
                    return;
                }
                retval.set(v8::null(scope).into());
            }
            Err(_) => retval.set(v8::null(scope).into()),
        }
    }
}

fn document_query_selector(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let selector = arg_string(scope, &args, 0);
    let source = document_source(scope, this);
    let kind = document_kind(scope, this);
    if kind == "html" {
        let html = Html::parse_document(&source);
        match Selector::parse(&selector) {
            Ok(sel) => {
                if let Some(el) = html.select(&sel).next() {
                    retval.set(html_element_from_ref(scope, &el).into());
                } else {
                    retval.set(v8::null(scope).into());
                }
            }
            Err(_) => {
                throw_syntax_error(
                    scope,
                    &format!("DOMParser querySelector: invalid selector '{selector}'"),
                );
            }
        }
    } else {
        match roxmltree::Document::parse(&source) {
            Ok(doc) => match parse_xml_selector(&selector) {
                Ok(_) => {
                    if let Some(node) = xml_find_first(&doc, &selector) {
                        retval.set(xml_element_from_node(scope, &node).into());
                    } else {
                        retval.set(v8::null(scope).into());
                    }
                }
                Err(msg) => throw_syntax_error(scope, &msg),
            },
            Err(_) => retval.set(v8::null(scope).into()),
        }
    }
}

fn document_query_selector_all(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let selector = arg_string(scope, &args, 0);
    let source = document_source(scope, this);
    let kind = document_kind(scope, this);
    if kind == "html" {
        let html = Html::parse_document(&source);
        match Selector::parse(&selector) {
            Ok(sel) => {
                let matches: Vec<_> = html.select(&sel).collect();
                let arr = v8::Array::new(scope, matches.len() as i32);
                for (i, el) in matches.iter().enumerate() {
                    let obj = html_element_from_ref(scope, el);
                    arr.set_index(scope, i as u32, obj.into());
                }
                retval.set(arr.into());
            }
            Err(_) => {
                throw_syntax_error(
                    scope,
                    &format!("DOMParser querySelectorAll: invalid selector '{selector}'"),
                );
            }
        }
    } else {
        match roxmltree::Document::parse(&source) {
            Ok(doc) => match xml_find_all(&doc, &selector) {
                Ok(nodes) => {
                    let arr = v8::Array::new(scope, nodes.len() as i32);
                    for (i, node) in nodes.iter().enumerate() {
                        let obj = xml_element_from_node(scope, node);
                        arr.set_index(scope, i as u32, obj.into());
                    }
                    retval.set(arr.into());
                }
                Err(msg) => throw_syntax_error(scope, &msg),
            },
            Err(_) => retval.set(v8::Array::new(scope, 0).into()),
        }
    }
}

fn document_get_elements_by_tag_name(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let tag = arg_string(scope, &args, 0);
    let source = document_source(scope, this);
    let kind = document_kind(scope, this);
    if kind == "html" {
        let html = Html::parse_document(&source);
        let sel_str = if tag == "*" {
            "*".to_string()
        } else {
            css_escape_ident(&tag.to_ascii_lowercase())
        };
        let arr = match Selector::parse(&sel_str) {
            Ok(sel) => {
                let matches: Vec<_> = html.select(&sel).collect();
                let arr = v8::Array::new(scope, matches.len() as i32);
                for (i, el) in matches.iter().enumerate() {
                    let obj = html_element_from_ref(scope, el);
                    arr.set_index(scope, i as u32, obj.into());
                }
                arr
            }
            Err(_) => v8::Array::new(scope, 0),
        };
        retval.set(arr.into());
    } else {
        match roxmltree::Document::parse(&source) {
            Ok(doc) => {
                let nodes: Vec<_> = doc
                    .descendants()
                    .filter(|n| {
                        n.is_element()
                            && (tag == "*" || n.tag_name().name().eq_ignore_ascii_case(&tag))
                    })
                    .collect();
                let arr = v8::Array::new(scope, nodes.len() as i32);
                for (i, node) in nodes.iter().enumerate() {
                    let obj = xml_element_from_node(scope, node);
                    arr.set_index(scope, i as u32, obj.into());
                }
                retval.set(arr.into());
            }
            Err(_) => retval.set(v8::Array::new(scope, 0).into()),
        }
    }
}

fn element_get_attribute(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let name = arg_string(scope, &args, 0);
    let source = document_source(scope, this);
    let kind = document_kind(scope, this);
    if kind == "html" {
        let frag = Html::parse_fragment(&source);
        let root = first_fragment_element(&frag).unwrap_or(frag.root_element());
        match root.value().attr(&name) {
            Some(v) => {
                let s = v8::String::new(scope, v).unwrap();
                retval.set(s.into());
            }
            None => retval.set(v8::null(scope).into()),
        }
    } else {
        match roxmltree::Document::parse(&source) {
            Ok(doc) => {
                let root = doc.root_element();
                match root.attribute(name.as_str()) {
                    Some(v) => {
                        let s = v8::String::new(scope, v).unwrap();
                        retval.set(s.into());
                    }
                    None => retval.set(v8::null(scope).into()),
                }
            }
            Err(_) => retval.set(v8::null(scope).into()),
        }
    }
}

fn first_fragment_element<'a>(html: &'a Html) -> Option<scraper::ElementRef<'a>> {
    // parse_fragment builds a tree; the useful root is often under html/body.
    if let Ok(sel) = Selector::parse("body > *") {
        if let Some(el) = html.select(&sel).next() {
            return Some(el);
        }
    }
    html.tree
        .root()
        .descendants()
        .filter_map(scraper::ElementRef::wrap)
        .find(|e| {
            let name = e.value().name();
            name != "html" && name != "head" && name != "body"
        })
}

fn element_query_selector(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let selector = arg_string(scope, &args, 0);
    let source = document_source(scope, this);
    let kind = document_kind(scope, this);
    if kind == "html" {
        let frag = Html::parse_fragment(&source);
        let root = first_fragment_element(&frag).unwrap_or(frag.root_element());
        match Selector::parse(&selector) {
            Ok(sel) => {
                if let Some(el) = root.select(&sel).next() {
                    retval.set(html_element_from_ref(scope, &el).into());
                } else {
                    retval.set(v8::null(scope).into());
                }
            }
            Err(_) => {
                throw_syntax_error(
                    scope,
                    &format!("DOMParser querySelector: invalid selector '{selector}'"),
                );
            }
        }
    } else {
        match roxmltree::Document::parse(&source) {
            Ok(doc) => match xml_find_first(&doc, &selector) {
                Some(node) => retval.set(xml_element_from_node(scope, &node).into()),
                None => retval.set(v8::null(scope).into()),
            },
            Err(_) => retval.set(v8::null(scope).into()),
        }
    }
}

fn element_query_selector_all(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let selector = arg_string(scope, &args, 0);
    let source = document_source(scope, this);
    let kind = document_kind(scope, this);
    if kind == "html" {
        let frag = Html::parse_fragment(&source);
        let root = first_fragment_element(&frag).unwrap_or(frag.root_element());
        match Selector::parse(&selector) {
            Ok(sel) => {
                let matches: Vec<_> = root.select(&sel).collect();
                let arr = v8::Array::new(scope, matches.len() as i32);
                for (i, el) in matches.iter().enumerate() {
                    let obj = html_element_from_ref(scope, el);
                    arr.set_index(scope, i as u32, obj.into());
                }
                retval.set(arr.into());
            }
            Err(_) => {
                throw_syntax_error(
                    scope,
                    &format!("DOMParser querySelectorAll: invalid selector '{selector}'"),
                );
            }
        }
    } else {
        match roxmltree::Document::parse(&source) {
            Ok(doc) => match xml_find_all(&doc, &selector) {
                Ok(nodes) => {
                    let arr = v8::Array::new(scope, nodes.len() as i32);
                    for (i, node) in nodes.iter().enumerate() {
                        let obj = xml_element_from_node(scope, node);
                        arr.set_index(scope, i as u32, obj.into());
                    }
                    retval.set(arr.into());
                }
                Err(msg) => throw_syntax_error(scope, &msg),
            },
            Err(_) => retval.set(v8::Array::new(scope, 0).into()),
        }
    }
}

fn element_get_elements_by_tag_name(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let tag = arg_string(scope, &args, 0);
    let source = document_source(scope, this);
    let kind = document_kind(scope, this);
    if kind == "html" {
        let frag = Html::parse_fragment(&source);
        let root = first_fragment_element(&frag).unwrap_or(frag.root_element());
        let sel_str = if tag == "*" {
            "*".to_string()
        } else {
            css_escape_ident(&tag.to_ascii_lowercase())
        };
        let arr = match Selector::parse(&sel_str) {
            Ok(sel) => {
                let matches: Vec<_> = root.select(&sel).collect();
                let arr = v8::Array::new(scope, matches.len() as i32);
                for (i, el) in matches.iter().enumerate() {
                    let obj = html_element_from_ref(scope, el);
                    arr.set_index(scope, i as u32, obj.into());
                }
                arr
            }
            Err(_) => v8::Array::new(scope, 0),
        };
        retval.set(arr.into());
    } else {
        match roxmltree::Document::parse(&source) {
            Ok(doc) => {
                let nodes: Vec<_> = doc
                    .descendants()
                    .filter(|n| {
                        n.is_element()
                            && (tag == "*" || n.tag_name().name().eq_ignore_ascii_case(&tag))
                    })
                    .collect();
                let arr = v8::Array::new(scope, nodes.len() as i32);
                for (i, node) in nodes.iter().enumerate() {
                    let obj = xml_element_from_node(scope, node);
                    arr.set_index(scope, i as u32, obj.into());
                }
                retval.set(arr.into());
            }
            Err(_) => retval.set(v8::Array::new(scope, 0).into()),
        }
    }
}

/// Minimal XML selector: `tag`, `*`, `#id`, `tag#id`, and descendant `a b`.
fn parse_xml_selector(selector: &str) -> Result<Vec<XmlSimpleSel>, String> {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return Err("DOMParser querySelector: empty selector".into());
    }
    if trimmed.contains(',')
        || trimmed.contains('[')
        || trimmed.contains(':')
        || trimmed.contains('>')
    {
        return Err(format!(
            "DOMParser XML querySelector: unsupported selector '{selector}' (tag/#id/descendant only)"
        ));
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let mut out = Vec::with_capacity(parts.len());
    for part in parts {
        out.push(XmlSimpleSel::parse(part)?);
    }
    Ok(out)
}

#[derive(Clone, Debug)]
struct XmlSimpleSel {
    tag: Option<String>,
    id: Option<String>,
}

impl XmlSimpleSel {
    fn parse(part: &str) -> Result<Self, String> {
        if let Some(rest) = part.strip_prefix('#') {
            if rest.is_empty() {
                return Err("DOMParser querySelector: invalid #id".into());
            }
            return Ok(Self {
                tag: None,
                id: Some(rest.to_string()),
            });
        }
        if let Some((tag, id)) = part.split_once('#') {
            if tag.is_empty() || id.is_empty() {
                return Err(format!(
                    "DOMParser querySelector: invalid selector '{part}'"
                ));
            }
            return Ok(Self {
                tag: Some(tag.to_string()),
                id: Some(id.to_string()),
            });
        }
        if part == "*" {
            return Ok(Self {
                tag: None,
                id: None,
            });
        }
        if part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
        {
            return Ok(Self {
                tag: Some(part.to_string()),
                id: None,
            });
        }
        Err(format!(
            "DOMParser XML querySelector: unsupported selector '{part}'"
        ))
    }

    fn matches(&self, node: &roxmltree::Node<'_, '_>) -> bool {
        if !node.is_element() {
            return false;
        }
        if let Some(tag) = &self.tag {
            if !node.tag_name().name().eq_ignore_ascii_case(tag) {
                return false;
            }
        }
        if let Some(id) = &self.id {
            if node.attribute("id") != Some(id.as_str()) {
                return false;
            }
        }
        true
    }
}

fn xml_find_first<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    selector: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    let chain = parse_xml_selector(selector).ok()?;
    xml_find_all_nodes(doc, &chain).into_iter().next()
}

fn xml_find_all<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    selector: &str,
) -> Result<Vec<roxmltree::Node<'a, 'input>>, String> {
    let chain = parse_xml_selector(selector)?;
    Ok(xml_find_all_nodes(doc, &chain))
}

fn xml_find_all_nodes<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    chain: &[XmlSimpleSel],
) -> Vec<roxmltree::Node<'a, 'input>> {
    if chain.is_empty() {
        return Vec::new();
    }
    let mut current: Vec<roxmltree::Node<'a, 'input>> =
        doc.descendants().filter(|n| chain[0].matches(n)).collect();
    for step in chain.iter().skip(1) {
        let mut next = Vec::new();
        for node in current {
            let mut queue: VecDeque<_> = node.children().filter(|n| n.is_element()).collect();
            while let Some(child) = queue.pop_front() {
                if step.matches(&child) {
                    next.push(child);
                }
                for grand in child.children().filter(|n| n.is_element()) {
                    queue.push_back(grand);
                }
            }
        }
        current = next;
    }
    current
}
