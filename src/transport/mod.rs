mod base;
// mod http2;
mod tcp;
mod tls;
mod websocket;

use tungstenite::client;
use url::Url;

use std::{
    cell::UnsafeCell, error::Error, net::{TcpStream, ToSocketAddrs}, time::Duration
};

pub use base::Transport;
use native_tls::TlsConnector;
pub use tcp::TcpTransport;
pub use tls::TlsTransport;
use tungstenite::connect;
pub use websocket::WsTransport;
pub use websocket::WssTransport;

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

// pub fn create_transport(tun_opts: &TunnelOpts) -> Result<Box<dyn Transport + Send + Sync>, Box<dyn Error>> {
//     let tunport:u16= tun_opts.port.parse()?;
//     let tsport=new_tcp_transport(&tun_opts.host,tunport)?;

//     Ok(Box::new(tsport))
// }

pub fn new_tcp_transport(host: &str, port: u16) -> Result<Box<dyn Transport + Send + Sync>, Box<dyn Error>> {
    let socket_addr = (host, port).to_socket_addrs()?.next().unwrap();
    let conn = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))?;
    let ts = TcpTransport { conn };
    Ok(Box::new(ts))
}

pub fn new_tls_transport(host: &str, port: u16) -> Result<Box<dyn Transport + Send + Sync>, Box<dyn Error>> {
    // let remote_addr = format!("{}:{}", host, port);
    let connector = TlsConnector::new().unwrap();
    let stream = TcpStream::connect(format!("{}:{}", host, port))?;
    let stream = connector.connect(host, stream)?;
    let ts = TlsTransport { conn: UnsafeCell::new(stream) };
    Ok(Box::new(ts))
}

pub fn new_websocket_transport(host: &str, port: u16, path: &str, secure: bool) -> Result<Box<dyn Transport + Send + Sync>, Box<dyn Error>> {
    let ws_url = if secure { format!("wss://{}{}", host, path) } else { format!("ws://{}:{}{}", host, port, path) };
    println!("transport wsurl: {}", ws_url);
    let (socket, _) = connect(Url::parse(&ws_url).unwrap())?;
    Ok(Box::new(WssTransport { conn: UnsafeCell::new(socket) }))
}

// fn new_http2_transport(host: &str, port: &str, path: &str) -> Result<Http2Transport, Box<dyn Error>> {
//     Err(format!("Unexpected stream status: {},{},{},{}",host,port,path, 101).into())
// }
