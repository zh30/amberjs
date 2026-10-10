//! Shared HTTP/1.1 framing for `amber serve` (cleartext) and `amber serve --https`.
//!
//! - Cleartext contract: [`docs/SERVE_HTTP_CONTRACT.md`](../docs/SERVE_HTTP_CONTRACT.md) (G41).
//! - TLS contract: [`docs/SERVE_HTTPS_CONTRACT.md`](../docs/SERVE_HTTPS_CONTRACT.md) (G7).
//!
//! PEM / rustls helpers live here so the Node `http` compatibility layer is not
//! part of either CLI serve command's limits.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

/// Stderr prefix for contracted `amber serve` / `amber serve --https` failures before listen.
pub const SERVE_ERROR_PREFIX: &str = "error: amber serve:";

/// Maximum size of the request header block, including the final `\r\n\r\n`.
pub const MAX_HEADER_BYTES: usize = 16 * 1024;

/// Maximum `Content-Length` accepted for one request body.
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Maximum number of header fields, including `Host`.
pub const MAX_HEADER_COUNT: usize = 64;

/// Read and write timeout applied to each accepted TCP connection.
pub const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum PEM file size for `--cert` and `--key`.
pub const MAX_PEM_BYTES: usize = 1024 * 1024;

/// Limits used by the CLI. Tests may clone this and shorten the timeout.
#[derive(Debug, Clone)]
pub struct ServeLimits {
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
    pub max_header_count: usize,
    pub read_timeout: Duration,
}

impl Default for ServeLimits {
    fn default() -> Self {
        Self {
            max_header_bytes: MAX_HEADER_BYTES,
            max_body_bytes: MAX_BODY_BYTES,
            max_header_count: MAX_HEADER_COUNT,
            read_timeout: READ_TIMEOUT,
        }
    }
}

/// Failure before the process binds a port.
#[derive(Debug)]
pub enum ServeHttpsError {
    MissingCert,
    MissingKey,
    CertNotFound(String),
    KeyNotFound(String),
    CertNotFile(String),
    KeyNotFile(String),
    InvalidCert(String),
    InvalidKey(String),
    Tls(String),
    Script(String),
    Bind(String),
}

impl fmt::Display for ServeHttpsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{SERVE_ERROR_PREFIX} ")?;
        match self {
            Self::MissingCert => {
                write!(f, "--https requires --cert PATH (PEM certificate)")
            }
            Self::MissingKey => write!(f, "--https requires --key PATH (PEM private key)"),
            Self::CertNotFound(path) => write!(f, "TLS certificate not found: {path}"),
            Self::KeyNotFound(path) => write!(f, "TLS private key not found: {path}"),
            Self::CertNotFile(path) => write!(f, "TLS certificate is not a file: {path}"),
            Self::KeyNotFile(path) => write!(f, "TLS private key is not a file: {path}"),
            Self::InvalidCert(msg) => write!(f, "invalid TLS certificate: {msg}"),
            Self::InvalidKey(msg) => write!(f, "invalid TLS private key: {msg}"),
            Self::Tls(msg) => write!(f, "invalid TLS material: {msg}"),
            Self::Script(msg) => write!(f, "failed to load script: {msg}"),
            Self::Bind(msg) => write!(f, "failed to bind {msg}"),
        }
    }
}

impl std::error::Error for ServeHttpsError {}

#[derive(Debug, Clone)]
pub struct Http11Request {
    pub method: String,
    pub target: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Http11Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct ProtocolFailure {
    status: u16,
    body: &'static str,
    head: bool,
}

enum Incoming {
    Closed,
    Request(Http11Request),
}

enum ReadHttpError {
    Io(io::Error),
    Protocol(ProtocolFailure),
}

/// Wrap a script so its fetch export is visible to [`FETCH_BRIDGE`].
///
/// A script-local `function fetch` is an export. The runtime's global `fetch`
/// client is not treated as the server handler.
pub fn wrap_user_script(code: &str) -> String {
    format!(
        r#"
                    (function() {{
                        const module = {{ exports: {{}} }};
                        const exports = module.exports;
                        {code}
                        const exported = module.exports;
                        const hasExport = exported && (typeof exported === 'function' || exported.default || exported.fetch);
                        const localFetch = (typeof fetch === 'function' && fetch !== globalThis.fetch) ? fetch : undefined;
                        globalThis.__amberjs_app__ = hasExport ? exported : (localFetch ? {{ fetch: localFetch }} : {{}});
                    }})();
                    "#
    )
}

/// Fetch-handler bridge installed before the user script. Text in, text out.
pub const FETCH_BRIDGE: &str = r#"
globalThis.__amberjs_app__ = undefined;
globalThis.__amberjs_handle_http__ = async function(method, url, headersJson, bodyStr) {
    try {
        const headers = JSON.parse(headersJson);
        const reqInit = { method, headers };
        if (method !== "GET" && method !== "HEAD" && bodyStr && bodyStr.length > 0) {
            reqInit.body = bodyStr;
        }
        const req = new Request(url, reqInit);
        let handler = globalThis.__amberjs_app__;
        if (handler && typeof handler.default === 'object' && typeof handler.default.fetch === 'function') {
            handler = handler.default.fetch.bind(handler.default);
        } else if (handler && typeof handler.default === 'function') {
            handler = handler.default;
        } else if (handler && typeof handler.fetch === 'function') {
            handler = handler.fetch;
        } else if (typeof globalThis.fetchHandler === 'function') {
            handler = globalThis.fetchHandler;
        }
        if (typeof handler !== 'function') {
            return JSON.stringify({ status: 404, headers: { "content-type": "text/plain" }, body: "Not Found: No fetch handler exported" });
        }
        const res = await handler(req);
        const status = (res && res.status) ? res.status : 200;
        const resHeaders = {};
        if (res && res.headers && typeof res.headers.forEach === 'function') {
            res.headers.forEach((v, k) => { resHeaders[k] = v; });
        }
        let bodyText = "";
        if (res) {
            if (typeof res._bodyText === 'string') {
                bodyText = res._bodyText;
            } else if (typeof res.text === 'function') {
                try { bodyText = await res.text(); } catch (_) { bodyText = res.body ? String(res.body) : ""; }
            } else {
                bodyText = res.body ? String(res.body) : "";
            }
        }
        return JSON.stringify({ status, headers: resHeaders, body: bodyText });
    } catch (e) {
        return JSON.stringify({ status: 500, headers: { "content-type": "text/plain" }, body: "Internal Server Error: " + (e ? e.message : e) });
    }
};
"#;

/// Load `--cert` / `--key` PEM into a rustls config that advertises only `http/1.1`.
pub fn load_server_config(
    cert_path: &Path,
    key_path: &Path,
) -> Result<Arc<rustls::ServerConfig>, ServeHttpsError> {
    require_regular_file(cert_path, true)?;
    require_regular_file(key_path, false)?;
    let cert_pem = read_pem(cert_path, true)?;
    let key_pem = read_pem(key_path, false)?;
    let certs = parse_certs(&cert_pem)?;
    let key = parse_private_key(&key_pem)?;
    ensure_key_matches_leaf(&cert_pem, &key_pem)?;
    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| ServeHttpsError::Tls(err.to_string()))?;
    tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(tls_config))
}

fn require_regular_file(path: &Path, cert: bool) -> Result<(), ServeHttpsError> {
    if !path.exists() {
        let shown = path.display().to_string();
        return Err(if cert {
            ServeHttpsError::CertNotFound(shown)
        } else {
            ServeHttpsError::KeyNotFound(shown)
        });
    }
    if !path.is_file() {
        let shown = path.display().to_string();
        return Err(if cert {
            ServeHttpsError::CertNotFile(shown)
        } else {
            ServeHttpsError::KeyNotFile(shown)
        });
    }
    Ok(())
}

fn read_pem(path: &Path, cert: bool) -> Result<Vec<u8>, ServeHttpsError> {
    let bytes = fs::read(path).map_err(|err| {
        let msg = format!("failed to read {}: {err}", path.display());
        if cert {
            ServeHttpsError::InvalidCert(msg)
        } else {
            ServeHttpsError::InvalidKey(msg)
        }
    })?;
    if bytes.len() > MAX_PEM_BYTES {
        let msg = format!("PEM file exceeds {MAX_PEM_BYTES} bytes");
        return Err(if cert {
            ServeHttpsError::InvalidCert(msg)
        } else {
            ServeHttpsError::InvalidKey(msg)
        });
    }
    Ok(bytes)
}

fn parse_certs(pem: &[u8]) -> Result<Vec<rustls::Certificate>, ServeHttpsError> {
    let mut reader = std::io::BufReader::new(std::io::Cursor::new(pem));
    let mut certs = Vec::new();
    for cert in rustls_pemfile::certs(&mut reader) {
        let der = cert.map_err(|err| {
            ServeHttpsError::InvalidCert(format!("failed to parse certificate: {err}"))
        })?;
        certs.push(rustls::Certificate(der.as_ref().to_vec()));
    }
    if certs.is_empty() {
        return Err(ServeHttpsError::InvalidCert(
            "no certificate found in PEM".to_string(),
        ));
    }
    Ok(certs)
}

fn parse_private_key(pem: &[u8]) -> Result<rustls::PrivateKey, ServeHttpsError> {
    if pem_is_encrypted(pem) {
        return Err(ServeHttpsError::InvalidKey(
            "encrypted private keys are not supported".to_string(),
        ));
    }
    if let Some(key) = first_key_der(pem, KeyFormat::Pkcs8)? {
        return Ok(key);
    }
    if let Some(key) = first_key_der(pem, KeyFormat::Pkcs1)? {
        return Ok(key);
    }
    if let Some(key) = first_key_der(pem, KeyFormat::Sec1)? {
        return Ok(key);
    }
    Err(ServeHttpsError::InvalidKey(
        "no private key found in PEM".to_string(),
    ))
}

fn ensure_key_matches_leaf(cert_pem: &[u8], key_pem: &[u8]) -> Result<(), ServeHttpsError> {
    let leaf = openssl::x509::X509::from_pem(cert_pem).map_err(|err| {
        ServeHttpsError::InvalidCert(format!("failed to parse certificate: {err}"))
    })?;
    let private = openssl::pkey::PKey::private_key_from_pem(key_pem).map_err(|err| {
        ServeHttpsError::InvalidKey(format!("failed to parse private key: {err}"))
    })?;
    let public = leaf.public_key().map_err(|err| {
        ServeHttpsError::InvalidCert(format!("failed to read certificate public key: {err}"))
    })?;
    if !private.public_eq(&public) {
        return Err(ServeHttpsError::Tls(
            "private key does not match certificate".to_string(),
        ));
    }
    Ok(())
}

fn pem_is_encrypted(pem: &[u8]) -> bool {
    let text = String::from_utf8_lossy(pem);
    text.contains("BEGIN ENCRYPTED PRIVATE KEY")
        || text.contains("Proc-Type: 4,ENCRYPTED")
        || text.contains("DEK-Info:")
}

#[derive(Clone, Copy)]
enum KeyFormat {
    Pkcs8,
    Pkcs1,
    Sec1,
}

fn first_key_der(
    pem: &[u8],
    format: KeyFormat,
) -> Result<Option<rustls::PrivateKey>, ServeHttpsError> {
    let mut reader = std::io::BufReader::new(std::io::Cursor::new(pem));
    match format {
        KeyFormat::Pkcs8 => {
            for item in rustls_pemfile::pkcs8_private_keys(&mut reader) {
                let key = item.map_err(|err| {
                    ServeHttpsError::InvalidKey(format!(
                        "failed to parse PKCS#8 private key: {err}"
                    ))
                })?;
                return Ok(Some(rustls::PrivateKey(key.secret_pkcs8_der().to_vec())));
            }
        }
        KeyFormat::Pkcs1 => {
            for item in rustls_pemfile::rsa_private_keys(&mut reader) {
                let key = item.map_err(|err| {
                    ServeHttpsError::InvalidKey(format!(
                        "failed to parse PKCS#1 private key: {err}"
                    ))
                })?;
                return Ok(Some(rustls::PrivateKey(key.secret_pkcs1_der().to_vec())));
            }
        }
        KeyFormat::Sec1 => {
            for item in rustls_pemfile::ec_private_keys(&mut reader) {
                let key = item.map_err(|err| {
                    ServeHttpsError::InvalidKey(format!("failed to parse SEC1 private key: {err}"))
                })?;
                return Ok(Some(rustls::PrivateKey(key.secret_sec1_der().to_vec())));
            }
        }
    }
    Ok(None)
}

/// JSON health document served when no script is selected.
pub fn health_response(version: &str) -> Http11Response {
    let mut body = serde_json::to_vec(&serde_json::json!({
        "runtime": "amberjs",
        "ok": true,
        "version": version,
    }))
    .unwrap_or_else(|_| b"{}".to_vec());
    body.push(b'\n');
    Http11Response {
        status: 200,
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        body,
    }
}

/// Turn the fetch bridge's JSON result into a response.
///
/// A missing `status` defaults to 200. A status outside 100–599 becomes 500.
/// A non-object payload becomes 500. Header values that are not strings are skipped.
pub fn response_from_handler_json(raw: &str) -> Http11Response {
    let parsed: Value = match serde_json::from_str(raw.trim()) {
        Ok(value) => value,
        Err(_) => return text_response(500, "invalid handler result"),
    };
    let Some(obj) = parsed.as_object() else {
        return text_response(500, "invalid handler result");
    };
    let status = match obj.get("status").and_then(Value::as_u64) {
        Some(code) if (100..600).contains(&code) => code as u16,
        Some(_) => 500,
        None => 200,
    };
    let body = obj
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or("")
        .as_bytes()
        .to_vec();
    let headers = match obj.get("headers").and_then(Value::as_object) {
        Some(map) => map
            .iter()
            .filter_map(|(name, value)| value.as_str().map(|text| (name.clone(), text.to_string())))
            .collect(),
        None => vec![("Content-Type".to_string(), "text/plain".to_string())],
    };
    Http11Response {
        status,
        headers,
        body,
    }
}

fn text_response(status: u16, body: &str) -> Http11Response {
    Http11Response {
        status,
        headers: vec![(
            "Content-Type".to_string(),
            "text/plain; charset=utf-8".to_string(),
        )],
        body: body.as_bytes().to_vec(),
    }
}

/// Accept cleartext connections until the listener is closed.
///
/// Same HTTP/1.1 framing and limits as [`serve_connections`], without TLS.
pub fn serve_plain_connections<F>(listener: TcpListener, limits: ServeLimits, mut handler: F)
where
    F: FnMut(&Http11Request) -> Http11Response,
{
    for incoming in listener.incoming() {
        let Ok(tcp) = incoming else {
            continue;
        };
        handle_plain_connection(tcp, &limits, &mut handler);
    }
}

/// Accept TLS connections until the listener is closed. Per-connection failures stay in-process.
pub fn serve_connections<F>(
    listener: TcpListener,
    tls_config: Arc<rustls::ServerConfig>,
    limits: ServeLimits,
    mut handler: F,
) where
    F: FnMut(&Http11Request) -> Http11Response,
{
    for incoming in listener.incoming() {
        let Ok(tcp) = incoming else {
            continue;
        };
        handle_connection(tcp, &tls_config, &limits, &mut handler);
    }
}

fn handle_plain_connection<F>(tcp: std::net::TcpStream, limits: &ServeLimits, handler: &mut F)
where
    F: FnMut(&Http11Request) -> Http11Response,
{
    let _ = tcp.set_read_timeout(Some(limits.read_timeout));
    let _ = tcp.set_write_timeout(Some(limits.read_timeout));
    let mut stream = tcp;
    match read_http11(&mut stream, limits) {
        Ok(Incoming::Closed) => {}
        Ok(Incoming::Request(request)) => {
            let response = handler(&request);
            let bytes = encode_response(&request.method, &response);
            let _ = stream.write_all(&bytes);
            let _ = stream.flush();
        }
        Err(ReadHttpError::Protocol(failure)) => {
            let bytes = encode_failure(failure);
            let _ = stream.write_all(&bytes);
            let _ = stream.flush();
        }
        Err(ReadHttpError::Io(err)) => {
            if err.kind() == io::ErrorKind::TimedOut {
                let bytes = encode_failure(ProtocolFailure {
                    status: 408,
                    body: "request timeout",
                    head: false,
                });
                let _ = stream.write_all(&bytes);
                let _ = stream.flush();
            }
        }
    }
    let _ = stream.shutdown(std::net::Shutdown::Write);
}

fn handle_connection<F>(
    tcp: std::net::TcpStream,
    tls_config: &Arc<rustls::ServerConfig>,
    limits: &ServeLimits,
    handler: &mut F,
) where
    F: FnMut(&Http11Request) -> Http11Response,
{
    let _ = tcp.set_read_timeout(Some(limits.read_timeout));
    let _ = tcp.set_write_timeout(Some(limits.read_timeout));
    let Ok(conn) = rustls::ServerConnection::new(Arc::clone(tls_config)) else {
        return;
    };
    let mut stream = rustls::StreamOwned::new(conn, tcp);
    match read_http11(&mut stream, limits) {
        Ok(Incoming::Closed) => {}
        Ok(Incoming::Request(request)) => {
            let response = handler(&request);
            let bytes = encode_response(&request.method, &response);
            let _ = write_response(&mut stream, &bytes);
        }
        Err(ReadHttpError::Protocol(failure)) => {
            if !stream.conn.is_handshaking() {
                let bytes = encode_failure(failure);
                let _ = write_response(&mut stream, &bytes);
            }
        }
        Err(ReadHttpError::Io(err)) => {
            if stream.conn.is_handshaking() {
                return;
            }
            if err.kind() == io::ErrorKind::TimedOut {
                let bytes = encode_failure(ProtocolFailure {
                    status: 408,
                    body: "request timeout",
                    head: false,
                });
                let _ = write_response(&mut stream, &bytes);
            }
        }
    }
    if !stream.conn.is_handshaking() {
        let _ = close_tls(&mut stream);
    }
}

/// `rustls::Stream::write` accepts plaintext and then ignores `complete_io`
/// errors, so a later drop can leave the body in the TLS buffer. Drain until
/// rustls has nothing left to write, then send `close_notify`.
fn write_response(
    stream: &mut rustls::StreamOwned<rustls::ServerConnection, std::net::TcpStream>,
    bytes: &[u8],
) -> io::Result<()> {
    stream.write_all(bytes)?;
    flush_tls(stream)
}

fn flush_tls(
    stream: &mut rustls::StreamOwned<rustls::ServerConnection, std::net::TcpStream>,
) -> io::Result<()> {
    while stream.conn.wants_write() {
        stream.conn.complete_io(&mut stream.sock)?;
    }
    stream.sock.flush()
}

fn close_tls(
    stream: &mut rustls::StreamOwned<rustls::ServerConnection, std::net::TcpStream>,
) -> io::Result<()> {
    stream.conn.send_close_notify();
    flush_tls(stream)?;
    let _ = stream.sock.shutdown(std::net::Shutdown::Write);
    Ok(())
}

fn read_http11<R: Read>(reader: &mut R, limits: &ServeLimits) -> Result<Incoming, ReadHttpError> {
    let block = read_header_block(reader, limits.max_header_bytes)?;
    let Some((headers, rest)) = block else {
        return Ok(Incoming::Closed);
    };
    parse_request(&headers, &rest, reader, limits)
}

fn read_header_block<R: Read>(
    reader: &mut R,
    max_header_bytes: usize,
) -> Result<Option<(Vec<u8>, Vec<u8>)>, ReadHttpError> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        if let Some(pos) = find_crlf_crlf(&buf) {
            let header_len = pos + 4;
            if header_len > max_header_bytes {
                return Err(protocol(431, "request headers too large", false));
            }
            let rest = buf[header_len..].to_vec();
            buf.truncate(header_len);
            return Ok(Some((buf, rest)));
        }
        if buf.len() > max_header_bytes {
            return Err(protocol(431, "request headers too large", false));
        }
        match read_some(reader, &mut tmp)? {
            0 => {
                if buf.is_empty() {
                    return Ok(None);
                }
                return Err(protocol(400, "bad request", false));
            }
            n => buf.extend_from_slice(&tmp[..n]),
        }
    }
}

fn read_some<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<usize, ReadHttpError> {
    loop {
        match reader.read(buf) {
            Ok(n) => return Ok(n),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) if err.kind() == io::ErrorKind::TimedOut => {
                return Err(protocol(408, "request timeout", false));
            }
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(0),
            Err(err) => return Err(ReadHttpError::Io(err)),
        }
    }
}

fn parse_request<R: Read>(
    header_block: &[u8],
    rest: &[u8],
    reader: &mut R,
    limits: &ServeLimits,
) -> Result<Incoming, ReadHttpError> {
    if header_block.len() < 4 {
        return Err(protocol(400, "bad request", false));
    }
    let header_bytes = &header_block[..header_block.len() - 4];
    if header_bytes.contains(&0) {
        return Err(protocol(400, "bad request", false));
    }
    let text =
        std::str::from_utf8(header_bytes).map_err(|_| protocol(400, "bad request", false))?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = request_line.split(' ').collect();
    if parts.len() != 3 {
        return Err(protocol(400, "bad request", false));
    }
    let method = parts[0];
    let target = parts[1];
    let version = parts[2];
    if version != "HTTP/1.1" {
        if version.starts_with("HTTP/") {
            return Err(protocol(505, "HTTP/1.1 required", false));
        }
        return Err(protocol(400, "bad request", false));
    }
    if !is_uppercase_method(method) {
        return Err(protocol(400, "bad request", false));
    }
    let head = method == "HEAD";
    if !target.starts_with('/') || target.contains('\0') {
        return Err(protocol(400, "origin-form target required", head));
    }

    let mut headers = HashMap::new();
    let mut header_count = 0usize;
    let mut host_count = 0usize;
    let mut content_lengths = Vec::new();
    let mut saw_transfer_encoding = false;
    let mut saw_expect = false;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            return Err(protocol(400, "bad request", head));
        }
        let Some((name, raw_value)) = line.split_once(':') else {
            return Err(protocol(400, "bad request", head));
        };
        if !is_token(name) || raw_value.as_bytes().contains(&0) {
            return Err(protocol(400, "bad request", head));
        }
        header_count += 1;
        if header_count > limits.max_header_count {
            return Err(protocol(431, "request headers too large", head));
        }
        let value = raw_value.trim();
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "host" => host_count += 1,
            "content-length" => content_lengths.push(value.to_string()),
            "transfer-encoding" => saw_transfer_encoding = true,
            "expect" => saw_expect = true,
            _ => {}
        }
        headers.insert(lower, value.to_string());
    }

    if host_count == 0 {
        return Err(protocol(400, "host header required", head));
    }
    if host_count > 1 {
        return Err(protocol(400, "invalid host header", head));
    }
    if saw_transfer_encoding && !content_lengths.is_empty() {
        return Err(protocol(400, "invalid content-length", head));
    }
    if saw_transfer_encoding {
        return Err(protocol(501, "transfer-encoding is not supported", head));
    }
    if saw_expect {
        return Err(protocol(417, "expectation failed", head));
    }
    if content_lengths.len() > 1 {
        return Err(protocol(400, "invalid content-length", head));
    }
    let body_len = if let Some(value) = content_lengths.first() {
        parse_content_length(value)?
    } else {
        0
    };
    if body_len > limits.max_body_bytes as u64 {
        return Err(protocol(413, "request body too large", head));
    }
    let body = read_body(reader, rest, body_len as usize)?;
    Ok(Incoming::Request(Http11Request {
        method: method.to_string(),
        target: target.to_string(),
        headers,
        body,
    }))
}

fn parse_content_length(value: &str) -> Result<u64, ReadHttpError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(protocol(400, "invalid content-length", false));
    }
    value
        .parse::<u64>()
        .map_err(|_| protocol(400, "invalid content-length", false))
}

fn read_body<R: Read>(reader: &mut R, rest: &[u8], need: usize) -> Result<Vec<u8>, ReadHttpError> {
    let mut body = Vec::with_capacity(need);
    let take = rest.len().min(need);
    body.extend_from_slice(&rest[..take]);
    let mut tmp = [0u8; 8192];
    while body.len() < need {
        let want = (need - body.len()).min(tmp.len());
        match read_some(reader, &mut tmp[..want])? {
            0 => return Err(protocol(400, "incomplete request body", false)),
            n => body.extend_from_slice(&tmp[..n]),
        }
    }
    Ok(body)
}

fn protocol(status: u16, body: &'static str, head: bool) -> ReadHttpError {
    ReadHttpError::Protocol(ProtocolFailure { status, body, head })
}

fn is_uppercase_method(method: &str) -> bool {
    (1..=20).contains(&method.len()) && method.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn is_token(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            matches!(
                byte,
                b'!'
                    | b'#'
                    | b'$'
                    | b'%'
                    | b'&'
                    | b'\''
                    | b'*'
                    | b'+'
                    | b'-'
                    | b'.'
                    | b'^'
                    | b'_'
                    | b'`'
                    | b'|'
                    | b'~'
                    | b'0'..=b'9'
                    | b'A'..=b'Z'
                    | b'a'..=b'z'
            )
        })
}

fn find_crlf_crlf(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|window| window == b"\r\n\r\n")
}

fn encode_failure(failure: ProtocolFailure) -> Vec<u8> {
    let response = text_response(failure.status, failure.body);
    let method = if failure.head { "HEAD" } else { "GET" };
    encode_response(method, &response)
}

fn encode_response(method: &str, response: &Http11Response) -> Vec<u8> {
    let status = if (100..600).contains(&response.status) {
        response.status
    } else {
        500
    };
    let reason = reason_phrase(status);
    let head = method.eq_ignore_ascii_case("HEAD");
    let mut out = Vec::with_capacity(128 + response.body.len());
    out.extend_from_slice(format!("HTTP/1.1 {status} {reason}\r\n").as_bytes());
    for (name, value) in &response.headers {
        if !header_is_safe(name, value) {
            continue;
        }
        if name.eq_ignore_ascii_case("content-length")
            || name.eq_ignore_ascii_case("connection")
            || name.eq_ignore_ascii_case("transfer-encoding")
        {
            continue;
        }
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(
        format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            response.body.len()
        )
        .as_bytes(),
    );
    if !head {
        out.extend_from_slice(&response.body);
    }
    out
}

fn header_is_safe(name: &str, value: &str) -> bool {
    is_token(name) && !value.bytes().any(|byte| matches!(byte, b'\r' | b'\n' | 0))
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        417 => "Expectation Failed",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        505 => "HTTP Version Not Supported",
        _ => "Status",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor, Read};
    use std::process::Command;
    use std::thread;

    fn assert_status(raw: &[u8], status: u16, body: &str) {
        let text = String::from_utf8_lossy(raw);
        let line = text.lines().next().unwrap_or("");
        assert!(
            line.starts_with(&format!("HTTP/1.1 {status} ")),
            "status line {line} for body {text}"
        );
        assert!(
            !line.ends_with(" OK") || status == 200,
            "non-200 must not use reason OK: {line}"
        );
        assert!(text.contains(body), "missing {body:?} in {text}");
    }

    fn exchange(bytes: &[u8]) -> Result<Incoming, ReadHttpError> {
        let mut cursor = Cursor::new(bytes.to_vec());
        read_http11(&mut cursor, &ServeLimits::default())
    }

    fn expect_protocol(bytes: &[u8], status: u16, body: &str) {
        match exchange(bytes) {
            Err(ReadHttpError::Protocol(failure)) => {
                assert_eq!(failure.status, status, "{body}");
                assert_eq!(failure.body, body);
                let encoded = encode_failure(failure);
                assert_status(&encoded, status, body);
                assert!(encoded.windows(4).any(|w| w == b"\r\n\r\n"));
                assert!(String::from_utf8_lossy(&encoded).contains("Connection: close"));
            }
            other => panic!("expected {status} {body}, got {other:?}"),
        }
    }

    #[test]
    fn documented_limits_match_contract_numbers() {
        assert_eq!(MAX_HEADER_BYTES, 16 * 1024);
        assert_eq!(MAX_BODY_BYTES, 1024 * 1024);
        assert_eq!(MAX_HEADER_COUNT, 64);
        assert_eq!(MAX_PEM_BYTES, 1024 * 1024);
        assert_eq!(READ_TIMEOUT, Duration::from_secs(10));
        assert_eq!(SERVE_ERROR_PREFIX, "error: amber serve:");
        assert_eq!(ServeLimits::default().read_timeout, Duration::from_secs(10));
    }

    #[test]
    fn header_limit_is_inclusive_and_rejects_one_extra_byte() {
        let ok = sized_headers(MAX_HEADER_BYTES);
        match exchange(&ok) {
            Ok(Incoming::Request(req)) => {
                assert_eq!(req.method, "GET");
                assert_eq!(req.target, "/");
            }
            other => panic!("exact header limit should parse: {other:?}"),
        }
        expect_protocol(
            &sized_headers(MAX_HEADER_BYTES + 1),
            431,
            "request headers too large",
        );
    }

    #[test]
    fn header_count_includes_host() {
        let ok = headers_with_extras(MAX_HEADER_COUNT - 1);
        assert!(matches!(exchange(&ok), Ok(Incoming::Request(_))));
        expect_protocol(
            &headers_with_extras(MAX_HEADER_COUNT),
            431,
            "request headers too large",
        );
    }

    #[test]
    fn rejects_version_method_target_and_framing() {
        expect_protocol(
            b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n",
            505,
            "HTTP/1.1 required",
        );
        expect_protocol(
            b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n",
            505,
            "HTTP/1.1 required",
        );
        expect_protocol(
            b"get / HTTP/1.1\r\nHost: localhost\r\n\r\n",
            400,
            "bad request",
        );
        expect_protocol(
            b"GET http://example/ HTTP/1.1\r\nHost: localhost\r\n\r\n",
            400,
            "origin-form target required",
        );
        expect_protocol(b"GET / HTTP/1.1\r\n\r\n", 400, "host header required");
        expect_protocol(
            b"GET / HTTP/1.1\r\nHost: a\r\nHost: b\r\n\r\n",
            400,
            "invalid host header",
        );
        expect_protocol(
            b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\n",
            400,
            "invalid content-length",
        );
        expect_protocol(
            b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: no\r\n\r\n",
            400,
            "invalid content-length",
        );
        expect_protocol(
            b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 999999999999999999999\r\n\r\n",
            400,
            "invalid content-length",
        );
        expect_protocol(
            b"POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\nContent-Length: 1\r\n\r\n",
            400,
            "invalid content-length",
        );
        expect_protocol(
            b"POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
            501,
            "transfer-encoding is not supported",
        );
        expect_protocol(
            b"GET / HTTP/1.1\r\nHost: localhost\r\nExpect: 100-continue\r\n\r\n",
            417,
            "expectation failed",
        );
        let too_big = (MAX_BODY_BYTES as u64 + 1).to_string();
        let raw =
            format!("POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: {too_big}\r\n\r\n");
        expect_protocol(raw.as_bytes(), 413, "request body too large");
        expect_protocol(
            b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\n\r\nab",
            400,
            "incomplete request body",
        );
        expect_protocol(b"GET / HTTP/1.1\nHost: localhost\n\n", 400, "bad request");
    }

    #[test]
    fn body_arriving_after_headers_is_kept() {
        let mut reader = SeqReader {
            chunks: vec![
                b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\n\r\n".to_vec(),
                b"hello".to_vec(),
            ],
            index: 0,
            offset: 0,
        };
        match read_http11(&mut reader, &ServeLimits::default()) {
            Ok(Incoming::Request(req)) => {
                assert_eq!(req.target, "/echo");
                assert_eq!(req.body, b"hello");
            }
            other => panic!("split body was not read: {other:?}"),
        }
    }

    #[test]
    fn timeout_while_headers_are_incomplete_is_408() {
        let mut reader = StallReader {
            first: b"GET / HTTP/1.1\r\nHost: localhost\r\n".to_vec(),
            sent: false,
        };
        match read_http11(&mut reader, &ServeLimits::default()) {
            Err(ReadHttpError::Protocol(failure)) => {
                assert_eq!(failure.status, 408);
                assert_eq!(failure.body, "request timeout");
            }
            other => panic!("expected timeout, got {other:?}"),
        }
    }

    #[test]
    fn response_reasons_lengths_and_header_injection() {
        let created = response_from_handler_json(
            r#"{"status":201,"headers":{"x-amber":"serve","x-bad":"a\r\nX-Injected: yes"},"body":"é"}"#,
        );
        let bytes = encode_response("POST", &created);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("HTTP/1.1 201 Created\r\n"), "{text}");
        assert!(text.contains("x-amber: serve\r\n"), "{text}");
        assert!(!text.contains("X-Injected"), "{text}");
        assert!(text.contains("Content-Length: 2\r\n"), "{text}");
        assert!(bytes.ends_with("é".as_bytes()));

        let head = encode_response("HEAD", &created);
        let head_text = String::from_utf8_lossy(&head);
        assert!(head_text.contains("Content-Length: 2\r\n"), "{head_text}");
        assert!(!head.ends_with("é".as_bytes()));
        assert!(head_text.ends_with("\r\n\r\n"), "{head_text}");

        let missing = response_from_handler_json("not-json");
        let missing_bytes = encode_response("GET", &missing);
        assert_status(&missing_bytes, 500, "invalid handler result");

        let not_found = response_from_handler_json(
            r#"{"status":404,"headers":{"content-type":"text/plain"},"body":"Not Found: No fetch handler exported"}"#,
        );
        let not_found_bytes = encode_response("GET", &not_found);
        assert_status(&not_found_bytes, 404, "Not Found");
        assert!(String::from_utf8_lossy(&not_found_bytes).starts_with("HTTP/1.1 404 Not Found\r\n"));

        let health = health_response("1.17.0");
        let health_bytes = encode_response("GET", &health);
        let health_text = String::from_utf8_lossy(&health_bytes);
        assert!(health_text.contains("\"runtime\":\"amberjs\""));
        assert!(health_text.contains("\"ok\":true"));
        assert!(health_text.contains("\"version\":\"1.17.0\""));
        let wrapped = wrap_user_script("function fetch() { return 1; }");
        assert!(wrapped.contains("fetch !== globalThis.fetch"));
        assert!(wrapped.contains("function fetch() { return 1; }"));
    }

    #[test]
    fn pem_files_fail_closed_and_accept_pkcs1_pkcs8_and_sec1() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("missing.pem");
        let err = load_server_config(&missing, &missing).unwrap_err();
        assert!(err.to_string().contains("TLS certificate not found"));
        assert!(err.to_string().starts_with(SERVE_ERROR_PREFIX));

        let dir_path = dir.path().join("certdir");
        fs::create_dir(&dir_path).unwrap();
        let key = dir.path().join("key.pem");
        fs::write(&key, b"not a key").unwrap();
        let err = load_server_config(&dir_path, &key).unwrap_err();
        assert!(err.to_string().contains("not a file"), "{err}");

        let cert = dir.path().join("cert.pem");
        fs::write(&cert, b"this is not a certificate").unwrap();
        fs::write(&key, b"this is not a key").unwrap();
        let err = load_server_config(&cert, &key).unwrap_err();
        assert!(
            err.to_string().contains("no certificate found in PEM"),
            "{err}"
        );

        fs::write(&cert, vec![b'A'; MAX_PEM_BYTES + 1]).unwrap();
        let err = load_server_config(&cert, &key).unwrap_err();
        assert!(err.to_string().contains("exceeds"), "{err}");

        let (rsa_cert, rsa_key) = mint_rsa(dir.path());
        assert!(
            load_server_config(&rsa_cert, &rsa_key).is_ok(),
            "PKCS#8 RSA should load"
        );
        let pkcs1 = dir.path().join("pkcs1.pem");
        openssl(&[
            "pkey",
            "-in",
            rsa_key.to_str().unwrap(),
            "-traditional",
            "-out",
            pkcs1.to_str().unwrap(),
        ]);
        let pkcs1_text = fs::read_to_string(&pkcs1).unwrap();
        assert!(pkcs1_text.contains("BEGIN RSA PRIVATE KEY"), "{pkcs1_text}");
        assert!(
            load_server_config(&rsa_cert, &pkcs1).is_ok(),
            "PKCS#1 RSA should load"
        );

        let (ec_cert, ec_key) = mint_ec(dir.path());
        assert!(load_server_config(&ec_cert, &ec_key).is_ok(), "EC PKCS#8");
        let sec1 = dir.path().join("sec1.pem");
        openssl(&[
            "pkey",
            "-in",
            ec_key.to_str().unwrap(),
            "-traditional",
            "-out",
            sec1.to_str().unwrap(),
        ]);
        let sec1_text = fs::read_to_string(&sec1).unwrap();
        assert!(sec1_text.contains("BEGIN EC PRIVATE KEY"), "{sec1_text}");
        assert!(
            load_server_config(&ec_cert, &sec1).is_ok(),
            "SEC1 EC should load"
        );

        let err = load_server_config(&rsa_cert, &ec_key).unwrap_err();
        assert!(err.to_string().contains("invalid TLS material"), "{err}");

        let enc_key = dir.path().join("enc.pem");
        let enc_cert = dir.path().join("enc-cert.pem");
        openssl(&[
            "genpkey",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            "rsa_keygen_bits:2048",
            "-aes-128-cbc",
            "-pass",
            "pass:secret",
            "-out",
            enc_key.to_str().unwrap(),
        ]);
        openssl(&[
            "req",
            "-new",
            "-x509",
            "-key",
            enc_key.to_str().unwrap(),
            "-passin",
            "pass:secret",
            "-out",
            enc_cert.to_str().unwrap(),
            "-days",
            "1",
            "-subj",
            "/CN=localhost",
        ]);
        let err = load_server_config(&enc_cert, &enc_key).unwrap_err();
        assert!(
            err.to_string()
                .contains("encrypted private keys are not supported"),
            "{err}"
        );
    }

    #[test]
    fn rustls_serves_http11_and_rejects_h2_alpn() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (cert, key) = mint_ec(dir.path());
        let tls = load_server_config(&cert, &key).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server_tls = Arc::clone(&tls);
        let handle = thread::spawn(move || {
            let (tcp, _) = listener.accept().unwrap();
            handle_connection(tcp, &server_tls, &ServeLimits::default(), &mut |_| {
                health_response("test")
            });
        });
        let response = tls_exchange(
            addr,
            b"http/1.1",
            b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        handle.join().unwrap();
        assert_status(&response, 200, "\"ok\":true");

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server_tls = Arc::clone(&tls);
        let handle = thread::spawn(move || {
            let (tcp, _) = listener.accept().unwrap();
            handle_connection(tcp, &server_tls, &ServeLimits::default(), &mut |_| {
                health_response("test")
            });
        });
        let err = tls_exchange_result(addr, b"h2", b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");
        handle.join().unwrap();
        assert!(
            err.is_err(),
            "h2-only ALPN must fail the handshake: {err:?}"
        );
    }

    fn sized_headers(total: usize) -> Vec<u8> {
        let prefix = b"GET / HTTP/1.1\r\nHost: localhost\r\nX-Pad: ";
        let suffix = b"\r\n\r\n";
        assert!(total >= prefix.len() + suffix.len());
        let pad = total - prefix.len() - suffix.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(prefix);
        out.extend(std::iter::repeat(b'a').take(pad));
        out.extend_from_slice(suffix);
        assert_eq!(out.len(), total);
        out
    }

    fn headers_with_extras(extra: usize) -> Vec<u8> {
        let mut text = String::from("GET / HTTP/1.1\r\nHost: localhost\r\n");
        for index in 0..extra {
            text.push_str(&format!("X-{index}: v\r\n"));
        }
        text.push_str("\r\n");
        text.into_bytes()
    }

    struct SeqReader {
        chunks: Vec<Vec<u8>>,
        index: usize,
        offset: usize,
    }

    impl Read for SeqReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.index >= self.chunks.len() {
                return Ok(0);
            }
            let chunk = &self.chunks[self.index];
            let available = chunk.len() - self.offset;
            let n = available.min(buf.len());
            buf[..n].copy_from_slice(&chunk[self.offset..self.offset + n]);
            self.offset += n;
            if self.offset == chunk.len() {
                self.index += 1;
                self.offset = 0;
            }
            Ok(n)
        }
    }

    struct StallReader {
        first: Vec<u8>,
        sent: bool,
    }

    impl Read for StallReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if !self.sent {
                self.sent = true;
                let n = self.first.len().min(buf.len());
                buf[..n].copy_from_slice(&self.first[..n]);
                self.first.drain(..n);
                if self.first.is_empty() {
                    return Ok(n);
                }
                return Ok(n);
            }
            Err(io::Error::new(io::ErrorKind::TimedOut, "stalled"))
        }
    }

    fn openssl(args: &[&str]) {
        let output = Command::new("openssl")
            .args(args)
            .output()
            .expect("openssl");
        assert!(
            output.status.success(),
            "openssl {args:?} failed: {}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }

    fn mint_rsa(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let cert = dir.join("rsa-cert.pem");
        let key = dir.join("rsa-key.pem");
        openssl(&[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key.to_str().unwrap(),
            "-out",
            cert.to_str().unwrap(),
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=localhost",
        ]);
        (cert, key)
    }

    fn mint_ec(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let cert = dir.join("ec-cert.pem");
        let key = dir.join("ec-key.pem");
        openssl(&[
            "req",
            "-x509",
            "-newkey",
            "ec",
            "-pkeyopt",
            "ec_paramgen_curve:prime256v1",
            "-keyout",
            key.to_str().unwrap(),
            "-out",
            cert.to_str().unwrap(),
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=localhost",
            "-addext",
            "subjectAltName=DNS:localhost,IP:127.0.0.1",
        ]);
        (cert, key)
    }

    fn tls_exchange(addr: std::net::SocketAddr, alpn: &[u8], request: &[u8]) -> Vec<u8> {
        tls_exchange_result(addr, alpn, request).expect("tls exchange")
    }

    fn tls_exchange_result(
        addr: std::net::SocketAddr,
        alpn: &[u8],
        request: &[u8],
    ) -> io::Result<Vec<u8>> {
        let mut tcp = std::net::TcpStream::connect(addr)?;
        tcp.set_read_timeout(Some(Duration::from_secs(3)))?;
        tcp.set_write_timeout(Some(Duration::from_secs(3)))?;
        let name = rustls::ServerName::try_from("localhost").expect("localhost");
        let mut conn = rustls::ClientConnection::new(client_config(alpn), name)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        let mut stream = rustls::Stream::new(&mut conn, &mut tcp);
        stream.write_all(request)?;
        stream.flush()?;
        let mut out = Vec::new();
        let mut tmp = [0u8; 1024];
        loop {
            match stream.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => out.extend_from_slice(&tmp[..n]),
                Err(err) if err.kind() == io::ErrorKind::TimedOut => break,
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof && !out.is_empty() => break,
                Err(err) => return Err(err),
            }
        }
        Ok(out)
    }

    fn client_config(alpn: &[u8]) -> Arc<rustls::ClientConfig> {
        struct AcceptAny;
        impl rustls::client::ServerCertVerifier for AcceptAny {
            fn verify_server_cert(
                &self,
                _end_entity: &rustls::Certificate,
                _intermediates: &[rustls::Certificate],
                _server_name: &rustls::ServerName,
                _scts: &mut dyn Iterator<Item = &[u8]>,
                _ocsp_response: &[u8],
                _now: std::time::SystemTime,
            ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
                Ok(rustls::client::ServerCertVerified::assertion())
            }
        }
        let mut config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(Arc::new(AcceptAny))
            .with_no_client_auth();
        config.alpn_protocols = vec![alpn.to_vec()];
        Arc::new(config)
    }
}

impl fmt::Debug for ReadHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "Io({err})"),
            Self::Protocol(failure) => f
                .debug_struct("Protocol")
                .field("failure", failure)
                .finish(),
        }
    }
}

impl fmt::Debug for Incoming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => write!(f, "Closed"),
            Self::Request(req) => f
                .debug_struct("Request")
                .field("method", &req.method)
                .field("target", &req.target)
                .field("body", &req.body)
                .finish(),
        }
    }
}
