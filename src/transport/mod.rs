
mod base;
// mod http2;
mod tcp;
// mod tls;
// mod websocket;

pub use base::Transport;
pub use tcp::TcpTransport;


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