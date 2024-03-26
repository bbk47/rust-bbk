use std::{
    error::Error,
    net::{TcpListener, TcpStream},
};

use native_tls::TlsStream;
use tungstenite::WebSocket;

pub struct TunnelConn {
    pub tuntype: String,
    pub tcp_socket: Option<TcpStream>,
    pub tls_socket: Option<TlsStream<TcpStream>>,
    pub websocket: Option<WebSocket<TcpStream>>,
}

pub trait FrameServer {
    fn get_addr(&self) -> String;
    fn accept(&mut self) -> Result<TunnelConn, Box<dyn Error>>;
}
