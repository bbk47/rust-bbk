use std::future::Future;
use std::io;

use tokio::net::{TcpListener, TcpStream, UdpSocket};

pub mod connect;
pub mod socks5;
pub mod udprelay;

pub use udprelay::{client_udp, is_udp_marker, serve_udp, udp_marker};

/// Result of a local proxy handshake: either a plain TCP target to relay, or a
/// SOCKS5 UDP association.
pub enum Inbound {
    /// TCP CONNECT: the accepted socket plus the SOCKS5-encoded target address.
    Tcp(TcpStream, Vec<u8>),
    /// SOCKS5 UDP ASSOCIATE.
    Udp(Socks5UdpProxy),
}

/// Holds the SOCKS5 control TCP connection and the relay UDP socket.
pub struct Socks5UdpProxy {
    pub ctrl: TcpStream,
    pub udp: UdpSocket,
}

/// Accepts connections on `host:port`, invoking `handler` per connection.
pub async fn listen<H, F>(host: &str, port: u16, handler: H) -> io::Result<()>
where
    H: Fn(TcpStream) -> F + Clone + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let ln = TcpListener::bind(format!("{}:{}", host, port)).await?;
    loop {
        let (sock, _) = ln.accept().await?;
        sock.set_nodelay(true).ok();
        let handler = handler.clone();
        tokio::spawn(handler(sock));
    }
}
