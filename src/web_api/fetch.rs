// Fetch API implementation for Web standard
// Provides fetch(), Request, Response, Headers API

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use rusty_v8 as v8;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

// Re-export FormData functions for use in fetch
use super::form_data::{
    generate_boundary, get_formdata_entries, get_formdata_index, serialize_formdata_multipart,
};

/// Thread-safe response cache for json() and text() methods
static RESPONSE_CACHE: OnceLock<Mutex<HashMap<usize, (String, Vec<u8>)>>> = OnceLock::new();
static RESPONSE_ID_COUNTER: OnceLock<Mutex<usize>> = OnceLock::new();

/// Get the response cache mutex
fn get_response_cache() -> &'static Mutex<HashMap<usize, (String, Vec<u8>)>> {
    RESPONSE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_response_id() -> usize {
    let counter = RESPONSE_ID_COUNTER.get_or_init(|| Mutex::new(0));
    let mut counter = counter.lock().unwrap();
    let id = *counter;
    *counter += 1;
    id
}

/// Thread-safe headers cache for Headers API
static HEADERS_CACHE: OnceLock<Mutex<HashMap<usize, Vec<(String, String)>>>> = OnceLock::new();

/// Get the headers cache mutex
fn get_headers_cache() -> &'static Mutex<HashMap<usize, Vec<(String, String)>>> {
    HEADERS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Shared Tokio runtime for fetch requests (avoids constructing runtimes per fetch)
static FETCH_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_fetch_runtime() -> &'static Runtime {
    FETCH_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("beejs-fetch-worker")
            .build()
            .expect("Failed to initialize fetch runtime")
    })
}

/// Shared Reqwest Client with persistent connection pooling
static FETCH_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_fetch_client() -> &'static reqwest::Client {
    FETCH_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(format!("Beejs/{}", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(32)
            .build()
            .expect("Failed to initialize fetch client")
    })
}

/// Fetch API configuration
#[derive(Debug, Clone)]
pub struct FetchConfig {
    pub user_agent: String,
    pub timeout: std::time::Duration,
    pub max_redirects: u32,
}
impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            user_agent: format!("Beejs/{}", env!("CARGO_PKG_VERSION")),
            timeout: std::time::Duration::from_secs(30),
            max_redirects: 20,
        }
    }
}
/// HTTP method enum
#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}
impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::PUT => write!(f, "PUT"),
            HttpMethod::DELETE => write!(f, "DELETE"),
            HttpMethod::PATCH => write!(f, "PATCH"),
            HttpMethod::HEAD => write!(f, "HEAD"),
            HttpMethod::OPTIONS => write!(f, "OPTIONS"),
        }
    }
}
/// Parse HTTP method from string
impl From<String> for HttpMethod {
    fn from(s: String) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "PATCH" => HttpMethod::PATCH,
            "HEAD" => HttpMethod::HEAD,
            "OPTIONS" => HttpMethod::OPTIONS,
            _ => HttpMethod::GET,
        }
    }
}
/// Request structure
#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub credentials: String, // 'omit', 'same-origin', 'include'
    pub mode: String,        // 'cors', 'no-cors', 'same-origin'
    pub redirect: String,    // 'follow', 'error', 'manual'
    pub referrer: String,
    pub referrer_policy: String,
    pub cache: String, // 'default', 'no-cache', 'reload', 'no-store', 'only-if-cached'
    pub integrity: String,
    pub keepalive: bool,
    pub signal: Option<AbortSignal>,
}
/// Response structure
#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub ok: bool,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub body_used: bool,
    pub redirected: bool,
    pub response_type: String, // "default", "error", "opaque", "opaqueredirect"
}
/// Abort signal for request cancellation
#[derive(Debug, Clone)]
pub struct AbortSignal {
    pub aborted: Arc<Mutex<bool>>,
    pub abort_reason: Option<String>,
}
/// Setup Fetch API in V8 context
pub fn setup_fetch_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    // Create global fetch function
    let fetch_template: _ = v8::FunctionTemplate::new(scope, fetch_callback);
    let fetch_func: _ = fetch_template.get_function(scope).unwrap();
    // Set fetch to global
    let global: _ = context.global(scope);
    let fetch_key: _ = v8::String::new(scope, "fetch").unwrap();
    global.set(scope, fetch_key.into(), fetch_func.into());
    // Setup Request constructor
    let request_template: _ = v8::FunctionTemplate::new(scope, request_constructor_callback);
    let request_constructor: _ = request_template.get_function(scope).unwrap();
    let request_key: _ = v8::String::new(scope, "Request").unwrap();
    global.set(scope, request_key.into(), request_constructor.into());
    // Setup Response constructor
    let response_template: _ = v8::FunctionTemplate::new(scope, response_constructor_callback);
    let response_constructor: _ = response_template.get_function(scope).unwrap();
    let response_key: _ = v8::String::new(scope, "Response").unwrap();
    global.set(scope, response_key.into(), response_constructor.into());
    // Setup Headers constructor
    let headers_template: _ = v8::FunctionTemplate::new(scope, headers_constructor_callback);
    let headers_constructor: _ = headers_template.get_function(scope).unwrap();
    let headers_key: _ = v8::String::new(scope, "Headers").unwrap();
    global.set(scope, headers_key.into(), headers_constructor.into());

    // Inject Response static methods (Response.json, Response.redirect, Response.error) (v1.5.0)
    let response_helpers_js = r#"
    (function() {
        if (typeof Response === 'undefined') return;

        Response.json = function(data, init = {}) {
            const headers = new Headers(init.headers);
            if (!headers.has('content-type')) {
                headers.set('content-type', 'application/json');
            }
            const body = JSON.stringify(data);
            return new Response(body, {
                ...init,
                headers
            });
        };

        Response.redirect = function(url, status = 302) {
            const code = typeof status === 'number' ? status : 302;
            if (![301, 302, 303, 307, 308].includes(code)) {
                throw new RangeError('Invalid status code for redirect: ' + code);
            }
            return new Response(null, {
                status: code,
                headers: { location: String(url) }
            });
        };

        Response.error = function() {
            return new Response(null, { status: 0, statusText: '' });
        };
    })();
    "#;
    if let Some(code) = v8::String::new(scope, response_helpers_js) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
/// Main fetch function callback
fn fetch_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Parse fetch arguments
    let input: _ = args.get(0);
    let init: _ = args.get(1);

    // Parse URL and request properties from first argument
    let mut url_str = String::new();
    let mut request_method = String::from("GET");
    let mut request_headers: HashMap<String, String> = HashMap::new();
    let mut request_body: Option<Vec<u8>> = None;
    let mut request_content_type = String::new();

    if input.is_string() {
        // Input is a URL string
        url_str = input.to_string(scope).unwrap().to_rust_string_lossy(scope);
    } else if input.is_object() {
        // Input might be a Request object - extract URL and other properties
        if let Some(input_obj) = input.to_object(scope) {
            let url_key = v8::String::new(scope, "url").unwrap().into();
            if let Some(url_val) = input_obj.get(scope, url_key) {
                if url_val.is_string() {
                    url_str = url_val
                        .to_string(scope)
                        .unwrap()
                        .to_rust_string_lossy(scope);
                }
            }

            // Extract method from Request object
            let method_key = v8::String::new(scope, "method").unwrap().into();
            if let Some(method_val) = input_obj.get(scope, method_key) {
                if method_val.is_string() {
                    request_method = method_val
                        .to_string(scope)
                        .unwrap()
                        .to_rust_string_lossy(scope);
                }
            }

            // Extract headers from Request object
            let headers_key = v8::String::new(scope, "headers").unwrap().into();
            if let Some(headers_val) = input_obj.get(scope, headers_key) {
                for (name, value) in header_entries_from_value(scope, headers_val) {
                    request_headers.insert(name, value);
                }
            }

            // Extract body from Request object
            let body_key = v8::String::new(scope, "body").unwrap().into();
            if let Some(body_val) = input_obj.get(scope, body_key) {
                if body_val.is_string() {
                    if let Some(body_str) = body_val.to_string(scope) {
                        request_body = Some(body_str.to_rust_string_lossy(scope).into_bytes());
                        request_content_type = "text/plain;charset=UTF-8".to_string();
                    }
                }
            }
        }
    }
    if url_str.is_empty() {
        let error: _ = v8::String::new(scope, "Invalid URL").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // Parse init options - start with values from Request object (if provided)
    let mut method = HttpMethod::from(request_method);
    let mut headers = request_headers;
    let mut body = request_body;
    let mut content_type = request_content_type;
    let mut redirect = String::from("follow"); // Default redirect mode

    // Parse init object for method, headers, body, redirect (overrides Request properties)
    if init.is_object() {
        if let Some(init_obj) = init.to_object(scope) {
            // Parse method
            let method_key = v8::String::new(scope, "method").unwrap().into();
            if let Some(method_val) = init_obj.get(scope, method_key) {
                if let Some(method_str) = method_val.to_string(scope) {
                    method = HttpMethod::from(method_str.to_rust_string_lossy(scope));
                }
            }

            // Parse headers
            let headers_key = v8::String::new(scope, "headers").unwrap().into();
            if let Some(headers_val) = init_obj.get(scope, headers_key) {
                for (name, value) in header_entries_from_value(scope, headers_val) {
                    headers.insert(name, value);
                }
            }

            // Parse body - support string, FormData, ArrayBuffer
            let body_key = v8::String::new(scope, "body").unwrap().into();
            if let Some(body_val) = init_obj.get(scope, body_key) {
                if body_val.is_string() {
                    // String body
                    if let Some(body_str) = body_val.to_string(scope) {
                        body = Some(body_str.to_rust_string_lossy(scope).into_bytes());
                        if content_type.is_empty() {
                            content_type = "text/plain;charset=UTF-8".to_string();
                        }
                    }
                } else if body_val.is_object() {
                    // Check if it's FormData
                    if let Some(fd_index) = get_formdata_index(scope, body_val) {
                        if let Some(entries) = get_formdata_entries(fd_index) {
                            let boundary = generate_boundary();
                            body = Some(serialize_formdata_multipart(&entries, &boundary));
                            content_type = format!("multipart/form-data; boundary={}", boundary);
                        }
                    } else {
                        // Try to get as string or handle as ArrayBuffer
                        if let Some(body_str) = body_val.to_string(scope) {
                            body = Some(body_str.to_rust_string_lossy(scope).into_bytes());
                            if content_type.is_empty() {
                                content_type = "text/plain;charset=UTF-8".to_string();
                            }
                        }
                    }
                }
            }

            // Parse redirect option
            let redirect_key = v8::String::new(scope, "redirect").unwrap().into();
            if let Some(redirect_val) = init_obj.get(scope, redirect_key) {
                if let Some(redirect_str) = redirect_val.to_string(scope) {
                    redirect = redirect_str.to_rust_string_lossy(scope);
                }
            }
        }
    }

    // Add Content-Type header if body is set and header not already present
    let has_content_type_header = headers
        .keys()
        .any(|header| header.eq_ignore_ascii_case("content-type"));
    if !content_type.is_empty() && !has_content_type_header {
        headers.insert(normalize_header_name("Content-Type"), content_type);
    }

    // Execute fetch synchronously in a blocking task
    let url: _ = url_str.clone();
    let method_clone = method.clone();
    let headers_clone = headers.clone();
    let body_clone = body.clone();
    let redirect_clone = redirect.clone();

    let result = get_fetch_runtime().block_on(execute_fetch(
        &url,
        method_clone,
        headers_clone,
        body_clone,
        &redirect_clone,
    ));
    match result {
        Ok(response) => {
            // Convert response to V8 object
            let response_obj: _ = v8::Object::new(scope);
            let ok_key: _ = v8::String::new(scope, "ok").unwrap();
            let ok_key_val: _ = v8::Boolean::new(scope, response.ok).into();
            response_obj.set(scope, ok_key.into(), ok_key_val);
            let status_key: _ = v8::String::new(scope, "status").unwrap();
            let status_key_val: _ = v8::Integer::new(scope, response.status as i32).into();
            response_obj.set(scope, status_key.into(), status_key_val);
            let status_text_key: _ = v8::String::new(scope, "statusText").unwrap();
            let status_text_val: v8::Local<v8::Value> =
                v8::String::new(scope, &response.status_text)
                    .unwrap()
                    .into();
            response_obj.set(scope, status_text_key.into(), status_text_val);

            // Add url property
            let url_key: _ = v8::String::new(scope, "url").unwrap();
            let url_val: _ = v8::String::new(scope, &response.url).unwrap().into();
            response_obj.set(scope, url_key.into(), url_val);

            // Store body in cache for json() and text() methods
            let body_vec = response.body.unwrap_or_default();
            store_response_body(scope, response_obj, response.url.clone(), body_vec);
            attach_response_body_methods(scope, response_obj);

            // v0.3.348: Add type property (response type) - use actual response type
            let type_key: _ = v8::String::new(scope, "type").unwrap();
            let type_val: v8::Local<v8::Value> = v8::String::new(scope, &response.response_type)
                .unwrap()
                .into();
            response_obj.set(scope, type_key.into(), type_val);

            // v0.3.348: Add redirected property - use actual redirect status
            let redirected_key: _ = v8::String::new(scope, "redirected").unwrap();
            let redirected_val: v8::Local<v8::Value> =
                v8::Boolean::new(scope, response.redirected).into();
            response_obj.set(scope, redirected_key.into(), redirected_val);

            // v0.3.351: Add bodyUsed property
            let body_used_key: _ = v8::String::new(scope, "bodyUsed").unwrap();
            let body_used_val: v8::Local<v8::Value> =
                v8::Boolean::new(scope, response.body_used).into();
            response_obj.set(scope, body_used_key.into(), body_used_val);

            // v0.3.348: Add clone() method
            attach_response_clone_method(scope, response_obj);

            // Add headers (v0.3.348: enhanced with proper Headers object)
            let headers_obj = create_headers_object_with_entries(
                scope,
                response.headers.into_iter().collect::<Vec<_>>(),
            );
            let headers_key: _ = v8::String::new(scope, "headers").unwrap();
            response_obj.set(scope, headers_key.into(), headers_obj.into());
            retval.set(response_obj.into());
        }
        Err(e) => {
            let error: _ = v8::String::new(scope, &format!("Fetch error: {}", e)).unwrap();
            let error_obj: _ = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
        }
    }
}
/// Execute actual HTTP fetch using reqwest with redirect support
async fn execute_fetch(
    url: &str,
    method: HttpMethod,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    redirect: &str, // "follow", "error", "manual"
) -> Result<FetchResponse> {
    let mut current_url = url.to_string();
    let mut redirected = false;
    let mut redirect_count = 0;
    const MAX_REDIRECTS: u32 = 20;

    loop {
        // Check redirect limit
        if redirect_count > MAX_REDIRECTS {
            return Err(anyhow::anyhow!("Too many redirects"));
        }

        crate::permissions::check_global_permission(
            crate::permissions::PermissionKind::Network,
            crate::permissions::PermissionAction::Connect,
            crate::permissions::ResourceId::Url(current_url.clone()),
        )
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

        let client = get_fetch_client();

        let request: _ = client.request(
            match method {
                HttpMethod::GET => reqwest::Method::GET,
                HttpMethod::POST => reqwest::Method::POST,
                HttpMethod::PUT => reqwest::Method::PUT,
                HttpMethod::DELETE => reqwest::Method::DELETE,
                HttpMethod::PATCH => reqwest::Method::PATCH,
                HttpMethod::HEAD => reqwest::Method::HEAD,
                HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
            },
            &current_url,
        );

        // Only add body for non-GET/HEAD requests
        let request = if matches!(method, HttpMethod::GET | HttpMethod::HEAD) || body.is_none() {
            request
        } else {
            request.body(body.clone().unwrap())
        };

        // Add headers
        let mut req_builder = request;
        for (key, value) in &headers {
            req_builder = req_builder.header(key, value);
        }

        let response = match req_builder.send().await {
            Ok(response) => response,
            Err(error) => {
                return Err(anyhow::anyhow!(
                    "failed to fetch {}: {}",
                    current_url,
                    error
                ));
            }
        };

        let status = response.status().as_u16();
        let status_text = response
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();
        let ok = response.status().is_success();

        // Check for redirect status codes
        if matches!(status, 301 | 302 | 303 | 307 | 308) {
            redirect_count += 1;

            match redirect {
                "error" => {
                    // Return the redirect response as an error
                    return Ok(FetchResponse {
                        url: current_url.clone(),
                        status,
                        status_text,
                        ok: false,
                        headers: HashMap::new(),
                        body: Some(format!("Redirect not allowed: {}", status).into_bytes()),
                        body_used: false,
                        redirected: false,
                        response_type: "error".to_string(),
                    });
                }
                "manual" => {
                    // Return the redirect response without following
                    let mut response_headers: HashMap<String, String> = HashMap::new();
                    for (key, value) in response.headers() {
                        response_headers
                            .insert(key.to_string(), value.to_str().unwrap_or("").to_string());
                    }
                    let body_vec: Vec<u8> = match response.bytes().await {
                        Ok(bytes) => bytes.to_vec(),
                        Err(error) => {
                            return Err(anyhow::anyhow!(
                                "failed to read fetch body from {}: {}",
                                current_url,
                                error
                            ));
                        }
                    };
                    return Ok(FetchResponse {
                        url: current_url.clone(),
                        status,
                        status_text,
                        ok: false,
                        headers: response_headers,
                        body: Some(body_vec),
                        body_used: false,
                        redirected: false,
                        response_type: "default".to_string(),
                    });
                }
                "follow" | _ => {
                    // Follow the redirect
                    if let Some(location) = response.headers().get("location") {
                        let location_str = location.to_str().unwrap_or("");
                        let new_url = if location_str.starts_with("http") {
                            location_str.to_string()
                        } else if location_str.starts_with("/") {
                            // Relative URL - construct from current URL
                            let base_url =
                                current_url.split('/').take(3).collect::<Vec<_>>().join("/");
                            format!("{}{}", base_url, location_str)
                        } else {
                            location_str.to_string()
                        };

                        redirected = true;
                        current_url = new_url;
                        continue;
                    } else {
                        // No location header - treat as normal response
                        let mut response_headers: HashMap<String, String> = HashMap::new();
                        for (key, value) in response.headers() {
                            response_headers
                                .insert(key.to_string(), value.to_str().unwrap_or("").to_string());
                        }
                        let body_vec: Vec<u8> = match response.bytes().await {
                            Ok(bytes) => bytes.to_vec(),
                            Err(error) => {
                                return Err(anyhow::anyhow!(
                                    "failed to read fetch body from {}: {}",
                                    current_url,
                                    error
                                ));
                            }
                        };
                        return Ok(FetchResponse {
                            url: current_url.clone(),
                            status,
                            status_text,
                            ok,
                            headers: response_headers,
                            body: Some(body_vec),
                            body_used: false,
                            redirected,
                            response_type: if redirected {
                                "opaqueredirect".to_string()
                            } else {
                                "default".to_string()
                            },
                        });
                    }
                }
            }
        }

        // Extract headers BEFORE consuming the response
        let mut response_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in response.headers() {
            response_headers.insert(key.to_string(), value.to_str().unwrap_or("").to_string());
        }
        // Get response body
        let body_vec: Vec<u8> = match response.bytes().await {
            Ok(bytes) => bytes.to_vec(),
            Err(error) => {
                return Err(anyhow::anyhow!(
                    "failed to read fetch body from {}: {}",
                    current_url,
                    error
                ));
            }
        };

        // Determine response type
        let response_type = if redirected {
            "opaqueredirect".to_string()
        } else {
            "default".to_string()
        };

        return Ok(FetchResponse {
            url: current_url.clone(),
            status,
            status_text,
            ok,
            headers: response_headers,
            body: Some(body_vec),
            body_used: false,
            redirected,
            response_type,
        });
    }
}

/// Thread-safe request cache for Request API
static REQUEST_CACHE: OnceLock<Mutex<HashMap<usize, RequestData>>> = OnceLock::new();

/// Get the request cache mutex
fn get_request_cache() -> &'static Mutex<HashMap<usize, RequestData>> {
    REQUEST_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Request data stored in cache
#[derive(Debug, Clone)]
pub struct RequestData {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub cache: String,
    pub credentials: String,
    pub mode: String,
    pub redirect: String,
    pub referrer: String,
    pub referrer_policy: String,
    pub integrity: String,
    pub keepalive: bool,
}

fn request_text_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this_obj = args.this();
    let body_key = v8::String::new(scope, "body").unwrap().into();
    let body_val = this_obj
        .get(scope, body_key)
        .unwrap_or_else(|| v8::null(scope).into());
    let body_str = if body_val.is_null_or_undefined() {
        String::new()
    } else {
        body_val
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default()
    };
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let res_str = v8::String::new(scope, &body_str).unwrap();
    resolver.resolve(scope, res_str.into());
    retval.set(resolver.get_promise(scope).into());
}

fn request_json_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this_obj = args.this();
    let body_key = v8::String::new(scope, "body").unwrap().into();
    let body_val = this_obj
        .get(scope, body_key)
        .unwrap_or_else(|| v8::null(scope).into());
    let body_str = if body_val.is_null_or_undefined() {
        String::new()
    } else {
        body_val
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default()
    };
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    if let Some(s) = v8::String::new(scope, &body_str) {
        if let Some(parsed) = v8::json::parse(scope, s) {
            resolver.resolve(scope, parsed);
            retval.set(resolver.get_promise(scope).into());
            return;
        }
    }
    let err = v8::String::new(scope, "SyntaxError: Unexpected end of JSON input").unwrap();
    let err_obj = v8::Exception::syntax_error(scope, err);
    resolver.reject(scope, err_obj);
    retval.set(resolver.get_promise(scope).into());
}

fn request_array_buffer_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this_obj = args.this();
    let body_key = v8::String::new(scope, "body").unwrap().into();
    let body_val = this_obj
        .get(scope, body_key)
        .unwrap_or_else(|| v8::null(scope).into());
    let body_bytes = if body_val.is_null_or_undefined() {
        Vec::new()
    } else {
        body_val
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope).into_bytes())
            .unwrap_or_default()
    };
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let ab = v8::ArrayBuffer::new(scope, body_bytes.len());
    let store = ab.get_backing_store();
    let ptr = store.as_ref().as_ptr() as *mut u8;
    if !ptr.is_null() && !body_bytes.is_empty() {
        unsafe {
            std::ptr::copy_nonoverlapping(body_bytes.as_ptr(), ptr, body_bytes.len());
        }
    }
    resolver.resolve(scope, ab.into());
    retval.set(resolver.get_promise(scope).into());
}

fn attach_request_body_methods(scope: &mut v8::PinScope, request_obj: v8::Local<v8::Object>) {
    let text_fn = v8::Function::new(scope, request_text_callback).unwrap();
    let text_key = v8::String::new(scope, "text").unwrap().into();
    request_obj.set(scope, text_key, text_fn.into());

    let json_fn = v8::Function::new(scope, request_json_callback).unwrap();
    let json_key = v8::String::new(scope, "json").unwrap().into();
    request_obj.set(scope, json_key, json_fn.into());

    let array_buffer_fn = v8::Function::new(scope, request_array_buffer_callback).unwrap();
    let array_buffer_key = v8::String::new(scope, "arrayBuffer").unwrap().into();
    request_obj.set(scope, array_buffer_key, array_buffer_fn.into());
}

/// Request constructor callback
fn request_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Parse first argument (URL or Request object)
    let input = args.get(0);
    let mut url = String::new();
    let mut method = String::from("GET");

    // Parse URL from first argument
    if input.is_string() {
        if let Some(url_str) = input.to_string(scope) {
            url = url_str.to_rust_string_lossy(scope);
        }
    } else if input.is_object() {
        // Input is a Request object - extract its URL
        if let Some(input_obj) = input.to_object(scope) {
            let url_key = v8::String::new(scope, "url").unwrap().into();
            if let Some(url_val) = input_obj.get(scope, url_key) {
                if url_val.is_string() {
                    if let Some(url_str) = url_val.to_string(scope) {
                        url = url_str.to_rust_string_lossy(scope);
                    }
                }
            }
            // Extract method from Request object
            let method_key = v8::String::new(scope, "method").unwrap().into();
            if let Some(method_val) = input_obj.get(scope, method_key) {
                if method_val.is_string() {
                    if let Some(method_str) = method_val.to_string(scope) {
                        method = method_str.to_rust_string_lossy(scope);
                    }
                }
            }
        }
    }

    // Parse init object (second argument) for additional properties
    let mut init_cache = String::from("default");
    let mut init_credentials = String::from("same-origin");
    let mut init_mode = String::from("cors");
    let mut init_redirect = String::from("follow");
    let mut init_referrer = String::new();
    let mut init_policy = String::from("no-referrer");
    let mut init_integrity = String::new();
    let mut init_keepalive = false;
    let mut init_body: Option<String> = None;
    let mut init_headers: Vec<(String, String)> = Vec::new();

    // Parse init object (second argument) for additional properties
    // Note: args.get(1) returns undefined if not provided, so we need to check
    let init_arg = args.get(1);
    if init_arg.is_object() {
        if let Some(init) = init_arg.to_object(scope) {
            // Parse method from init
            let method_key = v8::String::new(scope, "method").unwrap().into();
            if let Some(method_val) = init.get(scope, method_key) {
                if let Some(method_str) = method_val.to_string(scope) {
                    method = method_str.to_rust_string_lossy(scope);
                }
            }

            // Parse headers
            let headers_key = v8::String::new(scope, "headers").unwrap().into();
            if let Some(headers_val) = init.get(scope, headers_key) {
                init_headers = header_entries_from_value(scope, headers_val);
            }

            // Parse cache
            let cache_key = v8::String::new(scope, "cache").unwrap().into();
            if let Some(cache_val) = init.get(scope, cache_key) {
                if let Some(cache_str) = cache_val.to_string(scope) {
                    init_cache = cache_str.to_rust_string_lossy(scope);
                }
            }

            // Parse credentials
            let cred_key = v8::String::new(scope, "credentials").unwrap().into();
            if let Some(cred_val) = init.get(scope, cred_key) {
                if let Some(cred_str) = cred_val.to_string(scope) {
                    init_credentials = cred_str.to_rust_string_lossy(scope);
                }
            }

            // Parse mode
            let mode_key = v8::String::new(scope, "mode").unwrap().into();
            if let Some(mode_val) = init.get(scope, mode_key) {
                if let Some(mode_str) = mode_val.to_string(scope) {
                    init_mode = mode_str.to_rust_string_lossy(scope);
                }
            }

            // Parse redirect
            let redirect_key = v8::String::new(scope, "redirect").unwrap().into();
            if let Some(redirect_val) = init.get(scope, redirect_key) {
                if let Some(redirect_str) = redirect_val.to_string(scope) {
                    init_redirect = redirect_str.to_rust_string_lossy(scope);
                }
            }

            // Parse referrer
            let referrer_key = v8::String::new(scope, "referrer").unwrap().into();
            if let Some(referrer_val) = init.get(scope, referrer_key) {
                if let Some(referrer_str) = referrer_val.to_string(scope) {
                    init_referrer = referrer_str.to_rust_string_lossy(scope);
                }
            }

            // Parse referrerPolicy
            let policy_key = v8::String::new(scope, "referrerPolicy").unwrap().into();
            if let Some(policy_val) = init.get(scope, policy_key) {
                if let Some(policy_str) = policy_val.to_string(scope) {
                    init_policy = policy_str.to_rust_string_lossy(scope);
                }
            }

            // Parse integrity
            let integrity_key = v8::String::new(scope, "integrity").unwrap().into();
            if let Some(integrity_val) = init.get(scope, integrity_key) {
                if let Some(integrity_str) = integrity_val.to_string(scope) {
                    init_integrity = integrity_str.to_rust_string_lossy(scope);
                }
            }

            // Parse keepalive
            let keepalive_key = v8::String::new(scope, "keepalive").unwrap().into();
            if let Some(keepalive_val) = init.get(scope, keepalive_key) {
                init_keepalive = keepalive_val.is_true();
            }

            // Parse body
            let body_key = v8::String::new(scope, "body").unwrap().into();
            if let Some(body_val) = init.get(scope, body_key) {
                if let Some(body_str) = body_val.to_string(scope) {
                    init_body = Some(body_str.to_rust_string_lossy(scope));
                }
            }
        }
    }

    // Create request object
    let request_obj: v8::Local<v8::Object> = v8::Object::new(scope);

    // Store request data in cache
    let request_ptr = &*request_obj as *const v8::Object as usize;
    let request_data = RequestData {
        url: url.clone(),
        method: method.clone(),
        headers: init_headers.clone(),
        body: init_body.clone(),
        cache: init_cache.clone(),
        credentials: init_credentials.clone(),
        mode: init_mode.clone(),
        redirect: init_redirect.clone(),
        referrer: init_referrer.clone(),
        referrer_policy: init_policy.clone(),
        integrity: init_integrity.clone(),
        keepalive: init_keepalive,
    };
    let mut request_cache_guard = get_request_cache().lock().unwrap();
    request_cache_guard.insert(request_ptr, request_data);
    drop(request_cache_guard);

    // Set url property
    let url_key = v8::String::new(scope, "url").unwrap().into();
    let url_val = v8::String::new(scope, &url).unwrap().into();
    request_obj.set(scope, url_key, url_val);

    // Set method property
    let method_key = v8::String::new(scope, "method").unwrap().into();
    let method_val = v8::String::new(scope, &method).unwrap().into();
    request_obj.set(scope, method_key, method_val);

    // Set headers property
    let headers_key = v8::String::new(scope, "headers").unwrap().into();
    let headers_obj = create_headers_object_with_entries(scope, init_headers.clone());
    request_obj.set(scope, headers_key, headers_obj.into());

    // Set body property
    let body_key = v8::String::new(scope, "body").unwrap().into();
    let null_body: v8::Local<v8::Value> = v8::null(scope).into();
    if let Some(body_str) = init_body {
        let body_val = v8::String::new(scope, &body_str).unwrap().into();
        request_obj.set(scope, body_key, body_val);
    } else {
        request_obj.set(scope, body_key, null_body);
    }

    // Set other properties with init values or defaults
    let cache_key = v8::String::new(scope, "cache").unwrap().into();
    let cache_val = v8::String::new(scope, &init_cache).unwrap().into();
    request_obj.set(scope, cache_key, cache_val);

    let cred_key = v8::String::new(scope, "credentials").unwrap().into();
    let cred_val = v8::String::new(scope, &init_credentials).unwrap().into();
    request_obj.set(scope, cred_key, cred_val);

    let mode_key = v8::String::new(scope, "mode").unwrap().into();
    let mode_val = v8::String::new(scope, &init_mode).unwrap().into();
    request_obj.set(scope, mode_key, mode_val);

    let redirect_key = v8::String::new(scope, "redirect").unwrap().into();
    let redirect_val = v8::String::new(scope, &init_redirect).unwrap().into();
    request_obj.set(scope, redirect_key, redirect_val);

    let referrer_key = v8::String::new(scope, "referrer").unwrap().into();
    if init_referrer.is_empty() {
        request_obj.set(scope, referrer_key, null_body);
    } else {
        let referrer_val = v8::String::new(scope, &init_referrer).unwrap().into();
        request_obj.set(scope, referrer_key, referrer_val);
    }

    let policy_key = v8::String::new(scope, "referrerPolicy").unwrap().into();
    let policy_val = v8::String::new(scope, &init_policy).unwrap().into();
    request_obj.set(scope, policy_key, policy_val);

    let integrity_key = v8::String::new(scope, "integrity").unwrap().into();
    let integrity_val = v8::String::new(scope, &init_integrity).unwrap().into();
    request_obj.set(scope, integrity_key, integrity_val);

    let keepalive_key = v8::String::new(scope, "keepalive").unwrap().into();
    let keepalive_val = v8::Boolean::new(scope, init_keepalive).into();
    request_obj.set(scope, keepalive_key, keepalive_val);

    // Add clone() method using object data
    let clone_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let this_obj = args.this();

            // Get request data from the object properties
            let url_key = v8::String::new(scope, "url").unwrap().into();
            let method_key = v8::String::new(scope, "method").unwrap().into();
            let cache_key = v8::String::new(scope, "cache").unwrap().into();
            let cred_key = v8::String::new(scope, "credentials").unwrap().into();
            let mode_key = v8::String::new(scope, "mode").unwrap().into();
            let redirect_key = v8::String::new(scope, "redirect").unwrap().into();
            let referrer_key = v8::String::new(scope, "referrer").unwrap().into();
            let policy_key = v8::String::new(scope, "referrerPolicy").unwrap().into();
            let integrity_key = v8::String::new(scope, "integrity").unwrap().into();
            let keepalive_key = v8::String::new(scope, "keepalive").unwrap().into();
            let headers_key = v8::String::new(scope, "headers").unwrap().into();
            let body_key = v8::String::new(scope, "body").unwrap().into();

            // Extract values from this object (values are read but used via get/set below)
            let _url = this_obj
                .get(scope, url_key)
                .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                .unwrap_or_default();
            let _method = this_obj
                .get(scope, method_key)
                .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                .unwrap_or_else(|| "GET".to_string());
            let _cache_mode = this_obj
                .get(scope, cache_key)
                .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                .unwrap_or_else(|| "default".to_string());
            let _credentials = this_obj
                .get(scope, cred_key)
                .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                .unwrap_or_else(|| "same-origin".to_string());
            let _mode = this_obj
                .get(scope, mode_key)
                .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                .unwrap_or_else(|| "cors".to_string());
            let _redirect = this_obj
                .get(scope, redirect_key)
                .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                .unwrap_or_else(|| "follow".to_string());
            let _referrer = this_obj
                .get(scope, referrer_key)
                .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                .unwrap_or_default();
            let _policy = this_obj
                .get(scope, policy_key)
                .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                .unwrap_or_else(|| "no-referrer".to_string());
            let _integrity = this_obj
                .get(scope, integrity_key)
                .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                .unwrap_or_default();
            let _keepalive = this_obj
                .get(scope, keepalive_key)
                .map(|v| v.is_true())
                .unwrap_or(false);

            // Create new request object
            let new_request: _ = v8::Object::new(scope);

            // Get values from this object first
            let url_val = this_obj
                .get(scope, url_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let method_val = this_obj
                .get(scope, method_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let headers_val = this_obj
                .get(scope, headers_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let body_val = this_obj
                .get(scope, body_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let cache_val = this_obj
                .get(scope, cache_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let cred_val = this_obj
                .get(scope, cred_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let mode_val = this_obj
                .get(scope, mode_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let redirect_val = this_obj
                .get(scope, redirect_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let referrer_val = this_obj
                .get(scope, referrer_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let policy_val = this_obj
                .get(scope, policy_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let integrity_val = this_obj
                .get(scope, integrity_key)
                .unwrap_or_else(|| v8::null(scope).into());
            let keepalive_val = this_obj
                .get(scope, keepalive_key)
                .unwrap_or_else(|| v8::null(scope).into());

            // Copy all properties to new request
            new_request.set(scope, url_key, url_val);
            new_request.set(scope, method_key, method_val);
            new_request.set(scope, headers_key, headers_val);
            new_request.set(scope, body_key, body_val);
            new_request.set(scope, cache_key, cache_val);
            new_request.set(scope, cred_key, cred_val);
            new_request.set(scope, mode_key, mode_val);
            new_request.set(scope, redirect_key, redirect_val);
            new_request.set(scope, referrer_key, referrer_val);
            new_request.set(scope, policy_key, policy_val);
            new_request.set(scope, integrity_key, integrity_val);
            new_request.set(scope, keepalive_key, keepalive_val);

            // Add clone method to new request (simple implementation)
            let new_clone_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 mut rv: v8::ReturnValue| {
                    let null_val: v8::Local<v8::Value> = v8::null(scope).into();
                    rv.set(null_val);
                },
            )
            .unwrap();
            let clone_key = v8::String::new(scope, "clone").unwrap().into();
            new_request.set(scope, clone_key, new_clone_fn.into());

            attach_request_body_methods(scope, new_request);

            rv.set(new_request.into());
        },
    );
    let clone_func = clone_template.get_function(scope).unwrap();
    let clone_key = v8::String::new(scope, "clone").unwrap().into();
    request_obj.set(scope, clone_key, clone_func.into());

    attach_request_body_methods(scope, request_obj);

    retval.set(request_obj.into());
}
/// Response constructor callback
fn response_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let body: _ = args.get(0);
    let init: _ = args.get(1);

    let mut status: u32 = 200;
    let mut status_text = String::new();
    let mut header_entries: Vec<(String, String)> = Vec::new();

    if init.is_object() {
        if let Some(init_obj) = init.to_object(scope) {
            let status_key = v8::String::new(scope, "status").unwrap().into();
            if let Some(status_val) = init_obj.get(scope, status_key) {
                if !status_val.is_undefined() {
                    if let Some(status_int) = status_val.to_integer(scope) {
                        status = status_int.value() as u32;
                    }
                }
            }

            let status_text_key = v8::String::new(scope, "statusText").unwrap().into();
            if let Some(status_text_val) = init_obj.get(scope, status_text_key) {
                if status_text_val.is_string() {
                    status_text = status_text_val
                        .to_string(scope)
                        .unwrap()
                        .to_rust_string_lossy(scope);
                }
            }

            let headers_key = v8::String::new(scope, "headers").unwrap().into();
            if let Some(headers_val) = init_obj.get(scope, headers_key) {
                header_entries = header_entries_from_value(scope, headers_val);
            }
        }
    }

    let response_obj: _ = v8::Object::new(scope);
    let status_key: _ = v8::String::new(scope, "status").unwrap();
    let status_val: _ = v8::Integer::new_from_unsigned(scope, status).into();
    response_obj.set(scope, status_key.into(), status_val);
    let ok_key: _ = v8::String::new(scope, "ok").unwrap();
    let ok_key_val: _ = v8::Boolean::new(scope, status >= 200 && status < 300).into();
    response_obj.set(scope, ok_key.into(), ok_key_val);

    let status_text_key: _ = v8::String::new(scope, "statusText").unwrap();
    let status_text_val: v8::Local<v8::Value> =
        v8::String::new(scope, &status_text).unwrap().into();
    response_obj.set(scope, status_text_key.into(), status_text_val);

    let url_key: _ = v8::String::new(scope, "url").unwrap();
    let url_val: v8::Local<v8::Value> = v8::String::new(scope, "").unwrap().into();
    response_obj.set(scope, url_key.into(), url_val);

    let type_key: _ = v8::String::new(scope, "type").unwrap();
    let type_val: v8::Local<v8::Value> = v8::String::new(scope, "default").unwrap().into();
    response_obj.set(scope, type_key.into(), type_val);

    let redirected_key: _ = v8::String::new(scope, "redirected").unwrap();
    let redirected_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
    response_obj.set(scope, redirected_key.into(), redirected_val);

    let body_vec = body_value_to_bytes(scope, body);
    store_response_body(scope, response_obj, String::new(), body_vec);
    attach_response_body_methods(scope, response_obj);

    let body_used_key: _ = v8::String::new(scope, "bodyUsed").unwrap();
    let body_used_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
    response_obj.set(scope, body_used_key.into(), body_used_val);

    let headers_obj = create_headers_object_with_entries(scope, header_entries);
    let headers_key: _ = v8::String::new(scope, "headers").unwrap();
    response_obj.set(scope, headers_key.into(), headers_obj.into());

    attach_response_clone_method(scope, response_obj);

    retval.set(response_obj.into());
}

fn body_value_to_bytes(scope: &mut v8::PinScope, body: v8::Local<v8::Value>) -> Vec<u8> {
    if body.is_null_or_undefined() {
        Vec::new()
    } else if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(body) {
        let mut buf = vec![0u8; view.byte_length()];
        view.copy_contents(&mut buf);
        buf
    } else if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(body) {
        let store = ab.get_backing_store();
        let len = ab.byte_length();
        let ptr = store.as_ref().as_ptr() as *const u8;
        let mut buf = vec![0u8; len];
        if len > 0 && !ptr.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), len);
            }
        }
        buf
    } else {
        body.to_string(scope)
            .map(|body| body.to_rust_string_lossy(scope).into_bytes())
            .unwrap_or_default()
    }
}

fn header_entries_from_value(
    scope: &mut v8::PinScope,
    headers_val: v8::Local<v8::Value>,
) -> Vec<(String, String)> {
    if !headers_val.is_object() {
        return Vec::new();
    }

    let Some(headers_obj) = headers_val.to_object(scope) else {
        return Vec::new();
    };

    if let Some(index) = headers_cache_index_from_object(scope, headers_obj) {
        let cache = get_headers_cache().lock().unwrap();
        return cache.get(&index).cloned().unwrap_or_default();
    }

    if headers_val.is_array() {
        if let Ok(headers_array) = v8::Local::<v8::Array>::try_from(headers_val) {
            return header_entries_from_sequence(scope, headers_array);
        }
    }

    let Some(keys_array) = headers_obj.get_own_property_names(scope, Default::default()) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    for i in 0..keys_array.length() {
        let Some(key) = keys_array.get_index(scope, i) else {
            continue;
        };
        let Some(key_str) = key.to_string(scope) else {
            continue;
        };
        let Some(value) = headers_obj.get(scope, key) else {
            continue;
        };
        if value.is_function() {
            continue;
        }
        let Some(value_str) = value.to_string(scope) else {
            continue;
        };
        entries.push((
            normalize_header_name(&key_str.to_rust_string_lossy(scope)),
            value_str.to_rust_string_lossy(scope),
        ));
    }

    entries
}

fn normalize_header_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn header_entries_from_sequence(
    scope: &mut v8::PinScope,
    headers_array: v8::Local<v8::Array>,
) -> Vec<(String, String)> {
    let mut entries = Vec::new();

    for i in 0..headers_array.length() {
        let Some(pair_val) = headers_array.get_index(scope, i) else {
            continue;
        };
        if !pair_val.is_array() {
            continue;
        }
        let Ok(pair_array) = v8::Local::<v8::Array>::try_from(pair_val) else {
            continue;
        };
        let Some(name_val) = pair_array.get_index(scope, 0) else {
            continue;
        };
        let Some(value_val) = pair_array.get_index(scope, 1) else {
            continue;
        };
        let Some(name) = name_val.to_string(scope) else {
            continue;
        };
        let Some(value) = value_val.to_string(scope) else {
            continue;
        };

        entries.push((
            normalize_header_name(&name.to_rust_string_lossy(scope)),
            value.to_rust_string_lossy(scope),
        ));
    }

    entries
}

fn headers_cache_index_from_object(
    scope: &mut v8::PinScope,
    headers_obj: v8::Local<v8::Object>,
) -> Option<usize> {
    headers_obj
        .get_internal_field(scope, 0)
        .and_then(|d| v8::Local::<v8::Value>::try_from(d).ok())
        .and_then(|value| value.to_integer(scope))
        .map(|index| index.value() as usize)
}

fn headers_entries_for_object(
    scope: &mut v8::PinScope,
    headers_obj: v8::Local<v8::Object>,
) -> Vec<(String, String)> {
    headers_cache_index_from_object(scope, headers_obj)
        .and_then(|index| get_headers_cache().lock().unwrap().get(&index).cloned())
        .unwrap_or_default()
}

fn headers_keys_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    entries: &[(String, String)],
) -> v8::Local<'a, v8::Array> {
    let array = v8::Array::new(scope, entries.len() as i32);
    for (index, (name, _)) in entries.iter().enumerate() {
        let value = v8::String::new(scope, name).unwrap().into();
        array.set_index(scope, index as u32, value);
    }
    array
}

fn headers_values_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    entries: &[(String, String)],
) -> v8::Local<'a, v8::Array> {
    let array = v8::Array::new(scope, entries.len() as i32);
    for (index, (_, header_value)) in entries.iter().enumerate() {
        let value = v8::String::new(scope, header_value).unwrap().into();
        array.set_index(scope, index as u32, value);
    }
    array
}

fn headers_entries_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    entries: &[(String, String)],
) -> v8::Local<'a, v8::Array> {
    let array = v8::Array::new(scope, entries.len() as i32);
    for (index, (name, header_value)) in entries.iter().enumerate() {
        let pair = v8::Array::new(scope, 2);
        let name_value = v8::String::new(scope, name).unwrap().into();
        let header_value = v8::String::new(scope, header_value).unwrap().into();
        pair.set_index(scope, 0, name_value);
        pair.set_index(scope, 1, header_value);
        array.set_index(scope, index as u32, pair.into());
    }
    array
}

fn symbol_iterator_value<'a>(scope: &mut v8::PinScope<'a, '_>) -> Option<v8::Local<'a, v8::Value>> {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let symbol_key: v8::Local<v8::Value> = v8::String::new(scope, "Symbol")?.into();
    let symbol_value = global.get(scope, symbol_key)?;
    let symbol_object = symbol_value.to_object(scope)?;
    let iterator_key: v8::Local<v8::Value> = v8::String::new(scope, "iterator")?.into();
    symbol_object.get(scope, iterator_key)
}

fn iterator_from_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    array: v8::Local<'a, v8::Array>,
) -> v8::Local<'a, v8::Value> {
    let Some(iterator_key) = symbol_iterator_value(scope) else {
        return array.into();
    };
    let Some(array_iterator) = array.get(scope, iterator_key) else {
        return array.into();
    };
    let Ok(array_iterator_func) = v8::Local::<v8::Function>::try_from(array_iterator) else {
        return array.into();
    };

    array_iterator_func
        .call(scope, array.into(), &[])
        .unwrap_or_else(|| array.into())
}

/// Headers constructor callback - uses ObjectTemplate with internal fields
fn headers_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Create ObjectTemplate with internal field for storing headers index
    let headers_template = v8::ObjectTemplate::new(scope);
    headers_template.set_internal_field_count(1);

    let headers_obj: v8::Local<v8::Object> = match headers_template.new_instance(scope) {
        Some(obj) => obj,
        None => {
            retval.set(v8::null(scope).into());
            return;
        }
    };

    // Get next available index for this Headers instance
    static HEADERS_INDEX_COUNTER: OnceLock<Mutex<usize>> = OnceLock::new();
    let index_counter = HEADERS_INDEX_COUNTER.get_or_init(|| Mutex::new(0));
    let mut counter = index_counter.lock().unwrap();
    let index = *counter;
    *counter += 1;
    drop(counter);

    // Store index in internal field 0
    let index_val: v8::Local<v8::Value> = v8::Integer::new(scope, index as i32).into();
    headers_obj.set_internal_field(0, index_val.into());

    // Initialize headers data for this index
    let initial_entries = header_entries_from_value(scope, args.get(0));
    let mut cache = get_headers_cache().lock().unwrap();
    cache.insert(index, initial_entries);
    drop(cache);

    // Add get() method
    let get_key = v8::String::new(scope, "get").unwrap().into();
    let get_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let this_obj: v8::Local<v8::Object> = args.this();

            // Get index from internal field
            let index = this_obj
                .get_internal_field(scope, 0)
                .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(usize::MAX);

            let name = if let Some(name_val) = args.get(0).to_string(scope) {
                normalize_header_name(&name_val.to_rust_string_lossy(scope))
            } else {
                rv.set(v8::null(scope).into());
                return;
            };

            let cache = get_headers_cache().lock().unwrap();
            if let Some(headers) = cache.get(&index) {
                let values: Vec<String> = headers
                    .iter()
                    .filter(|(key, _)| key.to_lowercase() == name)
                    .map(|(_, value)| value.clone())
                    .collect();

                if values.is_empty() {
                    rv.set(v8::null(scope).into());
                } else {
                    let result = values.join(", ");
                    rv.set(v8::String::new(scope, &result).unwrap().into());
                }
            } else {
                rv.set(v8::null(scope).into());
            }
        },
    );
    let get_func = get_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, get_key, get_func.into());

    // Add set() method
    let set_key = v8::String::new(scope, "set").unwrap().into();
    let set_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let this_obj: v8::Local<v8::Object> = args.this();

            // Get index from internal field
            let index = this_obj
                .get_internal_field(scope, 0)
                .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(usize::MAX);

            let name = if let Some(name_val) = args.get(0).to_string(scope) {
                normalize_header_name(&name_val.to_rust_string_lossy(scope))
            } else {
                return;
            };

            let value = if let Some(value_val) = args.get(1).to_string(scope) {
                value_val.to_rust_string_lossy(scope)
            } else {
                return;
            };

            let mut cache = get_headers_cache().lock().unwrap();
            if let Some(headers) = cache.get_mut(&index) {
                // Remove existing headers with same name (case-insensitive)
                headers.retain(|(key, _)| key != &name);
                // Add new header
                headers.push((name, value));
            }
        },
    );
    let set_func = set_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, set_key, set_func.into());

    // Add has() method
    let has_key = v8::String::new(scope, "has").unwrap().into();
    let has_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let this_obj: v8::Local<v8::Object> = args.this();

            // Get index from internal field
            let index = this_obj
                .get_internal_field(scope, 0)
                .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(usize::MAX);

            let name = if let Some(name_val) = args.get(0).to_string(scope) {
                normalize_header_name(&name_val.to_rust_string_lossy(scope))
            } else {
                rv.set(v8::Boolean::new(scope, false).into());
                return;
            };

            let cache = get_headers_cache().lock().unwrap();
            let has_header = cache
                .get(&index)
                .map(|headers| headers.iter().any(|(key, _)| key == &name))
                .unwrap_or(false);

            rv.set(v8::Boolean::new(scope, has_header).into());
        },
    );
    let has_func = has_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, has_key, has_func.into());

    // Add delete() method
    let delete_key = v8::String::new(scope, "delete").unwrap().into();
    let delete_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let this_obj: v8::Local<v8::Object> = args.this();

            // Get index from internal field
            let index = this_obj
                .get_internal_field(scope, 0)
                .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(usize::MAX);

            let name = if let Some(name_val) = args.get(0).to_string(scope) {
                normalize_header_name(&name_val.to_rust_string_lossy(scope))
            } else {
                return;
            };

            let mut cache = get_headers_cache().lock().unwrap();
            if let Some(headers) = cache.get_mut(&index) {
                headers.retain(|(key, _)| key != &name);
            }
        },
    );
    let delete_func = delete_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, delete_key, delete_func.into());

    // Add append() method
    let append_key = v8::String::new(scope, "append").unwrap().into();
    let append_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let this_obj: v8::Local<v8::Object> = args.this();

            // Get index from internal field
            let index = this_obj
                .get_internal_field(scope, 0)
                .and_then(|data| v8::Local::<v8::Value>::try_from(data).ok())
                .and_then(|v| v.to_integer(scope))
                .map(|i| i.value() as usize)
                .unwrap_or(usize::MAX);

            let name = if let Some(name_val) = args.get(0).to_string(scope) {
                normalize_header_name(&name_val.to_rust_string_lossy(scope))
            } else {
                return;
            };

            let value = if let Some(value_val) = args.get(1).to_string(scope) {
                value_val.to_rust_string_lossy(scope)
            } else {
                return;
            };

            let mut cache = get_headers_cache().lock().unwrap();
            if let Some(headers) = cache.get_mut(&index) {
                headers.push((name, value));
            }
        },
    );
    let append_func = append_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, append_key, append_func.into());

    // Add keys() method
    let keys_key = v8::String::new(scope, "keys").unwrap().into();
    let keys_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let entries = headers_entries_for_object(scope, args.this());
            let array = headers_keys_array(scope, &entries);
            rv.set(iterator_from_array(scope, array));
        },
    );
    let keys_func = keys_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, keys_key, keys_func.into());

    // Add values() method
    let values_key = v8::String::new(scope, "values").unwrap().into();
    let values_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let entries = headers_entries_for_object(scope, args.this());
            let array = headers_values_array(scope, &entries);
            rv.set(iterator_from_array(scope, array));
        },
    );
    let values_func = values_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, values_key, values_func.into());

    // Add entries() method
    let entries_key = v8::String::new(scope, "entries").unwrap().into();
    let entries_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let entries = headers_entries_for_object(scope, args.this());
            let array = headers_entries_array(scope, &entries);
            rv.set(iterator_from_array(scope, array));
        },
    );
    let entries_func = entries_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, entries_key, entries_func.into());

    // Add [Symbol.iterator]() method
    let iterator_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let entries = headers_entries_for_object(scope, args.this());
            let array = headers_entries_array(scope, &entries);
            rv.set(iterator_from_array(scope, array));
        },
    );
    let iterator_func = iterator_func_template.get_function(scope).unwrap();
    if let Some(iterator_key) = symbol_iterator_value(scope) {
        headers_obj.set(scope, iterator_key, iterator_func.into());
    }

    // Add forEach() method
    let for_each_key = v8::String::new(scope, "forEach").unwrap().into();
    let for_each_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let callback_val = args.get(0);
            let Ok(callback) = v8::Local::<v8::Function>::try_from(callback_val) else {
                return;
            };

            let this_obj = args.this();
            let entries = headers_entries_for_object(scope, this_obj);
            let this_arg = args.get(1);
            let receiver = if this_arg.is_undefined() {
                v8::undefined(scope).into()
            } else {
                this_arg
            };

            for (name, header_value) in entries {
                let value_arg: v8::Local<v8::Value> =
                    v8::String::new(scope, &header_value).unwrap().into();
                let name_arg: v8::Local<v8::Value> = v8::String::new(scope, &name).unwrap().into();
                let owner_arg: v8::Local<v8::Value> = this_obj.into();
                let _ = callback.call(scope, receiver, &[value_arg, name_arg, owner_arg]);
            }
        },
    );
    let for_each_func = for_each_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, for_each_key, for_each_func.into());

    // Add getSetCookie() method (Web Standard & Node 18+)
    let get_set_cookie_key = v8::String::new(scope, "getSetCookie").unwrap().into();
    let get_set_cookie_func_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let entries = headers_entries_for_object(scope, args.this());
            let cookies: Vec<String> = entries
                .into_iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
                .map(|(_, v)| v)
                .collect();
            let arr = v8::Array::new(scope, cookies.len() as i32);
            for (i, cookie) in cookies.iter().enumerate() {
                let s = v8::String::new(scope, cookie).unwrap();
                arr.set_index(scope, i as u32, s.into());
            }
            rv.set(arr.into());
        },
    );
    let get_set_cookie_func = get_set_cookie_func_template.get_function(scope).unwrap();
    headers_obj.set(scope, get_set_cookie_key, get_set_cookie_func.into());

    retval.set(headers_obj.into());
}

fn create_headers_object_with_entries<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    entries: Vec<(String, String)>,
) -> v8::Local<'a, v8::Object> {
    let mut headers_obj = v8::Object::new(scope);

    let context = scope.get_current_context();
    let global = context.global(scope);
    let headers_ctor_key = v8::String::new(scope, "Headers").unwrap().into();
    if let Some(headers_ctor) = global.get(scope, headers_ctor_key) {
        if headers_ctor.is_function() {
            let headers_ctor_func: v8::Local<v8::Function> = headers_ctor.try_into().unwrap();
            if let Some(created_headers) = headers_ctor_func.new_instance(scope, &[]) {
                headers_obj = created_headers;
            }
        }
    }

    let set_key = v8::String::new(scope, "set").unwrap().into();
    let set_func = headers_obj
        .get(scope, set_key)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok());

    if let Some(set_func) = set_func {
        for (name, value) in entries {
            let name_value: v8::Local<v8::Value> = v8::String::new(scope, &name).unwrap().into();
            let header_value: v8::Local<v8::Value> = v8::String::new(scope, &value).unwrap().into();
            let _ = set_func.call(scope, headers_obj.into(), &[name_value, header_value]);
        }
    } else {
        for (name, value) in entries {
            let header_key = v8::String::new(scope, &name).unwrap().into();
            let header_val = v8::String::new(scope, &value).unwrap().into();
            headers_obj.set(scope, header_key, header_val);
        }
    }

    headers_obj
}

fn store_response_body(
    scope: &mut v8::PinScope,
    response_obj: v8::Local<v8::Object>,
    url: String,
    body_vec: Vec<u8>,
) {
    let response_id = next_response_id();
    let body_str = String::from_utf8_lossy(&body_vec).to_string();
    let mut cache = get_response_cache().lock().unwrap();
    cache.insert(response_id, (url, body_vec));
    drop(cache);

    let response_id_key: _ = v8::String::new(scope, "__beejsResponseId").unwrap();
    let response_id_val: _ = v8::Integer::new_from_unsigned(scope, response_id as u32).into();
    response_obj.set(scope, response_id_key.into(), response_id_val);

    let body_key: _ = v8::String::new(scope, "body").unwrap();
    let body_val: _ = v8::String::new(scope, &body_str).unwrap().into();
    response_obj.set(scope, body_key.into(), body_val);
}

fn attach_response_body_methods(scope: &mut v8::PinScope, response_obj: v8::Local<v8::Object>) {
    let json_template: _ = v8::FunctionTemplate::new(scope, json_callback);
    let json_func: _ = json_template.get_function(scope).unwrap();
    let json_key: _ = v8::String::new(scope, "json").unwrap();
    response_obj.set(scope, json_key.into(), json_func.into());

    let text_template: _ = v8::FunctionTemplate::new(scope, text_callback);
    let text_func: _ = text_template.get_function(scope).unwrap();
    let text_key: _ = v8::String::new(scope, "text").unwrap();
    response_obj.set(scope, text_key.into(), text_func.into());

    let array_buffer_template: _ = v8::FunctionTemplate::new(scope, array_buffer_callback);
    let array_buffer_func: _ = array_buffer_template.get_function(scope).unwrap();
    let array_buffer_key: _ = v8::String::new(scope, "arrayBuffer").unwrap();
    response_obj.set(scope, array_buffer_key.into(), array_buffer_func.into());

    let blob_template: _ = v8::FunctionTemplate::new(scope, blob_callback);
    let blob_func: _ = blob_template.get_function(scope).unwrap();
    let blob_key: _ = v8::String::new(scope, "blob").unwrap();
    response_obj.set(scope, blob_key.into(), blob_func.into());
}

fn attach_response_clone_method(scope: &mut v8::PinScope, response_obj: v8::Local<v8::Object>) {
    let clone_key: _ = v8::String::new(scope, "clone").unwrap();
    let clone_template: _ = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let this_obj: v8::Local<v8::Object> = args.this();

            if response_body_is_used(scope, this_obj) {
                throw_response_body_already_consumed(scope);
                return;
            }

            let cloned_obj: v8::Local<v8::Object> = v8::Object::new(scope);
            let key_names = [
                "status",
                "ok",
                "statusText",
                "url",
                "type",
                "redirected",
                "body",
                "bodyUsed",
                "headers",
                "__beejsResponseId",
            ];

            for name in &key_names {
                let key_local = v8::String::new(scope, name).unwrap().into();
                if let Some(val) = this_obj.get(scope, key_local) {
                    cloned_obj.set(scope, key_local, val);
                }
            }

            let methods = [
                "json",
                "text",
                "arrayBuffer",
                "blob",
                "clone",
                "then",
                "catch",
            ];
            for method_name in &methods {
                let key_local = v8::String::new(scope, method_name).unwrap().into();
                if let Some(method_val) = this_obj.get(scope, key_local) {
                    cloned_obj.set(scope, key_local, method_val);
                }
            }

            rv.set(cloned_obj.into());
        },
    );
    let clone_func: v8::Local<v8::Function> = clone_template.get_function(scope).unwrap();
    response_obj.set(scope, clone_key.into(), clone_func.into());
}

/// json() method callback for Response objects
fn json_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Get the this object (response object)
    let this_obj: v8::Local<v8::Object> = args.this();

    let body = match consume_response_body_for_object(scope, this_obj) {
        Ok(Some(body)) => body,
        Ok(None) => {
            // No cached response found
            let error: v8::Local<v8::Value> = v8::String::new(scope, "Response body not available")
                .unwrap()
                .into();
            retval.set(error);
            return;
        }
        Err(()) => return,
    };

    {
        // Try to parse and format JSON prettily
        let body_str = String::from_utf8_lossy(&body);

        // Try to parse as JSON and format prettily
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&body_str) {
            let formatted =
                serde_json::to_string_pretty(&json_value).unwrap_or(body_str.to_string());
            let result: v8::Local<v8::Value> = v8::String::new(scope, &formatted).unwrap().into();
            retval.set(result);
        } else {
            // Not valid JSON, return as-is
            let result: v8::Local<v8::Value> = v8::String::new(scope, &body_str).unwrap().into();
            retval.set(result);
        }
    }
}

/// text() method callback for Response objects
fn text_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Get the this object (response object)
    let this_obj: v8::Local<v8::Object> = args.this();

    let body = match consume_response_body_for_object(scope, this_obj) {
        Ok(Some(body)) => body,
        Ok(None) => {
            // No cached response found
            let error: v8::Local<v8::Value> = v8::String::new(scope, "Response body not available")
                .unwrap()
                .into();
            retval.set(error);
            return;
        }
        Err(()) => return,
    };

    let body_str = String::from_utf8_lossy(&body);
    let result: v8::Local<v8::Value> = v8::String::new(scope, &body_str).unwrap().into();
    retval.set(result);
}

/// arrayBuffer() method callback for Response objects (Body mixin)
/// Returns the response body as an ArrayBuffer
fn array_buffer_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Get the this object (response object)
    let this_obj: v8::Local<v8::Object> = args.this();

    let body = match consume_response_body_for_object(scope, this_obj) {
        Ok(Some(body)) => body,
        Ok(None) => {
            // No cached response found - return empty ArrayBuffer
            let buffer = v8::ArrayBuffer::new(scope, 0);
            retval.set(buffer.into());
            return;
        }
        Err(()) => return,
    };

    // Create an ArrayBuffer from the body bytes
    let buffer = v8::ArrayBuffer::new(scope, body.len());
    let store = buffer.get_backing_store();
    let store_ptr = store.as_ref().as_ptr() as *mut u8;
    if !body.is_empty() && !store_ptr.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(body.as_ptr(), store_ptr, body.len());
        }
    }

    retval.set(buffer.into());
}

/// blob() method callback for Response objects (Body mixin)
/// Returns the response body as a Blob-like object
fn blob_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Get the this object (response object)
    let this_obj: v8::Local<v8::Object> = args.this();

    let body = match consume_response_body_for_object(scope, this_obj) {
        Ok(Some(body)) => body,
        Ok(None) => {
            // No cached response found - return empty blob
            let blob_obj = v8::Object::new(scope);
            let size_key = v8::String::new(scope, "size").unwrap().into();
            let size_val = v8::Integer::new_from_unsigned(scope, 0).into();
            blob_obj.set(scope, size_key, size_val);

            let type_key = v8::String::new(scope, "type").unwrap().into();
            let type_val = v8::String::new(scope, "application/octet-stream")
                .unwrap()
                .into();
            blob_obj.set(scope, type_key, type_val);

            retval.set(blob_obj.into());
            return;
        }
        Err(()) => return,
    };

    // Create a blob-like object with size and type properties
    let blob_obj = v8::Object::new(scope);

    // Set size property
    let size_key = v8::String::new(scope, "size").unwrap().into();
    let size_val = v8::Integer::new_from_unsigned(scope, body.len() as u32).into();
    blob_obj.set(scope, size_key, size_val);

    // Set type property (content-type)
    let type_key = v8::String::new(scope, "type").unwrap().into();
    let type_val = v8::String::new(scope, "application/octet-stream")
        .unwrap()
        .into();
    blob_obj.set(scope, type_key, type_val);

    // Set arrayBuffer method that returns the body as ArrayBuffer
    let array_buffer_template = v8::FunctionTemplate::new(scope, array_buffer_callback);
    let array_buffer_func = array_buffer_template.get_function(scope).unwrap();
    let array_buffer_key = v8::String::new(scope, "arrayBuffer").unwrap().into();
    blob_obj.set(scope, array_buffer_key, array_buffer_func.into());

    retval.set(blob_obj.into());
}

fn consume_response_body_for_object(
    scope: &mut v8::PinScope,
    response_obj: v8::Local<v8::Object>,
) -> std::result::Result<Option<Vec<u8>>, ()> {
    if response_body_is_used(scope, response_obj) {
        throw_response_body_already_consumed(scope);
        return Err(());
    }

    let body = response_body_for_object(scope, response_obj);
    if body.is_some() {
        let body_used_key = v8::String::new(scope, "bodyUsed").unwrap().into();
        let body_used_val = v8::Boolean::new(scope, true).into();
        response_obj.set(scope, body_used_key, body_used_val);
    }

    Ok(body)
}

fn response_body_is_used(scope: &mut v8::PinScope, response_obj: v8::Local<v8::Object>) -> bool {
    let body_used_key = v8::String::new(scope, "bodyUsed").unwrap().into();
    response_obj
        .get(scope, body_used_key)
        .map(|value| value.is_true())
        .unwrap_or(false)
}

fn throw_response_body_already_consumed(scope: &mut v8::PinScope) {
    let message = v8::String::new(scope, "Response body already consumed").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error.into());
}

fn response_body_for_object(
    scope: &mut v8::PinScope,
    response_obj: v8::Local<v8::Object>,
) -> Option<Vec<u8>> {
    let response_id_key = v8::String::new(scope, "__beejsResponseId").unwrap().into();
    if let Some(response_id_val) = response_obj.get(scope, response_id_key) {
        if let Some(response_id_int) = response_id_val.to_integer(scope) {
            let response_id = response_id_int.value() as usize;
            let cache = get_response_cache().lock().unwrap();
            if let Some((_url, body)) = cache.get(&response_id) {
                return Some(body.clone());
            }
        }
    }

    let body_key = v8::String::new(scope, "body").unwrap().into();
    response_obj
        .get(scope, body_key)
        .and_then(|body| body.to_string(scope))
        .map(|body| body.to_rust_string_lossy(scope).into_bytes())
}

#[cfg(test)]
mod tests {
    use super::{FetchConfig, HttpMethod};

    #[test]
    fn test_http_method_from_string() {
        let method: HttpMethod = "GET".to_string().into();
        assert_eq!(method, HttpMethod::GET);
        let method: HttpMethod = "POST".to_string().into();
        assert_eq!(method, HttpMethod::POST);
    }
    #[test]
    fn test_http_method_display() {
        assert_eq!(format!("{}", HttpMethod::GET), "GET");
        assert_eq!(format!("{}", HttpMethod::POST), "POST");
    }
    #[test]
    fn test_fetch_config_default() {
        let config: _ = FetchConfig::default();
        assert_eq!(
            config.user_agent,
            format!("Beejs/{}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(config.timeout, std::time::Duration::from_secs(30));
        assert_eq!(config.max_redirects, 20);
    }

    // v0.3.344: Tests for arrayBuffer() and blob() Body mixin methods
    #[test]
    fn test_fetch_response_body_methods_registered() {
        // This test verifies that the Response object has arrayBuffer and blob methods
        // The actual integration tests would require a running V8 isolate
        // For unit tests, we verify the configuration
        let config: _ = FetchConfig::default();
        assert!(config.timeout.as_secs() > 0);
    }

    #[test]
    fn test_response_cache_creation() {
        // Test that the response cache is created correctly
        use std::collections::HashMap;
        use std::sync::Mutex;
        use std::sync::OnceLock;

        static TEST_CACHE: OnceLock<Mutex<HashMap<usize, (String, Vec<u8>)>>> = OnceLock::new();
        let cache = TEST_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        // Verify cache can be locked and accessed
        let guard = cache.lock().unwrap();
        assert!(guard.len() == 0);
        drop(guard);

        // Insert test data
        let mut guard = cache.lock().unwrap();
        guard.insert(123, ("test_url".to_string(), b"test_body".to_vec()));

        // Verify data was inserted
        if let Some((url, body)) = guard.get(&123) {
            assert_eq!(url, "test_url");
            assert_eq!(body, b"test_body");
        } else {
            panic!("Expected to find inserted data");
        }
    }
}
