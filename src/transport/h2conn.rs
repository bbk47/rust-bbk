use std::io;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use bytes::{Buf, Bytes};
use h2::{RecvStream, SendStream};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::dial_tcp;

fn io_other<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e.to_string())
}

/// Full-duplex byte stream over an HTTP/2 POST, interoperable with Go's
/// `posener/h2conn`: the request body is the write side and the response body
/// is the read side.
pub struct H2Conn {
    send: SendStream<Bytes>,
    recv: RecvStream,
    read_rem: Bytes,
    // Keeps the client SendRequest handle alive so the underlying HTTP/2
    // connection isn't torn down while this stream is in use. None on server.
    _keepalive: Option<h2::client::SendRequest<Bytes>>,
}

impl H2Conn {
    pub fn new(send: SendStream<Bytes>, recv: RecvStream) -> Self {
        H2Conn { send, recv, read_rem: Bytes::new(), _keepalive: None }
    }

    fn new_client(send: SendStream<Bytes>, recv: RecvStream, keepalive: h2::client::SendRequest<Bytes>) -> Self {
        H2Conn { send, recv, read_rem: Bytes::new(), _keepalive: Some(keepalive) }
    }
}

impl AsyncRead for H2Conn {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.read_rem.is_empty() {
            match ready!(this.recv.poll_data(cx)) {
                Some(Ok(data)) => {
                    let _ = this.recv.flow_control().release_capacity(data.len());
                    this.read_rem = data;
                }
                Some(Err(e)) => return Poll::Ready(Err(io_other(e))),
                None => return Poll::Ready(Ok(())),
            }
        }
        let n = std::cmp::min(buf.remaining(), this.read_rem.len());
        buf.put_slice(&this.read_rem[..n]);
        this.read_rem.advance(n);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for H2Conn {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        this.send.reserve_capacity(buf.len());
        loop {
            let cap = this.send.capacity();
            if cap > 0 {
                let n = std::cmp::min(cap, buf.len());
                this.send
                    .send_data(Bytes::copy_from_slice(&buf[..n]), false)
                    .map_err(io_other)?;
                return Poll::Ready(Ok(n));
            }
            match ready!(this.send.poll_capacity(cx)) {
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Poll::Ready(Err(io_other(e))),
                None => return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into())),
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.send.send_data(Bytes::new(), true).map_err(io_other)?;
        Poll::Ready(Ok(()))
    }
}

/// Establishes an HTTP/2 POST tunnel to `https://host:port/path` with ALPN h2
/// and TLS verification disabled (matches Go's DialRawH2).
pub async fn dial_h2(host: &str, port: &str, path: &str) -> io::Result<H2Conn> {
    let tcp = dial_tcp(host, port).await?;

    let mut builder = SslConnector::builder(SslMethod::tls()).map_err(io_other)?;
    builder.set_verify(SslVerifyMode::NONE);
    builder.set_alpn_protos(b"\x02h2").map_err(io_other)?;
    let connector = builder.build();
    let config = connector.configure().map_err(io_other)?;
    let ssl = config.into_ssl(host).map_err(io_other)?;

    let mut tls = tokio_openssl::SslStream::new(ssl, tcp).map_err(io_other)?;
    Pin::new(&mut tls).connect().await.map_err(io_other)?;

    let (mut send_req, conn) = h2::client::handshake(tls).await.map_err(io_other)?;
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let url = format!("https://{}:{}{}", host, port, path);
    let req = http::Request::builder()
        .method(http::Method::POST)
        .uri(url)
        .body(())
        .map_err(io_other)?;
    let (resp_fut, send_stream) = send_req.send_request(req, false).map_err(io_other)?;
    let resp = resp_fut.await.map_err(io_other)?;
    if resp.status() == http::StatusCode::INTERNAL_SERVER_ERROR {
        return Err(io_other("server error 500"));
    }
    let recv = resp.into_body();
    Ok(H2Conn::new_client(send_stream, recv, send_req))
}
