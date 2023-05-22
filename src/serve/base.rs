use std::io::Result;
use std::net::{TcpListener, TcpStream};

pub struct TunnelConn {
    pub tuntype: String,
    pub tcp_socket: TcpStream,
}



pub trait FrameServer {
    fn get_addr(&self) -> String;
}
