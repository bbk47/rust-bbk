use std::future::Future;
use std::io;
use std::sync::Arc;

use log::{error, info};
use openssl::pkey::PKey;
use openssl::ssl::{select_next_proto, AlpnError, SslAcceptor, SslMethod};
use openssl::x509::X509;
use std::pin::Pin;
use tokio::net::{TcpListener, TcpStream};
use tokio_openssl::SslStream;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};

use crate::transport::H2Conn;
use crate::tunnel::{self, Session, WsConn};
use crate::utils::encrypt::Encryptor;

fn io_other<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e.to_string())
}

/// Dispatches to the configured transport server. `handler` is invoked once per
/// established tunnel session (uniform `Session` type regardless of carrier).
pub async fn run<H, F>(
    work_mode: &str,
    host: &str,
    port: u16,
    path: &str,
    ssl_crt: &str,
    ssl_key: &str,
    enc: Arc<Encryptor>,
    handler: H,
) -> io::Result<()>
where
    H: Fn(Session) -> F + Clone + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let addr = format!("{}:{}", host, port);
    match work_mode {
        "tcp" => serve_tcp(addr, enc, handler).await,
        "tls" => serve_tls(addr, ssl_crt.to_string(), ssl_key.to_string(), enc, handler).await,
        "ws" => serve_ws(addr, path.to_string(), enc, handler).await,
        "h2" => serve_h2(addr, path.to_string(), ssl_crt.to_string(), ssl_key.to_string(), enc, handler).await,
        other => Err(io_other(format!("unknown workMode: {}", other))),
    }
}

fn spawn_session<H, F, C>(carrier: C, enc: Arc<Encryptor>, handler: H)
where
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    H: Fn(Session) -> F + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        match tunnel::server_session(carrier, &enc).await {
            Ok(sess) => handler(sess).await,
            Err(e) => error!("tunnel handshake failed: {}", e),
        }
    });
}

async fn serve_tcp<H, F>(addr: String, enc: Arc<Encryptor>, handler: H) -> io::Result<()>
where
    H: Fn(Session) -> F + Clone + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let ln = TcpListener::bind(&addr).await?;
    info!("tcp tunnel server listen on tcp://{}", addr);
    loop {
        let (sock, _) = ln.accept().await?;
        sock.set_nodelay(true).ok();
        spawn_session(sock, enc.clone(), handler.clone());
    }
}

async fn serve_tls<H, F>(addr: String, crt: String, key: String, enc: Arc<Encryptor>, handler: H) -> io::Result<()>
where
    H: Fn(Session) -> F + Clone + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let identity = native_tls::Identity::from_pkcs8(crt.as_bytes(), key.as_bytes()).map_err(io_other)?;
    let acceptor = native_tls::TlsAcceptor::new(identity).map_err(io_other)?;
    let acceptor = tokio_native_tls::TlsAcceptor::from(acceptor);

    let ln = TcpListener::bind(&addr).await?;
    info!("tls tunnel server listen on tls://{}", addr);
    loop {
        let (sock, _) = ln.accept().await?;
        sock.set_nodelay(true).ok();
        let acceptor = acceptor.clone();
        let enc = enc.clone();
        let handler = handler.clone();
        tokio::spawn(async move {
            match acceptor.accept(sock).await {
                Ok(tls) => spawn_session(tls, enc, handler),
                Err(e) => error!("tls accept failed: {}", e),
            }
        });
    }
}

async fn serve_ws<H, F>(addr: String, path: String, enc: Arc<Encryptor>, handler: H) -> io::Result<()>
where
    H: Fn(Session) -> F + Clone + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let ln = TcpListener::bind(&addr).await?;
    info!("websocket tunnel server listen on ws://{}{}", addr, path);
    loop {
        let (sock, _) = ln.accept().await?;
        sock.set_nodelay(true).ok();
        let path = path.clone();
        let enc = enc.clone();
        let handler = handler.clone();
        tokio::spawn(async move {
            let check = |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
                if req.uri().path() == path {
                    Ok(resp)
                } else {
                    let err = Response::builder()
                        .status(404)
                        .body(Some("not found".to_string()))
                        .unwrap();
                    Err(err)
                }
            };
            match tokio_tungstenite::accept_hdr_async(sock, check).await {
                Ok(ws) => spawn_session(WsConn::new(ws), enc, handler),
                Err(e) => error!("ws upgrade failed: {}", e),
            }
        });
    }
}

async fn serve_h2<H, F>(addr: String, path: String, crt: String, key: String, enc: Arc<Encryptor>, handler: H) -> io::Result<()>
where
    H: Fn(Session) -> F + Clone + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let cert = X509::from_pem(crt.as_bytes()).map_err(io_other)?;
    let pkey = PKey::private_key_from_pem(key.as_bytes()).map_err(io_other)?;

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).map_err(io_other)?;
    builder.set_certificate(&cert).map_err(io_other)?;
    builder.set_private_key(&pkey).map_err(io_other)?;
    builder.set_alpn_select_callback(|_ssl, client| {
        select_next_proto(b"\x02h2", client).ok_or(AlpnError::NOACK)
    });
    let acceptor = Arc::new(builder.build());

    let ln = TcpListener::bind(&addr).await?;
    info!("http2 tunnel server listen on https://{}{}", addr, path);
    loop {
        let (sock, _) = ln.accept().await?;
        sock.set_nodelay(true).ok();
        let acceptor = acceptor.clone();
        let enc = enc.clone();
        let handler = handler.clone();
        let path = path.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_h2_conn(sock, acceptor, path, enc, handler).await {
                error!("h2 conn error: {}", e);
            }
        });
    }
}

async fn handle_h2_conn<H, F>(
    sock: TcpStream,
    acceptor: Arc<SslAcceptor>,
    path: String,
    enc: Arc<Encryptor>,
    handler: H,
) -> io::Result<()>
where
    H: Fn(Session) -> F + Clone + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let ssl = openssl::ssl::Ssl::new(acceptor.context()).map_err(io_other)?;
    let mut tls = SslStream::new(ssl, sock).map_err(io_other)?;
    Pin::new(&mut tls).accept().await.map_err(io_other)?;

    let mut conn = h2::server::handshake(tls).await.map_err(io_other)?;
    while let Some(req) = conn.accept().await {
        let (req, mut respond) = req.map_err(io_other)?;
        if req.method() == http::Method::POST && req.uri().path() == path {
            let recv = req.into_body();
            let resp = http::Response::builder().status(200).body(()).map_err(io_other)?;
            let send = respond.send_response(resp, false).map_err(io_other)?;
            spawn_session(H2Conn::new(send, recv), enc.clone(), handler.clone());
        } else {
            let resp = http::Response::builder().status(404).body(()).map_err(io_other)?;
            let _ = respond.send_response(resp, true);
        }
    }
    Ok(())
}
