use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::io::{AsyncRead as FAsyncRead, AsyncWrite as FAsyncWrite};
use futures::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot, Mutex};
use yamux::{Config, Connection, Mode};

use super::stream::{Stream, STATUS_OK};

const MAX_ADDR_LEN: usize = 1024;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
const OPEN_WAIT_TIMEOUT: Duration = Duration::from_secs(15);
const MUX_WINDOW_SIZE: usize = 256 * 1024;

type OpenResp = oneshot::Sender<Result<yamux::Stream, String>>;

fn yamux_config() -> Config {
    let mut cfg = Config::default();
    cfg.set_max_num_streams(1024);
    // Per-stream initial window already defaults to 256 KiB (matches Go's
    // MaxStreamWindowSize); make sure the connection cap allows it.
    cfg.set_max_connection_receive_window(Some(MUX_WINDOW_SIZE * 1024));
    cfg.set_read_after_close(true);
    cfg
}

/// Multiplexed tunnel session built on yamux (wire-compatible with
/// hashicorp/yamux). Wraps the poll-based yamux `Connection` in a driver task
/// and exposes Go-style `open_stream` / `accept_stream`.
pub struct Session {
    open_tx: mpsc::UnboundedSender<OpenResp>,
    accept_rx: Mutex<mpsc::UnboundedReceiver<yamux::Stream>>,
    closed: Arc<AtomicBool>,
}

impl Session {
    pub fn client<T>(io: T) -> Session
    where
        T: FAsyncRead + FAsyncWrite + Unpin + Send + 'static,
    {
        Self::spawn(io, Mode::Client)
    }

    pub fn server<T>(io: T) -> Session
    where
        T: FAsyncRead + FAsyncWrite + Unpin + Send + 'static,
    {
        Self::spawn(io, Mode::Server)
    }

    fn spawn<T>(io: T, mode: Mode) -> Session
    where
        T: FAsyncRead + FAsyncWrite + Unpin + Send + 'static,
    {
        let conn = Connection::new(io, yamux_config(), mode);
        let (open_tx, open_rx) = mpsc::unbounded_channel();
        let (accept_tx, accept_rx) = mpsc::unbounded_channel();
        let closed = Arc::new(AtomicBool::new(false));
        let closed2 = closed.clone();
        tokio::spawn(drive(conn, open_rx, accept_tx, closed2));
        Session {
            open_tx,
            accept_rx: Mutex::new(accept_rx),
            closed,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Client side: open a stream and run the address handshake
    /// (`[u16 len][addr]` then wait for a `0x00` status byte).
    pub async fn open_stream(&self, addr: &[u8]) -> io::Result<Stream> {
        let (tx, rx) = oneshot::channel();
        self.open_tx.send(tx).map_err(|_| closed_err())?;
        let mut raw = rx
            .await
            .map_err(|_| closed_err())?
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        write_addr(&mut raw, addr).await?;

        let mut sb = [0u8; 1];
        match tokio::time::timeout(OPEN_WAIT_TIMEOUT, raw.read_exact(&mut sb)).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                let _ = raw.close().await;
                return Err(e);
            }
            Err(_) => {
                let _ = raw.close().await;
                return Err(io::Error::new(io::ErrorKind::TimedOut, "open stream timeout"));
            }
        }
        if sb[0] != STATUS_OK {
            let _ = raw.close().await;
            return Err(io::Error::new(io::ErrorKind::ConnectionRefused, "stream refused"));
        }
        Ok(Stream::new(raw, addr.to_vec()))
    }

    /// Server side: accept the next stream and read its target address. On a
    /// malformed/timed-out handshake the stream is dropped and we keep serving
    /// (the session stays alive), matching Go's `AcceptStream` loop.
    pub async fn accept_stream(&self) -> io::Result<Stream> {
        let mut rx = self.accept_rx.lock().await;
        loop {
            let mut raw = rx.recv().await.ok_or_else(closed_err)?;
            match tokio::time::timeout(HANDSHAKE_TIMEOUT, read_addr(&mut raw)).await {
                Ok(Ok(addr)) => return Ok(Stream::new(raw, addr)),
                _ => {
                    let _ = raw.close().await;
                    continue;
                }
            }
        }
    }
}

fn closed_err() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, "tunnel session closed")
}

async fn write_addr<W: FAsyncWrite + Unpin>(w: &mut W, addr: &[u8]) -> io::Result<()> {
    if addr.len() > MAX_ADDR_LEN {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "addr too long"));
    }
    let mut buf = Vec::with_capacity(2 + addr.len());
    buf.extend_from_slice(&(addr.len() as u16).to_be_bytes());
    buf.extend_from_slice(addr);
    w.write_all(&buf).await?;
    w.flush().await?;
    Ok(())
}

async fn read_addr<R: FAsyncRead + Unpin>(r: &mut R) -> io::Result<Vec<u8>> {
    let mut lb = [0u8; 2];
    r.read_exact(&mut lb).await?;
    let n = u16::from_be_bytes(lb) as usize;
    if n == 0 || n > MAX_ADDR_LEN {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid addr length"));
    }
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Drives the yamux connection: services outbound open requests and forwards
/// inbound streams. Keeps running until the connection closes, regardless of
/// whether the owning `Session` handle has been dropped (existing streams must
/// stay alive).
async fn drive<T>(
    mut conn: Connection<T>,
    mut open_rx: mpsc::UnboundedReceiver<OpenResp>,
    accept_tx: mpsc::UnboundedSender<yamux::Stream>,
    closed: Arc<AtomicBool>,
) where
    T: FAsyncRead + FAsyncWrite + Unpin,
{
    let mut pending: VecDeque<OpenResp> = VecDeque::new();

    futures::future::poll_fn(|cx| poll_drive(&mut conn, &mut open_rx, &accept_tx, &mut pending, cx)).await;

    closed.store(true, Ordering::SeqCst);
    // Fail any opens that never completed.
    while let Some(resp) = pending.pop_front() {
        let _ = resp.send(Err("session closed".to_string()));
    }
    let _ = futures::future::poll_fn(|cx| conn.poll_close(cx)).await;
}

fn poll_drive<T>(
    conn: &mut Connection<T>,
    open_rx: &mut mpsc::UnboundedReceiver<OpenResp>,
    accept_tx: &mpsc::UnboundedSender<yamux::Stream>,
    pending: &mut VecDeque<OpenResp>,
    cx: &mut Context<'_>,
) -> Poll<()>
where
    T: FAsyncRead + FAsyncWrite + Unpin,
{
    // 1. Collect any new open requests.
    loop {
        match open_rx.poll_recv(cx) {
            Poll::Ready(Some(resp)) => pending.push_back(resp),
            Poll::Ready(None) | Poll::Pending => break,
        }
    }

    // 2. Service pending opens.
    while !pending.is_empty() {
        match conn.poll_new_outbound(cx) {
            Poll::Ready(Ok(s)) => {
                if let Some(resp) = pending.pop_front() {
                    let _ = resp.send(Ok(s));
                }
            }
            Poll::Ready(Err(e)) => {
                while let Some(resp) = pending.pop_front() {
                    let _ = resp.send(Err(e.to_string()));
                }
                return Poll::Ready(());
            }
            Poll::Pending => break,
        }
    }

    // 3. Accept inbound streams (this also drives all socket I/O).
    loop {
        match conn.poll_next_inbound(cx) {
            Poll::Ready(Some(Ok(s))) => {
                let _ = accept_tx.send(s);
            }
            Poll::Ready(Some(Err(_))) | Poll::Ready(None) => return Poll::Ready(()),
            Poll::Pending => return Poll::Pending,
        }
    }
}
