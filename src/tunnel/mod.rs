use std::io;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::utils::encrypt::Encryptor;

mod secure;
mod session;
mod stream;
mod wsconn;

pub use secure::SecureConn;
pub use session::Session;
pub use stream::Stream;
pub use wsconn::WsConn;

/// Client side: wrap a raw carrier in SecureConn (random-IV continuous cipher)
/// and start a yamux client session over it.
pub async fn client_session<C>(raw: C, enc: &Encryptor) -> io::Result<Session>
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let secure = SecureConn::handshake(raw, enc).await?;
    Ok(Session::client(secure.compat()))
}

/// Server side: same handshake, but start a yamux server session.
pub async fn server_session<C>(raw: C, enc: &Encryptor) -> io::Result<Session>
where
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let secure = SecureConn::handshake(raw, enc).await?;
    Ok(Session::server(secure.compat()))
}
