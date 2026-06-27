use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt};

pub(crate) const STATUS_OK: u8 = 0x00;

/// A multiplexed tunnel stream. Wraps a yamux stream (adapted to tokio I/O)
/// plus the target address carried by the stream-level handshake. Half-close
/// (`CloseWrite`) maps to `poll_shutdown` -> yamux FIN, matching Go.
pub struct Stream {
    io: Compat<yamux::Stream>,
    pub addr: Vec<u8>,
}

impl Stream {
    pub fn new(raw: yamux::Stream, addr: Vec<u8>) -> Self {
        Stream { io: raw.compat(), addr }
    }

    /// Server side: signal the client that the upstream target is ready
    /// (single status byte `0x00`, replacing the old EST frame).
    pub async fn set_ready(&mut self) -> io::Result<()> {
        self.io.write_all(&[STATUS_OK]).await?;
        self.io.flush().await
    }
}

impl AsyncRead for Stream {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_read(cx, buf)
    }
}

impl AsyncWrite for Stream {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.io).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_shutdown(cx)
    }
}
