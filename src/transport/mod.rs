use std::io;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub mod h2conn;

pub use h2conn::H2Conn;

fn io_other<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e.to_string())
}

/// Raw TCP carrier (no application framing).
pub async fn dial_tcp(host: &str, port: &str) -> io::Result<TcpStream> {
    let addr = format!("{}:{}", host, port);
    let fut = TcpStream::connect(addr);
    let conn = tokio::time::timeout(Duration::from_secs(10), fut)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "tcp dial timeout"))??;
    conn.set_nodelay(true).ok();
    Ok(conn)
}

/// Raw TLS carrier. Certificates are not verified, matching Go's
/// `InsecureSkipVerify: true`.
pub async fn dial_tls(host: &str, port: &str) -> io::Result<TlsStream<TcpStream>> {
    let tcp = dial_tcp(host, port).await?;
    let connector = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .map_err(io_other)?;
    let connector = tokio_native_tls::TlsConnector::from(connector);
    let tls = connector.connect(host, tcp).await.map_err(io_other)?;
    Ok(tls)
}

/// Raw WebSocket carrier. Wrap the returned stream in `tunnel::WsConn`.
pub async fn dial_ws(
    host: &str,
    port: &str,
    path: &str,
    secure: bool,
) -> io::Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let url = if secure {
        format!("wss://{}:{}{}", host, port, path)
    } else {
        format!("ws://{}:{}{}", host, port, path)
    };
    let req = url.into_client_request().map_err(io_other)?;
    let (ws, _resp) = connect_async(req).await.map_err(io_other)?;
    Ok(ws)
}
