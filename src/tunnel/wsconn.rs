use std::io;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use futures::{Sink, SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

/// Adapts a message-framed WebSocket into an ordered byte stream so it can
/// carry SecureConn + yamux, mirroring Go's `tunnel.WsConn`. Each `Write`
/// becomes one binary message; `Read` concatenates incoming messages.
pub struct WsConn<S> {
    ws: WebSocketStream<S>,
    read_buf: Vec<u8>,
    read_pos: usize,
}

impl<S> WsConn<S> {
    pub fn new(ws: WebSocketStream<S>) -> Self {
        WsConn { ws, read_buf: Vec::new(), read_pos: 0 }
    }
}

fn io_other<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e.to_string())
}

impl<S> AsyncRead for WsConn<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if this.read_pos < this.read_buf.len() {
                let n = std::cmp::min(buf.remaining(), this.read_buf.len() - this.read_pos);
                buf.put_slice(&this.read_buf[this.read_pos..this.read_pos + n]);
                this.read_pos += n;
                return Poll::Ready(Ok(()));
            }
            match ready!(this.ws.poll_next_unpin(cx)) {
                Some(Ok(msg)) => match msg {
                    Message::Binary(d) => {
                        this.read_buf = d;
                        this.read_pos = 0;
                    }
                    Message::Text(t) => {
                        this.read_buf = t.into_bytes();
                        this.read_pos = 0;
                    }
                    Message::Close(_) => return Poll::Ready(Ok(())),
                    // Ping/Pong are handled internally by tungstenite; skip.
                    _ => {}
                },
                Some(Err(e)) => return Poll::Ready(Err(io_other(e))),
                None => return Poll::Ready(Ok(())),
            }
        }
    }
}

impl<S> AsyncWrite for WsConn<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(Pin::new(&mut this.ws).poll_ready(cx)).map_err(io_other)?;
        Pin::new(&mut this.ws)
            .start_send(Message::Binary(buf.to_vec()))
            .map_err(io_other)?;
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.ws.poll_flush_unpin(cx).map_err(io_other)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.ws.poll_close_unpin(cx).map_err(io_other)
    }
}
