use std::io;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use rand::RngCore;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::utils::encrypt::{Encryptor, StreamCrypter};

/// Whole-connection stream encryption. A fresh random IV is generated per
/// connection and exchanged in clear before any cipher traffic; afterwards the
/// continuous keystream encrypts every byte. This mirrors Go's
/// `tunnel.SecureConn` (per-connection random IV + continuous cipher.Stream).
pub struct SecureConn<C> {
    raw: C,
    enc: StreamCrypter,
    dec: StreamCrypter,
    // ciphertext produced by Write but not yet fully flushed to `raw`.
    out_buf: Vec<u8>,
    out_pos: usize,
}

fn io_err(s: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, s)
}

impl<C> SecureConn<C>
where
    C: AsyncRead + AsyncWrite + Unpin,
{
    /// Performs the plaintext IV exchange and builds the enc/dec streams.
    /// Writing then reading is deadlock-free because IVs are tiny (<=16 bytes)
    /// and fit entirely in the transport's send buffer.
    pub async fn handshake(mut raw: C, enc: &Encryptor) -> io::Result<SecureConn<C>> {
        let iv_len = enc.iv_len();
        let mut local_iv = vec![0u8; iv_len];
        rand::thread_rng().fill_bytes(&mut local_iv);

        raw.write_all(&local_iv).await?;
        raw.flush().await?;

        let mut peer_iv = vec![0u8; iv_len];
        raw.read_exact(&mut peer_iv).await?;

        let enc_stream = enc.new_enc_stream(&local_iv).map_err(io_err)?;
        let dec_stream = enc.new_dec_stream(&peer_iv).map_err(io_err)?;
        Ok(SecureConn {
            raw,
            enc: enc_stream,
            dec: dec_stream,
            out_buf: Vec::new(),
            out_pos: 0,
        })
    }

    fn poll_flush_buf(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        while self.out_pos < self.out_buf.len() {
            let n = ready!(Pin::new(&mut self.raw).poll_write(cx, &self.out_buf[self.out_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
            }
            self.out_pos += n;
        }
        self.out_buf.clear();
        self.out_pos = 0;
        Poll::Ready(Ok(()))
    }
}

impl<C> AsyncRead for SecureConn<C>
where
    C: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let want = buf.remaining();
        if want == 0 {
            return Poll::Ready(Ok(()));
        }
        let mut tmp = vec![0u8; want];
        let mut rb = ReadBuf::new(&mut tmp);
        ready!(Pin::new(&mut this.raw).poll_read(cx, &mut rb))?;
        let filled = rb.filled();
        if !filled.is_empty() {
            let dec = this.dec.xor(filled).map_err(io_err)?;
            buf.put_slice(&dec);
        }
        Poll::Ready(Ok(()))
    }
}

impl<C> AsyncWrite for SecureConn<C>
where
    C: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, p: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        // Flush any pending ciphertext before encrypting new plaintext so the
        // continuous keystream never gets ahead of what was actually sent.
        ready!(this.poll_flush_buf(cx))?;
        if p.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let enc = this.enc.xor(p).map_err(io_err)?;
        this.out_buf = enc;
        this.out_pos = 0;
        if let Poll::Ready(Err(e)) = this.poll_flush_buf(cx) {
            return Poll::Ready(Err(e));
        }
        Poll::Ready(Ok(p.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_flush_buf(cx))?;
        Pin::new(&mut this.raw).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_flush_buf(cx))?;
        Pin::new(&mut this.raw).poll_shutdown(cx)
    }
}
