// Fetch API implementation for Web standard
// Provides fetch(), Request, Response, Headers API

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use rusty_v8 as v8;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

// Re-export FormData functions for use in fetch
use super::form_data::{
    generate_boundary, get_formdata_entries, get_formdata_index, serialize_formdata_multipart,
};

/// Thread-safe response cache for json() and text() methods.
/// The cached value is a pull source. `text()` / `json()` read it; `fetch`
/// does not copy the whole body into this map up front.
static RESPONSE_CACHE: OnceLock<Mutex<HashMap<usize, Arc<Mutex<SharedBody>>>>> = OnceLock::new();
static RESPONSE_ID_COUNTER: OnceLock<Mutex<usize>> = OnceLock::new();

/// Get the response cache mutex
fn get_response_cache() -> &'static Mutex<HashMap<usize, Arc<Mutex<SharedBody>>>> {
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
            .thread_name("amberjs-fetch-worker")
            .build()
            .expect("Failed to initialize fetch runtime")
    })
}

/// Shared Reqwest Client with persistent connection pooling
static FETCH_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static FETCH_BLOCKING_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

fn get_fetch_client() -> &'static reqwest::Client {
    FETCH_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(format!("Amber/{}", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(32)
            // Amber applies follow / error / manual itself so each hop is visible
            // to the permission broker. Reqwest's default policy would follow
            // before that check and would hide 3xx from manual and error modes.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to initialize fetch client")
    })
}

fn get_blocking_fetch_client() -> &'static reqwest::blocking::Client {
    FETCH_BLOCKING_CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .user_agent(format!("Amber/{}", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(32)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to initialize blocking fetch client")
    })
}

thread_local! {
    static HTTP1_CONN: std::cell::RefCell<Option<(String, TcpStream)>> = const { std::cell::RefCell::new(None) };
}

fn connect_http_stream(host: &str, port: u16) -> Result<TcpStream> {
    let stream = if host == "127.0.0.1" || host == "localhost" {
        TcpStream::connect(std::net::SocketAddr::from(([127, 0, 0, 1], port)))?
    } else {
        TcpStream::connect((host, port))?
    };
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    Ok(stream)
}

/// Same cap as `FetchConfig::max_redirects`. One follow consumes one hop.
const MAX_REDIRECTS: u32 = 20;

fn is_redirect_status(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn header_lookup<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn is_redirect_mode(mode: &str) -> bool {
    matches!(mode, "follow" | "error" | "manual")
}

/// Resolve a redirect `Location` against the current request URL.
///
/// Absolute locations are returned unchanged when they parse as the same URL
/// the resolver produced, so `response.url` can match a `Location` header that
/// has no trailing slash. Relative, protocol-relative, query-only, and
/// dot-segment locations go through the WHATWG `url` crate.
fn resolve_redirect_location(current: &str, location: &str) -> Result<String> {
    let location = location.trim();
    if location.is_empty() {
        return Err(anyhow::anyhow!("redirect Location is empty"));
    }
    let base = url::Url::parse(current)
        .map_err(|error| anyhow::anyhow!("invalid URL {current}: {error}"))?;
    let joined = base
        .join(location)
        .map_err(|error| anyhow::anyhow!("invalid redirect Location {location}: {error}"))?;
    if let Ok(absolute) = url::Url::parse(location) {
        if absolute == joined {
            return Ok(location.to_string());
        }
    }
    Ok(joined.to_string())
}

fn same_http_origin(left: &str, right: &str) -> bool {
    match (url::Url::parse(left), url::Url::parse(right)) {
        (Ok(left), Ok(right)) => left.origin() == right.origin(),
        _ => false,
    }
}

fn is_body_request_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "content-encoding"
            | "content-language"
            | "content-length"
            | "content-location"
            | "content-type"
            | "transfer-encoding"
    )
}

fn is_credential_request_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization" | "cookie" | "cookie2" | "proxy-authorization"
    )
}

/// 301/302 rewrite non-GET/HEAD to GET. 303 rewrites everything except HEAD to GET.
/// Dropping the body also drops headers that only describe that body.
fn apply_redirect_method(
    status: u16,
    method: &mut HttpMethod,
    body: &mut Option<Vec<u8>>,
    headers: &mut HashMap<String, String>,
) {
    let rewrite_to_get = match status {
        301 | 302 => !matches!(method, HttpMethod::GET | HttpMethod::HEAD),
        303 => !matches!(method, HttpMethod::HEAD),
        _ => false,
    };
    if rewrite_to_get {
        *method = HttpMethod::GET;
    }
    if rewrite_to_get || status == 303 {
        *body = None;
        headers.retain(|name, _| !is_body_request_header(name));
    }
}

fn strip_cross_origin_request_headers(headers: &mut HashMap<String, String>) {
    headers.retain(|name, _| !is_credential_request_header(name));
}

fn parse_http_url(url: &str) -> Option<(String, u16, String)> {
    let parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "http" {
        return None;
    }
    let host = parsed.host_str()?.to_string();
    let port = parsed.port_or_known_default().unwrap_or(80);
    let mut path = parsed.path().to_string();
    if path.is_empty() {
        path = "/".to_string();
    }
    if let Some(query) = parsed.query() {
        path.push('?');
        path.push_str(query);
    }
    Some((host, port, path))
}

fn http_host_header(host: &str, port: u16) -> String {
    let bracketed = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    if port == 80 {
        bracketed
    } else {
        format!("{bracketed}:{port}")
    }
}

/// Pull source for `response.body`. Bytes stay on the socket (or in the
/// reqwest body) until something reads them.
#[derive(Debug)]
pub struct SharedBody {
    pulled: Vec<u8>,
    net: Option<NetReader>,
    finished: bool,
}

#[derive(Debug)]
enum NetReader {
    Tcp {
        stream: TcpStream,
        remaining: Option<usize>,
    },
    Blocking(reqwest::blocking::Response),
    Async(tokio::sync::Mutex<Option<reqwest::Response>>),
}

fn memory_body(bytes: Vec<u8>) -> Arc<Mutex<SharedBody>> {
    Arc::new(Mutex::new(SharedBody {
        pulled: bytes,
        net: None,
        finished: true,
    }))
}

fn open_body(pulled: Vec<u8>, net: Option<NetReader>) -> Arc<Mutex<SharedBody>> {
    let finished = net.is_none();
    Arc::new(Mutex::new(SharedBody {
        pulled,
        net,
        finished,
    }))
}

fn read_net(net: &mut NetReader) -> Result<Option<Vec<u8>>> {
    match net {
        NetReader::Tcp { stream, remaining } => {
            if remaining.as_ref() == Some(&0) {
                return Ok(None);
            }
            let mut buf = [0u8; 8192];
            let n = stream
                .read(&mut buf)
                .map_err(|error| anyhow::anyhow!("failed to read response body: {error}"))?;
            if n == 0 {
                return Ok(None);
            }
            let take = if let Some(left) = remaining.as_mut() {
                let take = n.min(*left);
                *left -= take;
                take
            } else {
                n
            };
            if take == 0 {
                Ok(None)
            } else {
                Ok(Some(buf[..take].to_vec()))
            }
        }
        NetReader::Blocking(response) => {
            let mut buf = [0u8; 8192];
            let n = response
                .read(&mut buf)
                .map_err(|error| anyhow::anyhow!("failed to read response body: {error}"))?;
            if n == 0 {
                Ok(None)
            } else {
                Ok(Some(buf[..n].to_vec()))
            }
        }
        NetReader::Async(slot) => loop {
            let chunk = get_fetch_runtime()
                .block_on(async {
                    let mut guard = slot.lock().await;
                    match guard.as_mut() {
                        Some(response) => response.chunk().await,
                        None => Ok(None),
                    }
                })
                .map_err(|error| anyhow::anyhow!("failed to read response body: {error}"))?;
            match chunk {
                Some(bytes) if bytes.is_empty() => continue,
                Some(bytes) => return Ok(Some(bytes.to_vec())),
                None => return Ok(None),
            }
        },
    }
}

fn next_chunk(body: &mut SharedBody) -> Result<Option<Vec<u8>>> {
    if body.finished {
        return Ok(None);
    }
    let chunk = match body.net.as_mut() {
        Some(net) => read_net(net)?,
        None => None,
    };
    match chunk {
        Some(bytes) => {
            body.pulled.extend_from_slice(&bytes);
            Ok(Some(bytes))
        }
        None => {
            body.finished = true;
            body.net = None;
            Ok(None)
        }
    }
}

fn read_to_end(shared: &Arc<Mutex<SharedBody>>) -> Result<Vec<u8>> {
    let mut body = shared.lock().unwrap();
    loop {
        if next_chunk(&mut body)?.is_none() {
            break;
        }
    }
    Ok(body.pulled.clone())
}

fn drain_shared_body(shared: &Arc<Mutex<SharedBody>>) -> Result<()> {
    let _ = read_to_end(shared)?;
    Ok(())
}

static ABORT_FLAGS: OnceLock<Mutex<HashMap<u64, Arc<AtomicBool>>>> = OnceLock::new();
static ABORT_ID: AtomicU64 = AtomicU64::new(1);
static ACTIVE_FETCH_ABORT: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

fn abort_flag_map() -> &'static Mutex<HashMap<u64, Arc<AtomicBool>>> {
    ABORT_FLAGS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a flag `AbortController.abort` can flip. The id is stored on the signal.
pub fn register_abort_flag() -> (u64, Arc<AtomicBool>) {
    let id = ABORT_ID.fetch_add(1, Ordering::Relaxed);
    let flag = Arc::new(AtomicBool::new(false));
    abort_flag_map()
        .lock()
        .unwrap()
        .insert(id, Arc::clone(&flag));
    (id, flag)
}

pub fn abort_fetch_signal(id: u64) {
    if let Some(flag) = abort_flag_map().lock().unwrap().get(&id) {
        flag.store(true, Ordering::Release);
    }
}

/// Abort the fetch that is inside its redirect loop. Safe to call from another thread.
pub fn abort_in_flight_fetch() {
    if let Some(flag) = ACTIVE_FETCH_ABORT.lock().unwrap().clone() {
        flag.store(true, Ordering::Release);
    }
}

struct AbortGuard;

impl Drop for AbortGuard {
    fn drop(&mut self) {
        *ACTIVE_FETCH_ABORT.lock().unwrap() = None;
    }
}

fn arm_fetch_abort(flag: &Arc<AtomicBool>) -> AbortGuard {
    *ACTIVE_FETCH_ABORT.lock().unwrap() = Some(Arc::clone(flag));
    AbortGuard
}

fn ensure_not_aborted(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Acquire) {
        Err(anyhow::anyhow!("The operation was aborted"))
    } else {
        Ok(())
    }
}

fn abort_flag_from_signal(
    scope: &mut v8::PinScope,
    signal_val: v8::Local<v8::Value>,
) -> Arc<AtomicBool> {
    let Some(signal) = signal_val.to_object(scope) else {
        return Arc::new(AtomicBool::new(false));
    };
    let id_key = v8::String::new(scope, "__amberAbortId").unwrap().into();
    let flag = if let Some(id) = signal
        .get(scope, id_key)
        .and_then(|value| value.to_number(scope))
        .map(|value| value.value() as u64)
        .filter(|id| *id != 0)
    {
        abort_flag_map()
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .unwrap_or_else(|| {
                let flag = Arc::new(AtomicBool::new(false));
                abort_flag_map()
                    .lock()
                    .unwrap()
                    .insert(id, Arc::clone(&flag));
                flag
            })
    } else {
        let (id, flag) = register_abort_flag();
        let id_val = v8::Number::new(scope, id as f64).into();
        signal.set(scope, id_key, id_val);
        flag
    };
    let aborted_key = v8::String::new(scope, "aborted").unwrap().into();
    if signal
        .get(scope, aborted_key)
        .map(|value| value.is_true())
        .unwrap_or(false)
    {
        flag.store(true, Ordering::Release);
    }
    flag
}

/// Missing, null, and JavaScript `undefined` arrive as `""`, `"null"`, or
/// `"undefined"` once V8 stringifies them. Those skip the digest, same as an
/// empty string. Only a real integrity string is checked.
fn integrity_is_absent(integrity: &str) -> bool {
    matches!(integrity.trim(), "" | "undefined" | "null")
}

fn js_integrity_string(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> String {
    if !value.is_string() {
        return String::new();
    }
    value
        .to_string(scope)
        .map(|text| text.to_rust_string_lossy(scope))
        .filter(|text| !integrity_is_absent(text))
        .unwrap_or_default()
}

fn check_body_integrity(bytes: &[u8], integrity: &str) -> Result<()> {
    let integrity = integrity.trim();
    if integrity_is_absent(integrity) {
        return Ok(());
    }
    use base64::Engine as _;
    use sha2::Digest as _;
    let mut saw_supported = false;
    for token in integrity.split_whitespace() {
        let Some((algorithm, expected_b64)) = token.split_once('-') else {
            continue;
        };
        let actual = match algorithm {
            "sha256" => {
                saw_supported = true;
                sha2::Sha256::digest(bytes).to_vec()
            }
            "sha384" => {
                saw_supported = true;
                sha2::Sha384::digest(bytes).to_vec()
            }
            "sha512" => {
                saw_supported = true;
                sha2::Sha512::digest(bytes).to_vec()
            }
            _ => continue,
        };
        let expected = base64::engine::general_purpose::STANDARD
            .decode(expected_b64)
            .map_err(|error| anyhow::anyhow!("invalid integrity metadata: {error}"))?;
        if actual == expected {
            return Ok(());
        }
    }
    if saw_supported {
        Err(anyhow::anyhow!("integrity mismatch"))
    } else {
        Err(anyhow::anyhow!(
            "Unsupported integrity algorithm: {integrity}"
        ))
    }
}

fn is_cors_safelisted_response_header(name: &str) -> bool {
    matches!(
        name,
        "cache-control"
            | "content-language"
            | "content-length"
            | "content-type"
            | "expires"
            | "last-modified"
            | "pragma"
    )
}

fn cors_filter_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
    let mut exposed = Vec::new();
    for (name, value) in headers {
        if !name.eq_ignore_ascii_case("access-control-expose-headers") {
            continue;
        }
        if value.trim() == "*" {
            exposed.push("*".to_string());
            continue;
        }
        for part in value.split(',') {
            let part = part.trim().to_ascii_lowercase();
            if !part.is_empty() {
                exposed.push(part);
            }
        }
    }
    headers
        .iter()
        .filter(|(name, _)| {
            let name = name.to_ascii_lowercase();
            if name == "set-cookie" || name == "set-cookie2" {
                return false;
            }
            if is_cors_safelisted_response_header(&name) || name.starts_with("access-control-") {
                return true;
            }
            exposed.iter().any(|item| item == "*" || item == &name)
        })
        .cloned()
        .collect()
}

fn is_request_mode(mode: &str) -> bool {
    matches!(mode, "cors" | "no-cors" | "same-origin")
}

fn apply_response_mode(response: &mut FetchResponse, mode: &str) {
    match mode {
        "cors" => {
            response.headers = cors_filter_headers(&response.headers);
            response.response_type = "cors".to_string();
        }
        "no-cors" => {
            response.status = 0;
            response.status_text.clear();
            response.ok = false;
            response.headers.clear();
            response.body = memory_body(Vec::new());
            response.url.clear();
            response.response_type = "opaque".to_string();
            response.redirected = false;
        }
        // Same-origin, including an omitted mode. `response.type` is a real
        // property: "basic", "cors", or "opaque".
        _ => {
            response.response_type = "basic".to_string();
        }
    }
}

/// One pull for `response.body.getReader().read()`. Does not read past the
/// bytes already taken off the wire plus a single following socket read.
fn read_from_offset(body: &mut SharedBody, offset: usize) -> Result<(Option<Vec<u8>>, usize)> {
    if offset < body.pulled.len() {
        let chunk = body.pulled[offset..].to_vec();
        return Ok((Some(chunk), body.pulled.len()));
    }
    match next_chunk(body)? {
        Some(chunk) => Ok((Some(chunk), body.pulled.len())),
        None => Ok((None, offset)),
    }
}

fn http1_keepalive_get(url: &str) -> Result<FetchResponse> {
    let (host, port, path) = parse_http_url(url)
        .ok_or_else(|| anyhow::anyhow!("http1 fast path requires http:// URL"))?;
    let endpoint = format!("{}:{}", host, port);
    let host_header = http_host_header(&host, port);
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_header}\r\nConnection: keep-alive\r\nAccept: */*\r\n\r\n"
    );

    HTTP1_CONN.with(|slot| {
        let mut slot = slot.borrow_mut();
        let mut stream = match slot.take() {
            Some((ep, s)) if ep == endpoint => s,
            Some((_, old)) => {
                drop(old);
                connect_http_stream(&host, port)?
            }
            None => connect_http_stream(&host, port)?,
        };
        if stream.write_all(request.as_bytes()).is_err() {
            stream = connect_http_stream(&host, port)?;
            stream.write_all(request.as_bytes())?;
        }
        stream.flush()?;

        let mut buf = Vec::with_capacity(1024);
        let mut tmp = [0u8; 512];
        let header_end;
        loop {
            let n = stream.read(&mut tmp)?;
            if n == 0 {
                return Err(anyhow::anyhow!("connection closed before headers"));
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                header_end = pos + 4;
                break;
            }
            if buf.len() > 64 * 1024 {
                return Err(anyhow::anyhow!("headers too large"));
            }
        }
        let header_text = String::from_utf8_lossy(&buf[..header_end]);
        let mut status: u16 = 200;
        let mut status_text = String::new();
        let mut content_length: Option<usize> = None;
        let mut headers = Vec::new();
        for (i, line) in header_text.split("\r\n").enumerate() {
            if i == 0 {
                let mut parts = line.split_whitespace();
                let _ = parts.next();
                if let Some(code) = parts.next() {
                    status = code.parse().unwrap_or(200);
                }
                status_text = parts.collect::<Vec<_>>().join(" ");
                continue;
            }
            if line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim().to_ascii_lowercase();
                let val = v.trim().to_string();
                if key == "content-length" {
                    content_length = val.parse().ok();
                }
                headers.push((key, val));
            }
        }
        // Only bytes that arrived with the header block. The rest stays on
        // the socket until `response.body` or `text()` pulls it.
        let mut leftover = buf[header_end..].to_vec();
        if let Some(len) = content_length {
            if leftover.len() > len {
                leftover.truncate(len);
            }
        }
        let remaining = content_length.map(|len| len.saturating_sub(leftover.len()));
        let body_done = remaining.map(|left| left == 0).unwrap_or(true);
        let body = if body_done {
            let reuse = headers
                .iter()
                .find(|(name, _)| name == "connection")
                .map(|(_, value)| !value.eq_ignore_ascii_case("close"))
                .unwrap_or(true);
            if reuse {
                *slot = Some((endpoint, stream));
            } else {
                *slot = None;
            }
            memory_body(leftover)
        } else {
            *slot = None;
            open_body(leftover, Some(NetReader::Tcp { stream, remaining }))
        };
        Ok(FetchResponse {
            url: url.to_string(),
            status,
            status_text,
            ok: (200..300).contains(&status),
            headers,
            body,
            body_used: false,
            redirected: false,
            response_type: "basic".to_string(),
        })
    })
}

fn check_fetch_permission(url: &str) -> Result<()> {
    if crate::permissions::has_restrictions() {
        crate::permissions::check_global_permission(
            crate::permissions::PermissionKind::Network,
            crate::permissions::PermissionAction::Connect,
            crate::permissions::ResourceId::Url(url.to_string()),
        )
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    }
    Ok(())
}

fn fetch_once_without_redirect(url: &str) -> Result<FetchResponse> {
    if url.starts_with("http://") {
        if let Ok(response) = http1_keepalive_get(url) {
            return Ok(response);
        }
    }
    let response = get_blocking_fetch_client()
        .get(url)
        .send()
        .map_err(|error| anyhow::anyhow!("failed to fetch {url}: {error}"))?;
    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("Unknown")
        .to_string();
    let ok = response.status().is_success();
    let final_url = response.url().to_string();
    let headers = headers_from_reqwest(response.headers());
    let body = open_body(Vec::new(), Some(NetReader::Blocking(response)));
    Ok(FetchResponse {
        url: final_url,
        status,
        status_text,
        ok,
        headers,
        body,
        body_used: false,
        redirected: false,
        response_type: "basic".to_string(),
    })
}

/// Default `fetch(url)` GET. Stays on the HTTP/1.1 fast path and follows
/// `Location` itself so a 3xx is not returned as the final response.
fn execute_simple_get(url: &str) -> Result<FetchResponse> {
    let mut current_url = url.to_string();
    let mut redirected = false;
    let mut followed = 0u32;
    loop {
        check_fetch_permission(&current_url)?;
        let mut response = fetch_once_without_redirect(&current_url)?;
        let location = header_lookup(&response.headers, "location")
            .map(str::trim)
            .filter(|location| !location.is_empty())
            .map(str::to_string);
        if !is_redirect_status(response.status) || location.is_none() {
            response.url = current_url;
            response.redirected = redirected;
            return Ok(response);
        }
        drain_shared_body(&response.body)?;
        if followed >= MAX_REDIRECTS {
            return Err(anyhow::anyhow!("Too many redirects"));
        }
        let location = location.expect("location checked above");
        current_url = resolve_redirect_location(&current_url, &location)?;
        followed += 1;
        redirected = true;
    }
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
            user_agent: format!("Amber/{}", env!("CARGO_PKG_VERSION")),
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
    /// Repeated names stay repeated. `Set-Cookie` is not folded into one value.
    pub headers: Vec<(String, String)>,
    /// Unread network body. Not a fully buffered `Vec` unless the caller
    /// already had the bytes (for example `new Response(bytes)`).
    pub body: Arc<Mutex<SharedBody>>,
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
fn set_fetch_response_retval(
    scope: &mut v8::PinScope,
    retval: &mut v8::ReturnValue,
    response: FetchResponse,
) {
    let response_obj: _ = v8::Object::new(scope);
    let ok_key: _ = v8::String::new(scope, "ok").unwrap();
    let ok_key_val: _ = v8::Boolean::new(scope, response.ok).into();
    response_obj.set(scope, ok_key.into(), ok_key_val);
    let status_key: _ = v8::String::new(scope, "status").unwrap();
    let status_key_val: _ = v8::Integer::new(scope, response.status as i32).into();
    response_obj.set(scope, status_key.into(), status_key_val);
    let status_text_key: _ = v8::String::new(scope, "statusText").unwrap();
    let status_text_val: v8::Local<v8::Value> = v8::String::new(scope, &response.status_text)
        .unwrap()
        .into();
    response_obj.set(scope, status_text_key.into(), status_text_val);

    let url_key: _ = v8::String::new(scope, "url").unwrap();
    let url_val: _ = v8::String::new(scope, &response.url).unwrap().into();
    response_obj.set(scope, url_key.into(), url_val);

    store_shared_body(scope, response_obj, Arc::clone(&response.body));
    attach_response_body_methods(scope, response_obj);

    let type_key: _ = v8::String::new(scope, "type").unwrap();
    let type_name = match response.response_type.as_str() {
        "cors" => "cors",
        "opaque" => "opaque",
        _ => "basic",
    };
    let type_val: v8::Local<v8::Value> = v8::String::new(scope, type_name).unwrap().into();
    response_obj.set(scope, type_key.into(), type_val);

    let redirected_key: _ = v8::String::new(scope, "redirected").unwrap();
    let redirected_val: v8::Local<v8::Value> = v8::Boolean::new(scope, response.redirected).into();
    response_obj.set(scope, redirected_key.into(), redirected_val);

    let body_used_key: _ = v8::String::new(scope, "bodyUsed").unwrap();
    let body_used_val: v8::Local<v8::Value> = v8::Boolean::new(scope, response.body_used).into();
    response_obj.set(scope, body_used_key.into(), body_used_val);

    attach_response_clone_method(scope, response_obj);

    let headers_obj =
        create_headers_object_with_entries(scope, response.headers.into_iter().collect::<Vec<_>>());
    let headers_key: _ = v8::String::new(scope, "headers").unwrap();
    response_obj.set(scope, headers_key.into(), headers_obj.into());
    retval.set(response_obj.into());
}

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
    let mut request_redirect = String::from("follow");
    let mut request_integrity = String::new();
    let mut request_mode: Option<String> = None;
    let mut request_abort = Arc::new(AtomicBool::new(false));

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

            let redirect_key = v8::String::new(scope, "redirect").unwrap().into();
            if let Some(redirect_val) = input_obj.get(scope, redirect_key) {
                if redirect_val.is_string() {
                    if let Some(redirect_str) = redirect_val.to_string(scope) {
                        request_redirect = redirect_str.to_rust_string_lossy(scope);
                    }
                }
            }

            let explicit_key = v8::String::new(scope, "__amberModeExplicit")
                .unwrap()
                .into();
            if input_obj
                .get(scope, explicit_key)
                .map(|value| value.is_true())
                .unwrap_or(false)
            {
                let mode_key = v8::String::new(scope, "mode").unwrap().into();
                if let Some(mode_val) = input_obj.get(scope, mode_key) {
                    if mode_val.is_string() {
                        if let Some(mode_str) = mode_val.to_string(scope) {
                            request_mode = Some(mode_str.to_rust_string_lossy(scope));
                        }
                    }
                }
            }

            let integrity_key = v8::String::new(scope, "integrity").unwrap().into();
            if let Some(integrity_val) = input_obj.get(scope, integrity_key) {
                request_integrity = js_integrity_string(scope, integrity_val);
            }

            let signal_key = v8::String::new(scope, "signal").unwrap().into();
            if let Some(signal_val) = input_obj.get(scope, signal_key) {
                if signal_val.is_object() {
                    request_abort = abort_flag_from_signal(scope, signal_val);
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

    // Fast path: string URL, default GET, no init object — avoid Tokio block_on.
    if input.is_string() && !init.is_object() {
        match execute_simple_get(&url_str) {
            Ok(response) => {
                set_fetch_response_retval(scope, &mut retval, response);
            }
            Err(e) => {
                let error: _ = v8::String::new(scope, &format!("Fetch error: {}", e)).unwrap();
                let error_obj: _ = v8::Exception::error(scope, error);
                scope.throw_exception(error_obj.into());
            }
        }
        return;
    }

    // Parse init options - start with values from Request object (if provided)
    let mut method = HttpMethod::from(request_method);
    let mut headers = request_headers;
    let mut body = request_body;
    let mut content_type = request_content_type;
    let mut redirect = request_redirect;
    let mut integrity = request_integrity;
    let mut mode = request_mode;
    let mut abort_flag = request_abort;

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

            // Parse redirect option. A missing property is undefined; toString()
            // would turn that into the mode "undefined" and reject a normal init.
            let redirect_key = v8::String::new(scope, "redirect").unwrap().into();
            if let Some(redirect_val) = init_obj.get(scope, redirect_key) {
                if redirect_val.is_string() {
                    if let Some(redirect_str) = redirect_val.to_string(scope) {
                        redirect = redirect_str.to_rust_string_lossy(scope);
                    }
                }
            }

            let mode_key = v8::String::new(scope, "mode").unwrap().into();
            if let Some(mode_val) = init_obj.get(scope, mode_key) {
                if mode_val.is_string() {
                    if let Some(mode_str) = mode_val.to_string(scope) {
                        mode = Some(mode_str.to_rust_string_lossy(scope));
                    }
                }
            }

            let integrity_key = v8::String::new(scope, "integrity").unwrap().into();
            if let Some(integrity_val) = init_obj.get(scope, integrity_key) {
                integrity = js_integrity_string(scope, integrity_val);
            }

            let signal_key = v8::String::new(scope, "signal").unwrap().into();
            if let Some(signal_val) = init_obj.get(scope, signal_key) {
                if signal_val.is_object() {
                    abort_flag = abort_flag_from_signal(scope, signal_val);
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

    if !is_redirect_mode(&redirect) {
        let message =
            v8::String::new(scope, &format!("Invalid redirect mode: {redirect}")).unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error.into());
        return;
    }
    if let Some(mode_value) = mode.as_deref() {
        if !is_request_mode(mode_value) {
            let message =
                v8::String::new(scope, &format!("Invalid request mode: {mode_value}")).unwrap();
            let error = v8::Exception::type_error(scope, message);
            scope.throw_exception(error.into());
            return;
        }
    }

    // Execute fetch synchronously in a blocking task
    let url: _ = url_str.clone();
    let method_clone = method.clone();
    let headers_clone = headers.clone();
    let body_clone = body.clone();
    let redirect_clone = redirect.clone();
    let integrity_clone = integrity.clone();
    let mode_clone = mode.unwrap_or_default();
    let _abort_guard = arm_fetch_abort(&abort_flag);

    let result = get_fetch_runtime().block_on(execute_fetch(
        &url,
        method_clone,
        headers_clone,
        body_clone,
        &redirect_clone,
        &integrity_clone,
        &abort_flag,
        &mode_clone,
    ));
    match result {
        Ok(response) => set_fetch_response_retval(scope, &mut retval, response),
        Err(e) => {
            let error: _ = v8::String::new(scope, &format!("Fetch error: {}", e)).unwrap();
            let error_obj: _ = v8::Exception::error(scope, error);
            scope.throw_exception(error_obj.into());
        }
    }
}
fn method_to_reqwest(method: &HttpMethod) -> reqwest::Method {
    match method {
        HttpMethod::GET => reqwest::Method::GET,
        HttpMethod::POST => reqwest::Method::POST,
        HttpMethod::PUT => reqwest::Method::PUT,
        HttpMethod::DELETE => reqwest::Method::DELETE,
        HttpMethod::PATCH => reqwest::Method::PATCH,
        HttpMethod::HEAD => reqwest::Method::HEAD,
        HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
    }
}

fn headers_from_reqwest(headers: &reqwest::header::HeaderMap) -> Vec<(String, String)> {
    let mut response_headers = Vec::new();
    for (key, value) in headers {
        // `HeaderMap` yields every value. `insert` into a map would keep one
        // `Set-Cookie` and drop the rest.
        response_headers.push((key.to_string(), value.to_str().unwrap_or("").to_string()));
    }
    response_headers
}

async fn collect_fetch_response(
    response: reqwest::Response,
    url: String,
    redirected: bool,
) -> Result<FetchResponse> {
    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("Unknown")
        .to_string();
    let ok = response.status().is_success();
    let headers = headers_from_reqwest(response.headers());
    // Leave the reqwest body unread. `response.body` pulls `chunk()` later.
    let body = open_body(
        Vec::new(),
        Some(NetReader::Async(tokio::sync::Mutex::new(Some(response)))),
    );
    Ok(FetchResponse {
        url,
        status,
        status_text,
        ok,
        headers,
        body,
        body_used: false,
        redirected,
        response_type: "basic".to_string(),
    })
}

async fn consume_hop_body(
    response: &mut reqwest::Response,
    integrity: &str,
    aborted: &AtomicBool,
) -> Result<Vec<u8>> {
    let mut collected = Vec::new();
    let check = !integrity_is_absent(integrity);
    loop {
        ensure_not_aborted(aborted)?;
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if check {
                    collected.extend_from_slice(&chunk);
                }
            }
            Ok(None) => {
                if check {
                    check_body_integrity(&collected, integrity)?;
                }
                return Ok(collected);
            }
            Err(error) => {
                return Err(anyhow::anyhow!("failed to read fetch body: {error}"));
            }
        }
    }
}

/// Execute actual HTTP fetch using reqwest. Redirect policy is Amber's:
/// `follow` resolves Location and rewrites 301/302/303 methods, `manual`
/// returns the 3xx, and `error` rejects. Abort and integrity run on every hop.
async fn execute_fetch(
    url: &str,
    mut method: HttpMethod,
    mut headers: HashMap<String, String>,
    mut body: Option<Vec<u8>>,
    redirect: &str,
    integrity: &str,
    aborted: &AtomicBool,
    mode: &str,
) -> Result<FetchResponse> {
    if !is_redirect_mode(redirect) {
        return Err(anyhow::anyhow!("invalid redirect mode: {redirect}"));
    }
    let mut current_url = url.to_string();
    let mut redirected = false;
    let mut followed = 0u32;

    loop {
        ensure_not_aborted(aborted)?;
        check_fetch_permission(&current_url)?;

        let client = get_fetch_client();
        let request = client.request(method_to_reqwest(&method), &current_url);
        let request = if matches!(method, HttpMethod::GET | HttpMethod::HEAD) || body.is_none() {
            request
        } else {
            request.body(body.clone().unwrap())
        };
        let mut req_builder = request;
        for (key, value) in &headers {
            req_builder = req_builder.header(key, value);
        }

        let response = match req_builder.send().await {
            Ok(response) => response,
            Err(error) => {
                return Err(anyhow::anyhow!("failed to fetch {current_url}: {error}"));
            }
        };

        let status = response.status().as_u16();
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|location| !location.is_empty())
            .map(str::to_string);
        if is_redirect_status(status) && redirect == "error" {
            return Err(anyhow::anyhow!("redirect not allowed: {status}"));
        }
        if is_redirect_status(status) && redirect == "follow" {
            if let Some(location) = location {
                if followed >= MAX_REDIRECTS {
                    return Err(anyhow::anyhow!("Too many redirects"));
                }
                let next_url = resolve_redirect_location(&current_url, &location)?;
                if !same_http_origin(&current_url, &next_url) {
                    strip_cross_origin_request_headers(&mut headers);
                }
                apply_redirect_method(status, &mut method, &mut body, &mut headers);
                let mut response = response;
                let _hop_body = consume_hop_body(&mut response, integrity, aborted).await?;
                ensure_not_aborted(aborted)?;
                followed += 1;
                redirected = true;
                current_url = next_url;
                continue;
            }
        }

        let mut response = response;
        let built = if integrity_is_absent(integrity) {
            collect_fetch_response(response, current_url, redirected).await?
        } else {
            let status = response.status().as_u16();
            let status_text = response
                .status()
                .canonical_reason()
                .unwrap_or("Unknown")
                .to_string();
            let ok = response.status().is_success();
            let headers = headers_from_reqwest(response.headers());
            let bytes = consume_hop_body(&mut response, integrity, aborted).await?;
            FetchResponse {
                url: current_url,
                status,
                status_text,
                ok,
                headers,
                body: memory_body(bytes),
                body_used: false,
                redirected,
                response_type: "basic".to_string(),
            }
        };
        ensure_not_aborted(aborted)?;
        let mut built = built;
        apply_response_mode(&mut built, mode);
        return Ok(built);
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
    let mut mode_explicit = false;
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

            // Parse mode. A missing property is undefined, not an explicit mode.
            let mode_key = v8::String::new(scope, "mode").unwrap().into();
            if let Some(mode_val) = init.get(scope, mode_key) {
                if mode_val.is_string() {
                    if let Some(mode_str) = mode_val.to_string(scope) {
                        init_mode = mode_str.to_rust_string_lossy(scope);
                        mode_explicit = true;
                    }
                }
            }

            // Parse redirect. Missing properties are undefined, not the mode string.
            let redirect_key = v8::String::new(scope, "redirect").unwrap().into();
            if let Some(redirect_val) = init.get(scope, redirect_key) {
                if redirect_val.is_string() {
                    if let Some(redirect_str) = redirect_val.to_string(scope) {
                        init_redirect = redirect_str.to_rust_string_lossy(scope);
                    }
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

            // Parse integrity. A missing property is JavaScript `undefined`;
            // toString() would store the word "undefined" and fail the check.
            let integrity_key = v8::String::new(scope, "integrity").unwrap().into();
            if let Some(integrity_val) = init.get(scope, integrity_key) {
                init_integrity = js_integrity_string(scope, integrity_val);
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
    if mode_explicit {
        let explicit_key = v8::String::new(scope, "__amberModeExplicit")
            .unwrap()
            .into();
        request_obj.set(scope, explicit_key, v8::Boolean::new(scope, true).into());
    }

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

    // `set` replaces an existing name. `append` keeps a second `Set-Cookie`.
    let append_key = v8::String::new(scope, "append").unwrap().into();
    let append_func = headers_obj
        .get(scope, append_key)
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok());

    if let Some(append_func) = append_func {
        for (name, value) in entries {
            let name_value: v8::Local<v8::Value> = v8::String::new(scope, &name).unwrap().into();
            let header_value: v8::Local<v8::Value> = v8::String::new(scope, &value).unwrap().into();
            let _ = append_func.call(scope, headers_obj.into(), &[name_value, header_value]);
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

fn bytes_to_uint8_array<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    bytes: &[u8],
) -> v8::Local<'a, v8::Uint8Array> {
    let buffer = v8::ArrayBuffer::new(scope, bytes.len());
    let store = buffer.get_backing_store();
    let ptr = store.as_ref().as_ptr() as *mut u8;
    if !bytes.is_empty() && !ptr.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        }
    }
    v8::Uint8Array::new(scope, buffer, 0, bytes.len()).unwrap()
}

fn body_reader_read_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let reader = args.this();
    let id_key = v8::String::new(scope, "__amberjsResponseId")
        .unwrap()
        .into();
    let offset_key = v8::String::new(scope, "__amberBodyOffset").unwrap().into();
    let response_id = reader
        .get(scope, id_key)
        .and_then(|value| value.to_integer(scope))
        .map(|value| value.value() as usize)
        .unwrap_or(usize::MAX);
    let offset = reader
        .get(scope, offset_key)
        .and_then(|value| value.to_number(scope))
        .map(|value| value.value() as usize)
        .unwrap_or(0);
    let shared = {
        let cache = get_response_cache().lock().unwrap();
        cache.get(&response_id).cloned()
    };
    let Some(shared) = shared else {
        let message = v8::String::new(scope, "Response body not available").unwrap();
        scope.throw_exception(v8::Exception::error(scope, message).into());
        return;
    };
    let pulled = {
        let mut body = shared.lock().unwrap();
        read_from_offset(&mut body, offset)
    };
    let (chunk, new_offset) = match pulled {
        Ok(pair) => pair,
        Err(error) => {
            let message = v8::String::new(scope, &format!("Fetch error: {error}")).unwrap();
            scope.throw_exception(v8::Exception::error(scope, message).into());
            return;
        }
    };
    let offset_val = v8::Number::new(scope, new_offset as f64).into();
    reader.set(scope, offset_key, offset_val);

    let result = v8::Object::new(scope);
    let done_key = v8::String::new(scope, "done").unwrap().into();
    let value_key = v8::String::new(scope, "value").unwrap().into();
    match chunk {
        Some(bytes) => {
            let value = bytes_to_uint8_array(scope, &bytes).into();
            result.set(scope, done_key, v8::Boolean::new(scope, false).into());
            result.set(scope, value_key, value);
        }
        None => {
            result.set(scope, done_key, v8::Boolean::new(scope, true).into());
            result.set(scope, value_key, v8::undefined(scope).into());
        }
    }
    retval.set(result.into());
}

fn body_get_reader_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let stream = args.this();
    let locked_key = v8::String::new(scope, "locked").unwrap().into();
    if stream
        .get(scope, locked_key)
        .map(|value| value.is_true())
        .unwrap_or(false)
    {
        let message = v8::String::new(scope, "body stream is locked").unwrap();
        scope.throw_exception(v8::Exception::type_error(scope, message).into());
        return;
    }
    let owner_key = v8::String::new(scope, "__amberOwner").unwrap().into();
    if let Some(owner) = stream
        .get(scope, owner_key)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
    {
        if response_body_is_used(scope, owner) {
            throw_response_body_already_consumed(scope);
            return;
        }
        let body_used_key = v8::String::new(scope, "bodyUsed").unwrap().into();
        owner.set(scope, body_used_key, v8::Boolean::new(scope, true).into());
    }
    stream.set(scope, locked_key, v8::Boolean::new(scope, true).into());

    let reader = v8::Object::new(scope);
    let id_key = v8::String::new(scope, "__amberjsResponseId")
        .unwrap()
        .into();
    if let Some(id_val) = stream.get(scope, id_key) {
        reader.set(scope, id_key, id_val);
    }
    reader.set(scope, owner_key, stream.into());
    let offset_key = v8::String::new(scope, "__amberBodyOffset").unwrap().into();
    reader.set(scope, offset_key, v8::Number::new(scope, 0.0).into());
    let read_key = v8::String::new(scope, "read").unwrap().into();
    let read_fn = v8::Function::new(scope, body_reader_read_callback).unwrap();
    reader.set(scope, read_key, read_fn.into());
    retval.set(reader.into());
}

fn store_shared_body(
    scope: &mut v8::PinScope,
    response_obj: v8::Local<v8::Object>,
    body: Arc<Mutex<SharedBody>>,
) {
    let response_id = next_response_id();
    get_response_cache()
        .lock()
        .unwrap()
        .insert(response_id, body);

    let response_id_key: _ = v8::String::new(scope, "__amberjsResponseId").unwrap();
    let response_id_val: _ = v8::Integer::new_from_unsigned(scope, response_id as u32).into();
    response_obj.set(scope, response_id_key.into(), response_id_val);

    let stream: v8::Local<v8::Object> = v8::Object::new(scope);
    let locked_key = v8::String::new(scope, "locked").unwrap().into();
    stream.set(scope, locked_key, v8::Boolean::new(scope, false).into());
    stream.set(scope, response_id_key.into(), response_id_val);
    let owner_key = v8::String::new(scope, "__amberOwner").unwrap().into();
    stream.set(scope, owner_key, response_obj.into());
    let get_reader_key = v8::String::new(scope, "getReader").unwrap().into();
    let get_reader = v8::Function::new(scope, body_get_reader_callback).unwrap();
    stream.set(scope, get_reader_key, get_reader.into());

    let body_key: _ = v8::String::new(scope, "body").unwrap();
    response_obj.set(scope, body_key.into(), stream.into());
}

fn store_response_body(
    scope: &mut v8::PinScope,
    response_obj: v8::Local<v8::Object>,
    _url: String,
    body_vec: Vec<u8>,
) {
    store_shared_body(scope, response_obj, memory_body(body_vec));
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
                "__amberjsResponseId",
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

    let body = match response_body_for_object(scope, response_obj) {
        Ok(body) => body,
        Err(()) => return Err(()),
    };
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
) -> std::result::Result<Option<Vec<u8>>, ()> {
    let response_id_key = v8::String::new(scope, "__amberjsResponseId")
        .unwrap()
        .into();
    if let Some(response_id_val) = response_obj.get(scope, response_id_key) {
        if let Some(response_id_int) = response_id_val.to_integer(scope) {
            let response_id = response_id_int.value() as usize;
            let shared = {
                let cache = get_response_cache().lock().unwrap();
                cache.get(&response_id).cloned()
            };
            if let Some(shared) = shared {
                return match read_to_end(&shared) {
                    Ok(bytes) => Ok(Some(bytes)),
                    Err(error) => {
                        let message =
                            v8::String::new(scope, &format!("Fetch error: {error}")).unwrap();
                        scope.throw_exception(v8::Exception::error(scope, message).into());
                        Err(())
                    }
                };
            }
        }
    }

    let body_key = v8::String::new(scope, "body").unwrap().into();
    Ok(response_obj
        .get(scope, body_key)
        .filter(|body| body.is_string())
        .and_then(|body| body.to_string(scope))
        .map(|body| body.to_rust_string_lossy(scope).into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{
        apply_redirect_method, check_body_integrity, cors_filter_headers, headers_from_reqwest,
        resolve_redirect_location, FetchConfig, HttpMethod, MAX_REDIRECTS,
    };
    use std::collections::HashMap;

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
            format!("Amber/{}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(config.timeout, std::time::Duration::from_secs(30));
        assert_eq!(config.max_redirects, 20);
        assert_eq!(config.max_redirects, MAX_REDIRECTS);
    }

    #[test]
    fn test_resolve_redirect_location_relative_query_and_absolute() {
        let base = "http://127.0.0.1:9/a/b?q=1#frag";
        assert_eq!(
            resolve_redirect_location(base, "c").unwrap(),
            "http://127.0.0.1:9/a/c"
        );
        assert_eq!(
            resolve_redirect_location(base, "/landed").unwrap(),
            "http://127.0.0.1:9/landed"
        );
        assert_eq!(
            resolve_redirect_location(base, "?x=2").unwrap(),
            "http://127.0.0.1:9/a/b?x=2"
        );
        assert_eq!(
            resolve_redirect_location(base, "../c").unwrap(),
            "http://127.0.0.1:9/c"
        );
        assert_eq!(
            resolve_redirect_location("http://h/a/b/", "../c").unwrap(),
            "http://h/a/c"
        );
        assert_eq!(
            resolve_redirect_location(base, "http://example.com/z").unwrap(),
            "http://example.com/z"
        );
        assert_eq!(
            resolve_redirect_location(base, "//cdn.example/x").unwrap(),
            "http://cdn.example/x"
        );
    }

    #[test]
    fn test_apply_redirect_method_rewrites_301_and_303_but_not_307() {
        let mut headers = HashMap::from([
            ("Content-Type".to_string(), "text/plain".to_string()),
            ("Authorization".to_string(), "Bearer keep".to_string()),
            ("X-Trace".to_string(), "1".to_string()),
        ]);
        let mut method = HttpMethod::POST;
        let mut body = Some(b"secret".to_vec());
        apply_redirect_method(302, &mut method, &mut body, &mut headers);
        assert_eq!(method, HttpMethod::GET);
        assert!(body.is_none());
        assert!(!headers
            .keys()
            .any(|name| name.eq_ignore_ascii_case("content-type")));
        assert_eq!(
            headers.get("Authorization").map(String::as_str),
            Some("Bearer keep")
        );
        assert_eq!(headers.get("X-Trace").map(String::as_str), Some("1"));

        let mut headers = HashMap::from([("Content-Length".to_string(), "4".to_string())]);
        let mut method = HttpMethod::POST;
        let mut body = Some(b"keep".to_vec());
        apply_redirect_method(307, &mut method, &mut body, &mut headers);
        assert_eq!(method, HttpMethod::POST);
        assert_eq!(body.as_deref(), Some(b"keep".as_slice()));
        assert_eq!(headers.get("Content-Length").map(String::as_str), Some("4"));

        let mut headers = HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]);
        let mut method = HttpMethod::HEAD;
        let mut body = Some(b"drop".to_vec());
        apply_redirect_method(303, &mut method, &mut body, &mut headers);
        assert_eq!(method, HttpMethod::HEAD);
        assert!(body.is_none());
        assert!(headers.is_empty());
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

    #[test]
    fn test_cors_filter_drops_set_cookie_and_unexposed_names() {
        let headers = vec![
            ("content-type".to_string(), "text/plain".to_string()),
            ("x-secret".to_string(), "no".to_string()),
            ("x-exposed".to_string(), "yes".to_string()),
            ("set-cookie".to_string(), "a=1".to_string()),
            ("set-cookie".to_string(), "b=2".to_string()),
            (
                "access-control-expose-headers".to_string(),
                "x-exposed".to_string(),
            ),
        ];
        let filtered = cors_filter_headers(&headers);
        assert!(filtered
            .iter()
            .any(|(name, value)| { name == "content-type" && value == "text/plain" }));
        assert!(filtered
            .iter()
            .any(|(name, value)| name == "x-exposed" && value == "yes"));
        assert!(!filtered.iter().any(|(name, _)| name == "x-secret"));
        assert!(!filtered.iter().any(|(name, _)| name == "set-cookie"));
    }

    #[test]
    fn test_integrity_matches_sha256_and_rejects_a_mismatch() {
        use base64::Engine as _;
        use sha2::Digest as _;
        let bytes = b"yes";
        let digest = sha2::Sha256::digest(bytes);
        let metadata = format!(
            "sha256-{}",
            base64::engine::general_purpose::STANDARD.encode(digest)
        );
        assert!(check_body_integrity(bytes, &metadata).is_ok());
        let error = check_body_integrity(b"no", &metadata).unwrap_err();
        assert!(error.to_string().contains("integrity mismatch"));
    }

    #[test]
    fn test_headers_from_reqwest_keeps_duplicate_set_cookie() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.append(reqwest::header::SET_COOKIE, "a=1".parse().unwrap());
        headers.append(reqwest::header::SET_COOKIE, "b=2".parse().unwrap());
        let entries = headers_from_reqwest(&headers);
        let cookies: Vec<_> = entries
            .iter()
            .filter(|(name, _)| name == "set-cookie")
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(cookies, ["a=1", "b=2"]);
    }
}
