use std::{net::{TcpListener, TcpStream}, error::Error};


pub struct TunnelConn {
    pub tuntype: String,
    pub tcp_socket: TcpStream,
}


impl TunnelConn {
    pub fn new(tuntype: String, tcp_socket: TcpStream) -> TunnelConn {
        TunnelConn {
            tuntype,
            tcp_socket,
        }
    }
}


pub trait FrameServer {
    fn listen_conn(&self, handler: impl Fn(&TunnelConn)  + Send + Sync + 'static) -> Result<(), Box<dyn Error>>;
    fn get_addr(&self) -> String;
}


