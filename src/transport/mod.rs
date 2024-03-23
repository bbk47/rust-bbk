mod base;
// mod http2;
mod tcp;
// mod tls;
// mod websocket;

use std::error::Error;

pub use base::Transport;
pub use tcp::TcpTransport;

use crate::option::TunnelOpts;

use self::tcp::new_tcp_transport;


// pub fn wrap_tunnel(tunnel: &server::TunnelConn) -> Box<dyn Transport> {
//     match tunnel.tuntype.as_str() {
//         "ws" => Box::new(WebsocketTransport { conn: tunnel.wsocket.clone() }),
//         "h2" => Box::new(Http2Transport { h2socket: tunnel.h2socket.clone() }),
//         "tcp" => Box::new(TcpTransport {
//             conn: tunnel.tcpsocket.try_clone().unwrap(),
//         }),
//         _ => Box::new(TlsTransport {
//             conn: tunnel.tcpsocket.try_clone().unwrap(),
//         }),
//     }
// }

pub fn create_transport(tun_opts: &TunnelOpts) -> Result<Box<dyn Transport + Send + Sync>, Box<dyn Error>> {
    let tunport:u16= tun_opts.port.parse()?;
    let tsport=new_tcp_transport(&tun_opts.host,tunport)?;

    Ok(Box::new(tsport))
}
