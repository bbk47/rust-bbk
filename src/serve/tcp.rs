use std::error::Error;
use std::net::{TcpListener, TcpStream};

use super::base::FrameServer;
use super::TunnelConn;

// TCP 监听器
pub struct AbcTcpServer {
    listener: TcpListener,
}

impl AbcTcpServer {
    pub fn new(ln: TcpListener) -> AbcTcpServer {
        AbcTcpServer { listener: ln }
    }
}

impl FrameServer for AbcTcpServer {
    fn get_addr(&self) -> String {
        format!("tcp://{}", self.listener.local_addr().unwrap())
    }

    fn accept(&mut self) -> Result<TunnelConn, Box<dyn Error>> {
        match self.listener.accept() {
            Ok((stream, _addr)) => {
                let socket = stream;
                let tuntype: String = String::from("tcp");
                let tuncon = TunnelConn {
                    tuntype,
                    websocket: None,
                    tcp_socket: Some(socket),
                    tls_socket: None,
                };
                Ok(tuncon)
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}
