// WinterTC Sockets API native engine (TC55 proposal-sockets-api)
// Provides asynchronous TCP/TLS socket connections over Web Streams.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_rustls::{client::TlsStream, TlsConnector};

static NEXT_SOCKET_ID: AtomicU64 = AtomicU64::new(1);
static EXTRA_TEST_ROOTS: StdMutex<Vec<Vec<u8>>> = StdMutex::new(Vec::new());

/// Extra DER certificates trusted by the sockets TLS client (tests / custom CAs).
pub fn install_test_root_ca_der(der: Vec<u8>) {
    EXTRA_TEST_ROOTS.lock().unwrap().push(der);
}

pub fn clear_test_root_cas() {
    EXTRA_TEST_ROOTS.lock().unwrap().clear();
}

#[derive(Clone, Debug)]
pub struct SocketInfo {
    pub remote_address: String,
    pub local_address: String,
    pub alpn: Option<String>,
}

enum ConnectionStream {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
    Closed,
}

pub struct ActiveSocket {
    pub id: u64,
    pub info: SocketInfo,
    stream: Mutex<ConnectionStream>,
    pub upgraded: bool,
    hostname: String,
}

static SOCKET_REGISTRY: Lazy<Mutex<HashMap<u64, Arc<ActiveSocket>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn tls_connector() -> Result<TlsConnector> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    if let Ok(extra) = EXTRA_TEST_ROOTS.lock() {
        for der in extra.iter() {
            let _ = roots.add(CertificateDer::from(der.clone()));
        }
    }
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

fn rustls_server_name(host: &str) -> Result<ServerName<'static>> {
    ServerName::try_from(host)
        .map(|name| name.to_owned())
        .map_err(|_| anyhow!("invalid TLS server name: {host}"))
}

async fn wrap_tls(tcp: TcpStream, host: &str, sni: Option<&str>) -> Result<TlsStream<TcpStream>> {
    let connector = tls_connector()?;
    let name_host = sni.unwrap_or(host);
    let name = rustls_server_name(name_host).or_else(|_| rustls_server_name("localhost"))?;
    let handshake = connector.connect(name, tcp);
    match tokio::time::timeout(std::time::Duration::from_secs(2), handshake).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(e)) => Err(anyhow!("TLS handshake failed: {e}")),
        Err(_) => Err(anyhow!("TLS handshake timed out")),
    }
}

pub async fn connect_socket(
    host: &str,
    port: u16,
    secure_transport: &str,
    sni: Option<&str>,
    _alpn: Vec<String>,
) -> Result<(u64, SocketInfo)> {
    let addr_str = format!("{}:{}", host, port);
    let tcp_stream = TcpStream::connect(&addr_str)
        .await
        .map_err(|e| anyhow!("Failed to connect to {}: {}", addr_str, e))?;

    let peer_addr = tcp_stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| addr_str.clone());
    let local_addr = tcp_stream
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "127.0.0.1:0".to_string());

    let info = SocketInfo {
        remote_address: peer_addr,
        local_address: local_addr,
        alpn: None,
    };

    let (stream, upgraded) = if secure_transport == "on" {
        let tls = wrap_tls(tcp_stream, host, sni).await?;
        (ConnectionStream::Tls(tls), true)
    } else {
        (ConnectionStream::Plain(tcp_stream), false)
    };

    let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::SeqCst);
    let socket = Arc::new(ActiveSocket {
        id,
        info: info.clone(),
        stream: Mutex::new(stream),
        upgraded,
        hostname: host.to_string(),
    });

    let mut reg = SOCKET_REGISTRY.lock().await;
    reg.insert(id, socket);

    Ok((id, info))
}

pub async fn read_socket(id: u64, max_bytes: usize) -> Result<Option<Vec<u8>>> {
    let socket = {
        let reg = SOCKET_REGISTRY.lock().await;
        reg.get(&id).cloned()
    };

    let socket = socket.ok_or_else(|| anyhow!("Socket {} not found or closed", id))?;
    let mut stream_guard = socket.stream.lock().await;

    let mut buf = vec![0u8; max_bytes.min(65536)];
    let n = match &mut *stream_guard {
        ConnectionStream::Plain(stream) => stream
            .read(&mut buf)
            .await
            .map_err(|e| anyhow!("Socket read error: {}", e))?,
        ConnectionStream::Tls(stream) => stream
            .read(&mut buf)
            .await
            .map_err(|e| anyhow!("Socket read error: {}", e))?,
        ConnectionStream::Closed => return Ok(None),
    };
    if n == 0 {
        Ok(None)
    } else {
        buf.truncate(n);
        Ok(Some(buf))
    }
}

pub async fn write_socket(id: u64, data: &[u8]) -> Result<usize> {
    let socket = {
        let reg = SOCKET_REGISTRY.lock().await;
        reg.get(&id).cloned()
    };

    let socket = socket.ok_or_else(|| anyhow!("Socket {} not found or closed", id))?;
    let mut stream_guard = socket.stream.lock().await;

    match &mut *stream_guard {
        ConnectionStream::Plain(stream) => {
            stream
                .write_all(data)
                .await
                .map_err(|e| anyhow!("Socket write error: {}", e))?;
            stream
                .flush()
                .await
                .map_err(|e| anyhow!("Socket flush error: {}", e))?;
            Ok(data.len())
        }
        ConnectionStream::Tls(stream) => {
            stream
                .write_all(data)
                .await
                .map_err(|e| anyhow!("Socket write error: {}", e))?;
            stream
                .flush()
                .await
                .map_err(|e| anyhow!("Socket flush error: {}", e))?;
            Ok(data.len())
        }
        ConnectionStream::Closed => Err(anyhow!("Cannot write to closed socket {}", id)),
    }
}

pub async fn close_socket(id: u64) -> Result<()> {
    let socket = {
        let mut reg = SOCKET_REGISTRY.lock().await;
        reg.remove(&id)
    };

    if let Some(s) = socket {
        let mut stream_guard = s.stream.lock().await;
        match std::mem::replace(&mut *stream_guard, ConnectionStream::Closed) {
            ConnectionStream::Plain(mut stream) => {
                let _ = stream.shutdown().await;
            }
            ConnectionStream::Tls(mut stream) => {
                let _ = stream.shutdown().await;
            }
            ConnectionStream::Closed => {}
        }
    }
    Ok(())
}

pub async fn start_tls_socket(id: u64, sni: Option<&str>) -> Result<()> {
    let socket = {
        let reg = SOCKET_REGISTRY.lock().await;
        reg.get(&id).cloned()
    };
    let socket = socket.ok_or_else(|| anyhow!("Socket {} not found or closed", id))?;
    let mut stream_guard = socket.stream.lock().await;
    let plain = match std::mem::replace(&mut *stream_guard, ConnectionStream::Closed) {
        ConnectionStream::Plain(stream) => stream,
        ConnectionStream::Tls(stream) => {
            *stream_guard = ConnectionStream::Tls(stream);
            return Ok(());
        }
        ConnectionStream::Closed => {
            return Err(anyhow!("Cannot startTls on closed socket {}", id));
        }
    };
    match wrap_tls(plain, &socket.hostname, sni).await {
        Ok(tls) => {
            *stream_guard = ConnectionStream::Tls(tls);
            Ok(())
        }
        Err(e) => {
            *stream_guard = ConnectionStream::Closed;
            Err(e)
        }
    }
}

/// Self-signed localhost TLS listener for integration tests.
pub struct TestTlsListener {
    pub port: u16,
    pub cert_der: Vec<u8>,
    _join: std::thread::JoinHandle<()>,
}

fn generate_self_signed_localhost() -> Result<(Vec<u8>, Vec<u8>)> {
    use openssl::asn1::Asn1Time;
    use openssl::bn::{BigNum, MsbOption};
    use openssl::hash::MessageDigest;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectAlternativeName};
    use openssl::x509::{X509NameBuilder, X509};

    let rsa = Rsa::generate(2048)?;
    let pkey = PKey::from_rsa(rsa)?;

    let mut name = X509NameBuilder::new()?;
    name.append_entry_by_text("CN", "localhost")?;
    let name = name.build();

    let mut builder = X509::builder()?;
    builder.set_version(2)?;
    let mut serial = BigNum::new()?;
    serial.rand(64, MsbOption::MAYBE_ZERO, false)?;
    builder.set_serial_number(serial.to_asn1_integer()?.as_ref())?;
    builder.set_subject_name(&name)?;
    builder.set_issuer_name(&name)?;
    builder.set_pubkey(&pkey)?;
    builder.set_not_before(Asn1Time::days_from_now(0)?.as_ref())?;
    builder.set_not_after(Asn1Time::days_from_now(2)?.as_ref())?;
    builder.append_extension(BasicConstraints::new().build()?)?;
    builder.append_extension(
        KeyUsage::new()
            .digital_signature()
            .key_encipherment()
            .build()?,
    )?;
    let mut san = SubjectAlternativeName::new();
    san.dns("localhost").ip("127.0.0.1");
    let san_ext = san.build(&builder.x509v3_context(None, None))?;
    builder.append_extension(san_ext)?;
    builder.sign(&pkey, MessageDigest::sha256())?;
    let cert = builder.build();
    Ok((cert.to_der()?, pkey.private_key_to_pkcs8()?))
}

/// Accept TCP then complete a rustls server handshake (for `secureTransport` / `startTls` tests).
pub fn start_self_signed_tls_listener() -> Result<TestTlsListener> {
    let (cert_der, key_der) = generate_self_signed_localhost()?;
    let cert = CertificateDer::from(cert_der.clone());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));
    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .map_err(|e| anyhow!("TLS server cert: {e}"))?;
    let server_config = Arc::new(server_config);

    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let join = std::thread::spawn(move || {
        while let Ok((mut tcp, _)) = listener.accept() {
            let cfg = server_config.clone();
            std::thread::spawn(move || {
                if let Ok(mut conn) = rustls::ServerConnection::new(cfg) {
                    let mut stream = rustls::Stream::new(&mut conn, &mut tcp);
                    let mut buf = [0u8; 32];
                    let _ = std::io::Read::read(&mut stream, &mut buf);
                }
            });
        }
    });
    Ok(TestTlsListener {
        port,
        cert_der,
        _join: join,
    })
}
