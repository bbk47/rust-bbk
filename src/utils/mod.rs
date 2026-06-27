pub mod encrypt;
pub mod socks5;

use tokio::io::{AsyncRead, AsyncWrite};

/// Bidirectional copy with half-close, mirroring Go's utils.Relay: when one
/// side reaches EOF, the peer's write half is shut down (FIN) and the other
/// direction keeps flowing until it also ends.
pub async fn relay<A, B>(a: &mut A, b: &mut B)
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let _ = tokio::io::copy_bidirectional(a, b).await;
}
