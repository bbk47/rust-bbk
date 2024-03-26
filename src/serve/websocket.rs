use std::error::Error;

use std::net::{TcpListener, TcpStream};

use tungstenite::{accept, accept_hdr};

use super::base::FrameServer;
use super::TunnelConn;

// TCP 监听器
pub struct AbcWssServer {
    listener: TcpListener,
}

impl AbcWssServer {
    pub fn new(ln: TcpListener) -> AbcWssServer {
        AbcWssServer { listener: ln }
    }
}

impl FrameServer for AbcWssServer {
    fn get_addr(&self) -> String {
        format!("tcp://{}", self.listener.local_addr().unwrap())
    }

    fn accept(&mut self) -> Result<TunnelConn, Box<dyn Error>> {
        match self.listener.accept() {
            Ok((stream, _addr)) => {
                let mut wssss = accept(stream)?;
                let tuntype: String = String::from("ws");
                let tuncon = TunnelConn {
                    tuntype,
                    websocket: Some(wssss),
                    tcp_socket: None,
                    tls_socket: None,
                };
                Ok(tuncon)
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}
